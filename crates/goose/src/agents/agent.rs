use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::stream::BoxStream;
use futures::{stream, FutureExt, Stream, StreamExt};

use super::final_output_tool::FinalOutputTool;
use super::platform_tools;
use super::reply_loop::{ContextLengthResult, LoopExitResult, ToolProcessingContext};
use super::tool_execution::{ToolCallResult, DECLINED_RESPONSE};
use crate::action_required_manager::ActionRequiredManager;
use crate::agents::extension::{ExtensionConfig, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::final_output_tool::{FINAL_OUTPUT_CONTINUATION_MESSAGE, FINAL_OUTPUT_TOOL_NAME};
use crate::agents::platform_tools::PLATFORM_MANAGE_SCHEDULE_TOOL_NAME;
use crate::agents::prompt_manager::PromptManager;
use crate::agents::retry::{RetryManager, RetryResult};
use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::subagent_tool::{
    create_subagent_tool, handle_subagent_tool, SUBAGENT_TOOL_NAME,
};
use crate::agents::types::SessionConfig;
use crate::agents::types::{FrontendTool, SharedProvider, ToolResultReceiver};
use crate::config::{get_enabled_extensions, Config, GooseMode};
use crate::context_mgmt::{
    check_if_compaction_needed, compact_messages, DEFAULT_COMPACTION_THRESHOLD,
};
use crate::conversation::message::{
    ActionRequiredData, Message, MessageContent, SystemNotificationType, ToolRequest,
};
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
use crate::session::extension_data::{EnabledExtensionsState, ExtensionState};
use crate::session::{Session, SessionManager, SessionType};
use crate::tool_inspection::ToolInspectionManager;
use crate::tool_monitor::RepetitionInspector;
use crate::utils::is_token_cancelled;
use regex::Regex;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, ErrorCode, ErrorData, GetPromptResult, Prompt,
    ServerNotification, Tool,
};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

const DEFAULT_MAX_TURNS: u32 = 1000;
const COMPACTION_THINKING_TEXT: &str = "goose is compacting the conversation...";

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
    pub(super) sub_recipes: Mutex<HashMap<String, SubRecipe>>,
    pub(super) final_output_tool: Arc<Mutex<Option<FinalOutputTool>>>,
    pub(super) frontend_tools: Mutex<HashMap<String, FrontendTool>>,
    pub(super) frontend_instructions: Mutex<Option<String>>,
    pub(super) prompt_manager: Mutex<PromptManager>,
    pub(super) confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    pub(super) confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    pub(super) tool_result_tx: mpsc::Sender<(String, ToolResult<CallToolResult>)>,
    pub(super) tool_result_rx: ToolResultReceiver,

    pub(super) scheduler_service: Mutex<Option<Arc<dyn SchedulerTrait>>>,
    pub(super) retry_manager: RetryManager,
    pub(super) tool_inspection_manager: ToolInspectionManager,
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

pub type ToolStream =
    Pin<Box<dyn Stream<Item = ToolStreamItem<ToolResult<CallToolResult>>> + Send>>;

// tool_stream combines a stream of ServerNotifications with a future representing the
// final result of the tool call. MCP notifications are not request-scoped, but
// this lets us capture all notifications emitted during the tool call for
// simpler consumption
pub fn tool_stream<S, F>(rx: S, done: F) -> ToolStream
where
    S: Stream<Item = ServerNotification> + Send + Unpin + 'static,
    F: Future<Output = ToolResult<CallToolResult>> + Send + 'static,
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
            sub_recipes: Mutex::new(HashMap::new()),
            final_output_tool: Arc::new(Mutex::new(None)),
            frontend_tools: Mutex::new(HashMap::new()),
            frontend_instructions: Mutex::new(None),
            prompt_manager: Mutex::new(PromptManager::new()),
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
            scheduler_service: Mutex::new(None),
            retry_manager: RetryManager::new(),
            tool_inspection_manager: Self::create_default_tool_inspection_manager(),
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
    pub(super) async fn drain_elicitation_messages(session_id: &str) -> Vec<Message> {
        let mut messages = Vec::new();
        let mut elicitation_rx = ActionRequiredManager::global().request_rx.lock().await;
        while let Ok(elicitation_message) = elicitation_rx.try_recv() {
            if let Err(e) = SessionManager::add_message(session_id, &elicitation_message).await {
                warn!("Failed to save elicitation message to session: {}", e);
            }
            messages.push(elicitation_message);
        }
        messages
    }

    async fn prepare_reply_context(
        &self,
        unfixed_conversation: Conversation,
        working_dir: &std::path::Path,
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

        let (tools, toolshim_tools, system_prompt) =
            self.prepare_tools_and_prompt(working_dir).await?;
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

    pub(super) async fn categorize_tools(
        &self,
        response: &Message,
        tools: &[rmcp::model::Tool],
    ) -> ToolCategorizeResult {
        // Categorize tool requests
        let (frontend_requests, remaining_requests, filtered_response) =
            self.categorize_tool_requests(response, tools).await;

        ToolCategorizeResult {
            frontend_requests,
            remaining_requests,
            filtered_response,
        }
    }

    pub(super) async fn handle_approved_and_denied_tools(
        &self,
        permission_check_result: &PermissionCheckResult,
        request_to_response_map: &HashMap<String, Arc<Mutex<Message>>>,
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

        Self::handle_denied_tools(permission_check_result, request_to_response_map).await;
        Ok(tool_futures)
    }

    async fn handle_denied_tools(
        permission_check_result: &PermissionCheckResult,
        request_to_response_map: &HashMap<String, Arc<Mutex<Message>>>,
    ) {
        for request in &permission_check_result.denied {
            if let Some(response_msg) = request_to_response_map.get(&request.id) {
                let mut response = response_msg.lock().await;
                *response = response.clone().with_tool_response_with_metadata(
                    request.id.clone(),
                    Ok(CallToolResult {
                        content: vec![rmcp::model::Content::text(DECLINED_RESPONSE)],
                        structured_content: None,
                        is_error: Some(true),
                        meta: None,
                    }),
                    request.metadata.as_ref(),
                );
            }
        }
    }

    pub async fn set_scheduler(&self, scheduler: Arc<dyn SchedulerTrait>) {
        let mut scheduler_service = self.scheduler_service.lock().await;
        *scheduler_service = Some(scheduler);
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

    pub async fn add_sub_recipes(&self, sub_recipes_to_add: Vec<SubRecipe>) {
        let mut sub_recipes = self.sub_recipes.lock().await;
        for sr in sub_recipes_to_add {
            sub_recipes.insert(sr.name.clone(), sr);
        }
    }

    pub async fn apply_recipe_components(
        &self,
        sub_recipes: Option<Vec<SubRecipe>>,
        response: Option<Response>,
        include_final_output: bool,
    ) {
        if let Some(sub_recipes) = sub_recipes {
            self.add_sub_recipes(sub_recipes).await;
        }

        if include_final_output {
            if let Some(response) = response {
                self.add_final_output_tool(response).await;
            }
        }
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
        // Prevent subagents from creating other subagents
        if session.session_type == SessionType::SubAgent && tool_call.name == SUBAGENT_TOOL_NAME {
            return (
                request_id,
                Err(ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Subagents cannot create other subagents".to_string(),
                    None,
                )),
            );
        }

        if tool_call.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME {
            let arguments = tool_call
                .arguments
                .map(Value::Object)
                .unwrap_or(Value::Object(serde_json::Map::new()));
            let result = self
                .handle_schedule_management(arguments, request_id.clone())
                .await;
            let wrapped_result = result.map(|content| CallToolResult {
                content,
                structured_content: None,
                is_error: Some(false),
                meta: None,
            });
            return (request_id, Ok(ToolCallResult::from(wrapped_result)));
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
        let result: ToolCallResult = if tool_call.name == SUBAGENT_TOOL_NAME {
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

            let extensions = self.get_extension_configs().await;
            let task_config =
                TaskConfig::new(provider, &session.id, &session.working_dir, extensions);
            let sub_recipes = self.sub_recipes.lock().await.clone();

            let arguments = tool_call
                .arguments
                .clone()
                .map(Value::Object)
                .unwrap_or(Value::Object(serde_json::Map::new()));

            handle_subagent_tool(
                arguments,
                task_config,
                sub_recipes,
                session.working_dir.clone(),
                cancellation_token,
            )
        } else if self.is_frontend_tool(&tool_call.name).await {
            // For frontend tools, return an error indicating we need frontend execution
            ToolCallResult::from(Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Frontend tool execution required".to_string(),
                None,
            )))
        } else {
            // Clone the result to ensure no references to extension_manager are returned
            let result = self
                .extension_manager
                .dispatch_tool_call(tool_call.clone(), cancellation_token.unwrap_or_default())
                .await;
            result.unwrap_or_else(|e| {
                crate::posthog::emit_error(
                    "tool_execution_failed",
                    &format!("{}: {}", tool_call.name, e),
                );
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

        Ok(())
    }

    pub async fn subagents_enabled(&self) -> bool {
        let config = crate::config::Config::global();
        let is_autonomous = config.get_goose_mode().unwrap_or(GooseMode::Auto) == GooseMode::Auto;
        if !is_autonomous {
            return false;
        }
        if self
            .provider()
            .await
            .map(|provider| provider.get_active_model_name().starts_with("gemini"))
            .unwrap_or(false)
        {
            return false;
        }
        if let Some(ref session_id) = self.extension_manager.get_context().await.session_id {
            if matches!(
                SessionManager::get_session(session_id, false)
                    .await
                    .ok()
                    .map(|session| session.session_type),
                Some(SessionType::SubAgent)
            ) {
                return false;
            }
        }
        !self
            .extension_manager
            .list_extensions()
            .await
            .map(|ext| ext.is_empty())
            .unwrap_or(true)
    }

    pub async fn list_tools(&self, extension_name: Option<String>) -> Vec<Tool> {
        let mut prefixed_tools = self
            .extension_manager
            .get_prefixed_tools(extension_name.clone())
            .await
            .unwrap_or_default();

        let subagents_enabled = self.subagents_enabled().await;
        if extension_name.is_none() || extension_name.as_deref() == Some("platform") {
            prefixed_tools.push(platform_tools::manage_schedule_tool());
        }

        if extension_name.is_none() {
            if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                prefixed_tools.push(final_output_tool.tool());
            }

            if subagents_enabled {
                let sub_recipes = self.sub_recipes.lock().await;
                let sub_recipes_vec: Vec<_> = sub_recipes.values().cloned().collect();
                prefixed_tools.push(create_subagent_tool(&sub_recipes_vec));
            }
        }

        prefixed_tools
    }

    pub async fn remove_extension(&self, name: &str) -> Result<()> {
        self.extension_manager.remove_extension(name).await?;
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
        for content in &user_message.content {
            if let MessageContent::ActionRequired(action_required) = content {
                if let ActionRequiredData::ElicitationResponse { id, user_data } =
                    &action_required.data
                {
                    if let Err(e) = ActionRequiredManager::global()
                        .submit_response(id.clone(), user_data.clone())
                        .await
                    {
                        let error_text = format!("Failed to submit elicitation response: {}", e);
                        error!(error_text);
                        return Ok(Box::pin(stream::once(async {
                            Ok(AgentEvent::Message(
                                Message::assistant().with_text(error_text),
                            ))
                        })));
                    }
                    SessionManager::add_message(&session_config.id, &user_message).await?;
                    return Ok(Box::pin(futures::stream::empty()));
                }
            }
        }

        let message_text = user_message.as_concat_text();

        // Track custom slash command usage (don't track command name for privacy)
        if message_text.trim().starts_with('/') {
            let command = message_text.split_whitespace().next();
            if let Some(cmd) = command {
                if crate::slash_commands::get_recipe_for_command(cmd).is_some() {
                    crate::posthog::emit_custom_slash_command_used();
                }
            }
        }

        let command_result = self
            .execute_command(&message_text, &session_config.id)
            .await;

        match command_result {
            Err(e) => {
                let error_message = Message::assistant()
                    .with_text(e.to_string())
                    .with_visibility(true, false);
                return Ok(Box::pin(stream::once(async move {
                    Ok(AgentEvent::Message(error_message))
                })));
            }
            Ok(Some(response)) if response.role == rmcp::model::Role::Assistant => {
                SessionManager::add_message(
                    &session_config.id,
                    &user_message.clone().with_visibility(true, false),
                )
                .await?;
                SessionManager::add_message(
                    &session_config.id,
                    &response.clone().with_visibility(true, false),
                )
                .await?;

                // Check if this was a command that modifies conversation history
                let modifies_history = crate::agents::execute_commands::COMPACT_TRIGGERS
                    .contains(&message_text.trim())
                    || message_text.trim() == "/clear";

                return Ok(Box::pin(async_stream::try_stream! {
                    yield AgentEvent::Message(user_message);
                    yield AgentEvent::Message(response);

                    // After commands that modify history, notify UI that history was replaced
                    if modifies_history {
                        let updated_session = SessionManager::get_session(&session_config.id, true)
                            .await
                            .map_err(|e| anyhow!("Failed to fetch updated session: {}", e))?;
                        let updated_conversation = updated_session
                            .conversation
                            .ok_or_else(|| anyhow!("Session has no conversation after history modification"))?;
                        yield AgentEvent::HistoryReplaced(updated_conversation);
                    }
                }));
            }
            Ok(Some(resolved_message)) => {
                SessionManager::add_message(
                    &session_config.id,
                    &user_message.clone().with_visibility(true, false),
                )
                .await?;
                SessionManager::add_message(
                    &session_config.id,
                    &resolved_message.clone().with_visibility(false, true),
                )
                .await?;
            }
            Ok(None) => {
                SessionManager::add_message(&session_config.id, &user_message).await?;
            }
        }
        let session = SessionManager::get_session(&session_config.id, true).await?;
        let conversation = session
            .conversation
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Session {} has no conversation", session_config.id))?;

        let needs_auto_compact = check_if_compaction_needed(
            self.provider().await?.as_ref(),
            &conversation,
            None,
            &session,
        )
        .await?;

        let conversation_to_compact = conversation.clone();

        Ok(Box::pin(async_stream::try_stream! {
            let final_conversation = if !needs_auto_compact {
                conversation
            } else {
                let config = Config::global();
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

                yield AgentEvent::Message(
                    Message::assistant().with_system_notification(
                        SystemNotificationType::ThinkingMessage,
                        COMPACTION_THINKING_TEXT,
                    )
                );

                match compact_messages(self.provider().await?.as_ref(), &conversation_to_compact, false).await {
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

            let mut reply_stream = self.reply_internal(final_conversation, session_config, session, cancel_token).await?;
            while let Some(event) = reply_stream.next().await {
                yield event?;
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
        let context = self
            .prepare_reply_context(conversation, &session.working_dir)
            .await?;
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
        let working_dir = session.working_dir.clone();
        tokio::spawn(async move {
            if let Err(e) = SessionManager::maybe_update_name(&session_id, provider).await {
                warn!("Failed to generate session description: {}", e);
            }
        });

        Ok(Box::pin(async_stream::try_stream! {
            let _ = reply_span.enter();
            let mut turns_taken = 0u32;
            let max_turns = session_config.max_turns.unwrap_or(DEFAULT_MAX_TURNS);
            let mut compaction_attempts = 0;

            loop {
                if is_token_cancelled(&cancel_token) {
                    break;
                }

                // Check for final output completion
                if let Some(event) = self.check_final_output_complete().await {
                    yield event;
                    break;
                }

                turns_taken += 1;
                if turns_taken > max_turns {
                    yield AgentEvent::Message(Message::assistant().with_text(
                        "I've reached the maximum number of actions I can do without user input. Would you like me to continue?"
                    ));
                    break;
                }

                let conversation_with_moim = super::moim::inject_moim(
                    conversation.clone(),
                    &self.extension_manager,
                ).await;

                let mut stream = Self::stream_response_from_provider(
                    self.provider().await?,
                    &system_prompt,
                    conversation_with_moim.messages(),
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
                            compaction_attempts = 0;

                            // Emit model change event if provider is lead-worker
                            if let Some(event) = self.emit_model_change_event(&usage).await? {
                                yield event;
                            }

                            if let Some(ref usage) = usage {
                                Self::update_session_metrics(&session_config, usage, false).await?;
                            }

                            if let Some(response) = response {
                                let tool_ctx = ToolProcessingContext {
                                    tools: &tools,
                                    goose_mode,
                                    conversation: &conversation,
                                    session_config: &session_config,
                                    session: &session,
                                    cancel_token: &cancel_token,
                                };
                                let result = self.process_response_with_tools(
                                    &response,
                                    &tool_ctx,
                                ).await?;

                                // Yield the filtered response message
                                yield AgentEvent::Message(result.filtered_response.clone());
                                tokio::task::yield_now().await;

                                if result.tool_requests.is_empty() {
                                    messages_to_add.push(response.clone());
                                    continue;
                                }

                                // Yield all collected events from tool processing
                                for event in result.events {
                                    yield event;
                                }

                                tools_updated = result.tools_updated;

                                // Preserve thinking content
                                if let Some(thinking_msg) = Self::extract_thinking_content(&response) {
                                    messages_to_add.push(thinking_msg);
                                }

                                // Build and yield tool response messages
                                messages_to_add.extend(result.messages_to_add);
                                for response_msg in result.response_messages {
                                    yield AgentEvent::Message(response_msg);
                                }

                                no_tools_called = false;
                            }
                        }
                        Err(ref provider_err @ ProviderError::ContextLengthExceeded(_)) => {
                            match self.handle_context_length_exceeded(
                                provider_err,
                                &mut compaction_attempts,
                                &conversation,
                                &session_config,
                            ).await? {
                                ContextLengthResult::Compacted { events, new_conversation } => {
                                    for event in events {
                                        yield event;
                                    }
                                    conversation = new_conversation;
                                    did_recovery_compact_this_iteration = true;
                                    break;
                                }
                                ContextLengthResult::MaxAttemptsReached(event) => {
                                    yield event;
                                    break;
                                }
                                ContextLengthResult::Failed => {
                                    break;
                                }
                            }
                        }
                        Err(ref provider_err) => {
                            crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());
                            error!("Error: {}", provider_err);
                            yield AgentEvent::Message(Message::assistant().with_text(
                                format!("Ran into this error: {provider_err}.\n\nPlease retry if you think this is a transient or recoverable error.")
                            ));
                            break;
                        }
                    }
                }

                if tools_updated {
                    (tools, toolshim_tools, system_prompt) =
                        self.prepare_tools_and_prompt(&working_dir).await?;
                }

                let exit_result = self.handle_loop_exit(
                    no_tools_called,
                    did_recovery_compact_this_iteration,
                    &mut conversation,
                    &session_config,
                    &initial_messages,
                    &mut messages_to_add,
                ).await?;

                for event in exit_result.events {
                    yield event;
                }

                for msg in &messages_to_add {
                    SessionManager::add_message(&session_config.id, msg).await?;
                }
                conversation.extend(messages_to_add);

                if exit_result.should_exit {
                    break;
                }

                tokio::task::yield_now().await;
            }
        }))
    }

    async fn check_final_output_complete(&self) -> Option<AgentEvent> {
        if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
            if let Some(ref output) = final_output_tool.final_output {
                return Some(AgentEvent::Message(
                    Message::assistant().with_text(output.clone()),
                ));
            }
        }
        None
    }

    async fn emit_model_change_event(
        &self,
        usage: &Option<crate::providers::base::ProviderUsage>,
    ) -> Result<Option<AgentEvent>> {
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
                return Ok(Some(AgentEvent::ModelChange {
                    model: active_model,
                    mode: mode.to_string(),
                }));
            }
        }
        Ok(None)
    }

    async fn handle_context_length_exceeded(
        &self,
        provider_err: &ProviderError,
        compaction_attempts: &mut u32,
        conversation: &Conversation,
        session_config: &SessionConfig,
    ) -> Result<ContextLengthResult> {
        crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());
        *compaction_attempts += 1;

        if *compaction_attempts >= 2 {
            error!("Context limit exceeded after compaction - prompt too large");
            return Ok(ContextLengthResult::MaxAttemptsReached(AgentEvent::Message(
                Message::assistant().with_system_notification(
                    SystemNotificationType::InlineMessage,
                    "Unable to continue: Context limit still exceeded after compaction. Try using a shorter message, a model with a larger context window, or start a new session."
                )
            )));
        }

        let mut events = vec![
            AgentEvent::Message(Message::assistant().with_system_notification(
                SystemNotificationType::InlineMessage,
                "Context limit reached. Compacting to continue conversation...",
            )),
            AgentEvent::Message(Message::assistant().with_system_notification(
                SystemNotificationType::ThinkingMessage,
                COMPACTION_THINKING_TEXT,
            )),
        ];

        match compact_messages(self.provider().await?.as_ref(), conversation, false).await {
            Ok((compacted_conversation, usage)) => {
                SessionManager::replace_conversation(&session_config.id, &compacted_conversation)
                    .await?;
                Self::update_session_metrics(session_config, &usage, true).await?;
                events.push(AgentEvent::HistoryReplaced(compacted_conversation.clone()));
                Ok(ContextLengthResult::Compacted {
                    events,
                    new_conversation: compacted_conversation,
                })
            }
            Err(e) => {
                crate::posthog::emit_error("compaction_failed", &e.to_string());
                error!("Compaction failed: {}", e);
                Ok(ContextLengthResult::Failed)
            }
        }
    }

    async fn handle_loop_exit(
        &self,
        no_tools_called: bool,
        did_recovery_compact: bool,
        conversation: &mut Conversation,
        session_config: &SessionConfig,
        initial_messages: &[Message],
        messages_to_add: &mut Conversation,
    ) -> Result<LoopExitResult> {
        let mut events = Vec::new();
        let mut should_exit = false;

        if no_tools_called {
            if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
                if final_output_tool.final_output.is_none() {
                    warn!("Final output tool has not been called yet. Continuing agent loop.");
                    let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
                    messages_to_add.push(message.clone());
                    events.push(AgentEvent::Message(message));
                } else {
                    let message = Message::assistant()
                        .with_text(final_output_tool.final_output.clone().unwrap());
                    messages_to_add.push(message.clone());
                    events.push(AgentEvent::Message(message));
                    should_exit = true;
                }
            } else if did_recovery_compact {
                // Continue from last user message after compaction
            } else {
                match self
                    .handle_retry_logic(conversation, session_config, initial_messages)
                    .await
                {
                    Ok(should_retry) => {
                        if should_retry {
                            info!("Retry logic triggered, restarting agent loop");
                        } else {
                            should_exit = true;
                        }
                    }
                    Err(e) => {
                        error!("Retry logic failed: {}", e);
                        events
                            .push(AgentEvent::Message(Message::assistant().with_text(
                                format!("Retry logic encountered an error: {}", e),
                            )));
                        should_exit = true;
                    }
                }
            }
        }

        Ok(LoopExitResult {
            should_exit,
            events,
        })
    }

    pub async fn extend_system_prompt(&self, instruction: String) {
        let mut prompt_manager = self.prompt_manager.lock().await;
        prompt_manager.add_system_prompt_extra(instruction);
    }

    pub async fn update_provider(
        &self,
        provider: Arc<dyn Provider>,
        session_id: &str,
    ) -> Result<()> {
        let mut current_provider = self.provider.lock().await;
        *current_provider = Some(provider.clone());

        SessionManager::update_session(session_id)
            .provider_name(provider.get_name())
            .model_config(provider.get_model_config())
            .apply()
            .await
            .context("Failed to persist provider config to session")
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

    pub async fn handle_tool_result(&self, id: String, result: ToolResult<CallToolResult>) {
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
            .builder()
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
        let system_prompt = prompt_manager.builder().build();

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
