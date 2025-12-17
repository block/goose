use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use goose::model_training::{InferenceServerStatus, INFERENCE_MANAGER};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct StartInferenceRequest {
    pub job_id: String,
    pub base_model: String,
    pub adapter_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartInferenceResponse {
    pub status: InferenceServerStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Start an inference server for a fine-tuned model
async fn start_inference_server(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<StartInferenceRequest>,
) -> Response {
    info!(
        "Starting inference server for job {} with base model {}",
        req.job_id, req.base_model
    );

    let adapter_path = PathBuf::from(&req.adapter_path);

    match INFERENCE_MANAGER.start_server(req.job_id, req.base_model, adapter_path) {
        Ok(status) => {
            info!("Inference server started successfully: {:?}", status);
            (StatusCode::OK, Json(StartInferenceResponse { status })).into_response()
        }
        Err(e) => {
            error!("Failed to start inference server: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to start inference server: {}", e),
                }),
            )
                .into_response()
        }
    }
}

/// Stop an inference server
async fn stop_inference_server(
    State(_state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Response {
    info!("Stopping inference server for job {}", job_id);

    match INFERENCE_MANAGER.stop_server(&job_id) {
        Ok(()) => {
            info!("Inference server stopped successfully");
            (StatusCode::OK, Json(serde_json::json!({"status": "stopped"}))).into_response()
        }
        Err(e) => {
            error!("Failed to stop inference server: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to stop inference server: {}", e),
                }),
            )
                .into_response()
        }
    }
}

/// Get status of an inference server
async fn get_inference_status(
    State(_state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Response {
    info!("Getting inference server status for job {}", job_id);

    match INFERENCE_MANAGER.get_status(&job_id) {
        Some(status) => (StatusCode::OK, Json(status)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("No inference server found for job {}", job_id),
            }),
        )
            .into_response(),
    }
}

/// List all running inference servers
async fn list_inference_servers(State(_state): State<Arc<AppState>>) -> Response {
    info!("Listing all inference servers");

    let servers = INFERENCE_MANAGER.list_servers();
    (StatusCode::OK, Json(servers)).into_response()
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/start", post(start_inference_server))
        .route("/stop/{job_id}", post(stop_inference_server))
        .route("/status/{job_id}", get(get_inference_status))
        .route("/list", get(list_inference_servers))
}
