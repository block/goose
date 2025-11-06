use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::stream::BoxStream;
use futures::{stream, FutureExt, Stream, StreamExt, TryStreamExt};
use uuid::Uuid;

use crate::agents::extension::{ExtensionConfig, ExtensionError, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::extension_manager_extension::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::agents::final_output_tool::{FINAL_OUTPUT_CONTINUATION_MESSAGE, FINAL_OUTPUT_TOOL_NAME};
use crate::agents::platform_tools::PLATFORM_MANAGE_SCHEDULE_TOOL_NAME;
use crate::agents::prompt_manager::PromptManager;
use crate::agents::recipe_tools::dynamic_task_tools::{
    create_dynamic_task, create_dynamic_task_tool, DYNAMIC_TASK_TOOL_NAME_PREFIX,
};
use crate::agents::retry::{RetryManager, RetryResult};
use crate::agents::router_tools::ROUTER_LLM_SEARCH_TOOL_NAME;
use crate::agents::sub_recipe_manager::SubRecipeManager;
use crate::agents::subagent_execution_tool::lib::ExecutionMode;
use crate::agents::subagent_execution_tool::subagent_execute_task_tool::{
    self, SUBAGENT_EXECUTE_TASK_TOOL_NAME,
};
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::agents::tool_route_manager::ToolRouteManager;
use crate::agents::tool_router_index_manager::ToolRouterIndexManager;
use crate::agents::types::SessionConfig;
use crate::agents::types::{FrontendTool, SharedProvider, ToolResultReceiver};
use crate::config::{get_enabled_extensions, Config, GooseMode};
use crate::context_mgmt::DEFAULT_COMPACTION_THRESHOLD;
use crate::conversation::{debug_conversation_fix, fix_conversation, Conversation};
use crate::mcp_utils::ToolResult;
use crate::permission::permission_inspector::PermissionInspector;
use crate::permission::permission_judge::PermissionCheckResult;
use crate::permission::PermissionConfirmation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::recipe::{Author, Recipe, Response, Settings, SubRecipe};
use crate::scheduler_trait::SchedulerTrait;
use crate::security::security_inspector::SecurityInspector;
use crate::tool_inspection::ToolInspectionManager;
use crate::tool_monitor::RepetitionInspector;
use crate::utils::is_token_cancelled;
use regex::Regex;
use rmcp::model::{
    CallToolRequestParam, Content, ErrorCode, ErrorData, GetPromptResult, Prompt,
    ServerNotification, Tool,
};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use super::final_output_tool::FinalOutputTool;
use super::model_selector::autopilot::AutoPilot;
use super::platform_tools;
use super::tool_execution::{ToolCallResult, CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};
use crate::agents::subagent_task_config::TaskConfig;
use crate::conversation::message::{Message, MessageContent, SystemNotificationType, ToolRequest};
use crate::session::extension_data::{EnabledExtensionsState, ExtensionState};
use crate::session::{Session, SessionManager};

const DEFAULT_MAX_TURNS: u32 = 1000;
const COMPACTION_THINKING_TEXT: &str = "goose is compacting the conversation...";
pub const MANUAL_COMPACT_TRIGGER: &str = "Please compact this conversation";

/// Context needed for the reply function
pub struct ReplyContext {
    pub conversation: Conversation,
    pub tools: Vec<Tool>,
    pub toolshim_tools: Vec<Tool>,
    pub system_prompt: String,
    pub goose_mode: GooseMode,
    pub initial_messages: Vec<Message>,
}

pub struct ToolCategorizeResult {
    pub frontend_requests: Vec<ToolRequest>,
    pub remaining_requests: Vec<ToolRequest>,
    pub filtered_response: Message,
}

/// The main goose Agent
pub struct Agent {
    pub(super) provider: SharedProvider,

    pub extension_manager: Arc<ExtensionManager>,
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

    pub tool_route_manager: Arc<ToolRouteManager>,
    pub(super) scheduler_service: Mutex<Option<Arc<dyn SchedulerTrait>>>,
    pub(super) retry_manager: RetryManager,
    pub(super) tool_inspection_manager: ToolInspectionManager,
    pub(super) autopilot: Mutex<AutoPilot>,
}

#[derive(Clone, Debug)]
pub enum AgentEvent {
    Message(Message),
    McpNotification((String, ServerNotification)),
    ModelChange { model: String, mode: String },
    HistoryReplaced(Conversation),
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
        let provider = Arc::new(Mutex::new(None));

        Self {
            provider: provider.clone(),
            extension_manager: Arc::new(ExtensionManager::new(provider.clone())),
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
            tool_route_manager: Arc::new(ToolRouteManager::new()),
            scheduler_service: Mutex::new(None),
            retry_manager: RetryManager::new(),
            tool_inspection_manager: Self::create_default_tool_inspection_manager(),
            autopilot: Mutex::new(AutoPilot::new()),
        }
    }

    /// Create a tool inspection manager with default inspectors
    fn create_default_tool_inspection_manager() -> ToolInspectionManager {
        let mut tool_inspection_manager = ToolInspectionManager::new();

        // Add security inspector (highest priority - runs first)
        tool_inspection_manager.add_inspector(Box::new(SecurityInspector::new()));

        // Add permission inspector (medium-high priority)
        // Note: mode will be updated dynamically based on session config
        tool_inspection_manager.add_inspector(Box::new(PermissionInspector::new(
            GooseMode::SmartApprove,
            std::collections::HashSet::new(), // readonly tools - will be populated from extension manager
            std::collections::HashSet::new(), // regular tools - will be populated from extension manager
        )));

        // Add repetition inspector (lower priority - basic repetition checking)
        tool_inspection_manager.add_inspector(Box::new(RepetitionInspector::new(None)));

        tool_inspection_manager
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

    async fn handle_retry_logic(
        &self,
        messages: &mut Conversation,
        session_config: &SessionConfig,
        initial_messages: &[Message],
    ) -> Result<bool> {
        let result = self
            .retry_manager
            .handle_retry_logic(
                messages,
                session_config,
                initial_messages,
                &self.final_output_tool,
            )
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
    ) -> Result<ReplyContext> {
        let unfixed_messages = unfixed_conversation.messages().clone();
        let (conversation, issues) = fix_conversation(unfixed_conversation.clone());
        if !issues.is_empty() {
            debug!(
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
        let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

        self.tool_inspection_manager
            .update_permission_inspector_mode(goose_mode)
            .await;

        Ok(ReplyContext {
            conversation,
            tools,
            toolshim_tools,
            system_prompt,
            goose_mode,
            initial_messages,
        })
    }

    async fn categorize_tools(
        &self,
        response: &Message,
        _tools: &[rmcp::model::Tool],
    ) -> ToolCategorizeResult {
        // Categorize tool requests
        let (frontend_requests, remaining_requests, filtered_response) =
            self.categorize_tool_requests(response).await;

        ToolCategorizeResult {
            frontend_requests,
            remaining_requests,
            filtered_response,
        }
    }

    async fn handle_approved_and_denied_tools(
        &self,
        permission_check_result: &PermissionCheckResult,
        message_tool_response: Arc<Mutex<Message>>,
        cancel_token: Option<tokio_util::sync::CancellationToken>,
        session: &Session,
    ) -> Result<Vec<(String, ToolStream)>> {
        let mut tool_futures: Vec<(String, ToolStream)> = Vec::new();

        // Handle pre-approved and read-only tools
        for request in &permission_check_result.approved {
            if let Ok(tool_call) = request.tool_call.clone() {
                let (req_id, tool_result) = self
                    .dispatch_tool_call(
                        tool_call,
                        request.id.clone(),
                        cancel_token.clone(),
                        session,
                    )
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
        tool_call: CallToolRequestParam,
        request_id: String,
        cancellation_token: Option<CancellationToken>,
        session: &Session,
    ) -> (String, Result<ToolCallResult, ErrorData>) {
        if tool_call.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME {
            let arguments = tool_call
                .arguments
                .map(Value::Object)
                .unwrap_or(Value::Object(serde_json::Map::new()));
            let result = self
                .handle_schedule_management(arguments, request_id.clone())
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
                    Err(ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        "Final output tool not defined".to_string(),
                        None,
                    )),
                )
            };
        }

        debug!("WAITING_TOOL_START: {}", tool_call.name);
        let result: ToolCallResult = if self
            .sub_recipe_manager
            .lock()
            .await
            .is_sub_recipe_tool(&tool_call.name)
        {
            let sub_recipe_manager = self.sub_recipe_manager.lock().await;
            let arguments = tool_call
                .arguments
                .clone()
                .map(Value::Object)
                .unwrap_or(Value::Object(serde_json::Map::new()));
            sub_recipe_manager
                .dispatch_sub_recipe_tool_call(&tool_call.name, arguments, &self.tasks_manager)
                .await
        } else if tool_call.name == SUBAGENT_EXECUTE_TASK_TOOL_NAME {
            let provider = match self.provider().await {
                Ok(p) => p,
                Err(_) => {
                    return (
                        request_id,
                        Err(ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            "Provider is required".to_string(),
                            None,
                        )),
                    );
                }
            };

            // Get extensions from the agent's runtime state rather than global config
            // This ensures subagents inherit extensions that were dynamically enabled by the parent
            let extensions = self.get_extension_configs().await;

            let task_config =
                TaskConfig::new(provider, &session.id, &session.working_dir, extensions);

            let arguments = match tool_call.arguments.clone() {
                Some(args) => Value::Object(args),
                None => {
                    return (
                        request_id,
                        Err(ErrorData::new(
                            ErrorCode::INVALID_PARAMS,
                            "Tool call arguments are required".to_string(),
                            None,
                        )),
                    );
                }
            };
            let task_ids: Vec<String> = match arguments.get("task_ids") {
                Some(v) => match serde_json::from_value(v.clone()) {
                    Ok(ids) => ids,
                    Err(_) => {
                        return (
                            request_id,
                            Err(ErrorData::new(
                                ErrorCode::INVALID_PARAMS,
                                "Invalid task_ids format".to_string(),
                                None,
                            )),
                        );
                    }
                },
                None => {
                    return (
                        request_id,
                        Err(ErrorData::new(
                            ErrorCode::INVALID_PARAMS,
                            "task_ids parameter is required".to_string(),
                            None,
                        )),
                    );
                }
            };

            let execution_mode = arguments
                .get("execution_mode")
                .and_then(|v| serde_json::from_value::<ExecutionMode>(v.clone()).ok())
                .unwrap_or(ExecutionMode::Sequential);

            subagent_execute_task_tool::run_tasks(
                task_ids,
                execution_mode,
                task_config,
                &self.tasks_manager,
                cancellation_token,
            )
            .await
        } else if tool_call.name == DYNAMIC_TASK_TOOL_NAME_PREFIX {
            // Get loaded extensions for shortname resolution
            let loaded_extensions = self
                .extension_manager
                .list_extensions()
                .await
                .unwrap_or_default();
            let arguments = tool_call
                .arguments
                .clone()
                .map(Value::Object)
                .unwrap_or(Value::Object(serde_json::Map::new()));
            create_dynamic_task(arguments, &self.tasks_manager, loaded_extensions).await
        } else if self.is_frontend_tool(&tool_call.name).await {
            // For frontend tools, return an error indicating we need frontend execution
            ToolCallResult::from(Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Frontend tool execution required".to_string(),
                None,
            )))
        } else if tool_call.name == ROUTER_LLM_SEARCH_TOOL_NAME {
            match self
                .tool_route_manager
                .dispatch_route_search_tool(tool_call.arguments.unwrap_or_default())
                .await
            {
                Ok(tool_result) => tool_result,
                Err(e) => return (request_id, Err(e)),
            }
        } else {
            // Clone the result to ensure no references to extension_manager are returned
            let result = self
                .extension_manager
                .dispatch_tool_call(tool_call.clone(), cancellation_token.unwrap_or_default())
                .await;
            result.unwrap_or_else(|e| {
                ToolCallResult::from(Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    e.to_string(),
                    None,
                )))
            })
        };

        debug!("WAITING_TOOL_END: {}", tool_call.name);

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

    /// Save current extension state to session metadata
    /// Should be called after any extension add/remove operation
    pub async fn save_extension_state(&self, session: &SessionConfig) -> Result<()> {
        let extension_configs = self.extension_manager.get_extension_configs().await;

        let extensions_state = EnabledExtensionsState::new(extension_configs);

        let mut session_data = SessionManager::get_session(&session.id, false).await?;

        if let Err(e) = extensions_state.to_extension_data(&mut session_data.extension_data) {
            warn!("Failed to serialize extension state: {}", e);
            return Err(anyhow!("Extension state serialization failed: {}", e));
        }

        SessionManager::update_session(&session.id)
            .extension_data(session_data.extension_data)
            .apply()
            .await?;

        Ok(())
    }

    pub async fn add_extension(&self, extension: ExtensionConfig) -> ExtensionResult<()> {
        match &extension {
            ExtensionConfig::Frontend {
                tools,
                instructions,
                ..
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
                self.extension_manager
                    .add_extension(extension.clone())
                    .await?;
            }
        }

        // If LLM tool selection is functional, index the tools
        if self.tool_route_manager.is_router_functional().await {
            let selector = self.tool_route_manager.get_router_tool_selector().await;
            if let Some(selector) = selector {
                let selector = Arc::new(selector);
                if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                    &selector,
                    &self.extension_manager,
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
        let mut prefixed_tools = self
            .extension_manager
            .get_prefixed_tools(extension_name.clone())
            .await
            .unwrap_or_default();

        if extension_name.is_none() || extension_name.as_deref() == Some("platform") {
            // Add platform tools
            // TODO: migrate the manage schedule tool as well
            prefixed_tools.extend([platform_tools::manage_schedule_tool()]);
            // Dynamic task tool
            prefixed_tools.push(create_dynamic_task_tool());
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

    pub async fn list_tools_for_router(&self) -> Vec<Tool> {
        self.tool_route_manager
            .list_tools_for_router(&self.extension_manager)
            .await
    }

    pub async fn remove_extension(&self, name: &str) -> Result<()> {
        self.extension_manager.remove_extension(name).await?;

        // If LLM tool selection is functional, remove tools from the index
        if self.tool_route_manager.is_router_functional().await {
            let selector = self.tool_route_manager.get_router_tool_selector().await;
            if let Some(selector) = selector {
                ToolRouterIndexManager::update_extension_tools(
                    &selector,
                    &self.extension_manager,
                    name,
                    "remove",
                )
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_extensions(&self) -> Vec<String> {
        self.extension_manager
            .list_extensions()
            .await
            .expect("Failed to list extensions")
    }

    pub async fn get_extension_configs(&self) -> Vec<ExtensionConfig> {
        self.extension_manager.get_extension_configs().await
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

    #[instrument(skip(self, user_message, session_config), fields(user_message))]
    pub async fn reply(
        &self,
        user_message: Message,
        session_config: SessionConfig,
        cancel_token: Option<CancellationToken>,
    ) -> Result<BoxStream<'_, Result<AgentEvent>>> {
        let is_manual_compact = user_message.content.iter().any(|c| {
            if let MessageContent::Text(text) = c {
                text.text.trim() == MANUAL_COMPACT_TRIGGER
            } else {
                false
            }
        });

        SessionManager::add_message(&session_config.id, &user_message).await?;
        let session = SessionManager::get_session(&session_config.id, true).await?;

        let conversation = session
            .conversation
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Session {} has no conversation", session_config.id))?;

        let needs_auto_compact =
            crate::context_mgmt::check_if_compaction_needed(self, &conversation, None, &session)
                .await?;

        let conversation_to_compact = conversation.clone();

        Ok(Box::pin(async_stream::try_stream! {
            let final_conversation = if !needs_auto_compact {
                conversation
            } else {
                if !is_manual_compact {
                    let config = crate::config::Config::global();
                    let threshold = config
                        .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
                        .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);
                    let threshold_percentage = (threshold * 100.0) as u32;

                    let inline_msg = format!(
                        "Exceeded auto-compact threshold of {}%. Performing auto-compaction...",
                        threshold_percentage
                    );

                    yield AgentEvent::Message(
                        Message::assistant().with_system_notification(
                            SystemNotificationType::InlineMessage,
                            inline_msg,
                        )
                    );
                }

                yield AgentEvent::Message(
                    Message::assistant().with_system_notification(
                        SystemNotificationType::ThinkingMessage,
                        COMPACTION_THINKING_TEXT,
                    )
                );

                match crate::context_mgmt::compact_messages(self, &conversation_to_compact, false).await {
                    Ok((compacted_conversation, summarization_usage)) => {
                        SessionManager::replace_conversation(&session_config.id, &compacted_conversation).await?;
                        Self::update_session_metrics(&session_config, &summarization_usage, true).await?;

                        yield AgentEvent::HistoryReplaced(compacted_conversation.clone());

                        yield AgentEvent::Message(
                            Message::assistant().with_system_notification(
                                SystemNotificationType::InlineMessage,
                                "Compaction complete",
                            )
                        );

                        compacted_conversation
                    }
                    Err(e) => {
                        yield AgentEvent::Message(
                            Message::assistant().with_text(
                                format!("Ran into this error trying to compact: {e}.\n\nPlease try again or create a new session")
                            )
                        );
                        return;
                    }
                }
            };

            if !is_manual_compact {
                let mut reply_stream = self.reply_internal(final_conversation, session_config, session, cancel_token).await?;
                while let Some(event) = reply_stream.next().await {
                    yield event?;
                }
            }
        }))
    }

    async fn reply_internal(
        &self,
        conversation: Conversation,
        session_config: SessionConfig,
        session: Session,
        cancel_token: Option<CancellationToken>,
    ) -> Result<BoxStream<'_, Result<AgentEvent>>> {
        let context = self.prepare_reply_context(conversation).await?;
        let ReplyContext {
            mut conversation,
            mut tools,
            mut toolshim_tools,
            mut system_prompt,
            goose_mode,
            initial_messages,
        } = context;
        let reply_span = tracing::Span::current();
        self.reset_retry_attempts().await;

        let provider = self.provider().await?;
        let session_id = session_config.id.clone();
        tokio::spawn(async move {
            if let Err(e) = SessionManager::maybe_update_name(&session_id, provider).await {
                warn!("Failed to generate session description: {}", e);
            }
        });

        Ok(Box::pin(async_stream::try_stream! {
            let _ = reply_span.enter();
            let mut turns_taken = 0u32;
            let max_turns = session_config.max_turns.unwrap_or(DEFAULT_MAX_TURNS);

            loop {
                if is_token_cancelled(&cancel_token) {
                    break;
                }

                if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                    if final_output_tool.final_output.is_some() {
                        let final_event = AgentEvent::Message(
                            Message::assistant().with_text(final_output_tool.final_output.clone().unwrap())
                        );
                        yield final_event;
                        break;
                    }
                }

                turns_taken += 1;
                if turns_taken > max_turns {
                    yield AgentEvent::Message(
                        Message::assistant().with_text(
                            "I've reached the maximum number of actions I can do without user input. Would you like me to continue?"
                        )
                    );
                    break;
                }

                {
                    let mut autopilot = self.autopilot.lock().await;
                    if let Some((new_provider, role, model)) = autopilot.check_for_switch(&conversation, self.provider().await?).await? {
                        debug!("AutoPilot switching to {} role with model {}", role, model);
                        self.update_provider(new_provider).await?;

                        yield AgentEvent::ModelChange {
                            model: model.clone(),
                            mode: format!("autopilot:{}", role),
                        };
                    }
                }

                let mut stream = Self::stream_response_from_provider(
                    self.provider().await?,
                    &system_prompt,
                    conversation.messages(),
                    &tools,
                    &toolshim_tools,
                ).await?;

                let mut no_tools_called = true;
                let mut messages_to_add = Conversation::default();
                let mut tools_updated = false;
                let mut did_recovery_compact_this_iteration = false;

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

                            if let Some(ref usage) = usage {
                                Self::update_session_metrics(&session_config, usage, false).await?;
                            }

                            if let Some(response) = response {
                                messages_to_add.push(response.clone());
                                let ToolCategorizeResult {
                                    frontend_requests,
                                    remaining_requests,
                                    filtered_response,
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

                                if goose_mode == GooseMode::Chat {
                                    // Skip all tool calls in chat mode
                                    for request in remaining_requests {
                                        let mut response = message_tool_response.lock().await;
                                        *response = response.clone().with_tool_response(
                                            request.id.clone(),
                                            Ok(vec![Content::text(CHAT_MODE_TOOL_SKIPPED_RESPONSE)]),
                                        );
                                    }
                                } else {
                                    // Run all tool inspectors (security, repetition, permission, etc.)
                                    let inspection_results = self.tool_inspection_manager
                                        .inspect_tools(
                                            &remaining_requests,
                                            conversation.messages(),
                                        )
                                        .await?;

                                    // Process inspection results into permission decisions using the permission inspector
                                    let permission_check_result = self.tool_inspection_manager
                                        .process_inspection_results_with_permission_inspector(
                                            &remaining_requests,
                                            &inspection_results,
                                        )
                                        .unwrap_or_else(|| {
                                            // Fallback if permission inspector not found - default to needs approval
                                            let mut result = PermissionCheckResult {
                                                approved: vec![],
                                                needs_approval: vec![],
                                                denied: vec![],
                                            };
                                            result.needs_approval.extend(remaining_requests.iter().cloned());
                                            result
                                        });

                                    // Track extension requests for special handling
                                    let mut enable_extension_request_ids = vec![];
                                    for request in &remaining_requests {
                                        if let Ok(tool_call) = &request.tool_call {
                                            if tool_call.name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                                                enable_extension_request_ids.push(request.id.clone());
                                            }
                                        }
                                    }

                                    let mut tool_futures = self.handle_approved_and_denied_tools(
                                        &permission_check_result,
                                        message_tool_response.clone(),
                                        cancel_token.clone(),
                                        &session,
                                    ).await?;

                                    let tool_futures_arc = Arc::new(Mutex::new(tool_futures));

                                    let mut tool_approval_stream = self.handle_approval_tool_requests(
                                        &permission_check_result.needs_approval,
                                        tool_futures_arc.clone(),
                                        message_tool_response.clone(),
                                        cancel_token.clone(),
                                        &session,
                                        &inspection_results,
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

                                    if all_install_successful && !enable_extension_request_ids.is_empty() {
                                        if let Err(e) = self.save_extension_state(&session_config).await {
                                            warn!("Failed to save extension state after runtime changes: {}", e);
                                        }
                                        tools_updated = true;
                                    }
                                }

                                let final_message_tool_resp = message_tool_response.lock().await.clone();
                                yield AgentEvent::Message(final_message_tool_resp.clone());

                                no_tools_called = false;
                                messages_to_add.push(final_message_tool_resp);
                            }
                        }
                        Err(ProviderError::ContextLengthExceeded(_error_msg)) => {
                            yield AgentEvent::Message(
                                Message::assistant().with_system_notification(
                                    SystemNotificationType::InlineMessage,
                                    "Context limit reached. Compacting to continue conversation...",
                                )
                            );
                            yield AgentEvent::Message(
                                Message::assistant().with_system_notification(
                                    SystemNotificationType::ThinkingMessage,
                                    COMPACTION_THINKING_TEXT,
                                )
                            );

                            match crate::context_mgmt::compact_messages(self, &conversation, true).await {
                                Ok((compacted_conversation, usage)) => {
                                    SessionManager::replace_conversation(&session_config.id, &compacted_conversation).await?;
                                    Self::update_session_metrics(&session_config, &usage, true).await?;
                                    conversation = compacted_conversation;
                                    did_recovery_compact_this_iteration = true;
                                    yield AgentEvent::HistoryReplaced(conversation.clone());
                                    continue;
                                }
                                Err(e) => {
                                    error!("Error: {}", e);
                                    yield AgentEvent::Message(
                                        Message::assistant().with_text(
                                            format!("Ran into this error trying to compact: {e}.\n\nPlease retry if you think this is a transient or recoverable error.")
                                        )
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error: {}", e);
                            yield AgentEvent::Message(
                                Message::assistant().with_text(
                                    format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error.")
                                )
                            );
                            break;
                        }
                    }
                }
                if tools_updated {
                    (tools, toolshim_tools, system_prompt) = self.prepare_tools_and_prompt().await?;
                }
                let mut exit_chat = false;
                if no_tools_called {
                    if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                        if final_output_tool.final_output.is_none() {
                            warn!("Final output tool has not been called yet. Continuing agent loop.");
                            let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                        } else {
                            let message = Message::assistant().with_text(final_output_tool.final_output.clone().unwrap());
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                            exit_chat = true;
                        }
                    } else if did_recovery_compact_this_iteration {
                        // Avoid setting exit_chat; continue from last user message in the conversation
                    } else {
                        match self.handle_retry_logic(&mut conversation, &session_config, &initial_messages).await {
                            Ok(should_retry) => {
                                if should_retry {
                                    info!("Retry logic triggered, restarting agent loop");
                                } else {
                                    exit_chat = true;
                                }
                            }
                            Err(e) => {
                                error!("Retry logic failed: {}", e);
                                yield AgentEvent::Message(
                                    Message::assistant().with_text(
                                        format!("Retry logic encountered an error: {}", e)
                                    )
                                );
                                exit_chat = true;
                            }
                        }
                    }
                }

                for msg in &messages_to_add {
                    SessionManager::add_message(&session_config.id, msg).await?;
                }
                conversation.extend(messages_to_add);
                if exit_chat {
                    break;
                }

                tokio::task::yield_now().await;
            }
        }))
    }

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
        self.extension_manager
            .list_prompts(CancellationToken::default())
            .await
            .expect("Failed to list prompts")
    }

    pub async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult> {
        // First find which extension has this prompt
        let prompts = self
            .extension_manager
            .list_prompts(CancellationToken::default())
            .await
            .map_err(|e| anyhow!("Failed to list prompts: {}", e))?;

        if let Some(extension) = prompts
            .iter()
            .find(|(_, prompt_list)| prompt_list.iter().any(|p| p.name == name))
            .map(|(extension, _)| extension)
        {
            return self
                .extension_manager
                .get_prompt(extension, name, arguments, CancellationToken::default())
                .await
                .map_err(|e| anyhow!("Failed to get prompt: {}", e));
        }

        Err(anyhow!("Prompt '{}' not found", name))
    }

    pub async fn get_plan_prompt(&self) -> Result<String> {
        let tools = self.extension_manager.get_prefixed_tools(None).await?;
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

        let plan_prompt = self.extension_manager.get_planning_prompt(tools_info).await;

        Ok(plan_prompt)
    }

    pub async fn handle_tool_result(&self, id: String, result: ToolResult<Vec<Content>>) {
        if let Err(e) = self.tool_result_tx.send((id, result)).await {
            error!("Failed to send tool result: {}", e);
        }
    }

    pub async fn create_recipe(&self, mut messages: Conversation) -> Result<Recipe> {
        tracing::info!("Starting recipe creation with {} messages", messages.len());

        let extensions_info = self.extension_manager.get_extensions_info().await;
        tracing::debug!("Retrieved {} extensions info", extensions_info.len());
        let (extension_count, tool_count) =
            self.extension_manager.get_extension_and_tool_counts().await;

        // Get model name from provider
        let provider = self.provider().await.map_err(|e| {
            tracing::error!("Failed to get provider for recipe creation: {}", e);
            e
        })?;
        let model_config = provider.get_model_config();
        let model_name = &model_config.model_name;
        tracing::debug!("Using model: {}", model_name);

        let prompt_manager = self.prompt_manager.lock().await;
        let system_prompt = prompt_manager
            .builder(model_name)
            .with_extensions(extensions_info.into_iter())
            .with_frontend_instructions(self.frontend_instructions.lock().await.clone())
            .with_extension_and_tool_counts(extension_count, tool_count)
            .build();

        let recipe_prompt = prompt_manager.get_recipe_prompt().await;
        let tools = self
            .extension_manager
            .get_prefixed_tools(None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get tools for recipe creation: {}", e);
                e
            })?;

        messages.push(Message::user().with_text(recipe_prompt));

        let (messages, issues) = fix_conversation(messages);
        if !issues.is_empty() {
            issues
                .iter()
                .for_each(|issue| tracing::warn!(recipe.conversation.issue = issue));
        }

        tracing::debug!(
            "Added recipe prompt to messages, total messages: {}",
            messages.len()
        );

        tracing::info!("Calling provider to generate recipe content");
        let (result, _usage) = self
            .provider
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| {
                let error = anyhow!("Provider not available during recipe creation");
                tracing::error!("{}", error);
                error
            })?
            .complete(&system_prompt, messages.messages(), &tools)
            .await
            .map_err(|e| {
                tracing::error!("Provider completion failed during recipe creation: {}", e);
                e
            })?;

        let content = result.as_concat_text();
        tracing::debug!(
            "Provider returned content with {} characters",
            content.len()
        );

        // the response may be contained in ```json ```, strip that before parsing json
        let re = Regex::new(r"(?s)```[^\n]*\n(.*?)\n```").unwrap();
        let clean_content = re
            .captures(&content)
            .and_then(|caps| caps.get(1).map(|m| m.as_str()))
            .unwrap_or(&content)
            .trim()
            .to_string();

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
                tracing::warn!("Failed to parse JSON, falling back to string parsing");
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

        let extension_configs = get_enabled_extensions();

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
            .get_goose_provider()
            .expect("No provider configured. Run 'goose configure' first");

        let settings = Settings {
            goose_provider: Some(provider_name.clone()),
            goose_model: Some(model_name.clone()),
            temperature: Some(model_config.temperature.unwrap_or(0.0)),
        };

        tracing::debug!(
            "Building recipe with {} activities and {} extensions",
            activities.len(),
            extension_configs.len()
        );

        let (title, description) =
            if let Ok(json_content) = serde_json::from_str::<Value>(&clean_content) {
                let title = json_content
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("Custom recipe from chat")
                    .to_string();

                let description = json_content
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("a custom recipe instance from this chat session")
                    .to_string();

                (title, description)
            } else {
                (
                    "Custom recipe from chat".to_string(),
                    "a custom recipe instance from this chat session".to_string(),
                )
            };

        let recipe = Recipe::builder()
            .title(title)
            .description(description)
            .instructions(instructions)
            .activities(activities)
            .extensions(extension_configs)
            .settings(settings)
            .author(author)
            .build()
            .map_err(|e| {
                tracing::error!("Failed to build recipe: {}", e);
                anyhow!("Recipe build failed: {}", e)
            })?;

        tracing::info!("Recipe creation completed successfully");
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
        let system_prompt = prompt_manager.builder("gpt-4o").build();

        let final_output_tool_ref = agent.final_output_tool.lock().await;
        let final_output_tool_system_prompt =
            final_output_tool_ref.as_ref().unwrap().system_prompt();
        assert!(system_prompt.contains(&final_output_tool_system_prompt));
        Ok(())
    }

    #[tokio::test]
    async fn test_tool_inspection_manager_has_all_inspectors() -> Result<()> {
        let agent = Agent::new();

        // Verify that the tool inspection manager has all expected inspectors
        let inspector_names = agent.tool_inspection_manager.inspector_names();

        assert!(
            inspector_names.contains(&"repetition"),
            "Tool inspection manager should contain repetition inspector"
        );
        assert!(
            inspector_names.contains(&"permission"),
            "Tool inspection manager should contain permission inspector"
        );
        assert!(
            inspector_names.contains(&"security"),
            "Tool inspection manager should contain security inspector"
        );

        Ok(())
    }
}
