use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::stream::BoxStream;
use futures::{FutureExt, Stream, TryStreamExt};
use futures_util::stream;
use futures_util::stream::StreamExt;
use mcp_core::protocol::JsonRpcMessage;

use crate::config::{Config, ExtensionConfigManager, PermissionManager};
use crate::message::Message;
use crate::permission::permission_judge::check_tool_permissions;
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::recipe::{Author, Recipe};
use crate::scheduler_trait::SchedulerTrait;
use crate::tool_monitor::{ToolCall, ToolMonitor};
use regex::Regex;
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, instrument};

use crate::agents::extension::{ExtensionConfig, ExtensionError, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
    PLATFORM_MANAGE_SCHEDULE_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
    PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::agents::prompt_manager::PromptManager;
use crate::agents::router_tool_selector::{
    create_tool_selector, RouterToolSelectionStrategy, RouterToolSelector,
};
use crate::agents::router_tools::ROUTER_VECTOR_SEARCH_TOOL_NAME;
use crate::agents::tool_router_index_manager::ToolRouterIndexManager;
use crate::agents::tool_vectordb::generate_table_id;
use crate::agents::types::SessionConfig;
use crate::agents::types::{FrontendTool, ToolResultReceiver};
use mcp_core::{
    prompt::Prompt, protocol::GetPromptResult, tool::Tool, Content, ToolError, ToolResult,
};

use super::platform_tools;
use super::router_tools;
use super::tool_execution::{ToolCallResult, CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};

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
    pub(super) router_tool_selector: Mutex<Option<Arc<Box<dyn RouterToolSelector>>>>,
    pub(super) scheduler_service: Mutex<Option<Arc<dyn SchedulerTrait>>>,
}

#[derive(Clone, Debug)]
pub enum AgentEvent {
    Message(Message),
    McpNotification((String, JsonRpcMessage)),
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
            scheduler_service: Mutex::new(None),
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

    /// Set the scheduler service for this agent
    pub async fn set_scheduler(&self, scheduler: Arc<dyn SchedulerTrait>) {
        let mut scheduler_service = self.scheduler_service.lock().await;
        *scheduler_service = Some(scheduler);
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}

pub enum ToolStreamItem<T> {
    Message(JsonRpcMessage),
    Result(T),
}

pub type ToolStream = Pin<Box<dyn Stream<Item = ToolStreamItem<ToolResult<Vec<Content>>>> + Send>>;

// tool_stream combines a stream of JsonRpcMessages with a future representing the
// final result of the tool call. MCP notifications are not request-scoped, but
// this lets us capture all notifications emitted during the tool call for
// simpler consumption
pub fn tool_stream<S, F>(rx: S, done: F) -> ToolStream
where
    S: Stream<Item = JsonRpcMessage> + Send + Unpin + 'static,
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

        let extension_manager = self.extension_manager.lock().await;
        let result: ToolCallResult = if tool_call.name == PLATFORM_READ_RESOURCE_TOOL_NAME {
            // Check if the tool is read_resource and handle it separately
            ToolCallResult::from(
                extension_manager
                    .read_resource(tool_call.arguments.clone())
                    .await,
            )
        } else if tool_call.name == PLATFORM_LIST_RESOURCES_TOOL_NAME {
            ToolCallResult::from(
                extension_manager
                    .list_resources(tool_call.arguments.clone())
                    .await,
            )
        } else if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
            ToolCallResult::from(extension_manager.search_available_extensions().await)
        } else if self.is_frontend_tool(&tool_call.name).await {
            // For frontend tools, return an error indicating we need frontend execution
            ToolCallResult::from(Err(ToolError::ExecutionError(
                "Frontend tool execution required".to_string(),
            )))
        } else if tool_call.name == ROUTER_VECTOR_SEARCH_TOOL_NAME {
            let selector = self.router_tool_selector.lock().await.clone();
            ToolCallResult::from(if let Some(selector) = selector {
                selector.select_tools(tool_call.arguments.clone()).await
            } else {
                Err(ToolError::ExecutionError(
                    "Encountered vector search error.".to_string(),
                ))
            })
        } else {
            // Clone the result to ensure no references to extension_manager are returned
            let result = extension_manager
                .dispatch_tool_call(tool_call.clone())
                .await;
            match result {
                Ok(call_result) => call_result,
                Err(e) => ToolCallResult::from(Err(ToolError::ExecutionError(e.to_string()))),
            }
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

        // Update vector index if operation was successful and vector routing is enabled
        if result.is_ok() {
            let selector = self.router_tool_selector.lock().await.clone();
            if ToolRouterIndexManager::vector_tool_router_enabled(&selector) {
                if let Some(selector) = selector {
                    let vector_action = if action == "disable" { "remove" } else { "add" };
                    let extension_manager = self.extension_manager.lock().await;
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

    pub(crate) async fn handle_schedule_management(
        &self,
        arguments: serde_json::Value,
        _request_id: String,
    ) -> ToolResult<Vec<Content>> {
        let scheduler = match self.scheduler_service.lock().await.as_ref() {
            Some(s) => s.clone(),
            None => {
                return Err(ToolError::ExecutionError(
                    "Scheduler not available. This tool only works in server mode.".to_string(),
                ))
            }
        };

        let action = arguments
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'action' parameter".to_string()))?;

        match action {
            "list" => self.handle_list_jobs(scheduler).await,
            "create" => self.handle_create_job(scheduler, arguments).await,
            "run_now" => self.handle_run_now(scheduler, arguments).await,
            "pause" => self.handle_pause_job(scheduler, arguments).await,
            "unpause" => self.handle_unpause_job(scheduler, arguments).await,
            "delete" => self.handle_delete_job(scheduler, arguments).await,
            "kill" => self.handle_kill_job(scheduler, arguments).await,
            "inspect" => self.handle_inspect_job(scheduler, arguments).await,
            "sessions" => self.handle_list_sessions(scheduler, arguments).await,
            _ => Err(ToolError::ExecutionError(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }

    async fn handle_list_jobs(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
    ) -> ToolResult<Vec<Content>> {
        match scheduler.list_scheduled_jobs().await {
            Ok(jobs) => {
                let jobs_json = serde_json::to_string_pretty(&jobs).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to serialize jobs: {}", e))
                })?;
                Ok(vec![Content::text(format!(
                    "Scheduled Jobs:\n{}",
                    jobs_json
                ))])
            }
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to list jobs: {}",
                e
            ))),
        }
    }

    async fn handle_create_job(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let recipe_path = arguments
            .get("recipe_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::ExecutionError("Missing 'recipe_path' parameter".to_string())
            })?;

        let cron_expression = arguments
            .get("cron_expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::ExecutionError("Missing 'cron_expression' parameter".to_string())
            })?;

        // Validate recipe file exists and is readable
        if !std::path::Path::new(recipe_path).exists() {
            return Err(ToolError::ExecutionError(format!(
                "Recipe file not found: {}",
                recipe_path
            )));
        }

        // Validate it's a valid recipe by trying to parse it
        match std::fs::read_to_string(recipe_path) {
            Ok(content) => {
                if recipe_path.ends_with(".json") {
                    serde_json::from_str::<Recipe>(&content).map_err(|e| {
                        ToolError::ExecutionError(format!("Invalid JSON recipe: {}", e))
                    })?;
                } else {
                    serde_yaml::from_str::<Recipe>(&content).map_err(|e| {
                        ToolError::ExecutionError(format!("Invalid YAML recipe: {}", e))
                    })?;
                }
            }
            Err(e) => {
                return Err(ToolError::ExecutionError(format!(
                    "Cannot read recipe file: {}",
                    e
                )))
            }
        }

        // Generate unique job ID
        let job_id = format!("agent_created_{}", Utc::now().timestamp());

        let job = crate::scheduler::ScheduledJob {
            id: job_id.clone(),
            source: recipe_path.to_string(),
            cron: cron_expression.to_string(),
            last_run: None,
            currently_running: false,
            paused: false,
            current_session_id: None,
            process_start_time: None,
        };

        match scheduler.add_scheduled_job(job).await {
            Ok(()) => Ok(vec![Content::text(format!(
                "Successfully created scheduled job '{}' for recipe '{}' with cron expression '{}'",
                job_id, recipe_path, cron_expression
            ))]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to create job: {}",
                e
            ))),
        }
    }

    async fn handle_run_now(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let job_id = arguments
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'job_id' parameter".to_string()))?;

        match scheduler.run_now(job_id).await {
            Ok(session_id) => Ok(vec![Content::text(format!(
                "Successfully started job '{}'. Session ID: {}",
                job_id, session_id
            ))]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to run job: {}",
                e
            ))),
        }
    }

    async fn handle_pause_job(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let job_id = arguments
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'job_id' parameter".to_string()))?;

        match scheduler.pause_schedule(job_id).await {
            Ok(()) => Ok(vec![Content::text(format!(
                "Successfully paused job '{}'",
                job_id
            ))]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to pause job: {}",
                e
            ))),
        }
    }

    async fn handle_unpause_job(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let job_id = arguments
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'job_id' parameter".to_string()))?;

        match scheduler.unpause_schedule(job_id).await {
            Ok(()) => Ok(vec![Content::text(format!(
                "Successfully unpaused job '{}'",
                job_id
            ))]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to unpause job: {}",
                e
            ))),
        }
    }

    async fn handle_delete_job(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let job_id = arguments
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'job_id' parameter".to_string()))?;

        match scheduler.remove_scheduled_job(job_id).await {
            Ok(()) => Ok(vec![Content::text(format!(
                "Successfully deleted job '{}'",
                job_id
            ))]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to delete job: {}",
                e
            ))),
        }
    }

    async fn handle_kill_job(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let job_id = arguments
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'job_id' parameter".to_string()))?;

        match scheduler.kill_running_job(job_id).await {
            Ok(()) => Ok(vec![Content::text(format!(
                "Successfully killed running job '{}'",
                job_id
            ))]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to kill job: {}",
                e
            ))),
        }
    }

    async fn handle_inspect_job(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let job_id = arguments
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'job_id' parameter".to_string()))?;

        match scheduler.get_running_job_info(job_id).await {
            Ok(Some((session_id, start_time))) => {
                let duration = Utc::now().signed_duration_since(start_time);
                Ok(vec![Content::text(format!(
                    "Job '{}' is currently running:\n- Session ID: {}\n- Started: {}\n- Duration: {} seconds",
                    job_id, session_id, start_time.to_rfc3339(), duration.num_seconds()
                ))])
            }
            Ok(None) => Ok(vec![Content::text(format!(
                "Job '{}' is not currently running",
                job_id
            ))]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to inspect job: {}",
                e
            ))),
        }
    }

    async fn handle_list_sessions(
        &self,
        scheduler: Arc<dyn SchedulerTrait>,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let job_id = arguments
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'job_id' parameter".to_string()))?;

        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        match scheduler.sessions(job_id, limit).await {
            Ok(sessions) => {
                if sessions.is_empty() {
                    Ok(vec![Content::text(format!(
                        "No sessions found for job '{}'",
                        job_id
                    ))])
                } else {
                    let sessions_info: Vec<String> = sessions
                        .into_iter()
                        .map(|(session_name, metadata)| {
                            format!(
                                "- Session: {} (Messages: {}, Working Dir: {})",
                                session_name,
                                metadata.message_count,
                                metadata.working_dir.display()
                            )
                        })
                        .collect();

                    Ok(vec![Content::text(format!(
                        "Sessions for job '{}':\n{}",
                        job_id,
                        sessions_info.join("\n")
                    ))])
                }
            }
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to list sessions: {}",
                e
            ))),
        }
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
        let selector = self.router_tool_selector.lock().await.clone();
        if ToolRouterIndexManager::vector_tool_router_enabled(&selector) {
            if let Some(selector) = selector {
                let extension_manager = self.extension_manager.lock().await;
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
        let extension_manager = self.extension_manager.lock().await;
        let mut prefixed_tools = extension_manager
            .get_prefixed_tools(extension_name.clone())
            .await
            .unwrap_or_default();

        if extension_name.is_none() || extension_name.as_deref() == Some("platform") {
            // Add platform tools
            prefixed_tools.push(platform_tools::search_available_extensions_tool());
            prefixed_tools.push(platform_tools::manage_extensions_tool());
            prefixed_tools.push(platform_tools::manage_schedule_tool());

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
        let selector = self.router_tool_selector.lock().await.clone();
        if let Some(selector) = selector {
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

    pub async fn remove_extension(&self, name: &str) -> Result<()> {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager.remove_extension(name).await?;

        // If vector tool selection is enabled, remove tools from the index
        let selector = self.router_tool_selector.lock().await.clone();
        if ToolRouterIndexManager::vector_tool_router_enabled(&selector) {
            if let Some(selector) = selector {
                let extension_manager = self.extension_manager.lock().await;
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
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<AgentEvent>>> {
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
                        let selector = self.router_tool_selector.lock().await.clone();
                        if let Some(selector) = selector {
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
                        yield AgentEvent::Message(filtered_response.clone());

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
                            yield AgentEvent::Message(msg);
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
                            let mut tool_futures: Vec<(String, ToolStream)> = Vec::new();

                            // Skip the confirmation for approved tools
                            for request in &permission_check_result.approved {
                                if let Ok(tool_call) = request.tool_call.clone() {
                                    let (req_id, tool_result) = self.dispatch_tool_call(tool_call, request.id.clone()).await;

                                    tool_futures.push((req_id, match tool_result {
                                        Ok(result) => tool_stream(
                                            result.notification_stream.unwrap_or_else(|| Box::new(stream::empty())),
                                            result.result,
                                        ),
                                        Err(e) => tool_stream(
                                            Box::new(stream::empty()),
                                            futures::future::ready(Err(e)),
                                        ),
                                    }));
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
                                yield AgentEvent::Message(msg);
                            }

                            tool_futures = {
                                // Lock the mutex asynchronously
                                let mut futures_lock = tool_futures_arc.lock().await;
                                // Drain the vector and collect into a new Vec
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
                                match item {
                                    ToolStreamItem::Result(output) => {
                                        if enable_extension_request_ids.contains(&request_id) && output.is_err(){
                                            all_install_successful = false;
                                        }
                                        let mut response = message_tool_response.lock().await;
                                        *response = response.clone().with_tool_response(request_id, output);
                                    },
                                    ToolStreamItem::Message(msg) => {
                                        yield AgentEvent::McpNotification((request_id, msg))
                                    }
                                }
                            }

                            // Update system prompt and tools if installations were successful
                            if all_install_successful {
                                (tools, toolshim_tools, system_prompt) = self.prepare_tools_and_prompt().await?;
                            }
                        }

                        let final_message_tool_resp = message_tool_response.lock().await.clone();
                        yield AgentEvent::Message(final_message_tool_resp.clone());

                        messages.push(response);
                        messages.push(final_message_tool_resp);
                    },
                    Err(ProviderError::ContextLengthExceeded(_)) => {
                        // At this point, the last message should be a user message
                        // because call to provider led to context length exceeded error
                        // Immediately yield a special message and break
                        yield AgentEvent::Message(Message::assistant().with_context_length_exceeded(
                            "The context length of the model has been exceeded. Please start a new session and try again.",
                        ));
                        break;
                    },
                    Err(e) => {
                        // Create an error message & terminate the stream
                        error!("Error: {}", e);
                        yield AgentEvent::Message(Message::assistant().with_text(format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error.")));
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

        let strategy = match router_tool_selection_strategy.to_lowercase().as_str() {
            "vector" => Some(RouterToolSelectionStrategy::Vector),
            _ => None,
        };

        if let Some(strategy) = strategy {
            let table_name = generate_table_id();
            let selector = create_tool_selector(Some(strategy), provider, table_name)
                .await
                .map_err(|e| anyhow!("Failed to create tool selector: {}", e))?;

            let selector = Arc::new(selector);
            *self.router_tool_selector.lock().await = Some(selector.clone());

            let extension_manager = self.extension_manager.lock().await;
            ToolRouterIndexManager::index_platform_tools(&selector, &extension_manager).await?;
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

#[cfg(test)]
mod schedule_tool_tests {
    use super::*;
    use crate::scheduler::{ScheduledJob, SchedulerError};
    use crate::scheduler_trait::SchedulerTrait;
    use crate::session::storage::SessionMetadata;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use mcp_core::{Content, ToolError};
    use serde_json::json;
    use std::sync::Arc;

    // Mock scheduler for testing
    struct MockScheduler {
        jobs: tokio::sync::Mutex<Vec<ScheduledJob>>,
    }

    impl MockScheduler {
        fn new() -> Self {
            Self {
                jobs: tokio::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl SchedulerTrait for MockScheduler {
        async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
            let mut jobs = self.jobs.lock().await;
            jobs.push(job);
            Ok(())
        }

        async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
            let jobs = self.jobs.lock().await;
            Ok(jobs.clone())
        }

        async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
            let mut jobs = self.jobs.lock().await;
            if let Some(pos) = jobs.iter().position(|job| job.id == id) {
                jobs.remove(pos);
                Ok(())
            } else {
                Err(SchedulerError::JobNotFound(id.to_string()))
            }
        }

        async fn pause_schedule(&self, _id: &str) -> Result<(), SchedulerError> {
            Ok(())
        }

        async fn unpause_schedule(&self, _id: &str) -> Result<(), SchedulerError> {
            Ok(())
        }

        async fn run_now(&self, _id: &str) -> Result<String, SchedulerError> {
            Ok("test_session_123".to_string())
        }

        async fn sessions(
            &self,
            _sched_id: &str,
            _limit: usize,
        ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
            Ok(vec![])
        }

        async fn update_schedule(
            &self,
            _sched_id: &str,
            _new_cron: String,
        ) -> Result<(), SchedulerError> {
            Ok(())
        }

        async fn kill_running_job(&self, _sched_id: &str) -> Result<(), SchedulerError> {
            Ok(())
        }

        async fn get_running_job_info(
            &self,
            _sched_id: &str,
        ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_schedule_management_tool_list() {
        let agent = Agent::new();
        let mock_scheduler = Arc::new(MockScheduler::new());
        agent.set_scheduler(mock_scheduler.clone()).await;

        // Test list action
        let arguments = json!({
            "action": "list"
        });

        let result = agent
            .handle_schedule_management(arguments, "test_req".to_string())
            .await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert_eq!(content.len(), 1);
        if let Content::Text(text_content) = &content[0] {
            assert!(text_content.text.contains("Scheduled Jobs:"));
        }
    }

    #[tokio::test]
    async fn test_schedule_management_tool_no_scheduler() {
        let agent = Agent::new();
        // Don't set scheduler

        let arguments = json!({
            "action": "list"
        });

        let result = agent
            .handle_schedule_management(arguments, "test_req".to_string())
            .await;
        assert!(result.is_err());

        if let Err(ToolError::ExecutionError(msg)) = result {
            assert!(msg.contains("Scheduler not available"));
        }
    }

    #[tokio::test]
    async fn test_schedule_management_tool_run_now() {
        let agent = Agent::new();
        let mock_scheduler = Arc::new(MockScheduler::new());
        agent.set_scheduler(mock_scheduler.clone()).await;

        let arguments = json!({
            "action": "run_now",
            "job_id": "test_job"
        });

        let result = agent
            .handle_schedule_management(arguments, "test_req".to_string())
            .await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert_eq!(content.len(), 1);
        if let Content::Text(text_content) = &content[0] {
            assert!(text_content
                .text
                .contains("Successfully started job 'test_job'"));
            assert!(text_content.text.contains("test_session_123"));
        }
    }

    #[tokio::test]
    async fn test_schedule_management_tool_dispatch() {
        let agent = Agent::new();
        let mock_scheduler = Arc::new(MockScheduler::new());
        agent.set_scheduler(mock_scheduler.clone()).await;

        // Test that the tool is properly dispatched through dispatch_tool_call
        let tool_call = mcp_core::tool::ToolCall {
            name: PLATFORM_MANAGE_SCHEDULE_TOOL_NAME.to_string(),
            arguments: json!({
                "action": "list"
            }),
        };

        let (request_id, result) = agent
            .dispatch_tool_call(tool_call, "test_dispatch".to_string())
            .await;
        assert_eq!(request_id, "test_dispatch");
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        // The result should be a future that resolves to the tool output
        let output = tool_result.result.await;
        assert!(output.is_ok());

        let content = output.unwrap();
        assert_eq!(content.len(), 1);
        if let Content::Text(text_content) = &content[0] {
            assert!(text_content.text.contains("Scheduled Jobs:"));
        }
    }

    #[tokio::test]
    async fn test_schedule_management_tool_in_list_tools() {
        let agent = Agent::new();
        let tools = agent.list_tools(None).await;

        // Check that the schedule management tool is included in the list
        let schedule_tool = tools
            .iter()
            .find(|tool| tool.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME);
        assert!(schedule_tool.is_some());

        let tool = schedule_tool.unwrap();
        assert!(tool
            .description
            .contains("Manage scheduled recipe execution"));
    }
}
