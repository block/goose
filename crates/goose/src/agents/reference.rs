/// A simplified agent implementation used as a reference
/// It makes no attempt to handle context limits, and cannot read resources
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, instrument};

use super::agent::SessionConfig;
use super::capabilities::get_parameter_names;
use super::extension::ToolInfo;
use super::types::ToolResultReceiver;
use super::Agent;
use crate::agents::capabilities::Capabilities;
use crate::agents::extension::{ExtensionConfig, ExtensionResult};
use crate::message::{Message, ToolRequest};
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::token_counter::TokenCounter;
use crate::{register_agent, session};
use anyhow::{anyhow, Result};
use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use mcp_core::{prompt::Prompt, protocol::GetPromptResult, Content, ToolResult};
use serde_json::{json, Value};

/// Reference implementation of an Agent
pub struct ReferenceAgent {
    capabilities: Mutex<Capabilities>,
    _token_counter: TokenCounter,
    tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    tool_result_rx: ToolResultReceiver,
}

impl ReferenceAgent {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        let (tx, rx) = mpsc::channel(32);
        Self {
            capabilities: Mutex::new(Capabilities::new(provider)),
            _token_counter: token_counter,
            tool_result_tx: tx,
            tool_result_rx: Arc::new(Mutex::new(rx)),
        }
    }
}

#[async_trait]
impl Agent for ReferenceAgent {
    async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()> {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.add_extension(extension).await
    }

    async fn list_tools(&self) -> Vec<Tool> {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.get_prefixed_tools().await.unwrap_or_default()
    }

    async fn remove_extension(&mut self, name: &str) {
        let mut capabilities = self.capabilities.lock().await;
        capabilities
            .remove_extension(name)
            .await
            .expect("Failed to remove extension");
    }

    async fn list_extensions(&self) -> Vec<String> {
        let capabilities = self.capabilities.lock().await;
        capabilities
            .list_extensions()
            .await
            .expect("Failed to list extensions")
    }

    async fn passthrough(&self, _extension: &str, _request: Value) -> ExtensionResult<Value> {
        // TODO implement
        Ok(Value::Null)
    }

    async fn handle_confirmation(
        &self,
        _request_id: String,
        _confirmation: PermissionConfirmation,
    ) {
        // TODO implement
    }

    #[instrument(skip(self, messages, session), fields(user_message))]
    async fn reply(
        &self,
        messages: &[Message],
        session: Option<SessionConfig>,
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message>>> {
        let mut messages = messages.to_vec();
        let reply_span = tracing::Span::current();
        let mut capabilities = self.capabilities.lock().await;
        let mut tools = capabilities.get_prefixed_tools().await?;
        // we add in the read_resource tool by default
        // TODO: make sure there is no collision with another extension's tool name
        let read_resource_tool = Tool::new(
            "platform__read_resource".to_string(),
            indoc! {r#"
                Read a resource from an extension.

                Resources allow extensions to share data that provide context to LLMs, such as
                files, database schemas, or application-specific information. This tool searches for the
                resource URI in the provided extension, and reads in the resource content. If no extension
                is provided, the tool will search all extensions for the resource.
            "#}.to_string(),
            json!({
                "type": "object",
                "required": ["uri"],
                "properties": {
                    "uri": {"type": "string", "description": "Resource URI"},
                    "extension_name": {"type": "string", "description": "Optional extension name"}
                }
            }),
            Some(ToolAnnotations {
                title: Some("Read a resource".to_string()),
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let list_resources_tool = Tool::new(
            "platform__list_resources".to_string(),
            indoc! {r#"
                List resources from an extension(s).

                Resources allow extensions to share data that provide context to LLMs, such as
                files, database schemas, or application-specific information. This tool lists resources
                in the provided extension, and returns a list for the user to browse. If no extension
                is provided, the tool will search all extensions for the resource.
            "#}.to_string(),
            json!({
                "type": "object",
                "properties": {
                    "extension_name": {"type": "string", "description": "Optional extension name"}
                }
            }),
            Some(ToolAnnotations {
                title: Some("List resources".to_string()),
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        if capabilities.supports_resources() {
            tools.push(read_resource_tool);
            tools.push(list_resources_tool);
        }

        let system_prompt = capabilities.get_system_prompt().await;

        // Set the user_message field in the span instead of creating a new event
        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        Ok(Box::pin(async_stream::try_stream! {
            let _reply_guard = reply_span.enter();
            loop {
                // Get completion from provider
                let (response, usage) = capabilities.provider().complete(
                    &system_prompt,
                    &messages,
                    &tools,
                ).await?;

                // record usage for the session in the session file
                if let Some(session) = session.clone() {
                    // TODO: track session_id in langfuse tracing
                    let session_file = session::get_path(session.id);
                    let mut metadata = session::read_metadata(&session_file)?;
                    metadata.working_dir = session.working_dir;
                    metadata.total_tokens = usage.usage.total_tokens;
                    metadata.input_tokens = usage.usage.input_tokens;
                    metadata.output_tokens = usage.usage.output_tokens;
                    // The message count is the number of messages in the session + 1 for the response
                    // The message count does not include the tool response till next iteration
                    metadata.message_count = messages.len() + 1;
                    session::update_metadata(&session_file, &metadata).await?;
                }

                // Yield the assistant's response
                yield response.clone();

                tokio::task::yield_now().await;

                // First collect any tool requests
                let tool_requests: Vec<&ToolRequest> = response.content
                    .iter()
                    .filter_map(|content| content.as_tool_request())
                    .collect();

                if tool_requests.is_empty() {
                    break;
                }

                // Then dispatch each in parallel
                let mut message_tool_response = Message::user();
                for request in tool_requests {
                    if let Ok(tool_call) = &request.tool_call {
                        // Check if it's a frontend tool
                        if capabilities.is_frontend_tool(&tool_call.name) {
                            // Send frontend tool request and wait for response
                            yield Message::assistant().with_frontend_tool_request(
                                request.id.clone(),
                                request.tool_call.clone()
                            );

                            // Wait for the result using our channel
                            if let Some((id, result)) = self.tool_result_rx.lock().await.recv().await {
                                message_tool_response = message_tool_response.with_tool_response(id, result);
                            }
                            continue;
                        }

                        // Handle regular tool calls
                        let result = capabilities.dispatch_tool_call(tool_call.clone()).await;
                        message_tool_response = message_tool_response.with_tool_response(
                            request.id.clone(),
                            result,
                        );
                    }
                }

                yield message_tool_response.clone();

                messages.push(response);
                messages.push(message_tool_response);
            }
        }))
    }

    async fn extend_system_prompt(&mut self, extension: String) {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.add_system_prompt_extension(extension);
    }

    async fn override_system_prompt(&mut self, template: String) {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.set_system_prompt_override(template);
    }

    async fn list_extension_prompts(&self) -> HashMap<String, Vec<Prompt>> {
        let capabilities = self.capabilities.lock().await;
        capabilities
            .list_prompts()
            .await
            .expect("Failed to list prompts")
    }

    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult> {
        let capabilities = self.capabilities.lock().await;

        // First find which extension has this prompt
        let prompts = capabilities
            .list_prompts()
            .await
            .map_err(|e| anyhow!("Failed to list prompts: {}", e))?;

        if let Some(extension) = prompts
            .iter()
            .find(|(_, prompt_list)| prompt_list.iter().any(|p| p.name == name))
            .map(|(extension, _)| extension)
        {
            return capabilities
                .get_prompt(extension, name, arguments)
                .await
                .map_err(|e| anyhow!("Failed to get prompt: {}", e));
        }

        Err(anyhow!("Prompt '{}' not found", name))
    }

    async fn get_plan_prompt(&self) -> anyhow::Result<String> {
        let mut capabilities = self.capabilities.lock().await;
        let tools = capabilities.get_prefixed_tools().await?;
        let tools_info = tools
            .into_iter()
            .map(|tool| ToolInfo::new(&tool.name, &tool.description, get_parameter_names(&tool)))
            .collect();

        let plan_prompt = capabilities.get_planning_prompt(tools_info).await;

        Ok(plan_prompt)
    }

    async fn provider(&self) -> Arc<Box<dyn Provider>> {
        let capabilities = self.capabilities.lock().await;
        capabilities.provider()
    }

    async fn handle_tool_result(&self, id: String, result: ToolResult<Vec<Content>>) {
        if let Err(e) = self.tool_result_tx.send((id, result)).await {
            tracing::error!("Failed to send tool result: {}", e);
        }
    }
}

register_agent!("reference", ReferenceAgent);
