use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::download_manager::{DownloadProgress, DownloadStatus};
use crate::routes::errors::ErrorResponse;
use crate::state::AppState;

#[derive(Deserialize, ToSchema)]
pub struct StartDownloadRequest {
    #[serde(default)]
    pub auth_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ModelListResponse {
    pub models: Vec<ModelInfoResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct ModelInfoResponse {
    pub model_id: String,
    pub normalized_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct ProgressResponse {
    pub downloaded_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f64>,
    pub speed_bytes_per_sec: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u64>,
}

impl From<&DownloadProgress> for ProgressResponse {
    fn from(progress: &DownloadProgress) -> Self {
        Self {
            downloaded_bytes: progress.downloaded_bytes,
            total_bytes: progress.total_bytes,
            percentage: progress.percentage(),
            speed_bytes_per_sec: progress.speed_bytes_per_sec,
            eta_seconds: progress.eta_seconds,
        }
    }
}

impl From<DownloadProgress> for ProgressResponse {
    fn from(progress: DownloadProgress) -> Self {
        Self::from(&progress)
    }
}

fn status_to_string(status: &DownloadStatus) -> String {
    match status {
        DownloadStatus::Pending => "pending",
        DownloadStatus::Downloading => "downloading",
        DownloadStatus::Verifying => "verifying",
        DownloadStatus::Completed => "completed",
        DownloadStatus::Failed => "failed",
        DownloadStatus::Cancelled => "cancelled",
    }
    .to_string()
}

/// List all models (downloaded and actively downloading)
#[utoipa::path(
    get,
    path = "/models",
    responses(
        (status = 200, description = "List of models", body = ModelListResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "models"
)]
pub async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ModelListResponse>, ErrorResponse> {
    let local_models = state.download_manager.list_local_models().await?;
    let active_downloads = state.download_manager.list_active_downloads().await?;

    let mut models = Vec::new();

    // Add completed downloads
    for model in local_models {
        models.push(ModelInfoResponse {
            model_id: model.model_id,
            normalized_id: model.normalized_id,
            status: "completed".to_string(),
            file_size: Some(model.file_size),
            downloaded_at: Some(model.downloaded_at.to_rfc3339()),
            progress: None,
        });
    }

    // Add active downloads
    for download in active_downloads {
        let status = download.status.read().await;
        let progress = download.progress.read().await;
        models.push(ModelInfoResponse {
            model_id: download.model_id.clone(),
            normalized_id: download.normalized_id.clone(),
            status: status_to_string(&status),
            file_size: progress.total_bytes,
            downloaded_at: None,
            progress: Some(ProgressResponse::from(&*progress)),
        });
    }

    Ok(Json(ModelListResponse { models }))
}

/// Get information about a specific model
#[utoipa::path(
    get,
    path = "/models/{model_id}",
    params(
        ("model_id" = String, Path, description = "Model ID in org/model or org/model:variant format (URL-encoded)")
    ),
    responses(
        (status = 200, description = "Model information", body = ModelInfoResponse),
        (status = 400, description = "Invalid model ID", body = ErrorResponse),
        (status = 404, description = "Model not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "models"
)]
pub async fn get_model_info(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Result<Json<ModelInfoResponse>, ErrorResponse> {
    let decoded = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model ID encoding"))?;

    let info = state.download_manager.get_model_info(&decoded).await?;

    Ok(Json(ModelInfoResponse {
        model_id: info.model_id,
        normalized_id: info.normalized_id,
        status: status_to_string(&info.status),
        file_size: info.file_size,
        downloaded_at: info.downloaded_at.map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339()),
        progress: info.progress.map(|p| ProgressResponse::from(&p)),
    }))
}

/// Start downloading a model
#[utoipa::path(
    post,
    path = "/models/{model_id}/download",
    params(
        ("model_id" = String, Path, description = "Model ID in org/model or org/model:variant format (URL-encoded)")
    ),
    request_body = StartDownloadRequest,
    responses(
        (status = 200, description = "Download started", body = ModelInfoResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 409, description = "Already downloading or exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "models"
)]
pub async fn start_download(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
    Json(req): Json<StartDownloadRequest>,
) -> Result<Json<ModelInfoResponse>, ErrorResponse> {
    let decoded = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model ID encoding"))?;

    let task = state
        .download_manager
        .start_download(decoded.to_string(), req.auth_token)
        .await?;

    let status = task.status.read().await;
    let progress = task.progress.read().await;

    Ok(Json(ModelInfoResponse {
        model_id: task.model_id.clone(),
        normalized_id: task.normalized_id.clone(),
        status: status_to_string(&status),
        file_size: progress.total_bytes,
        downloaded_at: None,
        progress: Some(ProgressResponse::from(&*progress)),
    }))
}

/// Get download progress for a model
#[utoipa::path(
    get,
    path = "/models/{model_id}/progress",
    params(
        ("model_id" = String, Path, description = "Model ID (URL-encoded)")
    ),
    responses(
        (status = 200, description = "Download progress", body = ProgressResponse),
        (status = 400, description = "Invalid model ID", body = ErrorResponse),
        (status = 404, description = "Download not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "models"
)]
pub async fn get_progress(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Result<Json<ProgressResponse>, ErrorResponse> {
    let decoded = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model ID encoding"))?;

    let progress = state.download_manager.get_progress(&decoded).await?;

    Ok(Json(ProgressResponse::from(progress)))
}

/// Cancel an active download
#[utoipa::path(
    delete,
    path = "/models/{model_id}/download",
    params(
        ("model_id" = String, Path, description = "Model ID (URL-encoded)")
    ),
    responses(
        (status = 200, description = "Download cancelled"),
        (status = 400, description = "Invalid model ID", body = ErrorResponse),
        (status = 404, description = "Download not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "models"
)]
pub async fn cancel_download(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    let decoded = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model ID encoding"))?;

    state.download_manager.cancel_download(&decoded).await?;

    Ok(StatusCode::OK)
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/models", get(list_models))
        .route("/models/:model_id", get(get_model_info))
        .route("/models/:model_id/download", post(start_download))
        .route("/models/:model_id/progress", get(get_progress))
        .route("/models/:model_id/download", delete(cancel_download))
        .with_state(state)
}
