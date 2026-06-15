use crate::acp::custom_notifications::*;
use crate::acp::custom_requests::*;
use crate::acp::fs::AcpTools;
pub(super) use crate::acp::response_builder::{
    build_config_options, build_mode_state, build_model_state, build_provider_options,
    build_session_info, build_session_setup_config, send_session_setup_notifications, session_meta,
    session_provider_selection, should_refresh_inventory_for_session_init,
};
use crate::acp::tools::AcpAwareToolMeta;
use crate::acp::{PermissionDecision, ACP_CURRENT_MODEL};
use crate::action_required_manager::ActionRequiredManager;
use crate::agents::extension::{Envs, PLATFORM_EXTENSIONS};
use crate::agents::extension_manager::TRUSTED_TOOL_UPDATE_META_KEY;
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
    ToolRequest,
};
use crate::execution::manager::{AgentManager, AgentManagerGetResult, RuntimeContext};
use crate::mcp_utils::ToolResult;
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::Provider;
use crate::providers::inventory::{
    ProviderInventoryEntry, ProviderInventoryService, RefreshJobPlan, RefreshPlan,
    RefreshSkipReason,
};
use crate::session::{
    EnabledExtensionsState, ExtensionData, ExtensionState, Session, SessionManager,
};
use crate::source_roots::SourceRoot;
use crate::utils::sanitize_unicode_tags;
use agent_client_protocol::schema::{
    AgentCapabilities, Annotations, AuthMethod, AuthMethodAgent, AuthenticateRequest,
    AuthenticateResponse, BlobResourceContents, CancelNotification, CloseSessionRequest,
    CloseSessionResponse, ConfigOptionUpdate, Content, ContentBlock, ContentChunk,
    CurrentModeUpdate, EmbeddedResource, EmbeddedResourceResource, FileSystemCapabilities,
    ForkSessionRequest, ForkSessionResponse, ImageContent, InitializeRequest, InitializeResponse,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    McpCapabilities, McpServer, Meta, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, PromptCapabilities, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, ResourceLink, SessionCapabilities,
    SessionCloseCapabilities, SessionConfigOption, SessionId, SessionInfoUpdate,
    SessionListCapabilities, SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse,
    SetSessionModelRequest, SetSessionModelResponse, StopReason, TextContent, TextResourceContents,
    ToolCall, ToolCallContent, ToolCallId, ToolCallLocation, ToolCallStatus, ToolCallUpdate,
    ToolCallUpdateFields, ToolKind, Usage, UsageUpdate,
};
use agent_client_protocol::util::MatchDispatchFrom;
use agent_client_protocol::{
    Agent as SacpAgent, ByteStreams, Client, ConnectionTo, Dispatch, HandleDispatchFrom, Handled,
    Responder,
};
use anyhow::Result;
use fs_err as fs;
use futures::future::BoxFuture;
use futures::stream::{self, StreamExt};
use futures::FutureExt;
use rmcp::model::{
    AnnotateAble, CallToolResult, RawContent, RawTextContent, ResourceContents, Role,
};
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

mod config;
mod custom_dispatch;
mod dictation;
mod dispatch;
mod extensions;
mod fork_session;
mod list_sessions;
mod load_session;
mod manage_sessions;
mod new_session;
mod onboarding;
mod providers;
mod resources;
mod sources;
mod tools;

pub type AcpProviderFactory = Arc<
    dyn Fn(
            String,
            crate::model::ModelConfig,
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
    tool_requests: HashMap<String, crate::conversation::message::ToolRequest>,
    /// For each tool_call_id that belongs to a multi-tool chain (run of
    /// consecutive ToolRequest blocks within one assistant message), the chain
    /// it belongs to. Populated when the assistant message is processed.
    /// Used by `handle_tool_response` to detect when a chain has fully
    /// completed and fire a single LLM summary covering the run.
    chain_membership: HashMap<String, Arc<ToolChain>>,
    /// Set of tool_call_ids whose ToolResponse has already been processed.
    /// Drives the "all responses present" check for chain completion.
    responded_tool_ids: HashSet<String>,
    /// Tool_call_ids of chains that have already had a summary task fired.
    /// Idempotence guard so we summarize each chain at most once.
    summarized_chains: HashSet<String>,
    cancel_token: Option<CancellationToken>,
    active_run_id: Option<String>,
}

/// A run of consecutive ToolRequest blocks within one assistant message,
/// tracked by [`GooseAcpSession::chain_membership`]. Used to drive a single
/// LLM summary for the whole run once every step has a recorded ToolResponse.
#[derive(Debug, Clone)]
struct ToolChain {
    /// Tool call ids in document order. Always `len() >= 2`.
    ids: Vec<String>,
    /// The message_id of the assistant message containing these tool calls.
    /// Used to persist chain summaries back to the messages table.
    message_id: String,
}

pub struct GooseAcpAgentOptions {
    pub provider_factory: AcpProviderFactory,
    pub builtins: Vec<String>,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub disable_session_naming: bool,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
}

pub struct GooseAcpAgent {
    sessions: Arc<Mutex<HashMap<String, GooseAcpSession>>>,
    agent_manager: Arc<AgentManager>,
    provider_factory: AcpProviderFactory,
    builtins: Vec<String>,
    client_fs_capabilities: OnceCell<FileSystemCapabilities>,
    client_terminal: OnceCell<bool>,
    client_mcp_host_info: OnceCell<GooseMcpHostInfo>,
    client_supports_goose_custom_notifications: OnceCell<bool>,
    use_login_shell_path: OnceCell<bool>,
    client_cx: OnceCell<ConnectionTo<Client>>,
    config_dir: std::path::PathBuf,
    session_manager: Arc<SessionManager>,
    permission_manager: Arc<PermissionManager>,
    disable_session_naming: bool,
    provider_inventory: ProviderInventoryService,
    additional_source_roots: Vec<SourceRoot>,
}

/// Shorten a session/thread id for perf log correlation.
/// All `perf:` logs use `sid=<8-char-prefix>` so a single session's activity
/// can be extracted with `grep 'perf:' <log> | grep 'sid=abc12345'`.
pub(super) fn sid_short(id: &str) -> String {
    id.chars().take(8).collect()
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
) -> Result<(String, crate::model::ModelConfig), agent_client_protocol::Error> {
    let resolved_provider = config.get_goose_provider().map_err(|error| {
        agent_client_protocol::Error::internal_error()
            .data(format!("Failed to resolve provider: {}", error))
    })?;
    let resolved_model = config.get_goose_model().map_err(|error| {
        agent_client_protocol::Error::internal_error()
            .data(format!("Failed to resolve model: {}", error))
    })?;
    let resolved_model_config = crate::model::ModelConfig::new(&resolved_model)
        .map(|model_config| model_config.with_canonical_limits(&resolved_provider))
        .map_err(|error| {
            agent_client_protocol::Error::internal_error()
                .data(format!("Failed to resolve model: {}", error))
        })?;
    Ok((resolved_provider, resolved_model_config))
}

async fn resolve_provider_default_model_config(
    provider_name: &str,
) -> Result<crate::model::ModelConfig, agent_client_protocol::Error> {
    let entry = crate::providers::get_from_registry(provider_name)
        .await
        .map_err(|error| {
            agent_client_protocol::Error::invalid_params()
                .data(format!("Unknown provider '{}': {}", provider_name, error))
        })?;
    crate::model::ModelConfig::new(&entry.metadata().default_model)
        .map(|model_config| model_config.with_canonical_limits(provider_name))
        .map_err(|error| {
            agent_client_protocol::Error::internal_error()
                .data(format!("Failed to resolve model: {}", error))
        })
}

fn get_requested_line(arguments: Option<&rmcp::model::JsonObject>) -> Option<u32> {
    arguments
        .and_then(|args| args.get("line"))
        .and_then(|v| v.as_u64())
        .map(|l| l as u32)
}

fn is_developer_file_tool(tool_name: &str) -> bool {
    matches!(tool_name, "read" | "write" | "edit")
}

fn extract_locations_from_meta(
    tool_response: &crate::conversation::message::ToolResponse,
) -> Option<Vec<ToolCallLocation>> {
    let result = tool_response.tool_result.as_ref().ok()?;
    let meta = result.meta.as_ref()?;
    let locations_val = meta.get("tool_locations")?;
    let entries: Vec<serde_json::Value> = serde_json::from_value(locations_val.clone()).ok()?;
    let locations = entries
        .into_iter()
        .filter_map(|entry| {
            let path = entry.get("path")?.as_str()?;
            let line = entry.get("line").and_then(|v| v.as_u64()).map(|l| l as u32);
            Some(ToolCallLocation::new(path).line(line))
        })
        .collect::<Vec<_>>();
    if locations.is_empty() {
        None
    } else {
        Some(locations)
    }
}

fn extract_tool_locations(
    tool_request: &crate::conversation::message::ToolRequest,
    tool_response: &crate::conversation::message::ToolResponse,
) -> Vec<ToolCallLocation> {
    let mut locations = Vec::new();

    if let Ok(tool_call) = &tool_request.tool_call {
        if !is_developer_file_tool(tool_call.name.as_ref()) {
            return locations;
        }

        let tool_name = tool_call.name.as_ref();
        let path_str = tool_call
            .arguments
            .as_ref()
            .and_then(|args| args.get("path"))
            .and_then(|p| p.as_str());

        if let Some(path_str) = path_str {
            if matches!(tool_name, "read") {
                let line = get_requested_line(tool_call.arguments.as_ref());
                locations.push(ToolCallLocation::new(path_str).line(line));
                return locations;
            }

            if matches!(tool_name, "write" | "edit") {
                locations.push(ToolCallLocation::new(path_str).line(1));
                return locations;
            }

            let command = tool_call
                .arguments
                .as_ref()
                .and_then(|args| args.get("command"))
                .and_then(|c| c.as_str());

            if let Ok(result) = &tool_response.tool_result {
                for content in &result.content {
                    if let RawContent::Text(text_content) = &content.raw {
                        let text = &text_content.text;

                        match command {
                            Some("view") => {
                                let line = extract_view_line_range(text)
                                    .map(|range| range.0 as u32)
                                    .or(Some(1));
                                locations.push(ToolCallLocation::new(path_str).line(line));
                            }
                            Some("str_replace") | Some("insert") => {
                                let line = extract_first_line_number(text)
                                    .map(|l| l as u32)
                                    .or(Some(1));
                                locations.push(ToolCallLocation::new(path_str).line(line));
                            }
                            Some("write") => {
                                locations.push(ToolCallLocation::new(path_str).line(1));
                            }
                            _ => {
                                locations.push(ToolCallLocation::new(path_str).line(1));
                            }
                        }
                        break;
                    }
                }
            }

            if locations.is_empty() {
                locations.push(ToolCallLocation::new(path_str).line(1));
            }
        }
    }

    locations
}

fn extract_view_line_range(text: &str) -> Option<(usize, usize)> {
    let re = regex::Regex::new(r"\(lines (\d+)-(\d+|end)\)").ok()?;
    if let Some(caps) = re.captures(text) {
        let start = caps.get(1)?.as_str().parse::<usize>().ok()?;
        let end = if caps.get(2)?.as_str() == "end" {
            start
        } else {
            caps.get(2)?.as_str().parse::<usize>().ok()?
        };
        return Some((start, end));
    }
    None
}

fn extract_first_line_number(text: &str) -> Option<usize> {
    let re = regex::Regex::new(r"```[^\n]*\n(\d+):").ok()?;
    if let Some(caps) = re.captures(text) {
        return caps.get(1)?.as_str().parse::<usize>().ok();
    }
    None
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

fn format_tool_name(tool_name: &str) -> String {
    if let Some((extension, tool)) = tool_name.split_once("__") {
        format!(
            "{}: {}",
            extension.replace('_', " "),
            tool.replace('_', " ")
        )
    } else {
        tool_name.replace('_', " ")
    }
}

/// Build a short fallback title from the tool name and arguments by extracting
/// the most useful value (file path, command, query, url, etc.).
fn summarize_tool_call(tool_name: &str, arguments: Option<&serde_json::Value>) -> String {
    let base = format_tool_name(tool_name);

    let detail = arguments.and_then(|args| {
        let obj = args.as_object()?;
        let keys = [
            "path", "file", "command", "query", "url", "uri", "name", "pattern", "source",
        ];
        for key in &keys {
            if let Some(v) = obj.get(*key) {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                if !s.is_empty() {
                    let first_line = s.lines().next().unwrap_or(&s);
                    if first_line.len() > 60 {
                        return Some(format!("{}…", crate::utils::safe_truncate(first_line, 57)));
                    }
                    return Some(first_line.to_string());
                }
            }
        }
        None
    });

    match detail {
        Some(d) => format!("{base} · {d}"),
        None => base,
    }
}

fn tool_call_identity_meta(tool_request: &ToolRequest) -> Option<Meta> {
    let tool_call = tool_request.tool_call.as_ref().ok()?;
    let tool_name = tool_call.name.to_string();
    let extension_name = tool_request
        .tool_meta
        .as_ref()
        .and_then(|meta| meta.get("goose_extension"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            tool_name
                .split_once("__")
                .map(|(extension_name, _)| extension_name.to_string())
        });

    let mut tool_call_meta = serde_json::Map::new();
    tool_call_meta.insert("toolName".to_string(), serde_json::Value::String(tool_name));
    if let Some(extension_name) = extension_name {
        tool_call_meta.insert(
            "extensionName".to_string(),
            serde_json::Value::String(extension_name),
        );
    }

    let mut goose_meta = serde_json::Map::new();
    goose_meta.insert(
        "toolCall".to_string(),
        serde_json::Value::Object(tool_call_meta),
    );

    let mut meta = serde_json::Map::new();
    meta.insert("goose".to_string(), serde_json::Value::Object(goose_meta));
    Some(meta)
}

/// Add `goose.toolChainSummary = { summary, count }` to a `Meta` blob,
/// preserving any existing `goose.*` keys (e.g. `goose.toolCall` set by
/// [`tool_call_identity_meta`]).
fn with_tool_chain_summary_meta(base: Option<Meta>, summary: &str, count: usize) -> Option<Meta> {
    let mut meta = base.unwrap_or_default();
    let goose_entry = meta
        .entry("goose".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let goose_obj = match goose_entry {
        serde_json::Value::Object(obj) => obj,
        other => {
            *other = serde_json::Value::Object(serde_json::Map::new());
            match other {
                serde_json::Value::Object(obj) => obj,
                _ => unreachable!(),
            }
        }
    };
    let mut chain = serde_json::Map::new();
    chain.insert(
        "summary".to_string(),
        serde_json::Value::String(summary.to_string()),
    );
    chain.insert(
        "count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(count)),
    );
    goose_obj.insert(
        "toolChainSummary".to_string(),
        serde_json::Value::Object(chain),
    );
    Some(meta)
}

struct PendingToolCall {
    tool_call: ToolCall,
    identity_meta: Option<Meta>,
    fallback_title: String,
}

/// If `buffer` holds a multi-tool run (≥ 2 tool requests), (re)register a
/// [`ToolChain`] in `chain_membership` anchored on the **first** tool's
/// message_id (the row [`SessionManager::update_tool_request_meta`] will patch
/// when persisting the LLM-generated summary). Does **not** clear the buffer
/// — chains can grow as more tools arrive (sequential tool use), so callers
/// keep accumulating and re-registering with the larger set of ids.
///
/// The buffer contains `(tool_call_id, message_id)` pairs in arrival order,
/// fed by the prompt stream loop. Sequential tool use (Bedrock/Anthropic)
/// interleaves request → response → request → response across separate
/// `AgentEvent::Message` events, so a per-event view would only see length-1
/// chains and miss the run. Tool responses are chain-neutral (they don't
/// split the run); only non-tool content (text, thinking, image, etc.) does,
/// matching the frontend's `groupContentSections` behavior.
fn extend_chain_membership(
    buffer: &[(String, String)],
    chain_membership: &mut HashMap<String, Arc<ToolChain>>,
) {
    if buffer.len() >= 2 {
        let ids: Vec<String> = buffer.iter().map(|(id, _)| id.clone()).collect();
        let message_id = buffer[0].1.clone();
        let chain = Arc::new(ToolChain {
            ids: ids.clone(),
            message_id,
        });
        for id in ids {
            chain_membership.insert(id, chain.clone());
        }
    }
}

fn pending_tool_call_from_request(tool_request: &ToolRequest) -> PendingToolCall {
    let tool_name = match &tool_request.tool_call {
        Ok(tool_call) => tool_call.name.to_string(),
        Err(_) => "error".to_string(),
    };
    let args_value = tool_request
        .tool_call
        .as_ref()
        .ok()
        .and_then(|tc| tc.arguments.as_ref())
        .map(|a| serde_json::Value::Object(a.clone()));
    let fallback_title = summarize_tool_call(&tool_name, args_value.as_ref());
    let identity_meta = tool_call_identity_meta(tool_request);

    // Prefer the persisted LLM-generated title when available so replay (and
    // any subsequent live initial ToolCall after the title task has already
    // resolved) emits the nice title up front, with no flash of the
    // deterministic fallback.
    let initial_title = tool_request
        .persisted_title()
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback_title.clone());

    let mut tool_call = ToolCall::new(ToolCallId::new(tool_request.id.clone()), initial_title)
        .status(ToolCallStatus::Pending);
    if let Some(args) = args_value {
        tool_call = tool_call.raw_input(args);
    }

    PendingToolCall {
        tool_call,
        identity_meta,
        fallback_title,
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
    let total = to_nonnegative_u64(session.total_tokens)?;
    let input = to_nonnegative_u64(session.input_tokens).unwrap_or(0);
    let output = to_nonnegative_u64(session.output_tokens).unwrap_or(0);
    Some(Usage::new(total, input, output))
}

pub(super) struct UsageUpdates {
    pub(super) custom: GooseSessionNotification,
    pub(super) standard: UsageUpdate,
}

pub(super) fn build_usage_updates(session: &Session) -> Option<UsageUpdates> {
    let used = session.total_tokens.unwrap_or(0).max(0) as u64;
    let ctx_limit = session.model_config.as_ref()?.context_limit() as u64;
    let accumulated_input_tokens =
        to_nonnegative_u64(session.accumulated_input_tokens).unwrap_or(0);
    let accumulated_output_tokens =
        to_nonnegative_u64(session.accumulated_output_tokens).unwrap_or(0);
    Some(UsageUpdates {
        custom: GooseSessionNotification {
            session_id: session.id.clone(),
            update: GooseSessionUpdate::UsageUpdate(SessionUsageUpdate {
                used,
                context_limit: ctx_limit,
                accumulated_input_tokens,
                accumulated_output_tokens,
                accumulated_cost: session.accumulated_cost,
            }),
        },
        standard: UsageUpdate::new(used, ctx_limit),
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
            None,
            Config::global().get_goose_mode().unwrap_or_default(),
            options.disable_session_naming,
            options.goose_platform.clone(),
        );
        let agent_manager = Arc::new(AgentManager::new(agent_config, None).await?);

        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            agent_manager,
            provider_factory: options.provider_factory,
            builtins: options.builtins,
            client_fs_capabilities: OnceCell::new(),
            client_terminal: OnceCell::new(),
            client_mcp_host_info: OnceCell::new(),
            client_supports_goose_custom_notifications: OnceCell::new(),
            use_login_shell_path: OnceCell::new(),
            client_cx: OnceCell::new(),
            config_dir: options.config_dir,
            session_manager,
            permission_manager,
            disable_session_naming: options.disable_session_naming,
            provider_inventory,
            additional_source_roots: options.additional_source_roots,
        })
    }

    fn config(&self) -> Result<&'static Config, agent_client_protocol::Error> {
        Ok(Config::global())
    }

    async fn create_provider(
        &self,
        provider_name: &str,
        model_config: crate::model::ModelConfig,
        extensions: Vec<ExtensionConfig>,
        working_dir: Option<PathBuf>,
    ) -> Result<Arc<dyn Provider>> {
        (self.provider_factory)(
            provider_name.to_string(),
            model_config,
            extensions,
            working_dir,
        )
        .await
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
        mcp_servers: Vec<McpServer>,
    ) -> Result<Vec<ExtensionConfig>, agent_client_protocol::Error> {
        let mut extensions = Vec::new();
        for builtin in &self.builtins {
            push_or_replace_extension(&mut extensions, builtin_to_extension_config(builtin));
        }

        if mcp_servers.is_empty() {
            for extension in get_enabled_extensions_with_config(config) {
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

        let client: Arc<dyn McpClientTrait> = Arc::new(AcpTools {
            inner: Arc::new(dev_client),
            cx: cx.clone(),
            session_id: SessionId::new(session.id.clone()),
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
                self.build_enabled_extensions_data(config, &session, mcp_servers)?;
            builder = builder.extension_data(extension_data);
            session_needs_update = true;
        }

        if session_needs_update {
            let session_id = session.id.clone();
            builder
                .apply()
                .await
                .internal_err_ctx("Failed to update session")?;

            let _ = self.agent_manager.remove_session(&session_id).await;

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
    ) -> Result<ExtensionData, agent_client_protocol::Error> {
        let extensions = self.initial_session_extensions(config, mcp_servers)?;
        let mut extension_data = session.extension_data.clone();
        EnabledExtensionsState::new(extensions)
            .to_extension_data(&mut extension_data)
            .internal_err_ctx("Failed to initialize session extensions")?;
        Ok(extension_data)
    }

    async fn register_acp_session(
        &self,
        session_id: String,
        agent: Arc<Agent>,
        tool_requests: HashMap<String, ToolRequest>,
    ) {
        let acp_session = GooseAcpSession {
            agent,
            tool_requests,
            chain_membership: HashMap::new(),
            responded_tool_ids: HashSet::new(),
            summarized_chains: HashSet::new(),
            cancel_token: None,
            active_run_id: None,
        };
        self.sessions.lock().await.insert(session_id, acp_session);
    }

    async fn activate_acp_session(
        &self,
        cx: &ConnectionTo<Client>,
        session: &Session,
        tool_requests: HashMap<String, ToolRequest>,
    ) -> Result<(Arc<Agent>, Vec<ExtensionLoadResult>), agent_client_protocol::Error> {
        let (agent, extension_results) = self.prepare_acp_session_agent(cx, session).await?;
        self.register_acp_session(session.id.clone(), agent.clone(), tool_requests)
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
                                        agent_client_protocol::schema::Role::Assistant => {
                                            Some(Role::Assistant)
                                        }
                                        agent_client_protocol::schema::Role::User => {
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
        session: &mut GooseAcpSession,
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
                    session,
                    cx,
                )
                .await?;
            }
            MessageContent::ToolResponse(tool_response) => {
                self.handle_tool_response(
                    tool_response,
                    session_id,
                    session_id_str,
                    message_id,
                    session,
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
                    send_elicitation_interaction_update(
                        cx,
                        self.supports_goose_custom_notifications(),
                        session_id.0.as_ref(),
                        InteractionUpdate {
                            interaction: Interaction::Elicitation {
                                id: id.clone(),
                                state: InteractionState::Pending,
                                message: Some(message.clone()),
                                requested_schema: Some(requested_schema.clone()),
                            },
                            meta: Some(interaction_update_meta(message_id, message_created)),
                        },
                    )?;
                }
                ActionRequiredData::ElicitationResponse { .. } => {}
            },
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

    async fn handle_tool_request(
        &self,
        tool_request: &crate::conversation::message::ToolRequest,
        session_id: &SessionId,
        session_id_for_persist: &str,
        message_id: Option<&str>,
        session: &mut GooseAcpSession,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        session
            .tool_requests
            .insert(tool_request.id.clone(), tool_request.clone());

        let pending_tool_call = pending_tool_call_from_request(tool_request);
        let initial_tool_call = pending_tool_call
            .tool_call
            .meta(pending_tool_call.identity_meta.clone());
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCall(initial_tool_call),
        ))?;

        if Config::global()
            .get_goose_disable_tool_call_summary()
            .unwrap_or(false)
        {
            return Ok(());
        }

        if let Ok(tool_call) = &tool_request.tool_call {
            let agent = session.agent.clone();
            let sid = session_id.clone();
            let request_id = tool_request.id.clone();
            let cx = cx.clone();
            let name = tool_call.name.to_string();
            let identity_meta = pending_tool_call.identity_meta.clone();
            let fallback_title = pending_tool_call.fallback_title.clone();
            let session_id_for_persist = session_id_for_persist.to_string();
            let message_id_for_persist = message_id.map(|s| s.to_string());
            let session_manager = self.session_manager.clone();
            let args_json = tool_call
                .arguments
                .as_ref()
                .map(|a| {
                    let s = serde_json::to_string(a).unwrap_or_default();
                    if s.len() > 300 {
                        format!("{}…", crate::utils::safe_truncate(&s, 300))
                    } else {
                        s
                    }
                })
                .unwrap_or_default();

            tokio::spawn(async move {
                let (title, from_llm) = match agent.provider().await {
                    Ok(provider) => {
                        if provider.manages_own_context() {
                            return;
                        }

                        let system =
                            "Summarize this tool call in a short lowercase phrase (3-8 words). \
                             No punctuation. No quotes. Examples: reading project configuration, \
                             checking network connectivity, listing files in src directory";
                        let user_text = format!("Tool: {name}\nArguments: {args_json}");
                        let message = Message::user().with_text(&user_text);
                        // The fast model occasionally returns an empty response
                        // under load (rate limiting, transient network). One
                        // retry with a short backoff is enough to recover the
                        // common cases without paying for the regular model.
                        let mut llm_outcome: Option<String> = None;
                        for attempt in 0..2 {
                            match provider
                                .complete_fast(&sid.0, system, std::slice::from_ref(&message), &[])
                                .await
                            {
                                Ok((response, _)) => {
                                    let summary: String = response
                                        .content
                                        .iter()
                                        .filter_map(|c: &MessageContent| c.as_text())
                                        .collect::<String>()
                                        .trim()
                                        .to_string();
                                    if !summary.is_empty() {
                                        llm_outcome = Some(summary);
                                        break;
                                    }
                                    if attempt == 0 {
                                        warn!(
                                            "tool call summary: fast_complete returned empty for {request_id} ({name}), retrying once",
                                        );
                                        tokio::time::sleep(std::time::Duration::from_millis(150))
                                            .await;
                                    }
                                }
                                Err(e) => {
                                    if attempt == 0 {
                                        warn!(
                                            "tool call summary: fast_complete errored for {request_id} ({name}): {e}, retrying once",
                                        );
                                        tokio::time::sleep(std::time::Duration::from_millis(150))
                                            .await;
                                    } else {
                                        warn!(
                                            "tool call summary: fast_complete errored for {request_id} ({name}) after retry: {e}",
                                        );
                                    }
                                }
                            }
                        }
                        match llm_outcome {
                            Some(summary) => (summary, true),
                            None => {
                                warn!(
                                    "tool call summary: falling back to deterministic title for {request_id} ({name}) — replay will not show an LLM summary for this call",
                                );
                                (fallback_title.clone(), false)
                            }
                        }
                    }
                    Err(e) => {
                        warn!("tool call summary: failed to get provider: {e}");
                        (fallback_title.clone(), false)
                    }
                };

                let fields = ToolCallUpdateFields::new().title(title.clone());
                let _ = cx.send_notification(SessionNotification::new(
                    sid,
                    SessionUpdate::ToolCallUpdate(
                        ToolCallUpdate::new(ToolCallId::new(request_id.clone()), fields)
                            .meta(identity_meta),
                    ),
                ));

                // Best-effort persistence: only persist the LLM-generated title
                // (not the deterministic fallback) so reload uses fallback_title
                // for older or failed cases just like today.
                if from_llm {
                    if let Some(msg_id) = message_id_for_persist {
                        let patch = serde_json::json!({
                            crate::conversation::message::TOOL_META_TITLE_KEY: title,
                        });
                        if let Err(e) = session_manager
                            .update_tool_request_meta(
                                &session_id_for_persist,
                                &msg_id,
                                &request_id,
                                patch,
                            )
                            .await
                        {
                            warn!(
                                "tool call summary: persist failed for {request_id} in {msg_id}: {e}",
                            );
                        }
                    } else {
                        warn!(
                            "tool call summary: missing message_id for {request_id} — title will not survive reload",
                        );
                    }
                }
            });
        }

        Ok(())
    }

    async fn handle_tool_response(
        &self,
        tool_response: &crate::conversation::message::ToolResponse,
        session_id: &SessionId,
        session_id_str: &str,
        message_id: Option<&str>,
        session: &mut GooseAcpSession,
        cx: &ConnectionTo<Client>,
    ) -> Result<(), agent_client_protocol::Error> {
        let status = match &tool_response.tool_result {
            Ok(result) if result.is_error == Some(true) => ToolCallStatus::Failed,
            Ok(_) => ToolCallStatus::Completed,
            Err(_) => ToolCallStatus::Failed,
        };

        let mut fields = ToolCallUpdateFields::new().status(status);
        if let Some(raw_output) = extract_tool_raw_output(&tool_response.tool_result) {
            fields = fields.raw_output(raw_output);
        }
        if !tool_response
            .tool_result
            .as_ref()
            .is_ok_and(|r| r.is_acp_aware())
        {
            let content = build_tool_call_content(&tool_response.tool_result);
            fields = fields.content(content);

            let locations = extract_locations_from_meta(tool_response).unwrap_or_else(|| {
                if let Some(tool_request) = session.tool_requests.get(&tool_response.id) {
                    extract_tool_locations(tool_request, tool_response)
                } else {
                    Vec::new()
                }
            });
            if !locations.is_empty() {
                fields = fields.locations(locations);
            }
        }

        let update = ToolCallUpdate::new(ToolCallId::new(tool_response.id.clone()), fields)
            .meta(extract_tool_call_update_meta(tool_response));
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCallUpdate(update),
        ))?;

        // Chain summarization: when this response completes a multi-tool
        // chain, fire one LLM summary covering the run.
        session.responded_tool_ids.insert(tool_response.id.clone());
        self.maybe_summarize_chain(&tool_response.id, session_id, session_id_str, session, cx);
        let _ = message_id;

        Ok(())
    }

    /// If `tool_call_id` belongs to a multi-tool chain and every step in that
    /// chain has now had its response processed, spawn a single LLM
    /// summarization task that persists the chain summary on the first tool
    /// request and notifies the client. Idempotent — fires at most once per
    /// chain.
    fn maybe_summarize_chain(
        &self,
        tool_call_id: &str,
        session_id: &SessionId,
        _session_id_str: &str,
        session: &mut GooseAcpSession,
        cx: &ConnectionTo<Client>,
    ) {
        let Some(chain) = session.chain_membership.get(tool_call_id).cloned() else {
            warn!(
                "tool chain summary: skipped — no chain registered for tool_call_id {tool_call_id}",
            );
            return;
        };
        if !chain
            .ids
            .iter()
            .all(|id| session.responded_tool_ids.contains(id))
        {
            let total = chain.ids.len();
            let responded = chain
                .ids
                .iter()
                .filter(|id| session.responded_tool_ids.contains(*id))
                .count();
            let missing: Vec<&String> = chain
                .ids
                .iter()
                .filter(|id| !session.responded_tool_ids.contains(*id))
                .collect();
            warn!(
                "tool chain summary: waiting on {pending}/{total} responses for chain anchored at {anchor:?} (missing: {missing:?})",
                pending = total - responded,
                anchor = chain.ids.first(),
            );
            return;
        }
        let Some(first_id) = chain.ids.first() else {
            warn!("tool chain summary: skipped — empty chain.ids for tool_call_id {tool_call_id}");
            return;
        };
        if !session.summarized_chains.insert(first_id.clone()) {
            debug!("tool chain summary: chain anchored at {first_id} already summarized; skipping");
            return;
        }

        let agent = session.agent.clone();

        // Snapshot (name, args_json) for each step in document order.
        let steps: Vec<(String, String)> = chain
            .ids
            .iter()
            .filter_map(|id| {
                let req = session.tool_requests.get(id)?;
                let tool_call = req.tool_call.as_ref().ok()?;
                let name = tool_call.name.to_string();
                let args = tool_call
                    .arguments
                    .as_ref()
                    .map(|a| serde_json::to_string(a).unwrap_or_default())
                    .unwrap_or_default();
                let args = if args.len() > 200 {
                    format!("{}…", crate::utils::safe_truncate(&args, 200))
                } else {
                    args
                };
                Some((name, args))
            })
            .collect();
        if steps.len() < 2 {
            return;
        }

        let identity_meta = session
            .tool_requests
            .get(first_id)
            .and_then(tool_call_identity_meta);

        let sid = session_id.clone();
        let chain_for_task = chain.clone();
        let cx = cx.clone();
        let session_manager = self.session_manager.clone();

        let first_id = first_id.clone();
        tokio::spawn(async move {
            let provider = match agent.provider().await {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        "tool chain summary: failed to get provider for chain anchored at {first_id}: {e}",
                    );
                    return;
                }
            };
            if provider.manages_own_context() {
                warn!(
                    "tool chain summary: provider manages own context; skipping chain anchored at {first_id}",
                );
                return;
            }

            let system = "Summarize this sequence of tool calls in a short lowercase phrase \
                 (3-8 words). No punctuation. No quotes. \
                 Examples: applied dark mode polish, scanned for security issues, \
                 refactored config loading";

            let mut user_text = String::from("Tool call sequence:\n");
            for (i, (name, args)) in steps.iter().enumerate() {
                user_text.push_str(&format!("Step {}: {} {}\n", i + 1, name, args));
            }
            let message = Message::user().with_text(&user_text);

            // Match the per-tool retry policy: one retry on empty/error keeps
            // the chain header reliable when the fast model is rate-limited or
            // momentarily flaky, without escalating to the regular model.
            let mut summary: Option<String> = None;
            for attempt in 0..2 {
                match provider
                    .complete_fast(&sid.0, system, std::slice::from_ref(&message), &[])
                    .await
                {
                    Ok((response, _)) => {
                        let s = response
                            .content
                            .iter()
                            .filter_map(|c: &MessageContent| c.as_text())
                            .collect::<String>()
                            .trim()
                            .to_string();
                        if !s.is_empty() {
                            summary = Some(s);
                            break;
                        }
                        if attempt == 0 {
                            warn!(
                                "tool chain summary: fast_complete returned empty for chain anchored at {first_id} ({} steps), retrying once",
                                steps.len(),
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        }
                    }
                    Err(e) => {
                        if attempt == 0 {
                            warn!(
                                "tool chain summary: fast_complete errored for chain anchored at {first_id}: {e}, retrying once",
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        } else {
                            warn!(
                                "tool chain summary: fast_complete errored for chain anchored at {first_id} after retry: {e}",
                            );
                        }
                    }
                }
            }
            let Some(summary) = summary else {
                warn!(
                    "tool chain summary: no LLM summary produced for chain anchored at {first_id} — replay will fall back to the deterministic phrase",
                );
                return;
            };

            let count = chain_for_task.ids.len();
            let patch = serde_json::json!({
                crate::conversation::message::TOOL_META_CHAIN_SUMMARY_KEY: {
                    "summary": &summary,
                    "count": count,
                },
            });
            if let Err(e) = session_manager
                .update_tool_request_meta(&sid.0, &chain_for_task.message_id, &first_id, patch)
                .await
            {
                warn!(
                    "tool chain summary: persist failed for chain anchored at {first_id} in {}: {e}",
                    chain_for_task.message_id,
                );
            }

            let meta = with_tool_chain_summary_meta(identity_meta, &summary, count);
            let fields = ToolCallUpdateFields::new();
            let _ = cx.send_notification(SessionNotification::new(
                sid,
                SessionUpdate::ToolCallUpdate(
                    ToolCallUpdate::new(ToolCallId::new(first_id), fields).meta(meta),
                ),
            ));
        });
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

        let formatted_name = format_tool_name(&tool_name);

        let mut fields = ToolCallUpdateFields::new()
            .title(formatted_name)
            .kind(ToolKind::default())
            .status(ToolCallStatus::Pending)
            .raw_input(serde_json::Value::Object(arguments));
        if let Some(p) = prompt {
            fields = fields.content(vec![ToolCallContent::Content(Content::new(
                ContentBlock::Text(TextContent::new(p)),
            ))]);
        }
        let tool_call_update = ToolCallUpdate::new(ToolCallId::new(request_id.clone()), fields);

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

fn status_message_from_system_notification(
    notification: &SystemNotificationContent,
) -> Option<StatusMessage> {
    match notification.notification_type {
        SystemNotificationType::InlineMessage => Some(StatusMessage::Notice {
            message: notification.msg.clone(),
        }),
        SystemNotificationType::ThinkingMessage => Some(StatusMessage::Progress {
            message: notification.msg.clone(),
        }),
        SystemNotificationType::CreditsExhausted => None,
    }
}

fn send_elicitation_interaction_update(
    cx: &ConnectionTo<Client>,
    supports_goose_custom_notifications: bool,
    session_id: &str,
    update: InteractionUpdate,
) -> Result<(), agent_client_protocol::Error> {
    if supports_goose_custom_notifications {
        cx.send_notification(GooseSessionNotification {
            session_id: session_id.to_string(),
            update: GooseSessionUpdate::InteractionUpdate(update),
        })?;
    }
    Ok(())
}

fn interaction_update_meta(message_id: Option<&str>, created: i64) -> serde_json::Value {
    serde_json::Value::Object(message_update_meta(message_id, created, false))
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

fn extract_tool_call_update_meta(
    tool_response: &crate::conversation::message::ToolResponse,
) -> Option<Meta> {
    let tool_result = tool_response.tool_result.as_ref().ok()?;
    let goose_meta = tool_result
        .meta
        .as_ref()?
        .0
        .get(TRUSTED_TOOL_UPDATE_META_KEY)?
        .clone();
    let mut meta_map = serde_json::Map::new();
    meta_map.insert("goose".to_string(), goose_meta);
    Some(meta_map)
}

fn replay_message_meta(message: &Message) -> Meta {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "goose".to_string(),
        serde_json::Value::Object(replay_message_goose_meta(message)),
    );
    meta
}

fn replay_message_goose_meta(message: &Message) -> serde_json::Map<String, serde_json::Value> {
    let mut goose = serde_json::Map::new();
    goose.insert("created".to_string(), serde_json::json!(message.created));
    if let Some(id) = &message.id {
        goose.insert("messageId".to_string(), serde_json::json!(id));
    }
    if message.metadata.steer {
        goose.insert("steer".to_string(), serde_json::json!(true));
    }
    goose
}

fn merge_replay_message_meta(meta: Option<Meta>, message: &Message) -> Meta {
    let replay_goose = replay_message_goose_meta(message);
    let mut meta = meta.unwrap_or_default();
    let goose_value = meta
        .entry("goose".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let serde_json::Value::Object(goose) = goose_value {
        for (key, value) in replay_goose {
            goose.insert(key, value);
        }
    } else {
        *goose_value = serde_json::Value::Object(replay_goose);
    }

    meta
}

fn build_tool_call_content(tool_result: &ToolResult<CallToolResult>) -> Vec<ToolCallContent> {
    match tool_result {
        Ok(result) => result
            .content
            .iter()
            .filter_map(|content| match &content.raw {
                RawContent::Text(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Text(TextContent::new(val.text.clone())),
                ))),
                RawContent::Image(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Image(ImageContent::new(val.data.clone(), val.mime_type.clone())),
                ))),
                RawContent::Resource(val) => {
                    let resource = match &val.resource {
                        ResourceContents::TextResourceContents {
                            mime_type,
                            text,
                            uri,
                            ..
                        } => EmbeddedResourceResource::TextResourceContents(
                            TextResourceContents::new(text.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                        ResourceContents::BlobResourceContents {
                            mime_type,
                            blob,
                            uri,
                            ..
                        } => EmbeddedResourceResource::BlobResourceContents(
                            BlobResourceContents::new(blob.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                    };
                    Some(ToolCallContent::Content(Content::new(
                        ContentBlock::Resource(EmbeddedResource::new(resource)),
                    )))
                }
                RawContent::Audio(_) | RawContent::ResourceLink(_) => None,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn extract_tool_raw_output(tool_result: &ToolResult<CallToolResult>) -> Option<serde_json::Value> {
    tool_result
        .as_ref()
        .ok()
        .and_then(|result| result.structured_content.clone())
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
            .mcp_capabilities(McpCapabilities::new().http(true));
        Ok(InitializeResponse::new(args.protocol_version)
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

    /// Look up the session's agent.  Optionally sets a cancellation token on
    /// the session (needed by `on_prompt`).
    async fn get_session_agent(
        &self,
        session_id: &str,
        cancel_token: Option<CancellationToken>,
    ) -> Result<Arc<Agent>, agent_client_protocol::Error> {
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                if let Some(token) = cancel_token {
                    session.cancel_token = Some(token);
                }
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
        let (agent, _) = self
            .activate_acp_session(cx, &session, HashMap::new())
            .await?;

        if let Some(token) = cancel_token {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.cancel_token = Some(token);
            }
        }
        Ok(agent)
    }

    async fn start_active_run(
        &self,
        session_id: &str,
        run_id: String,
        cancel_token: CancellationToken,
    ) -> Result<(), agent_client_protocol::Error> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            agent_client_protocol::Error::resource_not_found(Some(session_id.to_string()))
                .data(format!("Session not found: {}", session_id))
        })?;

        if let Some(active_run_id) = &session.active_run_id {
            return Err(agent_client_protocol::Error::invalid_params().data(format!(
                "session already has active run `{active_run_id}`; use _goose/unstable/session/steer"
            )));
        }

        session.cancel_token = Some(cancel_token);
        session.active_run_id = Some(run_id);
        Ok(())
    }

    async fn clear_active_run(&self, session_id: &str, run_id: &str) {
        let agent = {
            let mut sessions = self.sessions.lock().await;
            let Some(session) = sessions.get_mut(session_id) else {
                return;
            };
            if session.active_run_id.as_deref() != Some(run_id) {
                return;
            }
            session.cancel_token = None;
            session.active_run_id = None;
            session.agent.clone()
        };
        agent.discard_pending_steers(session_id).await;
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

        let sessions = self.sessions.lock().await;
        let session = sessions.get(session_id).ok_or_else(|| {
            agent_client_protocol::Error::resource_not_found(Some(session_id.to_string()))
                .data(format!("Session not found: {}", session_id))
        })?;
        let active_run_id = session.active_run_id.as_ref().ok_or_else(|| {
            agent_client_protocol::Error::invalid_params().data("no active run to steer")
        })?;
        if active_run_id != expected_run_id {
            return Err(
                agent_client_protocol::Error::invalid_params().data(serde_json::json!({
                    "message": format!(
                        "expected active run id `{expected_run_id}` but found `{active_run_id}`"
                    ),
                    "expectedRunId": expected_run_id,
                    "actualRunId": active_run_id,
                })),
            );
        }
        Ok(active_run_id.clone())
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

    #[allow(dead_code)]
    async fn add_mcp_extensions(
        agent: &Arc<Agent>,
        mcp_servers: Vec<McpServer>,
        session_id: &str,
    ) -> Result<(), agent_client_protocol::Error> {
        let mut configs = Vec::with_capacity(mcp_servers.len());
        for mcp_server in mcp_servers {
            let config = match mcp_server_to_extension_config(mcp_server) {
                Ok(c) => c,
                Err(msg) => {
                    return Err(agent_client_protocol::Error::invalid_params().data(msg));
                }
            };
            configs.push(config);
        }

        if configs.is_empty() {
            return Ok(());
        }

        let results = agent
            .add_extensions_bulk(configs, session_id)
            .await
            .internal_err()?;
        for result in &results {
            if !result.success {
                let error_msg = result.error.as_deref().unwrap_or("unknown error");
                return Err(agent_client_protocol::Error::internal_error().data(format!(
                    "Failed to add MCP server '{}': {}",
                    result.name, error_msg
                )));
            }
        }
        Ok(())
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
        let sid = sid_short(&session_id);
        let t_start = std::time::Instant::now();

        let run_id = format!("run_{}", Uuid::new_v4());
        let cancel_token = CancellationToken::new();
        self.start_active_run(&session_id, run_id.clone(), cancel_token.clone())
            .await?;
        if let Err(error) = Self::send_active_run_update(cx, &args.session_id, Some(&run_id)) {
            self.clear_active_run(&session_id, &run_id).await;
            return Err(error);
        }

        let agent = match self.get_session_agent(&session_id, None).await {
            Ok(agent) => agent,
            Err(error) => {
                self.clear_active_run(&session_id, &run_id).await;
                let _ = Self::send_active_run_update(cx, &args.session_id, None);
                return Err(error);
            }
        };

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
        let mut first_event_logged = false;
        let mut event_count: u32 = 0;
        // Streaming chain buffer: tracks consecutive tool requests across
        // `AgentEvent::Message` events so chains that span multiple rows are
        // still registered. Sequential tool use (Bedrock/Anthropic) yields
        // request → response → request → response across separate
        // assistant/user messages, so tool responses are chain-neutral; only
        // non-tool content (text, thinking, image, etc.) breaks the run.
        // Holds `(tool_call_id, message_id_of_owning_row)` in arrival order;
        // re-registered eagerly each time a request arrives so
        // `handle_tool_response` finds the chain when subsequent responses
        // are processed.
        let mut chain_buffer: Vec<(String, String)> = Vec::new();
        let mut stream_error = None;

        while let Some(event) = stream.next().await {
            if cancel_token.is_cancelled() {
                was_cancelled = true;
                break;
            }
            event_count += 1;
            if !first_event_logged {
                debug!(
                    target: "perf",
                    sid = %sid,
                    ttft_ms = t_start.elapsed().as_millis() as u64,
                    "perf: prompt first stream event (time-to-first-token from prompt start)"
                );
                first_event_logged = true;
            }

            match event {
                Ok(crate::agents::AgentEvent::Message(message)) => {
                    // Agent persists messages via session_manager.add_message() internally.
                    let stored_message_id = message.id.clone();

                    let mut sessions = self.sessions.lock().await;
                    let Some(session) = sessions.get_mut(&session_id) else {
                        stream_error = Some(
                            agent_client_protocol::Error::invalid_params()
                                .data(format!("Session not found: {}", session_id)),
                        );
                        break;
                    };

                    for content_item in &message.content {
                        if let Some(error) = prompt_error_from_message_content(content_item) {
                            stream_error = Some(error);
                            break;
                        }

                        match content_item {
                            MessageContent::ToolRequest(tr) => {
                                if let Some(msg_id) = stored_message_id.as_deref() {
                                    chain_buffer.push((tr.id.clone(), msg_id.to_string()));
                                    // Re-register eagerly so the chain is in
                                    // place by the time the matching
                                    // `tool_response` triggers
                                    // `maybe_summarize_chain` (sequential
                                    // tool use interleaves request/response
                                    // events).
                                    extend_chain_membership(
                                        &chain_buffer,
                                        &mut session.chain_membership,
                                    );
                                }
                            }
                            MessageContent::ToolResponse(_) => {
                                // Chain-neutral: a response between two
                                // requests doesn't break the run, matching
                                // the frontend's `groupContentSections`.
                            }
                            _ => {
                                // Text, thinking, image, etc. end the run.
                                chain_buffer.clear();
                            }
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
                                session,
                                cx,
                            )
                            .await
                        {
                            stream_error = Some(error);
                            break;
                        }
                    }
                    if stream_error.is_some() {
                        break;
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

        {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                // Final safety net: in case the stream ended without any
                // chain-breaking content, make sure a multi-tool buffer is
                // registered. (Eager registration during the loop usually
                // covers this.)
                extend_chain_membership(&chain_buffer, &mut session.chain_membership);
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
        if let Some(updates) = build_usage_updates(&session) {
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

        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            events = event_count,
            cancelled = was_cancelled,
            "perf: prompt done"
        );
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
        let agent = self.get_session_agent(&req.session_id, None).await?;
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
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(&session_id) {
            if let Some(ref token) = session.cancel_token {
                info!(session_id = %session_id, "prompt cancelled");
                token.cancel();
            }
        } else {
            warn!(session_id = %session_id, "cancel request for unknown session");
        }

        Ok(())
    }

    async fn on_elicitation_respond(
        &self,
        cx: &ConnectionTo<Client>,
        req: ElicitationRespondRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        ActionRequiredManager::global()
            .submit_response(req.elicitation_id.clone(), req.user_data.clone())
            .await
            .invalid_params_err_ctx("Failed to submit elicitation response")?;

        let response_message = Message::user()
            .with_generated_id()
            .with_content(MessageContent::action_required_elicitation_response(
                req.elicitation_id.clone(),
                req.user_data,
            ))
            .agent_only();

        self.session_manager
            .add_message(&req.session_id, &response_message)
            .await
            .internal_err_ctx("Failed to persist elicitation response")?;

        send_elicitation_interaction_update(
            cx,
            self.supports_goose_custom_notifications(),
            &req.session_id,
            InteractionUpdate {
                interaction: Interaction::Elicitation {
                    id: req.elicitation_id,
                    state: InteractionState::Submitted,
                    message: None,
                    requested_schema: None,
                },
                meta: Some(interaction_update_meta(
                    response_message.id.as_deref(),
                    response_message.created,
                )),
            },
        )?;

        Ok(EmptyResponse {})
    }

    async fn on_set_model(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> Result<SetSessionModelResponse, agent_client_protocol::Error> {
        let agent = self.get_session_agent(session_id, None).await?;
        let current_provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let provider_name = current_provider.get_name().to_string();
        let current_model_config = current_provider.get_model_config();
        let model_config = crate::model::ModelConfig::new(model_id)
            .invalid_params_err_ctx("Invalid model config")?
            .with_canonical_limits(&provider_name);
        let model_config =
            model_config.with_inherited_session_settings_from(Some(&current_model_config), None);
        agent
            .recreate_provider_for_session(session_id, &provider_name, model_config)
            .await
            .internal_err_ctx("Failed to recreate provider")?;
        // model_config is already updated on the session by the agent's update_provider call.
        Ok(SetSessionModelResponse::new())
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
        let agent = self.get_session_agent(&session_id.0, None).await?;
        let provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let provider_name = provider.get_name().to_string();
        let current_model_config = provider.get_model_config();
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

        let agent = self.get_session_agent(session_id, None).await?;
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
        let agent = self.get_session_agent(session_id, None).await?;
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
        let agent = self.get_session_agent(session_id, None).await?;
        let current_provider = agent
            .provider()
            .await
            .internal_err_ctx("Failed to get provider")?;
        let current_provider_name = current_provider.get_name();
        let current_model_config = current_provider.get_model_config();
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
            ACP_CURRENT_MODEL.to_string()
        } else {
            current_model
        };
        let model = model_name.unwrap_or(&default_model);
        let mut model_config = crate::model::ModelConfig::new(model)
            .invalid_params_err_ctx("Invalid model config")?
            .with_canonical_limits(&resolved_provider_name)
            .with_context_limit(context_limit);
        model_config = model_config
            .with_inherited_session_settings_from(Some(&current_model_config), request_params);

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
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            if let Some(ref token) = session.cancel_token {
                token.cancel();
            }
        }
        sessions.remove(session_id);
        drop(sessions);

        let _ = self.agent_manager.remove_session(session_id).await;

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
    use crate::conversation::message::{ToolRequest, ToolResponse};
    use crate::session::session_manager::SessionType;
    use agent_client_protocol::schema::{
        EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
        PermissionOptionId, ResourceLink, SelectedPermissionOutcome,
    };
    use rmcp::model::{CallToolRequestParams, Content as RmcpContent};
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

    #[test]
    fn test_format_tool_name_with_extension() {
        assert_eq!(format_tool_name("developer__edit"), "developer: edit");
        assert_eq!(
            format_tool_name("platform__manage_extensions"),
            "platform: manage extensions"
        );
        assert_eq!(format_tool_name("todo__write"), "todo: write");
    }

    #[test]
    fn test_format_tool_name_without_extension() {
        assert_eq!(format_tool_name("simple_tool"), "simple tool");
        assert_eq!(format_tool_name("another_name"), "another name");
        assert_eq!(format_tool_name("single"), "single");
    }

    #[test]
    fn test_summarize_tool_call_no_args() {
        assert_eq!(
            summarize_tool_call("developer__shell", None),
            "developer: shell"
        );
    }

    #[test]
    fn test_summarize_tool_call_with_path() {
        let args = serde_json::json!({"path": "/src/main.rs", "content": "fn main() {}"});
        assert_eq!(
            summarize_tool_call("developer__edit", Some(&args)),
            "developer: edit · /src/main.rs"
        );
    }

    #[test]
    fn test_summarize_tool_call_with_command() {
        let args = serde_json::json!({"command": "cargo build"});
        assert_eq!(
            summarize_tool_call("developer__shell", Some(&args)),
            "developer: shell · cargo build"
        );
    }

    #[test]
    fn test_tool_call_identity_meta_uses_goose_extension_metadata() {
        let request = ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("context7__query-docs")),
            metadata: None,
            tool_meta: Some(serde_json::json!({"goose_extension": "context7"})),
        };

        let meta = tool_call_identity_meta(&request).expect("expected metadata");

        assert_eq!(
            meta.get("goose"),
            Some(&serde_json::json!({
                "toolCall": {
                    "toolName": "context7__query-docs",
                    "extensionName": "context7",
                },
            })),
        );
    }

    fn buf_entry(tool_id: &str, msg_id: &str) -> (String, String) {
        (tool_id.to_string(), msg_id.to_string())
    }

    #[test]
    fn extend_chain_membership_skips_singleton_and_leaves_buffer() {
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let buffer = vec![buf_entry("a", "row_1")];

        extend_chain_membership(&buffer, &mut membership);

        assert_eq!(buffer.len(), 1, "buffer is left intact for caller");
        assert!(
            membership.is_empty(),
            "single-tool runs should not register a chain",
        );
    }

    #[test]
    fn extend_chain_membership_registers_each_id_against_shared_chain() {
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let buffer = vec![
            buf_entry("a", "row_first"),
            buf_entry("b", "row_second"),
            buf_entry("c", "row_third"),
        ];

        extend_chain_membership(&buffer, &mut membership);

        assert_eq!(membership.len(), 3);
        let chain_a = membership.get("a").expect("a registered");
        let chain_b = membership.get("b").expect("b registered");
        let chain_c = membership.get("c").expect("c registered");
        assert!(
            Arc::ptr_eq(chain_a, chain_b) && Arc::ptr_eq(chain_b, chain_c),
            "every id in the run must point at the same ToolChain Arc",
        );
        assert_eq!(
            chain_a.ids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
    }

    #[test]
    fn extend_chain_membership_anchors_on_first_row_for_split_messages() {
        // Sequential tool use (Bedrock/Anthropic) emits each tool request as
        // its own assistant message, with the tool response interleaved in
        // between. The chain should still form, anchored on the *first*
        // tool's row id so `update_tool_request_meta` can find that
        // ToolRequest when persisting the summary.
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let buffer = vec![
            buf_entry("toolu_bdrk_1", "row_for_tool_1"),
            buf_entry("toolu_bdrk_2", "row_for_tool_2"),
        ];

        extend_chain_membership(&buffer, &mut membership);

        let chain = membership
            .get("toolu_bdrk_1")
            .expect("first tool registered");
        assert_eq!(
            chain.ids,
            vec!["toolu_bdrk_1".to_string(), "toolu_bdrk_2".to_string()],
        );
        let chain_via_second = membership
            .get("toolu_bdrk_2")
            .expect("second tool registered");
        assert!(Arc::ptr_eq(chain, chain_via_second));
    }

    #[test]
    fn extend_chain_membership_grows_chain_as_more_requests_arrive() {
        // The streaming loop re-registers eagerly each time a new request
        // arrives, so a chain that started at length 2 must grow to include
        // a third tool whose response is yet to come. Both the original
        // members and the new member must point at the new (extended) chain.
        let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
        let mut buffer = vec![buf_entry("a", "row_1"), buf_entry("b", "row_2")];
        extend_chain_membership(&buffer, &mut membership);

        buffer.push(buf_entry("c", "row_3"));
        extend_chain_membership(&buffer, &mut membership);

        let chain_a = membership.get("a").expect("a present");
        let chain_b = membership.get("b").expect("b present");
        let chain_c = membership.get("c").expect("c present");
        assert!(Arc::ptr_eq(chain_a, chain_b) && Arc::ptr_eq(chain_b, chain_c));
        assert_eq!(
            chain_a.ids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
    }

    #[test]
    fn with_tool_chain_summary_meta_creates_fresh_when_none() {
        let meta = with_tool_chain_summary_meta(None, "applied dark mode", 4)
            .expect("meta should be created");
        assert_eq!(
            meta.get("goose"),
            Some(&serde_json::json!({
                "toolChainSummary": { "summary": "applied dark mode", "count": 4 },
            })),
        );
    }

    #[test]
    fn with_tool_chain_summary_meta_preserves_existing_tool_call_identity() {
        let existing = tool_call_identity_meta(&ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")),
            metadata: None,
            tool_meta: None,
        });
        let meta = with_tool_chain_summary_meta(existing, "ran two commands", 2)
            .expect("meta should be created");
        let goose = meta.get("goose").expect("goose key");
        assert_eq!(
            goose.get("toolCall"),
            Some(
                &serde_json::json!({ "toolName": "developer__shell", "extensionName": "developer" })
            )
        );
        assert_eq!(
            goose.get("toolChainSummary"),
            Some(&serde_json::json!({ "summary": "ran two commands", "count": 2 }))
        );
    }

    #[test]
    fn replay_attaches_chain_summary_meta_for_first_tool_request_with_persisted_summary() {
        let tool_request = ToolRequest {
            id: "req_first".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")),
            metadata: None,
            tool_meta: Some(serde_json::json!({
                crate::conversation::message::TOOL_META_CHAIN_SUMMARY_KEY: {
                    "summary": "applied dark mode polish",
                    "count": 3,
                },
            })),
        };

        let pending_tool_call = pending_tool_call_from_request(&tool_request);
        let mut meta = pending_tool_call.identity_meta;
        let chain_summary = tool_request
            .persisted_chain_summary()
            .expect("chain summary should be present");
        meta = with_tool_chain_summary_meta(meta, &chain_summary.summary, chain_summary.count);

        let goose = meta
            .as_ref()
            .and_then(|m| m.get("goose"))
            .expect("replay meta must include a goose namespace");
        assert_eq!(
            goose.get("toolCall"),
            Some(
                &serde_json::json!({ "toolName": "developer__shell", "extensionName": "developer" })
            ),
            "replay must preserve identity meta alongside the chain summary",
        );
        assert_eq!(
            goose.get("toolChainSummary"),
            Some(&serde_json::json!({ "summary": "applied dark mode polish", "count": 3 })),
            "replay must attach toolChainSummary so the chain header renders on first paint",
        );
    }

    #[test]
    fn replay_does_not_attach_chain_summary_for_tool_requests_without_persisted_summary() {
        let tool_request = ToolRequest {
            id: "req_second".to_string(),
            tool_call: Ok(CallToolRequestParams::new("developer__shell")),
            metadata: None,
            tool_meta: None,
        };

        let chain_summary = tool_request.persisted_chain_summary();
        assert!(
            chain_summary.is_none(),
            "non-first tool requests must not carry chain summaries",
        );
    }

    #[test]
    fn test_summarize_tool_call_long_value_truncated() {
        let long_path = "a".repeat(80);
        let args = serde_json::json!({"path": long_path});
        let result = summarize_tool_call("developer__read_file", Some(&args));
        assert!(result.ends_with('…'));
        assert!(result.len() < 90);
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

    fn json_object(pairs: Vec<(&str, serde_json::Value)>) -> rmcp::model::JsonObject {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test_case(None => None ; "none arguments")]
    #[test_case(Some(json_object(vec![])) => None ; "missing line key")]
    #[test_case(Some(json_object(vec![("line", serde_json::json!(5))])) => Some(5) ; "line present")]
    #[test_case(Some(json_object(vec![("line", serde_json::json!("not_a_number"))])) => None ; "line not a number")]
    fn test_get_requested_line(arguments: Option<rmcp::model::JsonObject>) -> Option<u32> {
        get_requested_line(arguments.as_ref())
    }

    #[test_case("read", true ; "read is developer file tool")]
    #[test_case("write", true ; "write is developer file tool")]
    #[test_case("edit", true ; "edit is developer file tool")]
    #[test_case("shell", false ; "shell is not developer file tool")]
    #[test_case("analyze", false ; "analyze is not developer file tool")]
    fn test_is_developer_file_tool(tool_name: &str, expected: bool) {
        assert_eq!(is_developer_file_tool(tool_name), expected);
    }

    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("read").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "line": 5}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(5))]
        ; "read returns requested line"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("read").with_arguments(serde_json::json!({"path": "/tmp/f.txt"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), None)]
        ; "read without line"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("write").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "content": "hi"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(1))]
        ; "write returns line 1"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("edit").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "before": "a", "after": "b"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(1))]
        ; "edit returns line 1"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("shell").with_arguments(serde_json::json!({"command": "ls"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => Vec::<(PathBuf, Option<u32>)>::new()
        ; "non file tool returns empty"
    )]
    fn test_extract_tool_locations(
        request: ToolRequest,
        response: ToolResponse,
    ) -> Vec<(PathBuf, Option<u32>)> {
        extract_tool_locations(&request, &response)
            .into_iter()
            .map(|loc| (loc.path, loc.line))
            .collect()
    }

    fn response_with_meta(meta: Option<serde_json::Value>) -> ToolResponse {
        let mut result = CallToolResult::success(vec![RmcpContent::text("")]);
        result.meta = meta.map(|v| serde_json::from_value(v).unwrap());
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(result),
            metadata: None,
        }
    }

    #[test_case(
        response_with_meta(Some(serde_json::json!({"tool_locations": [{"path": "/tmp/f.txt", "line": 5}]})))
        => Some(vec![(PathBuf::from("/tmp/f.txt"), Some(5))])
        ; "meta with path and line"
    )]
    #[test_case(
        response_with_meta(Some(serde_json::json!({"tool_locations": [{"path": "/tmp/f.txt"}]})))
        => Some(vec![(PathBuf::from("/tmp/f.txt"), None)])
        ; "meta with path no line"
    )]
    #[test_case(
        response_with_meta(Some(serde_json::json!({})))
        => None
        ; "meta without tool_locations key"
    )]
    #[test_case(
        response_with_meta(None)
        => None
        ; "no meta"
    )]
    fn test_extract_locations_from_meta(
        response: ToolResponse,
    ) -> Option<Vec<(PathBuf, Option<u32>)>> {
        extract_locations_from_meta(&response)
            .map(|locs| locs.into_iter().map(|loc| (loc.path, loc.line)).collect())
    }

    #[test]
    fn test_extract_tool_call_update_meta_ignores_untrusted_goose_meta() {
        let response = response_with_meta(Some(serde_json::json!({
            "goose": {
                "mcpApp": {
                    "resourceUri": "ui://spoofed/app",
                },
            },
        })));

        assert_eq!(extract_tool_call_update_meta(&response), None);
    }

    #[test]
    fn test_extract_tool_call_update_meta_uses_trusted_meta_only() {
        let response = response_with_meta(Some(serde_json::json!({
            "goose": {
                "mcpApp": {
                    "resourceUri": "ui://spoofed/app",
                },
            },
            TRUSTED_TOOL_UPDATE_META_KEY: {
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            },
        })));

        let extracted = extract_tool_call_update_meta(&response).expect("expected trusted meta");
        assert_eq!(
            extracted.get("goose"),
            Some(&serde_json::json!({
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            })),
        );
    }

    #[test]
    fn test_merge_replay_message_meta_preserves_existing_goose_meta() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_1");
        let existing = serde_json::from_value(serde_json::json!({
            "goose": {
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            },
        }))
        .unwrap();

        let merged = merge_replay_message_meta(Some(existing), &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_1",
                "mcpApp": {
                    "resourceUri": "ui://trusted/app",
                    "extensionName": "weather",
                    "toolName": "weather__render",
                },
            })),
        );
    }

    #[test]
    fn test_merge_replay_message_meta_creates_fresh_when_none() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_2");

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_2",
            })),
        );
    }

    #[test]
    fn test_merge_replay_message_meta_includes_steer_marker() {
        let message = Message::new(Role::User, 1_700_000_000, vec![])
            .with_id("msg_steer")
            .with_steer();

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
                "messageId": "msg_steer",
                "steer": true,
            })),
            "replay must carry the steer marker so the boundary survives reload"
        );
    }

    #[test]
    fn test_merge_replay_message_meta_omits_steer_when_not_set() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]).with_id("msg_plain");

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(merged.get("goose").and_then(|g| g.get("steer")), None);
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

    #[test]
    fn test_merge_replay_message_meta_omits_message_id_when_none() {
        let message = Message::new(Role::Assistant, 1_700_000_000, vec![]);

        let merged = merge_replay_message_meta(None, &message);

        assert_eq!(
            merged.get("goose"),
            Some(&serde_json::json!({
                "created": 1_700_000_000,
            })),
        );
    }

    #[test]
    fn test_extract_tool_raw_output_preserves_structured_content() {
        let mut result = CallToolResult::success(vec![RmcpContent::text("fallback")]);
        result.structured_content = Some(serde_json::json!({
            "restaurants": [
                {
                    "name": "Coffee Shop",
                    "unitToken": "unit-1",
                },
            ],
        }));

        assert_eq!(
            extract_tool_raw_output(&Ok(result)),
            Some(serde_json::json!({
                "restaurants": [
                    {
                        "name": "Coffee Shop",
                        "unitToken": "unit-1",
                    },
                ],
            })),
        );
    }

    fn make_session_with_usage(
        total_tokens: Option<i32>,
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
        accumulated_total_tokens: Option<i32>,
        accumulated_input_tokens: Option<i32>,
        accumulated_output_tokens: Option<i32>,
    ) -> Session {
        Session {
            id: "session-1".to_string(),
            working_dir: PathBuf::from("/tmp"),
            name: "ACP Session".to_string(),
            user_set_name: false,
            session_type: SessionType::Acp,
            created_at: Default::default(),
            updated_at: Default::default(),
            extension_data: crate::session::ExtensionData::default(),
            total_tokens,
            input_tokens,
            output_tokens,
            accumulated_total_tokens,
            accumulated_input_tokens,
            accumulated_output_tokens,
            accumulated_cost: None,
            schedule_id: None,
            recipe: None,
            user_recipe_values: None,
            conversation: None,
            message_count: 0,
            provider_name: None,
            model_config: None,
            goose_mode: GooseMode::default(),
            archived_at: None,
            project_id: None,
            last_message_snippet: None,
        }
    }

    #[test]
    fn test_build_prompt_usage_uses_current_turn_tokens() {
        let session = make_session_with_usage(
            Some(120),
            Some(80),
            Some(40),
            Some(360),
            Some(210),
            Some(150),
        );
        let usage = build_prompt_usage(&session).expect("usage should be present");
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.input_tokens, 80);
        assert_eq!(usage.output_tokens, 40);
    }

    #[test]
    fn test_build_prompt_usage_falls_back_to_current_tokens() {
        let session = make_session_with_usage(Some(120), Some(80), Some(40), None, None, None);
        let usage = build_prompt_usage(&session).expect("usage should be present");
        assert_eq!(usage.total_tokens, 120);
        assert_eq!(usage.input_tokens, 80);
        assert_eq!(usage.output_tokens, 40);
    }

    #[test]
    fn test_build_prompt_usage_requires_total_tokens() {
        let session = make_session_with_usage(None, Some(80), Some(40), None, None, None);
        assert!(build_prompt_usage(&session).is_none());
    }

    #[test]
    fn test_build_usage_update_clamps_negative_used_to_zero() {
        let mut session = make_session_with_usage(Some(-7), Some(0), Some(0), None, None, None);
        session.model_config = Some(
            crate::model::ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(Some(258_000)),
        );
        let updates = build_usage_updates(&session).expect("usage updates should be present");
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
        let session = make_session_with_usage(Some(120), Some(80), Some(40), None, None, None);
        assert!(build_usage_updates(&session).is_none());
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
                    agent_client_protocol::schema::ClientCapabilities::new().meta(meta),
                );
        let goose_client_capabilities =
            extract_client_capabilities_meta(&request).and_then(|meta| meta.goose);

        assert!(extract_client_supports_goose_custom_notifications(
            goose_client_capabilities.as_ref()
        ));
    }
}
