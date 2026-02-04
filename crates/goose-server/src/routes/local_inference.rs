use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use goose::dictation::download_manager::{get_download_manager, DownloadProgress};
use goose::providers::local_inference::{
    available_local_models, get_local_model, recommend_local_model, LocalLlmModel,
    LOCAL_LLM_MODEL_CONFIG_KEY,
};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct LocalModelResponse {
    #[serde(flatten)]
    #[schema(inline)]
    pub model: &'static LocalLlmModel,
    pub downloaded: bool,
    pub recommended: bool,
}

fn convert_error(e: anyhow::Error) -> ErrorResponse {
    let error_msg = e.to_string();

    if error_msg.contains("not found") {
        ErrorResponse::not_found(error_msg)
    } else if error_msg.contains("not configured") {
        ErrorResponse {
            message: error_msg,
            status: StatusCode::PRECONDITION_FAILED,
        }
    } else if error_msg.contains("already in progress") {
        ErrorResponse::bad_request(error_msg)
    } else {
        ErrorResponse::internal(error_msg)
    }
}

#[utoipa::path(
    get,
    path = "/local-inference/models",
    responses(
        (status = 200, description = "List of available local LLM models", body = Vec<LocalModelResponse>)
    )
)]
pub async fn list_local_models() -> Result<Json<Vec<LocalModelResponse>>, ErrorResponse> {
    let recommended_id = recommend_local_model();
    let models = available_local_models()
        .iter()
        .map(|m| LocalModelResponse {
            model: m,
            downloaded: m.is_downloaded(),
            recommended: m.id == recommended_id,
        })
        .collect();

    Ok(Json(models))
}

#[utoipa::path(
    post,
    path = "/local-inference/models/{model_id}/download",
    responses(
        (status = 202, description = "Download started"),
        (status = 400, description = "Model not found or download already in progress"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn download_local_model(
    Path(model_id): Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    let model =
        get_local_model(&model_id).ok_or_else(|| ErrorResponse::bad_request("Model not found"))?;

    let manager = get_download_manager();

    // Download model file (don't set config yet - wait for tokenizer)
    manager
        .download_model(
            format!("{}-model", model.id),
            model.url.to_string(),
            model.local_path(),
            None,
            None,
        )
        .await
        .map_err(convert_error)?;

    // Download tokenizer file (set config and provider when this completes)
    // We'll set GOOSE_PROVIDER to "local" after the tokenizer download completes
    // This is handled in the download_manager callback
    manager
        .download_model(
            format!("{}-tokenizer", model.id),
            model.tokenizer_url.to_string(),
            model.tokenizer_path(),
            Some(LOCAL_LLM_MODEL_CONFIG_KEY.to_string()),
            Some(model.id.to_string()),
        )
        .await
        .map_err(convert_error)?;

    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    get,
    path = "/local-inference/models/{model_id}/download",
    responses(
        (status = 200, description = "Download progress", body = DownloadProgress),
        (status = 404, description = "Download not found")
    )
)]
pub async fn get_local_model_download_progress(
    Path(model_id): Path<String>,
) -> Result<Json<DownloadProgress>, ErrorResponse> {
    let manager = get_download_manager();

    // Check both model and tokenizer progress
    let model_progress = manager
        .get_progress(&format!("{}-model", model_id))
        .ok_or_else(|| ErrorResponse::not_found("Download not found"))?;

    let tokenizer_progress = manager.get_progress(&format!("{}-tokenizer", model_id));

    // If tokenizer failed, return that error
    if let Some(tok_prog) = tokenizer_progress {
        if tok_prog.status == goose::dictation::download_manager::DownloadStatus::Failed {
            return Ok(Json(tok_prog));
        }
    }

    // If model failed, return that error
    if model_progress.status == goose::dictation::download_manager::DownloadStatus::Failed {
        return Ok(Json(model_progress));
    }

    // Otherwise return model progress (which shows overall download progress)
    Ok(Json(model_progress))
}

#[utoipa::path(
    delete,
    path = "/local-inference/models/{model_id}/download",
    responses(
        (status = 200, description = "Download cancelled"),
        (status = 404, description = "Download not found")
    )
)]
pub async fn cancel_local_model_download(
    Path(model_id): Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    let manager = get_download_manager();
    manager
        .cancel_download(&format!("{}-model", model_id))
        .map_err(convert_error)?;
    manager
        .cancel_download(&format!("{}-tokenizer", model_id))
        .map_err(convert_error)?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/local-inference/models/{model_id}",
    responses(
        (status = 200, description = "Model deleted"),
        (status = 404, description = "Model not found or not downloaded"),
        (status = 500, description = "Failed to delete model")
    )
)]
pub async fn delete_local_model(Path(model_id): Path<String>) -> Result<StatusCode, ErrorResponse> {
    let model = get_local_model(&model_id)
        .ok_or_else(|| ErrorResponse::not_found("Model not found"))?;

    let model_path = model.local_path();
    let tokenizer_path = model.tokenizer_path();

    if !model_path.exists() && !tokenizer_path.exists() {
        return Err(ErrorResponse::not_found("Model not downloaded"));
    }

    // Delete both files
    if model_path.exists() {
        tokio::fs::remove_file(&model_path)
            .await
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete model: {}", e)))?;
    }
    if tokenizer_path.exists() {
        tokio::fs::remove_file(&tokenizer_path)
            .await
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete tokenizer: {}", e)))?;
    }

    Ok(StatusCode::OK)
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/local-inference/models", get(list_local_models))
        .route(
            "/local-inference/models/{model_id}/download",
            post(download_local_model),
        )
        .route(
            "/local-inference/models/{model_id}/download",
            get(get_local_model_download_progress),
        )
        .route(
            "/local-inference/models/{model_id}/download",
            delete(cancel_local_model_download),
        )
        .route(
            "/local-inference/models/{model_id}",
            delete(delete_local_model),
        )
        .with_state(state)
}
