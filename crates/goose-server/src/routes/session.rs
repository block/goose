use crate::routes::errors::ErrorResponse;
use crate::routes::recipe_utils::build_recipe_with_parameter_values;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use goose::prompt_template::render_global_file;
use goose::recipe::Recipe;
use goose::session::session_manager::SessionInsights;
use goose::session::{Session, SessionManager};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
pub struct UpdateSessionDescriptionRequest {
    /// Updated description (name) for the session (max 200 characters)
    description: String,
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

const MAX_DESCRIPTION_LENGTH: usize = 200;

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
    path = "/sessions/{session_id}/description",
    request_body = UpdateSessionDescriptionRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session description updated successfully"),
        (status = 400, description = "Bad request - Description too long (max 200 characters)"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn update_session_description(
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionDescriptionRequest>,
) -> Result<StatusCode, StatusCode> {
    if request.description.len() > MAX_DESCRIPTION_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    SessionManager::update_session(&session_id)
        .description(request.description)
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
    match build_recipe_with_parameter_values(&session_id, request.user_recipe_values).await {
        Ok(Some(recipe)) => {
            let agent = state
                .get_agent_for_route(session_id.clone())
                .await
                .map_err(|status| ErrorResponse {
                    message: format!("Failed to get agent: {}", status),
                    status,
                })?;
            if let Some(instructions) = &recipe.instructions {
                let mut context: HashMap<&str, Value> = HashMap::new();
                context.insert("recipe_instructions", Value::String(instructions.clone()));
                let prompt = render_global_file("desktop_recipe_instruction.md", &context)
                    .expect("Prompt should render");
                agent.extend_system_prompt(prompt).await;
            }
            if let Some(sub_recipes) = &recipe.sub_recipes {
                agent.add_sub_recipes(sub_recipes.clone()).await;
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

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session))
        .route("/sessions/{session_id}", delete(delete_session))
        .route("/sessions/{session_id}/export", get(export_session))
        .route("/sessions/import", post(import_session))
        .route("/sessions/insights", get(get_session_insights))
        .route(
            "/sessions/{session_id}/description",
            put(update_session_description),
        )
        .route(
            "/sessions/{session_id}/user_recipe_values",
            put(update_session_user_recipe_values),
        )
        .with_state(state)
}
