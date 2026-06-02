use super::*;

impl GooseAcpAgent {
    pub(super) async fn on_initialize(
        &self,
        args: InitializeRequest,
    ) -> Result<InitializeResponse, agent_client_protocol::Error> {
        debug!(?args, "initialize request");

        let _ = self
            .client_fs_capabilities
            .set(args.client_capabilities.fs.clone());
        let _ = self.client_terminal.set(args.client_capabilities.terminal);
        let _ = self
            .client_mcp_host_info
            .set(extract_client_mcp_host_info(&args));
        let _ = self
            .use_login_shell_path
            .set(extract_use_login_shell_path(&args));

        let capabilities = AgentCapabilities::new()
            .load_session(true)
            .session_capabilities(
                SessionCapabilities::new()
                    .list(SessionListCapabilities::new())
                    .close(SessionCloseCapabilities::new()),
            )
            .prompt_capabilities(
                PromptCapabilities::new()
                    .image(true)
                    .audio(false)
                    .embedded_context(true),
            )
            .mcp_capabilities(McpCapabilities::new().http(true));
        Ok(InitializeResponse::new(args.protocol_version)
            .agent_capabilities(capabilities)
            .auth_methods(vec![AuthMethod::Agent(
                AuthMethodAgent::new("goose-provider", "Configure Provider")
                    .description("Run `goose configure` to set up your AI provider and API key"),
            )]))
    }

    pub(super) async fn on_new_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: NewSessionRequest,
    ) -> Result<NewSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "new session request");
        let t_start = std::time::Instant::now();
        validate_absolute_cwd(&args.cwd)?;

        let requested_provider = args
            .meta
            .as_ref()
            .and_then(|m| m.get("provider"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let project_id = args
            .meta
            .as_ref()
            .and_then(|m| m.get("projectId"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // When _meta.client is set, the session is created by a known client
        // (e.g. "goose" for the desktop app) and treated as a User session.
        // Without it, sessions default to Acp for programmatic ACP clients.
        let session_type = match args
            .meta
            .as_ref()
            .and_then(|m| m.get("client"))
            .and_then(|v| v.as_str())
        {
            Some(_) => SessionType::User,
            None => SessionType::Acp,
        };

        let t0 = std::time::Instant::now();
        let goose_session = self
            .session_manager
            .create_session(
                args.cwd.clone(),
                "New Chat".to_string(),
                session_type,
                self.goose_mode,
            )
            .await
            .internal_err_ctx("Failed to create session")?;

        let mut builder = self.session_manager.update(&goose_session.id);
        if let Some(ref provider) = requested_provider {
            builder = builder.provider_name(provider);
        }
        if let Some(pid) = project_id {
            builder = builder.project_id(Some(pid));
        }
        builder
            .apply()
            .await
            .internal_err_ctx("Failed to update session")?;

        let goose_session = self
            .session_manager
            .get_session(&goose_session.id, false)
            .await
            .internal_err_ctx("Failed to reload session")?;

        let session_id_str = goose_session.id.clone();
        let sid = sid_short(&session_id_str);
        debug!(target: "perf", sid = %sid, ms = t0.elapsed().as_millis() as u64, "perf: new_session create_session");

        let (agent_tx, agent_rx) = tokio::sync::watch::channel::<AgentSetupSignal>(None);

        let acp_session = GooseAcpSession {
            agent: AgentHandle::Loading(agent_rx),
            tool_requests: HashMap::new(),
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

        let mode_state = build_mode_state(self.goose_mode)?;

        let resolved = resolve_provider_and_model(&self.config_dir, &goose_session).await;
        let initial_usage_update = resolved
            .as_ref()
            .ok()
            .map(|(_, mc)| build_usage_update(&goose_session, mc.context_limit()));
        let acp_session_id = SessionId::new(session_id_str);
        let (model_state, config_options, prebuilt_provider) = self
            .prepare_session_init_config(&resolved, &mode_state, &goose_session)
            .await;

        let working_dir = goose_session.working_dir.clone();

        self.spawn_agent_setup(
            cx,
            agent_tx,
            AgentSetupRequest {
                session_id: acp_session_id.clone(),
                goose_session,
                mcp_servers: args.mcp_servers,
                resolved_provider: resolved.as_ref().ok().cloned(),
                prebuilt_provider,
            },
        );

        let mut response = NewSessionResponse::new(acp_session_id.clone()).modes(mode_state);
        if let Some(ms) = model_state {
            response = response.models(ms);
        }
        if let Some(co) = config_options {
            response = response.config_options(co);
        }
        if let Some(usage_update) = initial_usage_update {
            cx.send_notification(SessionNotification::new(
                acp_session_id.clone(),
                SessionUpdate::UsageUpdate(usage_update),
            ))?;
        }
        Self::send_available_commands_update(cx, &acp_session_id, &working_dir)?;
        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            "perf: new_session done (agent setup continues in background)"
        );
        Ok(response)
    }

    /// Look up the session and return the agent if already ready, or the watch
    /// receiver if still loading.  Optionally sets a cancellation token on the
    /// session (needed by `on_prompt`).
    pub(super) async fn get_agent_or_receiver(
        &self,
        session_id: &str,
        cancel_token: Option<CancellationToken>,
    ) -> Result<
        Either<Arc<Agent>, tokio::sync::watch::Receiver<AgentSetupSignal>>,
        agent_client_protocol::Error,
    > {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            agent_client_protocol::Error::resource_not_found(Some(session_id.to_string()))
                .data(format!("Session not found: {}", session_id))
        })?;
        if let Some(token) = cancel_token {
            session.cancel_token = Some(token);
        }
        match &session.agent {
            AgentHandle::Ready(agent) => Ok(Either::Left(agent.clone())),
            AgentHandle::Loading(rx) => Ok(Either::Right(rx.clone())),
        }
    }

    /// Wait until the agent is **fully ready** (provider + all extensions).
    /// Most callers (e.g. `on_prompt`, `on_get_tools`) should use this.
    pub(super) async fn get_session_agent(
        &self,
        session_id: &str,
        cancel_token: Option<CancellationToken>,
    ) -> Result<Arc<Agent>, agent_client_protocol::Error> {
        let mut rx = match self.get_agent_or_receiver(session_id, cancel_token).await? {
            Either::Left(agent) => return Ok(agent),
            Either::Right(rx) => rx,
        };
        // Wait specifically for FullyReady (not just ProviderReady).
        let guard = rx
            .wait_for(|v| {
                matches!(
                    v,
                    Some(Ok(AgentSetupProgress::FullyReady(_))) | Some(Err(_))
                )
            })
            .await
            .map_err(|_| {
                agent_client_protocol::Error::internal_error()
                    .data("Agent setup task was dropped".to_string())
            })?;
        match guard.as_ref().unwrap() {
            Ok(AgentSetupProgress::FullyReady(agent)) => Ok(agent.clone()),
            Err(e) => Err(agent_client_protocol::Error::internal_error().data(e.clone())),
            // wait_for predicate excludes ProviderReady
            _ => unreachable!(),
        }
    }

    /// Wait only until the **provider** is initialized.  Extensions may still
    /// be loading in the background.  Use this for operations that only touch
    /// the provider (e.g. `update_provider`, `set_model`, `build_config_update`).
    pub(super) async fn get_session_agent_provider_ready(
        &self,
        session_id: &str,
    ) -> Result<Arc<Agent>, agent_client_protocol::Error> {
        let mut rx = match self.get_agent_or_receiver(session_id, None).await? {
            Either::Left(agent) => return Ok(agent),
            Either::Right(rx) => rx,
        };
        // Any signal (ProviderReady, FullyReady, or Err) unblocks us.
        let guard = rx.wait_for(|v| v.is_some()).await.map_err(|_| {
            agent_client_protocol::Error::internal_error()
                .data("Agent setup task was dropped".to_string())
        })?;
        match guard.as_ref().unwrap() {
            Ok(progress) => match progress {
                AgentSetupProgress::ProviderReady(agent)
                | AgentSetupProgress::FullyReady(agent) => Ok(agent.clone()),
            },
            Err(e) => Err(agent_client_protocol::Error::internal_error().data(e.clone())),
        }
    }

    pub(super) async fn add_mcp_extensions(
        agent: &Arc<Agent>,
        mcp_servers: Vec<McpServer>,
        session_id: &str,
    ) -> Result<(), agent_client_protocol::Error> {
        let mut configs = Vec::with_capacity(mcp_servers.len());
        for mcp_server in mcp_servers {
            let config = match mcp_server_to_extension_config(mcp_server) {
                Ok(c) => c,
                Err(msg) => {
                    return Err(agent_client_protocol::Error::invalid_params().data(msg));
                }
            };
            configs.push(config);
        }

        if configs.is_empty() {
            return Ok(());
        }

        let results = agent
            .add_extensions_bulk(configs, session_id)
            .await
            .internal_err()?;
        for result in &results {
            if !result.success {
                let error_msg = result.error.as_deref().unwrap_or("unknown error");
                return Err(agent_client_protocol::Error::internal_error().data(format!(
                    "Failed to add MCP server '{}': {}",
                    result.name, error_msg
                )));
            }
        }
        Ok(())
    }

    pub(super) async fn on_load_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "load session request");
        validate_absolute_cwd(&args.cwd)?;

        let session_id = args.session_id.0.to_string();
        let sid = sid_short(&session_id);
        let t_start = std::time::Instant::now();

        let t0 = std::time::Instant::now();
        let goose_session = self
            .session_manager
            .get_session(&session_id, true)
            .await
            .map_err(|_| {
                agent_client_protocol::Error::resource_not_found(Some(session_id.clone()))
                    .data(format!("Session not found: {}", session_id))
            })?;
        debug!(target: "perf", sid = %sid, ms = t0.elapsed().as_millis() as u64, "perf: load_session get_session");
        let loaded_mode = goose_session.goose_mode;

        // ── REPLAY MESSAGES ──
        // Stream user-visible messages back to the client so the chat view
        // populates immediately, before the slow agent/provider/extension setup.
        let messages = goose_session
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
                        cx.send_notification(SessionNotification::new(
                            args.session_id.clone(),
                            update,
                        ))?;
                    }
                    MessageContent::ToolRequest(tool_request) => {
                        // Replay-only: emit the ToolCall notification and
                        // stash the request for location extraction, but
                        // don't require a full GooseAcpSession.
                        replay_tool_requests.insert(tool_request.id.clone(), tool_request.clone());

                        let pending_tool_call = pending_tool_call_from_request(tool_request);
                        let mut meta = pending_tool_call.identity_meta;
                        // If this tool request is the first of a chain whose
                        // summary was persisted at completion time, attach the
                        // chain summary to the initial ToolCall so the chain
                        // header is correct on first paint after reload.
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
                            args.session_id.clone(),
                            SessionUpdate::ToolCall(tool_call),
                        ))?;
                    }
                    MessageContent::ToolResponse(tool_response) => {
                        // Replay-only: emit the ToolCallUpdate notification,
                        // using the stashed replay_tool_requests for location
                        // extraction.
                        let status = match &tool_response.tool_result {
                            Ok(result) if result.is_error == Some(true) => ToolCallStatus::Failed,
                            Ok(_) => ToolCallStatus::Completed,
                            Err(_) => ToolCallStatus::Failed,
                        };

                        let mut fields = ToolCallUpdateFields::new().status(status);
                        if let Some(raw_output) =
                            extract_tool_raw_output(&tool_response.tool_result)
                        {
                            fields = fields.raw_output(raw_output);
                        }
                        if !tool_response
                            .tool_result
                            .as_ref()
                            .is_ok_and(|r| r.is_acp_aware())
                        {
                            let content = build_tool_call_content(&tool_response.tool_result);
                            fields = fields.content(content);

                            let locations = extract_locations_from_meta(tool_response)
                                .unwrap_or_else(|| {
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
                            args.session_id.clone(),
                            SessionUpdate::ToolCallUpdate(update),
                        ))?;
                    }
                    MessageContent::Thinking(thinking) => {
                        cx.send_notification(SessionNotification::new(
                            args.session_id.clone(),
                            SessionUpdate::AgentThoughtChunk(
                                ContentChunk::new(ContentBlock::Text(TextContent::new(
                                    thinking.thinking.clone(),
                                )))
                                .meta(replay_message_meta(message)),
                            ),
                        ))?;
                    }
                    _ => {}
                }
            }
        }

        // Update working directory.
        self.session_manager
            .update(&session_id)
            .working_dir(args.cwd.clone())
            .apply()
            .await
            .internal_err_ctx("Failed to update session working directory")?;
        let goose_session = self
            .session_manager
            .get_session(&session_id, false)
            .await
            .internal_err_ctx("Failed to reload session")?;

        // Register the session with a Loading handle.
        let (agent_tx, agent_rx) = tokio::sync::watch::channel::<AgentSetupSignal>(None);

        let acp_session = GooseAcpSession {
            agent: AgentHandle::Loading(agent_rx),
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
            .insert(session_id.clone(), acp_session);

        let mode_state = build_mode_state(loaded_mode)?;

        let resolved = resolve_provider_and_model(&self.config_dir, &goose_session).await;
        let initial_usage_update = resolved
            .as_ref()
            .ok()
            .map(|(_, mc)| build_usage_update(&goose_session, mc.context_limit()))
            .or_else(|| {
                goose_session
                    .model_config
                    .as_ref()
                    .map(|mc| build_usage_update(&goose_session, mc.context_limit()))
            });
        let (model_state, config_options, prebuilt_provider) = self
            .prepare_session_init_config(&resolved, &mode_state, &goose_session)
            .await;

        self.spawn_agent_setup(
            cx,
            agent_tx,
            AgentSetupRequest {
                session_id: args.session_id.clone(),
                goose_session,
                mcp_servers: args.mcp_servers,
                resolved_provider: None,
                prebuilt_provider,
            },
        );

        let mut response = LoadSessionResponse::new().modes(mode_state);
        if let Some(ms) = model_state {
            response = response.models(ms);
        }
        if let Some(co) = config_options {
            response = response.config_options(co);
        }
        if let Some(usage_update) = initial_usage_update {
            cx.send_notification(SessionNotification::new(
                args.session_id.clone(),
                SessionUpdate::UsageUpdate(usage_update),
            ))?;
        }
        Self::send_available_commands_update(cx, &args.session_id, &args.cwd)?;
        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            "perf: load_session done (agent setup continues in background)"
        );
        Ok(response)
    }

    pub(super) async fn on_prompt(
        &self,
        cx: &ConnectionTo<Client>,
        args: PromptRequest,
    ) -> Result<PromptResponse, agent_client_protocol::Error> {
        // The ACP session_id IS the thread ID.
        let session_id = args.session_id.0.to_string();
        let sid = sid_short(&session_id);
        let t_start = std::time::Instant::now();

        let cancel_token = CancellationToken::new();
        let agent = self
            .get_session_agent(&session_id, Some(cancel_token.clone()))
            .await?;

        let user_message = Self::convert_acp_prompt_to_message(&args.prompt);

        let message_text = user_message.as_concat_text();
        if let Some(parsed) = crate::agents::execute_commands::parse_slash_command(&message_text) {
            let full_command = format!("/{}", parsed.command);

            if !Self::is_builtin_agent_command(parsed.command) {
                if let Some(recipe_path) =
                    crate::slash_commands::recipe_slash_command::get_recipe_for_command(
                        &full_command,
                    )
                {
                    if recipe_path.exists() {
                        cx.send_notification(SessionNotification::new(
                            args.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                ContentBlock::Text(TextContent::new(format!(
                                    "Running recipe: {}",
                                    full_command
                                ))),
                            )),
                        ))?;
                    }
                }
            }
        }

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let mut stream = agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await
            .internal_err_ctx("Error getting agent reply")?;

        let mut was_cancelled = false;
        let mut first_event_logged = false;
        let mut event_count: u32 = 0;
        // Streaming chain buffer: tracks consecutive tool requests across
        // `AgentEvent::Message` events so chains that span multiple rows are
        // still registered. Sequential tool use (Bedrock/Anthropic) yields
        // request → response → request → response across separate
        // assistant/user messages, so tool responses are chain-neutral; only
        // non-tool content (text, thinking, image, etc.) breaks the run.
        // Holds `(tool_call_id, message_id_of_owning_row)` in arrival order;
        // re-registered eagerly each time a request arrives so
        // `handle_tool_response` finds the chain when subsequent responses
        // are processed.
        let mut chain_buffer: Vec<(String, String)> = Vec::new();

        while let Some(event) = stream.next().await {
            if cancel_token.is_cancelled() {
                was_cancelled = true;
                break;
            }
            event_count += 1;
            if !first_event_logged {
                debug!(
                    target: "perf",
                    sid = %sid,
                    ttft_ms = t_start.elapsed().as_millis() as u64,
                    "perf: prompt first stream event (time-to-first-token from prompt start)"
                );
                first_event_logged = true;
            }

            match event {
                Ok(crate::agents::AgentEvent::Message(message)) => {
                    // Agent persists messages via session_manager.add_message() internally.
                    let stored_message_id = message.id.clone();

                    let mut sessions = self.sessions.lock().await;
                    let session = sessions.get_mut(&session_id).ok_or_else(|| {
                        agent_client_protocol::Error::invalid_params()
                            .data(format!("Session not found: {}", session_id))
                    })?;

                    for content_item in &message.content {
                        match content_item {
                            MessageContent::ToolRequest(tr) => {
                                if let Some(msg_id) = stored_message_id.as_deref() {
                                    chain_buffer.push((tr.id.clone(), msg_id.to_string()));
                                    // Re-register eagerly so the chain is in
                                    // place by the time the matching
                                    // `tool_response` triggers
                                    // `maybe_summarize_chain` (sequential
                                    // tool use interleaves request/response
                                    // events).
                                    extend_chain_membership(
                                        &chain_buffer,
                                        &mut session.chain_membership,
                                    );
                                }
                            }
                            MessageContent::ToolResponse(_) => {
                                // Chain-neutral: a response between two
                                // requests doesn't break the run, matching
                                // the frontend's `groupContentSections`.
                            }
                            _ => {
                                // Text, thinking, image, etc. end the run.
                                chain_buffer.clear();
                            }
                        }

                        self.handle_message_content(
                            content_item,
                            &args.session_id,
                            &session_id,
                            stored_message_id.as_deref(),
                            &agent,
                            session,
                            cx,
                        )
                        .await?;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    return Err(agent_client_protocol::Error::internal_error()
                        .data(format!("Error in agent response stream: {}", e)));
                }
            }
        }

        {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                // Final safety net: in case the stream ended without any
                // chain-breaking content, make sure a multi-tool buffer is
                // registered. (Eager registration during the loop usually
                // covers this.)
                extend_chain_membership(&chain_buffer, &mut session.chain_membership);
                session.cancel_token = None;
            }
        }

        let session = self
            .session_manager
            .get_session(&session_id, false)
            .await
            .internal_err_ctx("Failed to load session")?;
        let provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let usage_update =
            build_usage_update(&session, provider.get_model_config().context_limit());
        cx.send_notification(SessionNotification::new(
            args.session_id.clone(),
            SessionUpdate::UsageUpdate(usage_update),
        ))?;

        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            events = event_count,
            cancelled = was_cancelled,
            "perf: prompt done"
        );
        let stop_reason = if was_cancelled {
            StopReason::Cancelled
        } else {
            StopReason::EndTurn
        };

        let mut response = PromptResponse::new(stop_reason);
        if let Some(usage) = build_prompt_usage(&session) {
            response = response.usage(usage);
        }
        Ok(response)
    }

    pub(super) async fn on_cancel(
        &self,
        args: CancelNotification,
    ) -> Result<(), agent_client_protocol::Error> {
        debug!(?args, "cancel request");

        let session_id = args.session_id.0.to_string();
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(&session_id) {
            if let Some(ref token) = session.cancel_token {
                info!(session_id = %session_id, "prompt cancelled");
                token.cancel();
            }
        } else {
            warn!(session_id = %session_id, "cancel request for unknown session");
        }

        Ok(())
    }

    pub(super) async fn on_set_model(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> Result<SetSessionModelResponse, agent_client_protocol::Error> {
        let config = self.config()?;
        let agent = self.get_session_agent_provider_ready(session_id).await?;
        let current_provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let provider_name = current_provider.get_name().to_string();
        let current_model_config = current_provider.get_model_config();
        let extensions =
            EnabledExtensionsState::for_session(&self.session_manager, session_id, &config).await;
        let model_config = crate::model::ModelConfig::new(model_id)
            .invalid_params_err_ctx("Invalid model config")?
            .with_canonical_limits(&provider_name);
        let model_config =
            with_preserved_session_request_params(model_config, Some(&current_model_config), None);
        let session = self
            .session_manager
            .get_session(session_id, false)
            .await
            .internal_err_ctx("Failed to get session")?;
        let provider = self
            .create_provider(
                &provider_name,
                model_config,
                extensions,
                Some(session.working_dir),
            )
            .await
            .internal_err_ctx("Failed to create provider")?;
        agent
            .update_provider(provider, session_id)
            .await
            .internal_err_ctx("Failed to update provider")?;
        let mode = agent.goose_mode().await;
        agent
            .update_goose_mode(mode, session_id)
            .await
            .internal_err_ctx("Failed to propagate mode")?;
        // model_config is already updated on the session by the agent's update_provider call.
        Ok(SetSessionModelResponse::new())
    }

    pub(super) async fn build_config_update(
        &self,
        session_id: &SessionId,
    ) -> Result<(SessionNotification, Vec<SessionConfigOption>), agent_client_protocol::Error> {
        let session = self
            .session_manager
            .get_session(&session_id.0, false)
            .await
            .internal_err()?;
        let agent = self.get_session_agent_provider_ready(&session_id.0).await?;
        let provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let provider_name = provider.get_name().to_string();
        let current_model = provider.get_model_config().model_name.clone();
        let goose_mode = agent.goose_mode().await;
        let inventory = self
            .provider_inventory
            .entry_for_provider(&provider_name)
            .await
            .internal_err()?;
        let Some(inventory) = inventory else {
            return Err(agent_client_protocol::Error::internal_error()
                .data(format!("Unknown provider inventory: {}", provider_name)));
        };
        let model_state = build_model_state(current_model.as_str(), &inventory);
        let mode_state = build_mode_state(goose_mode)?;
        let provider_options = build_provider_options(Some(&provider_name)).await;
        let config_options = build_config_options(
            &mode_state,
            &model_state,
            session_provider_selection(&session),
            provider_options,
        );
        let notification = SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ConfigOptionUpdate(ConfigOptionUpdate::new(config_options.clone())),
        );
        Ok((notification, config_options))
    }

    pub(super) async fn on_set_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<SetSessionModeResponse, agent_client_protocol::Error> {
        let mode = mode_id.parse::<GooseMode>().map_err(|_| {
            agent_client_protocol::Error::invalid_params()
                .data(format!("Invalid mode: {}", mode_id))
        })?;

        let agent = self.get_session_agent_provider_ready(session_id).await?;
        agent
            .update_goose_mode(mode, session_id)
            .await
            .internal_err_ctx("Failed to update mode")?;

        // goose_mode is already updated on the session above.

        Ok(SetSessionModeResponse::new())
    }

    pub(super) async fn update_provider(
        &self,
        session_id: &str,
        provider_name: &str,
        model_name: Option<&str>,
        context_limit: Option<usize>,
        request_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<(), agent_client_protocol::Error> {
        let config = self.config()?;
        let agent = self.get_session_agent_provider_ready(session_id).await?;
        let current_provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let current_provider_name = current_provider.get_name();
        let current_model_config = current_provider.get_model_config();
        let current_model = current_model_config.model_name.clone();
        let has_default_overrides =
            model_name.is_some() || context_limit.is_some() || request_params.is_some();
        let use_default_provider = provider_name == DEFAULT_PROVIDER_ID;
        let resolved_provider_name = if use_default_provider {
            config
                .get_goose_provider()
                .internal_err_ctx("Failed to resolve default provider from config")?
        } else {
            provider_name.to_string()
        };
        let is_changing_provider = resolved_provider_name != current_provider_name;
        let default_model = if let Some(model_name) = model_name {
            model_name.to_string()
        } else if use_default_provider {
            config
                .get_goose_model()
                .internal_err_ctx("Failed to resolve default model from config")?
        } else if is_changing_provider {
            ACP_CURRENT_MODEL.to_string()
        } else {
            current_model
        };
        let model = model_name.unwrap_or(&default_model);
        let mut model_config = crate::model::ModelConfig::new(model)
            .invalid_params_err_ctx("Invalid model config")?
            .with_canonical_limits(&resolved_provider_name)
            .with_context_limit(context_limit);
        model_config = with_preserved_session_request_params(
            model_config,
            (!is_changing_provider).then_some(&current_model_config),
            request_params,
        );

        let extensions =
            EnabledExtensionsState::for_session(&self.session_manager, session_id, &config).await;
        let session = self
            .session_manager
            .get_session(session_id, false)
            .await
            .internal_err_ctx("Failed to get session")?;
        let new_provider = self
            .create_provider(
                &resolved_provider_name,
                model_config,
                extensions,
                Some(session.working_dir),
            )
            .await
            .internal_err_ctx("Failed to create provider")?;
        agent
            .update_provider(new_provider, session_id)
            .await
            .internal_err_ctx("Failed to update provider")?;
        let mode = agent.goose_mode().await;
        agent
            .update_goose_mode(mode, session_id)
            .await
            .internal_err_ctx("Failed to propagate mode")?;
        let provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;

        // provider_name is already updated on the session by the agent's update_provider call.

        if use_default_provider {
            let update = self
                .session_manager
                .update(session_id)
                .provider_name(DEFAULT_PROVIDER_ID);
            if has_default_overrides {
                update
                    .model_config(provider.get_model_config())
                    .apply()
                    .await
                    .internal_err_ctx("Failed to persist default provider selection overrides")?;
            } else {
                update
                    .clear_model_config()
                    .apply()
                    .await
                    .internal_err_ctx("Failed to persist default provider selection")?;
            }
        }
        Ok(())
    }

    pub(super) async fn on_list_sessions(
        &self,
        req: ListSessionsRequest,
    ) -> Result<ListSessionsResponse, agent_client_protocol::Error> {
        if let Some(cwd) = req.cwd.as_deref() {
            if !cwd.is_absolute() {
                return Err(agent_client_protocol::Error::invalid_params()
                    .data("cwd must be an absolute path"));
            }
        }

        let cwd = req.cwd.as_deref();
        let cursor =
            decode_session_list_cursor(req.cursor.as_deref(), cwd, &ACP_SESSION_LIST_TYPES)?;

        // ACP clients see their own (Acp) sessions plus legacy User/Scheduled ones.
        let page = self
            .session_manager
            .list_nonempty_sessions_by_types_paged(
                &ACP_SESSION_LIST_TYPES,
                cwd,
                cursor.as_ref(),
                SESSION_LIST_PAGE_SIZE,
            )
            .await
            .internal_err()?;
        let session_infos: Vec<SessionInfo> = page
            .sessions
            .into_iter()
            .map(|s| {
                let meta = session_meta(&s);
                SessionInfo::new(SessionId::new(s.id), s.working_dir)
                    .title(s.name)
                    .updated_at(s.updated_at.to_rfc3339())
                    .meta(meta)
            })
            .collect();
        let next_cursor = page
            .next_cursor
            .as_ref()
            .map(|cursor| encode_session_list_cursor(cursor, cwd, &ACP_SESSION_LIST_TYPES))
            .transpose()?;
        Ok(ListSessionsResponse::new(session_infos).next_cursor(next_cursor))
    }

    pub(super) async fn on_fork_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: ForkSessionRequest,
    ) -> Result<ForkSessionResponse, agent_client_protocol::Error> {
        validate_absolute_cwd(&args.cwd)?;
        let source_session_id = &*args.session_id.0;

        let new_session = self
            .session_manager
            .copy_session(source_session_id, "Fork".to_string())
            .await
            .internal_err()?;
        let new_session_id = new_session.id.clone();

        // Update working dir for the fork.
        self.session_manager
            .update(&new_session_id)
            .working_dir(args.cwd.clone())
            .apply()
            .await
            .internal_err()?;

        let goose_session = self
            .session_manager
            .get_session(&new_session_id, false)
            .await
            .internal_err()?;

        let (agent_tx, agent_rx) = tokio::sync::watch::channel::<AgentSetupSignal>(None);

        let acp_session = GooseAcpSession {
            agent: AgentHandle::Loading(agent_rx),
            tool_requests: HashMap::new(),
            chain_membership: HashMap::new(),
            responded_tool_ids: HashSet::new(),
            summarized_chains: HashSet::new(),
            cancel_token: None,
            pending_working_dir: None,
        };
        self.sessions
            .lock()
            .await
            .insert(new_session_id.clone(), acp_session);

        let mode_state = build_mode_state(self.goose_mode)?;
        let resolved = resolve_provider_and_model(&self.config_dir, &goose_session).await;
        let (model_state, config_options, prebuilt_provider) = self
            .prepare_session_init_config(&resolved, &mode_state, &goose_session)
            .await;

        let acp_session_id = SessionId::new(new_session_id.clone());

        self.spawn_agent_setup(
            cx,
            agent_tx,
            AgentSetupRequest {
                session_id: acp_session_id.clone(),
                goose_session,
                mcp_servers: args.mcp_servers,
                resolved_provider: resolved.ok(),
                prebuilt_provider,
            },
        );

        let meta = session_meta(&new_session);

        let mut response = ForkSessionResponse::new(acp_session_id.clone())
            .modes(mode_state)
            .meta(meta);

        if let Some(ms) = model_state {
            response = response.models(ms);
        }
        if let Some(co) = config_options {
            response = response.config_options(co);
        }
        Self::send_available_commands_update(cx, &acp_session_id, &args.cwd)?;
        Ok(response)
    }

    pub(super) async fn on_close_session(
        &self,
        session_id: &str,
    ) -> Result<CloseSessionResponse, agent_client_protocol::Error> {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(ref token) = session.cancel_token {
                token.cancel();
            }
        }
        sessions.remove(session_id);
        info!(session_id = %session_id, "ACP session closed");
        Ok(CloseSessionResponse::new())
    }
}
