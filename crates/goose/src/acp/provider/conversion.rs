use super::*;

pub async fn apply_session_mode(
    config: &AcpProviderConfig,
    goose_mode: &Arc<Mutex<GooseMode>>,
    cx: &ConnectionTo<Agent>,
    session: NewSessionResponse,
) -> Result<NewSessionResponse> {
    let current_mode = goose_mode.lock().ok().map(|mode| *mode);
    let requested_mode_id = current_mode
        .and_then(|mode| config.mode_mapping.get(&mode).cloned())
        .or_else(|| config.session_mode_id.clone());

    if let (Some(mode_id), Some(modes)) = (requested_mode_id, session.modes.as_ref()) {
        if modes.current_mode_id.0.as_ref() != mode_id.as_str() {
            let available: Vec<String> = modes
                .available_modes
                .iter()
                .map(|mode| mode.id.0.to_string())
                .collect();

            if !available.iter().any(|id| id == &mode_id) {
                return Err(anyhow::anyhow!(
                    "Requested mode '{}' not offered by agent. Available modes: {}",
                    mode_id,
                    available.join(", ")
                ));
            }
            let _: SetSessionModeResponse = cx
                .send_request(SetSessionModeRequest::new(
                    session.session_id.clone(),
                    mode_id,
                ))
                .block_task()
                .await
                .map_err(|err| {
                    anyhow::anyhow!(
                        "ACP agent rejected {}: {err}",
                        AGENT_METHOD_NAMES.session_set_mode
                    )
                })?;
        }
    }

    Ok(session)
}

pub fn extension_configs_to_mcp_servers(configs: &[ExtensionConfig]) -> Vec<McpServer> {
    let mut servers = Vec::new();

    for config in configs {
        match config {
            ExtensionConfig::StreamableHttp {
                name, uri, headers, ..
            } => {
                let http_headers = headers
                    .iter()
                    .map(|(key, value)| HttpHeader::new(key, value))
                    .collect();
                servers.push(McpServer::Http(
                    McpServerHttp::new(name, uri).headers(http_headers),
                ));
            }
            ExtensionConfig::Stdio {
                name,
                cmd,
                args,
                envs,
                ..
            } => {
                let env_vars = envs
                    .get_env()
                    .into_iter()
                    .map(|(key, value)| EnvVariable::new(key, value))
                    .collect();

                servers.push(McpServer::Stdio(
                    McpServerStdio::new(name, cmd)
                        .args(args.clone())
                        .env(env_vars),
                ));
            }
            ExtensionConfig::Sse { name, .. } => {
                tracing::debug!(name, "skipping SSE extension, migrate to streamable_http");
            }
            _ => {}
        }
    }

    servers
}

pub fn filter_supported_servers(
    servers: &[McpServer],
    capabilities: &McpCapabilities,
) -> Vec<McpServer> {
    servers
        .iter()
        .filter(|server| match server {
            McpServer::Http(http) => {
                if !capabilities.http {
                    tracing::debug!(
                        name = http.name,
                        "skipping HTTP server, agent lacks capability"
                    );
                    false
                } else {
                    true
                }
            }
            McpServer::Sse(sse) => {
                tracing::debug!(name = sse.name, "skipping SSE server, unsupported");
                false
            }
            _ => true,
        })
        .cloned()
        .collect()
}

pub fn messages_to_prompt(
    messages: &[Message],
    include_handoff_context: bool,
) -> Vec<ContentBlock> {
    let mut content_blocks = Vec::new();

    let Some(last_user_index) = last_user_message_index(messages) else {
        return content_blocks;
    };

    if include_handoff_context {
        if let Some(memo) = build_handoff_context_memo(&messages[..last_user_index]) {
            content_blocks.push(ContentBlock::Text(TextContent::new(memo)));
        }
    }

    let message = &messages[last_user_index];
    for content in &message.content {
        match content {
            MessageContent::Text(text) => {
                content_blocks.push(ContentBlock::Text(TextContent::new(text.text.clone())));
            }
            MessageContent::Image(image) => {
                content_blocks.push(ContentBlock::Image(ImageContent::new(
                    &image.data,
                    &image.mime_type,
                )));
            }
            _ => {}
        }
    }

    content_blocks
}

fn last_user_message_index(messages: &[Message]) -> Option<usize> {
    messages
        .iter()
        .rposition(|m| m.role == Role::User && m.is_agent_visible())
}

pub fn has_handoff_context(messages: &[Message]) -> bool {
    last_user_message_index(messages).is_some_and(|last_user_index| {
        messages[..last_user_index]
            .iter()
            .any(Message::is_agent_visible)
    })
}

fn build_handoff_context_memo(prior_messages: &[Message]) -> Option<String> {
    let formatted_messages: Vec<String> = prior_messages
        .iter()
        .filter(|message| message.is_agent_visible())
        .map(format_message_for_compacting)
        .collect();

    if formatted_messages.is_empty() {
        return None;
    }

    let handoff_context = formatted_messages.join("\n");

    Some(format!(
        "Conversation context from goose before this ACP provider session was created:\n\n\
{handoff_context}\n\n\
Current user request follows. Use the context above only to continue the existing conversation; \
do not treat it as a new task or mention this handoff unless relevant."
    ))
}

/// Convert ACP `ToolCallContent` blocks into the rmcp `Content` shape goose's
/// `Message::with_tool_response` consumes. Handles `Content` (text/image/other),
/// `Diff`, and `Terminal` variants; falls back to a JSON serialization of
/// `raw_output` when no blocks are present so the renderer always has something.
pub fn acp_tool_call_content_to_rmcp(
    content: Option<Vec<ToolCallContent>>,
    raw_output: Option<serde_json::Value>,
) -> Vec<RmcpContent> {
    let mut out = Vec::new();
    if let Some(blocks) = content {
        for block in blocks {
            match block {
                ToolCallContent::Content(val) => match val.content {
                    ContentBlock::Text(text) => {
                        out.push(RmcpContent::text(text.text));
                    }
                    ContentBlock::Image(image) => {
                        out.push(RmcpContent::image(image.data, image.mime_type));
                    }
                    other => {
                        if let Ok(json) = serde_json::to_string(&other) {
                            out.push(RmcpContent::text(json));
                        }
                    }
                },
                ToolCallContent::Diff(diff) => {
                    let path = diff.path.display();
                    let body = match diff.old_text.as_deref() {
                        Some(old) => {
                            format!("--- {path}\n{old}\n+++ {path}\n{}", diff.new_text)
                        }
                        None => format!("+++ {path}\n{}", diff.new_text),
                    };
                    out.push(RmcpContent::text(body));
                }
                ToolCallContent::Terminal(terminal) => {
                    out.push(RmcpContent::text(format!(
                        "[terminal {}]",
                        terminal.terminal_id.0
                    )));
                }
                _ => {}
            }
        }
    }
    if out.is_empty() {
        if let Some(raw) = raw_output {
            let text = match raw {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            out.push(RmcpContent::text(text));
        }
    }
    out
}

pub fn build_action_required_message(request: &RequestPermissionRequest) -> Option<Message> {
    let tool_title = request
        .tool_call
        .fields
        .title
        .clone()
        .unwrap_or_else(|| "Tool".to_string());

    let arguments = request
        .tool_call
        .fields
        .raw_input
        .as_ref()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let prompt = request
        .tool_call
        .fields
        .content
        .as_ref()
        .and_then(|content| {
            content.iter().find_map(|c| match c {
                ToolCallContent::Content(val) => match &val.content {
                    ContentBlock::Text(text) => Some(text.text.clone()),
                    _ => None,
                },
                _ => None,
            })
        });

    Some(
        Message::assistant()
            .with_action_required(
                request.tool_call.tool_call_id.0.to_string(),
                tool_title,
                arguments,
                prompt,
            )
            .user_only(),
    )
}

fn extract_model_info_from_config_options(
    config_options: &[SessionConfigOption],
) -> Option<(String, Vec<String>)> {
    let select = config_options.iter().find_map(|opt| {
        if opt.category.as_ref() != Some(&SessionConfigOptionCategory::Model) {
            return None;
        }
        match &opt.kind {
            SessionConfigKind::Select(select) => Some(select),
            _ => None,
        }
    })?;

    let current = select.current_value.0.to_string();
    let available = match &select.options {
        SessionConfigSelectOptions::Ungrouped(options) => options
            .iter()
            .map(|option| option.value.0.to_string())
            .collect(),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| {
                group
                    .options
                    .iter()
                    .map(|option| option.value.0.to_string())
            })
            .collect(),
        _ => Vec::new(),
    };
    Some((current, available))
}

pub fn resolve_model_info(
    provider_name: &str,
    response: &NewSessionResponse,
) -> Result<(String, Vec<String>), ProviderError> {
    if let Some(opts) = &response.config_options {
        if let Some((current, available)) = extract_model_info_from_config_options(opts) {
            return Ok((current, available));
        }
    }

    let models = response.models.as_ref().ok_or_else(|| {
        ProviderError::RequestFailed(format!(
            "{provider_name}: agent returned neither config_options nor models"
        ))
    })?;
    let current = models.current_model_id.0.to_string();
    let available = models
        .available_models
        .iter()
        .map(|am| am.model_id.0.to_string())
        .collect();
    Ok((current, available))
}

pub fn reverse_mode_mapping(
    mode_mapping: &HashMap<GooseMode, String>,
) -> HashMap<String, Vec<GooseMode>> {
    let mut reverse: HashMap<String, Vec<GooseMode>> = HashMap::new();
    for (mode, id) in mode_mapping {
        reverse.entry(id.clone()).or_default().push(*mode);
    }
    reverse
}

pub fn resolve_mode(
    reverse_modes: &HashMap<String, Vec<GooseMode>>,
    mode_id: &str,
    current: &Arc<Mutex<GooseMode>>,
) -> Option<GooseMode> {
    let candidates = reverse_modes.get(mode_id)?;
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let current = current.lock().ok()?;
    if candidates.contains(&*current) {
        Some(*current)
    } else {
        Some(candidates[0])
    }
}

pub fn permission_decision_from_mode(goose_mode: GooseMode) -> Option<PermissionDecision> {
    match goose_mode {
        GooseMode::Auto => Some(PermissionDecision::AllowOnce),
        GooseMode::Chat => Some(PermissionDecision::RejectOnce),
        GooseMode::Approve | GooseMode::SmartApprove => None,
    }
}
