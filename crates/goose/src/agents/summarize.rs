/// A summarize agent that lets the model summarize the conversation when the history exceeds the
/// model's context limit. If the model fails to summarize, then it falls back to the legacy
/// truncation method. Still cannot read resources.
use async_trait::async_trait;
use futures::stream::BoxStream;
use mcp_core::tool::ToolAnnotations;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{debug, error, instrument, warn};

use super::agent::SessionConfig;
use super::capabilities::get_parameter_names;
use super::extension::ToolInfo;
use super::Agent;
use crate::agents::capabilities::Capabilities;
use crate::agents::extension::{ExtensionConfig, ExtensionResult};
use crate::config::Config;
use crate::memory_condense::condense_messages;
use crate::message::{Message, ToolRequest};
use crate::permission::detect_read_only_tools;
use crate::permission::Permission;
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::register_agent;
use crate::session;
use crate::token_counter::TokenCounter;
use crate::truncate::{truncate_messages, OldestFirstTruncation};
use anyhow::{anyhow, Result};
use indoc::indoc;
use mcp_core::{prompt::Prompt, protocol::GetPromptResult, tool::Tool, Content, ToolResult};
use serde_json::{json, Value};

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

/// Summarize implementation of an Agent
pub struct SummarizeAgent {
    capabilities: Mutex<Capabilities>,
    token_counter: TokenCounter,
    confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
}

impl SummarizeAgent {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        // Create channels with buffer size 32 (adjust if needed)
        let (confirm_tx, confirm_rx) = mpsc::channel(32);
        let (tool_tx, _tool_rx) = mpsc::channel(32);

        Self {
            capabilities: Mutex::new(Capabilities::new(provider)),
            token_counter,
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
        }
    }

    /// Truncates the messages to fit within the model's context window
    /// Ensures the last message is a user message and removes tool call-response pairs
    async fn summarize_messages(
        &self,
        messages: &mut Vec<Message>,
        estimate_factor: f32,
        system_prompt: &str,
        tools: &mut Vec<Tool>,
    ) -> anyhow::Result<()> {
        // Model's actual context limit
        let context_limit = self
            .capabilities
            .lock()
            .await
            .provider()
            .get_model_config()
            .context_limit();

        // Our conservative estimate of the **target** context limit
        // Our token count is an estimate since model providers often don't provide the tokenizer (eg. Claude)
        let context_limit = (context_limit as f32 * estimate_factor) as usize;

        // Take into account the system prompt, and our tools input and subtract that from the
        // remaining context limit
        let system_prompt_token_count = self.token_counter.count_tokens(system_prompt);
        let tools_token_count = self.token_counter.count_tokens_for_tools(tools.as_slice());

        // Check if system prompt + tools exceed our context limit
        let remaining_tokens = context_limit
            .checked_sub(system_prompt_token_count)
            .and_then(|remaining| remaining.checked_sub(tools_token_count))
            .ok_or_else(|| {
                anyhow::anyhow!("System prompt and tools exceed estimated context limit")
            })?;

        let context_limit = remaining_tokens;

        // Calculate current token count of each message, use count_chat_tokens to ensure we
        // capture the full content of the message, include ToolRequests and ToolResponses
        let mut token_counts: Vec<usize> = messages
            .iter()
            .map(|msg| {
                self.token_counter
                    .count_chat_tokens("", std::slice::from_ref(msg), &[])
            })
            .collect();

        let capabilities_guard = self.capabilities.lock().await;
        if condense_messages(
            &capabilities_guard,
            &self.token_counter,
            messages,
            &mut token_counts,
            context_limit,
        )
        .await
        .is_err()
        {
            // Fallback to the legacy truncator if the model fails to condense the messages.
            truncate_messages(
                messages,
                &mut token_counts,
                context_limit,
                &OldestFirstTruncation,
            )
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl Agent for SummarizeAgent {
    async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()> {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.add_extension(extension).await
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

    /// Handle a confirmation response for a tool request
    async fn handle_confirmation(&self, request_id: String, confirmation: PermissionConfirmation) {
        if let Err(e) = self.confirmation_tx.send((request_id, confirmation)).await {
            error!("Failed to send confirmation: {}", e);
        }
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
        let mut truncation_attempt: usize = 0;

        // Load settings from config
        let config = Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());

        // we add in the 2 resource tools if any extensions support resources
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
                match capabilities.provider().complete(
                    &system_prompt,
                    &messages,
                    &tools,
                ).await {
                    Ok((response, usage)) => {
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

                        // Reset truncation attempt
                        truncation_attempt = 0;

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

                        // Process tool requests depending on goose_mode
                        let mut message_tool_response = Message::user();
                        // Clone goose_mode once before the match to avoid move issues
                        let mode = goose_mode.clone();
                        match mode.as_str() {
                            "approve" => {
                                let read_only_tools = detect_read_only_tools(&capabilities, tool_requests.clone()).await;
                                for request in &tool_requests {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        // Skip confirmation if the tool_call.name is in the read_only_tools list
                                        if read_only_tools.contains(&tool_call.name) {
                                            let output = capabilities.dispatch_tool_call(tool_call).await;
                                                    message_tool_response = message_tool_response.with_tool_response(
                                                        request.id.clone(),
                                                        output,
                                                    );
                                        } else {
                                            let confirmation = Message::user().with_tool_confirmation_request(
                                                request.id.clone(),
                                                tool_call.name.clone(),
                                                tool_call.arguments.clone(),
                                                Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
                                            );
                                            yield confirmation;

                                            // Wait for confirmation response through the channel
                                            let mut rx = self.confirmation_rx.lock().await;
                                            // Loop the recv until we have a matched req_id due to potential duplicate messages.
                                            while let Some((req_id, tool_confirmation)) = rx.recv().await {
                                                if req_id == request.id {
                                                    if tool_confirmation.permission == Permission::AllowOnce || tool_confirmation.permission == Permission::AlwaysAllow {
                                                        // User approved - dispatch the tool call
                                                        let output = capabilities.dispatch_tool_call(tool_call).await;
                                                        message_tool_response = message_tool_response.with_tool_response(
                                                            request.id.clone(),
                                                            output,
                                                        );
                                                    } else {
                                                        // User declined - add declined response
                                                        message_tool_response = message_tool_response.with_tool_response(
                                                            request.id.clone(),
                                                            Ok(vec![Content::text("User declined to run this tool.")]),
                                                        );
                                                    }
                                                    break; // Exit the loop once the matching `req_id` is found
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "chat" => {
                                // Skip all tool calls in chat mode
                                for request in &tool_requests {
                                    message_tool_response = message_tool_response.with_tool_response(
                                        request.id.clone(),
                                        Ok(vec![Content::text(
                                            "The following tool call was skipped in Goose chat mode. \
                                            In chat mode, you cannot run tool calls, instead, you can \
                                            only provide a detailed plan to the user. Provide an \
                                            explanation of the proposed tool call as if it were a plan. \
                                            Only if the user asks, provide a short explanation to the \
                                            user that they could consider running the tool above on \
                                            their own or with a different goose mode."
                                        )]),
                                    );
                                }
                            },
                            _ => {
                                if mode != "auto" {
                                    warn!("Unknown GOOSE_MODE: {mode:?}. Defaulting to 'auto' mode.");
                                }
                                // Process tool requests in parallel
                                let mut tool_futures = Vec::new();
                                for request in &tool_requests {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        tool_futures.push(async {
                                            let output = capabilities.dispatch_tool_call(tool_call).await;
                                            (request.id.clone(), output)
                                        });
                                    }
                                }
                                // Wait for all tool calls to complete
                                let results = futures::future::join_all(tool_futures).await;
                                for (request_id, output) in results {
                                    message_tool_response = message_tool_response.with_tool_response(
                                        request_id,
                                        output,
                                    );
                                }
                            }
                        }

                        yield message_tool_response.clone();

                        messages.push(response);
                        messages.push(message_tool_response);
                    },
                    Err(ProviderError::ContextLengthExceeded(_)) => {
                        if truncation_attempt >= MAX_TRUNCATION_ATTEMPTS {
                            // Create an error message & terminate the stream
                            // the previous message would have been a user message (e.g. before any tool calls, this is just after the input message.
                            // at the start of a loop after a tool call, it would be after a tool_use assistant followed by a tool_result user)
                            yield Message::assistant().with_text("Error: Context length exceeds limits even after multiple attempts to truncate. Please start a new session with fresh context and try again.");
                            break;
                        }

                        truncation_attempt += 1;
                        warn!("Context length exceeded. Truncation Attempt: {}/{}.", truncation_attempt, MAX_TRUNCATION_ATTEMPTS);

                        // Decay the estimate factor as we make more truncation attempts
                        // Estimate factor decays like this over time: 0.9, 0.81, 0.729, ...
                        let estimate_factor: f32 = ESTIMATE_FACTOR_DECAY.powi(truncation_attempt as i32);

                        // release the lock before truncation to prevent deadlock
                        drop(capabilities);

                        if let Err(err) = self.summarize_messages(&mut messages, estimate_factor, &system_prompt, &mut tools).await {
                            yield Message::assistant().with_text(format!("Error: Unable to truncate messages to stay within context limit. \n\nRan into this error: {}.\n\nPlease start a new session with fresh context and try again.", err));
                            break;
                        }


                        // Re-acquire the lock
                        capabilities = self.capabilities.lock().await;

                        // Retry the loop after truncation
                        continue;
                    },
                    Err(e) => {
                        // Create an error message & terminate the stream
                        error!("Error: {}", e);
                        yield Message::assistant().with_text(format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error."));
                        break;
                    }
                }

                // Yield control back to the scheduler to prevent blocking
                tokio::task::yield_now().await;
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

register_agent!("summarize", SummarizeAgent);
