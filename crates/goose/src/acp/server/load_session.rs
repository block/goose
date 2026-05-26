use super::*;

fn replay_conversation_to_client(
    cx: &ConnectionTo<Client>,
    session: &Session,
) -> Result<HashMap<String, crate::conversation::message::ToolRequest>, agent_client_protocol::Error>
{
    let session_id = SessionId::new(session.id.clone());
    let sid = sid_short(session_id.0.as_ref());

    let messages = session
        .conversation
        .as_ref()
        .map(|c| c.messages().to_vec())
        .unwrap_or_default();
    debug!(
        target: "perf",
        sid = %sid,
        messages = messages.len(),
            "perf: load_session messages loaded"
    );

    let mut replay_tool_requests =
        HashMap::<String, crate::conversation::message::ToolRequest>::new();
    let submitted_elicitation_ids = collect_submitted_elicitation_ids(&messages);

    for message in &messages {
        if !message.metadata.user_visible {
            continue;
        }

        for content_item in &message.content {
            match content_item {
                MessageContent::Text(text) => {
                    let mut tc = TextContent::new(text.text.clone());
                    if let Some(audience) = text.audience() {
                        tc = tc.annotations(
                            Annotations::new().audience(
                                audience
                                    .iter()
                                    .map(|r| match r {
                                        Role::Assistant => {
                                            agent_client_protocol::schema::Role::Assistant
                                        }
                                        Role::User => agent_client_protocol::schema::Role::User,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                        );
                    }
                    let chunk = ContentChunk::new(ContentBlock::Text(tc))
                        .meta(replay_message_meta(message));
                    let update = match message.role {
                        Role::User => SessionUpdate::UserMessageChunk(chunk),
                        Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
                    };
                    cx.send_notification(SessionNotification::new(session_id.clone(), update))?;
                }
                MessageContent::Image(image) => {
                    let chunk = ContentChunk::new(ContentBlock::Image(ImageContent::new(
                        image.data.clone(),
                        image.mime_type.clone(),
                    )))
                    .meta(replay_message_meta(message));
                    let update = match message.role {
                        Role::User => SessionUpdate::UserMessageChunk(chunk),
                        Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
                    };
                    cx.send_notification(SessionNotification::new(session_id.clone(), update))?;
                }
                MessageContent::ToolRequest(tool_request) => {
                    replay_tool_requests.insert(tool_request.id.clone(), tool_request.clone());

                    let pending_tool_call = pending_tool_call_from_request(tool_request);
                    let mut meta = pending_tool_call.identity_meta;
                    if let Some(chain_summary) = tool_request.persisted_chain_summary() {
                        meta = with_tool_chain_summary_meta(
                            meta,
                            &chain_summary.summary,
                            chain_summary.count,
                        );
                    }
                    let tool_call = pending_tool_call
                        .tool_call
                        .meta(merge_replay_message_meta(meta, message));

                    cx.send_notification(SessionNotification::new(
                        session_id.clone(),
                        SessionUpdate::ToolCall(tool_call),
                    ))?;
                }
                MessageContent::ToolResponse(tool_response) => {
                    let status = match &tool_response.tool_result {
                        Ok(result) if result.is_error == Some(true) => ToolCallStatus::Failed,
                        Ok(_) => ToolCallStatus::Completed,
                        Err(_) => ToolCallStatus::Failed,
                    };

                    let mut fields = ToolCallUpdateFields::new().status(status);
                    if let Some(raw_output) = extract_tool_raw_output(&tool_response.tool_result) {
                        fields = fields.raw_output(raw_output);
                    }
                    if !tool_response
                        .tool_result
                        .as_ref()
                        .is_ok_and(|r| r.is_acp_aware())
                    {
                        let content = build_tool_call_content(&tool_response.tool_result);
                        fields = fields.content(content);

                        let locations =
                            extract_locations_from_meta(tool_response).unwrap_or_else(|| {
                                if let Some(tool_request) =
                                    replay_tool_requests.get(&tool_response.id)
                                {
                                    extract_tool_locations(tool_request, tool_response)
                                } else {
                                    Vec::new()
                                }
                            });
                        if !locations.is_empty() {
                            fields = fields.locations(locations);
                        }
                    }

                    let update =
                        ToolCallUpdate::new(ToolCallId::new(tool_response.id.clone()), fields)
                            .meta(merge_replay_message_meta(
                                extract_tool_call_update_meta(tool_response),
                                message,
                            ));
                    cx.send_notification(SessionNotification::new(
                        session_id.clone(),
                        SessionUpdate::ToolCallUpdate(update),
                    ))?;
                }
                MessageContent::Thinking(thinking) => {
                    cx.send_notification(SessionNotification::new(
                        session_id.clone(),
                        SessionUpdate::AgentThoughtChunk(
                            ContentChunk::new(ContentBlock::Text(TextContent::new(
                                thinking.thinking.clone(),
                            )))
                            .meta(replay_message_meta(message)),
                        ),
                    ))?;
                }
                MessageContent::ActionRequired(action_required) => {
                    if let ActionRequiredData::Elicitation {
                        id,
                        message: elicitation_message,
                        requested_schema,
                    } = &action_required.data
                    {
                        if !submitted_elicitation_ids.contains(id) {
                            send_elicitation_interaction_update(
                                cx,
                                session_id.0.as_ref(),
                                id.clone(),
                                InteractionState::Pending,
                                Some(elicitation_message.clone()),
                                Some(requested_schema.clone()),
                                Some(serde_json::Value::Object(replay_message_meta(message))),
                            )?;
                        }
                    }
                }
                MessageContent::SystemNotification(_) => {}
                _ => {}
            }
        }
    }

    Ok(replay_tool_requests)
}

fn collect_submitted_elicitation_ids(messages: &[Message]) -> HashSet<String> {
    let mut submitted_ids = HashSet::new();

    for message in messages {
        for content_item in &message.content {
            if let MessageContent::ActionRequired(action_required) = content_item {
                if let ActionRequiredData::ElicitationResponse { id, .. } = &action_required.data {
                    submitted_ids.insert(id.clone());
                }
            }
        }
    }

    submitted_ids
}

impl GooseAcpAgent {
    async fn build_agent_for_session(
        &self,
        cx: &ConnectionTo<Client>,
        session: &Session,
        resolved_provider: Option<(String, crate::model::ModelConfig)>,
        prebuilt_provider: Option<Arc<dyn Provider>>,
    ) -> Result<(Arc<Agent>, Vec<crate::agents::ExtensionLoadResult>), agent_client_protocol::Error>
    {
        use crate::agents::ExtensionLoadResult;

        let session_id = SessionId::new(session.id.clone());
        let sid = sid_short(session_id.0.as_ref());
        let t_setup = std::time::Instant::now();
        let goose_mode = session.goose_mode;

        let config = self.load_config().map_err(|e| {
            agent_client_protocol::Error::internal_error()
                .data(format!("Failed to read config: {}", e))
        })?;

        let session_name_update_tx =
            (!self.disable_session_naming).then(|| spawn_session_name_update_notifier(cx.clone()));

        let client_mcp_host_info = self.client_mcp_host_info.get().cloned();
        let use_login_shell_path = self.use_login_shell_path.get().copied().unwrap_or(false);
        let agent = Arc::new(Agent::with_config(
            AgentConfig::new(
                Arc::clone(&self.session_manager),
                Arc::clone(&self.permission_manager),
                None,
                goose_mode,
                self.disable_session_naming,
                self.goose_platform.clone(),
            )
            .with_mcp_host_info(client_mcp_host_info)
            .with_session_name_update_tx(session_name_update_tx)
            .with_use_login_shell_path(use_login_shell_path),
        ));

        let (provider_name, model_config) = match resolved_provider {
            Some(resolved) => resolved,
            None => resolve_provider_and_model_from_config(&config, session)
                .await
                .map_err(|e| {
                    agent_client_protocol::Error::internal_error()
                        .data(format!("Failed to resolve provider: {}", e))
                })?,
        };

        let ext_state =
            EnabledExtensionsState::extensions_or_default(Some(&session.extension_data), &config);

        let provider = match prebuilt_provider {
            Some(provider) => provider,
            None => self
                .create_provider(
                    &provider_name,
                    model_config,
                    ext_state,
                    Some(session.working_dir.clone()),
                )
                .await
                .map_err(|e| {
                    agent_client_protocol::Error::internal_error()
                        .data(format!("Failed to create provider: {}", e))
                })?,
        };

        agent
            .update_provider(provider.clone(), &session.id)
            .await
            .map_err(|e| {
                agent_client_protocol::Error::internal_error()
                    .data(format!("Failed to update provider: {}", e))
            })?;
        agent
            .update_goose_mode(goose_mode, &session.id)
            .await
            .map_err(|e| {
                agent_client_protocol::Error::internal_error()
                    .data(format!("Failed to update goose mode: {}", e))
            })?;

        debug!(
            target: "perf",
            sid = %sid,
            ms = t_setup.elapsed().as_millis() as u64,
            "perf: build_agent_for_session provider_ready"
        );

        let mut extensions = get_enabled_extensions_with_config(&config);
        extensions.extend(self.builtins.iter().map(|b| builtin_to_extension_config(b)));

        let client_fs_capabilities = self
            .client_fs_capabilities
            .get()
            .cloned()
            .unwrap_or_default();
        let client_terminal = self.client_terminal.get().copied().unwrap_or(false);

        let acp_developer = if (client_fs_capabilities.read_text_file
            || client_fs_capabilities.write_text_file
            || client_terminal)
            && extensions.iter().any(|e| e.name() == "developer")
        {
            let context = agent.extension_manager.get_context().clone();
            match DeveloperClient::new(context) {
                Ok(dev_client) => {
                    let client: Arc<dyn McpClientTrait> = Arc::new(AcpTools {
                        inner: Arc::new(dev_client),
                        cx: cx.clone(),
                        session_id: session_id.clone(),
                        fs_read: client_fs_capabilities.read_text_file,
                        fs_write: client_fs_capabilities.write_text_file,
                        terminal: client_terminal,
                    });
                    let dev_ext = extensions.iter().find(|e| e.name() == "developer");
                    let available_tools = dev_ext
                        .and_then(|e| match e {
                            ExtensionConfig::Platform {
                                available_tools, ..
                            } => Some(available_tools.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let def = &PLATFORM_EXTENSIONS["developer"];
                    let config = ExtensionConfig::Platform {
                        name: def.name.into(),
                        description: def.description.into(),
                        display_name: Some(def.display_name.into()),
                        bundled: Some(true),
                        available_tools,
                    };
                    Some((client, config))
                }
                Err(e) => {
                    warn!(error = %e, "Failed to create developer client");
                    None
                }
            }
        } else {
            None
        };

        let skip_developer = acp_developer.is_some();
        let sid_str = Some(session.id.clone());

        if skip_developer {
            extensions.retain(|ext| ext.name() != "developer");
        }

        let ext_manager = &agent.extension_manager;
        let working_dir = session.working_dir.clone();
        let extension_futures = extensions
            .into_iter()
            .map(|ext| {
                let ext_manager = Arc::clone(ext_manager);
                let sid_inner = sid_str.clone();
                let working_dir = working_dir.clone();
                async move {
                    let name = ext.name().to_string();
                    match ext_manager
                        .add_extension(ext, Some(working_dir), None, sid_inner.as_deref())
                        .await
                    {
                        Ok(_) => ExtensionLoadResult {
                            name,
                            success: true,
                            error: None,
                        },
                        Err(e) => {
                            let error_msg = e.to_string();
                            warn!(extension = %name, error = %error_msg, "extension load failed");
                            ExtensionLoadResult {
                                name,
                                success: false,
                                error: Some(error_msg),
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        let mut extension_results: Vec<ExtensionLoadResult> =
            futures::future::join_all(extension_futures).await;

        if let Some((client, config)) = acp_developer {
            let info = client.get_info().cloned();
            agent
                .extension_manager
                .add_client("developer".into(), config, client, info, None)
                .await;
            extension_results.push(ExtensionLoadResult {
                name: "developer".to_string(),
                success: true,
                error: None,
            });
        }

        // `add_mcp_extensions` intentionally NOT called:
        // `handle_load_session` rejects non-empty `args.mcp_servers`.

        debug!(
            target: "perf",
            sid = %sid,
            ms = t_setup.elapsed().as_millis() as u64,
            "perf: build_agent_for_session done"
        );

        Ok((agent, extension_results))
    }

    pub(super) async fn handle_load_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "load session request");

        if !args.mcp_servers.is_empty() {
            return Err(agent_client_protocol::Error::invalid_params().data(
                "goose manages MCP servers server-side; use _goose/extensions/add \
                 to add extensions to a session",
            ));
        }

        validate_absolute_cwd(&args.cwd)?;

        let session_id_str = args.session_id.0.to_string();
        let sid = sid_short(&session_id_str);
        let t_start = std::time::Instant::now();

        let mut session = self
            .session_manager
            .get_session(&session_id_str, true)
            .await
            .map_err(|_| {
                agent_client_protocol::Error::resource_not_found(Some(session_id_str.clone()))
                    .data(format!("Session not found: {}", session_id_str))
            })?;

        let replay_tool_requests = replay_conversation_to_client(cx, &session)?;

        if args.cwd != session.working_dir {
            self.session_manager
                .update(&session_id_str)
                .working_dir(args.cwd.clone())
                .apply()
                .await
                .internal_err()?;
            session.working_dir = args.cwd.clone();
        }

        let resolved = resolve_provider_and_model(&self.config_dir, &session).await;

        let mode_state = build_mode_state(session.goose_mode)?;
        let (model_state, config_options, prebuilt_provider) = self
            .prepare_session_init_config(&resolved, &mode_state, &session)
            .await;

        let (agent, extension_results) = self
            .build_agent_for_session(
                cx,
                &session,
                resolved.as_ref().ok().cloned(),
                prebuilt_provider,
            )
            .await?;

        let acp_session = GooseAcpSession {
            agent: AgentHandle::Ready(agent.clone()),
            tool_requests: replay_tool_requests,
            chain_membership: HashMap::new(),
            responded_tool_ids: HashSet::new(),
            summarized_chains: HashSet::new(),
            cancel_token: None,
            pending_working_dir: None,
        };
        self.sessions
            .lock()
            .await
            .insert(session_id_str.clone(), acp_session);

        let initial_usage_updates = resolved
            .as_ref()
            .ok()
            .map(|(_, mc)| build_usage_updates(&args.session_id, &session, mc.context_limit()))
            .or_else(|| {
                session
                    .model_config
                    .as_ref()
                    .map(|mc| build_usage_updates(&args.session_id, &session, mc.context_limit()))
            });
        if let Some(updates) = initial_usage_updates {
            cx.send_notification(updates.custom)?;
            // Legacy UsageUpdate alongside the custom one for compat.
            cx.send_notification(SessionNotification::new(
                args.session_id.clone(),
                SessionUpdate::UsageUpdate(updates.legacy),
            ))?;
        }

        Self::send_available_commands_update(cx, &args.session_id, &session.working_dir)?;

        let mut response = LoadSessionResponse::new().modes(mode_state);
        if let Some(ms) = model_state {
            response = response.models(ms);
        }
        if let Some(co) = config_options {
            response = response.config_options(co);
        }

        let mut meta = serde_json::Map::new();
        if let Some(recipe) = &session.recipe {
            if let Ok(v) = serde_json::to_value(recipe) {
                meta.insert("recipe".to_string(), v);
            }
        }
        if let Some(values) = &session.user_recipe_values {
            if let Ok(v) = serde_json::to_value(values) {
                meta.insert("userRecipeValues".to_string(), v);
            }
        }
        if let Ok(v) = serde_json::to_value(&extension_results) {
            meta.insert("extensionResults".to_string(), v);
        }
        meta.insert(
            "workingDir".to_string(),
            serde_json::Value::String(session.working_dir.to_string_lossy().to_string()),
        );
        if !meta.is_empty() {
            response = response.meta(meta);
        }

        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            "perf: load_session done"
        );
        Ok(response)
    }
}
