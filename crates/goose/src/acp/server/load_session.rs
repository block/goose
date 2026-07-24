use super::tool_calls::conversion::{
    build_initial_tool_call, tool_call_update_fields_from_response, trusted_update_meta,
};
use super::tool_calls::enrichment::tool_chain_summary;
use super::*;
use agent_client_protocol::schema::v1::ToolCall;

fn replay_message_meta(message: &Message) -> Meta {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "goose".to_string(),
        serde_json::Value::Object(replay_message_goose_meta(message)),
    );
    meta
}

fn replay_message_goose_meta(message: &Message) -> serde_json::Map<String, serde_json::Value> {
    let mut goose = serde_json::Map::new();
    goose.insert("created".to_string(), serde_json::json!(message.created));
    if let Some(id) = &message.id {
        goose.insert("messageId".to_string(), serde_json::json!(id));
    }
    if message.metadata.steer {
        goose.insert("steer".to_string(), serde_json::json!(true));
    }
    goose
}

fn merge_replay_message_meta(meta: Option<Meta>, message: &Message) -> Meta {
    let replay_goose = replay_message_goose_meta(message);
    let mut meta = meta.unwrap_or_default();
    let goose_value = meta
        .entry("goose".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let serde_json::Value::Object(goose) = goose_value {
        for (key, value) in replay_goose {
            goose.insert(key, value);
        }
    } else {
        *goose_value = serde_json::Value::Object(replay_goose);
    }

    meta
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

fn build_replayed_tool_call(
    tool_request: &ToolRequest,
    client_requests_tool_call_label_enrichment: bool,
) -> ToolCall {
    let mut tool_call =
        build_initial_tool_call(tool_request, client_requests_tool_call_label_enrichment);

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
) -> Result<(), agent_client_protocol::Error> {
    let session_id = SessionId::new(session.id.clone());
    let tool_call_notifier = ToolCallNotifier::new(cx, &session_id);

    let messages = session
        .conversation
        .as_ref()
        .map(|c| c.user_visible_messages())
        .unwrap_or_default();

    let mut replay_tool_requests = HashMap::new();

    for message in &messages {
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

                    let mut tool_call = build_replayed_tool_call(
                        tool_request,
                        client_requests_tool_call_label_enrichment,
                    );
                    let meta = tool_call.meta.take();
                    let tool_call = tool_call.meta(merge_replay_message_meta(meta, message));

                    tool_call_notifier.send_initial(tool_call)?;
                }
                MessageContent::ToolResponse(tool_response) => {
                    let fields = tool_call_update_fields_from_response(
                        tool_response,
                        replay_tool_requests.get(&tool_response.id),
                        true,
                    );

                    let update =
                        ToolCallUpdate::new(ToolCallId::new(tool_response.id.clone()), fields)
                            .meta(merge_replay_message_meta(
                                trusted_update_meta(tool_response),
                                message,
                            ));
                    tool_call_notifier.send_update(update)?;
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
    pub(super) async fn handle_load_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "load session request");
        validate_absolute_cwd(&args.cwd)?;

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
        )?;
        let (agent, extension_results) = self.prepare_acp_session_agent(cx, &session).await?;
        self.apply_session_recipe(&agent, &session).await?;
        self.register_acp_session(session_id_str.clone(), agent.clone())
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
        let tool_call = build_replayed_tool_call(&persisted_enriched_tool_request(), true);
        let goose = tool_call
            .meta
            .as_ref()
            .and_then(|meta| meta.get("goose"))
            .expect("valid initial tool call should contain goose metadata");

        assert_eq!(tool_call.title, "applied dark mode polish");
        assert_eq!(
            goose.get("toolCall"),
            Some(
                &serde_json::json!({ "toolName": "developer__shell", "extensionName": "developer" })
            ),
        );
        assert_eq!(
            goose.get("toolChainSummary"),
            Some(&serde_json::json!({ "summary": "applied dark mode polish", "count": 3 })),
        );
    }

    #[test]
    fn replay_omits_persisted_enrichment_when_not_requested() {
        let tool_call = build_replayed_tool_call(&persisted_enriched_tool_request(), false);
        let goose = tool_call
            .meta
            .as_ref()
            .and_then(|meta| meta.get("goose"))
            .expect("valid initial tool call should contain goose metadata");

        assert_eq!(tool_call.title, "developer: shell");
        assert_eq!(
            goose.get("toolCall"),
            Some(
                &serde_json::json!({ "toolName": "developer__shell", "extensionName": "developer" })
            ),
        );
        assert_eq!(goose.get("toolChainSummary"), None);
    }

    #[test]
    fn merge_replay_message_meta_preserves_existing_goose_meta() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_1");
        let existing = serde_json::from_value(serde_json::json!({
            "goose": {
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            }
        }))
        .unwrap();

        let merged = merge_replay_message_meta(Some(existing), &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_1",
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            })),
        );
    }

    #[test]
    fn merge_replay_message_meta_creates_fresh_when_none() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_2");

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_2",
            })),
        );
    }

    #[test]
    fn merge_replay_message_meta_includes_steer_marker() {
        let message = Message::new(Role::User, 1_700_000_000, vec![])
            .with_id("msg_steer")
            .with_steer();

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_steer",
                "steer": true,
            })),
            "replay must carry the steer marker so the boundary survives reload"
        );
    }

    #[test]
    fn merge_replay_message_meta_omits_steer_when_not_set() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_plain");

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(merged.get("goose").and_then(|g| g.get("steer")), None);
    }

    #[test]
    fn merge_replay_message_meta_omits_message_id_when_none() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]);

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
            })),
        );
    }
}
