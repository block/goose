use super::*;

impl Agent {
    pub(super) async fn prepare_reply_context(
        &self,
        session_id: &str,
        unfixed_conversation: Conversation,
        working_dir: &std::path::Path,
    ) -> Result<ReplyContext> {
        let unfixed_messages = unfixed_conversation.messages().clone();
        let (conversation, issues) = fix_conversation(unfixed_conversation.clone());
        if !issues.is_empty() {
            debug!(
                "Conversation issue fixed: {}",
                debug_conversation_fix(
                    unfixed_messages.as_slice(),
                    conversation.messages(),
                    &issues
                )
            );
        }
        let initial_messages = conversation.messages().clone();

        let (tools, toolshim_tools, system_prompt) = self
            .prepare_tools_and_prompt(session_id, working_dir)
            .await?;

        let goose_mode = *self.current_goose_mode.lock().await;

        if goose_mode == GooseMode::SmartApprove {
            self.tool_inspection_manager.apply_tool_annotations(&tools);
        }

        let tool_call_cut_off = match Config::global().get_param::<usize>("GOOSE_TOOL_CALL_CUTOFF")
        {
            Ok(v) => v,
            Err(_) => {
                let context_limit = self
                    .provider()
                    .await
                    .map(|p| p.get_model_config().context_limit())
                    .unwrap_or(crate::model::DEFAULT_CONTEXT_LIMIT);
                let compaction_threshold = Config::global()
                    .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
                    .unwrap_or(crate::context_mgmt::DEFAULT_COMPACTION_THRESHOLD);
                crate::context_mgmt::compute_tool_call_cutoff(context_limit, compaction_threshold)
            }
        };

        Ok(ReplyContext {
            conversation,
            tools,
            toolshim_tools,
            system_prompt,
            goose_mode,
            tool_call_cut_off,
            initial_messages,
        })
    }

    pub(super) async fn categorize_tools(
        &self,
        response: &Message,
        tools: &[rmcp::model::Tool],
        suppress_replayed_thinking: bool,
    ) -> ToolCategorizeResult {
        // Categorize tool requests
        let (frontend_requests, remaining_requests, filtered_response) = self
            .categorize_tool_requests(response, tools, suppress_replayed_thinking)
            .await;

        ToolCategorizeResult {
            frontend_requests,
            remaining_requests,
            filtered_response,
        }
    }

    pub(super) async fn handle_approved_and_denied_tools(
        &self,
        permission_check_result: &PermissionCheckResult,
        request_to_response_map: &mut HashMap<String, Message>,
        cancel_token: Option<tokio_util::sync::CancellationToken>,
        session: &Session,
    ) -> Result<Vec<(String, ToolStream)>> {
        let mut tool_futures: Vec<(String, ToolStream)> = Vec::new();

        // Handle pre-approved and read-only tools
        for request in &permission_check_result.approved {
            if let Ok(tool_call) = request.tool_call.clone() {
                let (req_id, tool_result) = self
                    .dispatch_tool_call(
                        tool_call,
                        request.id.clone(),
                        cancel_token.clone(),
                        session,
                    )
                    .await;

                tool_futures.push((
                    req_id,
                    match tool_result {
                        Ok(result) => tool_stream(
                            result
                                .notification_stream
                                .unwrap_or_else(|| Box::new(stream::empty())),
                            result.result,
                        ),
                        Err(e) => {
                            tool_stream(Box::new(stream::empty()), futures::future::ready(Err(e)))
                        }
                    },
                ));
            }
        }

        Self::handle_denied_tools(permission_check_result, request_to_response_map);
        Ok(tool_futures)
    }

    fn handle_denied_tools(
        permission_check_result: &PermissionCheckResult,
        request_to_response_map: &mut HashMap<String, Message>,
    ) {
        for request in &permission_check_result.denied {
            if let Some(response) = request_to_response_map.get_mut(&request.id) {
                response.add_tool_response_with_metadata(
                    request.id.clone(),
                    Ok(CallToolResult::error(vec![rmcp::model::Content::text(
                        DECLINED_RESPONSE,
                    )])),
                    request.metadata.as_ref(),
                );
            }
        }
    }

    #[instrument(
        skip(self, user_message, session_config, cancel_token),
        fields(user_message, trace_input, session.id = %session_config.id)
    )]
    pub async fn reply(
        &self,
        user_message: Message,
        session_config: SessionConfig,
        cancel_token: Option<CancellationToken>,
    ) -> Result<BoxStream<'_, Result<AgentEvent>>> {
        let session_manager = self.config.session_manager.clone();

        let message_text_for_trace = user_message.as_concat_text();
        tracing::Span::current().record("user_message", message_text_for_trace.as_str());
        tracing::Span::current().record("trace_input", message_text_for_trace.as_str());

        for content in &user_message.content {
            if let MessageContent::ActionRequired(action_required) = content {
                if let ActionRequiredData::ElicitationResponse { id, user_data } =
                    &action_required.data
                {
                    // Surface stale/cancelled/timed-out elicitations as a hard
                    // error so callers (e.g. the HTTP handler) can propagate
                    // failure to the client instead of silently reporting
                    // success while the blocked tool call stays unblocked.
                    // The success path returns an empty stream; an Err here
                    // makes the contract: Ok(empty) on accept, Err on reject.
                    ActionRequiredManager::global()
                        .submit_response(id.clone(), user_data.clone())
                        .await
                        .map_err(|e| {
                            error!("Failed to submit elicitation response: {}", e);
                            anyhow!("Failed to submit elicitation response: {}", e)
                        })?;
                    session_manager
                        .add_message(&session_config.id, &user_message)
                        .await?;
                    return Ok(Box::pin(futures::stream::empty()));
                }
            }
        }

        let message_text = user_message.as_concat_text();

        if self
            .hook_manager
            .has_hooks(crate::hooks::HookEvent::UserPromptSubmit)
        {
            let ctx = crate::hooks::HookContext::new(
                crate::hooks::HookEvent::UserPromptSubmit,
                &session_config.id,
            )
            .with_message(message_text.clone());
            self.hook_manager
                .emit(crate::hooks::HookEvent::UserPromptSubmit, ctx)
                .await;
        }

        let command_result = self
            .execute_command(&message_text, &session_config.id)
            .await;

        match command_result {
            Err(e) => {
                let error_message = Message::assistant()
                    .with_text(e.to_string())
                    .with_visibility(true, false);
                return Ok(Box::pin(stream::once(async move {
                    Ok(AgentEvent::Message(error_message))
                })));
            }
            Ok(Some(response)) if response.role == rmcp::model::Role::Assistant => {
                session_manager
                    .add_message(
                        &session_config.id,
                        &user_message.clone().with_visibility(true, false),
                    )
                    .await?;
                session_manager
                    .add_message(
                        &session_config.id,
                        &response.clone().with_visibility(true, false),
                    )
                    .await?;

                // Check if this was a command that modifies conversation history
                let modifies_history = crate::agents::execute_commands::COMPACT_TRIGGERS
                    .contains(&message_text.trim())
                    || message_text.trim() == "/clear";

                return Ok(Box::pin(async_stream::try_stream! {
                    yield AgentEvent::Message(user_message);
                    yield AgentEvent::Message(response);

                    // After commands that modify history, notify UI that history was replaced
                    if modifies_history {
                        let updated_session = session_manager.get_session(&session_config.id, true)
                            .await
                            .map_err(|e| anyhow!("Failed to fetch updated session: {}", e))?;
                        let updated_conversation = updated_session
                            .conversation
                            .ok_or_else(|| anyhow!("Session has no conversation after history modification"))?;
                        yield AgentEvent::HistoryReplaced(updated_conversation);
                    }
                }));
            }
            Ok(Some(resolved_message)) => {
                session_manager
                    .add_message(
                        &session_config.id,
                        &user_message.clone().with_visibility(true, false),
                    )
                    .await?;
                session_manager
                    .add_message(
                        &session_config.id,
                        &resolved_message.clone().with_visibility(false, true),
                    )
                    .await?;
            }
            Ok(None) => {
                session_manager
                    .add_message(&session_config.id, &user_message)
                    .await?;
            }
        }
        let session = session_manager
            .get_session(&session_config.id, true)
            .await?;
        let conversation = session
            .conversation
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Session {} has no conversation", session_config.id))?;

        let needs_auto_compact = check_if_compaction_needed(
            self.provider().await?.as_ref(),
            &conversation,
            None,
            &session,
        )
        .await?;

        let conversation_to_compact = conversation.clone();

        Ok(Box::pin(async_stream::try_stream! {
            let final_conversation = if !needs_auto_compact {
                conversation
            } else {
                let config = Config::global();
                let threshold = config
                    .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
                    .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);
                let threshold_percentage = (threshold * 100.0) as u32;

                let inline_msg = format!(
                    "Exceeded auto-compact threshold of {}%. Performing auto-compaction...",
                    threshold_percentage
                );

                yield AgentEvent::Message(
                    Message::assistant().with_system_notification(
                        SystemNotificationType::InlineMessage,
                        inline_msg,
                    )
                );

                yield AgentEvent::Message(
                    Message::assistant().with_system_notification(
                        SystemNotificationType::ThinkingMessage,
                        super::COMPACTION_THINKING_TEXT,
                    )
                );

                match compact_messages(
                    self.provider().await?.as_ref(),
                    &session_config.id,
                    &conversation_to_compact,
                    false,
                )
                .await
                {
                    Ok((compacted_conversation, summarization_usage)) => {
                        session_manager.replace_conversation(&session_config.id, &compacted_conversation).await?;
                        self.update_session_metrics(&session_config.id, session_config.schedule_id.clone(), &summarization_usage, true).await?;

                        yield AgentEvent::HistoryReplaced(compacted_conversation.clone());

                        yield AgentEvent::Message(
                            Message::assistant().with_system_notification(
                                SystemNotificationType::InlineMessage,
                                "Compaction complete",
                            )
                        );

                        compacted_conversation
                    }
                    Err(e) => {
                        yield AgentEvent::Message(
                            Message::assistant().with_text(
                                format!("Ran into this error trying to compact: {e}.\n\nPlease try again or create a new session")
                            )
                        );
                        return;
                    }
                }
            };

            let mut reply_stream = self.reply_internal(final_conversation, session_config, session, cancel_token).await?;
            while let Some(event) = reply_stream.next().await {
                yield event?;
            }
        }))
    }

    pub(super) async fn reply_internal(
        &self,
        conversation: Conversation,
        session_config: SessionConfig,
        session: Session,
        cancel_token: Option<CancellationToken>,
    ) -> Result<BoxStream<'_, Result<AgentEvent>>> {
        let context = self
            .prepare_reply_context(&session.id, conversation, session.working_dir.as_path())
            .await?;
        let ReplyContext {
            mut conversation,
            mut tools,
            mut toolshim_tools,
            mut system_prompt,
            tool_call_cut_off,
            goose_mode,
            initial_messages,
        } = context;

        if let Some(project_addendum) = self.load_project_instructions(&session).await {
            system_prompt = format!("{system_prompt}\n\n{project_addendum}");
        }

        self.reset_retry_attempts().await;

        let provider = self.provider().await?;
        let provider_name = provider.get_name().to_string();
        let requested_model = provider.get_model_config().model_name;
        let inference = provider
            .fetch_model_info(&requested_model)
            .await
            .ok()
            .and_then(|model_info| model_info.resolved_model)
            .map(|resolved_model| InferenceMetadata {
                provider: provider_name,
                requested_model,
                resolved_model: Some(resolved_model),
            });
        let session_manager = self.config.session_manager.clone();
        let session_id = session_config.id.clone();
        if !self.config.disable_session_naming {
            let provider = provider.clone();
            let manager_for_spawn = session_manager.clone();
            let session_name_update_tx = self.config.session_name_update_tx.clone();
            tokio::spawn(async move {
                match manager_for_spawn
                    .maybe_update_name(&session_id, provider)
                    .await
                {
                    Ok(Some(update)) => {
                        if let Some(tx) = session_name_update_tx {
                            if tx.send(update).is_err() {
                                warn!("Failed to publish generated session name");
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => warn!("Failed to generate session description: {}", e),
                }
            });
        }

        // Count tool calls present before this reply — everything added during
        // the reply loop is part of the current turn and should not be summarized.
        let pre_turn_tool_count = conversation
            .messages()
            .iter()
            .flat_map(|m| m.content.iter())
            .filter(|c| matches!(c, MessageContent::ToolRequest(_)))
            .count();

        let working_dir = session.working_dir.clone();
        let reply_stream_span = tracing::info_span!(target: "goose::agents::agent", "reply_stream", trace_output = tracing::field::Empty, session.id = %session_config.id);
        let inner = Box::pin(async_stream::try_stream! {
            let mut turns_taken = 0u32;
            let max_turns = session_config.max_turns.unwrap_or_else(|| {
                Config::global()
                    .get_param::<u32>("GOOSE_MAX_TURNS")
                    .unwrap_or(super::DEFAULT_MAX_TURNS)
            });
            let mut compaction_attempts = 0;
            let mut last_assistant_text = String::new();
            let mut goal_check_pending = false;
            let mut tool_pair_summarization_done = false;

            loop {
                if is_token_cancelled(&cancel_token) {
                    break;
                }

                {
                    let guard = self.final_output_tool.lock().await;
                    if let Some(ref output) = guard.as_ref().and_then(|fot| fot.final_output.clone()) {
                        yield AgentEvent::Message(Message::assistant().with_text(output));
                        break;
                    }
                }

                turns_taken += 1;
                if turns_taken > max_turns {
                    yield AgentEvent::Message(
                        Message::assistant().with_text(
                            "I've reached the maximum number of actions I can do without user input. Would you like me to continue?"
                        )
                    );
                    break;
                }

                let conversation_with_moim = super::super::moim::inject_moim(
                    &session_config.id,
                    conversation.clone(),
                    &self.extension_manager,
                    &working_dir,
                ).await;

                let mut stream = Self::stream_response_from_provider(
                    self.provider().await?,
                    &session_config.id,
                    &system_prompt,
                    conversation_with_moim.messages(),
                    &tools,
                    &toolshim_tools,
                ).await?;

                let current_turn_tool_count = conversation.messages().iter()
                    .flat_map(|m| m.content.iter())
                    .filter(|c| matches!(c, MessageContent::ToolRequest(_)))
                    .count()
                    .saturating_sub(pre_turn_tool_count);

                let tool_pair_summarization_task = if tool_pair_summarization_done {
                    None
                } else {
                    crate::context_mgmt::maybe_summarize_tool_pairs(
                        self.provider().await?,
                        session_config.id.clone(),
                        conversation.clone(),
                        tool_call_cut_off,
                        current_turn_tool_count,
                    )
                };

                let mut no_tools_called = true;
                let mut messages_to_add = Conversation::default();
                let mut tools_updated = false;
                let mut did_recovery_compact_this_iteration = false;
                let mut exit_chat = false;

                // Track whether this provider turn has already emitted visible
                // thinking so a later tool-call chunk can suppress replayed
                // reasoning without hiding final-only non-streaming thoughts.
                let mut surfaced_thinking_in_turn = false;

                while let Some(next) = stream.next().await {
                    if is_token_cancelled(&cancel_token) || exit_chat {
                        break;
                    }

                    match next {
                        Ok((response, usage)) => {
                            compaction_attempts = 0;

                            if let Some(ref usage) = usage {
                                self.update_session_metrics(&session_config.id, session_config.schedule_id.clone(), usage, false).await?;
                            }

                            if let Some(response) = response {
                                let ToolCategorizeResult {
                                    frontend_requests,
                                    remaining_requests,
                                    filtered_response,
                                } = self
                                    .categorize_tools(
                                        &response,
                                        &tools,
                                        surfaced_thinking_in_turn,
                                    )
                                    .await;

                                let filtered_response = if let Some(inference) = inference.as_ref() {
                                    filtered_response.with_inference(inference.clone())
                                } else {
                                    filtered_response
                                };
                                let response = if let Some(inference) = inference.as_ref() {
                                    response.with_inference(inference.clone())
                                } else {
                                    response
                                };

                                surfaced_thinking_in_turn |= filtered_response.content.iter().any(
                                    |content| {
                                        matches!(
                                            content,
                                            MessageContent::Thinking(_)
                                                | MessageContent::RedactedThinking(_)
                                        )
                                    },
                                );

                                yield AgentEvent::Message(filtered_response.clone());
                                tokio::task::yield_now().await;

                                let num_tool_requests = frontend_requests.len() + remaining_requests.len();
                                if num_tool_requests == 0 {
                                    let text = filtered_response.as_concat_text();
                                    if !text.is_empty() {
                                        last_assistant_text = text;
                                    }
                                    messages_to_add.push(response);
                                    continue;
                                }

                                let mut request_to_response_map = HashMap::new();
                                let mut request_metadata: HashMap<String, Option<ProviderMetadata>> = HashMap::new();
                                for request in frontend_requests.iter().chain(remaining_requests.iter()) {
                                    request_to_response_map.insert(request.id.clone(), Message::user().with_generated_id());
                                    request_metadata.insert(request.id.clone(), request.metadata.clone());
                                }

                                for request in frontend_requests.iter() {
                                    let response_msg = request_to_response_map.get_mut(&request.id)
                                        .ok_or_else(|| anyhow::anyhow!("missing response entry for request {}", request.id))?;
                                    let mut frontend_tool_stream = self.handle_frontend_tool_request(
                                        request,
                                        response_msg,
                                    );

                                    while let Some(msg) = frontend_tool_stream.try_next().await? {
                                        yield AgentEvent::Message(msg);
                                    }
                                }
                                if goose_mode == GooseMode::Chat {
                                    for request in remaining_requests.iter() {
                                        if let Some(response) = request_to_response_map.get_mut(&request.id) {
                                            response.add_tool_response_with_metadata(
                                                request.id.clone(),
                                                Ok(CallToolResult::success(vec![Content::text(CHAT_MODE_TOOL_SKIPPED_RESPONSE)])),
                                                request.metadata.as_ref(),
                                            );
                                        }
                                    }
                                } else {
                                    // Run all tool inspectors
                                    let inspection_results = self.tool_inspection_manager
                                        .inspect_tools(
                                            &session_config.id,
                                            &remaining_requests,
                                            conversation.messages(),
                                            goose_mode,
                                        )
                                        .await?;

                                    let permission_check_result = self.tool_inspection_manager
                                        .process_inspection_results_with_permission_inspector(
                                            &remaining_requests,
                                            &inspection_results,
                                        )
                                        .unwrap_or_else(|| {
                                            let mut result = PermissionCheckResult {
                                                approved: vec![],
                                                needs_approval: vec![],
                                                denied: vec![],
                                            };
                                            result.needs_approval.extend(remaining_requests.iter().cloned());
                                            result
                                        });

                                    // Track extension requests
                                    let mut enable_extension_request_ids = vec![];
                                    for request in &remaining_requests {
                                        if let Ok(tool_call) = &request.tool_call {
                                            if tool_call.name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                                                enable_extension_request_ids.push(request.id.clone());
                                            }
                                        }
                                    }

                                    let mut tool_futures = self.handle_approved_and_denied_tools(
                                        &permission_check_result,
                                        &mut request_to_response_map,
                                        cancel_token.clone(),
                                        &session,
                                    ).await?;

                                    {
                                        let mut tool_approval_stream = self.handle_approval_tool_requests(
                                            &permission_check_result.needs_approval,
                                            &mut tool_futures,
                                            &mut request_to_response_map,
                                            cancel_token.clone(),
                                            &session,
                                            &inspection_results,
                                        );

                                        while let Some(msg) = tool_approval_stream.try_next().await? {
                                            yield AgentEvent::Message(msg);
                                        }
                                    }

                                    let with_id = tool_futures
                                        .into_iter()
                                        .map(|(request_id, stream)| {
                                            stream.map(move |item| (request_id.clone(), item))
                                        })
                                        .collect::<Vec<_>>();

                                    let mut combined = stream::select_all(with_id);
                                    let mut all_install_successful = true;

                                    loop {
                                        if is_token_cancelled(&cancel_token) {
                                            break;
                                        }

                                        for msg in self.drain_elicitation_messages(&session_config.id).await {
                                            yield AgentEvent::Message(msg);
                                        }

                                        tokio::select! {
                                            biased;

                                            tool_item = combined.next() => {
                                                match tool_item {
                                                    Some((request_id, item)) => {
                                                        match item {
                                                            ToolStreamItem::Result(output) => {
                                                                if let Ok(ref call_result) = output {
                                                                    if let Some(ref meta) = call_result.meta {
                                                                        if let Some(notification_data) = meta.0.get("platform_notification") {
                                                                            if let Some(method) = notification_data.get("method").and_then(|v| v.as_str()) {
                                                                                let params = notification_data.get("params").cloned();
                                                                                let custom_notification = rmcp::model::CustomNotification::new(
                                                                                    method.to_string(),
                                                                                    params,
                                                                                );

                                                                                let server_notification = rmcp::model::ServerNotification::CustomNotification(custom_notification);
                                                                                yield AgentEvent::McpNotification((request_id.clone(), server_notification));
                                                                            }
                                                                        }
                                                                    }
                                                                }

                                                                if enable_extension_request_ids.contains(&request_id)
                                                                    && output.is_err()
                                                                {
                                                                    all_install_successful = false;
                                                                }
                                                                if let Some(response) = request_to_response_map.get_mut(&request_id) {
                                                                    let metadata = request_metadata.get(&request_id).and_then(|m| m.as_ref());
                                                                    response.add_tool_response_with_metadata(request_id, output, metadata);
                                                                }
                                                            }
                                                            ToolStreamItem::Message(msg) => {
                                                                yield AgentEvent::McpNotification((request_id, msg));
                                                            }
                                                        }
                                                    }
                                                    None => break,
                                                }
                                            }

                                            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                                                // Continue loop to drain elicitation messages
                                            }
                                        }
                                    }

                                    // check for remaining elicitation messages after all tools complete
                                    for msg in self.drain_elicitation_messages(&session_config.id).await {
                                        yield AgentEvent::Message(msg);
                                    }

                                    if all_install_successful && !enable_extension_request_ids.is_empty() {
                                        if let Err(e) = self.save_extension_state(&session_config).await {
                                            warn!("Failed to save extension state after runtime changes: {}", e);
                                        }
                                        tools_updated = true;
                                    }
                                }

                                // Preserve thinking/reasoning content from the original response
                                // Gemini (and other thinking models) require thinking to be echoed back
                                // Kimi/DeepSeek require reasoning_content on assistant tool call messages
                                let thinking_content: Vec<MessageContent> = response.content.iter()
                                    .filter(|c| matches!(c, MessageContent::Thinking(_)))
                                    .cloned()
                                    .collect();
                                if !thinking_content.is_empty() {
                                    let thinking_msg = Message::new(
                                        response.role.clone(),
                                        response.created,
                                        thinking_content,
                                    ).with_id(format!("msg_{}", Uuid::new_v4()));
                                    messages_to_add.push(thinking_msg);
                                }

                                // Collect reasoning content to attach to tool request messages
                                let reasoning_content: Vec<MessageContent> = response.content.iter()
                                    .filter(|c| matches!(c, MessageContent::Thinking(_)))
                                    .cloned()
                                    .collect();

                                for request in frontend_requests.iter().chain(remaining_requests.iter()) {
                                    if request.tool_call.is_ok() {
                                        let mut request_msg = Message::assistant()
                                            .with_id(format!("msg_{}", Uuid::new_v4()));

                                        // Providers like Kimi require reasoning_content on all assistant
                                        // messages with tool_calls when thinking mode is enabled.
                                        for rc in &reasoning_content {
                                            request_msg = request_msg.with_content(rc.clone());
                                        }

                                        request_msg = request_msg
                                            .with_tool_request_with_metadata(
                                                request.id.clone(),
                                                request.tool_call.clone(),
                                                request.metadata.as_ref(),
                                                request.tool_meta.clone(),
                                            );
                                        messages_to_add.push(request_msg);
                                        let final_response = request_to_response_map
                                            .remove(&request.id)
                                            .unwrap_or_else(|| Message::user().with_generated_id());
                                        yield AgentEvent::Message(final_response.clone());
                                        messages_to_add.push(final_response);
                                    } else {
                                        error!(
                                            "Tool call could not be parsed: {}",
                                            request.tool_call.as_ref().unwrap_err(),
                                        );
                                        yield AgentEvent::Message(
                                            Message::assistant().with_text(
                                                "A tool call could not be parsed — the response may have been truncated. Try breaking the task into smaller steps or resending your message."
                                            )
                                        );
                                        exit_chat = true;
                                        break;
                                    }
                                }

                                no_tools_called = false;
                                // Agent is actively working — re-check goal when it next finishes
                                goal_check_pending = false;
                            }
                        }
                        #[allow(unused_variables)]
                        Err(ref provider_err @ ProviderError::ContextLengthExceeded(_)) => {
                            #[cfg(feature = "telemetry")]
                            crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());
                            compaction_attempts += 1;

                            if compaction_attempts >= 2 {
                                error!("Context limit exceeded after compaction - prompt too large");
                                yield AgentEvent::Message(
                                    Message::assistant().with_system_notification(
                                        SystemNotificationType::InlineMessage,
                                        "Unable to continue: Context limit still exceeded after compaction. Try using a shorter message, a model with a larger context window, or start a new session."
                                    )
                                );
                                break;
                            }

                            yield AgentEvent::Message(
                                Message::assistant().with_system_notification(
                                    SystemNotificationType::InlineMessage,
                                    "Context limit reached. Compacting to continue conversation...",
                                )
                            );
                            yield AgentEvent::Message(
                                Message::assistant().with_system_notification(
                                    SystemNotificationType::ThinkingMessage,
                                    super::COMPACTION_THINKING_TEXT,
                                )
                            );

                            match compact_messages(
                                self.provider().await?.as_ref(),
                                &session_config.id,
                                &conversation,
                                false,
                            )
                            .await
                            {
                                Ok((compacted_conversation, usage)) => {
                                    session_manager.replace_conversation(&session_config.id, &compacted_conversation).await?;
                                    self.update_session_metrics(&session_config.id, session_config.schedule_id.clone(), &usage, true).await?;
                                    conversation = compacted_conversation;
                                    did_recovery_compact_this_iteration = true;
                                    yield AgentEvent::HistoryReplaced(conversation.clone());
                                    break;
                                }
                                Err(e) => {
                                    #[cfg(feature = "telemetry")]
                                    crate::posthog::emit_error("compaction_failed", &e.to_string());
                                    error!("Compaction failed: {}", e);
                                    yield AgentEvent::Message(
                                        Message::assistant().with_text(
                                            format!("Ran into this error trying to compact: {e}.\n\nPlease try again or create a new session")
                                        )
                                    );
                                    break;
                                }
                            }
                        }
                        Err(ref provider_err @ ProviderError::CreditsExhausted { details: _, ref top_up_url }) => {
                            #[cfg(feature = "telemetry")]
                            crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());
                            error!("Error: {}", provider_err);

                            let user_msg = if top_up_url.is_some() {
                                "Please add credits to your account, then resend your message to continue.".to_string()
                            } else {
                                "Please check your account with your provider to add more credits, then resend your message to continue.".to_string()
                            };

                            let notification_data = serde_json::json!({
                                "top_up_url": top_up_url,
                            });

                            yield AgentEvent::Message(
                                Message::assistant().with_system_notification_with_data(
                                    SystemNotificationType::CreditsExhausted,
                                    user_msg,
                                    notification_data,
                                )
                            );
                            break;
                        }
                        Err(ref provider_err @ ProviderError::NetworkError(_)) => {
                            #[cfg(feature = "telemetry")]
                            crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());
                            error!("Error: {}", provider_err);
                            yield AgentEvent::Message(
                                Message::assistant().with_text(
                                    format!("{provider_err}\n\nPlease resend your message to try again.")
                                )
                            );
                            break;
                        }
                        Err(ref provider_err) => {
                            #[cfg(feature = "telemetry")]
                            crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());
                            error!("Error: {}", provider_err);
                            yield AgentEvent::Message(
                                Message::assistant().with_text(
                                    format!("Ran into this error: {provider_err}.\n\nPlease retry if you think this is a transient or recoverable error.")
                                )
                            );
                            break;
                        }
                    }
                }
                if tools_updated {
                    (tools, toolshim_tools, system_prompt) =
                        self.prepare_tools_and_prompt(&session_config.id, &session.working_dir).await?;
                }

                {
                    let has_new_hints = self
                        .prompt_manager
                        .lock()
                        .await
                        .load_subdirectory_hints(&working_dir);
                    if has_new_hints && !tools_updated {
                        (tools, toolshim_tools, system_prompt) =
                            self.prepare_tools_and_prompt(&session_config.id, &session.working_dir).await?;
                    }
                }

                if no_tools_called {
                    // Lock, extract state, drop guard before branching — handle_retry_logic
                    // also locks final_output_tool and tokio::sync::Mutex is not reentrant.
                    let final_output = {
                        let guard = self.final_output_tool.lock().await;
                        guard.as_ref().map(|fot| fot.final_output.clone())
                    };

                    match final_output {
                        Some(None) => {
                            warn!("Final output tool has not been called yet. Continuing agent loop.");
                            let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                        }
                        Some(Some(output)) => {
                            let message = Message::assistant().with_text(output);
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                            exit_chat = true;
                        }
                        None if did_recovery_compact_this_iteration => {
                            // continue from last user message after recovery compact
                        }
                        None if self.goal.lock().await.is_some() && !goal_check_pending => {
                            goal_check_pending = true;
                            let goal = self.goal.lock().await.clone().unwrap();
                            let nudge = format!(
                                "Before finishing, check whether the following goal has been fully met:\n\n\
                                 **Goal:** {goal}\n\n\
                                 If not, continue working toward it."
                            );
                            let message = Message::user().with_text(&nudge)
                                .with_visibility(false, true);
                            messages_to_add.push(message);
                            yield AgentEvent::Message(
                                Message::assistant().with_system_notification(
                                    SystemNotificationType::InlineMessage,
                                    format!("Goal: {goal}"),
                                )
                            );
                        }

                        None if self.grind.lock().await.is_some() => {
                            let grind = self.grind.lock().await.clone().unwrap();
                            let nudge = format!(
                                "Keep working. The grind goal is not yet complete:\n\n\
                                 **Goal:** {grind}\n\n\
                                 Continue until it is fully done."
                            );
                            let message = Message::user().with_text(&nudge)
                                .with_visibility(false, true);
                            messages_to_add.push(message);
                            yield AgentEvent::Message(
                                Message::assistant().with_system_notification(
                                    SystemNotificationType::InlineMessage,
                                    format!("Grind: {grind}"),
                                )
                            );
                        }

                        None => {
                            self.set_goal(None).await;
                            self.set_grind(None).await;
                            match self.handle_retry_logic(&mut conversation, &session_config, &initial_messages).await {
                                Ok(should_retry) => {
                                    if should_retry {
                                        info!("Retry logic triggered, restarting agent loop");
                                        messages_to_add = Conversation::default();
                                        session_manager.replace_conversation(&session_config.id, &conversation).await?;
                                        yield AgentEvent::HistoryReplaced(conversation.clone());
                                    } else {
                                        exit_chat = true;
                                    }
                                }
                                Err(e) => {
                                    error!("Retry logic failed: {}", e);
                                    yield AgentEvent::Message(
                                        Message::assistant().with_text(
                                            format!("Retry logic encountered an error: {}", e)
                                        )
                                    );
                                    exit_chat = true;
                                }
                            }
                        }
                    }
                }

                if is_token_cancelled(&cancel_token) {
                    if let Some(ref task) = tool_pair_summarization_task {
                        task.abort();
                    }
                }

                if let Some(task) = tool_pair_summarization_task {
                    tool_pair_summarization_done = true;
                    if let Ok(summaries) = task.await {
                        for (summary_msg, tool_id) in summaries {
                            let matching_ids: Vec<String> = conversation.messages()
                                .iter()
                                .filter(|msg| {
                                    msg.id.is_some() && msg.content.iter().any(|c| match c {
                                        MessageContent::ToolRequest(req) => req.id == tool_id,
                                        MessageContent::ToolResponse(resp) => resp.id == tool_id,
                                        _ => false,
                                    })
                                })
                                .filter_map(|msg| msg.id.clone())
                                .collect();

                            if matching_ids.len() == 2 {
                                for id in &matching_ids {
                                    SessionManager::update_message_metadata(&session_config.id, id, |metadata| {
                                        metadata.with_agent_invisible()
                                    }).await?;
                                }
                                session_manager.add_message(&session_config.id, &summary_msg).await?;
                            } else {
                                warn!("Expected a tool request/reply pair, but found {} matching messages",
                                    matching_ids.len());
                            }
                        }
                    }
                }

                let messages_to_add = if let Some(ref inference) = inference {
                    Conversation::new_unvalidated(
                        messages_to_add
                            .into_iter()
                            .map(|message| message.with_inference_if_assistant(inference.clone())),
                    )
                } else {
                    messages_to_add
                };

                for msg in &messages_to_add {
                    session_manager.add_message(&session_config.id, msg).await?;
                }
                conversation.extend(messages_to_add);
                if exit_chat {
                    break;
                }

                tokio::task::yield_now().await;
            }

            if !last_assistant_text.is_empty() {
                tracing::Span::current().record("trace_output", last_assistant_text.as_str());
            }

            self.emit_hook(crate::hooks::HookEvent::Stop, &session_config.id).await;
        }.instrument(reply_stream_span));
        Ok(inner)
    }
}
