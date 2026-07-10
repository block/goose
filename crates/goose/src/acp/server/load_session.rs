use super::*;

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
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<HashMap<String, crate::conversation::message::ToolRequest>, agent_client_protocol::Error>
{
    let session_id = SessionId::new(session.id.clone());
    let sid = sid_short(session_id.0.as_ref());

    let messages = session
        .conversation
        .as_ref()
        .map(|c| c.messages().to_vec())
        .unwrap_or_default();
    let messages = if limit.is_some() || offset.is_some() {
        paginate_messages(&messages, limit, offset)
    } else {
        messages
    };
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

    Ok(replay_tool_requests)
}

impl GooseAcpAgent {
    pub(super) async fn handle_load_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "load session request");
        validate_absolute_cwd(&args.cwd)?;
        let (message_limit, message_offset) = message_pagination_from_meta(args.meta.as_ref())?;

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
            message_limit,
            message_offset,
        )?;
        let (agent, extension_results) = self.prepare_acp_session_agent(cx, &session).await?;
        self.apply_session_recipe(&agent, &session).await?;
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

        let (mode_state, config_options) =
            build_session_setup_config(&self.provider_inventory, &session).await?;

        self.notify_session_setup(cx, &session).await?;

        let mut response = LoadSessionResponse::new().modes(mode_state);
        if let Some(co) = config_options {
            response = response.config_options(co);
        }

        response = response.meta(session_response_meta(&session, &extension_results));

        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            "perf: load_session_refactor done"
        );
        self.closed_session_ids.lock().await.remove(&session_id_str);
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
