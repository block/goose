use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use goose::goose_apps::{GooseApp, GooseAppsManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppListResponse {
    pub apps: Vec<GooseApp>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppResponse {
    pub app: GooseApp,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppRequest {
    pub app: GooseApp,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppRequest {
    pub app: GooseApp,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub message: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: String,
}

#[utoipa::path(
    get,
    path = "/apps/list_apps",
    responses(
        (status = 200, description = "List of installed apps retrieved successfully", body = AppListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn list_apps() -> Result<Json<AppListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = GooseAppsManager::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to initialize apps manager: {}", e),
            }),
        )
    })?;

    let apps = manager.list_apps().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list apps: {}", e),
            }),
        )
    })?;

    Ok(Json(AppListResponse { apps }))
}

#[utoipa::path(
    get,
    path = "/apps/{name}",
    responses(
        (status = 200, description = "App retrieved successfully", body = AppResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 404, description = "App not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("name" = String, Path, description = "Name of the app")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn get_app(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<AppResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = GooseAppsManager::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to initialize apps manager: {}", e),
            }),
        )
    })?;

    let app = manager.get_app(&name).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get app: {}", e),
            }),
        )
    })?;

    match app {
        Some(app) => Ok(Json(AppResponse { app })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("App '{}' not found", name),
            }),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/apps",
    request_body = CreateAppRequest,
    responses(
        (status = 201, description = "App created successfully", body = SuccessResponse),
        (status = 400, description = "Bad request - Invalid app data", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 409, description = "Conflict - App already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn create_app(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateAppRequest>,
) -> Result<(StatusCode, Json<SuccessResponse>), (StatusCode, Json<ErrorResponse>)> {
    let manager = GooseAppsManager::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to initialize apps manager: {}", e),
            }),
        )
    })?;

    if manager.app_exists(&request.app.name) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("App '{}' already exists", request.app.name),
            }),
        ));
    }

    manager.update_app(&request.app).map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("extends GooseWidget") {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: error_msg }),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: error_msg }),
            )
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse {
            message: format!("App '{}' created successfully", request.app.name),
        }),
    ))
}

#[utoipa::path(
    put,
    path = "/apps/{name}",
    request_body = UpdateAppRequest,
    responses(
        (status = 200, description = "App updated successfully", body = SuccessResponse),
        (status = 400, description = "Bad request - Invalid app data", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 404, description = "App not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("name" = String, Path, description = "Name of the app to update")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn update_app(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<UpdateAppRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = GooseAppsManager::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to initialize apps manager: {}", e),
            }),
        )
    })?;

    if !manager.app_exists(&name) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("App '{}' not found", name),
            }),
        ));
    }

    manager.update_app(&request.app).map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("extends GooseWidget") {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: error_msg }),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: error_msg }),
            )
        }
    })?;

    Ok(Json(SuccessResponse {
        message: format!("App '{}' updated successfully", name),
    }))
}

#[utoipa::path(
    delete,
    path = "/apps/{name}",
    responses(
        (status = 200, description = "App deleted successfully", body = SuccessResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 404, description = "App not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("name" = String, Path, description = "Name of the app to delete")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn delete_app(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let manager = GooseAppsManager::new().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to initialize apps manager: {}", e),
            }),
        )
    })?;

    manager.delete_app(&name).map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("not found") {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: error_msg }),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: error_msg }),
            )
        }
    })?;

    Ok(Json(SuccessResponse {
        message: format!("App '{}' deleted successfully", name),
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/apps/list_apps", get(list_apps))
        .route("/apps/{name}", get(get_app))
        .route("/apps", post(create_app))
        .route("/apps/{name}", put(update_app))
        .route("/apps/{name}", delete(delete_app))
        .with_state(state)
}
