use super::*;

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
pub struct GooseAcpSession {
    pub agent: AgentHandle,
    pub tool_requests: HashMap<String, crate::conversation::message::ToolRequest>,
    /// For each tool_call_id that belongs to a multi-tool chain (run of
    /// consecutive ToolRequest blocks within one assistant message), the chain
    /// it belongs to. Populated when the assistant message is processed.
    /// Used by `handle_tool_response` to detect when a chain has fully
    /// completed and fire a single LLM summary covering the run.
    pub chain_membership: HashMap<String, Arc<ToolChain>>,
    /// Set of tool_call_ids whose ToolResponse has already been processed.
    /// Drives the "all responses present" check for chain completion.
    pub responded_tool_ids: HashSet<String>,
    /// Tool_call_ids of chains that have already had a summary task fired.
    /// Idempotence guard so we summarize each chain at most once.
    pub summarized_chains: HashSet<String>,
    pub cancel_token: Option<CancellationToken>,
    /// Working directory set while the agent was still loading.
    /// Applied once the agent becomes ready.
    pub pending_working_dir: Option<std::path::PathBuf>,
}

/// A run of consecutive ToolRequest blocks within one assistant message,
/// tracked by [`GooseAcpSession::chain_membership`]. Used to drive a single
/// LLM summary for the whole run once every step has a recorded ToolResponse.
#[derive(Debug, Clone)]
pub struct ToolChain {
    /// Tool call ids in document order. Always `len() >= 2`.
    pub ids: Vec<String>,
    /// The message_id of the assistant message containing these tool calls.
    /// Used to persist chain summaries back to the messages table.
    pub message_id: String,
}

/// Progress stages signalled by the background agent setup task via the watch
/// channel.  `ProviderReady` fires as soon as the provider (and goose-mode)
/// are initialized — before extensions finish loading.  `FullyReady` fires
/// once every extension has been loaded (or failed).
#[derive(Clone)]
pub enum AgentSetupProgress {
    /// Provider is initialized; extensions are still loading in the background.
    ProviderReady(Arc<Agent>),
    /// Provider *and* all extensions are initialized.
    FullyReady(Arc<Agent>),
}

pub type AgentSetupSignal = Option<Result<AgentSetupProgress, String>>;

/// The agent may still be initializing in the background (extension loading,
/// provider setup).  Callers that need the live agent (e.g. `on_prompt`) await
/// the handle; callers that only need the session metadata can proceed without it.
pub enum AgentHandle {
    Ready(Arc<Agent>),
    Loading(tokio::sync::watch::Receiver<AgentSetupSignal>),
}

pub struct AgentSetupRequest {
    pub session_id: SessionId,
    pub goose_session: Session,
    pub mcp_servers: Vec<McpServer>,
    /// Pre-resolved provider name + model config (from config, no network).
    /// When present the spawn skips re-deriving these from config.
    pub resolved_provider: Option<(String, crate::model::ModelConfig)>,
    /// Pre-instantiated provider reused from synchronous session initialization.
    pub prebuilt_provider: Option<Arc<dyn Provider>>,
}

pub struct GooseAcpAgentOptions {
    pub provider_factory: AcpProviderFactory,
    pub builtins: Vec<String>,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub goose_mode: GooseMode,
    pub disable_session_naming: bool,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
}

pub struct GooseAcpAgent {
    pub sessions: Arc<Mutex<HashMap<String, GooseAcpSession>>>,
    pub provider_factory: AcpProviderFactory,
    pub builtins: Vec<String>,
    pub client_fs_capabilities: OnceCell<FileSystemCapabilities>,
    pub client_terminal: OnceCell<bool>,
    pub client_mcp_host_info: OnceCell<GooseMcpHostInfo>,
    pub use_login_shell_path: OnceCell<bool>,
    pub config_dir: std::path::PathBuf,
    pub session_manager: Arc<SessionManager>,
    pub permission_manager: Arc<PermissionManager>,
    pub goose_mode: GooseMode,
    pub disable_session_naming: bool,
    pub provider_inventory: ProviderInventoryService,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
}

/// Shorten a session/thread id for perf log correlation.
/// All `perf:` logs use `sid=<8-char-prefix>` so a single session's activity
/// can be extracted with `grep 'perf:' <log> | grep 'sid=abc12345'`.
pub fn sid_short(id: &str) -> String {
    id.chars().take(8).collect()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListCursorToken {
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // Goose stores updated_at with second precision in common write paths, so the
    // cursor needs the full (updated_at, id) sort key to avoid skipping tied rows.
    pub session_id: String,
    pub filter_hash: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListCursorFilters {
    pub cwd: Option<String>,
    pub session_types: Vec<String>,
    pub non_empty: bool,
}

pub fn invalid_session_list_cursor(message: &'static str) -> agent_client_protocol::Error {
    agent_client_protocol::Error::invalid_params().data(message)
}

// bind cursors to the effective filters so they cannot be reused for a different list.
pub fn session_list_filter_hash(
    cwd: Option<&std::path::Path>,
    session_types: &[SessionType],
) -> Result<String, agent_client_protocol::Error> {
    let mut session_type_names = session_types
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    session_type_names.sort();
    let filters = SessionListCursorFilters {
        cwd: cwd.map(|path| path.to_string_lossy().to_string()),
        session_types: session_type_names,
        non_empty: true,
    };
    let bytes =
        serde_json::to_vec(&filters).internal_err_ctx("Failed to encode session list filters")?;
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(bytes)))
}

pub fn decode_session_list_cursor(
    cursor: Option<&str>,
    cwd: Option<&std::path::Path>,
    session_types: &[SessionType],
) -> Result<Option<SessionListCursor>, agent_client_protocol::Error> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };

    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| invalid_session_list_cursor("malformed session list cursor"))?;
    let token: SessionListCursorToken = serde_json::from_slice(&bytes)
        .map_err(|_| invalid_session_list_cursor("malformed session list cursor"))?;

    if token.session_id.is_empty() || token.filter_hash.is_empty() {
        return Err(invalid_session_list_cursor("malformed session list cursor"));
    }

    let expected_filter_hash = session_list_filter_hash(cwd, session_types)?;
    if token.filter_hash != expected_filter_hash {
        return Err(invalid_session_list_cursor(
            "session list cursor does not match filters",
        ));
    }

    Ok(Some(SessionListCursor {
        updated_at: token.updated_at,
        session_id: token.session_id,
    }))
}

pub fn encode_session_list_cursor(
    cursor: &SessionListCursor,
    cwd: Option<&std::path::Path>,
    session_types: &[SessionType],
) -> Result<String, agent_client_protocol::Error> {
    let token = SessionListCursorToken {
        updated_at: cursor.updated_at,
        session_id: cursor.session_id.clone(),
        filter_hash: session_list_filter_hash(cwd, session_types)?,
    };
    let bytes =
        serde_json::to_vec(&token).internal_err_ctx("Failed to encode session list cursor")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn session_meta(session: &Session) -> serde_json::Map<String, serde_json::Value> {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "messageCount".to_string(),
        serde_json::Value::Number(session.message_count.into()),
    );
    meta.insert(
        "createdAt".to_string(),
        serde_json::Value::String(session.created_at.to_rfc3339()),
    );
    if let Some(ref archived_at) = session.archived_at {
        meta.insert(
            "archivedAt".to_string(),
            serde_json::Value::String(archived_at.to_rfc3339()),
        );
    }
    meta.insert(
        "userSetName".to_string(),
        serde_json::Value::Bool(session.user_set_name),
    );

    if let Some(ref pid) = session.project_id {
        meta.insert(
            "projectId".to_string(),
            serde_json::Value::String(pid.clone()),
        );
    }
    if let Some(ref provider) = session.provider_name {
        meta.insert(
            "providerId".to_string(),
            serde_json::Value::String(provider.clone()),
        );
    }
    if let Some(ref mc) = session.model_config {
        meta.insert(
            "modelId".to_string(),
            serde_json::Value::String(mc.model_name.clone()),
        );
    }
    meta
}

pub fn spawn_session_name_update_notifier(
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

pub fn extract_timeout_from_meta(meta: &Option<Meta>) -> Option<u64> {
    meta.as_ref()
        .and_then(|m| m.get("timeout"))
        .and_then(|v| v.as_u64())
}

#[derive(Debug, Default, Deserialize)]
pub struct GooseClientMetaEnvelope {
    #[serde(default)]
    pub goose: Option<GooseClientMeta>,
}

#[derive(Debug, Default, Deserialize)]
pub struct GooseClientMeta {
    #[serde(rename = "mcpHostCapabilities", default)]
    pub mcp_host_capabilities: Option<GooseMcpHostCapabilities>,
}

#[derive(Debug, Default, Deserialize)]
pub struct GooseMcpHostCapabilities {
    #[serde(default)]
    pub extensions: Option<rmcp::model::ExtensionCapabilities>,
}

pub fn extract_goose_client_meta(meta: &Meta) -> Option<GooseClientMetaEnvelope> {
    serde_json::from_value(serde_json::Value::Object(meta.clone())).ok()
}

pub fn extract_client_mcp_host_info(args: &InitializeRequest) -> GooseMcpHostInfo {
    let host_capabilities = args
        .client_capabilities
        .meta
        .as_ref()
        .and_then(extract_goose_client_meta)
        .and_then(|meta| meta.goose)
        .and_then(|goose| goose.mcp_host_capabilities);
    let explicit_extensions = host_capabilities
        .as_ref()
        .and_then(|capabilities| capabilities.extensions.as_ref())
        .is_some();
    let extensions = host_capabilities
        .and_then(|capabilities| capabilities.extensions)
        .unwrap_or_default();

    GooseMcpHostInfo {
        explicit_extensions,
        extensions,
        client_name: args.client_info.as_ref().map(|info| info.name.clone()),
        client_version: args.client_info.as_ref().map(|info| info.version.clone()),
    }
}

pub fn extract_use_login_shell_path(args: &InitializeRequest) -> bool {
    args.meta
        .as_ref()
        .and_then(|meta| meta.get("goose/useLoginShellPath"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn mcp_server_to_extension_config(mcp_server: McpServer) -> Result<ExtensionConfig, String> {
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
