use super::*;

fn replay_audience_annotations(audience: &[Role]) -> Annotations {
    Annotations::new().audience(
        audience
            .iter()
            .map(|role| match role {
                Role::Assistant => agent_client_protocol::schema::Role::Assistant,
                Role::User => agent_client_protocol::schema::Role::User,
            })
            .collect::<Vec<_>>(),
    )
}

fn send_replay_content_chunk(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    message: &Message,
    content: ContentBlock,
) -> std::result::Result<(), agent_client_protocol::Error> {
    let chunk = ContentChunk::new(content).meta(replay_message_meta(message));
    let update = match message.role {
        Role::User => SessionUpdate::UserMessageChunk(chunk),
        Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
    };
    cx.send_notification(SessionNotification::new(session_id.clone(), update))
}

fn replay_conversation_to_client(
    cx: &ConnectionTo<Client>,
    session: &Session,
    supports_goose_custom_notifications: bool,
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
                        tc = tc.annotations(replay_audience_annotations(audience));
                    }
                    send_replay_content_chunk(cx, &session_id, message, ContentBlock::Text(tc))?;
                }
                MessageContent::Image(image) => {
                    let mut image_content =
                        ImageContent::new(image.data.clone(), image.mime_type.clone());
                    if let Some(audience) = image.audience() {
                        image_content =
                            image_content.annotations(replay_audience_annotations(audience));
                    }
                    send_replay_content_chunk(
                        cx,
                        &session_id,
                        message,
                        ContentBlock::Image(image_content),
                    )?;
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
                                supports_goose_custom_notifications,
                                session_id.0.as_ref(),
                                InteractionUpdate {
                                    interaction: Interaction::Elicitation {
                                        id: id.clone(),
                                        state: InteractionState::Pending,
                                        message: Some(elicitation_message.clone()),
                                        requested_schema: Some(requested_schema.clone()),
                                    },
                                    meta: Some(serde_json::Value::Object(replay_message_meta(
                                        message,
                                    ))),
                                },
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
    pub(super) async fn handle_load_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "load session request");
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

        session = self
            .prepare_session_for_activation(session, args.cwd.clone(), args.mcp_servers, true)
            .await?;

        let replay_tool_requests = replay_conversation_to_client(
            cx,
            &session,
            self.supports_goose_custom_notifications(),
        )?;
        let (agent, extension_results) = self.prepare_acp_session_agent(cx, &session).await?;
        self.register_acp_session(session_id_str.clone(), agent.clone(), replay_tool_requests)
            .await;

        session = self
            .session_manager
            .get_session(&session_id_str, true)
            .await
            .internal_err_ctx("Failed to reload session")?;

        agent
            .extension_manager
            .update_working_dir(&session.working_dir)
            .await;

        let (mode_state, model_state, config_options) =
            build_session_setup_config(&self.provider_inventory, &session).await?;

        send_session_setup_notifications(cx, &session, self.supports_goose_custom_notifications())?;

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
            "perf: load_session_refactor done"
        );
        Ok(response)
    }
}
