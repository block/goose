use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use goose::pipeline::{
    delete_pipeline, list_pipelines, load_pipeline, save_pipeline, Pipeline, PipelineManifest,
};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SavePipelineRequest {
    pub pipeline: Pipeline,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SavePipelineResponse {
    pub id: String,
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidatePipelineResponse {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[utoipa::path(
    get,
    path = "/pipelines/list",
    responses(
        (status = 200, description = "List all saved pipelines", body = Vec<PipelineManifest>),
        (status = 500, description = "Internal error")
    ),
    operation_id = "listPipelines"
)]
async fn list_pipelines_handler() -> Result<Json<Vec<PipelineManifest>>, StatusCode> {
    match list_pipelines() {
        Ok(manifests) => Ok(Json(manifests)),
        Err(e) => {
            tracing::error!("Failed to list pipelines: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/pipelines/{id}",
    params(
        ("id" = String, Path, description = "Pipeline ID (filename without extension)")
    ),
    responses(
        (status = 200, description = "Pipeline loaded", body = Pipeline),
        (status = 404, description = "Pipeline not found")
    ),
    operation_id = "getPipeline"
)]
async fn get_pipeline_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Pipeline>, StatusCode> {
    match load_pipeline(&id) {
        Ok((pipeline, _)) => Ok(Json(pipeline)),
        Err(e) => {
            tracing::error!("Pipeline not found: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

#[utoipa::path(
    post,
    path = "/pipelines/save",
    request_body = SavePipelineRequest,
    responses(
        (status = 200, description = "Pipeline saved", body = SavePipelineResponse),
        (status = 400, description = "Invalid pipeline")
    ),
    operation_id = "savePipeline"
)]
async fn save_pipeline_handler(
    Json(request): Json<SavePipelineRequest>,
) -> Result<Json<SavePipelineResponse>, (StatusCode, String)> {
    if let Err(e) = request.pipeline.validate() {
        return Err((StatusCode::BAD_REQUEST, e.to_string()));
    }

    let file_path = request.id.map(|id| {
        let dir = goose::pipeline::get_pipeline_dir();
        dir.join(format!("{}.yaml", id))
    });

    match save_pipeline(&request.pipeline, file_path) {
        Ok(path) => {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            Ok(Json(SavePipelineResponse {
                id,
                file_path: path.to_string_lossy().to_string(),
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[utoipa::path(
    put,
    path = "/pipelines/{id}",
    request_body = Pipeline,
    responses(
        (status = 200, description = "Pipeline updated", body = SavePipelineResponse),
        (status = 400, description = "Invalid pipeline"),
        (status = 404, description = "Pipeline not found")
    ),
    operation_id = "updatePipeline"
)]
async fn update_pipeline_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(pipeline): Json<Pipeline>,
) -> Result<Json<SavePipelineResponse>, (StatusCode, String)> {
    if let Err(e) = pipeline.validate() {
        return Err((StatusCode::BAD_REQUEST, e.to_string()));
    }

    let dir = goose::pipeline::get_pipeline_dir();
    let path = dir.join(format!("{}.yaml", id));
    if !path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Pipeline '{}' not found", id),
        ));
    }

    match save_pipeline(&pipeline, Some(path)) {
        Ok(path) => Ok(Json(SavePipelineResponse {
            id,
            file_path: path.to_string_lossy().to_string(),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[utoipa::path(
    delete,
    path = "/pipelines/{id}",
    params(
        ("id" = String, Path, description = "Pipeline ID to delete")
    ),
    responses(
        (status = 200, description = "Pipeline deleted"),
        (status = 404, description = "Pipeline not found")
    ),
    operation_id = "deletePipeline"
)]
async fn delete_pipeline_handler(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    match delete_pipeline(&id) {
        Ok(()) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

#[utoipa::path(
    post,
    path = "/pipelines/validate",
    request_body = Pipeline,
    responses(
        (status = 200, description = "Validation result", body = ValidatePipelineResponse)
    ),
    operation_id = "validatePipeline"
)]
async fn validate_pipeline_handler(
    Json(pipeline): Json<Pipeline>,
) -> Json<ValidatePipelineResponse> {
    match pipeline.validate() {
        Ok(warnings) => Json(ValidatePipelineResponse {
            valid: true,
            warnings,
            error: None,
        }),
        Err(e) => Json(ValidatePipelineResponse {
            valid: false,
            warnings: vec![],
            error: Some(e.to_string()),
        }),
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/pipelines/list", get(list_pipelines_handler))
        .route("/pipelines/{id}", get(get_pipeline_handler))
        .route("/pipelines/save", post(save_pipeline_handler))
        .route("/pipelines/{id}", put(update_pipeline_handler))
        .route("/pipelines/{id}", delete(delete_pipeline_handler))
        .route("/pipelines/validate", post(validate_pipeline_handler))
        .with_state(state)
}
