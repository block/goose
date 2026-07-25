use crate::acp::custom_notifications::*;
use crate::acp::custom_requests::*;
use crate::acp::fs::AcpTools;
pub(super) use crate::acp::response_builder::{
    build_config_options, build_mode_state, build_model_state, build_provider_options,
    build_session_info, build_session_setup_config, send_session_setup_notifications, session_meta,
    session_provider_selection, session_response_meta, should_refresh_inventory_for_session_init,
};
use crate::acp::tool_call_notifier::ToolCallNotifier;
use crate::acp::{PermissionDecision, ACP_CURRENT_MODEL};
use crate::agents::extension::{Envs, PLATFORM_EXTENSIONS};
use crate::agents::mcp_client::{GooseMcpHostInfo, McpClientTrait};
use crate::agents::platform_extensions::developer::DeveloperClient;
use crate::agents::{
    Agent, AgentConfig, ExtensionConfig, ExtensionLoadResult, GoosePlatform, SessionConfig,
};
use crate::config::base::CONFIG_YAML_NAME;
use crate::config::extensions::get_enabled_extensions_with_config;
use crate::config::paths::Paths;
use crate::config::permission::PermissionManager;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{
    ActionRequiredData, Message, MessageContent, SystemNotificationContent, SystemNotificationType,
    ToolRequest, ToolResponse,
};
use crate::execution::manager::{AgentManager, AgentManagerGetResult, RuntimeContext};
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::Provider;
use crate::providers::inventory::{
    ProviderInventoryEntry, ProviderInventoryService, RefreshJobPlan, RefreshPlan,
    RefreshSkipReason,
};
use crate::scheduler_trait::SchedulerTrait;
use crate::session::session_manager::SessionUsageTotals;
use crate::session::{
    EnabledExtensionsState, ExtensionData, ExtensionState, Session, SessionManager, SessionType,
};
use crate::source_roots::SourceRoot;
use crate::utils::sanitize_unicode_tags;
use agent_client_protocol::schema::v1::{
    AgentCapabilities, Annotations, AuthMethod, AuthMethodAgent, AuthenticateRequest,
    AuthenticateResponse, CancelNotification, CloseSessionRequest, CloseSessionResponse,
    ConfigOptionUpdate, ContentBlock, ContentChunk, Cost, CurrentModeUpdate,
    EmbeddedResourceResource, FileSystemCapabilities, ForkSessionRequest, ForkSessionResponse,
    ImageContent, Implementation, InitializeRequest, InitializeResponse, ListSessionsRequest,
    ListSessionsResponse, LoadSessionRequest, LoadSessionResponse, McpCapabilities, McpServer,
    Meta, NewSessionRequest, NewSessionResponse, PermissionOption, PermissionOptionKind,
    PromptCapabilities, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, ResourceLink, SessionCapabilities, SessionCloseCapabilities,
    SessionConfigOption, SessionId, SessionInfoUpdate, SessionListCapabilities,
    SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TextContent, ToolCallId, ToolCallUpdate, Usage, UsageUpdate,
};
use agent_client_protocol::util::MatchDispatchFrom;
use agent_client_protocol::{
    Agent as SacpAgent, ByteStreams, Client, ConnectionTo, Dispatch, HandleDispatchFrom, Handled,
    Responder,
};
use anyhow::Result;
use fs_err as fs;
use futures::future::{BoxFuture, FutureExt};
use futures::stream::{self, StreamExt};
use rmcp::model::{AnnotateAble, RawTextContent, Role};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;
use uuid::Uuid;

use self::tool_calls::chain::{breaks_consecutive_tool_calls, ReadyToolChain, ToolChainTracker};
use self::tool_calls::conversion::{
    build_initial_tool_call, build_permission_tool_call_update,
    tool_call_update_fields_from_response, trusted_update_meta,
};
use self::tool_calls::enrichment::{spawn_chain_summary_enrichment, spawn_tool_title_enrichment};

mod agent_requests;
pub use agent_requests::agent_request_schemas;
mod agent_mentions;
mod apps;
mod config;
mod custom_dispatch;
mod diagnostics;
mod dictation;
mod dispatch;
mod elicitation;
mod extensions;
mod fork_session;
mod list_sessions;
mod load_session;
mod local_inference;
mod manage_sessions;
mod new_session;
mod onboarding;
mod prompts;
mod providers;
mod recipe;
mod resources;
mod schedule;
mod slash_commands;
mod sources;
mod tool_calls;
mod tool_notifications;
mod tools;

pub type AcpProviderFactory = Arc<
    dyn Fn(
            String,
            Vec<ExtensionConfig>,
            Option<PathBuf>,
        ) -> BoxFuture<'static, Result<Arc<dyn Provider>>>
        + Send
        + Sync,
>;

/// Convenience conversions from any `Display` error into an `agent_client_protocol::Error`.
///
/// Replaces the repetitive `.internal_err()`
/// pattern. Use `.internal_err()?` for server-side failures and `.invalid_params_err()?`
/// for bad client input. For custom messages use `.internal_err_ctx("context")?`.
#[allow(dead_code)]
trait ResultExt<T> {
    fn internal_err(self) -> Result<T, agent_client_protocol::Error>;
    fn invalid_params_err(self) -> Result<T, agent_client_protocol::Error>;
    fn internal_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error>;
    fn invalid_params_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn internal_err(self) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| agent_client_protocol::Error::internal_error().data(e.to_string()))
    }
    fn invalid_params_err(self) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| agent_client_protocol::Error::invalid_params().data(e.to_string()))
    }
    fn internal_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| {
            agent_client_protocol::Error::internal_error().data(format!("{context}: {e}"))
        })
    }
    fn invalid_params_err_ctx(self, context: &str) -> Result<T, agent_client_protocol::Error> {
        self.map_err(|e| {
            agent_client_protocol::Error::invalid_params().data(format!("{context}: {e}"))
        })
    }
}

pub(super) const DEFAULT_PROVIDER_ID: &str = "goose";
pub(super) const DEFAULT_PROVIDER_LABEL: &str = "Goose (Default)";
const PROVIDER_CONFIG_STATUS_CHECK_CONCURRENCY: usize = 16;

/// In-memory state for an active ACP session.
///
/// ## Terminology (temporary, until all clients migrate to ACP)
///
/// The ACP protocol uses "session" to mean the conversation as the human sees it —
/// a durable, append-only exchange of messages. Internally, goose also has a concept
/// called "Session" (the `sessions` DB table) which represents the agent's working
/// state: the message list the LLM sees, compaction state, provider binding, etc.
///
/// The ACP session ID maps directly to a `sessions` row. The `sessions` HashMap
/// below is keyed by session ID.
struct GooseAcpSession {
    agent: Arc<Agent>,
}

struct ActivePromptRun {
    run_id: String,
    cancel_token: CancellationToken,
}

pub struct GooseAcpAgentOptions {
    pub provider_factory: AcpProviderFactory,
    pub builtins: Vec<String>,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub disable_session_naming: bool,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
    pub scheduler: Arc<dyn SchedulerTrait>,
}

pub struct GooseAcpAgent {
    sessions: Arc<Mutex<HashMap<String, GooseAcpSession>>>,
    active_prompt_runs: Arc<Mutex<HashMap<String, ActivePromptRun>>>,
    closed_session_ids: Arc<Mutex<HashSet<String>>>,
    agent_manager: Arc<AgentManager>,
    provider_factory: AcpProviderFactory,
    builtins: Vec<String>,
    client_fs_capabilities: OnceCell<FileSystemCapabilities>,
    client_terminal: OnceCell<bool>,
    client_mcp_host_info: OnceCell<GooseMcpHostInfo>,
    client_supports_acp_elicitation: OnceCell<bool>,
    client_supports_goose_custom_notifications: OnceCell<bool>,
    client_supports_recipe_param_requests: OnceCell<bool>,
    client_requests_tool_call_label_enrichment: OnceCell<bool>,
    use_login_shell_path: OnceCell<bool>,
    client_cx: OnceCell<ConnectionTo<Client>>,
    config_dir: std::path::PathBuf,
    session_manager: Arc<SessionManager>,
    permission_manager: Arc<PermissionManager>,
    disable_session_naming: bool,
    provider_inventory: ProviderInventoryService,
    additional_source_roots: Vec<SourceRoot>,
    recipe_path_cache: Arc<Mutex<HashMap<String, PathBuf>>>,
}

fn meta_string(
    meta: Option<&Meta>,
    key: &str,
) -> Result<Option<String>, agent_client_protocol::Error> {
    let Some(value) = meta.and_then(|m| m.get(key)) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(
            agent_client_protocol::Error::invalid_params().data(format!("{key} must be a string"))
        );
    };
    Ok(Some(value.to_string()))
}

fn agent_capabilities_meta() -> Option<Meta> {
    let mut goose = serde_json::Map::new();
    if cfg!(feature = "local-inference") {
        goose.insert("localInference".to_string(), serde_json::json!({}));
    }

    if goose.is_empty() {
        return None;
    }

    let mut meta = serde_json::Map::new();
    meta.insert("goose".to_string(), serde_json::Value::Object(goose));
    Some(meta)
}

fn spawn_session_name_update_notifier(
    cx: ConnectionTo<Client>,
) -> tokio::sync::mpsc::UnboundedSender<crate::session::SessionNameUpdate> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<crate::session::SessionNameUpdate>();
    tokio::spawn(async move {
        while let Some(update) = rx.recv().await {
            let mut meta = serde_json::Map::new();
            meta.insert(
                "messageCount".to_string(),
                serde_json::Value::Number(update.message_count.into()),
            );
            meta.insert(
                "userSetName".to_string(),
                serde_json::Value::Bool(update.user_set_name),
            );
            let notification = SessionNotification::new(
                SessionId::new(update.session_id.clone()),
                SessionUpdate::SessionInfoUpdate(
                    SessionInfoUpdate::new()
                        .title(update.name)
                        .updated_at(update.updated_at.to_rfc3339())
                        .meta(meta),
                ),
            );
            if let Err(error) = cx.send_notification(notification) {
                warn!(
                    session_id = %update.session_id,
                    error = %error,
                    "Failed to send generated session name update"
                );
            }
        }
    });
    tx
}

fn extract_timeout_from_meta(meta: &Option<Meta>) -> Option<u64> {
    meta.as_ref()
        .and_then(|m| m.get("timeout"))
        .and_then(|v| v.as_u64())
}

#[derive(Debug, Default, Deserialize)]
struct ClientCapabilitiesMeta {
    #[serde(default)]
    goose: Option<GooseClientCapabilities>,
}

#[derive(Debug, Default, Deserialize)]
struct GooseClientCapabilities {
    #[serde(rename = "mcpHostCapabilities", default)]
    mcp_host_capabilities: Option<GooseMcpHostCapabilities>,
    #[serde(rename = "customNotifications", default)]
    custom_notifications: Option<bool>,
    #[serde(rename = "recipeParameterRequests", default)]
    recipe_parameter_requests: Option<bool>,
    #[serde(rename = "toolCallLabelEnrichment", default)]
    tool_call_label_enrichment: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct GooseMcpHostCapabilities {
    #[serde(default)]
    extensions: Option<rmcp::model::ExtensionCapabilities>,
}

fn extract_client_capabilities_meta(args: &InitializeRequest) -> Option<ClientCapabilitiesMeta> {
    args.client_capabilities
        .meta
        .as_ref()
        .and_then(|meta| serde_json::from_value(serde_json::Value::Object(meta.clone())).ok())
}

fn extract_client_mcp_host_info(
    args: &InitializeRequest,
    goose_client_capabilities: Option<&GooseClientCapabilities>,
) -> GooseMcpHostInfo {
    let host_capabilities =
        goose_client_capabilities.and_then(|goose| goose.mcp_host_capabilities.as_ref());
    let explicit_extensions = host_capabilities
        .as_ref()
        .and_then(|capabilities| capabilities.extensions.as_ref())
        .is_some();
    let extensions = host_capabilities
        .and_then(|capabilities| capabilities.extensions.clone())
        .unwrap_or_default();

    GooseMcpHostInfo {
        explicit_extensions,
        extensions,
        client_name: args.client_info.as_ref().map(|info| info.name.clone()),
        client_version: args.client_info.as_ref().map(|info| info.version.clone()),
    }
}

fn extract_use_login_shell_path(args: &InitializeRequest) -> bool {
    args.meta
        .as_ref()
        .and_then(|meta| meta.get("goose/useLoginShellPath"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn mcp_server_to_extension_config(mcp_server: McpServer) -> Result<ExtensionConfig, String> {
    match mcp_server {
        McpServer::Stdio(stdio) => {
            let timeout = extract_timeout_from_meta(&stdio.meta);
            Ok(ExtensionConfig::Stdio {
                name: stdio.name,
                description: String::new(),
                cmd: stdio.command.to_string_lossy().to_string(),
                args: stdio.args,
                envs: Envs::new(stdio.env.into_iter().map(|e| (e.name, e.value)).collect()),
                env_keys: vec![],
                timeout,
                cwd: None,
                bundled: Some(false),
                available_tools: vec![],
            })
        }
        McpServer::Http(http) => {
            let timeout = extract_timeout_from_meta(&http.meta);
            Ok(ExtensionConfig::StreamableHttp {
                name: http.name,
                description: String::new(),
                uri: http.url,
                envs: Envs::default(),
                env_keys: vec![],
                headers: http
                    .headers
                    .into_iter()
                    .map(|h| (h.name, h.value))
                    .collect(),
                timeout,
                socket: None,
                bundled: Some(false),
                available_tools: vec![],
            })
        }
        McpServer::Sse(_) => Err("SSE is unsupported, migrate to streamable_http".to_string()),
        _ => Err("Unknown MCP server type".to_string()),
    }
}

fn push_or_replace_extension(extensions: &mut Vec<ExtensionConfig>, extension: ExtensionConfig) {
    let name = extension.name().to_string();
    if let Some(index) = extensions
        .iter()
        .position(|existing| existing.name() == name)
    {
        extensions.remove(index);
    }
    extensions.push(extension);
}

fn resolve_default_provider_model_config(
    config: &Config,
) -> Result<(String, goose_providers::model::ModelConfig), agent_client_protocol::Error> {
    let resolved_provider = config.get_goose_provider().map_err(|error| {
        agent_client_protocol::Error::internal_error()
            .data(format!("Failed to resolve provider: {}", error))
    })?;
    let resolved_model = config.get_goose_model().map_err(|error| {
        agent_client_protocol::Error::internal_error()
            .data(format!("Failed to resolve model: {}", error))
    })?;
    let resolved_model_config =
        crate::model_config::model_config_from_user_config(&resolved_provider, &resolved_model)
            .map_err(|error| {
                agent_client_protocol::Error::internal_error()
                    .data(format!("Failed to resolve model: {}", error))
            })?;
    Ok((resolved_provider, resolved_model_config))
}

async fn resolve_provider_default_model_config(
    provider_name: &str,
) -> Result<goose_providers::model::ModelConfig, agent_client_protocol::Error> {
    let entry = crate::providers::get_from_registry(provider_name)
        .await
        .map_err(|error| {
            agent_client_protocol::Error::invalid_params()
                .data(format!("Unknown provider '{}': {}", provider_name, error))
        })?;
    crate::model_config::model_config_from_user_config(
        provider_name,
        &entry.metadata().default_model,
    )
    .map_err(|error| {
        agent_client_protocol::Error::internal_error()
            .data(format!("Failed to resolve model: {}", error))
    })
}

fn read_resource_link(link: ResourceLink) -> Option<String> {
    let url = Url::parse(&link.uri).ok()?;
    if url.scheme() == "file" {
        let path = url.to_file_path().ok()?;
        let contents = fs::read_to_string(&path).ok()?;

        Some(format!(
            "\n\n# {}\n```\n{}\n```",
            path.to_string_lossy(),
            contents
        ))
    } else {
        None
    }
}

fn builtin_to_extension_config(name: &str) -> ExtensionConfig {
    if let Some(def) = PLATFORM_EXTENSIONS.get(name) {
        ExtensionConfig::Platform {
            name: def.name.into(),
            description: def.description.into(),
            display_name: Some(def.display_name.into()),
            bundled: Some(true),
            available_tools: vec![],
        }
    } else {
        ExtensionConfig::Builtin {
            name: name.into(),
            display_name: None,
            timeout: None,
            bundled: Some(true),
            description: name.into(),
            available_tools: vec![],
        }
    }
}

fn to_nonnegative_u64(value: Option<i32>) -> Option<u64> {
    value.and_then(|v| u64::try_from(v).ok())
}

fn build_prompt_usage(session: &Session) -> Option<Usage> {
    let total = to_nonnegative_u64(session.usage.total_tokens)?;
    let input = to_nonnegative_u64(session.usage.input_tokens).unwrap_or(0);
    let output = to_nonnegative_u64(session.usage.output_tokens).unwrap_or(0);
    Some(Usage::new(total, input, output))
}

pub(super) struct UsageUpdates {
    pub(super) custom: GooseSessionNotification,
    pub(super) standard: UsageUpdate,
}

pub(super) fn build_usage_updates(
    session: &Session,
    totals: &SessionUsageTotals,
) -> Option<UsageUpdates> {
    let used = session.usage.total_tokens.unwrap_or(0).max(0) as u64;
    let ctx_limit = session.model_config.as_ref()?.context_limit() as u64;
    let accumulated_input_tokens =
        to_nonnegative_u64(totals.accumulated_usage.input_tokens).unwrap_or(0);
    let accumulated_output_tokens =
        to_nonnegative_u64(totals.accumulated_usage.output_tokens).unwrap_or(0);
    Some(UsageUpdates {
        custom: GooseSessionNotification {
            session_id: session.id.clone(),
            update: GooseSessionUpdate::UsageUpdate(SessionUsageUpdate {
                used,
                context_limit: ctx_limit,
                accumulated_input_tokens,
                accumulated_output_tokens,
                accumulated_cost: totals.accumulated_cost,
            }),
        },
        standard: {
            let mut standard = UsageUpdate::new(used, ctx_limit);
            if let Some(amount) = totals.accumulated_cost {
                standard = standard.cost(Cost::new(amount, "USD"));
            }
            standard
        },
    })
}

pub(super) fn validate_absolute_cwd(cwd: &Path) -> Result<(), agent_client_protocol::Error> {
    if !cwd.is_absolute() {
        return Err(
            agent_client_protocol::Error::invalid_params().data("cwd must be an absolute path")
        );
    }

    if !cwd.exists() || !cwd.is_dir() {
        return Err(agent_client_protocol::Error::invalid_params().data("invalid directory path"));
    }

    Ok(())
}

impl GooseAcpAgent {
    pub fn permission_manager(&self) -> Arc<PermissionManager> {
        Arc::clone(&self.permission_manager)
    }

    pub(super) fn supports_goose_custom_notifications(&self) -> bool {
        self.client_supports_goose_custom_notifications
            .get()
            .copied()
            .unwrap_or(false)
    }

    pub(super) async fn notify_session_setup(
        &self,
        cx: &ConnectionTo<Client>,
        session: &Session,
    ) -> Result<(), agent_client_protocol::Error> {
        let totals = self
            .session_manager
            .get_session_usage_totals(&session.id)
            .await
            .unwrap_or_default();
        send_session_setup_notifications(
            cx,
            session,
            &totals,
            self.supports_goose_custom_notifications(),
        )
    }

    pub(super) fn supports_recipe_param_requests(&self) -> bool {
        self.client_supports_recipe_param_requests
            .get()
            .copied()
            .unwrap_or(false)
    }

    fn requests_tool_call_label_enrichment(&self) -> bool {
        self.client_requests_tool_call_label_enrichment
            .get()
            .copied()
            .unwrap_or(false)
    }

    fn supports_acp_elicitation(&self) -> bool {
        self.client_supports_acp_elicitation
            .get()
            .copied()
            .unwrap_or(false)
    }

    // TODO: goose reads Paths::in_state_dir globally (e.g. RequestLog), ignoring this data_dir.
    pub async fn new(options: GooseAcpAgentOptions) -> Result<Self> {
        let session_manager = Arc::new(SessionManager::new(options.data_dir));

        // Eagerly initialize the SQLite pool so it's ready when providers/sessions need it.
        let storage_clone = session_manager.storage().clone();
        tokio::spawn(async move {
            let _ = storage_clone.pool().await;
        });

        let permission_manager = Arc::new(PermissionManager::new(options.config_dir.clone()));
        let provider_inventory = ProviderInventoryService::new(session_manager.storage().clone());
        let agent_config = AgentConfig::new(
            Arc::clone(&session_manager),
            Arc::clone(&permission_manager),
            Some(options.scheduler),
            Config::global().get_goose_mode().unwrap_or_default(),
            options.disable_session_naming,
            options.goose_platform.clone(),
        );
        let agent_manager = Arc::new(AgentManager::new(agent_config, None).await?);

        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            active_prompt_runs: Arc::new(Mutex::new(HashMap::new())),
            closed_session_ids: Arc::new(Mutex::new(HashSet::new())),
            agent_manager,
            provider_factory: options.provider_factory,
            builtins: options.builtins,
            client_fs_capabilities: OnceCell::new(),
            client_terminal: OnceCell::new(),
            client_mcp_host_info: OnceCell::new(),
            client_supports_acp_elicitation: OnceCell::new(),
            client_supports_goose_custom_notifications: OnceCell::new(),
            client_supports_recipe_param_requests: OnceCell::new(),
            client_requests_tool_call_label_enrichment: OnceCell::new(),
            use_login_shell_path: OnceCell::new(),
            client_cx: OnceCell::new(),
            config_dir: options.config_dir,
            session_manager,
            permission_manager,
            disable_session_naming: options.disable_session_naming,
            provider_inventory,
            additional_source_roots: options.additional_source_roots,
            recipe_path_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn config(&self) -> Result<&'static Config, agent_client_protocol::Error> {
        Ok(Config::global())
    }

    async fn create_provider(
        &self,
        provider_name: &str,
        extensions: Vec<ExtensionConfig>,
        working_dir: Option<PathBuf>,
    ) -> Result<Arc<dyn Provider>> {
        (self.provider_factory)(provider_name.to_string(), extensions, working_dir).await
    }

    async fn maybe_refresh_provider_inventory_with_agent(
        &self,
        goose_session: &Session,
        agent: &Arc<Agent>,
    ) {
        let Some(provider_name) = goose_session.provider_name.as_deref() else {
            return;
        };
        let Some(mut inventory) = self
            .provider_inventory
            .find_entry_for_provider(provider_name)
            .await
        else {
            return;
        };
        if !should_refresh_inventory_for_session_init(&inventory) {
            return;
        }
        let provider = match agent.provider().await {
            Ok(provider) => provider,
            Err(error) => {
                warn!(
                    provider = %provider_name,
                    session = %goose_session.id,
                    error = %error,
                    "agent has no provider available for inventory refresh"
                );
                return;
            }
        };
        self.provider_inventory
            .refresh_with_provider(provider_name, &provider, &mut inventory, "session init")
            .await;
    }

    async fn get_or_create_session_agent_with_results(
        &self,
        cx: &ConnectionTo<Client>,
        session_id: String,
    ) -> Result<AgentManagerGetResult, agent_client_protocol::Error> {
        self.agent_manager
            .get_or_create_agent_with_runtime_context(
                session_id,
                RuntimeContext {
                    mcp_host_info: self.client_mcp_host_info.get().cloned(),
                    use_login_shell_path: self.use_login_shell_path.get().copied(),
                    session_name_update_tx: (!self.disable_session_naming)
                        .then(|| spawn_session_name_update_notifier(cx.clone())),
                },
            )
            .await
            .internal_err_ctx("Failed to create agent")
    }

    fn initial_session_extensions(
        &self,
        config: &Config,
        project_root: &Path,
        mcp_servers: Vec<McpServer>,
        goose_extensions: Option<Vec<GooseExtension>>,
        recipe_extensions: Option<&[ExtensionConfig]>,
    ) -> Result<Vec<ExtensionConfig>, agent_client_protocol::Error> {
        let mut extensions = Vec::new();
        for builtin in &self.builtins {
            push_or_replace_extension(&mut extensions, builtin_to_extension_config(builtin));
        }

        if let Some(recipe_extensions) = recipe_extensions {
            for extension in recipe_extensions {
                push_or_replace_extension(&mut extensions, extension.clone());
            }
        } else if let Some(goose_extensions) = goose_extensions {
            for extension in extensions::goose_extensions_to_configs(goose_extensions)? {
                push_or_replace_extension(&mut extensions, extension);
            }
        } else if mcp_servers.is_empty() {
            for extension in get_enabled_extensions_with_config(config) {
                push_or_replace_extension(&mut extensions, extension);
            }
            for extension in
                crate::plugins::mcp_servers::enabled_plugin_mcp_servers(Some(project_root))
            {
                push_or_replace_extension(&mut extensions, extension);
            }
        } else {
            for mcp_server in mcp_servers {
                let extension = mcp_server_to_extension_config(mcp_server).map_err(|message| {
                    agent_client_protocol::Error::invalid_params().data(message)
                })?;
                push_or_replace_extension(&mut extensions, extension);
            }
        }

        Ok(extensions)
    }

    async fn apply_acp_extension_overrides(
        &self,
        cx: &ConnectionTo<Client>,
        agent: &Arc<Agent>,
        session: &Session,
    ) {
        let client_fs_capabilities = self
            .client_fs_capabilities
            .get()
            .cloned()
            .unwrap_or_default();
        let client_terminal = self.client_terminal.get().copied().unwrap_or(false);
        if !client_fs_capabilities.read_text_file
            && !client_fs_capabilities.write_text_file
            && !client_terminal
        {
            return;
        }

        if !agent
            .extension_manager
            .is_extension_enabled("developer")
            .await
        {
            return;
        }

        let context = agent.extension_manager.get_context().clone();
        let dev_client = match DeveloperClient::new(context) {
            Ok(dev_client) => dev_client,
            Err(error) => {
                warn!(error = %error, "Failed to create ACP developer client");
                return;
            }
        };

        let session_id = SessionId::new(session.id.clone());
        let client: Arc<dyn McpClientTrait> = Arc::new(AcpTools {
            inner: Arc::new(dev_client),
            cx: cx.clone(),
            session_id: session_id.clone(),
            tool_call_notifier: ToolCallNotifier::new(cx, &session_id),
            fs_read: client_fs_capabilities.read_text_file,
            fs_write: client_fs_capabilities.write_text_file,
            terminal: client_terminal,
        });
        let info = client.get_info().cloned();

        let developer_config = agent
            .extension_manager
            .get_extension_configs()
            .await
            .into_iter()
            .find(|extension| extension.name() == "developer")
            .unwrap_or_else(|| builtin_to_extension_config("developer"));

        agent
            .extension_manager
            .add_client("developer".into(), developer_config, client, info, None)
            .await;
    }

    async fn prepare_acp_session_agent(
        &self,
        cx: &ConnectionTo<Client>,
        session: &Session,
    ) -> Result<(Arc<Agent>, Vec<ExtensionLoadResult>), agent_client_protocol::Error> {
        let agent_result = self
            .get_or_create_session_agent_with_results(cx, session.id.clone())
            .await?;
        let agent = agent_result.agent.clone();
        self.apply_acp_extension_overrides(cx, &agent, session)
            .await;
        self.maybe_refresh_provider_inventory_with_agent(session, &agent)
            .await;

        Ok((agent, agent_result.extension_results))
    }

    async fn prepare_session_for_activation(
        &self,
        mut session: Session,
        cwd: std::path::PathBuf,
        mcp_servers: Vec<McpServer>,
        include_messages_on_reload: bool,
    ) -> Result<Session, agent_client_protocol::Error> {
        let config = Config::global();
        let mut builder = self.session_manager.update(&session.id);
        let mut session_needs_update = false;

        if cwd != session.working_dir {
            builder = builder.working_dir(cwd);
            session_needs_update = true;
        }

        if session.provider_name.is_none() || session.model_config.is_none() {
            let (resolved_provider, resolved_model_config) =
                resolve_default_provider_model_config(config)?;
            builder = builder
                .provider_name(resolved_provider)
                .model_config(resolved_model_config);
            session_needs_update = true;
        }

        if !mcp_servers.is_empty()
            || EnabledExtensionsState::from_extension_data(&session.extension_data).is_none()
        {
            let extension_data =
                self.build_enabled_extensions_data(config, &session, mcp_servers, None, None)?;
            builder = builder.extension_data(extension_data);
            session_needs_update = true;
        }

        if session_needs_update {
            let session_id = session.id.clone();
            builder
                .apply()
                .await
                .internal_err_ctx("Failed to update session")?;

            self.agent_manager
                .remove_session_if_loaded(&session_id)
                .await
                .internal_err_ctx("Failed to remove in-memory agent")?;

            session = self
                .session_manager
                .get_session(&session_id, include_messages_on_reload)
                .await
                .internal_err_ctx("Failed to reload session")?;
        }

        Ok(session)
    }

    fn build_enabled_extensions_data(
        &self,
        config: &Config,
        session: &Session,
        mcp_servers: Vec<McpServer>,
        goose_extensions: Option<Vec<GooseExtension>>,
        recipe_extensions: Option<&[ExtensionConfig]>,
    ) -> Result<ExtensionData, agent_client_protocol::Error> {
        let extensions = self.initial_session_extensions(
            config,
            &session.working_dir,
            mcp_servers,
            goose_extensions,
            recipe_extensions,
        )?;
        let mut extension_data = session.extension_data.clone();
        EnabledExtensionsState::new(extensions)
            .to_extension_data(&mut extension_data)
            .internal_err_ctx("Failed to initialize session extensions")?;
        Ok(extension_data)
    }

    async fn register_acp_session(&self, session_id: String, agent: Arc<Agent>) {
        let acp_session = GooseAcpSession { agent };
        self.sessions.lock().await.insert(session_id, acp_session);
    }

    async fn activate_acp_session(
        &self,
        cx: &ConnectionTo<Client>,
        session: &Session,
    ) -> Result<(Arc<Agent>, Vec<ExtensionLoadResult>), agent_client_protocol::Error> {
        let (agent, extension_results) = self.prepare_acp_session_agent(cx, session).await?;
        self.register_acp_session(session.id.clone(), agent.clone())
            .await;

        Ok((agent, extension_results))
    }

    pub async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.lock().await.contains_key(session_id)
    }

    /// Convert ACP prompt content blocks into a user message.
    fn convert_acp_prompt_to_message(prompt: &[ContentBlock]) -> Message {
        let mut message = Message::user();
        for block in prompt {
            match block {
                ContentBlock::Text(text) => {
                    let annotated = if let Some(ref ann) = text.annotations {
                        let audience: Vec<Role> = ann
                            .audience
                            .as_ref()
                            .map(|roles| {
                                roles
                                    .iter()
                                    .filter_map(|r| match r {
                                        agent_client_protocol::schema::v1::Role::Assistant => {
                                            Some(Role::Assistant)
                                        }
                                        agent_client_protocol::schema::v1::Role::User => {
                                            Some(Role::User)
                                        }
                                        _ => None,
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let raw = RawTextContent {
                            text: sanitize_unicode_tags(&text.text),
                            meta: None,
                        };
                        if audience.is_empty() {
                            raw.no_annotation()
                        } else {
                            raw.no_annotation().with_audience(audience)
                        }
                    } else {
                        // No annotations — regular user text.
                        let sanitized = sanitize_unicode_tags(&text.text);
                        RawTextContent {
                            text: sanitized,
                            meta: None,
                        }
                        .no_annotation()
                    };
                    message = message.with_content(MessageContent::Text(annotated));
                }
                ContentBlock::Image(image) => {
                    message = message.with_image(&image.data, &image.mime_type);
                }
                ContentBlock::Resource(resource) => {
                    if let EmbeddedResourceResource::TextResourceContents(text_resource) =
                        &resource.resource
                    {
                        let header = format!("--- Resource: {} ---\n", text_resource.uri);
                        let content = format!("{}{}\n---\n", header, text_resource.text);
                        message = message.with_text(&content);
                    }
                }
                ContentBlock::ResourceLink(link) => {
                    if let Some(text) = read_resource_link(link.clone()) {
                        message = message.with_text(text);
                    }
                }
                ContentBlock::Audio(..) | _ => (),
            }
        }
        message
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_message_content(
        &self,
        content_item: &MessageContent,
        session_id: &SessionId,
        session_id_str: &str,
        message_id: Option<&str>,
        message_created: i64,
        role: &Role,
        steer: bool,
        agent: &Arc<Agent>,
        tool_requests: &HashMap<String, ToolRequest>,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        match content_item {
            MessageContent::Text(text) => {
                let chunk =
                    ContentChunk::new(ContentBlock::Text(TextContent::new(text.text.clone())))
                        .meta(message_update_meta(message_id, message_created, steer));
                let update = match role {
                    Role::User => SessionUpdate::UserMessageChunk(chunk),
                    Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
                };
                cx.send_notification(SessionNotification::new(session_id.clone(), update))?;
            }
            MessageContent::ToolRequest(tool_request) => {
                self.handle_tool_request(
                    tool_request,
                    session_id,
                    session_id_str,
                    message_id,
                    agent,
                    cx,
                )
                .await?;
            }
            MessageContent::ToolResponse(tool_response) => {
                self.handle_tool_response(
                    tool_response,
                    tool_requests.get(&tool_response.id),
                    session_id,
                    cx,
                )
                .await?;
            }
            MessageContent::Thinking(thinking) => {
                cx.send_notification(SessionNotification::new(
                    session_id.clone(),
                    SessionUpdate::AgentThoughtChunk(
                        ContentChunk::new(ContentBlock::Text(TextContent::new(
                            thinking.thinking.clone(),
                        )))
                        .meta(message_update_meta(
                            message_id,
                            message_created,
                            steer,
                        )),
                    ),
                ))?;
            }
            MessageContent::ActionRequired(action_required) => match &action_required.data {
                ActionRequiredData::ToolConfirmation {
                    id,
                    tool_name,
                    arguments,
                    prompt,
                } => {
                    self.handle_tool_permission_request(
                        cx,
                        agent,
                        session_id,
                        id.clone(),
                        tool_name.clone(),
                        arguments.clone(),
                        prompt.clone(),
                    )?;
                }
                ActionRequiredData::Elicitation {
                    id,
                    message,
                    requested_schema,
                } => {
                    self.handle_form_elicitation(
                        cx,
                        session_id,
                        id,
                        message,
                        requested_schema,
                        message_update_meta(message_id, message_created, false),
                    )
                    .await?;
                }
                ActionRequiredData::ElicitationResponse { .. } => {}
            },
            MessageContent::Image(image) => {
                let mut image_content =
                    ImageContent::new(image.data.clone(), image.mime_type.clone());
                if let Some(audience) = image.audience() {
                    image_content = image_content.annotations(
                        Annotations::new().audience(
                            audience
                                .iter()
                                .map(|r| match r {
                                    Role::Assistant => {
                                        agent_client_protocol::schema::v1::Role::Assistant
                                    }
                                    Role::User => agent_client_protocol::schema::v1::Role::User,
                                })
                                .collect::<Vec<_>>(),
                        ),
                    );
                }
                let chunk = ContentChunk::new(ContentBlock::Image(image_content))
                    .meta(message_update_meta(message_id, message_created, steer));
                let update = match role {
                    Role::User => SessionUpdate::UserMessageChunk(chunk),
                    Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
                };
                cx.send_notification(SessionNotification::new(session_id.clone(), update))?;
            }
            MessageContent::SystemNotification(notification) => {
                send_status_message_update(
                    cx,
                    self.supports_goose_custom_notifications(),
                    session_id.0.as_ref(),
                    notification,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn spawn_ready_chain_summary(
        &self,
        chain: ReadyToolChain,
        agent: &Arc<Agent>,
        session_id: &SessionId,
        cx: &ConnectionTo<Client>,
    ) {
        if !self.requests_tool_call_label_enrichment() {
            return;
        }

        let tool_call_notifier = ToolCallNotifier::new(cx, session_id);
        spawn_chain_summary_enrichment(
            agent,
            session_id,
            tool_call_notifier,
            &self.session_manager,
            chain,
        );
    }

    async fn handle_tool_request(
        &self,
        tool_request: &ToolRequest,
        session_id: &SessionId,
        session_id_for_persist: &str,
        message_id: Option<&str>,
        agent: &Arc<Agent>,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        let client_requests_label_enrichment = self.requests_tool_call_label_enrichment();
        let initial_tool_call =
            build_initial_tool_call(tool_request, client_requests_label_enrichment);
        let tool_call_notifier = ToolCallNotifier::new(cx, session_id);
        tool_call_notifier.send_initial(initial_tool_call)?;

        if !client_requests_label_enrichment {
            return Ok(());
        }

        if tool_request.tool_call.is_ok() {
            spawn_tool_title_enrichment(
                agent,
                tool_call_notifier,
                &self.session_manager,
                session_id_for_persist,
                message_id,
                tool_request,
            );
        }

        Ok(())
    }

    async fn handle_tool_response(
        &self,
        tool_response: &ToolResponse,
        tool_request: Option<&ToolRequest>,
        session_id: &SessionId,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        let fields = tool_call_update_fields_from_response(tool_response, tool_request, false);

        let update = ToolCallUpdate::new(ToolCallId::new(tool_response.id.clone()), fields)
            .meta(trusted_update_meta(tool_response));
        let tool_call_notifier = ToolCallNotifier::new(cx, session_id);
        tool_call_notifier.send_update(update)?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_tool_permission_request(
        &self,
        cx: &ConnectionTo<Client>,
        agent: &Arc<Agent>,
        session_id: &SessionId,
        request_id: String,
        tool_name: String,
        arguments: serde_json::Map<String, serde_json::Value>,
        prompt: Option<String>,
    ) -> Result<(), agent_client_protocol::Error> {
        let cx = cx.clone();
        let agent = agent.clone();
        let session_id = session_id.clone();

        let tool_call_update =
            build_permission_tool_call_update(&request_id, &tool_name, arguments, prompt);

        fn option(kind: PermissionOptionKind) -> PermissionOption {
            let id = serde_json::to_value(kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            PermissionOption::new(id.clone(), id, kind)
        }
        let options = vec![
            option(PermissionOptionKind::AllowAlways),
            option(PermissionOptionKind::AllowOnce),
            option(PermissionOptionKind::RejectOnce),
            option(PermissionOptionKind::RejectAlways),
        ];

        let permission_request =
            RequestPermissionRequest::new(session_id, tool_call_update, options);

        cx.send_request(permission_request)
            .on_receiving_result(move |result| async move {
                match result {
                    Ok(response) => {
                        agent
                            .handle_confirmation(
                                request_id,
                                outcome_to_confirmation(&response.outcome),
                            )
                            .await;
                        Ok(())
                    }
                    Err(e) => {
                        error!(error = ?e, "permission request failed");
                        agent
                            .handle_confirmation(
                                request_id,
                                PermissionConfirmation {
                                    principal_type: PrincipalType::Tool,
                                    permission: Permission::Cancel,
                                },
                            )
                            .await;
                        Ok(())
                    }
                }
            })?;

        Ok(())
    }

    fn is_builtin_agent_command(command: &str) -> bool {
        let normalized = command.trim_start_matches('/');

        crate::agents::execute_commands::list_commands()
            .iter()
            .any(|cmd| cmd.name == normalized)
            || crate::agents::execute_commands::COMPACT_TRIGGERS
                .iter()
                .filter_map(|trigger| trigger.strip_prefix('/'))
                .any(|trigger| trigger == normalized)
    }
}

fn extract_client_supports_goose_custom_notifications(
    goose_client_capabilities: Option<&GooseClientCapabilities>,
) -> bool {
    goose_client_capabilities
        .and_then(|goose| goose.custom_notifications)
        .unwrap_or(false)
}

fn extract_client_supports_recipe_param_requests(
    goose_client_capabilities: Option<&GooseClientCapabilities>,
) -> bool {
    goose_client_capabilities
        .and_then(|goose| goose.recipe_parameter_requests)
        .unwrap_or(false)
}

fn outcome_to_confirmation(outcome: &RequestPermissionOutcome) -> PermissionConfirmation {
    PermissionConfirmation {
        principal_type: PrincipalType::Tool,
        permission: Permission::from(PermissionDecision::from(outcome)),
    }
}

fn prompt_error_from_message_content(
    content_item: &MessageContent,
) -> Option<agent_client_protocol::Error> {
    match content_item {
        MessageContent::SystemNotification(notification)
            if notification.notification_type == SystemNotificationType::CreditsExhausted =>
        {
            Some(credits_exhausted_prompt_error(notification))
        }
        _ => None,
    }
}

fn credits_exhausted_prompt_error(
    notification: &SystemNotificationContent,
) -> agent_client_protocol::Error {
    let mut data = serde_json::Map::new();
    data.insert(
        "reason".to_string(),
        serde_json::Value::String("credits_exhausted".to_string()),
    );

    if let Some(url) = notification
        .data
        .as_ref()
        .and_then(|data| data.get("top_up_url"))
        .and_then(|url| url.as_str())
    {
        data.insert(
            "url".to_string(),
            serde_json::Value::String(url.to_string()),
        );
    }

    agent_client_protocol::Error::new(-32603, notification.msg.clone())
        .data(serde_json::Value::Object(data))
}

fn send_status_message_update(
    cx: &ConnectionTo<Client>,
    supports_goose_custom_notifications: bool,
    session_id: &str,
    notification: &SystemNotificationContent,
) -> Result<(), agent_client_protocol::Error> {
    if let Some(status) = status_message_from_system_notification(notification) {
        if supports_goose_custom_notifications {
            cx.send_notification(GooseSessionNotification {
                session_id: session_id.to_string(),
                update: GooseSessionUpdate::StatusMessage(StatusMessageUpdate { status }),
            })?;
        }
    }
    Ok(())
}

fn send_progress_message_update(
    cx: &ConnectionTo<Client>,
    supports_goose_custom_notifications: bool,
    session_id: &str,
    message: String,
) -> Result<(), agent_client_protocol::Error> {
    if supports_goose_custom_notifications {
        cx.send_notification(GooseSessionNotification {
            session_id: session_id.to_string(),
            update: GooseSessionUpdate::StatusMessage(StatusMessageUpdate {
                status: StatusMessage::Progress { message },
            }),
        })?;
    }
    Ok(())
}

fn status_message_from_system_notification(
    notification: &SystemNotificationContent,
) -> Option<StatusMessage> {
    match notification.notification_type {
        SystemNotificationType::InlineMessage => Some(StatusMessage::Notice {
            message: notification.msg.clone(),
        }),
        SystemNotificationType::ThinkingMessage | SystemNotificationType::ProgressMessage => {
            Some(StatusMessage::Progress {
                message: notification.msg.clone(),
            })
        }
        SystemNotificationType::CreditsExhausted => None,
    }
}

/// Conversion to the sdk-types wire mirror carried by `message_usage`.
fn message_usage_update(
    message_id: Option<String>,
    usage: &crate::conversation::message::MessageUsage,
) -> MessageUsageUpdate {
    use crate::conversation::token_usage::CostSource;

    MessageUsageUpdate {
        message_id,
        usage: MessageUsageData {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            cache_write_tokens: usage.cache_write_tokens,
            cost: usage.cost,
            cost_source: usage.cost_source.map(|source| match source {
                CostSource::ProviderReported => CostSourceData::ProviderReported,
                CostSource::Estimated => CostSourceData::Estimated,
            }),
            elapsed_ms: usage.elapsed_ms,
            time_to_first_token_ms: usage.time_to_first_token_ms,
            is_compaction: usage.is_compaction,
        },
    }
}

fn message_update_meta(message_id: Option<&str>, created: i64, steer: bool) -> Meta {
    let mut goose = serde_json::Map::new();
    goose.insert("created".to_string(), serde_json::json!(created));
    if let Some(id) = message_id {
        goose.insert("messageId".to_string(), serde_json::json!(id));
    }
    if steer {
        goose.insert("steer".to_string(), serde_json::json!(true));
    }

    let mut meta = serde_json::Map::new();
    meta.insert("goose".to_string(), serde_json::Value::Object(goose));
    meta
}

impl GooseAcpAgent {
    async fn on_initialize(
        &self,
        args: InitializeRequest,
    ) -> Result<InitializeResponse, agent_client_protocol::Error> {
        debug!(?args, "initialize request");

        let _ = self
            .client_fs_capabilities
            .set(args.client_capabilities.fs.clone());
        let _ = self.client_terminal.set(args.client_capabilities.terminal);
        let goose_client_capabilities =
            extract_client_capabilities_meta(&args).and_then(|meta| meta.goose);
        let _ = self.client_mcp_host_info.set(extract_client_mcp_host_info(
            &args,
            goose_client_capabilities.as_ref(),
        ));
        let _ = self.client_supports_goose_custom_notifications.set(
            extract_client_supports_goose_custom_notifications(goose_client_capabilities.as_ref()),
        );
        let _ = self.client_supports_recipe_param_requests.set(
            extract_client_supports_recipe_param_requests(goose_client_capabilities.as_ref()),
        );
        let client_requests_tool_call_label_enrichment = goose_client_capabilities
            .as_ref()
            .and_then(|goose| goose.tool_call_label_enrichment)
            .unwrap_or(false);
        let _ = self
            .client_requests_tool_call_label_enrichment
            .set(client_requests_tool_call_label_enrichment);
        let _ = self
            .client_supports_acp_elicitation
            .set(elicitation::client_supports_form_elicitation(&args));
        let _ = self
            .use_login_shell_path
            .set(extract_use_login_shell_path(&args));

        let capabilities = AgentCapabilities::new()
            .load_session(true)
            .session_capabilities(
                SessionCapabilities::new()
                    .list(SessionListCapabilities::new())
                    .close(SessionCloseCapabilities::new()),
            )
            .prompt_capabilities(
                PromptCapabilities::new()
                    .image(true)
                    .audio(false)
                    .embedded_context(true),
            )
            .mcp_capabilities(McpCapabilities::new().http(true))
            .meta(agent_capabilities_meta());
        Ok(InitializeResponse::new(args.protocol_version)
            .agent_info(Implementation::new("goose", env!("CARGO_PKG_VERSION")))
            .agent_capabilities(capabilities)
            .auth_methods(vec![AuthMethod::Agent(
                AuthMethodAgent::new("goose-provider", "Configure Provider")
                    .description("Run `goose configure` to set up your AI provider and API key"),
            )]))
    }

    async fn on_new_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: NewSessionRequest,
    ) -> Result<NewSessionResponse, agent_client_protocol::Error> {
        self.handle_new_session(cx, args).await
    }

    /// Look up the session's agent.
    async fn get_session_agent(
        &self,
        session_id: &str,
    ) -> Result<Arc<Agent>, agent_client_protocol::Error> {
        if self.closed_session_ids.lock().await.contains(session_id) {
            return Err(agent_client_protocol::Error::resource_not_found(Some(
                session_id.to_string(),
            ))
            .data(format!("Session not found: {}", session_id)));
        }

        {
            let sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get(session_id) {
                return Ok(session.agent.clone());
            }
        }

        let cx = self.client_cx.get().ok_or_else(|| {
            agent_client_protocol::Error::resource_not_found(Some(session_id.to_string()))
                .data(format!("Session not found: {}", session_id))
        })?;
        let session = self
            .session_manager
            .get_session(session_id, false)
            .await
            .map_err(|_| {
                agent_client_protocol::Error::resource_not_found(Some(session_id.to_string()))
                    .data(format!("Session not found: {}", session_id))
            })?;
        let (agent, _) = self.activate_acp_session(cx, &session).await?;
        Ok(agent)
    }

    async fn start_active_run(
        &self,
        session_id: &str,
        run_id: String,
        cancel_token: CancellationToken,
    ) -> Result<(), agent_client_protocol::Error> {
        if self.closed_session_ids.lock().await.contains(session_id) {
            return Err(agent_client_protocol::Error::resource_not_found(Some(
                session_id.to_string(),
            ))
            .data(format!("Session not found: {}", session_id)));
        }

        let mut active_prompt_runs = self.active_prompt_runs.lock().await;
        if let Some(active_run) = active_prompt_runs.get(session_id) {
            return Err(agent_client_protocol::Error::invalid_params().data(format!(
                "session already has active run `{}`; use _goose/unstable/session/steer",
                active_run.run_id.as_str()
            )));
        }

        active_prompt_runs.insert(
            session_id.to_string(),
            ActivePromptRun {
                run_id,
                cancel_token,
            },
        );
        Ok(())
    }

    async fn clear_active_run(&self, session_id: &str, run_id: &str) {
        {
            let mut active_prompt_runs = self.active_prompt_runs.lock().await;
            let Some(active_run) = active_prompt_runs.get(session_id) else {
                return;
            };

            if active_run.run_id != run_id {
                return;
            }

            active_prompt_runs.remove(session_id);
        }

        let agent = {
            let sessions = self.sessions.lock().await;
            sessions
                .get(session_id)
                .map(|session| session.agent.clone())
        };
        if let Some(agent) = agent {
            agent.discard_pending_steers(session_id).await;
        }

        if self.closed_session_ids.lock().await.contains(session_id) {
            self.sessions.lock().await.remove(session_id);
            if let Err(error) = self
                .agent_manager
                .remove_session_if_loaded(session_id)
                .await
            {
                tracing::warn!(
                    session_id,
                    %error,
                    "Failed to remove in-memory agent for closed session"
                );
            }
        }
    }

    async fn require_active_run(
        &self,
        session_id: &str,
        expected_run_id: &str,
    ) -> Result<String, agent_client_protocol::Error> {
        if expected_run_id.is_empty() {
            return Err(agent_client_protocol::Error::invalid_params()
                .data("expectedRunId must not be empty"));
        }

        let active_prompt_runs = self.active_prompt_runs.lock().await;
        let active_run = active_prompt_runs.get(session_id).ok_or_else(|| {
            agent_client_protocol::Error::invalid_params().data("no active run to steer")
        })?;
        if active_run.run_id != expected_run_id {
            return Err(
                agent_client_protocol::Error::invalid_params().data(serde_json::json!({
                    "message": format!(
                        "expected active run id `{expected_run_id}` but found `{}`",
                        active_run.run_id.as_str()
                    ),
                    "expectedRunId": expected_run_id,
                    "actualRunId": active_run.run_id.as_str(),
                })),
            );
        }
        Ok(active_run.run_id.clone())
    }

    fn active_run_meta(active_run_id: Option<&str>) -> Meta {
        let mut goose = serde_json::Map::new();
        goose.insert(
            "activeRunId".to_string(),
            active_run_id
                .map(|run_id| serde_json::Value::String(run_id.to_string()))
                .unwrap_or(serde_json::Value::Null),
        );

        let mut meta = serde_json::Map::new();
        meta.insert("goose".to_string(), serde_json::Value::Object(goose));
        meta
    }

    fn send_active_run_update(
        cx: &ConnectionTo<Client>,
        session_id: &SessionId,
        active_run_id: Option<&str>,
    ) -> Result<(), agent_client_protocol::Error> {
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::SessionInfoUpdate(
                SessionInfoUpdate::new().meta(Self::active_run_meta(active_run_id)),
            ),
        ))
    }

    fn send_queued_steer_update(
        cx: &ConnectionTo<Client>,
        session_id: &SessionId,
        message_id: &str,
        run_id: &str,
    ) -> Result<(), agent_client_protocol::Error> {
        let mut goose = serde_json::Map::new();
        goose.insert(
            "queuedSteer".to_string(),
            serde_json::json!({
                "messageId": message_id,
                "runId": run_id,
            }),
        );
        let mut meta = serde_json::Map::new();
        meta.insert("goose".to_string(), serde_json::Value::Object(goose));

        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(meta)),
        ))
    }

    async fn send_local_inference_progress_update(
        &self,
        cx: &ConnectionTo<Client>,
        acp_session_id: &SessionId,
        session_id: &str,
        agent: &Arc<Agent>,
    ) -> Result<(), agent_client_protocol::Error> {
        let Ok(provider) = agent.provider().await else {
            return Ok(());
        };
        if provider.get_name() != "local" {
            return Ok(());
        }

        let model_config = agent.model_config_for_session(session_id).await.ok();
        let model_name = model_config
            .as_ref()
            .map(|config| config.model_name.clone())
            .unwrap_or_else(|| "local model".to_string());

        #[cfg(feature = "local-inference")]
        if let Some(model_config) = model_config.as_ref() {
            if crate::providers::local_inference::is_model_loaded(&model_config.model_name)
                .await
                .unwrap_or(false)
            {
                return Ok(());
            }
        }

        send_progress_message_update(
            cx,
            self.supports_goose_custom_notifications(),
            acp_session_id.0.as_ref(),
            format!("Loading local model {model_name}..."),
        )
    }

    async fn on_load_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, agent_client_protocol::Error> {
        self.handle_load_session(cx, args).await
    }

    async fn on_prompt(
        &self,
        cx: &ConnectionTo<Client>,
        args: PromptRequest,
    ) -> Result<PromptResponse, agent_client_protocol::Error> {
        // The ACP session_id IS the thread ID.
        let session_id = args.session_id.0.to_string();

        let run_id = format!("run_{}", Uuid::new_v4());
        let cancel_token = CancellationToken::new();
        self.start_active_run(&session_id, run_id.clone(), cancel_token.clone())
            .await?;

        let agent = match self.get_session_agent(&session_id).await {
            Ok(agent) => agent,
            Err(error) => {
                self.clear_active_run(&session_id, &run_id).await;
                return Err(error);
            }
        };

        if cancel_token.is_cancelled() {
            self.clear_active_run(&session_id, &run_id).await;
            Self::send_active_run_update(cx, &args.session_id, None)?;
            return Ok(PromptResponse::new(StopReason::Cancelled));
        }

        if let Err(error) = Self::send_active_run_update(cx, &args.session_id, Some(&run_id)) {
            self.clear_active_run(&session_id, &run_id).await;
            return Err(error);
        }

        if let Err(error) = self
            .send_local_inference_progress_update(cx, &args.session_id, &session_id, &agent)
            .await
        {
            self.clear_active_run(&session_id, &run_id).await;
            let _ = Self::send_active_run_update(cx, &args.session_id, None);
            return Err(error);
        }

        let user_message = Self::convert_acp_prompt_to_message(&args.prompt);

        let message_text = user_message.as_concat_text();
        if let Some(parsed) = crate::agents::execute_commands::parse_slash_command(&message_text) {
            let full_command = format!("/{}", parsed.command);

            if !Self::is_builtin_agent_command(parsed.command) {
                if let Some(recipe_path) =
                    crate::slash_commands::recipe_slash_command::get_recipe_for_command(
                        &full_command,
                    )
                {
                    if recipe_path.exists() {
                        if let Err(error) = cx.send_notification(SessionNotification::new(
                            args.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                ContentBlock::Text(TextContent::new(format!(
                                    "Running recipe: {}",
                                    full_command
                                ))),
                            )),
                        )) {
                            self.clear_active_run(&session_id, &run_id).await;
                            let _ = Self::send_active_run_update(cx, &args.session_id, None);
                            return Err(error);
                        }
                    }
                }
            }
        }

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let mut stream = match agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                self.clear_active_run(&session_id, &run_id).await;
                let _ = Self::send_active_run_update(cx, &args.session_id, None);
                return Err(agent_client_protocol::Error::internal_error()
                    .data(format!("Error getting agent reply: {error}")));
            }
        };

        let mut was_cancelled = false;
        let mut tool_requests = HashMap::new();
        let mut chain_tracker = ToolChainTracker::default();
        let mut stream_error = None;

        while let Some(event) = stream.next().await {
            if cancel_token.is_cancelled() {
                was_cancelled = true;
                break;
            }

            match event {
                Ok(crate::agents::AgentEvent::Message(message)) => {
                    // Agent persists messages via session_manager.add_message() internally.
                    let stored_message_id = message.id.clone();

                    let sessions = self.sessions.lock().await;
                    if !sessions.contains_key(&session_id) {
                        stream_error = Some(
                            agent_client_protocol::Error::invalid_params()
                                .data(format!("Session not found: {}", session_id)),
                        );
                        break;
                    }

                    for content_item in &message.content {
                        if let Some(error) = prompt_error_from_message_content(content_item) {
                            stream_error = Some(error);
                            break;
                        }

                        if let MessageContent::ToolRequest(tool_request) = content_item {
                            tool_requests.insert(tool_request.id.clone(), tool_request.clone());
                        }

                        if let Err(error) = self
                            .handle_message_content(
                                content_item,
                                &args.session_id,
                                &session_id,
                                stored_message_id.as_deref(),
                                message.created,
                                &message.role,
                                message.metadata.steer,
                                &agent,
                                &tool_requests,
                                cx,
                            )
                            .await
                        {
                            stream_error = Some(error);
                            break;
                        }

                        let ready_chain = match content_item {
                            MessageContent::ToolRequest(tool_request) => {
                                if let Some(message_id) = stored_message_id.as_deref() {
                                    chain_tracker.record_request(
                                        tool_request.clone(),
                                        message_id.to_string(),
                                    );
                                }
                                None
                            }
                            MessageContent::ToolResponse(tool_response) => {
                                chain_tracker.record_response(&tool_response.id)
                            }
                            content if breaks_consecutive_tool_calls(content) => {
                                chain_tracker.close_current_chain()
                            }
                            _ => None,
                        };

                        if let Some(chain) = ready_chain {
                            self.spawn_ready_chain_summary(chain, &agent, &args.session_id, cx);
                        }
                    }
                    if stream_error.is_some() {
                        break;
                    }
                }
                Ok(crate::agents::AgentEvent::McpNotification((request_id, notification))) => {
                    if let Some(update) =
                        tool_notifications::tool_notification_update(request_id, notification)
                    {
                        let tool_call_notifier = ToolCallNotifier::new(cx, &args.session_id);
                        tool_call_notifier.send_update(update)?;
                    }
                }
                Ok(crate::agents::AgentEvent::MessageUsage { message_id, usage }) => {
                    if self.supports_goose_custom_notifications() {
                        cx.send_notification(GooseSessionNotification {
                            session_id: session_id.clone(),
                            update: GooseSessionUpdate::MessageUsage(message_usage_update(
                                message_id, &usage,
                            )),
                        })?;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    stream_error = Some(
                        agent_client_protocol::Error::internal_error()
                            .data(format!("Error in agent response stream: {}", e)),
                    );
                    break;
                }
            }
        }

        if !was_cancelled && stream_error.is_none() {
            if let Some(chain) = chain_tracker.close_current_chain() {
                self.spawn_ready_chain_summary(chain, &agent, &args.session_id, cx);
            }
        }
        self.clear_active_run(&session_id, &run_id).await;
        Self::send_active_run_update(cx, &args.session_id, None)?;
        if let Some(error) = stream_error {
            return Err(error);
        }

        let session = self
            .session_manager
            .get_session(&session_id, false)
            .await
            .internal_err_ctx("Failed to load session")?;
        let totals = self
            .session_manager
            .get_session_usage_totals(&session_id)
            .await
            .unwrap_or_default();
        if let Some(updates) = build_usage_updates(&session, &totals) {
            if self.supports_goose_custom_notifications() {
                cx.send_notification(updates.custom)?;
            }
            // Standard ACP notification — emitted alongside the custom one for
            // backwards compatibility. Remove once all known clients have
            // migrated to `_goose/unstable/session/update`.
            cx.send_notification(SessionNotification::new(
                args.session_id.clone(),
                SessionUpdate::UsageUpdate(updates.standard),
            ))?;
        }

        let stop_reason = if was_cancelled {
            StopReason::Cancelled
        } else {
            StopReason::EndTurn
        };

        let mut response = PromptResponse::new(stop_reason);
        if let Some(usage) = build_prompt_usage(&session) {
            response = response.usage(usage);
        }
        Ok(response)
    }

    async fn on_steer_session(
        &self,
        req: SteerSessionRequest,
    ) -> Result<SteerSessionResponse, agent_client_protocol::Error> {
        if req.prompt.is_empty() {
            return Err(
                agent_client_protocol::Error::invalid_params().data("prompt must not be empty")
            );
        }

        self.require_active_run(&req.session_id, &req.expected_run_id)
            .await?;
        let agent = self.get_session_agent(&req.session_id).await?;
        let active_run_id = self
            .require_active_run(&req.session_id, &req.expected_run_id)
            .await?;

        let message = Self::convert_acp_prompt_to_message(&req.prompt);
        if message.content.is_empty() {
            return Err(agent_client_protocol::Error::invalid_params()
                .data("prompt must contain steerable content"));
        }

        let message_id = format!("steer_{}", Uuid::new_v4());
        let message = message.with_id(message_id.clone());
        agent.steer(&req.session_id, message).await;

        if let Some(cx) = self.client_cx.get() {
            let _ = Self::send_queued_steer_update(
                cx,
                &SessionId::new(req.session_id.clone()),
                &message_id,
                &active_run_id,
            );
        }

        Ok(SteerSessionResponse {
            run_id: active_run_id,
            message_id,
        })
    }

    async fn on_cancel(
        &self,
        args: CancelNotification,
    ) -> Result<(), agent_client_protocol::Error> {
        debug!(?args, "cancel request");

        let session_id = args.session_id.0.to_string();
        let token = {
            let active_prompt_runs = self.active_prompt_runs.lock().await;
            active_prompt_runs
                .get(&session_id)
                .map(|active_run| active_run.cancel_token.clone())
        };

        if let Some(token) = token {
            info!(session_id = %session_id, "prompt cancelled");
            token.cancel();
        } else if !self.sessions.lock().await.contains_key(&session_id) {
            warn!(session_id = %session_id, "cancel request for unknown session");
        }

        Ok(())
    }

    async fn on_set_model(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> Result<(), agent_client_protocol::Error> {
        let agent = self.get_session_agent(session_id).await?;
        let current_provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let provider_name = current_provider.get_name().to_string();
        let current_model_config = agent
            .model_config_for_session(session_id)
            .await
            .internal_err_ctx("Failed to resolve model config")?;
        let model_config =
            crate::model_config::model_config_from_user_config_with_session_settings(
                &provider_name,
                model_id,
                Some(&current_model_config),
                None,
                None,
            )
            .invalid_params_err_ctx("Invalid model config")?;
        agent
            .recreate_provider_for_session(session_id, &provider_name, model_config)
            .await
            .internal_err_ctx("Failed to recreate provider")?;
        // model_config is already updated on the session by the agent's update_provider call.
        Ok(())
    }

    async fn build_config_update(
        &self,
        session_id: &SessionId,
    ) -> Result<(SessionNotification, Vec<SessionConfigOption>), agent_client_protocol::Error> {
        let session = self
            .session_manager
            .get_session(&session_id.0, false)
            .await
            .internal_err()?;
        let agent = self.get_session_agent(&session_id.0).await?;
        let provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let provider_name = provider.get_name().to_string();
        let current_model_config = agent
            .model_config_for_session(&session_id.0)
            .await
            .internal_err_ctx("Failed to resolve model config")?;
        let current_model = current_model_config.model_name.clone();
        let goose_mode = agent.goose_mode().await;
        let inventory = self
            .provider_inventory
            .entry_for_provider(&provider_name)
            .await
            .internal_err()?;
        let Some(inventory) = inventory else {
            return Err(agent_client_protocol::Error::internal_error()
                .data(format!("Unknown provider inventory: {}", provider_name)));
        };
        let model_state = build_model_state(current_model.as_str(), &inventory);
        let mode_state = build_mode_state(goose_mode)?;
        let provider_options = build_provider_options(Some(&provider_name)).await;
        let config_options = build_config_options(
            &mode_state,
            &model_state,
            &current_model_config,
            session_provider_selection(&session),
            provider_options,
        );
        let notification = SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ConfigOptionUpdate(ConfigOptionUpdate::new(config_options.clone())),
        );
        Ok((notification, config_options))
    }

    async fn on_set_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> Result<SetSessionModeResponse, agent_client_protocol::Error> {
        let mode = mode_id.parse::<GooseMode>().map_err(|_| {
            agent_client_protocol::Error::invalid_params()
                .data(format!("Invalid mode: {}", mode_id))
        })?;

        let agent = self.get_session_agent(session_id).await?;
        agent
            .update_goose_mode(mode, session_id)
            .await
            .internal_err_ctx("Failed to update mode")?;

        // goose_mode is already updated on the session above.

        Ok(SetSessionModeResponse::new())
    }

    async fn on_set_thinking_effort(
        &self,
        session_id: &str,
        effort_id: &str,
    ) -> Result<(), agent_client_protocol::Error> {
        let effort = effort_id
            .parse::<goose_providers::thinking::ThinkingEffort>()
            .map_err(|_| {
                agent_client_protocol::Error::invalid_params()
                    .data(format!("Invalid thinking effort: {}", effort_id))
            })?;
        let agent = self.get_session_agent(session_id).await?;
        agent
            .update_thinking_effort(session_id, effort)
            .await
            .internal_err_ctx("Failed to update thinking effort")?;

        Ok(())
    }

    async fn update_provider(
        &self,
        session_id: &str,
        provider_name: &str,
        model_name: Option<&str>,
        context_limit: Option<usize>,
        request_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<(), agent_client_protocol::Error> {
        let config = self.config()?;
        let agent = self.get_session_agent(session_id).await?;
        let current_provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let current_provider_name = current_provider.get_name();
        let current_model_config = agent
            .model_config_for_session(session_id)
            .await
            .internal_err_ctx("Failed to resolve model config")?;
        let current_model = current_model_config.model_name.clone();
        let use_default_provider = provider_name == DEFAULT_PROVIDER_ID;
        let resolved_provider_name = if use_default_provider {
            config
                .get_goose_provider()
                .internal_err_ctx("Failed to resolve default provider from config")?
        } else {
            provider_name.to_string()
        };
        let is_changing_provider = resolved_provider_name != current_provider_name;
        let default_model = if let Some(model_name) = model_name {
            model_name.to_string()
        } else if use_default_provider {
            config
                .get_goose_model()
                .internal_err_ctx("Failed to resolve default model from config")?
        } else if is_changing_provider {
            crate::providers::get_from_registry(&resolved_provider_name)
                .await
                .ok()
                .map(|entry| entry.metadata().default_model.clone())
                .unwrap_or(ACP_CURRENT_MODEL.to_string())
        } else {
            current_model
        };
        let model = model_name.unwrap_or(&default_model);
        let model_config =
            crate::model_config::model_config_from_user_config_with_session_settings(
                &resolved_provider_name,
                model,
                Some(&current_model_config),
                request_params,
                context_limit,
            )
            .invalid_params_err_ctx("Invalid model config")?;

        agent
            .recreate_provider_for_session(session_id, &resolved_provider_name, model_config)
            .await
            .internal_err_ctx("Failed to recreate provider")?;

        // provider_name is already updated on the session by the agent's update_provider call.
        Ok(())
    }

    async fn on_fork_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: ForkSessionRequest,
    ) -> Result<ForkSessionResponse, agent_client_protocol::Error> {
        self.handle_fork_session(cx, args).await
    }

    async fn on_close_session(
        &self,
        session_id: &str,
    ) -> Result<CloseSessionResponse, agent_client_protocol::Error> {
        self.closed_session_ids
            .lock()
            .await
            .insert(session_id.to_string());

        let active_run_token = {
            let active_prompt_runs = self.active_prompt_runs.lock().await;
            active_prompt_runs
                .get(session_id)
                .map(|active_run| active_run.cancel_token.clone())
        };

        if let Some(token) = active_run_token {
            token.cancel();
        }

        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id);
        drop(sessions);

        self.agent_manager
            .remove_session_if_loaded(session_id)
            .await
            .internal_err_ctx("Failed to remove in-memory agent")?;

        info!(session_id = %session_id, "ACP session closed");
        Ok(CloseSessionResponse::new())
    }
}

pub struct GooseAcpHandler {
    pub agent: Arc<GooseAcpAgent>,
}

pub fn serve<R, W>(
    agent: Arc<GooseAcpAgent>,
    read: R,
    write: W,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
where
    R: futures::AsyncRead + Unpin + Send + 'static,
    W: futures::AsyncWrite + Unpin + Send + 'static,
{
    Box::pin(async move {
        let handler = GooseAcpHandler { agent };

        SacpAgent
            .builder()
            .name("goose-acp")
            .with_handler(handler)
            .connect_to(ByteStreams::new(write, read))
            .await?;

        Ok(())
    })
}

/// A lazily-initialized agent connection used by the HTTP/WebSocket transport.
///
/// The `agent-client-protocol-http` server takes a synchronous factory that
/// yields a [`ConnectTo<Client>`] per connection, but creating a goose agent is
/// async. Agent creation is therefore deferred into [`ConnectTo::connect_to`],
/// which runs as the connection's serving future.
pub struct GooseAgentConnection {
    server: Arc<crate::acp::server_factory::AcpServer>,
}

impl GooseAgentConnection {
    pub fn new(server: Arc<crate::acp::server_factory::AcpServer>) -> Self {
        Self { server }
    }
}

impl agent_client_protocol::ConnectTo<Client> for GooseAgentConnection {
    async fn connect_to(
        self,
        client: impl agent_client_protocol::ConnectTo<SacpAgent>,
    ) -> std::result::Result<(), agent_client_protocol::Error> {
        let agent = self.server.create_agent().await.internal_err()?;
        let handler = GooseAcpHandler { agent };
        SacpAgent
            .builder()
            .name("goose-acp")
            .with_handler(handler)
            .connect_to(client)
            .await
    }
}

pub async fn run(builtins: Vec<String>) -> Result<()> {
    info!("listening on stdio");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let server = crate::acp::server_factory::AcpServer::new(
        crate::acp::server_factory::AcpServerFactoryConfig {
            builtins,
            data_dir: Paths::data_dir(),
            config_dir: Paths::config_dir(),
            goose_platform: GoosePlatform::GooseCli,
            additional_source_roots: Vec::new(),
        },
    );
    let agent = server.create_agent().await?;
    serve(agent, incoming, outgoing).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::session_manager::SessionType;
    use agent_client_protocol::schema::v1::{
        EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
        PermissionOptionId, ResourceLink, SelectedPermissionOutcome,
    };
    use goose_providers::conversation::token_usage::Usage as TokenUsage;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;
    use test_case::test_case;

    #[test_case(
        McpServer::Stdio(
            McpServerStdio::new("github", "/path/to/github-mcp-server")
                .args(vec!["stdio".into()])
                .env(vec![EnvVariable::new("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_xxxxxxxxxxxx")])
        ),
        Ok(ExtensionConfig::Stdio {
            name: "github".into(),
            description: String::new(),
            cmd: "/path/to/github-mcp-server".into(),
            args: vec!["stdio".into()],
            envs: Envs::new(
                [(
                    "GITHUB_PERSONAL_ACCESS_TOKEN".into(),
                    "ghp_xxxxxxxxxxxx".into()
                )]
                .into()
            ),
            env_keys: vec![],
            timeout: None,
            cwd: None,
            bundled: Some(false),
            available_tools: vec![],
        })
    )]
    #[test_case(
        McpServer::Http(
            McpServerHttp::new("github", "https://api.githubcopilot.com/mcp/")
                .headers(vec![HttpHeader::new("Authorization", "Bearer ghp_xxxxxxxxxxxx")])
        ),
        Ok(ExtensionConfig::StreamableHttp {
            name: "github".into(),
            description: String::new(),
            uri: "https://api.githubcopilot.com/mcp/".into(),
            envs: Envs::default(),
            env_keys: vec![],
            headers: HashMap::from([(
                "Authorization".into(),
                "Bearer ghp_xxxxxxxxxxxx".into()
            )]),
            timeout: None,
            socket: None,
            bundled: Some(false),
            available_tools: vec![],
        })
    )]
    #[test_case(
        McpServer::Sse(McpServerSse::new("test-sse", "https://agent-fin.biodnd.com/sse")),
        Err("SSE is unsupported, migrate to streamable_http".to_string())
    )]
    fn test_mcp_server_to_extension_config(
        input: McpServer,
        expected: Result<ExtensionConfig, String>,
    ) {
        assert_eq!(mcp_server_to_extension_config(input), expected);
    }

    fn new_resource_link(content: &str) -> anyhow::Result<(ResourceLink, NamedTempFile)> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;

        let name = file
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let uri = format!("file://{}", file.path().to_str().unwrap());
        let link = ResourceLink::new(name, uri);
        Ok((link, file))
    }

    #[test]
    fn test_read_resource_link_non_file_scheme() {
        let (link, file) = new_resource_link("print(\"hello, world\")").unwrap();

        let result = read_resource_link(link).unwrap();
        let expected = format!(
            "

# {}
```
print(\"hello, world\")
```",
            file.path().to_str().unwrap(),
        );

        assert_eq!(result, expected,)
    }

    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("allow_once".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AllowOnce };
        "allow_once_maps_to_allow_once"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("allow_always".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AlwaysAllow };
        "allow_always_maps_to_always_allow"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("reject_once".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::DenyOnce };
        "reject_once_maps_to_deny_once"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("reject_always".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AlwaysDeny };
        "reject_always_maps_to_always_deny"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("unknown".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::Cancel };
        "unknown_option_maps_to_cancel"
    )]
    #[test_case(
        RequestPermissionOutcome::Cancelled,
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::Cancel };
        "cancelled_maps_to_cancel"
    )]
    fn test_outcome_to_confirmation(
        input: RequestPermissionOutcome,
        expected: PermissionConfirmation,
    ) {
        assert_eq!(outcome_to_confirmation(&input), expected);
    }

    #[test]
    fn test_message_update_meta_includes_created_and_message_id() {
        let meta = message_update_meta(Some("msg_live"), 1_700_000_000, false);

        assert_eq!(
            meta.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_live",
            })),
        );
    }

    #[test]
    fn test_credits_exhausted_system_notification_maps_to_prompt_error() {
        let content = MessageContent::SystemNotification(SystemNotificationContent {
            notification_type: SystemNotificationType::CreditsExhausted,
            msg: "Please add credits to your account, then resend your message to continue."
                .to_string(),
            data: Some(serde_json::json!({
                "top_up_url": "https://router.tetrate.ai/billing"
            })),
        });

        let error = prompt_error_from_message_content(&content).expect("expected prompt error");
        let value = serde_json::to_value(error).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "code": -32603,
                "message": "Please add credits to your account, then resend your message to continue.",
                "data": {
                    "reason": "credits_exhausted",
                    "url": "https://router.tetrate.ai/billing"
                }
            })
        );
    }

    #[test]
    fn test_non_credit_system_notification_does_not_map_to_prompt_error() {
        let content = MessageContent::SystemNotification(SystemNotificationContent {
            notification_type: SystemNotificationType::InlineMessage,
            msg: "Compaction complete".to_string(),
            data: None,
        });

        assert!(prompt_error_from_message_content(&content).is_none());
    }

    fn make_session_with_usage(usage: TokenUsage, accumulated_usage: TokenUsage) -> Session {
        Session {
            id: "session-1".to_string(),
            working_dir: PathBuf::from("/tmp"),
            name: "ACP Session".to_string(),
            session_type: SessionType::Acp,
            usage,
            accumulated_usage,
            ..Default::default()
        }
    }

    #[test]
    fn test_build_prompt_usage_uses_current_turn_tokens() {
        let session = make_session_with_usage(
            TokenUsage::new(Some(80), Some(40), Some(120)),
            TokenUsage::new(Some(210), Some(150), Some(360)),
        );
        let usage = build_prompt_usage(&session).expect("usage should be present");
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.input_tokens, 80);
        assert_eq!(usage.output_tokens, 40);
    }

    #[test]
    fn test_build_prompt_usage_falls_back_to_current_tokens() {
        let session = make_session_with_usage(
            TokenUsage::new(Some(80), Some(40), Some(120)),
            TokenUsage::default(),
        );
        let usage = build_prompt_usage(&session).expect("usage should be present");
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.input_tokens, 80);
        assert_eq!(usage.output_tokens, 40);
    }

    #[test]
    fn test_build_prompt_usage_requires_total_tokens() {
        let session = make_session_with_usage(
            TokenUsage {
                input_tokens: Some(80),
                output_tokens: Some(40),
                total_tokens: None,
                ..Default::default()
            },
            TokenUsage::default(),
        );
        assert!(build_prompt_usage(&session).is_none());
    }

    #[test]
    fn test_build_usage_update_clamps_negative_used_to_zero() {
        let mut session = make_session_with_usage(
            TokenUsage::new(Some(0), Some(0), Some(-7)),
            TokenUsage::default(),
        );
        session.model_config = Some(
            goose_providers::model::ModelConfig::new("test-model")
                .with_context_limit(Some(258_000)),
        );
        let totals = SessionUsageTotals {
            accumulated_usage: session.accumulated_usage,
            accumulated_cost: session.accumulated_cost,
        };
        let updates =
            build_usage_updates(&session, &totals).expect("usage updates should be present");
        assert_eq!(updates.custom.session_id, "session-1");
        let usage = match updates.custom.update {
            GooseSessionUpdate::UsageUpdate(usage) => usage,
            other => panic!("expected usage update, got {other:?}"),
        };
        assert_eq!(usage.used, 0);
        assert_eq!(usage.context_limit, 258_000);
        assert_eq!(updates.standard.used, 0);
        assert_eq!(updates.standard.size, 258_000);
    }

    #[test]
    fn test_build_usage_update_requires_model_config() {
        let session = make_session_with_usage(
            TokenUsage::new(Some(80), Some(40), Some(120)),
            TokenUsage::default(),
        );
        assert!(build_usage_updates(&session, &SessionUsageTotals::default()).is_none());
    }

    #[test]
    fn test_goose_custom_notifications_capability_defaults_to_false() {
        let request =
            InitializeRequest::new(agent_client_protocol::schema::ProtocolVersion::LATEST);
        let goose_client_capabilities =
            extract_client_capabilities_meta(&request).and_then(|meta| meta.goose);

        assert!(!extract_client_supports_goose_custom_notifications(
            goose_client_capabilities.as_ref()
        ));
    }

    #[test]
    fn test_goose_custom_notifications_capability_reads_client_meta() {
        let mut goose_meta = serde_json::Map::new();
        goose_meta.insert(
            "customNotifications".to_string(),
            serde_json::Value::Bool(true),
        );
        let mut meta = serde_json::Map::new();
        meta.insert("goose".to_string(), serde_json::Value::Object(goose_meta));

        let request =
            InitializeRequest::new(agent_client_protocol::schema::ProtocolVersion::LATEST)
                .client_capabilities(
                    agent_client_protocol::schema::v1::ClientCapabilities::new().meta(meta),
                );
        let goose_client_capabilities =
            extract_client_capabilities_meta(&request).and_then(|meta| meta.goose);

        assert!(extract_client_supports_goose_custom_notifications(
            goose_client_capabilities.as_ref()
        ));
    }

    #[test]
    fn test_tool_call_label_enrichment_capability() {
        let request =
            InitializeRequest::new(agent_client_protocol::schema::ProtocolVersion::LATEST);
        let goose_client_capabilities =
            extract_client_capabilities_meta(&request).and_then(|meta| meta.goose);
        assert!(!goose_client_capabilities
            .and_then(|goose| goose.tool_call_label_enrichment)
            .unwrap_or(false));

        let mut goose_meta = serde_json::Map::new();
        goose_meta.insert(
            "toolCallLabelEnrichment".to_string(),
            serde_json::Value::Bool(true),
        );
        let mut meta = serde_json::Map::new();
        meta.insert("goose".to_string(), serde_json::Value::Object(goose_meta));
        let request =
            InitializeRequest::new(agent_client_protocol::schema::ProtocolVersion::LATEST)
                .client_capabilities(
                    agent_client_protocol::schema::v1::ClientCapabilities::new().meta(meta),
                );
        let goose_client_capabilities =
            extract_client_capabilities_meta(&request).and_then(|meta| meta.goose);
        assert!(goose_client_capabilities
            .and_then(|goose| goose.tool_call_label_enrichment)
            .unwrap_or(false));
    }
}
