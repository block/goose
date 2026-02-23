use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use goose::agent_manager::client::AgentClientManager;
use goose::agent_manager::{NewSessionRequest, SessionId, SessionModeId, SetSessionModeRequest};
use goose::registry::manifest::{RegistryEntryDetail, RegistryEntryKind};
use goose::registry::sources::local::LocalRegistrySource;
use goose::registry::RegistryManager;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

use crate::routes::errors::ErrorResponse;
use crate::state::AppState;

fn acp_manager() -> &'static Arc<Mutex<AgentClientManager>> {
    static INSTANCE: OnceLock<Arc<Mutex<AgentClientManager>>> = OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(Mutex::new(AgentClientManager::default())))
}

fn default_registry() -> Result<RegistryManager, ErrorResponse> {
    let mut manager = RegistryManager::new();
    let local = LocalRegistrySource::from_default_paths()
        .map_err(|e| ErrorResponse::internal(format!("Registry init failed: {e}")))?;
    manager.add_source(Box::new(local));
    Ok(manager)
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConnectAgentRequest {
    pub name: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ConnectAgentResponse {
    pub agent_id: String,
    pub connected: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateSessionRequest {
    pub working_dir: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CreateSessionResponse {
    pub session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct PromptAgentRequest {
    pub session_id: String,
    pub text: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PromptAgentResponse {
    pub text: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetModeAgentRequest {
    pub session_id: String,
    pub mode_id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentListResponse {
    pub agents: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/agents/external/connect",
    request_body = ConnectAgentRequest,
    responses(
        (status = 200, description = "Agent connected", body = ConnectAgentResponse),
        (status = 404, description = "Agent not found"),
        (status = 422, description = "Agent has no distribution")
    ),
    tag = "External Agents"
)]
pub async fn connect_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConnectAgentRequest>,
) -> Result<Json<ConnectAgentResponse>, ErrorResponse> {
    let registry = default_registry()?;
    let entry = registry
        .get(&req.name, Some(RegistryEntryKind::Agent))
        .await
        .map_err(|e| ErrorResponse::internal(format!("Registry lookup failed: {e}")))?;

    let entry = entry.ok_or_else(|| ErrorResponse::not_found("Agent not found in registry"))?;

    let distribution = match &entry.detail {
        RegistryEntryDetail::Agent(detail) => detail
            .distribution
            .as_ref()
            .ok_or_else(|| ErrorResponse::unprocessable("Agent has no distribution targets"))?,
        _ => {
            return Err(ErrorResponse::unprocessable(
                "Registry entry is not an agent",
            ))
        }
    };

    let mgr = acp_manager().lock().await;
    mgr.connect_with_distribution(req.name.clone(), distribution)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Connection failed: {e}")))?;

    // Resolve agent manifest dependencies via ServiceBroker
    if let RegistryEntryDetail::Agent(detail) = &entry.detail {
        if !detail.dependencies.is_empty() {
            let broker = goose::agent_manager::ServiceBroker::new();
            let resolution = broker.resolve_dependencies(detail);
            tracing::info!(
                agent = %req.name,
                resolved = resolution.resolved.len(),
                missing_required = resolution.missing_required.len(),
                missing_optional = resolution.missing_optional.len(),
                "Resolved agent manifest dependencies"
            );
            for dep_name in &resolution.missing_required {
                tracing::warn!(
                    agent = %req.name,
                    dep = %dep_name,
                    "Required dependency unresolved"
                );
            }
        }
    }

    // Register ACP delegation strategy in the slot registry
    state
        .agent_slot_registry
        .register_acp_agent(&req.name)
        .await;

    Ok(Json(ConnectAgentResponse {
        agent_id: req.name,
        connected: true,
    }))
}

#[utoipa::path(
    post,
    path = "/agents/external/{agent_id}/session",
    params(("agent_id" = String, Path, description = "Agent identifier")),
    request_body = CreateSessionRequest,
    responses(
        (status = 200, description = "Session created", body = CreateSessionResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "External Agents"
)]
pub async fn create_session(
    Path(agent_id): Path<String>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ErrorResponse> {
    let cwd = req
        .working_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let mgr = acp_manager().lock().await;
    let resp = mgr
        .new_session(&agent_id, NewSessionRequest::new(cwd))
        .await
        .map_err(|e| ErrorResponse::internal(format!("Session creation failed: {e}")))?;

    Ok(Json(CreateSessionResponse {
        session_id: resp.session_id.0.to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/agents/external/{agent_id}/prompt",
    params(("agent_id" = String, Path, description = "Agent identifier")),
    request_body = PromptAgentRequest,
    responses(
        (status = 200, description = "Prompt response", body = PromptAgentResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "External Agents"
)]
pub async fn prompt_agent(
    Path(agent_id): Path<String>,
    Json(req): Json<PromptAgentRequest>,
) -> Result<Json<PromptAgentResponse>, ErrorResponse> {
    let session_id = SessionId::from(req.session_id);
    let mgr = acp_manager().lock().await;
    let text = mgr
        .prompt_agent_text(&agent_id, &session_id, &req.text)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Prompt failed: {e}")))?;

    Ok(Json(PromptAgentResponse { text }))
}

#[utoipa::path(
    post,
    path = "/agents/external/{agent_id}/mode",
    params(("agent_id" = String, Path, description = "Agent identifier")),
    request_body = SetModeAgentRequest,
    responses(
        (status = 200, description = "Mode set"),
        (status = 500, description = "Internal server error")
    ),
    tag = "External Agents"
)]
pub async fn set_mode(
    Path(agent_id): Path<String>,
    Json(req): Json<SetModeAgentRequest>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let session_id = SessionId::from(req.session_id);
    let mgr = acp_manager().lock().await;
    mgr.set_mode(
        &agent_id,
        SetSessionModeRequest::new(session_id, SessionModeId::from(req.mode_id)),
    )
    .await
    .map_err(|e| ErrorResponse::internal(format!("Set mode failed: {e}")))?;

    Ok(Json(serde_json::json!({"ok": true})))
}

#[utoipa::path(
    get,
    path = "/agents/external",
    responses((status = 200, description = "List of connected agents", body = AgentListResponse)),
    tag = "External Agents"
)]
pub async fn list_agents() -> Json<AgentListResponse> {
    let mgr = acp_manager().lock().await;
    let agents = mgr.list_agents().await;
    Json(AgentListResponse { agents })
}

#[utoipa::path(
    delete,
    path = "/agents/external/{agent_id}",
    params(("agent_id" = String, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "Agent disconnected"),
        (status = 500, description = "Internal server error")
    ),
    tag = "External Agents"
)]
pub async fn disconnect_agent(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let mgr = acp_manager().lock().await;
    mgr.disconnect_agent(&agent_id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Disconnect failed: {e}")))?;

    // Unregister from slot registry
    state.agent_slot_registry.unregister_agent(&agent_id).await;

    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BuiltinAgentMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub tool_groups: Vec<String>,
    pub recommended_extensions: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BuiltinAgentInfo {
    pub name: String,
    pub description: String,
    pub status: String,
    pub modes: Vec<BuiltinAgentMode>,
    pub default_mode: String,
    pub enabled: bool,
    pub bound_extensions: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BuiltinAgentsResponse {
    pub agents: Vec<BuiltinAgentInfo>,
}

#[utoipa::path(
    get,
    path = "/agents/builtin",
    responses(
        (status = 200, description = "List builtin agents with their modes", body = BuiltinAgentsResponse)
    ),
    tag = "Builtin Agents"
)]
pub async fn list_builtin_agents(
    State(state): State<Arc<AppState>>,
) -> Json<BuiltinAgentsResponse> {
    use goose::agents::developer_agent::DeveloperAgent;
    use goose::agents::goose_agent::GooseAgent;

    let goose = GooseAgent::new();
    let dev = DeveloperAgent::new();

    fn format_tool_group(tg: &goose::registry::manifest::ToolGroupAccess) -> String {
        match tg {
            goose::registry::manifest::ToolGroupAccess::Full(name) => name.clone(),
            goose::registry::manifest::ToolGroupAccess::Restricted { group, .. } => {
                format!("{} (restricted)", group)
            }
        }
    }

    let goose_modes: Vec<BuiltinAgentMode> = goose
        .to_public_agent_modes()
        .into_iter()
        .map(|m| BuiltinAgentMode {
            slug: m.slug.clone(),
            name: m.name.clone(),
            description: m.description.clone(),
            tool_groups: m.tool_groups.iter().map(format_tool_group).collect(),
            recommended_extensions: vec![],
        })
        .collect();

    let dev_modes: Vec<BuiltinAgentMode> = dev
        .to_agent_modes()
        .into_iter()
        .map(|m| {
            let rec_ext = dev.recommended_extensions(&m.slug);
            BuiltinAgentMode {
                slug: m.slug.clone(),
                name: m.name.clone(),
                description: m.description.clone(),
                tool_groups: m.tool_groups.iter().map(format_tool_group).collect(),
                recommended_extensions: rec_ext,
            }
        })
        .collect();

    let goose_enabled = state.agent_slot_registry.is_enabled("Goose Agent").await;
    let dev_enabled = state
        .agent_slot_registry
        .is_enabled("Developer Agent")
        .await;
    let goose_exts: Vec<String> = state
        .agent_slot_registry
        .get_bound_extensions("Goose Agent")
        .await
        .into_iter()
        .collect();
    let dev_exts: Vec<String> = state
        .agent_slot_registry
        .get_bound_extensions("Developer Agent")
        .await
        .into_iter()
        .collect();

    let agents = vec![
        BuiltinAgentInfo {
            name: "Goose Agent".into(),
            description: "Core behavioral modes for the Goose AI assistant".into(),
            status: "active".into(),
            modes: goose_modes,
            default_mode: goose.default_mode_slug().into(),
            enabled: goose_enabled,
            bound_extensions: goose_exts,
        },
        BuiltinAgentInfo {
            name: "Developer Agent".into(),
            description: "Software engineer for writing, debugging, and deploying code".into(),
            status: "active".into(),
            modes: dev_modes,
            default_mode: dev.default_mode().into(),
            enabled: dev_enabled,
            bound_extensions: dev_exts,
        },
    ];

    Json(BuiltinAgentsResponse { agents })
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ToggleAgentResponse {
    pub name: String,
    pub enabled: bool,
}

#[utoipa::path(
    post,
    path = "/agents/builtin/{name}/toggle",
    params(("name" = String, Path, description = "Agent name")),
    responses(
        (status = 200, description = "Agent toggled", body = ToggleAgentResponse),
        (status = 404, description = "Agent not found")
    ),
    tag = "Builtin Agents"
)]
pub async fn toggle_builtin_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ToggleAgentResponse>, StatusCode> {
    let valid_names = ["Goose Agent", "Developer Agent"];
    if !valid_names.contains(&name.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }
    let enabled = state.agent_slot_registry.toggle(&name).await;
    Ok(Json(ToggleAgentResponse { name, enabled }))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BindExtensionRequest {
    pub extension_name: String,
}

#[utoipa::path(
    post,
    path = "/agents/builtin/{name}/extensions/bind",
    params(("name" = String, Path, description = "Agent name")),
    request_body = BindExtensionRequest,
    responses((status = 200, description = "Extension bound")),
    tag = "Builtin Agents"
)]
pub async fn bind_extension_to_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<BindExtensionRequest>,
) -> Result<StatusCode, StatusCode> {
    let valid_names = ["Goose Agent", "Developer Agent"];
    if !valid_names.contains(&name.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }
    state
        .agent_slot_registry
        .bind_extension(&name, &body.extension_name)
        .await;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agents/builtin/{name}/extensions/unbind",
    params(("name" = String, Path, description = "Agent name")),
    request_body = BindExtensionRequest,
    responses((status = 200, description = "Extension unbound")),
    tag = "Builtin Agents"
)]
pub async fn unbind_extension_from_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<BindExtensionRequest>,
) -> Result<StatusCode, StatusCode> {
    let valid_names = ["Goose Agent", "Developer Agent"];
    if !valid_names.contains(&name.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }
    state
        .agent_slot_registry
        .unbind_extension(&name, &body.extension_name)
        .await;
    Ok(StatusCode::OK)
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct OrchestratorStatus {
    pub enabled: bool,
    pub routing_mode: String,
    pub agents: Vec<OrchestratorAgentInfo>,
    pub total_modes: usize,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct OrchestratorAgentInfo {
    pub name: String,
    pub enabled: bool,
    pub mode_count: usize,
    pub default_mode: String,
}

#[utoipa::path(
    get,
    path = "/orchestrator/status",
    responses(
        (status = 200, description = "Orchestrator status")
    ),
    tag = "Orchestrator"
)]
pub async fn orchestrator_status(State(state): State<Arc<AppState>>) -> Json<OrchestratorStatus> {
    use goose::agents::orchestrator_agent::{is_orchestrator_enabled, OrchestratorAgent};

    let provider = Arc::new(tokio::sync::Mutex::new(None));
    let mut router = OrchestratorAgent::new(provider);
    state
        .agent_slot_registry
        .configure_orchestrator(&mut router)
        .await;

    let mut agents = Vec::new();
    let mut total_modes = 0;

    for slot in router.slots() {
        let enabled = state.agent_slot_registry.is_enabled(&slot.name).await;
        let mode_count = slot.modes.len();
        total_modes += mode_count;
        agents.push(OrchestratorAgentInfo {
            name: slot.name.clone(),
            enabled,
            mode_count,
            default_mode: slot.default_mode.clone(),
        });
    }

    Json(OrchestratorStatus {
        enabled: is_orchestrator_enabled(),
        routing_mode: if is_orchestrator_enabled() {
            "llm".to_string()
        } else {
            "keyword".to_string()
        },
        agents,
        total_modes,
    })
}

// ---------------------------------------------------------------------------
// Unified Agent Catalog — merges builtin + external + A2A into one view
// ---------------------------------------------------------------------------

#[derive(Serialize, utoipa::ToSchema)]
pub struct CatalogAgent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: CatalogAgentKind,
    pub status: CatalogAgentStatus,
    pub modes: Vec<CatalogAgentMode>,
    pub default_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatalogAgentKind {
    Builtin,
    External,
    A2a,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatalogAgentStatus {
    Active,
    Disabled,
    Connected,
    Unreachable,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CatalogAgentMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub tool_groups: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentCatalogResponse {
    pub agents: Vec<CatalogAgent>,
    pub total: usize,
}

#[utoipa::path(
    get,
    path = "/agents/catalog",
    responses(
        (status = 200, description = "Unified agent catalog", body = AgentCatalogResponse)
    ),
    tag = "Agent Catalog"
)]
pub async fn agent_catalog(State(state): State<Arc<AppState>>) -> Json<AgentCatalogResponse> {
    use goose::agents::orchestrator_agent::OrchestratorAgent;

    let mut agents = Vec::new();

    // 1. Builtin agents from orchestrator slots
    let provider = Arc::new(tokio::sync::Mutex::new(None));
    let mut router = OrchestratorAgent::new(provider);
    state
        .agent_slot_registry
        .configure_orchestrator(&mut router)
        .await;

    for slot in router.slots() {
        let enabled = state.agent_slot_registry.is_enabled(&slot.name).await;
        let modes: Vec<CatalogAgentMode> = slot
            .modes
            .iter()
            .map(|m| CatalogAgentMode {
                slug: m.slug.clone(),
                name: m.name.clone(),
                description: m.description.clone(),
                tool_groups: m
                    .tool_groups
                    .iter()
                    .map(|tg| match tg {
                        goose::registry::manifest::ToolGroupAccess::Full(name) => name.clone(),
                        goose::registry::manifest::ToolGroupAccess::Restricted {
                            group, ..
                        } => format!("{} (restricted)", group),
                    })
                    .collect(),
            })
            .collect();

        let default_mode = slot
            .modes
            .first()
            .map(|m| m.slug.clone())
            .unwrap_or_default();

        agents.push(CatalogAgent {
            id: slot.name.to_lowercase().replace(' ', "-"),
            name: slot.name.clone(),
            description: slot.description.clone(),
            kind: CatalogAgentKind::Builtin,
            status: if enabled {
                CatalogAgentStatus::Active
            } else {
                CatalogAgentStatus::Disabled
            },
            modes,
            default_mode,
            url: None,
            capabilities: vec!["in-process".into()],
        });
    }

    // 2. External ACP agents
    let mgr = acp_manager().lock().await;
    for id in mgr.list_agents().await {
        agents.push(CatalogAgent {
            id: id.clone(),
            name: id.clone(),
            description: "External ACP agent".into(),
            kind: CatalogAgentKind::External,
            status: CatalogAgentStatus::Connected,
            modes: vec![],
            default_mode: String::new(),
            url: None,
            capabilities: vec!["acp".into()],
        });
    }

    // 3. A2A agent instances from the agent pool
    for snap in state.agent_pool.status_all().await {
        agents.push(CatalogAgent {
            id: snap.id.clone(),
            name: if snap.persona.is_empty() {
                snap.id.clone()
            } else {
                snap.persona.clone()
            },
            description: format!("Agent instance ({})", snap.status),
            kind: CatalogAgentKind::A2a,
            status: match snap.status {
                goose::execution::pool::InstanceStatus::Running => CatalogAgentStatus::Active,
                goose::execution::pool::InstanceStatus::Completed => CatalogAgentStatus::Disabled,
                goose::execution::pool::InstanceStatus::Failed => CatalogAgentStatus::Unreachable,
                goose::execution::pool::InstanceStatus::Cancelled => CatalogAgentStatus::Disabled,
            },
            modes: vec![],
            default_mode: String::new(),
            url: Some(format!("/a2a/instances/{}", snap.id)),
            capabilities: vec!["a2a".into(), "streaming".into()],
        });
    }

    let total = agents.len();
    Json(AgentCatalogResponse { agents, total })
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Unified catalog
        .route("/agents/catalog", get(agent_catalog))
        // Builtin agent routes
        .route("/agents/builtin", get(list_builtin_agents))
        .route("/agents/builtin/{name}/toggle", post(toggle_builtin_agent))
        .route(
            "/agents/builtin/{name}/extensions/bind",
            post(bind_extension_to_agent),
        )
        .route(
            "/agents/builtin/{name}/extensions/unbind",
            post(unbind_extension_from_agent),
        )
        // External agent routes
        .route("/agents/external/connect", post(connect_agent))
        .route("/agents/external/{agent_id}/session", post(create_session))
        .route("/agents/external/{agent_id}/prompt", post(prompt_agent))
        .route("/agents/external/{agent_id}/mode", post(set_mode))
        .route("/agents/external", get(list_agents))
        .route("/agents/external/{agent_id}", delete(disconnect_agent))
        // Orchestrator status
        .route("/orchestrator/status", get(orchestrator_status))
        .with_state(state)
}
