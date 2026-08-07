use super::message_meta::{
    content_chunk_for_message, merge_message_meta, populate_output_token_limit_content,
};
use super::tool_calls::conversion::{
    build_initial_tool_call_with_message_meta, tool_call_update_fields_from_response,
    trusted_update_meta,
};
use super::tool_calls::enrichment::tool_chain_summary;
use super::*;
use crate::conversation::Conversation;
use agent_client_protocol::schema::v1::ToolCall;

fn usize_from_meta(
    meta: Option<&Meta>,
    key: &str,
) -> Result<Option<usize>, agent_client_protocol::Error> {
    let Some(value) = meta.and_then(|meta| meta.get(key)) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }

    value.as_u64().map(|n| n as usize).map(Some).ok_or_else(|| {
        agent_client_protocol::Error::invalid_params()
            .data(format!("{key} must be a non-negative integer"))
    })
}

fn message_pagination_from_meta(
    meta: Option<&Meta>,
) -> Result<(Option<usize>, Option<usize>), agent_client_protocol::Error> {
    let limit = usize_from_meta(meta, "messageLimit")?;
    let offset = usize_from_meta(meta, "messageOffset")?;
    Ok((limit, offset))
}

/// Returns the slice of `messages` selected by `offset`/`limit`, counting back from
/// the most recent message. The returned messages stay in chronological order.
fn paginate_messages<T: Clone>(
    messages: &[T],
    limit: Option<usize>,
    offset: Option<usize>,
) -> Vec<T> {
    let total = messages.len();
    let offset = offset.unwrap_or(0).min(total);
    let end = total - offset;
    let start = match limit {
        Some(limit) => end.saturating_sub(limit),
        None => 0,
    };
    messages[start..end].to_vec()
}

fn replay_audience_annotations(audience: &[Role]) -> Annotations {
    Annotations::new().audience(
        audience
            .iter()
            .map(|role| match role {
                Role::Assistant => agent_client_protocol::schema::v1::Role::Assistant,
                Role::User => agent_client_protocol::schema::v1::Role::User,
            })
            .collect::<Vec<_>>(),
    )
}

fn messages_for_acp_replay(conversation: &Conversation) -> Vec<Message> {
    conversation
        .messages()
        .iter()
        .filter(|message| message.is_user_visible())
        .map(Message::user_visible_content)
        .map(|mut message| {
            populate_output_token_limit_content(&mut message);
            message
        })
        .filter(|message| !message.content.is_empty())
        .collect()
}

fn active_turn_messages(conversation: &Conversation) -> &[Message] {
    let messages = conversation.messages();
    messages
        .iter()
        .rposition(|message| {
            message.role == Role::User && message.is_user_visible() && !message.is_tool_response()
        })
        .map(|start| &messages[start..])
        .unwrap_or(messages)
}

fn send_replay_content_chunk(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    message: &Message,
    content: ContentBlock,
) -> std::result::Result<(), agent_client_protocol::Error> {
    let chunk = content_chunk_for_message(message, content);
    let update = match message.role {
        Role::User => SessionUpdate::UserMessageChunk(chunk),
        Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
    };
    cx.send_notification(SessionNotification::new(session_id.clone(), update))
}

fn build_replayed_tool_call(
    tool_request: &ToolRequest,
    message: &Message,
    client_requests_tool_call_label_enrichment: bool,
) -> ToolCall {
    let mut tool_call = build_initial_tool_call_with_message_meta(
        tool_request,
        message,
        client_requests_tool_call_label_enrichment,
    );

    if !client_requests_tool_call_label_enrichment {
        return tool_call;
    }

    let Some(chain_summary) = tool_request.generated_chain_summary() else {
        return tool_call;
    };
    let goose_meta = tool_call
        .meta
        .get_or_insert_default()
        .entry("goose".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !goose_meta.is_object() {
        *goose_meta = serde_json::Value::Object(serde_json::Map::new());
    }
    goose_meta
        .as_object_mut()
        .expect("goose metadata was initialized as an object")
        .extend([tool_chain_summary(&chain_summary)]);

    tool_call
}

fn replay_conversation_to_client(
    cx: &ConnectionTo<Client>,
    session: &Session,
    supports_goose_custom_notifications: bool,
    client_requests_tool_call_label_enrichment: bool,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<(), agent_client_protocol::Error> {
    let session_id = SessionId::new(session.id.clone());
    let tool_call_notifier = ToolCallNotifier::new(cx, &session_id);

    let messages = session
        .conversation
        .as_ref()
        .map(messages_for_acp_replay)
        .unwrap_or_default();
    let messages = if limit.is_some() || offset.is_some() {
        paginate_messages(&messages, limit, offset)
    } else {
        messages
    };

    let mut replay_tool_requests = HashMap::new();

    for message in &messages {
        for content_item in &message.content {
            match content_item {
                MessageContent::Text(text) => {
                    let mut tc = TextContent::new(text.text.clone());
                    if let Some(audience) =
                        text.annotations.as_ref().and_then(|a| a.audience.as_ref())
                    {
                        tc = tc.annotations(replay_audience_annotations(audience));
                    }
                    send_replay_content_chunk(cx, &session_id, message, ContentBlock::Text(tc))?;
                }
                MessageContent::Image(image) => {
                    let mut image_content =
                        ImageContent::new(image.data.clone(), image.mime_type.clone());
                    if let Some(audience) =
                        image.annotations.as_ref().and_then(|a| a.audience.as_ref())
                    {
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

                    let tool_call = build_replayed_tool_call(
                        tool_request,
                        message,
                        client_requests_tool_call_label_enrichment,
                    );

                    tool_call_notifier.send_initial(tool_call)?;
                }
                MessageContent::ToolResponse(tool_response) => {
                    let fields = tool_call_update_fields_from_response(
                        tool_response,
                        replay_tool_requests.get(&tool_response.id),
                        true,
                    );
                    let meta = trusted_update_meta(tool_response).unwrap_or_default();

                    let update =
                        ToolCallUpdate::new(ToolCallId::new(tool_response.id.clone()), fields)
                            .meta(merge_message_meta(meta, message));
                    tool_call_notifier.send_update(update)?;
                }
                MessageContent::Thinking(thinking) => {
                    cx.send_notification(SessionNotification::new(
                        session_id.clone(),
                        SessionUpdate::AgentThoughtChunk(content_chunk_for_message(
                            message,
                            ContentBlock::Text(TextContent::new(thinking.thinking.clone())),
                        )),
                    ))?;
                }
                MessageContent::Error(error) => {
                    send_replay_content_chunk(
                        cx,
                        &session_id,
                        message,
                        ContentBlock::Text(TextContent::new(error.message.clone())),
                    )?;
                }
                MessageContent::SystemNotification(_) => {}
                _ => {}
            }
        }

        if supports_goose_custom_notifications {
            if let Some(usage) = &message.metadata.usage {
                cx.send_notification(GooseSessionNotification {
                    session_id: session.id.clone(),
                    update: GooseSessionUpdate::MessageUsage(message_usage_update(
                        message.id.clone(),
                        usage,
                    )),
                })?;
            }
        }
    }

    Ok(())
}

impl GooseAcpAgent {
    fn resend_pending_tool_permissions(
        &self,
        cx: &ConnectionTo<Client>,
        agent: &Arc<Agent>,
        session: &Session,
    ) -> Result<(), agent_client_protocol::Error> {
        let session_id = SessionId::new(session.id.clone());
        let messages = session
            .conversation
            .as_ref()
            .map(active_turn_messages)
            .unwrap_or(&[]);

        let mut answered = HashSet::new();
        let mut responses = HashSet::new();
        let mut requests = Vec::new();

        for message in messages {
            for content in &message.content {
                match content {
                    MessageContent::ToolResponse(response) => {
                        answered.insert(response.id.clone());
                    }
                    MessageContent::ActionRequired(action) => match &action.data {
                        ActionRequiredData::ToolConfirmation {
                            id,
                            tool_name,
                            arguments,
                            prompt,
                        } => requests.push((
                            id.clone(),
                            tool_name.clone(),
                            arguments.clone(),
                            prompt.clone(),
                        )),
                        ActionRequiredData::ToolConfirmationResponse { id, .. } => {
                            responses.insert(id.clone());
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        for (id, tool_name, arguments, prompt) in requests {
            if answered.contains(&id) || responses.contains(&id) {
                continue;
            }
            self.handle_tool_permission_request(
                cx,
                agent,
                &session_id,
                id,
                tool_name,
                arguments,
                prompt,
            )?;
        }

        Ok(())
    }

    pub(super) async fn handle_load_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "load session request");
        validate_absolute_cwd(&args.cwd)?;
        let (message_limit, message_offset) = message_pagination_from_meta(args.meta.as_ref())?;

        let session_id_str = args.session_id.0.to_string();

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

        replay_conversation_to_client(
            cx,
            &session,
            self.supports_goose_custom_notifications(),
            self.requests_tool_call_label_enrichment(),
            message_limit,
            message_offset,
        )?;
        let (agent, extension_results) = self.prepare_acp_session_agent(cx, &session).await?;
        self.apply_session_recipe(&agent, &session).await?;
        self.register_acp_session(session_id_str.clone(), agent.clone())
            .await;
        self.resend_pending_tool_permissions(cx, &agent, &session)?;

        session = self
            .session_manager
            .get_session(&session_id_str, false)
            .await
            .internal_err_ctx("Failed to reload session")?;

        agent
            .extension_manager
            .update_working_dir(&session.working_dir)
            .await;

        let (mode_state, config_options) =
            build_session_setup_config(&self.provider_inventory, &session).await?;

        self.notify_session_setup(cx, &session).await?;

        let mut response = LoadSessionResponse::new().modes(mode_state);
        if let Some(co) = config_options {
            response = response.config_options(co);
        }

        response = response.meta(session_response_meta(&session, &extension_results));

        self.closed_session_ids.lock().await.remove(&session_id_str);
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolRequestParams;

    #[test]
    fn acp_replay_populates_only_empty_marked_assistant_messages() {
        let visible_message = Message::assistant()
            .with_text("visible")
            .with_id("msg_visible");
        let empty_message = Message::assistant().with_id("msg_empty");

        let mut marked_message = Message::assistant().with_id("msg_limited");
        marked_message.metadata.output_token_limit_reached = true;

        let mut marked_user_message = Message::user().with_id("msg_user");
        marked_user_message.metadata.output_token_limit_reached = true;

        let mut hidden_marked_message = Message::assistant()
            .with_id("msg_hidden")
            .with_visibility(false, false);
        hidden_marked_message.metadata.output_token_limit_reached = true;

        let conversation = Conversation::new_unvalidated([
            visible_message,
            empty_message,
            marked_message,
            marked_user_message,
            hidden_marked_message,
        ]);

        let messages = messages_for_acp_replay(&conversation);

        assert_eq!(
            messages
                .iter()
                .filter_map(|message| message.id.as_deref())
                .collect::<Vec<_>>(),
            vec!["msg_visible", "msg_limited"]
        );
        assert_eq!(
            messages[1].as_concat_text(),
            "Response stopped because the model reached its output-token limit."
        );
        assert!(messages[1].metadata.output_token_limit_reached);
        assert!(conversation.messages()[2].content.is_empty());
    }

    fn persisted_enriched_tool_request() -> ToolRequest {
        ToolRequest {
            id: "req_first".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")),
            metadata: None,
            tool_meta: Some(serde_json::json!({
                (crate::conversation::message::TOOL_META_TITLE_KEY): "applied dark mode polish",
                (crate::conversation::message::TOOL_META_CHAIN_SUMMARY_KEY): {
                    "summary": "applied dark mode polish",
                    "count": 3,
                },
            })),
        }
    }

    #[test]
    fn replay_includes_persisted_enrichment_when_requested() {
        let mut message =
            Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_replay");
        message.metadata.output_token_limit_reached = true;
        let tool_call =
            build_replayed_tool_call(&persisted_enriched_tool_request(), &message, true);
        let goose = tool_call
            .meta
            .as_ref()
            .and_then(|meta| meta.get("goose"))
            .expect("valid initial tool call should contain goose metadata");

        assert_eq!(tool_call.title, "applied dark mode polish");
        assert_eq!(
            goose,
            &serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_replay",
                "outputTokenLimitReached": true,
                "toolCall": {
                    "toolName": "developer__shell",
                    "extensionName": "developer",
                },
                "toolChainSummary": {
                    "summary": "applied dark mode polish",
                    "count": 3,
                },
            }),
        );
    }

    #[test]
    fn replay_omits_persisted_enrichment_when_not_requested() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_replay");
        let tool_call =
            build_replayed_tool_call(&persisted_enriched_tool_request(), &message, false);
        let goose = tool_call
            .meta
            .as_ref()
            .and_then(|meta| meta.get("goose"))
            .expect("valid initial tool call should contain goose metadata");

        assert_eq!(tool_call.title, "developer: shell");
        assert_eq!(
            goose,
            &serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_replay",
                "toolCall": {
                    "toolName": "developer__shell",
                    "extensionName": "developer",
                },
            }),
        );
    }

    #[test]
    fn pending_permissions_are_limited_to_the_active_turn() {
        let approval = |id: &str| {
            Message::assistant().with_action_required(
                id,
                "tool".to_string(),
                Default::default(),
                None,
            )
        };
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("old turn"),
            approval("old"),
            Message::user().with_text("current turn"),
            approval("current"),
        ]);

        let active = active_turn_messages(&conversation);
        let approval_ids = active
            .iter()
            .flat_map(|message| &message.content)
            .filter_map(|content| match content {
                MessageContent::ActionRequired(action) => match &action.data {
                    ActionRequiredData::ToolConfirmation { id, .. } => Some(id.as_str()),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(approval_ids, ["current"]);

        let no_kickoff = Conversation::new_unvalidated([approval("orphan")]);
        assert_eq!(active_turn_messages(&no_kickoff).len(), 1);
    }

    #[test]
    fn pagination_applies_after_visibility_filtering() {
        // The two newest raw messages are agent-only; a limit of 2 must still
        // return the two visible messages rather than an empty page.
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("first"),
            Message::assistant().with_text("second"),
            Message::assistant().with_text("summary").agent_only(),
            Message::assistant().with_text("continuation").agent_only(),
        ]);

        let visible = messages_for_acp_replay(&conversation);
        let page = paginate_messages(&visible, Some(2), None);

        let texts: Vec<String> = page.iter().map(|m| m.as_concat_text()).collect();
        assert_eq!(texts, vec!["first".to_string(), "second".to_string()]);
    }

    fn meta_with(key: &str, value: serde_json::Value) -> Meta {
        let mut meta = Meta::new();
        meta.insert(key.to_string(), value);
        meta
    }

    #[test]
    fn paginate_returns_full_history_when_no_limit() {
        let messages = vec![1, 2, 3, 4, 5];
        assert_eq!(paginate_messages(&messages, None, None), messages);
    }

    #[test]
    fn paginate_returns_most_recent_with_limit() {
        let messages = vec![1, 2, 3, 4, 5];
        assert_eq!(paginate_messages(&messages, Some(2), None), vec![4, 5]);
    }

    #[test]
    fn paginate_offset_pages_backwards() {
        let messages = vec![1, 2, 3, 4, 5];
        assert_eq!(paginate_messages(&messages, Some(2), Some(2)), vec![2, 3]);
    }

    #[test]
    fn paginate_clamps_oversized_limit_and_offset() {
        let messages = vec![1, 2, 3];
        assert_eq!(paginate_messages(&messages, Some(10), None), vec![1, 2, 3]);
        assert_eq!(
            paginate_messages(&messages, None, Some(10)),
            Vec::<i32>::new()
        );
    }

    #[test]
    fn pagination_from_meta_is_none_when_absent() {
        assert_eq!(message_pagination_from_meta(None).unwrap(), (None, None));
        assert_eq!(
            message_pagination_from_meta(Some(&Meta::new())).unwrap(),
            (None, None)
        );
    }

    #[test]
    fn pagination_from_meta_treats_null_as_absent() {
        let meta = meta_with("messageLimit", serde_json::Value::Null);
        assert_eq!(
            message_pagination_from_meta(Some(&meta)).unwrap(),
            (None, None)
        );
    }

    #[test]
    fn pagination_from_meta_reads_limit_and_offset() {
        let mut meta = Meta::new();
        meta.insert("messageLimit".to_string(), serde_json::json!(50));
        meta.insert("messageOffset".to_string(), serde_json::json!(10));
        assert_eq!(
            message_pagination_from_meta(Some(&meta)).unwrap(),
            (Some(50), Some(10))
        );
    }

    #[test]
    fn pagination_from_meta_rejects_non_integer() {
        for value in [
            serde_json::json!("50"),
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!(true),
        ] {
            assert!(message_pagination_from_meta(Some(&meta_with("messageLimit", value))).is_err());
        }
    }
}
