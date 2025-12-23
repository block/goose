use crate::routes::errors::ErrorResponse;
use crate::routes::recipe_utils::{
    apply_recipe_to_agent, build_recipe_with_parameter_values, load_recipe_by_id, validate_recipe,
};
use crate::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use goose::agents::ExtensionConfig;
use goose::config::Config;
use goose::model::ModelConfig;
use goose::prompt_template::render_global_file;
use goose::providers::create;
use goose::recipe::Recipe;
use goose::recipe_deeplink;
use goose::session::session_manager::SessionInsights;
use goose::session::session_manager::SessionType;
use goose::session::{Session, SessionManager};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
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
pub struct UpdateSessionUserRecipeValuesRequest {
    /// Recipe parameter values entered by the user
    user_recipe_values: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateSessionUserRecipeValuesResponse {
    recipe: Recipe,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionRequest {
    pub json: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum EditType {
    Fork,
    Edit,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditMessageRequest {
    timestamp: i64,
    #[serde(default = "default_edit_type")]
    edit_type: EditType,
}

fn default_edit_type() -> EditType {
    EditType::Fork
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditMessageResponse {
    session_id: String,
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
            Ok(Some(built_recipe)) => {
                if let Some(prompt) = apply_recipe_to_agent(&agent, &built_recipe, true).await {
                    update_prompt = prompt;
                }
            }
            Ok(None) => {}
            Err(e) => {
                return Err(ErrorResponse {
                    message: e.to_string(),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    }

    agent.extend_system_prompt(update_prompt).await;

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
    path = "/sessions/{session_id}/user_recipe_values",
    request_body = UpdateSessionUserRecipeValuesRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "User recipe values updated successfully", body = UpdateSessionUserRecipeValuesResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn update_session_user_recipe_values(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionUserRecipeValuesRequest>,
) -> Result<Json<UpdateSessionUserRecipeValuesResponse>, ErrorResponse> {
    SessionManager::update_session(&session_id)
        .user_recipe_values(Some(request.user_recipe_values))
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

    let recipe = session.recipe.ok_or_else(|| ErrorResponse {
        message: "Recipe not found".to_string(),
        status: StatusCode::NOT_FOUND,
    })?;

    let user_recipe_values = session.user_recipe_values.unwrap_or_default();
    match build_recipe_with_parameter_values(&recipe, user_recipe_values).await {
        Ok(Some(recipe)) => {
            let agent = state
                .get_agent_for_route(session_id.clone())
                .await
                .map_err(|status| ErrorResponse {
                    message: format!("Failed to get agent: {}", status),
                    status,
                })?;
            if let Some(prompt) = apply_recipe_to_agent(&agent, &recipe, false).await {
                agent.extend_system_prompt(prompt).await;
            }
            Ok(Json(UpdateSessionUserRecipeValuesResponse { recipe }))
        }
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

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/edit_message",
    request_body = EditMessageRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session prepared for editing - frontend should submit the edited message", body = EditMessageResponse),
        (status = 400, description = "Bad request - Invalid message timestamp"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session or message not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn edit_message(
    Path(session_id): Path<String>,
    Json(request): Json<EditMessageRequest>,
) -> Result<Json<EditMessageResponse>, StatusCode> {
    match request.edit_type {
        EditType::Fork => {
            let new_session = SessionManager::copy_session(&session_id, "(edited)".to_string())
                .await
                .map_err(|e| {
                    tracing::error!("Failed to copy session: {}", e);
                    goose::posthog::emit_error("session_copy_failed", &e.to_string());
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            SessionManager::truncate_conversation(&new_session.id, request.timestamp)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to truncate conversation: {}", e);
                    goose::posthog::emit_error("session_truncate_failed", &e.to_string());
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            Ok(Json(EditMessageResponse {
                session_id: new_session.id,
            }))
        }
        EditType::Edit => {
            SessionManager::truncate_conversation(&session_id, request.timestamp)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to truncate conversation: {}", e);
                    goose::posthog::emit_error("session_truncate_failed", &e.to_string());
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            Ok(Json(EditMessageResponse {
                session_id: session_id.clone(),
            }))
        }
    }
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
        .route("/sessions/{session_id}/export", get(export_session))
        .route("/sessions/import", post(import_session))
        .route("/sessions/insights", get(get_session_insights))
        .route("/sessions/{session_id}/name", put(update_session_name))
        .route(
            "/sessions/{session_id}/user_recipe_values",
            put(update_session_user_recipe_values),
        )
        .route("/sessions/{session_id}/edit_message", post(edit_message))
        .with_state(state)
}
