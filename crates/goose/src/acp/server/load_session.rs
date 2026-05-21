use super::*;

impl GooseAcpAgent {
    /// Legacy `on_load_session` implementation. Preserved verbatim from
    /// `server.rs` as the kill-switch fallback during the migration to the
    /// inline implementation.
    ///
    /// Do not modify the logic here. Any change breaks the "flip env var to
    /// revert" guarantee that makes this safe to deploy.
    pub(super) async fn on_load_session_legacy(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "load session request");

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
        let initial_usage_updates = resolved
            .as_ref()
            .ok()
            .map(|(_, mc)| {
                build_usage_updates(&args.session_id, &goose_session, mc.context_limit())
            })
            .or_else(|| {
                goose_session.model_config.as_ref().map(|mc| {
                    build_usage_updates(&args.session_id, &goose_session, mc.context_limit())
                })
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
        if let Some(updates) = initial_usage_updates {
            cx.send_notification(updates.custom)?;
            // Legacy ACP notification — emitted alongside the custom one for
            // backwards compatibility. Remove once all known clients have
            // migrated to `_goose/session/update`.
            cx.send_notification(SessionNotification::new(
                args.session_id.clone(),
                SessionUpdate::UsageUpdate(updates.legacy),
            ))?;
        }

        self.send_available_commands_update(cx, &args.session_id)
            .await?;

        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            "perf: load_session done (agent setup continues in background)"
        );
        Ok(response)
    }
}
