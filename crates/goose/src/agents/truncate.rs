/// A truncate agent that truncates the conversation history when it exceeds the model's context limit
/// It makes no attempt to handle context limits, and cannot read resources
use async_trait::async_trait;
use futures::stream::BoxStream;
use mcp_core::tool::{Tool, ToolAnnotations};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::{debug, error, instrument, warn};

use super::agent::SessionConfig;
use super::extension::ToolInfo;
use super::types::ToolResultReceiver;
use super::Agent;
use crate::agents::capabilities::{get_parameter_names, Capabilities};
use crate::agents::extension::{ExtensionConfig, ExtensionResult};
use crate::config::{Config, ExtensionManager};
use crate::message::{Message, MessageContent, ToolRequest};
use crate::permission::detect_read_only_tools;
use crate::permission::Permission;
use crate::permission::PermissionConfirmation;
use crate::permission::ToolPermissionStore;
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
use mcp_core::{prompt::Prompt, protocol::GetPromptResult, Content, ToolError, ToolResult};
use serde_json::{json, Value};

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

/// Truncate implementation of an Agent
pub struct TruncateAgent {
    capabilities: Mutex<Capabilities>,
    token_counter: TokenCounter,
    confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    tool_result_rx: ToolResultReceiver,
}

impl TruncateAgent {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        // Create channels with buffer size 32 (adjust if needed)
        let (confirm_tx, confirm_rx) = mpsc::channel(32);
        let (tool_tx, tool_rx) = mpsc::channel(32);

        Self {
            capabilities: Mutex::new(Capabilities::new(provider)),
            token_counter,
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
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

    async fn enable_extension(
        capabilities: &mut Capabilities,
        extension_name: String,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let config = match ExtensionManager::get_config_by_name(&extension_name) {
            Ok(Some(config)) => config,
            Ok(None) => {
                return (
                    request_id,
                    Err(ToolError::ExecutionError(format!(
                        "Extension '{}' not found. Please check the extension name and try again.",
                        extension_name
                    ))),
                )
            }
            Err(e) => {
                return (
                    request_id,
                    Err(ToolError::ExecutionError(format!(
                        "Failed to get extension config: {}",
                        e
                    ))),
                )
            }
        };

        let result = capabilities
            .add_extension(config)
            .await
            .map(|_| {
                vec![Content::text(format!(
                    "The extension '{}' has been installed successfully",
                    extension_name
                ))]
            })
            .map_err(|e| ToolError::ExecutionError(e.to_string()));

        (request_id, result)
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

        let search_available_extensions = Tool::new(
            "platform__search_available_extensions".to_string(),
            "Searches for additional extensions available to help complete tasks.
            Use this tool when you're unable to find a specific feature or functionality you need to complete your task, or when standard approaches aren't working.
            These extensions might provide the exact tools needed to solve your problem.
            If you find a relevant one, consider using your tools to enable it.".to_string(),
            json!({
                "type": "object",
                "required": [],
                "properties": {}
            }),
            Some(ToolAnnotations {
                title: Some("Discover extensions".to_string()),
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let enable_extension_tool = Tool::new(
            "platform__enable_extension".to_string(),
            "Enable extensions to help complete tasks.
            Enable an extension by providing the extension name.
            "
            .to_string(),
            json!({
                "type": "object",
                "required": ["extension_name"],
                "properties": {
                    "extension_name": {"type": "string", "description": "The name of the extension to enable"}
                }
            }),
            Some(ToolAnnotations {
                title: Some("Enable extensions".to_string()),
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        if capabilities.supports_resources() {
            tools.push(read_resource_tool);
            tools.push(list_resources_tool);
        }
        tools.push(search_available_extensions);
        tools.push(enable_extension_tool);

        let (tools_with_readonly_annotation, tools_without_annotation): (Vec<String>, Vec<String>) =
            tools.iter().fold((vec![], vec![]), |mut acc, tool| {
                match &tool.annotations {
                    Some(annotations) => {
                        if annotations.read_only_hint {
                            acc.0.push(tool.name.clone());
                        }
                    }
                    None => {
                        acc.1.push(tool.name.clone());
                    }
                }
                acc
            });

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
                            metadata.input_tokens = usage.usage.input_tokens;
                            metadata.output_tokens = usage.usage.output_tokens;
                            // The message count is the number of messages in the session + 1 for the response
                            // The message count does not include the tool response till next iteration
                            metadata.message_count = messages.len() + 1;
                            session::update_metadata(&session_file, &metadata).await?;
                        }

                        // Reset truncation attempt
                        truncation_attempt = 0;

                        // Yield the assistant's response, but filter out frontend tool requests that we'll process separately
                        let filtered_response = Message {
                            role: response.role.clone(),
                            created: response.created,
                            content: response.content.iter().filter(|c| {
                                if let MessageContent::ToolRequest(req) = c {
                                    // Only filter out frontend tool requests
                                    if let Ok(tool_call) = &req.tool_call {
                                        return !capabilities.is_frontend_tool(&tool_call.name);
                                    }
                                }
                                true
                            }).cloned().collect(),
                        };
                        yield filtered_response.clone();

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

                        // First handle any frontend tool requests
                        let mut remaining_requests = Vec::new();
                        for request in &tool_requests {
                            if let Ok(tool_call) = request.tool_call.clone() {
                                if capabilities.is_frontend_tool(&tool_call.name) {
                                    // Send frontend tool request and wait for response
                                    yield Message::assistant().with_frontend_tool_request(
                                        request.id.clone(),
                                        Ok(tool_call.clone())
                                    );

                                    if let Some((id, result)) = self.tool_result_rx.lock().await.recv().await {
                                        message_tool_response = message_tool_response.with_tool_response(id, result);
                                    }
                                } else {
                                    remaining_requests.push(request);
                                }
                            } else {
                                remaining_requests.push(request);
                            }
                        }

                        // Split tool requests into enable_extension and others
                        let (enable_extension_requests, non_enable_extension_requests): (Vec<&ToolRequest>, Vec<&ToolRequest>) = remaining_requests.clone()
                            .into_iter()
                            .partition(|req| {
                                req.tool_call.as_ref()
                                    .map(|call| call.name == "platform__enable_extension")
                                    .unwrap_or(false)
                            });

                        let (search_extension_requests, _non_search_extension_requests): (Vec<&ToolRequest>, Vec<&ToolRequest>) = remaining_requests.clone()
                            .into_iter()
                            .partition(|req| {
                                req.tool_call.as_ref()
                                    .map(|call| call.name == "platform__search_available_extensions")
                                    .unwrap_or(false)
                            });

                        // Clone goose_mode once before the match to avoid move issues
                        let mode = goose_mode.clone();

                        // If there are install extension requests, always require confirmation
                        // or if goose_mode is approve or smart_approve, check permissions for all tools
                        if !enable_extension_requests.is_empty() || mode.as_str() == "approve" || mode.as_str() == "smart_approve" {
                            let mut needs_confirmation = Vec::<&ToolRequest>::new();
                            let mut approved_tools = Vec::new();
                            let mut llm_detect_candidates = Vec::<&ToolRequest>::new();
                            let mut detected_read_only_tools = Vec::new();

                            // If approve mode or smart approve mode, check permissions for all tools
                            if mode.as_str() == "approve" || mode.as_str() == "smart_approve" {
                                let store = ToolPermissionStore::load()?;
                                for request in &non_enable_extension_requests {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        // Regular permission checking for other tools
                                        if tools_with_readonly_annotation.contains(&tool_call.name) {
                                            approved_tools.push((request.id.clone(), tool_call));
                                        } else if let Some(allowed) = store.check_permission(request) {
                                            if allowed {
                                                // Instead of executing immediately, collect approved tools
                                                approved_tools.push((request.id.clone(), tool_call));
                                            } else {
                                                // If the tool doesn't have any annotation, we can use llm-as-a-judge to check permission.
                                                if tools_without_annotation.contains(&tool_call.name) {
                                                    llm_detect_candidates.push(request);
                                                }
                                                needs_confirmation.push(request);
                                            }
                                        } else {
                                            if tools_without_annotation.contains(&tool_call.name) {
                                                llm_detect_candidates.push(request);
                                            }
                                            needs_confirmation.push(request);
                                        }
                                    }
                                }
                            }
                            // Only check read-only status for tools needing confirmation
                            if !llm_detect_candidates.is_empty() && mode == "smart_approve" {
                                detected_read_only_tools = detect_read_only_tools(&capabilities, llm_detect_candidates.clone()).await;
                                // Remove install extensions from read-only tools
                                if !enable_extension_requests.is_empty() {
                                    detected_read_only_tools.retain(|tool_name| {
                                        !enable_extension_requests.iter().any(|req| {
                                            req.tool_call.as_ref()
                                                .map(|call| call.name == *tool_name)
                                                .unwrap_or(false)
                                        })
                                    });
                                }
                            }

                            // Handle pre-approved and read-only tools in parallel
                            let mut tool_futures = Vec::new();
                            let mut install_results = Vec::new();

                            // Handle install extension requests
                            for request in &enable_extension_requests {
                                if let Ok(tool_call) = request.tool_call.clone() {
                                    let confirmation = Message::user().with_enable_extension_request(
                                        request.id.clone(),
                                        tool_call.arguments.get("extension_name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string()
                                    );
                                    yield confirmation;

                                    let mut rx = self.confirmation_rx.lock().await;
                                    while let Some((req_id, extension_confirmation)) = rx.recv().await {
                                        if req_id == request.id {
                                            if extension_confirmation.permission == Permission::AllowOnce || extension_confirmation.permission == Permission::AlwaysAllow {
                                                let extension_name = tool_call.arguments.get("extension_name")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let install_result = Self::enable_extension(&mut capabilities, extension_name, request.id.clone()).await;
                                                install_results.push(install_result);
                                            }
                                            break;
                                        }
                                    }
                                }
                            }

                            // Process read-only tools
                            for request in &needs_confirmation {
                                if let Ok(tool_call) = request.tool_call.clone() {
                                    // Skip confirmation if the tool_call.name is in the read_only_tools list
                                    if detected_read_only_tools.contains(&tool_call.name) {
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
                                        while let Some((req_id, tool_confirmation)) = rx.recv().await {
                                            if req_id == request.id {
                                                let confirmed = tool_confirmation.permission == Permission::AllowOnce || tool_confirmation.permission == Permission::AlwaysAllow;
                                                if confirmed {
                                                    // Add this tool call to the futures collection
                                                    let tool_future = Self::create_tool_future(&capabilities, tool_call, request.id.clone());
                                                    tool_futures.push(tool_future);
                                                } else {
                                                    // User declined - add declined response
                                                    message_tool_response = message_tool_response.with_tool_response(
                                                        request.id.clone(),
                                                        Ok(vec![Content::text(
                                                            "The user has declined to run this tool. \
                                                            DO NOT attempt to call this tool again. \
                                                            If there are no alternative methods to proceed, clearly explain the situation and STOP.")]),
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

                            // Check if any install results had errors before processing them
                            let all_successful = !install_results.iter().any(|(_, result)| result.is_err());

                            for (request_id, output) in install_results {
                                message_tool_response = message_tool_response.with_tool_response(
                                    request_id,
                                    output
                                );
                            }

                            // Update system prompt and tools if all installations were successful
                            if all_successful {
                                system_prompt = capabilities.get_system_prompt().await;
                                tools = capabilities.get_prefixed_tools().await?;
                            }
                        }

                        if mode.as_str() == "auto" || !search_extension_requests.is_empty() {
                            let mut tool_futures = Vec::new();
                            // Process non_enable_extension_requests and search_extension_requests without duplicates
                            let mut processed_ids = HashSet::new();

                            for request in non_enable_extension_requests.iter().chain(search_extension_requests.iter()) {
                                if processed_ids.insert(request.id.clone()) {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        let tool_future = Self::create_tool_future(&capabilities, tool_call, request.id.clone());
                                        tool_futures.push(tool_future);
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
                        }

                        if mode.as_str() == "chat" {
                            // Skip all tool calls in chat mode
                            // Skip search extension requests since they were already processed
                            let non_search_non_enable_extension_requests = non_enable_extension_requests.iter()
                                .filter(|req| {
                                    if let Ok(tool_call) = &req.tool_call {
                                        tool_call.name != "platform__search_available_extensions"
                                    } else {
                                        true
                                    }
                                });
                            for request in non_search_non_enable_extension_requests {
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

    async fn handle_tool_result(&self, id: String, result: ToolResult<Vec<Content>>) {
        if let Err(e) = self.tool_result_tx.send((id, result)).await {
            tracing::error!("Failed to send tool result: {}", e);
        }
    }
}

register_agent!("truncate", TruncateAgent);
