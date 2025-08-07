use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::stream::BoxStream;
use futures::{stream, FutureExt, Stream, StreamExt, TryStreamExt};
use uuid::Uuid;

use crate::agents::extension::{ExtensionConfig, ExtensionError, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::final_output_tool::{FINAL_OUTPUT_CONTINUATION_MESSAGE, FINAL_OUTPUT_TOOL_NAME};
use crate::agents::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
    PLATFORM_MANAGE_SCHEDULE_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
    PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::agents::prompt_manager::PromptManager;
use crate::agents::recipe_tools::dynamic_task_tools::{
    create_dynamic_task, create_dynamic_task_tool, DYNAMIC_TASK_TOOL_NAME_PREFIX,
};
use crate::agents::retry::{RetryManager, RetryResult};
use crate::agents::router_tool_selector::RouterToolSelectionStrategy;
use crate::agents::router_tools::{ROUTER_LLM_SEARCH_TOOL_NAME, ROUTER_VECTOR_SEARCH_TOOL_NAME};
use crate::agents::sub_recipe_manager::SubRecipeManager;
use crate::agents::subagent_execution_tool::subagent_execute_task_tool::{
    self, SUBAGENT_EXECUTE_TASK_TOOL_NAME,
};
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::agents::tool_route_manager::ToolRouteManager;
use crate::agents::tool_router_index_manager::ToolRouterIndexManager;
use crate::agents::types::SessionConfig;
use crate::agents::types::{FrontendTool, ToolResultReceiver};
use crate::config::{Config, ExtensionConfigManager, PermissionManager};
use crate::context_mgmt::auto_compact;
use crate::conversation::{debug_conversation_fix, fix_conversation, Conversation};
use crate::permission::permission_judge::{check_tool_permissions, PermissionCheckResult};
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::recipe::{Author, Recipe, Response, Settings, SubRecipe};
use crate::scheduler_trait::SchedulerTrait;
use crate::session;
use crate::tool_monitor::{ToolCall, ToolMonitor};
use crate::utils::is_token_cancelled;
use mcp_core::{ToolError, ToolResult};
use regex::Regex;
use rmcp::model::{Content, GetPromptResult, Prompt, ServerNotification, Tool};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument};

use super::final_output_tool::FinalOutputTool;
use super::platform_tools;
use super::tool_execution::{ToolCallResult, CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};
use crate::agents::subagent_task_config::TaskConfig;
use crate::conversation::message::{Message, ToolRequest};

const DEFAULT_MAX_TURNS: u32 = 1000;

/// Context needed for the reply function
pub struct ReplyContext {
    pub messages: Conversation,
    pub tools: Vec<Tool>,
    pub toolshim_tools: Vec<Tool>,
    pub system_prompt: String,
    pub goose_mode: String,
    pub initial_messages: Vec<Message>,
    pub config: &'static Config,
}

pub struct ToolCategorizeResult {
    pub frontend_requests: Vec<ToolRequest>,
    pub remaining_requests: Vec<ToolRequest>,
    pub filtered_response: Message,
    pub readonly_tools: HashSet<String>,
    pub regular_tools: HashSet<String>,
}

/// The main goose Agent
pub struct Agent {
    pub(super) provider: Mutex<Option<Arc<dyn Provider>>>,
    pub extension_manager: Arc<RwLock<ExtensionManager>>,
    pub(super) sub_recipe_manager: Mutex<SubRecipeManager>,
    pub(super) tasks_manager: TasksManager,
    pub(super) final_output_tool: Arc<Mutex<Option<FinalOutputTool>>>,
    pub(super) frontend_tools: Mutex<HashMap<String, FrontendTool>>,
    pub(super) frontend_instructions: Mutex<Option<String>>,
    pub(super) prompt_manager: Mutex<PromptManager>,
    pub(super) confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    pub(super) confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    pub(super) tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    pub(super) tool_result_rx: ToolResultReceiver,
    pub(super) tool_monitor: Arc<Mutex<Option<ToolMonitor>>>,
    pub(super) tool_route_manager: ToolRouteManager,
    pub(super) scheduler_service: Mutex<Option<Arc<dyn SchedulerTrait>>>,
    pub(super) retry_manager: RetryManager,
}

#[derive(Clone, Debug)]
pub enum AgentEvent {
    Message(Message),
    McpNotification((String, ServerNotification)),
    ModelChange { model: String, mode: String },
    HistoryReplaced(Vec<Message>),
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}

pub enum ToolStreamItem<T> {
    Message(ServerNotification),
    Result(T),
}

pub type ToolStream = Pin<Box<dyn Stream<Item = ToolStreamItem<ToolResult<Vec<Content>>>> + Send>>;

// tool_stream combines a stream of ServerNotifications with a future representing the
// final result of the tool call. MCP notifications are not request-scoped, but
// this lets us capture all notifications emitted during the tool call for
// simpler consumption
pub fn tool_stream<S, F>(rx: S, done: F) -> ToolStream
where
    S: Stream<Item = ServerNotification> + Send + Unpin + 'static,
    F: Future<Output = ToolResult<Vec<Content>>> + Send + 'static,
{
    Box::pin(async_stream::stream! {
        tokio::pin!(done);
        let mut rx = rx;

        loop {
            tokio::select! {
                Some(msg) = rx.next() => {
                    yield ToolStreamItem::Message(msg);
                }
                r = &mut done => {
                    yield ToolStreamItem::Result(r);
                    break;
                }
            }
        }
    })
}

impl Agent {
    pub fn new() -> Self {
        // Create channels with buffer size 32 (adjust if needed)
        let (confirm_tx, confirm_rx) = mpsc::channel(32);
        let (tool_tx, tool_rx) = mpsc::channel(32);

        let tool_monitor = Arc::new(Mutex::new(None));
        let retry_manager = RetryManager::with_tool_monitor(tool_monitor.clone());

        Self {
            provider: Mutex::new(None),
            extension_manager: Arc::new(RwLock::new(ExtensionManager::new())),
            sub_recipe_manager: Mutex::new(SubRecipeManager::new()),
            tasks_manager: TasksManager::new(),
            final_output_tool: Arc::new(Mutex::new(None)),
            frontend_tools: Mutex::new(HashMap::new()),
            frontend_instructions: Mutex::new(None),
            prompt_manager: Mutex::new(PromptManager::new()),
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
            tool_monitor,
            tool_route_manager: ToolRouteManager::new(),
            scheduler_service: Mutex::new(None),
            retry_manager,
        }
    }

    pub async fn configure_tool_monitor(&self, max_repetitions: Option<u32>) {
        let mut tool_monitor = self.tool_monitor.lock().await;
        *tool_monitor = Some(ToolMonitor::new(max_repetitions));
    }

    /// Reset the retry attempts counter to 0
    pub async fn reset_retry_attempts(&self) {
        self.retry_manager.reset_attempts().await;
    }

    /// Increment the retry attempts counter and return the new value
    pub async fn increment_retry_attempts(&self) -> u32 {
        self.retry_manager.increment_attempts().await
    }

    /// Get the current retry attempts count
    pub async fn get_retry_attempts(&self) -> u32 {
        self.retry_manager.get_attempts().await
    }

    /// Handle retry logic for the agent reply loop
    async fn handle_retry_logic(
        &self,
        messages: &mut Conversation,
        session: &Option<SessionConfig>,
        initial_messages: &[Message],
    ) -> Result<bool> {
        let result = self
            .retry_manager
            .handle_retry_logic(messages, session, initial_messages, &self.final_output_tool)
            .await?;

        match result {
            RetryResult::Retried => Ok(true),
            RetryResult::Skipped
            | RetryResult::MaxAttemptsReached
            | RetryResult::SuccessChecksPassed => Ok(false),
        }
    }

    async fn prepare_reply_context(
        &self,
        unfixed_conversation: Conversation,
        session: &Option<SessionConfig>,
    ) -> Result<ReplyContext> {
        let unfixed_messages = unfixed_conversation.messages().clone();
        let (conversation, issues) = fix_conversation(unfixed_conversation.clone());
        if !issues.is_empty() {
            tracing::warn!(
                "Conversation issue fixed: {}",
                debug_conversation_fix(
                    unfixed_messages.as_slice(),
                    conversation.messages(),
                    &issues
                )
            );
        }
        let initial_messages = conversation.messages().clone();
        let config = Config::global();

        let (tools, toolshim_tools, system_prompt) = self.prepare_tools_and_prompt().await?;
        let goose_mode = Self::determine_goose_mode(session.as_ref(), config);

        Ok(ReplyContext {
            messages: conversation,
            tools,
            toolshim_tools,
            system_prompt,
            goose_mode,
            initial_messages,
            config,
        })
    }

    async fn categorize_tools(
        &self,
        response: &Message,
        tools: &[rmcp::model::Tool],
    ) -> ToolCategorizeResult {
        let (readonly_tools, regular_tools) = Self::categorize_tools_by_annotation(tools);

        // Categorize tool requests
        let (frontend_requests, remaining_requests, filtered_response) =
            self.categorize_tool_requests(response).await;

        ToolCategorizeResult {
            frontend_requests,
            remaining_requests,
            filtered_response,
            readonly_tools,
            regular_tools,
        }
    }

    async fn handle_approved_and_denied_tools(
        &self,
        permission_check_result: &PermissionCheckResult,
        message_tool_response: Arc<Mutex<Message>>,
        cancel_token: Option<tokio_util::sync::CancellationToken>,
    ) -> Result<Vec<(String, ToolStream)>> {
        let mut tool_futures: Vec<(String, ToolStream)> = Vec::new();

        // Handle pre-approved and read-only tools
        for request in &permission_check_result.approved {
            if let Ok(tool_call) = request.tool_call.clone() {
                let (req_id, tool_result) = self
                    .dispatch_tool_call(tool_call, request.id.clone(), cancel_token.clone())
                    .await;

                tool_futures.push((
                    req_id,
                    match tool_result {
                        Ok(result) => tool_stream(
                            result
                                .notification_stream
                                .unwrap_or_else(|| Box::new(stream::empty())),
                            result.result,
                        ),
                        Err(e) => {
                            tool_stream(Box::new(stream::empty()), futures::future::ready(Err(e)))
                        }
                    },
                ));
            }
        }

        // Handle denied tools
        for request in &permission_check_result.denied {
            let mut response = message_tool_response.lock().await;
            *response = response.clone().with_tool_response(
                request.id.clone(),
                Ok(vec![rmcp::model::Content::text(DECLINED_RESPONSE)]),
            );
        }

        Ok(tool_futures)
    }

    /// Set the scheduler service for this agent
    pub async fn set_scheduler(&self, scheduler: Arc<dyn SchedulerTrait>) {
        let mut scheduler_service = self.scheduler_service.lock().await;
        *scheduler_service = Some(scheduler);
    }

    pub async fn disable_router_for_recipe(&self) {
        self.tool_route_manager.disable_router_for_recipe().await;
    }

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

    pub async fn add_final_output_tool(&self, response: Response) {
        let mut final_output_tool = self.final_output_tool.lock().await;
        let created_final_output_tool = FinalOutputTool::new(response);
        let final_output_system_prompt = created_final_output_tool.system_prompt();
        *final_output_tool = Some(created_final_output_tool);
        self.extend_system_prompt(final_output_system_prompt).await;
    }

    pub async fn add_sub_recipes(&self, sub_recipes: Vec<SubRecipe>) {
        let mut sub_recipe_manager = self.sub_recipe_manager.lock().await;
        sub_recipe_manager.add_sub_recipe_tools(sub_recipes);
    }

    /// Dispatch a single tool call to the appropriate client
    #[instrument(skip(self, tool_call, request_id), fields(input, output))]
    pub async fn dispatch_tool_call(
        &self,
        tool_call: mcp_core::tool::ToolCall,
        request_id: String,
        cancellation_token: Option<CancellationToken>,
    ) -> (String, Result<ToolCallResult, ToolError>) {
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

        if tool_call.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME {
            let result = self
                .handle_schedule_management(tool_call.arguments, request_id.clone())
                .await;
            return (request_id, Ok(ToolCallResult::from(result)));
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
            let (request_id, result) = self
                .manage_extensions(action, extension_name, request_id)
                .await;

            return (request_id, Ok(ToolCallResult::from(result)));
        }

        if tool_call.name == FINAL_OUTPUT_TOOL_NAME {
            return if let Some(final_output_tool) = self.final_output_tool.lock().await.as_mut() {
                let result = final_output_tool.execute_tool_call(tool_call.clone()).await;
                (request_id, Ok(result))
            } else {
                (
                    request_id,
                    Err(ToolError::ExecutionError(
                        "Final output tool not defined".to_string(),
                    )),
                )
            };
        }

        let extension_manager = self.extension_manager.read().await;
        let sub_recipe_manager = self.sub_recipe_manager.lock().await;
        let result: ToolCallResult = if sub_recipe_manager.is_sub_recipe_tool(&tool_call.name) {
            sub_recipe_manager
                .dispatch_sub_recipe_tool_call(
                    &tool_call.name,
                    tool_call.arguments.clone(),
                    &self.tasks_manager,
                )
                .await
        } else if tool_call.name == SUBAGENT_EXECUTE_TASK_TOOL_NAME {
            let provider = self.provider().await.ok();

            let task_config = TaskConfig::new(provider);
            subagent_execute_task_tool::run_tasks(
                tool_call.arguments.clone(),
                task_config,
                &self.tasks_manager,
                cancellation_token,
            )
            .await
        } else if tool_call.name == DYNAMIC_TASK_TOOL_NAME_PREFIX {
            create_dynamic_task(tool_call.arguments.clone(), &self.tasks_manager).await
        } else if tool_call.name == PLATFORM_READ_RESOURCE_TOOL_NAME {
            // Check if the tool is read_resource and handle it separately
            ToolCallResult::from(
                extension_manager
                    .read_resource(
                        tool_call.arguments.clone(),
                        cancellation_token.unwrap_or_default(),
                    )
                    .await,
            )
        } else if tool_call.name == PLATFORM_LIST_RESOURCES_TOOL_NAME {
            ToolCallResult::from(
                extension_manager
                    .list_resources(
                        tool_call.arguments.clone(),
                        cancellation_token.unwrap_or_default(),
                    )
                    .await,
            )
        } else if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
            ToolCallResult::from(extension_manager.search_available_extensions().await)
        } else if self.is_frontend_tool(&tool_call.name).await {
            // For frontend tools, return an error indicating we need frontend execution
            ToolCallResult::from(Err(ToolError::ExecutionError(
                "Frontend tool execution required".to_string(),
            )))
        } else if tool_call.name == ROUTER_VECTOR_SEARCH_TOOL_NAME
            || tool_call.name == ROUTER_LLM_SEARCH_TOOL_NAME
        {
            match self
                .tool_route_manager
                .dispatch_route_search_tool(tool_call.arguments)
                .await
            {
                Ok(tool_result) => tool_result,
                Err(e) => return (request_id, Err(e)),
            }
        } else {
            // Clone the result to ensure no references to extension_manager are returned
            let result = extension_manager
                .dispatch_tool_call(tool_call.clone(), cancellation_token.unwrap_or_default())
                .await;
            result.unwrap_or_else(|e| {
                ToolCallResult::from(Err(ToolError::ExecutionError(e.to_string())))
            })
        };

        (
            request_id,
            Ok(ToolCallResult {
                notification_stream: result.notification_stream,
                result: Box::new(
                    result
                        .result
                        .map(super::large_response_handler::process_tool_response),
                ),
            }),
        )
    }

    pub(super) async fn manage_extensions(
        &self,
        action: String,
        extension_name: String,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let selector = self.tool_route_manager.get_router_tool_selector().await;
        if ToolRouterIndexManager::is_tool_router_enabled(&selector) {
            if let Some(selector) = selector {
                let selector_action = if action == "disable" { "remove" } else { "add" };
                let extension_manager = self.extension_manager.read().await;
                let selector = Arc::new(selector);
                if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                    &selector,
                    &extension_manager,
                    &extension_name,
                    selector_action,
                )
                .await
                {
                    return (
                        request_id,
                        Err(ToolError::ExecutionError(format!(
                            "Failed to update vector index: {}",
                            e
                        ))),
                    );
                }
            }
        }
        let mut extension_manager = self.extension_manager.write().await;
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

        drop(extension_manager);
        // Update vector index if operation was successful and vector routing is enabled
        if result.is_ok() {
            let selector = self.tool_route_manager.get_router_tool_selector().await;
            if ToolRouterIndexManager::is_tool_router_enabled(&selector) {
                if let Some(selector) = selector {
                    let vector_action = if action == "disable" { "remove" } else { "add" };
                    let extension_manager = self.extension_manager.read().await;
                    let selector = Arc::new(selector);
                    if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                        &selector,
                        &extension_manager,
                        &extension_name,
                        vector_action,
                    )
                    .await
                    {
                        return (
                            request_id,
                            Err(ToolError::ExecutionError(format!(
                                "Failed to update vector index: {}",
                                e
                            ))),
                        );
                    }
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
                        name: tool.name.to_string(),
                        tool: tool.clone(),
                    };
                    frontend_tools.insert(tool.name.to_string(), frontend_tool);
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
                let mut extension_manager = self.extension_manager.write().await;
                extension_manager.add_extension(extension.clone()).await?;
            }
        }

        // If vector tool selection is enabled, index the tools
        let selector = self.tool_route_manager.get_router_tool_selector().await;
        if ToolRouterIndexManager::is_tool_router_enabled(&selector) {
            if let Some(selector) = selector {
                let extension_manager = self.extension_manager.read().await;
                let selector = Arc::new(selector);
                if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                    &selector,
                    &extension_manager,
                    &extension.name(),
                    "add",
                )
                .await
                {
                    return Err(ExtensionError::SetupError(format!(
                        "Failed to index tools for extension {}: {}",
                        extension.name(),
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    pub async fn list_tools(&self, extension_name: Option<String>) -> Vec<Tool> {
        let extension_manager = self.extension_manager.read().await;
        let mut prefixed_tools = extension_manager
            .get_prefixed_tools(extension_name.clone())
            .await
            .unwrap_or_default();

        if extension_name.is_none() || extension_name.as_deref() == Some("platform") {
            // Add platform tools
            prefixed_tools.extend([
                platform_tools::search_available_extensions_tool(),
                platform_tools::manage_extensions_tool(),
                platform_tools::manage_schedule_tool(),
            ]);

            // Dynamic task tool
            prefixed_tools.push(create_dynamic_task_tool());

            // Add resource tools if supported
            if extension_manager.supports_resources() {
                prefixed_tools.extend([
                    platform_tools::read_resource_tool(),
                    platform_tools::list_resources_tool(),
                ]);
            }
        }

        if extension_name.is_none() {
            let sub_recipe_manager = self.sub_recipe_manager.lock().await;
            prefixed_tools.extend(sub_recipe_manager.sub_recipe_tools.values().cloned());

            if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                prefixed_tools.push(final_output_tool.tool());
            }
            prefixed_tools.push(subagent_execute_task_tool::create_subagent_execute_task_tool());
        }

        prefixed_tools
    }

    pub async fn list_tools_for_router(
        &self,
        strategy: Option<RouterToolSelectionStrategy>,
    ) -> Vec<Tool> {
        self.tool_route_manager
            .list_tools_for_router(strategy, &self.extension_manager)
            .await
    }

    pub async fn remove_extension(&self, name: &str) -> Result<()> {
        let mut extension_manager = self.extension_manager.write().await;
        extension_manager.remove_extension(name).await?;
        drop(extension_manager);

        // If vector tool selection is enabled, remove tools from the index
        let selector = self.tool_route_manager.get_router_tool_selector().await;
        if ToolRouterIndexManager::is_tool_router_enabled(&selector) {
            if let Some(selector) = selector {
                let extension_manager = self.extension_manager.read().await;
                ToolRouterIndexManager::update_extension_tools(
                    &selector,
                    &extension_manager,
                    name,
                    "remove",
                )
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_extensions(&self) -> Vec<String> {
        let extension_manager = self.extension_manager.read().await;
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

    /// Handle auto-compaction logic and return compacted messages if needed
    async fn handle_auto_compaction(
        &self,
        messages: &[Message],
        session: &Option<SessionConfig>,
    ) -> Result<Option<(Conversation, String)>> {
        // Try to get session metadata for more accurate token counts
        let session_metadata = if let Some(session_config) = session {
            match session::storage::get_path(session_config.id.clone()) {
                Ok(session_file_path) => session::storage::read_metadata(&session_file_path).ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        let compact_result = auto_compact::check_and_compact_messages(
            self,
            messages,
            None,
            session_metadata.as_ref(),
        )
        .await?;

        if compact_result.compacted {
            let compacted_messages = compact_result.messages;

            // Create compaction notification message
            let compaction_msg = if let (Some(before), Some(after)) =
                (compact_result.tokens_before, compact_result.tokens_after)
            {
                format!(
                    "Auto-compacted context: {} → {} tokens ({:.0}% reduction)\n\n",
                    before,
                    after,
                    (1.0 - (after as f64 / before as f64)) * 100.0
                )
            } else {
                "Auto-compacted context to reduce token usage\n\n".to_string()
            };

            return Ok(Some((compacted_messages, compaction_msg)));
        }

        Ok(None)
    }

    #[instrument(skip(self, unfixed_conversation, session), fields(user_message))]
    pub async fn reply(
        &self,
        unfixed_conversation: Conversation,
        session: Option<SessionConfig>,
        cancel_token: Option<CancellationToken>,
    ) -> Result<BoxStream<'_, Result<AgentEvent>>> {
        // Handle auto-compaction before processing
        let (messages, compaction_msg) = match self
            .handle_auto_compaction(unfixed_conversation.messages(), &session)
            .await?
        {
            Some((compacted_messages, msg)) => (compacted_messages, Some(msg)),
            None => {
                let context = self
                    .prepare_reply_context(unfixed_conversation, &session)
                    .await?;
                (context.messages, None)
            }
        };

        // If we compacted, yield the compaction message and history replacement event
        if let Some(compaction_msg) = compaction_msg {
            return Ok(Box::pin(async_stream::try_stream! {
                yield AgentEvent::Message(Message::assistant().with_text(compaction_msg));
                yield AgentEvent::HistoryReplaced(messages.messages().clone());

                // Continue with normal reply processing using compacted messages
                let mut reply_stream = self.reply_internal(messages, session, cancel_token).await?;
                while let Some(event) = reply_stream.next().await {
                    yield event?;
                }
            }));
        }

        // No compaction needed, proceed with normal processing
        self.reply_internal(messages, session, cancel_token).await
    }

    /// Main reply method that handles the actual agent processing
    async fn reply_internal(
        &self,
        messages: Conversation,
        session: Option<SessionConfig>,
        cancel_token: Option<CancellationToken>,
    ) -> Result<BoxStream<'_, Result<AgentEvent>>> {
        let context = self.prepare_reply_context(messages, &session).await?;
        let ReplyContext {
            mut messages,
            mut tools,
            mut toolshim_tools,
            mut system_prompt,
            goose_mode,
            initial_messages,
            config,
        } = context;
        let reply_span = tracing::Span::current();
        self.reset_retry_attempts().await;

        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        Ok(Box::pin(async_stream::try_stream! {
            let _ = reply_span.enter();
            let mut turns_taken = 0u32;
            let max_turns = session
                .as_ref()
                .and_then(|s| s.max_turns)
                .unwrap_or_else(|| {
                    config.get_param("GOOSE_MAX_TURNS").unwrap_or(DEFAULT_MAX_TURNS)
                });

            loop {
                if is_token_cancelled(&cancel_token) {
                    break;
                }

                if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                    if final_output_tool.final_output.is_some() {
                        let final_event = AgentEvent::Message(
                            Message::assistant().with_text(final_output_tool.final_output.clone().unwrap()),
                        );
                        yield final_event;
                        break;
                    }
                }

                turns_taken += 1;
                if turns_taken > max_turns {
                    yield AgentEvent::Message(Message::assistant().with_text(
                        "I've reached the maximum number of actions I can do without user input. Would you like me to continue?"
                    ));
                    break;
                }

                let mut stream = Self::stream_response_from_provider(
                    self.provider().await?,
                    &system_prompt,
                    messages.messages(),
                    &tools,
                    &toolshim_tools,
                ).await?;

                let mut added_message = false;
                let mut messages_to_add = Vec::new();
                let mut tools_updated = false;

                while let Some(next) = stream.next().await {
                    if is_token_cancelled(&cancel_token) {
                        break;
                    }

                    match next {
                        Ok((response, usage)) => {
                            // Emit model change event if provider is lead-worker
                            let provider = self.provider().await?;
                            if let Some(lead_worker) = provider.as_lead_worker() {
                                if let Some(ref usage) = usage {
                                    let active_model = usage.model.clone();
                                    let (lead_model, worker_model) = lead_worker.get_model_info();
                                    let mode = if active_model == lead_model {
                                        "lead"
                                    } else if active_model == worker_model {
                                        "worker"
                                    } else {
                                        "unknown"
                                    };

                                    yield AgentEvent::ModelChange {
                                        model: active_model,
                                        mode: mode.to_string(),
                                    };
                                }
                            }

                            // Record usage for the session
                            if let Some(ref session_config) = &session {
                                if let Some(ref usage) = usage {
                                    Self::update_session_metrics(session_config, usage, messages.len())
                                        .await?;
                                }
                            }

                            if let Some(response) = response {
                                let ToolCategorizeResult {
                                    frontend_requests,
                                    remaining_requests,
                                    filtered_response,
                                    readonly_tools,
                                    regular_tools,
                                } = self.categorize_tools(&response, &tools).await;
                                let requests_to_record: Vec<ToolRequest> = frontend_requests.iter().chain(remaining_requests.iter()).cloned().collect();
                                self.tool_route_manager
                                    .record_tool_requests(&requests_to_record)
                                    .await;

                                yield AgentEvent::Message(filtered_response.clone());
                                tokio::task::yield_now().await;

                                let num_tool_requests = frontend_requests.len() + remaining_requests.len();
                                if num_tool_requests == 0 {
                                    continue;
                                }

                                let message_tool_response = Arc::new(Mutex::new(Message::user().with_id(
                                    format!("msg_{}", Uuid::new_v4())
                                )));

                                let mut frontend_tool_stream = self.handle_frontend_tool_requests(
                                    &frontend_requests,
                                    message_tool_response.clone(),
                                );

                                while let Some(msg) = frontend_tool_stream.try_next().await? {
                                    yield AgentEvent::Message(msg);
                                }

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
                                    let mut permission_manager = PermissionManager::default();
                                    let (permission_check_result, enable_extension_request_ids) =
                                        check_tool_permissions(
                                            &remaining_requests,
                                            &mode,
                                            readonly_tools.clone(),
                                            regular_tools.clone(),
                                            &mut permission_manager,
                                            self.provider().await?,
                                        ).await;

                                    let mut tool_futures = self.handle_approved_and_denied_tools(
                                        &permission_check_result,
                                        message_tool_response.clone(),
                                        cancel_token.clone()
                                    ).await?;

                                    let tool_futures_arc = Arc::new(Mutex::new(tool_futures));

                                    // Process tools requiring approval
                                    let mut tool_approval_stream = self.handle_approval_tool_requests(
                                        &permission_check_result.needs_approval,
                                        tool_futures_arc.clone(),
                                        &mut permission_manager,
                                        message_tool_response.clone(),
                                        cancel_token.clone(),
                                    );

                                    while let Some(msg) = tool_approval_stream.try_next().await? {
                                        yield AgentEvent::Message(msg);
                                    }

                                    tool_futures = {
                                        let mut futures_lock = tool_futures_arc.lock().await;
                                        futures_lock.drain(..).collect::<Vec<_>>()
                                    };

                                    let with_id = tool_futures
                                        .into_iter()
                                        .map(|(request_id, stream)| {
                                            stream.map(move |item| (request_id.clone(), item))
                                        })
                                        .collect::<Vec<_>>();

                                    let mut combined = stream::select_all(with_id);
                                    let mut all_install_successful = true;

                                    while let Some((request_id, item)) = combined.next().await {
                                        if is_token_cancelled(&cancel_token) {
                                            break;
                                        }
                                        match item {
                                            ToolStreamItem::Result(output) => {
                                                if enable_extension_request_ids.contains(&request_id)
                                                    && output.is_err()
                                                {
                                                    all_install_successful = false;
                                                }
                                                let mut response = message_tool_response.lock().await;
                                                *response =
                                                    response.clone().with_tool_response(request_id, output);
                                            }
                                            ToolStreamItem::Message(msg) => {
                                                yield AgentEvent::McpNotification((
                                                    request_id, msg,
                                                ));
                                            }
                                        }
                                    }

                                    if all_install_successful {
                                        tools_updated = true;
                                    }
                                }

                                let final_message_tool_resp = message_tool_response.lock().await.clone();
                                yield AgentEvent::Message(final_message_tool_resp.clone());

                                added_message = true;
                                messages_to_add.push(response);
                                messages_to_add.push(final_message_tool_resp);
                            }
                        }
                        Err(ProviderError::ContextLengthExceeded(_)) => {
                            yield AgentEvent::Message(Message::assistant().with_context_length_exceeded(
                                    "The context length of the model has been exceeded. Please start a new session and try again.",
                                ));
                            break;
                        }
                        Err(e) => {
                            error!("Error: {}", e);
                            yield AgentEvent::Message(Message::assistant().with_text(
                                    format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error.")
                                ));
                            break;
                        }
                    }
                }
                if tools_updated {
                    (tools, toolshim_tools, system_prompt) = self.prepare_tools_and_prompt().await?;
                }
                if !added_message {
                    if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                        if final_output_tool.final_output.is_none() {
                            tracing::warn!("Final output tool has not been called yet. Continuing agent loop.");
                            let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                            continue
                        } else {
                            let message = Message::assistant().with_text(final_output_tool.final_output.clone().unwrap());
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                        }
                    }

                    match self.handle_retry_logic(&mut messages, &session, &initial_messages).await {
                        Ok(should_retry) => {
                            if should_retry {
                                info!("Retry logic triggered, restarting agent loop");
                                continue;
                            }
                        }
                        Err(e) => {
                            error!("Retry logic failed: {}", e);
                            yield AgentEvent::Message(Message::assistant().with_text(
                                format!("Retry logic encountered an error: {}", e)
                            ));
                        }
                    }
                    break;
                }

                messages.extend(messages_to_add);

                tokio::task::yield_now().await;
            }
        }))
    }

    fn determine_goose_mode(session: Option<&SessionConfig>, config: &Config) -> String {
        let mode = session.and_then(|s| s.execution_mode.as_deref());

        match mode {
            Some("foreground") => "chat".to_string(),
            Some("background") => "auto".to_string(),
            _ => config
                .get_param("GOOSE_MODE")
                .unwrap_or_else(|_| "auto".to_string()),
        }
    }

    /// Extend the system prompt with one line of additional instruction
    pub async fn extend_system_prompt(&self, instruction: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.add_system_prompt_extra(instruction);
    }

    pub async fn update_provider(&self, provider: Arc<dyn Provider>) -> Result<()> {
        let mut current_provider = self.provider.lock().await;
        *current_provider = Some(provider.clone());

        self.update_router_tool_selector(Some(provider), None)
            .await?;
        Ok(())
    }

    pub async fn update_router_tool_selector(
        &self,
        provider: Option<Arc<dyn Provider>>,
        reindex_all: Option<bool>,
    ) -> Result<()> {
        let provider = match provider {
            Some(p) => p,
            None => self.provider().await?,
        };

        // Delegate to ToolRouteManager
        self.tool_route_manager
            .update_router_tool_selector(provider, reindex_all, &self.extension_manager)
            .await
    }

    /// Override the system prompt with a custom template
    pub async fn override_system_prompt(&self, template: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.set_system_prompt_override(template);
    }

    pub async fn list_extension_prompts(&self) -> HashMap<String, Vec<Prompt>> {
        let extension_manager = self.extension_manager.read().await;
        extension_manager
            .list_prompts(CancellationToken::default())
            .await
            .expect("Failed to list prompts")
    }

    pub async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult> {
        let extension_manager = self.extension_manager.read().await;

        // First find which extension has this prompt
        let prompts = extension_manager
            .list_prompts(CancellationToken::default())
            .await
            .map_err(|e| anyhow!("Failed to list prompts: {}", e))?;

        if let Some(extension) = prompts
            .iter()
            .find(|(_, prompt_list)| prompt_list.iter().any(|p| p.name == name))
            .map(|(extension, _)| extension)
        {
            return extension_manager
                .get_prompt(extension, name, arguments, CancellationToken::default())
                .await
                .map_err(|e| anyhow!("Failed to get prompt: {}", e));
        }

        Err(anyhow!("Prompt '{}' not found", name))
    }

    pub async fn get_plan_prompt(&self) -> Result<String> {
        let extension_manager = self.extension_manager.read().await;
        let tools = extension_manager.get_prefixed_tools(None).await?;
        let tools_info = tools
            .into_iter()
            .map(|tool| {
                ToolInfo::new(
                    &tool.name,
                    tool.description
                        .as_ref()
                        .map(|d| d.as_ref())
                        .unwrap_or_default(),
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
            error!("Failed to send tool result: {}", e);
        }
    }

    pub async fn create_recipe(&self, mut messages: Conversation) -> Result<Recipe> {
        let extension_manager = self.extension_manager.read().await;
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
            .complete(&system_prompt, messages.messages(), &tools)
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
                let bullet_re = Regex::new(r"^[•\-*\d]+\.?\s*").expect("Invalid regex");

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

        // Ideally we'd get the name of the provider we are using from the provider itself,
        // but it doesn't know and the plumbing looks complicated.
        let config = Config::global();
        let provider_name: String = config
            .get_param("GOOSE_PROVIDER")
            .expect("No provider configured. Run 'goose configure' first");

        let settings = Settings {
            goose_provider: Some(provider_name.clone()),
            goose_model: Some(model_name.clone()),
            temperature: Some(model_config.temperature.unwrap_or(0.0)),
        };

        let recipe = Recipe::builder()
            .title("Custom recipe from chat")
            .description("a custom recipe instance from this chat session")
            .instructions(instructions)
            .activities(activities)
            .extensions(extension_configs)
            .settings(settings)
            .author(author)
            .build()
            .expect("valid recipe");

        Ok(recipe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Response;

    #[tokio::test]
    async fn test_add_final_output_tool() -> Result<()> {
        let agent = Agent::new();

        let response = Response {
            json_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"}
                }
            })),
        };

        agent.add_final_output_tool(response).await;

        let tools = agent.list_tools(None).await;
        let final_output_tool = tools
            .iter()
            .find(|tool| tool.name == FINAL_OUTPUT_TOOL_NAME);

        assert!(
            final_output_tool.is_some(),
            "Final output tool should be present after adding"
        );

        let prompt_manager = agent.prompt_manager.lock().await;
        let system_prompt =
            prompt_manager.build_system_prompt(vec![], None, Value::Null, None, None);

        let final_output_tool_ref = agent.final_output_tool.lock().await;
        let final_output_tool_system_prompt =
            final_output_tool_ref.as_ref().unwrap().system_prompt();
        assert!(system_prompt.contains(&final_output_tool_system_prompt));
        Ok(())
    }
}
