use crate::routes::errors::ErrorResponse;
use crate::routes::recipe_utils::{apply_recipe_to_agent, build_recipe_with_parameter_values};
use crate::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use goose::recipe::Recipe;
use goose::session::session_manager::SessionInsights;
use goose::session::{Session, SessionManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    /// List of available session information objects
    sessions: Vec<Session>,
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
    json: String,
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

const MAX_NAME_LENGTH: usize = 200;

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
        (status = 400, description = "Bad request - Name too long (max 200 characters)"),
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
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if name.len() > MAX_NAME_LENGTH {
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
        (status = 200, description = "Session user recipe values updated successfully", body = UpdateSessionUserRecipeValuesResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
// Update session user recipe parameter values
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
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            SessionManager::truncate_conversation(&new_session.id, request.timestamp)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to truncate conversation: {}", e);
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
        .route("/sessions/{session_id}", get(get_session))
        .route("/sessions/{session_id}", delete(delete_session))
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
