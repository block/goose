use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::stream::BoxStream;

use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, instrument, warn};

use crate::agents::extension::{ExtensionConfig, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::types::ToolResultReceiver;
use crate::config::{Config, ExtensionConfigManager};
use crate::message::{Message, MessageContent, ToolRequest};
use crate::permission::{
    detect_read_only_tools, Permission, PermissionConfirmation, ToolPermissionStore,
};
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::providers::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tool_json, OllamaInterpreter,
};
use crate::session;
use crate::token_counter::TokenCounter;
use crate::truncate::{truncate_messages, OldestFirstTruncation};

use mcp_core::{
    prompt::Prompt, protocol::GetPromptResult, tool::Tool, Content, ToolError, ToolResult,
};

use crate::agents::platform_tools::{
    self, PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
    PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::agents::prompt_manager::PromptManager;
use crate::agents::types::SessionConfig;

use super::platform_tools::PLATFORM_ENABLE_EXTENSION_TOOL_NAME;
use super::types::FrontendTool;

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

/// The main goose Agent
pub struct Agent {
    provider: Arc<dyn Provider>,
    extension_manager: Mutex<ExtensionManager>,
    frontend_tools: HashMap<String, FrontendTool>,
    frontend_instructions: Option<String>,
    prompt_manager: PromptManager,
    token_counter: TokenCounter,
    confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    tool_result_rx: ToolResultReceiver,
}

impl Agent {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        // Create channels with buffer size 32 (adjust if needed)
        let (confirm_tx, confirm_rx) = mpsc::channel(32);
        let (tool_tx, tool_rx) = mpsc::channel(32);

        Self {
            provider,
            extension_manager: Mutex::new(ExtensionManager::new()),
            frontend_tools: HashMap::new(),
            frontend_instructions: None,
            prompt_manager: PromptManager::new(),
            token_counter,
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
        }
    }

    /// Get a reference count clone to the provider
    pub fn provider(&self) -> Arc<dyn Provider> {
        Arc::clone(&self.provider)
    }

    /// Check if a tool is a frontend tool
    pub fn is_frontend_tool(&self, name: &str) -> bool {
        self.frontend_tools.contains_key(name)
    }

    /// Get a reference to a frontend tool
    pub fn get_frontend_tool(&self, name: &str) -> Option<&FrontendTool> {
        self.frontend_tools.get(name)
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(&mut self) -> ExtensionResult<Vec<Tool>> {
        let mut tools = self
            .extension_manager
            .lock()
            .await
            .get_prefixed_tools()
            .await?;

        // Add frontend tools directly - they don't need prefixing since they're already uniquely named
        for frontend_tool in self.frontend_tools.values() {
            tools.push(frontend_tool.tool.clone());
        }

        Ok(tools)
    }

    /// Dispatch a single tool call to the appropriate client
    #[instrument(skip(tool_call, extension_manager, request_id), fields(input, output))]
    async fn create_tool_future(
        extension_manager: &ExtensionManager,
        tool_call: mcp_core::tool::ToolCall,
        is_frontend_tool: bool,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let result = if tool_call.name == PLATFORM_READ_RESOURCE_TOOL_NAME {
            // Check if the tool is read_resource and handle it separately
            extension_manager
                .read_resource(tool_call.arguments.clone())
                .await
        } else if tool_call.name == PLATFORM_LIST_RESOURCES_TOOL_NAME {
            extension_manager
                .list_resources(tool_call.arguments.clone())
                .await
        } else if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
            extension_manager.search_available_extensions().await
        } else if is_frontend_tool {
            // For frontend tools, return an error indicating we need frontend execution
            Err(ToolError::ExecutionError(
                "Frontend tool execution required".to_string(),
            ))
        } else {
            extension_manager
                .dispatch_tool_call(tool_call.clone())
                .await
        };

        debug!(
            "input" = serde_json::to_string(&tool_call).unwrap(),
            "output" = serde_json::to_string(&result).unwrap(),
        );

        (request_id, result)
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
        let context_limit = self.provider.get_model_config().context_limit();

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

    async fn enable_extension(
        extension_manager: &mut ExtensionManager,
        extension_name: String,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let config = match ExtensionConfigManager::get_config_by_name(&extension_name) {
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

        let result = extension_manager
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

    pub async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()> {
        match &extension {
            ExtensionConfig::Frontend {
                name: _,
                tools,
                instructions,
            } => {
                // For frontend tools, just store them in the frontend_tools map
                for tool in tools {
                    let frontend_tool = FrontendTool {
                        name: tool.name.clone(),
                        tool: tool.clone(),
                    };
                    self.frontend_tools.insert(tool.name.clone(), frontend_tool);
                }
                // Store instructions if provided, using "frontend" as the key
                if let Some(instructions) = instructions {
                    self.frontend_instructions = Some(instructions.clone());
                } else {
                    // Default frontend instructions if none provided
                    self.frontend_instructions = Some(
                        "The following tools are provided directly by the frontend and will be executed by the frontend when called.".to_string(),
                    );
                }
            }
            _ => {
                let mut extension_manager = self.extension_manager.lock().await;
                let _ = extension_manager.add_extension(extension).await;
            }
        };

        Ok(())
    }

    pub async fn list_tools(&self) -> Vec<Tool> {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager
            .get_prefixed_tools()
            .await
            .unwrap_or_default()
    }

    pub async fn remove_extension(&mut self, name: &str) {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager
            .remove_extension(name)
            .await
            .expect("Failed to remove extension");
    }

    pub async fn list_extensions(&self) -> Vec<String> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_extensions()
            .await
            .expect("Failed to list extensions")
    }

    /// Handle a confirmation response for a tool request
    pub async fn handle_confirmation(
        &self,
        request_id: String,
        confirmation: PermissionConfirmation,
    ) {
        if let Err(e) = self.confirmation_tx.send((request_id, confirmation)).await {
            error!("Failed to send confirmation: {}", e);
        }
    }

    #[instrument(skip(self, messages, session), fields(user_message))]
    pub async fn reply(
        &self,
        messages: &[Message],
        session: Option<SessionConfig>,
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message>>> {
        let mut messages = messages.to_vec();
        let reply_span = tracing::Span::current();
        let mut extension_manager = self.extension_manager.lock().await;
        let mut tools = extension_manager.get_prefixed_tools().await?;
        let mut truncation_attempt: usize = 0;

        // Load settings from config
        let config = Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());

        // we add in the 2 resource tools if any extensions support resources
        // TODO: make sure there is no collision with another extension's tool name
        if extension_manager.supports_resources() {
            tools.push(platform_tools::read_resource_tool());
            tools.push(platform_tools::list_resources_tool());
        }
        tools.push(platform_tools::search_available_extensions_tool());
        tools.push(platform_tools::enable_extension_tool());

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

        let config = self.provider.get_model_config();
        let extensions_info = extension_manager.get_extensions_info().await;
        let mut system_prompt = self
            .prompt_manager
            .build_system_prompt(extensions_info, self.frontend_instructions.clone());
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
                match self.provider().complete(
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
                                        return !self.is_frontend_tool(&tool_call.name);
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
                                if self.is_frontend_tool(&tool_call.name) {
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
                                    .map(|call| call.name == PLATFORM_ENABLE_EXTENSION_TOOL_NAME)
                                    .unwrap_or(false)
                            });

                        let (search_extension_requests, _non_search_extension_requests): (Vec<&ToolRequest>, Vec<&ToolRequest>) = remaining_requests.clone()
                            .into_iter()
                            .partition(|req| {
                                req.tool_call.as_ref()
                                    .map(|call| call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME)
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
                                detected_read_only_tools = detect_read_only_tools(self.provider(), llm_detect_candidates.clone()).await;
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
                                                let install_result = Self::enable_extension(&mut extension_manager, extension_name, request.id.clone()).await;
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
                                    let is_frontend_tool = self.is_frontend_tool(&tool_call.name);
                                    // Skip confirmation if the tool_call.name is in the read_only_tools list
                                    if detected_read_only_tools.contains(&tool_call.name) {
                                        let tool_future = Self::create_tool_future(&extension_manager, tool_call, is_frontend_tool, request.id.clone());
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
                                                    let tool_future = Self::create_tool_future(&extension_manager, tool_call, is_frontend_tool, request.id.clone());
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
                                let extensions_info = extension_manager.get_extensions_info().await;
                                system_prompt = self.prompt_manager.build_system_prompt(extensions_info, self.frontend_instructions.clone());
                                tools = extension_manager.get_prefixed_tools().await?;
                            }
                        }

                        if mode.as_str() == "auto" || !search_extension_requests.is_empty() {
                            let mut tool_futures = Vec::new();
                            // Process non_enable_extension_requests and search_extension_requests without duplicates
                            let mut processed_ids = HashSet::new();

                            for request in non_enable_extension_requests.iter().chain(search_extension_requests.iter()) {
                                if processed_ids.insert(request.id.clone()) {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        let is_frontend_tool = self.is_frontend_tool(&tool_call.name);
                                        let tool_future = Self::create_tool_future(&extension_manager, tool_call, is_frontend_tool, request.id.clone());
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
                                        tool_call.name != PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME
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
                        drop(extension_manager);

                        if let Err(err) = self.truncate_messages(&mut messages, estimate_factor, &system_prompt, &mut tools).await {
                            yield Message::assistant().with_text(format!("Error: Unable to truncate messages to stay within context limit. \n\nRan into this error: {}.\n\nPlease start a new session with fresh context and try again.", err));
                            break;
                        }


                        // Re-acquire the lock
                        extension_manager = self.extension_manager.lock().await;

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

    /// Extend the system prompt with one line of additional instruction
    pub async fn extend_system_prompt(&mut self, instruction: String) {
        self.prompt_manager.add_system_prompt_extra(instruction);
    }

    /// Override the system prompt with a custom template
    pub async fn override_system_prompt(&mut self, template: String) {
        self.prompt_manager.set_system_prompt_override(template);
    }

    pub async fn list_extension_prompts(&self) -> HashMap<String, Vec<Prompt>> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_prompts()
            .await
            .expect("Failed to list prompts")
    }

    pub async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult> {
        let extension_manager = self.extension_manager.lock().await;

        // First find which extension has this prompt
        let prompts = extension_manager
            .list_prompts()
            .await
            .map_err(|e| anyhow!("Failed to list prompts: {}", e))?;

        if let Some(extension) = prompts
            .iter()
            .find(|(_, prompt_list)| prompt_list.iter().any(|p| p.name == name))
            .map(|(extension, _)| extension)
        {
            return extension_manager
                .get_prompt(extension, name, arguments)
                .await
                .map_err(|e| anyhow!("Failed to get prompt: {}", e));
        }

        Err(anyhow!("Prompt '{}' not found", name))
    }

    pub async fn get_plan_prompt(&self) -> anyhow::Result<String> {
        let mut extension_manager = self.extension_manager.lock().await;
        let tools = extension_manager.get_prefixed_tools().await?;
        let tools_info = tools
            .into_iter()
            .map(|tool| {
                ToolInfo::new(
                    &tool.name,
                    &tool.description,
                    get_parameter_names(&tool),
                    None,
                )
            })
            .collect();

        let plan_prompt = extension_manager.get_planning_prompt(tools_info).await;

        Ok(plan_prompt)
    }

    pub async fn handle_tool_result(&self, id: String, result: ToolResult<Vec<Content>>) {
        if let Err(e) = self.tool_result_tx.send((id, result)).await {
            tracing::error!("Failed to send tool result: {}", e);
        }
    }
}
