use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::stream::BoxStream;
use futures::{stream, FutureExt, Stream, StreamExt, TryStreamExt};
use tracing_futures::Instrument;
use uuid::Uuid;

use super::container::Container;
use super::final_output_tool::FinalOutputTool;
use super::mcp_client::GooseMcpHostInfo;
use super::platform_tools;
use super::tool_confirmation_router::ToolConfirmationRouter;
use super::tool_execution::{ToolCallResult, CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};
use crate::action_required_manager::ActionRequiredManager;
use crate::agents::extension::{ExtensionConfig, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{
    get_parameter_names, ExtensionManager, ExtensionManagerCapabilities,
};
use crate::agents::final_output_tool::{FINAL_OUTPUT_CONTINUATION_MESSAGE, FINAL_OUTPUT_TOOL_NAME};
use crate::agents::platform_extensions::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::agents::platform_tools::PLATFORM_MANAGE_SCHEDULE_TOOL_NAME;
use crate::agents::prompt_manager::PromptManager;
use crate::agents::retry::{RetryManager, RetryResult};
use crate::agents::types::{FrontendTool, SessionConfig, SharedProvider, ToolResultReceiver};
use crate::config::extensions::name_to_key;
use crate::config::permission::PermissionManager;
use crate::config::{get_enabled_extensions, Config, GooseMode};
use crate::context_mgmt::{
    check_if_compaction_needed, compact_messages, DEFAULT_COMPACTION_THRESHOLD,
};
use crate::conversation::message::{
    ActionRequiredData, InferenceMetadata, Message, MessageContent, ProviderMetadata,
    SystemNotificationType, ToolRequest,
};
use crate::conversation::{debug_conversation_fix, fix_conversation, Conversation};
use crate::mcp_utils::ToolResult;
use crate::permission::permission_inspector::PermissionInspector;
use crate::permission::permission_judge::PermissionCheckResult;
use crate::permission::PermissionConfirmation;
use crate::providers::base::{PermissionRouting, Provider};
use crate::providers::errors::ProviderError;
use crate::recipe::{Author, Recipe, Response, Settings};
use crate::scheduler_trait::SchedulerTrait;
use crate::security::adversary_inspector::AdversaryInspector;
use crate::security::egress_inspector::EgressInspector;
use crate::security::security_inspector::SecurityInspector;
use crate::session::extension_data::{EnabledExtensionsState, ExtensionState};
use crate::session::{Session, SessionManager, SessionNameUpdate};
use crate::tool_inspection::ToolInspectionManager;
use crate::tool_monitor::RepetitionInspector;
use crate::utils::is_token_cancelled;
use regex::Regex;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ErrorCode, ErrorData, GetPromptResult, Prompt,
    ServerNotification, Tool,
};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

const DEFAULT_MAX_TURNS: u32 = 1000;
const COMPACTION_THINKING_TEXT: &str = "goose is compacting the conversation...";
const DEFAULT_FRONTEND_INSTRUCTIONS: &str =
    "The following tools are provided directly by the frontend and will be executed by the frontend when called.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolCategory {
    Shell,
    Read,
    Write,
    Other,
}

fn categorize_tool(tool_name: &str) -> ToolCategory {
    let local = tool_name.rsplit("__").next().unwrap_or(tool_name);
    match local {
        "shell" | "bash" | "exec" | "run" => ToolCategory::Shell,
        "read" | "view" | "cat" | "read_file" => ToolCategory::Read,
        "write" | "edit" | "patch" | "write_file" | "edit_file" => ToolCategory::Write,
        _ => ToolCategory::Other,
    }
}

fn extract_string_arg(input: &Value, keys: &[&str]) -> Option<String> {
    let obj = input.as_object()?;
    for k in keys {
        if let Some(s) = obj.get(*k).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

mod extensions;
mod hooks;
mod provider;
mod reply;
mod types;

pub use types::*;

#[derive(Clone)]
pub struct AgentConfig {
    pub session_manager: Arc<SessionManager>,
    pub permission_manager: Arc<PermissionManager>,
    pub scheduler_service: Option<Arc<dyn SchedulerTrait>>,
    pub goose_mode: GooseMode,
    pub disable_session_naming: bool,
    pub goose_platform: GoosePlatform,
    pub mcp_host_info: Option<GooseMcpHostInfo>,
    pub session_name_update_tx: Option<mpsc::UnboundedSender<SessionNameUpdate>>,
    pub use_login_shell_path: Option<bool>,
}

impl AgentConfig {
    pub fn new(
        session_manager: Arc<SessionManager>,
        permission_manager: Arc<PermissionManager>,
        scheduler_service: Option<Arc<dyn SchedulerTrait>>,
        goose_mode: GooseMode,
        disable_session_naming: bool,
        goose_platform: GoosePlatform,
    ) -> Self {
        Self {
            session_manager,
            permission_manager,
            scheduler_service,
            goose_mode,
            disable_session_naming,
            goose_platform,
            mcp_host_info: None,
            session_name_update_tx: None,
            use_login_shell_path: None,
        }
    }

    pub fn with_mcp_host_info(mut self, mcp_host_info: Option<GooseMcpHostInfo>) -> Self {
        self.mcp_host_info = mcp_host_info;
        self
    }

    pub fn with_session_name_update_tx(
        mut self,
        tx: Option<mpsc::UnboundedSender<SessionNameUpdate>>,
    ) -> Self {
        self.session_name_update_tx = tx;
        self
    }

    pub fn with_use_login_shell_path(mut self, use_login_shell_path: bool) -> Self {
        self.use_login_shell_path = Some(use_login_shell_path);
        self
    }
}

/// The main goose Agent
pub struct Agent {
    pub(super) provider: SharedProvider,
    pub config: AgentConfig,
    pub(super) current_goose_mode: Mutex<GooseMode>,

    pub extension_manager: Arc<ExtensionManager>,
    pub(super) final_output_tool: Arc<Mutex<Option<FinalOutputTool>>>,
    pub(super) frontend_extensions: Mutex<HashMap<String, ExtensionConfig>>,
    pub(super) frontend_tools: Mutex<HashMap<String, FrontendTool>>,
    pub(super) frontend_instructions: Mutex<Option<String>>,
    pub(super) prompt_manager: Mutex<PromptManager>,
    pub tool_confirmation_router: ToolConfirmationRouter,
    pub(super) tool_result_tx: mpsc::Sender<(String, ToolResult<CallToolResult>)>,
    pub(super) tool_result_rx: ToolResultReceiver,

    pub(super) retry_manager: RetryManager,
    pub(super) tool_inspection_manager: ToolInspectionManager,
    pub(super) hook_manager: crate::hooks::HookManager,
    container: Mutex<Option<Container>>,
    goal: Mutex<Option<String>>,
    grind: Mutex<Option<String>>,
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent {
    pub fn new() -> Self {
        let config = Config::global();
        Self::with_config(AgentConfig::new(
            Arc::new(SessionManager::instance()),
            PermissionManager::instance(),
            None,
            config.get_goose_mode().unwrap_or_default(),
            config.get_goose_disable_session_naming().unwrap_or(false),
            GoosePlatform::GooseCli,
        ))
    }

    pub fn with_config(config: AgentConfig) -> Self {
        let (tool_tx, tool_rx) = mpsc::channel(32);
        let provider = Arc::new(Mutex::new(None));

        let goose_platform = config.goose_platform.clone();
        let initial_mode = config.goose_mode;
        let explicit_mcp_host_info = config.mcp_host_info.clone();
        let mcpui = explicit_mcp_host_info
            .as_ref()
            .filter(|host_info| host_info.explicit_extensions)
            .map(GooseMcpHostInfo::mcpui_enabled)
            .unwrap_or_else(|| match config.goose_platform {
                GoosePlatform::GooseDesktop => true,
                GoosePlatform::GooseCli => false,
            });
        let capabilities = ExtensionManagerCapabilities {
            mcpui,
            host_info: explicit_mcp_host_info.clone(),
        };
        let client_name = explicit_mcp_host_info
            .as_ref()
            .and_then(|host_info| host_info.client_name.clone())
            .unwrap_or_else(|| goose_platform.to_string());
        let session_manager = Arc::clone(&config.session_manager);
        let permission_manager = Arc::clone(&config.permission_manager);
        let use_login_shell_path = config
            .use_login_shell_path
            .unwrap_or(matches!(goose_platform, GoosePlatform::GooseDesktop));
        Self {
            provider: provider.clone(),
            config,
            current_goose_mode: Mutex::new(initial_mode),
            extension_manager: Arc::new(ExtensionManager::new(
                provider.clone(),
                session_manager,
                client_name,
                capabilities,
                use_login_shell_path,
            )),
            final_output_tool: Arc::new(Mutex::new(None)),
            frontend_extensions: Mutex::new(HashMap::new()),
            frontend_tools: Mutex::new(HashMap::new()),
            frontend_instructions: Mutex::new(None),
            prompt_manager: Mutex::new(PromptManager::new()),
            tool_confirmation_router: ToolConfirmationRouter::new(),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
            retry_manager: RetryManager::new(),
            tool_inspection_manager: Self::create_tool_inspection_manager(
                permission_manager,
                provider.clone(),
            ),
            hook_manager: crate::hooks::HookManager::load(std::env::current_dir().ok().as_deref()),
            container: Mutex::new(None),
            goal: Mutex::new(None),
            grind: Mutex::new(None),
        }
    }

    /// Create a tool inspection manager with default inspectors
    fn create_tool_inspection_manager(
        permission_manager: Arc<PermissionManager>,
        provider: SharedProvider,
    ) -> ToolInspectionManager {
        let mut tool_inspection_manager = ToolInspectionManager::new();

        // Add security inspector (highest priority - runs first)
        tool_inspection_manager.add_inspector(Box::new(SecurityInspector::new()));
        tool_inspection_manager.add_inspector(Box::new(EgressInspector::new()));

        // Add adversary inspector (LLM-based review, enabled by ~/.config/goose/adversary.md)
        tool_inspection_manager.add_inspector(Box::new(AdversaryInspector::new(provider.clone())));

        // Add permission inspector (medium-high priority)
        tool_inspection_manager.add_inspector(Box::new(PermissionInspector::new(
            permission_manager,
            provider,
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

    async fn drain_elicitation_messages(&self, session_id: &str) -> Vec<Message> {
        let mut messages = Vec::new();
        let manager = self.config.session_manager.clone();
        let mut elicitation_rx = ActionRequiredManager::global().request_rx.lock().await;
        while let Ok(mut elicitation_message) = elicitation_rx.try_recv() {
            if elicitation_message.id.is_none() {
                elicitation_message = elicitation_message.with_generated_id();
            }
            if let Err(e) = manager.add_message(session_id, &elicitation_message).await {
                warn!("Failed to save elicitation message to session: {}", e);
            }
            messages.push(elicitation_message);
        }
        messages
    }

    async fn load_project_instructions(&self, session: &Session) -> Option<String> {
        let project_id = session.project_id.as_deref()?;
        let entry = crate::sources::read_project(project_id).ok()?;
        let mut parts = Vec::new();
        parts.push(format!("# Project: {}", entry.name));
        if !entry.description.is_empty() {
            parts.push(entry.description.clone());
        }
        if !entry.content.is_empty() {
            parts.push(entry.content.clone());
        }
        Some(parts.join("\n\n"))
    }

    /// Get a reference count clone to the provider
    pub async fn provider(&self) -> Result<Arc<dyn Provider>, anyhow::Error> {
        match &*self.provider.lock().await {
            Some(provider) => Ok(Arc::clone(provider)),
            None => Err(anyhow!("Provider not set")),
        }
    }

    /// When set, all stdio extensions will be started via `docker exec` in the specified container.
    pub async fn set_container(&self, container: Option<Container>) {
        *self.container.lock().await = container.clone();
    }

    pub async fn container(&self) -> Option<Container> {
        self.container.lock().await.clone()
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
        self.extend_system_prompt("final_output".to_string(), final_output_system_prompt)
            .await;
    }

    pub async fn apply_recipe_components(
        &self,
        response: Option<Response>,
        include_final_output: bool,
    ) {
        if include_final_output {
            if let Some(response) = response {
                self.add_final_output_tool(response).await;
            }
        }
    }

    /// Dispatch a single tool call to the appropriate client
    #[instrument(skip(self, tool_call, request_id, cancellation_token, session), fields(input, output, session.id = %session.id))]
    pub async fn dispatch_tool_call(
        &self,
        tool_call: CallToolRequestParams,
        request_id: String,
        cancellation_token: Option<CancellationToken>,
        session: &Session,
    ) -> (String, Result<ToolCallResult, ErrorData>) {
        let input_summary = serde_json::json!({
            "tool": tool_call.name,
            "arguments": tool_call.arguments,
        });
        tracing::Span::current().record("input", tracing::field::display(&input_summary));

        self.prompt_manager
            .lock()
            .await
            .record_tool_arguments(&tool_call.arguments, &session.working_dir);

        if self
            .hook_manager
            .has_hooks(crate::hooks::HookEvent::PreToolUse)
        {
            let ctx =
                crate::hooks::HookContext::new(crate::hooks::HookEvent::PreToolUse, &session.id)
                    .with_tool(
                        tool_call.name.to_string(),
                        tool_call
                            .arguments
                            .as_ref()
                            .map(|a| serde_json::Value::Object(a.clone())),
                    )
                    .with_working_dir(session.working_dir.to_string_lossy().to_string());
            if let crate::hooks::HookDecision::Deny { reason, plugin } = self
                .hook_manager
                .emit_blocking(crate::hooks::HookEvent::PreToolUse, ctx)
                .await
            {
                return (
                    request_id,
                    Err(ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!(
                            "Tool call denied by policy hook `{plugin}`: {reason}. \
                             Do not retry; this is a policy denial, not a transient failure."
                        ),
                        None,
                    )),
                );
            }
        }

        let tool_input_for_extended = tool_call
            .arguments
            .as_ref()
            .map(|a| serde_json::Value::Object(a.clone()));
        self.emit_pre_tool_extended_hooks(
            &tool_call.name,
            tool_input_for_extended.as_ref(),
            session,
        )
        .await;

        if tool_call.name == PLATFORM_MANAGE_SCHEDULE_TOOL_NAME {
            let arguments = tool_call
                .arguments
                .clone()
                .map(Value::Object)
                .unwrap_or(Value::Object(serde_json::Map::new()));
            let result = self
                .handle_schedule_management(arguments, request_id.clone())
                .await;
            let wrapped_result = result.map(CallToolResult::success);
            return (
                request_id,
                Ok(self.with_post_tool_hook(
                    ToolCallResult::from(wrapped_result),
                    &tool_call,
                    session,
                )),
            );
        }

        if tool_call.name == FINAL_OUTPUT_TOOL_NAME {
            return if let Some(final_output_tool) = self.final_output_tool.lock().await.as_mut() {
                let result = final_output_tool.execute_tool_call(tool_call.clone()).await;
                (
                    request_id,
                    Ok(self.with_post_tool_hook(result, &tool_call, session)),
                )
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

        let ctx = super::tool_execution::ToolCallContext::new(
            session.id.clone(),
            Some(session.working_dir.clone()),
            Some(request_id.clone()),
        );

        debug!("WAITING_TOOL_START: {}", tool_call.name);
        let result: ToolCallResult = if self.is_frontend_tool(&tool_call.name).await {
            ToolCallResult::from(Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Frontend tool execution required".to_string(),
                None,
            )))
        } else {
            let result = self
                .extension_manager
                .dispatch_tool_call(
                    &ctx,
                    tool_call.clone(),
                    cancellation_token.unwrap_or_default(),
                )
                .await;
            result.unwrap_or_else(|e| {
                #[cfg(feature = "telemetry")]
                crate::posthog::emit_error(
                    "tool_execution_failed",
                    &format!("{}: {}", tool_call.name, e),
                );
                let error_data = e.downcast::<ErrorData>().unwrap_or_else(|e| {
                    ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                });
                ToolCallResult::from(Err(error_data))
            })
        };

        debug!("WAITING_TOOL_END: {}", tool_call.name);

        (
            request_id,
            Ok(self.with_post_tool_hook(result, &tool_call, session)),
        )
    }

    /// Handle a confirmation response for a tool request
    pub async fn handle_confirmation(
        &self,
        request_id: String,
        confirmation: PermissionConfirmation,
    ) {
        let provider = self.provider.lock().await.clone();
        if let Some(provider) = provider.as_ref() {
            if provider.permission_routing() == PermissionRouting::ActionRequired
                && provider
                    .handle_permission_confirmation(&request_id, &confirmation)
                    .await
            {
                return;
            }
        }
        if !self
            .tool_confirmation_router
            .deliver(request_id, confirmation)
            .await
        {
            error!("Failed to deliver confirmation");
        }
    }

    pub async fn supports_action_required_permissions(&self) -> bool {
        if let Some(provider) = self.provider.lock().await.as_ref() {
            return provider.permission_routing() == PermissionRouting::ActionRequired;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::permission_confirmation::PrincipalType;
    use crate::providers::base::PermissionRouting;
    use crate::recipe::Response;

    struct ActionRequiredProvider {
        handled: tokio::sync::Mutex<Vec<(String, PermissionConfirmation)>>,
    }

    impl ActionRequiredProvider {
        fn new() -> Self {
            Self {
                handled: tokio::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl std::fmt::Debug for ActionRequiredProvider {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ActionRequiredProvider").finish()
        }
    }

    #[async_trait::async_trait]
    impl crate::providers::base::Provider for ActionRequiredProvider {
        fn get_name(&self) -> &str {
            "test-action-required"
        }
        fn get_model_config(&self) -> crate::model::ModelConfig {
            crate::model::ModelConfig::new("test").unwrap()
        }
        async fn stream(
            &self,
            _: &crate::model::ModelConfig,
            _: &str,
            _: &str,
            _: &[crate::conversation::message::Message],
            _: &[rmcp::model::Tool],
        ) -> Result<crate::providers::base::MessageStream, crate::providers::errors::ProviderError>
        {
            unimplemented!()
        }
        fn permission_routing(&self) -> PermissionRouting {
            PermissionRouting::ActionRequired
        }
        async fn handle_permission_confirmation(
            &self,
            request_id: &str,
            confirmation: &PermissionConfirmation,
        ) -> bool {
            self.handled
                .lock()
                .await
                .push((request_id.to_string(), confirmation.clone()));
            request_id == "known"
        }
    }

    #[tokio::test]
    async fn test_handle_confirmation_routes_to_provider() {
        let agent = Agent::new();
        let provider = Arc::new(ActionRequiredProvider::new());
        *agent.provider.lock().await =
            Some(provider.clone() as Arc<dyn crate::providers::base::Provider>);

        // Known request_id → provider handles it, confirmation_router NOT called
        agent
            .handle_confirmation(
                "known".to_string(),
                PermissionConfirmation {
                    principal_type: PrincipalType::Tool,
                    permission: crate::permission::Permission::AllowOnce,
                },
            )
            .await;
        assert_eq!(provider.handled.lock().await.len(), 1);

        // Unknown request_id → provider returns false, falls through to confirmation_router
        // Register first so deliver() has somewhere to send
        let rx = agent
            .tool_confirmation_router
            .register("unknown".to_string())
            .await;
        agent
            .handle_confirmation(
                "unknown".to_string(),
                PermissionConfirmation {
                    principal_type: PrincipalType::Tool,
                    permission: crate::permission::Permission::DenyOnce,
                },
            )
            .await;
        assert_eq!(provider.handled.lock().await.len(), 2);
        // Verify the fallthrough went to confirmation_router
        let conf = rx.await.unwrap();
        assert_eq!(conf.permission, crate::permission::Permission::DenyOnce);
    }

    #[tokio::test]
    async fn test_handle_confirmation_noop_provider() {
        let agent = Agent::new();
        // No provider set → Noop routing, goes straight to confirmation_router
        // Register first so deliver() has somewhere to send
        let rx = agent
            .tool_confirmation_router
            .register("any".to_string())
            .await;
        agent
            .handle_confirmation(
                "any".to_string(),
                PermissionConfirmation {
                    principal_type: PrincipalType::Tool,
                    permission: crate::permission::Permission::AllowOnce,
                },
            )
            .await;

        let conf = rx.await.unwrap();
        assert_eq!(conf.permission, crate::permission::Permission::AllowOnce);
    }

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

        let tools = agent.list_tools("test-session-id", None).await;
        let final_output_tool = tools
            .iter()
            .find(|tool| tool.name == FINAL_OUTPUT_TOOL_NAME);

        assert!(
            final_output_tool.is_some(),
            "Final output tool should be present after adding"
        );

        let prompt_manager = agent.prompt_manager.lock().await;
        let system_prompt = prompt_manager
            .builder()
            .with_goose_mode(GooseMode::default())
            .build();

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
        assert!(
            inspector_names.contains(&"adversary"),
            "Tool inspection manager should contain adversary inspector"
        );

        Ok(())
    }

    #[test]
    fn categorize_tool_recognizes_conventional_names() {
        assert_eq!(categorize_tool("developer__shell"), ToolCategory::Shell);
        assert_eq!(categorize_tool("filesystem__write"), ToolCategory::Write);
        assert_eq!(categorize_tool("filesystem__edit"), ToolCategory::Write);
        assert_eq!(categorize_tool("filesystem__read"), ToolCategory::Read);
        assert_eq!(categorize_tool("filesystem__view"), ToolCategory::Read);
        assert_eq!(categorize_tool("filesystem__cat"), ToolCategory::Read);
        assert_eq!(categorize_tool("scheduler__list"), ToolCategory::Other);
        assert_eq!(categorize_tool("shell"), ToolCategory::Shell);
    }

    #[test]
    fn extract_string_arg_picks_first_present_key() {
        let input = serde_json::json!({ "file_path": "/tmp/a.txt", "path": "/tmp/b.txt" });
        assert_eq!(
            extract_string_arg(&input, &["path", "file", "file_path"]).as_deref(),
            Some("/tmp/b.txt")
        );
        let input = serde_json::json!({ "file_path": "/tmp/a.txt" });
        assert_eq!(
            extract_string_arg(&input, &["path", "file", "file_path"]).as_deref(),
            Some("/tmp/a.txt")
        );
        let input = serde_json::json!({ "other": 1 });
        assert!(extract_string_arg(&input, &["path"]).is_none());
        let input = serde_json::json!({ "path": "" });
        assert!(extract_string_arg(&input, &["path"]).is_none());
    }
}
