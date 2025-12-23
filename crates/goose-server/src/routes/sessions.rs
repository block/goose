use crate::routes::errors::ErrorResponse;
use crate::routes::recipe_utils::{
    build_recipe_with_parameter_values, load_recipe_by_id, reconcile_recipe_state, validate_recipe,
};
use crate::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use goose::agents::extension::ToolInfo;
use goose::agents::extension_manager::get_parameter_names;
use goose::agents::ExtensionConfig;
use goose::config::permission::PermissionLevel;
use goose::config::{Config, GooseMode, PermissionManager};
use goose::model::ModelConfig;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};
use goose::providers::create;
use goose::recipe::Recipe;
use goose::recipe_deeplink;
use goose::session::session_manager::SessionInsights;
use goose::session::session_manager::SessionType;
use goose::session::{Session, SessionManager};
use rmcp::model::{CallToolRequestParam, Content};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    /// List of available session information objects
    pub sessions: Vec<Session>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionNameRequest {
    /// Updated name for the session (max 200 characters)
    name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecipeRequest {
    /// Recipe parameter values entered by the user
    values: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateRecipeResponse {
    recipe: Recipe,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionRequest {
    pub json: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateSessionRequest {
    pub working_dir: String,
    #[serde(default)]
    pub recipe: Option<Recipe>,
    #[serde(default)]
    pub recipe_id: Option<String>,
    #[serde(default)]
    pub recipe_deeplink: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct OpenSessionRequest {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct AddSessionExtensionRequest {
    pub config: ExtensionConfig,
}

const MAX_NAME_LENGTH: usize = 200;

async fn setup_agent_for_session(
    state: &AppState,
    session: &Session,
    provider_override: Option<String>,
    model_override: Option<String>,
) -> Result<(), ErrorResponse> {
    let agent = state
        .get_agent_for_route(session.id.clone())
        .await
        .map_err(|code| ErrorResponse {
            message: "Failed to get agent".into(),
            status: code,
        })?;

    let config = Config::global();

    let provider_name = provider_override
        .or_else(|| session.provider_name.clone())
        .or_else(|| config.get_goose_provider().ok())
        .ok_or_else(|| ErrorResponse {
            message: "No provider configured".into(),
            status: StatusCode::BAD_REQUEST,
        })?;

    let model_config = if let Some(model) = model_override {
        ModelConfig::new(&model).map_err(|e| ErrorResponse {
            message: format!("Invalid model: {}", e),
            status: StatusCode::BAD_REQUEST,
        })?
    } else if let Some(saved_config) = session.model_config.clone() {
        saved_config
    } else {
        let model_name = config.get_goose_model().map_err(|_| ErrorResponse {
            message: "No model configured".into(),
            status: StatusCode::BAD_REQUEST,
        })?;
        ModelConfig::new(&model_name).map_err(|e| ErrorResponse {
            message: format!("Invalid model: {}", e),
            status: StatusCode::BAD_REQUEST,
        })?
    };

    let provider = create(&provider_name, model_config)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to create provider: {}", e),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    agent
        .update_provider(provider, &session.id)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to set provider: {}", e),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    // Idempotently reconcile recipe state from the session
    reconcile_recipe_state(&agent, session, true).await?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/sessions",
    request_body = CreateSessionRequest,
    responses(
        (status = 200, description = "Session created successfully", body = Session),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<Session>, ErrorResponse> {
    goose::posthog::set_session_context("desktop", false);

    let CreateSessionRequest {
        working_dir,
        recipe,
        recipe_id,
        recipe_deeplink: recipe_deeplink_str,
        provider,
        model,
    } = payload;

    let original_recipe = if let Some(deeplink) = recipe_deeplink_str {
        match recipe_deeplink::decode(&deeplink) {
            Ok(recipe) => Some(recipe),
            Err(err) => {
                error!("Failed to decode recipe deeplink: {}", err);
                goose::posthog::emit_error("recipe_deeplink_decode_failed", &err.to_string());
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
                goose::posthog::emit_error("session_create_failed", &err.to_string());
                ErrorResponse {
                    message: format!("Failed to create session: {}", err),
                    status: StatusCode::BAD_REQUEST,
                }
            })?;

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

        session = SessionManager::get_session(&session.id, false)
            .await
            .map_err(|err| {
                error!("Failed to get updated session: {}", err);
                ErrorResponse {
                    message: format!("Failed to get updated session: {}", err),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
    }

    setup_agent_for_session(&state, &session, provider, model).await?;

    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/open",
    request_body = OpenSessionRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session opened successfully", body = Session),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn open_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(payload): Json<OpenSessionRequest>,
) -> Result<Json<Session>, ErrorResponse> {
    goose::posthog::set_session_context("desktop", true);

    let session = SessionManager::get_session(&session_id, true)
        .await
        .map_err(|err| {
            error!("Failed to load session {}: {}", session_id, err);
            goose::posthog::emit_error("session_open_failed", &err.to_string());
            ErrorResponse {
                message: format!("Session not found: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    setup_agent_for_session(&state, &session, payload.provider, payload.model).await?;

    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/extensions",
    request_body = AddSessionExtensionRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Extension added successfully"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn add_session_extension(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<AddSessionExtensionRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let extension_name = request.config.name();
    let agent = state.get_agent(session_id).await?;

    agent.add_extension(request.config).await.map_err(|e| {
        goose::posthog::emit_error(
            "extension_add_failed",
            &format!("{}: {}", extension_name, e),
        );
        ErrorResponse::internal(format!("Failed to add extension: {}", e))
    })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/sessions",
    responses(
        (status = 200, description = "List of available sessions retrieved successfully", body = SessionListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn list_sessions() -> Result<Json<SessionListResponse>, StatusCode> {
    let sessions = SessionManager::list_sessions()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SessionListResponse { sessions }))
}

#[utoipa::path(
    get,
    path = "/sessions/{session_id}",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session history retrieved successfully", body = Session),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn get_session(Path(session_id): Path<String>) -> Result<Json<Session>, StatusCode> {
    let session = SessionManager::get_session(&session_id, true)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(session))
}

#[utoipa::path(
    get,
    path = "/sessions/insights",
    responses(
        (status = 200, description = "Session insights retrieved successfully", body = SessionInsights),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn get_session_insights() -> Result<Json<SessionInsights>, StatusCode> {
    let insights = SessionManager::get_insights()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(insights))
}

#[utoipa::path(
    put,
    path = "/sessions/{session_id}/name",
    request_body = UpdateSessionNameRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session name updated successfully"),
        (status = 400, description = "Bad request - Name is empty or too long"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn update_session_name(
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionNameRequest>,
) -> Result<StatusCode, StatusCode> {
    let name = request.name.trim();
    if name.is_empty() || name.len() > MAX_NAME_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    SessionManager::update_session(&session_id)
        .user_provided_name(name.to_string())
        .apply()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    put,
    path = "/sessions/{session_id}/recipe",
    request_body = UpdateRecipeRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Recipe values updated successfully", body = UpdateRecipeResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn update_session_recipe(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<UpdateRecipeRequest>,
) -> Result<Json<UpdateRecipeResponse>, ErrorResponse> {
    SessionManager::update_session(&session_id)
        .user_recipe_values(Some(request.values))
        .apply()
        .await
        .map_err(|err| ErrorResponse {
            message: err.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let session = SessionManager::get_session(&session_id, false)
        .await
        .map_err(|err| ErrorResponse {
            message: err.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let recipe = session.recipe.clone().ok_or_else(|| ErrorResponse {
        message: "Recipe not found".to_string(),
        status: StatusCode::NOT_FOUND,
    })?;

    let agent = state
        .get_agent_for_route(session_id.clone())
        .await
        .map_err(|status| ErrorResponse {
            message: format!("Failed to get agent: {}", status),
            status,
        })?;

    // Use reconcile_recipe_state for idempotent updates
    // Note: we use false for include_final_output_tool since this is an update, not initial setup
    reconcile_recipe_state(&agent, &session, false).await?;

    let user_recipe_values = session.user_recipe_values.unwrap_or_default();
    match build_recipe_with_parameter_values(&recipe, user_recipe_values).await {
        Ok(Some(built_recipe)) => Ok(Json(UpdateRecipeResponse {
            recipe: built_recipe,
        })),
        Ok(None) => Err(ErrorResponse {
            message: "Missing required parameters".to_string(),
            status: StatusCode::BAD_REQUEST,
        }),
        Err(e) => Err(ErrorResponse {
            message: e.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }),
    }
}

#[utoipa::path(
    delete,
    path = "/sessions/{session_id}",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session deleted successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn delete_session(Path(session_id): Path<String>) -> Result<StatusCode, StatusCode> {
    SessionManager::delete_session(&session_id)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/sessions/{session_id}/export",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session exported successfully", body = String),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn export_session(Path(session_id): Path<String>) -> Result<Json<String>, StatusCode> {
    let exported = SessionManager::export_session(&session_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(exported))
}

#[utoipa::path(
    post,
    path = "/sessions/import",
    request_body = ImportSessionRequest,
    responses(
        (status = 200, description = "Session imported successfully", body = Session),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 400, description = "Bad request - Invalid JSON"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn import_session(
    Json(request): Json<ImportSessionRequest>,
) -> Result<Json<Session>, StatusCode> {
    let session = SessionManager::import_session(&request.json)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(session))
}

// Session-centric endpoints (moved from agent.rs and action_required.rs)

#[derive(Deserialize, ToSchema)]
pub struct GetToolsQuery {
    extension_name: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReadResourceRequest {
    extension_name: String,
    uri: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ReadResourceResponse {
    html: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CallToolRequest {
    arguments: Value,
}

#[derive(Serialize, ToSchema)]
pub struct CallToolResponse {
    content: Vec<Content>,
    structured_content: Option<Value>,
    is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    _meta: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmToolActionRequest {
    id: String,
    #[serde(default = "default_principal_type")]
    principal_type: PrincipalType,
    action: String,
}

fn default_principal_type() -> PrincipalType {
    PrincipalType::Tool
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForkMessageResponse {
    session_id: String,
}

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/messages/{timestamp}/fork",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session"),
        ("timestamp" = i64, Path, description = "Message timestamp to fork at")
    ),
    responses(
        (status = 200, description = "Session forked successfully", body = ForkMessageResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session or message not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn fork_message(
    Path((session_id, timestamp)): Path<(String, i64)>,
) -> Result<Json<ForkMessageResponse>, StatusCode> {
    let new_session = SessionManager::copy_session(&session_id, "(edited)".to_string())
        .await
        .map_err(|e| {
            tracing::error!("Failed to copy session: {}", e);
            goose::posthog::emit_error("session_copy_failed", &e.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    SessionManager::truncate_conversation(&new_session.id, timestamp)
        .await
        .map_err(|e| {
            tracing::error!("Failed to truncate conversation: {}", e);
            goose::posthog::emit_error("session_truncate_failed", &e.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ForkMessageResponse {
        session_id: new_session.id,
    }))
}

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/messages/{timestamp}/edit",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session"),
        ("timestamp" = i64, Path, description = "Message timestamp to edit at")
    ),
    responses(
        (status = 200, description = "Session truncated for editing"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session or message not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn edit_message_in_place(
    Path((session_id, timestamp)): Path<(String, i64)>,
) -> Result<StatusCode, StatusCode> {
    SessionManager::truncate_conversation(&session_id, timestamp)
        .await
        .map_err(|e| {
            tracing::error!("Failed to truncate conversation: {}", e);
            goose::posthog::emit_error("session_truncate_failed", &e.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/sessions/{session_id}/extensions/{extension_name}",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session"),
        ("extension_name" = String, Path, description = "Name of the extension to remove")
    ),
    responses(
        (status = 200, description = "Extension removed successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session or extension not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn remove_session_extension(
    State(state): State<Arc<AppState>>,
    Path((session_id, extension_name)): Path<(String, String)>,
) -> Result<StatusCode, ErrorResponse> {
    let agent = state.get_agent(session_id).await?;
    agent.remove_extension(&extension_name).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/sessions/{session_id}/tools",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session"),
        ("extension_name" = Option<String>, Query, description = "Optional extension name to filter tools")
    ),
    responses(
        (status = 200, description = "Tools retrieved successfully", body = Vec<ToolInfo>),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn get_session_tools(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<GetToolsQuery>,
) -> Result<Json<Vec<ToolInfo>>, StatusCode> {
    let config = Config::global();
    let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
    let agent = state.get_agent_for_route(session_id).await?;
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
    path = "/sessions/{session_id}/tools/{tool_name}/call",
    request_body = CallToolRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session"),
        ("tool_name" = String, Path, description = "Name of the tool to call")
    ),
    responses(
        (status = 200, description = "Tool called successfully", body = CallToolResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 424, description = "Agent not initialized"),
        (status = 404, description = "Tool not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn call_session_tool(
    State(state): State<Arc<AppState>>,
    Path((session_id, tool_name)): Path<(String, String)>,
    Json(payload): Json<CallToolRequest>,
) -> Result<Json<CallToolResponse>, StatusCode> {
    let agent = state.get_agent_for_route(session_id).await?;

    let arguments = match payload.arguments {
        Value::Object(map) => Some(map),
        _ => None,
    };

    let tool_call = CallToolRequestParam {
        name: tool_name.into(),
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
        _meta: None,
    }))
}

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/resources/read",
    request_body = ReadResourceRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Resource read successfully", body = ReadResourceResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 424, description = "Agent not initialized"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn read_session_resource(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(payload): Json<ReadResourceRequest>,
) -> Result<Json<ReadResourceResponse>, StatusCode> {
    let agent = state.get_agent_for_route(session_id).await?;

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
    path = "/sessions/{session_id}/confirmations",
    request_body = ConfirmToolActionRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Tool confirmation action processed", body = Value),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
pub async fn confirm_tool_action(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<ConfirmToolActionRequest>,
) -> Result<Json<Value>, StatusCode> {
    let agent = state.get_agent_for_route(session_id).await?;
    let permission = match request.action.as_str() {
        "always_allow" => Permission::AlwaysAllow,
        "allow_once" => Permission::AllowOnce,
        "deny" => Permission::DenyOnce,
        _ => Permission::DenyOnce,
    };

    agent
        .handle_confirmation(
            request.id.clone(),
            PermissionConfirmation {
                principal_type: request.principal_type,
                permission,
            },
        )
        .await;

    Ok(Json(Value::Object(serde_json::Map::new())))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions", post(create_session))
        .route("/sessions/{session_id}", get(get_session))
        .route("/sessions/{session_id}", delete(delete_session))
        .route("/sessions/{session_id}/open", post(open_session))
        .route(
            "/sessions/{session_id}/extensions",
            post(add_session_extension),
        )
        .route(
            "/sessions/{session_id}/extensions/{extension_name}",
            delete(remove_session_extension),
        )
        .route("/sessions/{session_id}/export", get(export_session))
        .route("/sessions/import", post(import_session))
        .route("/sessions/insights", get(get_session_insights))
        .route("/sessions/{session_id}/name", put(update_session_name))
        .route("/sessions/{session_id}/recipe", put(update_session_recipe))
        .route(
            "/sessions/{session_id}/messages/{timestamp}/fork",
            post(fork_message),
        )
        .route(
            "/sessions/{session_id}/messages/{timestamp}/edit",
            post(edit_message_in_place),
        )
        .route("/sessions/{session_id}/tools", get(get_session_tools))
        .route(
            "/sessions/{session_id}/tools/{tool_name}/call",
            post(call_session_tool),
        )
        .route(
            "/sessions/{session_id}/resources/read",
            post(read_session_resource),
        )
        .route(
            "/sessions/{session_id}/confirmations",
            post(confirm_tool_action),
        )
        .with_state(state)
}
