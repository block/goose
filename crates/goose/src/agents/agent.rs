use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::stream::BoxStream;
use futures::TryStreamExt;

use crate::config::{Config, ExtensionConfigManager, PermissionManager};
use crate::message::Message;
use crate::permission::permission_judge::check_tool_permissions;
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::recipe::{Author, Recipe};
use crate::tool_monitor::{ToolCall, ToolMonitor};
use regex::Regex;
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, instrument};

use crate::agents::extension::{ExtensionConfig, ExtensionError, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
    PLATFORM_READ_RESOURCE_TOOL_NAME, PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::agents::prompt_manager::PromptManager;
use crate::agents::router_tool_selector::{
    create_tool_selector, RouterToolSelectionStrategy, RouterToolSelector,
};
use crate::agents::router_tools::ROUTER_VECTOR_SEARCH_TOOL_NAME;
use crate::agents::tool_vectordb::generate_table_id;
use crate::agents::types::SessionConfig;
use crate::agents::types::{FrontendTool, ToolResultReceiver};
use mcp_core::{
    prompt::Prompt, protocol::GetPromptResult, tool::Tool, Content, ToolError, ToolResult,
};

use super::platform_tools;
use super::router_tools;
use super::tool_execution::{ToolFuture, CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};

/// The main goose Agent
pub struct Agent {
    pub(super) provider: Mutex<Option<Arc<dyn Provider>>>,
    pub(super) extension_manager: Mutex<ExtensionManager>,
    pub(super) frontend_tools: Mutex<HashMap<String, FrontendTool>>,
    pub(super) frontend_instructions: Mutex<Option<String>>,
    pub(super) prompt_manager: Mutex<PromptManager>,
    pub(super) confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    pub(super) confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    pub(super) tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    pub(super) tool_result_rx: ToolResultReceiver,
    pub(super) tool_monitor: Mutex<Option<ToolMonitor>>,
    pub(super) router_tool_selector: Mutex<Option<Box<dyn RouterToolSelector>>>,
}

impl Agent {
    pub fn new() -> Self {
        // Create channels with buffer size 32 (adjust if needed)
        let (confirm_tx, confirm_rx) = mpsc::channel(32);
        let (tool_tx, tool_rx) = mpsc::channel(32);

        Self {
            provider: Mutex::new(None),
            extension_manager: Mutex::new(ExtensionManager::new()),
            frontend_tools: Mutex::new(HashMap::new()),
            frontend_instructions: Mutex::new(None),
            prompt_manager: Mutex::new(PromptManager::new()),
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
            tool_monitor: Mutex::new(None),
            router_tool_selector: Mutex::new(None),
        }
    }

    pub async fn configure_tool_monitor(&self, max_repetitions: Option<u32>) {
        let mut tool_monitor = self.tool_monitor.lock().await;
        *tool_monitor = Some(ToolMonitor::new(max_repetitions));
    }

    pub async fn get_tool_stats(&self) -> Option<HashMap<String, u32>> {
        let tool_monitor = self.tool_monitor.lock().await;
        tool_monitor.as_ref().map(|monitor| monitor.get_stats())
    }

    pub async fn reset_tool_monitor(&self) {
        if let Some(monitor) = self.tool_monitor.lock().await.as_mut() {
            monitor.reset();
        }
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent {
    /// Get a reference count clone to the provider
    pub async fn provider(&self) -> Result<Arc<dyn Provider>, anyhow::Error> {
        match &*self.provider.lock().await {
            Some(provider) => Ok(Arc::clone(provider)),
            None => Err(anyhow!("Provider not set")),
        }
    }

    /// Check if a tool is a frontend tool
    pub async fn is_frontend_tool(&self, name: &str) -> bool {
        self.frontend_tools.lock().await.contains_key(name)
    }

    /// Get a reference to a frontend tool
    pub async fn get_frontend_tool(&self, name: &str) -> Option<FrontendTool> {
        self.frontend_tools.lock().await.get(name).cloned()
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(&self) -> ExtensionResult<Vec<Tool>> {
        let mut tools = self
            .extension_manager
            .lock()
            .await
            .get_prefixed_tools(None)
            .await?;

        // Add frontend tools directly - they don't need prefixing since they're already uniquely named
        let frontend_tools = self.frontend_tools.lock().await;
        for frontend_tool in frontend_tools.values() {
            tools.push(frontend_tool.tool.clone());
        }

        Ok(tools)
    }

    /// Dispatch a single tool call to the appropriate client
    #[instrument(skip(self, tool_call, request_id), fields(input, output))]
    pub(super) async fn dispatch_tool_call(
        &self,
        tool_call: mcp_core::tool::ToolCall,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        // Check if this tool call should be allowed based on repetition monitoring
        if let Some(monitor) = self.tool_monitor.lock().await.as_mut() {
            let tool_call_info = ToolCall::new(tool_call.name.clone(), tool_call.arguments.clone());

            if !monitor.check_tool_call(tool_call_info) {
                return (
                    request_id,
                    Err(ToolError::ExecutionError(
                        "Tool call rejected: exceeded maximum allowed repetitions".to_string(),
                    )),
                );
            }
        }

        if tool_call.name == PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME {
            let extension_name = tool_call
                .arguments
                .get("extension_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let action = tool_call
                .arguments
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return self
                .manage_extensions(action, extension_name, request_id)
                .await;
        }

        let extension_manager = self.extension_manager.lock().await;
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
        } else if self.is_frontend_tool(&tool_call.name).await {
            // For frontend tools, return an error indicating we need frontend execution
            Err(ToolError::ExecutionError(
                "Frontend tool execution required".to_string(),
            ))
        } else if tool_call.name == ROUTER_VECTOR_SEARCH_TOOL_NAME {
            eprintln!("[DEBUG] Received tool call: {:?}", tool_call);
            let router_tool_selector = self.router_tool_selector.lock().await;
            eprintln!("[DEBUG] Router tool selector: ");
            if let Some(selector) = router_tool_selector.as_ref() {
                selector.select_tools(tool_call.arguments.clone()).await
            } else {
                Err(ToolError::ExecutionError(
                    "Encountered vector search error.".to_string(),
                ))
            }
        } else {
            extension_manager
                .dispatch_tool_call(tool_call.clone())
                .await
        };

        debug!(
            "input" = serde_json::to_string(&tool_call).unwrap(),
            "output" = serde_json::to_string(&result).unwrap(),
        );

        // Process the response to handle large text content
        let processed_result = super::large_response_handler::process_tool_response(result);

        (request_id, processed_result)
    }

    pub(super) async fn manage_extensions(
        &self,
        action: String,
        extension_name: String,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let mut extension_manager = self.extension_manager.lock().await;

        if action == "disable" {
            let result = extension_manager
                .remove_extension(&extension_name)
                .await
                .map(|_| {
                    vec![Content::text(format!(
                        "The extension '{}' has been disabled successfully",
                        extension_name
                    ))]
                })
                .map_err(|e| ToolError::ExecutionError(e.to_string()));
            return (request_id, result);
        }

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

        // If vector tool selection is enabled, index the tools
        if result.is_ok() {
            if action == "disable" {
                if let Err(e) = self
                    .index_tools_if_vector_enabled(
                        Some(extension_name.clone()),
                        Some("remove"),
                        false,
                    )
                    .await
                {
                    tracing::error!("Failed to remove tools from vector index: {}", e);
                }
            } else {
                if let Err(e) = self
                    .index_tools_if_vector_enabled(Some(extension_name.clone()), Some("add"), false)
                    .await
                {
                    tracing::error!("Failed to index tools: {}", e);
                }
            }
        }

        (request_id, result)
    }

    pub async fn add_extension(&self, extension: ExtensionConfig) -> ExtensionResult<()> {
        match &extension {
            ExtensionConfig::Frontend {
                name: _,
                tools,
                instructions,
                bundled: _,
            } => {
                // For frontend tools, just store them in the frontend_tools map
                let mut frontend_tools = self.frontend_tools.lock().await;
                for tool in tools {
                    let frontend_tool = FrontendTool {
                        name: tool.name.clone(),
                        tool: tool.clone(),
                    };
                    frontend_tools.insert(tool.name.clone(), frontend_tool);
                }
                // Store instructions if provided, using "frontend" as the key
                let mut frontend_instructions = self.frontend_instructions.lock().await;
                if let Some(instructions) = instructions {
                    *frontend_instructions = Some(instructions.clone());
                } else {
                    // Default frontend instructions if none provided
                    *frontend_instructions = Some(
                        "The following tools are provided directly by the frontend and will be executed by the frontend when called.".to_string(),
                    );
                }
            }
            _ => {
                let mut extension_manager = self.extension_manager.lock().await;
                extension_manager.add_extension(extension.clone()).await?;
            }
        };

        // If vector tool selection is enabled, index the tools
        if let Err(e) = self
            .index_tools_if_vector_enabled(Some(extension.name()), Some("add"), false)
            .await
        {
            return Err(ExtensionError::SetupError(format!(
                "Failed to index tools for extension {}: {}",
                extension.name(),
                e
            )));
        }

        Ok(())
    }

    pub async fn list_tools(&self, extension_name: Option<String>) -> Vec<Tool> {
        let extension_manager = self.extension_manager.lock().await;
        let mut prefixed_tools = extension_manager
            .get_prefixed_tools(extension_name.clone())
            .await
            .unwrap_or_default();

        if extension_name.is_none() || extension_name.as_deref() == Some("platform") {
            // Add platform tools
            prefixed_tools.push(platform_tools::search_available_extensions_tool());
            prefixed_tools.push(platform_tools::manage_extensions_tool());

            // Add resource tools if supported
            if extension_manager.supports_resources() {
                prefixed_tools.push(platform_tools::read_resource_tool());
                prefixed_tools.push(platform_tools::list_resources_tool());
            }
        }

        prefixed_tools
    }

    pub async fn list_tools_for_router(
        &self,
        strategy: Option<RouterToolSelectionStrategy>,
    ) -> Vec<Tool> {
        let mut prefixed_tools = vec![];
        match strategy {
            Some(RouterToolSelectionStrategy::Vector) => {
                prefixed_tools.push(router_tools::vector_search_tool());
            }
            None => {}
        }

        // Get recent tool calls from router tool selector if available
        if let Some(selector) = self.router_tool_selector.lock().await.as_ref() {
            if let Ok(recent_calls) = selector.get_recent_tool_calls(20).await {
                let extension_manager = self.extension_manager.lock().await;
                // Add recent tool calls to the list, avoiding duplicates
                for tool_name in recent_calls {
                    // Find the tool in the extension manager's tools
                    if let Ok(extension_tools) = extension_manager.get_prefixed_tools(None).await {
                        if let Some(tool) = extension_tools.iter().find(|t| t.name == tool_name) {
                            // Only add if not already in prefixed_tools
                            if !prefixed_tools.iter().any(|t| t.name == tool.name) {
                                prefixed_tools.push(tool.clone());
                            }
                        }
                    }
                }
            }
        }

        prefixed_tools
    }

    pub async fn remove_extension(&self, name: &str) {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager
            .remove_extension(name)
            .await
            .expect("Failed to remove extension");

        // If vector tool selection is enabled, remove tools from the index
        if let Err(e) = self
            .index_tools_if_vector_enabled(Some(name.to_string()), Some("remove"), false)
            .await
        {
            tracing::error!("Failed to remove tools from vector index: {}", e);
        }
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

        // Load settings from config
        let config = Config::global();

        // Setup tools and prompt
        let (mut tools, mut toolshim_tools, mut system_prompt) =
            self.prepare_tools_and_prompt().await?;

        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());

        let (tools_with_readonly_annotation, tools_without_annotation) =
            Self::categorize_tools_by_annotation(&tools);

        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        Ok(Box::pin(async_stream::try_stream! {
            let _ = reply_span.enter();
            loop {
                match Self::generate_response_from_provider(
                    self.provider().await?,
                    &system_prompt,
                    &messages,
                    &tools,
                    &toolshim_tools,
                ).await {
                    Ok((response, usage)) => {
                        // record usage for the session in the session file
                        if let Some(session_config) = session.clone() {
                            Self::update_session_metrics(session_config, &usage, messages.len()).await?;
                        }

                        // categorize the type of requests we need to handle
                        let (frontend_requests,
                            remaining_requests,
                            filtered_response) =
                            self.categorize_tool_requests(&response).await;

                        // Record tool calls in the router selector
                        if let Some(selector) = self.router_tool_selector.lock().await.as_ref() {
                            // Record frontend tool calls
                            for request in &frontend_requests {
                                if let Ok(tool_call) = &request.tool_call {
                                    if let Err(e) = selector.record_tool_call(&tool_call.name).await {
                                        tracing::error!("Failed to record frontend tool call: {}", e);
                                    }
                                }
                            }
                            // Record remaining tool calls
                            for request in &remaining_requests {
                                if let Ok(tool_call) = &request.tool_call {
                                    if let Err(e) = selector.record_tool_call(&tool_call.name).await {
                                        tracing::error!("Failed to record tool call: {}", e);
                                    }
                                }
                            }
                        }

                        // Yield the assistant's response with frontend tool requests filtered out
                        yield filtered_response.clone();

                        tokio::task::yield_now().await;

                        let num_tool_requests = frontend_requests.len() + remaining_requests.len();
                        if num_tool_requests == 0 {
                            break;
                        }

                        // Process tool requests depending on frontend tools and then goose_mode
                        let message_tool_response = Arc::new(Mutex::new(Message::user()));

                        // First handle any frontend tool requests
                        let mut frontend_tool_stream = self.handle_frontend_tool_requests(
                            &frontend_requests,
                            message_tool_response.clone()
                        );

                        // we have a stream of frontend tools to handle, inside the stream
                        // execution is yeield back to this reply loop, and is of the same Message
                        // type, so we can yield that back up to be handled
                        while let Some(msg) = frontend_tool_stream.try_next().await? {
                            yield msg;
                        }

                        // Clone goose_mode once before the match to avoid move issues
                        let mode = goose_mode.clone();
                        if mode.as_str() == "chat" {
                            // Skip all tool calls in chat mode
                            for request in remaining_requests {
                                let mut response = message_tool_response.lock().await;
                                *response = response.clone().with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![Content::text(CHAT_MODE_TOOL_SKIPPED_RESPONSE)]),
                                );
                            }
                        } else {
                            // At this point, we have handled the frontend tool requests and know goose_mode != "chat"
                            // What remains is handling the remaining tool requests (enable extension,
                            // regular tool calls) in goose_mode == ["auto", "approve" or "smart_approve"]
                            let mut permission_manager = PermissionManager::default();
                            let (permission_check_result, enable_extension_request_ids) = check_tool_permissions(
                                &remaining_requests,
                                &mode,
                                tools_with_readonly_annotation.clone(),
                                tools_without_annotation.clone(),
                                &mut permission_manager,
                                self.provider().await?).await;

                            // Handle pre-approved and read-only tools in parallel
                            let mut tool_futures: Vec<ToolFuture> = Vec::new();

                            // Skip the confirmation for approved tools
                            for request in &permission_check_result.approved {
                                if let Ok(tool_call) = request.tool_call.clone() {
                                    let tool_future = self.dispatch_tool_call(tool_call, request.id.clone());
                                    tool_futures.push(Box::pin(tool_future));
                                }
                            }

                            for request in &permission_check_result.denied {
                                let mut response = message_tool_response.lock().await;
                                *response = response.clone().with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![Content::text(DECLINED_RESPONSE)]),
                                );
                            }

                            // We need interior mutability in handle_approval_tool_requests
                            let tool_futures_arc = Arc::new(Mutex::new(tool_futures));

                            // Process tools requiring approval (enable extension, regular tool calls)
                            let mut tool_approval_stream = self.handle_approval_tool_requests(
                                &permission_check_result.needs_approval,
                                tool_futures_arc.clone(),
                                &mut permission_manager,
                                message_tool_response.clone()
                            );

                            // We have a stream of tool_approval_requests to handle
                            // Execution is yielded back to this reply loop, and is of the same Message
                            // type, so we can yield the Message back up to be handled and grab any
                            // confirmations or denials
                            while let Some(msg) = tool_approval_stream.try_next().await? {
                                yield msg;
                            }

                            tool_futures = {
                                // Lock the mutex asynchronously
                                let mut futures_lock = tool_futures_arc.lock().await;
                                // Drain the vector and collect into a new Vec
                                futures_lock.drain(..).collect::<Vec<_>>()
                            };

                            // Wait for all tool calls to complete
                            let results = futures::future::join_all(tool_futures).await;
                            let mut all_install_successful = true;

                            for (request_id, output) in results.into_iter() {
                                if enable_extension_request_ids.contains(&request_id) && output.is_err(){
                                    all_install_successful = false;
                                }
                                let mut response = message_tool_response.lock().await;
                                *response = response.clone().with_tool_response(request_id, output);
                            }

                            // Update system prompt and tools if installations were successful
                            if all_install_successful {
                                (tools, toolshim_tools, system_prompt) = self.prepare_tools_and_prompt().await?;
                            }
                        }

                        let final_message_tool_resp = message_tool_response.lock().await.clone();
                        yield final_message_tool_resp.clone();

                        messages.push(response);
                        messages.push(final_message_tool_resp);
                    },
                    Err(ProviderError::ContextLengthExceeded(_)) => {
                        // At this point, the last message should be a user message
                        // because call to provider led to context length exceeded error
                        // Immediately yield a special message and break
                        yield Message::assistant().with_context_length_exceeded(
                            "The context length of the model has been exceeded. Please start a new session and try again.",
                        );
                        break;
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
    pub async fn extend_system_prompt(&self, instruction: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.add_system_prompt_extra(instruction);
    }

    /// Update the provider used by this agent
    pub async fn update_provider(&self, provider: Arc<dyn Provider>) -> Result<()> {
        *self.provider.lock().await = Some(provider.clone());
        self.update_router_tool_selector(provider).await?;
        Ok(())
    }

    async fn update_router_tool_selector(&self, provider: Arc<dyn Provider>) -> Result<()> {
        let config = Config::global();
        let router_tool_selection_strategy = config
            .get_param("GOOSE_ROUTER_TOOL_SELECTION_STRATEGY")
            .unwrap_or_else(|_| "default".to_string());

        eprintln!(
            "[DEBUG] Router tool selection strategy from config: {}",
            router_tool_selection_strategy
        );

        let strategy = match router_tool_selection_strategy.to_lowercase().as_str() {
            "vector" => Some(RouterToolSelectionStrategy::Vector),
            _ => None,
        };

        eprintln!("[DEBUG] Parsed strategy: {:?}", strategy);

        if let Some(strategy) = strategy {
            eprintln!("[DEBUG] Creating tool selector with vector strategy...");
            let table_name = generate_table_id();
            eprintln!("[DEBUG] Table name: {}", table_name);
            let selector = create_tool_selector(Some(strategy), provider, table_name)
                .await
                .map_err(|e| {
                    eprintln!("[DEBUG] Failed to create tool selector: {}", e);
                    anyhow!("Failed to create tool selector: {}", e)
                })?;

            eprintln!("[DEBUG] Clearing existing tools from vector database...");
            // // Clear tools from the vector database
            // selector
            //     .clear_tools()
            //     .await
            //     .map_err(|e| {
            //         eprintln!("[DEBUG] Failed to clear tools: {}", e);
            //         anyhow!("Failed to clear tools: {}", e)
            //     })?;

            eprintln!("[DEBUG] Setting router tool selector...");
            *self.router_tool_selector.lock().await = Some(selector);

            eprintln!("[DEBUG] Indexing platform tools...");
            self.index_platform_tools().await?;
            eprintln!("[DEBUG] Router tool selector initialization complete");
        } else {
            eprintln!(
                "[DEBUG] No vector strategy selected, skipping router tool selector initialization"
            );
        }

        Ok(())
    }

    /// Override the system prompt with a custom template
    pub async fn override_system_prompt(&self, template: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.set_system_prompt_override(template);
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
        let extension_manager = self.extension_manager.lock().await;
        let tools = extension_manager.get_prefixed_tools(None).await?;
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

    async fn index_tools_if_vector_enabled(
        &self,
        extension_name: Option<String>,
        action: Option<&str>,
        reindex_all: bool,
    ) -> Result<()> {
        // Only proceed if vector strategy is enabled
        let config = Config::global();
        let router_tool_selection_strategy = config
            .get_param("GOOSE_ROUTER_TOOL_SELECTION_STRATEGY")
            .unwrap_or_else(|_| "default".to_string());

        let is_vector_enabled = router_tool_selection_strategy.eq_ignore_ascii_case("vector");

        if !is_vector_enabled {
            return Ok(());
        }

        let router_tool_selector = self.router_tool_selector.lock().await;
        if let Some(selector) = router_tool_selector.as_ref() {
            let extension_manager = self.extension_manager.lock().await;

            if reindex_all {
                // Clear and reindex everything
                selector
                    .clear_tools()
                    .await
                    .map_err(|e| anyhow!("Failed to clear tools: {}", e))?;

                // Index all extension tools
                let all_tools = extension_manager.get_prefixed_tools(None).await?;
                for tool in &all_tools {
                    let schema_str = serde_json::to_string_pretty(&tool.input_schema)
                        .unwrap_or_else(|_| "{}".to_string());

                    selector
                        .index_tool(tool.name.clone(), tool.description.clone(), schema_str)
                        .await
                        .map_err(|e| anyhow!("Failed to index tool {}: {}", tool.name, e))?;
                }

                // Index all frontend tools
                let frontend_tools = self.frontend_tools.lock().await;
                for frontend_tool in frontend_tools.values() {
                    let schema_str = serde_json::to_string_pretty(&frontend_tool.tool.input_schema)
                        .unwrap_or_else(|_| "{}".to_string());

                    selector
                        .index_tool(
                            frontend_tool.tool.name.clone(),
                            frontend_tool.tool.description.clone(),
                            schema_str,
                        )
                        .await
                        .map_err(|e| {
                            anyhow!(
                                "Failed to index frontend tool {}: {}",
                                frontend_tool.tool.name,
                                e
                            )
                        })?;
                }

                // Index platform tools
                self.index_platform_tools().await?;

                tracing::info!("Reindexed all tools for vector search");
                return Ok(());
            }

            // Handle specific extension operations
            if let (Some(ext_name), Some(act)) = (extension_name, action) {
                match act {
                    "add" => {
                        // Get tools for specific extension
                        let tools = extension_manager
                            .get_prefixed_tools(Some(ext_name.clone()))
                            .await?;
                        for tool in &tools {
                            let schema_str = serde_json::to_string_pretty(&tool.input_schema)
                                .unwrap_or_else(|_| "{}".to_string());

                            selector
                                .index_tool(tool.name.clone(), tool.description.clone(), schema_str)
                                .await
                                .map_err(|e| {
                                    anyhow!("Failed to index tool {}: {}", tool.name, e)
                                })?;
                        }
                        tracing::info!("Indexed {} tools for extension {}", tools.len(), ext_name);
                    }
                    "remove" => {
                        // Get tool names for the extension to remove them
                        let tools = extension_manager
                            .get_prefixed_tools(Some(ext_name.clone()))
                            .await?;
                        for tool in &tools {
                            selector.remove_tool(&tool.name).await.map_err(|e| {
                                anyhow!("Failed to remove tool {}: {}", tool.name, e)
                            })?;
                        }
                        tracing::info!("Removed {} tools for extension {}", tools.len(), ext_name);
                    }
                    _ => {
                        anyhow::bail!("Invalid action '{}' for tool indexing", act);
                    }
                }
            } else {
                anyhow::bail!("Extension name and action required for tool indexing");
            }
        }

        Ok(())
    }

    async fn index_platform_tools(&self) -> Result<()> {
        let router_tool_selector = self.router_tool_selector.lock().await;
        if let Some(selector) = router_tool_selector.as_ref() {
            let extension_manager = self.extension_manager.lock().await;

            // Index the standard platform tools
            let search_tool = platform_tools::search_available_extensions_tool();
            let schema_str = serde_json::to_string_pretty(&search_tool.input_schema)
                .unwrap_or_else(|_| "{}".to_string());
            selector
                .index_tool(search_tool.name, search_tool.description, schema_str)
                .await
                .map_err(|e| anyhow!("Failed to index search_available_extensions tool: {}", e))?;

            let manage_tool = platform_tools::manage_extensions_tool();
            let schema_str = serde_json::to_string_pretty(&manage_tool.input_schema)
                .unwrap_or_else(|_| "{}".to_string());
            selector
                .index_tool(manage_tool.name, manage_tool.description, schema_str)
                .await
                .map_err(|e| anyhow!("Failed to index manage_extensions tool: {}", e))?;

            // Index resource tools if supported
            if extension_manager.supports_resources() {
                let read_tool = platform_tools::read_resource_tool();
                let schema_str = serde_json::to_string_pretty(&read_tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string());
                selector
                    .index_tool(read_tool.name, read_tool.description, schema_str)
                    .await
                    .map_err(|e| anyhow!("Failed to index read_resource tool: {}", e))?;

                let list_tool = platform_tools::list_resources_tool();
                let schema_str = serde_json::to_string_pretty(&list_tool.input_schema)
                    .unwrap_or_else(|_| "{}".to_string());
                selector
                    .index_tool(list_tool.name, list_tool.description, schema_str)
                    .await
                    .map_err(|e| anyhow!("Failed to index list_resources tool: {}", e))?;
            }

            tracing::info!("Indexed platform tools for vector search");
        }

        Ok(())
    }

    pub async fn create_recipe(&self, mut messages: Vec<Message>) -> Result<Recipe> {
        let extension_manager = self.extension_manager.lock().await;
        let extensions_info = extension_manager.get_extensions_info().await;

        // Get model name from provider
        let provider = self.provider().await?;
        let model_config = provider.get_model_config();
        let model_name = &model_config.model_name;

        let prompt_manager = self.prompt_manager.lock().await;
        let system_prompt = prompt_manager.build_system_prompt(
            extensions_info,
            self.frontend_instructions.lock().await.clone(),
            extension_manager.suggest_disable_extensions_prompt().await,
            Some(model_name),
            None,
        );

        let recipe_prompt = prompt_manager.get_recipe_prompt().await;
        let tools = extension_manager.get_prefixed_tools(None).await?;

        messages.push(Message::user().with_text(recipe_prompt));

        let (result, _usage) = self
            .provider
            .lock()
            .await
            .as_ref()
            .unwrap()
            .complete(&system_prompt, &messages, &tools)
            .await?;

        let content = result.as_concat_text();

        // the response may be contained in ```json ```, strip that before parsing json
        let re = Regex::new(r"(?s)```[^\n]*\n(.*?)\n```").unwrap();
        let clean_content = re
            .captures(&content)
            .and_then(|caps| caps.get(1).map(|m| m.as_str()))
            .unwrap_or(&content)
            .trim()
            .to_string();

        // try to parse json response from the LLM
        let (instructions, activities) =
            if let Ok(json_content) = serde_json::from_str::<Value>(&clean_content) {
                let instructions = json_content
                    .get("instructions")
                    .ok_or_else(|| anyhow!("Missing 'instructions' in json response"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("instructions' is not a string"))?
                    .to_string();

                let activities = json_content
                    .get("activities")
                    .ok_or_else(|| anyhow!("Missing 'activities' in json response"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("'activities' is not an array'"))?
                    .iter()
                    .map(|act| {
                        act.as_str()
                            .map(|s| s.to_string())
                            .ok_or(anyhow!("'activities' array element is not a string"))
                    })
                    .collect::<Result<_, _>>()?;

                (instructions, activities)
            } else {
                // If we can't get valid JSON, try string parsing
                // Use split_once to get the content after "Instructions:".
                let after_instructions = content
                    .split_once("instructions:")
                    .map(|(_, rest)| rest)
                    .unwrap_or(&content);

                // Split once more to separate instructions from activities.
                let (instructions_part, activities_text) = after_instructions
                    .split_once("activities:")
                    .unwrap_or((after_instructions, ""));

                let instructions = instructions_part
                    .trim_end_matches(|c: char| c.is_whitespace() || c == '#')
                    .trim()
                    .to_string();
                let activities_text = activities_text.trim();

                // Regex to remove bullet markers or numbers with an optional dot.
                let bullet_re = Regex::new(r"^[•\-\*\d]+\.?\s*").expect("Invalid regex");

                // Process each line in the activities section.
                let activities: Vec<String> = activities_text
                    .lines()
                    .map(|line| bullet_re.replace(line, "").to_string())
                    .map(|s| s.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();

                (instructions, activities)
            };

        let extensions = ExtensionConfigManager::get_all().unwrap_or_default();
        let extension_configs: Vec<_> = extensions
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.config.clone())
            .collect();

        let author = Author {
            contact: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .ok(),
            metadata: None,
        };

        let recipe = Recipe::builder()
            .title("Custom recipe from chat")
            .description("a custom recipe instance from this chat session")
            .instructions(instructions)
            .activities(activities)
            .extensions(extension_configs)
            .author(author)
            .build()
            .expect("valid recipe");

        Ok(recipe)
    }
}
