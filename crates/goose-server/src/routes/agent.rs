use crate::routes::agent_utils::{
    persist_session_extensions, restore_agent_extensions, restore_agent_provider,
    ExtensionLoadResult,
};
use crate::routes::errors::ErrorResponse;
use crate::routes::recipe_utils::{
    apply_recipe_to_agent, build_recipe_with_parameter_values, load_recipe_by_id, validate_recipe,
};
use crate::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use goose::config::PermissionManager;

use goose::agents::ExtensionConfig;
use goose::config::{Config, GooseMode};
use goose::model::ModelConfig;
use goose::prompt_template::render_global_file;
use goose::providers::create;
use goose::recipe::Recipe;
use goose::recipe_deeplink;
use goose::session::extension_data::ExtensionState;
use goose::session::session_manager::SessionType;
use goose::session::{EnabledExtensionsState, Session, SessionManager};
use goose::{
    agents::{extension::ToolInfo, extension_manager::get_parameter_names},
    config::permission::PermissionLevel,
};
use rmcp::model::{CallToolRequestParam, Content};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateFromSessionRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateProviderRequest {
    provider: String,
    model: Option<String>,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GetToolsQuery {
    extension_name: Option<String>,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateRouterToolSelectorRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StartAgentRequest {
    working_dir: String,
    #[serde(default)]
    recipe: Option<Recipe>,
    #[serde(default)]
    recipe_id: Option<String>,
    #[serde(default)]
    recipe_deeplink: Option<String>,
    #[serde(default)]
    extension_overrides: Option<Vec<ExtensionConfig>>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StopAgentRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RestartAgentRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkingDirRequest {
    session_id: String,
    working_dir: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResumeAgentRequest {
    session_id: String,
    load_model_and_extensions: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddExtensionRequest {
    session_id: String,
    config: ExtensionConfig,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RemoveExtensionRequest {
    name: String,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ReadResourceRequest {
    session_id: String,
    extension_name: String,
    uri: String,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReadResourceResponse {
    html: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CallToolRequest {
    session_id: String,
    name: String,
    arguments: Value,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CallToolResponse {
    content: Vec<Content>,
    structured_content: Option<Value>,
    is_error: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ResumeAgentResponse {
    pub session: Session,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_results: Option<Vec<ExtensionLoadResult>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RestartAgentResponse {
    pub extension_results: Vec<ExtensionLoadResult>,
}

#[utoipa::path(
    post,
    path = "/agent/start",
    request_body = StartAgentRequest,
    responses(
        (status = 200, description = "Agent started successfully", body = Session),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn start_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartAgentRequest>,
) -> Result<Json<Session>, ErrorResponse> {
    goose::posthog::set_session_context("desktop", false);

    let StartAgentRequest {
        working_dir,
        recipe,
        recipe_id,
        recipe_deeplink,
        extension_overrides,
    } = payload;

    let original_recipe = if let Some(deeplink) = recipe_deeplink {
        match recipe_deeplink::decode(&deeplink) {
            Ok(recipe) => Some(recipe),
            Err(err) => {
                error!("Failed to decode recipe deeplink: {}", err);
                return Err(ErrorResponse {
                    message: err.to_string(),
                    status: StatusCode::BAD_REQUEST,
                });
            }
        }
    } else if let Some(id) = recipe_id {
        match load_recipe_by_id(state.as_ref(), &id).await {
            Ok(recipe) => Some(recipe),
            Err(err) => return Err(err),
        }
    } else {
        recipe
    };

    if let Some(ref recipe) = original_recipe {
        if let Err(err) = validate_recipe(recipe) {
            return Err(ErrorResponse {
                message: err.message,
                status: err.status,
            });
        }
    }

    let counter = state.session_counter.fetch_add(1, Ordering::SeqCst) + 1;
    let name = format!("New session {}", counter);

    let mut session =
        SessionManager::create_session(PathBuf::from(&working_dir), name, SessionType::User)
            .await
            .map_err(|err| {
                error!("Failed to create session: {}", err);
                ErrorResponse {
                    message: format!("Failed to create session: {}", err),
                    status: StatusCode::BAD_REQUEST,
                }
            })?;

    // Initialize session with extensions (either overrides from hub or global defaults)
    let extensions_to_use =
        extension_overrides.unwrap_or_else(goose::config::get_enabled_extensions);
    let mut extension_data = session.extension_data.clone();
    let extensions_state = EnabledExtensionsState::new(extensions_to_use);
    if let Err(e) = extensions_state.to_extension_data(&mut extension_data) {
        tracing::warn!("Failed to initialize session with extensions: {}", e);
    } else {
        SessionManager::update_session(&session.id)
            .extension_data(extension_data.clone())
            .apply()
            .await
            .map_err(|err| {
                error!("Failed to save initial extension state: {}", err);
                ErrorResponse {
                    message: format!("Failed to save initial extension state: {}", err),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
    }

    if let Some(recipe) = original_recipe {
        SessionManager::update_session(&session.id)
            .recipe(Some(recipe))
            .apply()
            .await
            .map_err(|err| {
                error!("Failed to update session with recipe: {}", err);
                ErrorResponse {
                    message: format!("Failed to update session with recipe: {}", err),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
    }

    // Refetch session to get all updates
    session = SessionManager::get_session(&session.id, false)
        .await
        .map_err(|err| {
            error!("Failed to get updated session: {}", err);
            ErrorResponse {
                message: format!("Failed to get updated session: {}", err),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/agent/resume",
    request_body = ResumeAgentRequest,
    responses(
        (status = 200, description = "Agent started successfully", body = ResumeAgentResponse),
        (status = 400, description = "Bad request - invalid working directory"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error")
    )
)]
async fn resume_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ResumeAgentRequest>,
) -> Result<Json<ResumeAgentResponse>, ErrorResponse> {
    goose::posthog::set_session_context("desktop", true);

    let session = SessionManager::get_session(&payload.session_id, true)
        .await
        .map_err(|err| {
            error!("Failed to resume session {}: {}", payload.session_id, err);
            ErrorResponse {
                message: format!("Failed to resume session: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    let extension_results = if payload.load_model_and_extensions {
        let agent = state
            .get_agent_for_route(payload.session_id.clone())
            .await
            .map_err(|code| ErrorResponse {
                message: "Failed to get agent for route".into(),
                status: code,
            })?;

        let provider_result = restore_agent_provider(&agent, &session, &payload.session_id);
        let extensions_future = restore_agent_extensions(agent.clone(), &session);

        let (provider_result, extension_results) = tokio::join!(provider_result, extensions_future);
        provider_result?;
        Some(extension_results)
    } else {
        None
    };

    Ok(Json(ResumeAgentResponse {
        session,
        extension_results,
    }))
}

#[utoipa::path(
    post,
    path = "/agent/update_from_session",
    request_body = UpdateFromSessionRequest,
    responses(
        (status = 200, description = "Update agent from session data successfully"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
    ),
)]
async fn update_from_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateFromSessionRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let agent = state
        .get_agent_for_route(payload.session_id.clone())
        .await
        .map_err(|status| ErrorResponse {
            message: format!("Failed to get agent: {}", status),
            status,
        })?;
    let session = SessionManager::get_session(&payload.session_id, false)
        .await
        .map_err(|err| ErrorResponse {
            message: format!("Failed to get session: {}", err),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    let context: HashMap<&str, Value> = HashMap::new();
    let desktop_prompt =
        render_global_file("desktop_prompt.md", &context).expect("Prompt should render");
    let mut update_prompt = desktop_prompt;
    if let Some(recipe) = session.recipe {
        match build_recipe_with_parameter_values(
            &recipe,
            session.user_recipe_values.unwrap_or_default(),
        )
        .await
        {
            Ok(Some(recipe)) => {
                if let Some(prompt) = apply_recipe_to_agent(&agent, &recipe, true).await {
                    update_prompt = prompt;
                }
            }
            Ok(None) => {
                // Recipe has missing parameters - use default prompt
            }
            Err(e) => {
                return Err(ErrorResponse {
                    message: e.to_string(),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    }
    agent.extend_system_prompt(update_prompt).await;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/agent/tools",
    params(
        ("extension_name" = Option<String>, Query, description = "Optional extension name to filter tools"),
        ("session_id" = String, Query, description = "Required session ID to scope tools to a specific session")
    ),
    responses(
        (status = 200, description = "Tools retrieved successfully", body = Vec<ToolInfo>),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_tools(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetToolsQuery>,
) -> Result<Json<Vec<ToolInfo>>, StatusCode> {
    let config = Config::global();
    let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
    let agent = state.get_agent_for_route(query.session_id).await?;
    let permission_manager = PermissionManager::default();

    let mut tools: Vec<ToolInfo> = agent
        .list_tools(query.extension_name)
        .await
        .into_iter()
        .map(|tool| {
            let permission = permission_manager
                .get_user_permission(&tool.name)
                .or_else(|| {
                    if goose_mode == GooseMode::SmartApprove {
                        permission_manager.get_smart_approve_permission(&tool.name)
                    } else if goose_mode == GooseMode::Approve {
                        Some(PermissionLevel::AskBefore)
                    } else {
                        None
                    }
                });

            ToolInfo::new(
                &tool.name,
                tool.description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default(),
                get_parameter_names(&tool),
                permission,
            )
        })
        .collect::<Vec<ToolInfo>>();
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(tools))
}

#[utoipa::path(
    post,
    path = "/agent/update_provider",
    request_body = UpdateProviderRequest,
    responses(
        (status = 200, description = "Provider updated successfully"),
        (status = 400, description = "Bad request - missing or invalid parameters"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_agent_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateProviderRequest>,
) -> Result<(), impl IntoResponse> {
    let agent = state
        .get_agent_for_route(payload.session_id.clone())
        .await
        .map_err(|e| (e, "No agent for session id".to_owned()))?;

    let config = Config::global();
    let model = match payload.model.or_else(|| config.get_goose_model().ok()) {
        Some(m) => m,
        None => {
            return Err((StatusCode::BAD_REQUEST, "No model specified".to_owned()));
        }
    };

    let model_config = ModelConfig::new(&model).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid model config: {}", e),
        )
    })?;

    let new_provider = create(&payload.provider, model_config).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to create {} provider: {}", &payload.provider, e),
        )
    })?;

    agent
        .update_provider(new_provider, &payload.session_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update provider: {}", e),
            )
        })?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/agent/update_router_tool_selector",
    request_body = UpdateRouterToolSelectorRequest,
    responses(
        (status = 200, description = "Tool selection strategy updated successfully", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_router_tool_selector(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRouterToolSelectorRequest>,
) -> Result<Json<String>, StatusCode> {
    let agent = state.get_agent_for_route(payload.session_id).await?;
    agent
        .update_router_tool_selector(None, Some(true))
        .await
        .map_err(|e| {
            error!("Failed to update tool selection strategy: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(
        "Tool selection strategy updated successfully".to_string(),
    ))
}

#[utoipa::path(
    post,
    path = "/agent/add_extension",
    request_body = AddExtensionRequest,
    responses(
        (status = 200, description = "Extension added", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn agent_add_extension(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AddExtensionRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let session = SessionManager::get_session(&request.session_id, false)
        .await
        .map_err(|err| {
            error!("Failed to get session for add_extension: {}", err);
            ErrorResponse {
                message: format!("Failed to get session: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    let agent = state.get_agent(request.session_id.clone()).await?;

    // Set the agent's working directory from the session before adding the extension
    agent.set_working_dir(session.working_dir).await;

    agent
        .add_extension(request.config)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to add extension: {}", e)))?;

    // Persist the updated extension state to the session
    persist_session_extensions(&agent, &request.session_id).await?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/remove_extension",
    request_body = RemoveExtensionRequest,
    responses(
        (status = 200, description = "Extension removed", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn agent_remove_extension(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RemoveExtensionRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let agent = state.get_agent(request.session_id.clone()).await?;
    agent.remove_extension(&request.name).await?;

    // Persist the updated extension state to the session
    persist_session_extensions(&agent, &request.session_id).await?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/stop",
    request_body = StopAgentRequest,
    responses(
        (status = 200, description = "Agent stopped successfully", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn stop_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StopAgentRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let session_id = payload.session_id;
    state
        .agent_manager
        .remove_session(&session_id)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to stop agent for session {}: {}", session_id, e),
            status: StatusCode::NOT_FOUND,
        })?;

    Ok(StatusCode::OK)
}

async fn restart_agent_internal(
    state: &Arc<AppState>,
    session_id: &str,
    session: &Session,
) -> Result<Vec<ExtensionLoadResult>, ErrorResponse> {
    // Remove existing agent (ignore error if not found)
    let _ = state.agent_manager.remove_session(session_id).await;

    let agent = state
        .get_agent_for_route(session_id.to_string())
        .await
        .map_err(|code| ErrorResponse {
            message: "Failed to create new agent during restart".into(),
            status: code,
        })?;

    let provider_result = restore_agent_provider(&agent, session, session_id);
    let extensions_future = restore_agent_extensions(agent.clone(), session);

    let (provider_result, extension_results) = tokio::join!(provider_result, extensions_future);
    provider_result?;

    let context: HashMap<&str, Value> = HashMap::new();
    let desktop_prompt =
        render_global_file("desktop_prompt.md", &context).expect("Prompt should render");
    let mut update_prompt = desktop_prompt;

    if let Some(ref recipe) = session.recipe {
        match build_recipe_with_parameter_values(
            recipe,
            session.user_recipe_values.clone().unwrap_or_default(),
        )
        .await
        {
            Ok(Some(recipe)) => {
                if let Some(prompt) = apply_recipe_to_agent(&agent, &recipe, true).await {
                    update_prompt = prompt;
                }
            }
            Ok(None) => {
                // Recipe has missing parameters - use default prompt
            }
            Err(e) => {
                return Err(ErrorResponse {
                    message: e.to_string(),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    }
    agent.extend_system_prompt(update_prompt).await;

    Ok(extension_results)
}

#[utoipa::path(
    post,
    path = "/agent/restart",
    request_body = RestartAgentRequest,
    responses(
        (status = 200, description = "Agent restarted successfully", body = RestartAgentResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn restart_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RestartAgentRequest>,
) -> Result<Json<RestartAgentResponse>, ErrorResponse> {
    let session_id = payload.session_id.clone();

    let session = SessionManager::get_session(&session_id, false)
        .await
        .map_err(|err| {
            error!("Failed to get session during restart: {}", err);
            ErrorResponse {
                message: format!("Failed to get session: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    let extension_results = restart_agent_internal(&state, &session_id, &session).await?;

    Ok(Json(RestartAgentResponse { extension_results }))
}

#[utoipa::path(
    post,
    path = "/agent/update_working_dir",
    request_body = UpdateWorkingDirRequest,
    responses(
        (status = 200, description = "Working directory updated and agent restarted successfully"),
        (status = 400, description = "Bad request - invalid directory path"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_working_dir(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateWorkingDirRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let session_id = payload.session_id.clone();
    let working_dir = payload.working_dir.trim();

    if working_dir.is_empty() {
        return Err(ErrorResponse {
            message: "Working directory cannot be empty".into(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let path = PathBuf::from(working_dir);
    if !path.exists() || !path.is_dir() {
        return Err(ErrorResponse {
            message: "Invalid directory path".into(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    // Update the session's working directory
    SessionManager::update_session(&session_id)
        .working_dir(path)
        .apply()
        .await
        .map_err(|e| {
            error!("Failed to update session working directory: {}", e);
            ErrorResponse {
                message: format!("Failed to update working directory: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    // Get the updated session and restart the agent
    let session = SessionManager::get_session(&session_id, false)
        .await
        .map_err(|err| {
            error!("Failed to get session after working dir update: {}", err);
            ErrorResponse {
                message: format!("Failed to get session: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    restart_agent_internal(&state, &session_id, &session).await?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/read_resource",
    request_body = ReadResourceRequest,
    responses(
        (status = 200, description = "Resource read successfully", body = ReadResourceResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn read_resource(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReadResourceRequest>,
) -> Result<Json<ReadResourceResponse>, StatusCode> {
    let agent = state
        .get_agent_for_route(payload.session_id.clone())
        .await?;

    let html = agent
        .extension_manager
        .read_ui_resource(
            &payload.uri,
            &payload.extension_name,
            CancellationToken::default(),
        )
        .await
        .map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ReadResourceResponse { html }))
}

#[utoipa::path(
    post,
    path = "/agent/call_tool",
    request_body = CallToolRequest,
    responses(
        (status = 200, description = "Resource read successfully", body = CallToolResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn call_tool(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CallToolRequest>,
) -> Result<Json<CallToolResponse>, StatusCode> {
    let agent = state
        .get_agent_for_route(payload.session_id.clone())
        .await?;

    let arguments = match payload.arguments {
        Value::Object(map) => Some(map),
        _ => None,
    };

    let tool_call = CallToolRequestParam {
        name: payload.name.into(),
        arguments,
    };

    let tool_result = agent
        .extension_manager
        .dispatch_tool_call(tool_call, CancellationToken::default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = tool_result
        .result
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CallToolResponse {
        content: result.content,
        structured_content: result.structured_content,
        is_error: result.is_error.unwrap_or(false),
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/agent/start", post(start_agent))
        .route("/agent/resume", post(resume_agent))
        .route("/agent/restart", post(restart_agent))
        .route("/agent/update_working_dir", post(update_working_dir))
        .route("/agent/tools", get(get_tools))
        .route("/agent/read_resource", post(read_resource))
        .route("/agent/call_tool", post(call_tool))
        .route("/agent/update_provider", post(update_agent_provider))
        .route(
            "/agent/update_router_tool_selector",
            post(update_router_tool_selector),
        )
        .route("/agent/update_from_session", post(update_from_session))
        .route("/agent/add_extension", post(agent_add_extension))
        .route("/agent/remove_extension", post(agent_remove_extension))
        .route("/agent/stop", post(stop_agent))
        .with_state(state)
}
