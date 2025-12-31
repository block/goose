use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use sacp::schema::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SessionNotification, SessionUpdate, TextContent, ToolCallContent, ToolCallStatus,
    VERSION as PROTOCOL_VERSION,
};
use sacp::{ByteStreams, ClientToAgent, JrConnectionCx};

use crate::conversation::message::{Message, MessageContent, ToolRequest, ToolResponse};
use crate::model::ModelConfig;
use crate::providers::base::{MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;
use crate::subprocess::configure_command_no_window;
use rmcp::model::{CallToolRequestParam, CallToolResult, Content, RawContent, Role, Tool};

/// Default ACP agent - claude-code-acp
pub const ACP_DEFAULT_MODEL: &str = "claude-code";
pub const ACP_DOC_URL: &str = "https://github.com/zed-industries/claude-code-acp";

/// Known ACP agent shortcuts and their npx packages
fn resolve_acp_command(model: &str) -> (String, Vec<String>) {
    match model {
        // Short aliases
        "claude-code" | "claude" => (
            "npx".to_string(),
            vec!["@zed-industries/claude-code-acp".to_string()],
        ),
        "codex" => ("npx".to_string(), vec!["@anthropics/codex-acp".to_string()]),
        // Full package names (npx @scope/package)
        s if s.starts_with("@") => ("npx".to_string(), vec![s.to_string()]),
        // Direct command (e.g., a local binary path)
        s if s.contains('/') || s.contains('\\') => (s.to_string(), vec![]),
        // Assume it's an npx package name
        other => ("npx".to_string(), vec![other.to_string()]),
    }
}

#[derive(Debug)]
pub struct AcpProvider {
    command: String,
    args: Vec<String>,
    model: ModelConfig,
    name: String,
}

impl AcpProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let (command, args) = resolve_acp_command(&model.model_name);

        Ok(Self {
            command,
            args,
            model,
            name: "acp".to_string(),
        })
    }

    fn convert_messages_to_prompt(&self, messages: &[Message]) -> Vec<ContentBlock> {
        let mut content_blocks = Vec::new();

        for message in messages.iter().filter(|m| m.is_agent_visible()) {
            for content in &message.content {
                match content {
                    MessageContent::Text(text) => {
                        let prefix = match message.role {
                            Role::User => "",
                            Role::Assistant => "[Previous assistant response]: ",
                        };
                        content_blocks.push(ContentBlock::Text(TextContent {
                            text: format!("{}{}", prefix, text.text),
                            annotations: None,
                            meta: None,
                        }));
                    }
                    MessageContent::ToolRequest(req) => {
                        if let Ok(call) = &req.tool_call {
                            content_blocks.push(ContentBlock::Text(TextContent {
                                text: format!(
                                    "[Tool call: {}]\n{}",
                                    call.name,
                                    serde_json::to_string_pretty(&call.arguments)
                                        .unwrap_or_default()
                                ),
                                annotations: None,
                                meta: None,
                            }));
                        }
                    }
                    MessageContent::ToolResponse(resp) => {
                        if let Ok(result) = &resp.tool_result {
                            let text = result
                                .content
                                .iter()
                                .filter_map(|c| match &c.raw {
                                    RawContent::Text(t) => Some(t.text.as_str()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            content_blocks.push(ContentBlock::Text(TextContent {
                                text: format!("[Tool result]: {}", text),
                                annotations: None,
                                meta: None,
                            }));
                        }
                    }
                    _ => {}
                }
            }
        }

        content_blocks
    }

    fn spawn_agent(&self) -> Result<Child, ProviderError> {
        let mut cmd = Command::new(&self.command);
        configure_command_no_window(&mut cmd);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn ACP agent '{}': {}",
                self.command, e
            ))
        })
    }
}

#[derive(Debug, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: Option<serde_json::Value>,
}

#[derive(Debug)]
enum AcpEvent {
    Text(String),
    ToolCallStart(PendingToolCall),
    ToolCallComplete {
        id: String,
        status: ToolCallStatus,
        content: Vec<ToolCallContent>,
    },
    Done,
    Error(String),
}

fn tool_call_content_to_text(content: &[ToolCallContent]) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            ToolCallContent::Content {
                content: ContentBlock::Text(t),
            } => Some(t.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[async_trait]
impl Provider for AcpProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            "acp",
            "ACP Agent",
            "Connect to ACP agents. Set model to: claude-code, codex, or any npx package name",
            ACP_DEFAULT_MODEL,
            vec!["claude-code", "codex"],
            ACP_DOC_URL,
            vec![],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let stream = self.stream(system, messages, tools).await?;

        use futures::StreamExt;
        tokio::pin!(stream);

        let mut content: Vec<MessageContent> = Vec::new();
        while let Some(result) = stream.next().await {
            if let Ok((Some(msg), _)) = result {
                content.extend(msg.content);
            }
        }

        if content.is_empty() {
            return Err(ProviderError::RequestFailed(
                "No response received from ACP agent".to_string(),
            ));
        }

        let message = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), Usage::default()),
        ))
    }

    async fn stream(
        &self,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut child = self.spawn_agent()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ProviderError::RequestFailed("Failed to get stdin of ACP agent".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ProviderError::RequestFailed("Failed to get stdout of ACP agent".to_string())
        })?;

        let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
        let prompt_blocks = self.convert_messages_to_prompt(messages);
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let (tx, mut rx) = mpsc::unbounded_channel::<AcpEvent>();

        let tx_notify = tx.clone();
        let tx_done = tx.clone();

        tokio::spawn(async move {
            let conn_result = ClientToAgent::builder()
                .name("goose-acp-client")
                .on_receive_notification(
                    {
                        let tx = tx_notify;
                        async move |notification: SessionNotification, _cx| {
                            match notification.update {
                                SessionUpdate::AgentMessageChunk(chunk) => {
                                    if let ContentBlock::Text(text) = chunk.content {
                                        let _ = tx.send(AcpEvent::Text(text.text));
                                    }
                                }
                                SessionUpdate::ToolCall(tool_call) => {
                                    let _ = tx.send(AcpEvent::ToolCallStart(PendingToolCall {
                                        id: tool_call.id.0.to_string(),
                                        name: tool_call.title,
                                        arguments: tool_call.raw_input,
                                    }));
                                }
                                SessionUpdate::ToolCallUpdate(update) => {
                                    if let Some(status) = update.fields.status {
                                        let _ = tx.send(AcpEvent::ToolCallComplete {
                                            id: update.id.0.to_string(),
                                            status,
                                            content: update.fields.content.unwrap_or_default(),
                                        });
                                    }
                                }
                                _ => {}
                            }
                            Ok(())
                        }
                    },
                    sacp::on_receive_notification!(),
                )
                .on_receive_request(
                    async move |request: RequestPermissionRequest, request_cx, _connection_cx| {
                        let option_id = request.options.first().map(|opt| opt.id.clone());
                        match option_id {
                            Some(id) => request_cx.respond(RequestPermissionResponse {
                                outcome: RequestPermissionOutcome::Selected { option_id: id },
                                meta: None,
                            }),
                            None => request_cx.respond(RequestPermissionResponse {
                                outcome: RequestPermissionOutcome::Cancelled,
                                meta: None,
                            }),
                        }
                    },
                    sacp::on_receive_request!(),
                )
                .connect_to(transport);

            match conn_result {
                Ok(conn) => {
                    let run_result = conn
                        .run_until({
                            let prompt = prompt_blocks;
                            let cwd = working_dir;
                            move |cx: JrConnectionCx<ClientToAgent>| async move {
                                cx.send_request(InitializeRequest {
                                    protocol_version: PROTOCOL_VERSION,
                                    client_capabilities: Default::default(),
                                    client_info: Default::default(),
                                    meta: None,
                                })
                                .block_task()
                                .await?;

                                let session = cx
                                    .send_request(NewSessionRequest {
                                        mcp_servers: vec![],
                                        cwd,
                                        meta: None,
                                    })
                                    .block_task()
                                    .await?;

                                let _response: PromptResponse = cx
                                    .send_request(PromptRequest {
                                        session_id: session.session_id,
                                        prompt,
                                        meta: None,
                                    })
                                    .block_task()
                                    .await?;

                                Ok::<_, sacp::Error>(())
                            }
                        })
                        .await;

                    match run_result {
                        Ok(_) => {
                            let _ = tx_done.send(AcpEvent::Done);
                        }
                        Err(e) => {
                            let _ = tx_done.send(AcpEvent::Error(format!("ACP error: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx_done.send(AcpEvent::Error(format!("Connection error: {}", e)));
                }
            }

            let _ = child.kill().await;
        });

        let pending_tools: Arc<Mutex<HashMap<String, PendingToolCall>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_tools_clone = pending_tools.clone();

        Ok(Box::pin(try_stream! {
            while let Some(event) = rx.recv().await {
                match event {
                    AcpEvent::Text(text) => {
                        let message = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            vec![MessageContent::text(text)],
                        );
                        yield (Some(message), None);
                    }
                    AcpEvent::ToolCallStart(tool_call) => {
                        let mut pending = pending_tools_clone.lock().await;
                        pending.insert(tool_call.id.clone(), tool_call.clone());

                        let arguments = tool_call.arguments
                            .and_then(|v| v.as_object().cloned())
                            .unwrap_or_default();

                        let tool_request = ToolRequest {
                            id: tool_call.id.clone(),
                            tool_call: Ok(CallToolRequestParam {
                                name: tool_call.name.into(),
                                arguments: Some(arguments),
                            }),
                            metadata: None,
                        };
                        let message = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            vec![MessageContent::ToolRequest(tool_request)],
                        );
                        yield (Some(message), None);
                    }
                    AcpEvent::ToolCallComplete { id, status, content } => {
                        let pending = pending_tools_clone.lock().await;
                        if pending.contains_key(&id) {
                            let result_text = tool_call_content_to_text(&content);
                            let is_error = matches!(status, ToolCallStatus::Failed);

                            let call_result = CallToolResult {
                                content: vec![Content::text(result_text)],
                                structured_content: None,
                                is_error: Some(is_error),
                                meta: None,
                            };

                            let tool_response = ToolResponse {
                                id,
                                tool_result: Ok(call_result),
                                metadata: None,
                            };
                            let message = Message::new(
                                Role::Assistant,
                                chrono::Utc::now().timestamp(),
                                vec![MessageContent::ToolResponse(tool_response)],
                            );
                            yield (Some(message), None);
                        }
                    }
                    AcpEvent::Done => {
                        break;
                    }
                    AcpEvent::Error(e) => {
                        Err(ProviderError::RequestFailed(e))?;
                    }
                }
            }
        }))
    }
}
