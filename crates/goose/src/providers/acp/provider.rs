use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use sacp::schema::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest, PromptResponse,
    SessionNotification, SessionUpdate, TextContent, VERSION as PROTOCOL_VERSION,
};
use sacp::{ByteStreams, ClientToAgent, JrConnectionCx};

use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;
use crate::subprocess::configure_command_no_window;
use rmcp::model::{Role, Tool};

pub const ACP_DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
pub const ACP_DOC_URL: &str = "https://github.com/anthropics/claude-code";

#[derive(Debug)]
pub struct AcpProvider {
    command: String,
    args: Vec<String>,
    model: ModelConfig,
    name: String,
}

impl AcpProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        // Default to npx @zed-industries/claude-code-acp
        let command = "npx".to_string();
        let args = vec!["@zed-industries/claude-code-acp".to_string()];

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
                                    rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
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

    async fn spawn_agent(&self) -> Result<Child, ProviderError> {
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

    async fn run_prompt(
        &self,
        _system: &str,
        messages: &[Message],
    ) -> Result<(Message, Usage), ProviderError> {
        let mut child = self.spawn_agent().await?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ProviderError::RequestFailed("Failed to get stdin of ACP agent".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ProviderError::RequestFailed("Failed to get stdout of ACP agent".to_string())
        })?;

        let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
        let prompt_blocks = self.convert_messages_to_prompt(messages);
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Collect response text
        let collected_text = Arc::new(Mutex::new(String::new()));
        let collected_text_clone = collected_text.clone();

        let result = ClientToAgent::builder()
            .name("goose-acp-client")
            .on_receive_notification(
                {
                    let collected = collected_text_clone.clone();
                    async move |notification: SessionNotification, _cx| {
                        if let SessionUpdate::AgentMessageChunk(chunk) = notification.update {
                            if let ContentBlock::Text(text) = chunk.content {
                                let mut guard = collected.lock().await;
                                guard.push_str(&text.text);
                            }
                        }
                        Ok(())
                    }
                },
                sacp::on_receive_notification!(),
            )
            .connect_to(transport)
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to connect: {}", e)))?
            .run_until({
                let prompt = prompt_blocks;
                let cwd = working_dir;
                move |cx: JrConnectionCx<ClientToAgent>| async move {
                    // Initialize
                    cx.send_request(InitializeRequest {
                        protocol_version: PROTOCOL_VERSION,
                        client_capabilities: Default::default(),
                        client_info: Default::default(),
                        meta: None,
                    })
                    .block_task()
                    .await?;

                    // Create session
                    let session = cx
                        .send_request(NewSessionRequest {
                            mcp_servers: vec![],
                            cwd,
                            meta: None,
                        })
                        .block_task()
                        .await?;

                    // Send prompt
                    let response: PromptResponse = cx
                        .send_request(PromptRequest {
                            session_id: session.session_id,
                            prompt,
                            meta: None,
                        })
                        .block_task()
                        .await?;

                    Ok::<_, sacp::Error>(response)
                }
            })
            .await;

        // Clean up child process
        let _ = child.kill().await;

        result.map_err(|e| ProviderError::RequestFailed(format!("ACP error: {}", e)))?;

        let text = collected_text.lock().await.clone();
        if text.is_empty() {
            return Err(ProviderError::RequestFailed(
                "No response received from ACP agent".to_string(),
            ));
        }

        let message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            vec![MessageContent::text(text)],
        );

        Ok((message, Usage::default()))
    }
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
            "Connect to any ACP-compatible agent (like Claude Code)",
            ACP_DEFAULT_MODEL,
            vec![],
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

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let (message, usage) = self.run_prompt(system, messages).await?;
        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }
}
