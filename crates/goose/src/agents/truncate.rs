/// A truncate agent that truncates the conversation history when it exceeds the model's context limit
/// It makes no attempt to handle context limits, and cannot read resources
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{debug, error, instrument, warn};

use super::agent::SessionConfig;
use super::detect_read_only_tools;
use super::extension::ToolInfo;
use super::Agent;
use crate::agents::capabilities::{get_parameter_names, Capabilities};
use crate::agents::extension::{ExtensionConfig, ExtensionResult};
use crate::agents::ToolPermissionStore;
use crate::config::Config;
use crate::message::{Message, ToolRequest};
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::providers::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tool_json, OllamaInterpreter,
};
use crate::register_agent;
use crate::session;
use crate::token_counter::TokenCounter;
use crate::truncate::{truncate_messages, OldestFirstTruncation};
use anyhow::{anyhow, Result};
use indoc::indoc;
use mcp_core::prompt::Prompt;
use mcp_core::protocol::GetPromptResult;
use mcp_core::{tool::Tool, Content, ToolError};
use serde_json::{json, Value};
use std::time::Duration;

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

/// Truncate implementation of an Agent
pub struct TruncateAgent {
    capabilities: Mutex<Capabilities>,
    token_counter: TokenCounter,
    confirmation_tx: mpsc::Sender<(String, bool)>, // (request_id, confirmed)
    confirmation_rx: Mutex<mpsc::Receiver<(String, bool)>>,
}

impl TruncateAgent {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        // Create channel with buffer size 32 (adjust if needed)
        let (tx, rx) = mpsc::channel(32);

        Self {
            capabilities: Mutex::new(Capabilities::new(provider)),
            token_counter,
            confirmation_tx: tx,
            confirmation_rx: Mutex::new(rx),
        }
    }

    /// Truncates the messages to fit within the model's context window
    /// Ensures the last message is a user message and removes tool call-response pairs
    async fn truncate_messages(
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

        truncate_messages(
            messages,
            &mut token_counts,
            context_limit,
            &OldestFirstTruncation,
        )
    }

    async fn create_tool_future(
        capabilities: &Capabilities,
        tool_call: mcp_core::tool::ToolCall,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let output = capabilities.dispatch_tool_call(tool_call).await;
        (request_id, output)
    }
}

#[async_trait]
impl Agent for TruncateAgent {
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
    async fn handle_confirmation(&self, request_id: String, confirmed: bool) {
        if let Err(e) = self.confirmation_tx.send((request_id, confirmed)).await {
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
        );

        if capabilities.supports_resources() {
            tools.push(read_resource_tool);
            tools.push(list_resources_tool);
        }

        let config = capabilities.provider().get_model_config();
        let mut system_prompt = capabilities.get_system_prompt().await;
        let mut toolshim_tools = vec![];
        if config.toolshim {
            // If tool interpretation is enabled, modify the system prompt to instruct to return JSON tool requests
            system_prompt = modify_system_prompt_for_tool_json(&system_prompt, &tools);
            // make a copy of tools before empty
            toolshim_tools = tools.clone();
            // pass empty tools vector to provider completion since toolshim will handle tool calls instead
            tools = vec![];
        }

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
                    Ok((mut response, usage)) => {
                        // Post-process / structure the response only if tool interpretation is enabled
                        if config.toolshim {
                            let interpreter = OllamaInterpreter::new()
                                .map_err(|e| anyhow::anyhow!("Failed to create OllamaInterpreter: {}", e))?;

                            response = augment_message_with_tool_calls(&interpreter, response, &toolshim_tools).await?;
                        }

                        // record usage for the session in the session file
                        if let Some(session) = session.clone() {
                            // TODO: track session_id in langfuse tracing
                            let session_file = session::get_path(session.id);
                            let mut metadata = session::read_metadata(&session_file)?;
                            metadata.working_dir = session.working_dir;
                            metadata.total_tokens = usage.usage.total_tokens;
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
                            "approve" | "smart_approve" => {
                                let mut read_only_tools = Vec::new();
                                let mut needs_confirmation = Vec::<&ToolRequest>::new();
                                let mut approved_tools = Vec::new();

                                // First check permissions for all tools
                                let store = ToolPermissionStore::load()?;
                                for request in tool_requests.iter() {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        if let Some(allowed) = store.check_permission(request) {
                                            if allowed {
                                                // Instead of executing immediately, collect approved tools
                                                approved_tools.push((request.id.clone(), tool_call));
                                            } else {
                                                needs_confirmation.push(request);
                                            }
                                        } else {
                                            needs_confirmation.push(request);
                                        }
                                    }
                                }

                                // Only check read-only status for tools needing confirmation
                                if !needs_confirmation.is_empty() && mode == "smart_approve" {
                                    read_only_tools = detect_read_only_tools(&capabilities, needs_confirmation.clone()).await;
                                }

                                // Handle pre-approved and read-only tools in parallel
                                let mut tool_futures = Vec::new();

                                // Add pre-approved tools
                                for (request_id, tool_call) in approved_tools {
                                    let tool_future = Self::create_tool_future(&capabilities, tool_call, request_id.clone());
                                    tool_futures.push(tool_future);
                                }

                                // Process read-only tools
                                for request in &needs_confirmation {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        // Skip confirmation if the tool_call.name is in the read_only_tools list
                                        if read_only_tools.contains(&tool_call.name) {
                                            let tool_future = Self::create_tool_future(&capabilities, tool_call, request.id.clone());
                                            tool_futures.push(tool_future);
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
                                            while let Some((req_id, confirmed)) = rx.recv().await {
                                                if req_id == request.id {
                                                    // Store the user's response with 30-day expiration
                                                    let mut store = ToolPermissionStore::load()?;
                                                    store.record_permission(request, confirmed, Some(Duration::from_secs(30 * 24 * 60 * 60)))?;

                                                    if confirmed {
                                                        // Add this tool call to the futures collection
                                                        let tool_future = Self::create_tool_future(&capabilities, tool_call, request.id.clone());
                                                        tool_futures.push(tool_future);
                                                    } else {
                                                        // User declined - add declined response
                                                        message_tool_response = message_tool_response.with_tool_response(
                                                            request.id.clone(),
                                                            Ok(vec![Content::text("User declined to run this tool. Don't try to make the same tool call again. If there is no other ways to do it, it is ok to stop.")]),
                                                        );
                                                    }
                                                    break; // Exit the loop once the matching `req_id` is found
                                                }
                                            }
                                        }
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
                            },
                            "chat" => {
                                // Skip all tool calls in chat mode
                                for request in &tool_requests {
                                    message_tool_response = message_tool_response.with_tool_response(
                                        request.id.clone(),
                                        Ok(vec![Content::text(
                                            "Let the user know the tool call was skipped in Goose chat mode. \
                                            DO NOT apologize for skipping the tool call. DO NOT say sorry. \
                                            Provide an explanation of what the tool call would do, structured as a \
                                            plan for the user. Again, DO NOT apologize. \
                                            **Example Plan:**\n \
                                            1. **Identify Task Scope** - Determine the purpose and expected outcome.\n \
                                            2. **Outline Steps** - Break down the steps.\n \
                                            If needed, adjust the explanation based on user preferences or questions."
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
                                        let tool_future = Self::create_tool_future(&capabilities, tool_call, request.id.clone());
                                        tool_futures.push(tool_future);
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

                        if let Err(err) = self.truncate_messages(&mut messages, estimate_factor, &system_prompt, &mut tools).await {
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
}

register_agent!("truncate", TruncateAgent);
