use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use goose::config::paths::Paths;
use goose::dictation::download_manager::{get_download_manager, DownloadProgress};
use goose::providers::local_inference::hf_models::{self, HfModelInfo, HfQuantVariant};
use goose::providers::local_inference::local_model_registry::{
    display_name_from_repo, get_registry, model_id_from_repo, LocalModelEntry, ModelSettings,
};
use goose::providers::local_inference::{
    available_inference_memory_bytes, available_local_models, get_local_model,
    recommend_local_model, LocalLlmModel, LOCAL_LLM_MODEL_CONFIG_KEY,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct LocalModelResponse {
    #[serde(flatten)]
    #[schema(inline)]
    pub model: &'static LocalLlmModel,
    pub downloaded: bool,
    pub recommended: bool,
    pub featured: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegistryModelResponse {
    pub id: String,
    pub display_name: String,
    pub repo_id: String,
    pub filename: String,
    pub quantization: String,
    pub size_bytes: u64,
    pub downloaded: bool,
    pub featured: bool,
    pub settings: ModelSettings,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum ModelListItem {
    Featured(LocalModelResponse),
    Registry(Box<RegistryModelResponse>),
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
        (status = 200, description = "List of available local LLM models", body = Vec<ModelListItem>)
    )
)]
pub async fn list_local_models(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<Json<Vec<ModelListItem>>, ErrorResponse> {
    let recommended_id = recommend_local_model(&state.inference_runtime);
    let featured_ids: Vec<&str> = available_local_models().iter().map(|m| m.id).collect();

    let mut items: Vec<ModelListItem> = available_local_models()
        .iter()
        .map(|m| {
            ModelListItem::Featured(LocalModelResponse {
                model: m,
                downloaded: m.is_downloaded(),
                recommended: m.id == recommended_id,
                featured: true,
            })
        })
        .collect();

    // Add registry models that aren't in the featured list
    if let Ok(registry) = get_registry().lock() {
        for entry in registry.list_models() {
            if !featured_ids.contains(&entry.id.as_str()) {
                items.push(ModelListItem::Registry(Box::new(RegistryModelResponse {
                    id: entry.id.clone(),
                    display_name: entry.display_name.clone(),
                    repo_id: entry.repo_id.clone(),
                    filename: entry.filename.clone(),
                    quantization: entry.quantization.clone(),
                    size_bytes: entry.file_size(),
                    downloaded: entry.is_downloaded(),
                    featured: false,
                    settings: entry.settings.clone(),
                })));
            }
        }
    }

    Ok(Json(items))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

#[utoipa::path(
    get,
    path = "/local-inference/search",
    params(
        ("q" = String, Query, description = "Search query"),
        ("limit" = Option<usize>, Query, description = "Max results")
    ),
    responses(
        (status = 200, description = "Search results", body = Vec<HfModelInfo>),
        (status = 500, description = "Search failed")
    )
)]
pub async fn search_hf_models(
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<HfModelInfo>>, ErrorResponse> {
    let limit = params.limit.unwrap_or(20).min(50);
    let results = hf_models::search_gguf_models(&params.q, limit)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Search failed: {}", e)))?;
    Ok(Json(results))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RepoVariantsResponse {
    pub variants: Vec<HfQuantVariant>,
    pub recommended_index: Option<usize>,
}

#[utoipa::path(
    get,
    path = "/local-inference/repo/{author}/{repo}/files",
    responses(
        (status = 200, description = "GGUF quantization variants in repo", body = RepoVariantsResponse),
        (status = 500, description = "Failed to fetch repo files")
    )
)]
pub async fn get_repo_files(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Path((author, repo)): Path<(String, String)>,
) -> Result<Json<RepoVariantsResponse>, ErrorResponse> {
    let repo_id = format!("{}/{}", author, repo);
    let variants = hf_models::get_repo_gguf_variants(&repo_id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to fetch repo files: {}", e)))?;

    let available_memory = available_inference_memory_bytes(&state.inference_runtime);
    let recommended_index = hf_models::recommend_variant(&variants, available_memory);

    Ok(Json(RepoVariantsResponse {
        variants,
        recommended_index,
    }))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct HfDownloadRequest {
    pub repo_id: Option<String>,
    pub filename: Option<String>,
    pub spec: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HfDownloadResponse {
    pub model_id: String,
}

#[utoipa::path(
    post,
    path = "/local-inference/download",
    request_body = HfDownloadRequest,
    responses(
        (status = 202, description = "Download started", body = HfDownloadResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn download_hf_model(
    Json(req): Json<HfDownloadRequest>,
) -> Result<(StatusCode, Json<HfDownloadResponse>), ErrorResponse> {
    let (repo_id, filename, quantization, download_url) = if let Some(spec) = &req.spec {
        let (repo_id, file) = hf_models::resolve_model_spec(spec)
            .await
            .map_err(|e| ErrorResponse::bad_request(format!("{}", e)))?;
        (repo_id, file.filename, file.quantization, file.download_url)
    } else if let (Some(repo_id), Some(filename)) = (&req.repo_id, &req.filename) {
        let quantization = hf_models::parse_quantization_from_filename(filename);
        let download_url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo_id, filename
        );
        (
            repo_id.clone(),
            filename.clone(),
            quantization,
            download_url,
        )
    } else {
        return Err(ErrorResponse::bad_request(
            "Provide either 'spec' or both 'repo_id' and 'filename'",
        ));
    };

    let model_id = model_id_from_repo(&repo_id, &quantization);
    let display_name = display_name_from_repo(&repo_id, &quantization);
    let local_path = Paths::in_data_dir("models").join(&filename);

    let entry = LocalModelEntry {
        id: model_id.clone(),
        display_name,
        repo_id,
        filename: filename.clone(),
        quantization,
        local_path: local_path.clone(),
        source_url: download_url.clone(),
        settings: ModelSettings::default(),
    };

    {
        let mut registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
        registry
            .add_model(entry)
            .map_err(|e| ErrorResponse::internal(format!("{}", e)))?;
    }

    let manager = get_download_manager();
    manager
        .download_model(
            format!("{}-model", model_id),
            download_url,
            local_path,
            Some(LOCAL_LLM_MODEL_CONFIG_KEY.to_string()),
            Some(model_id.clone()),
        )
        .await
        .map_err(convert_error)?;

    Ok((StatusCode::ACCEPTED, Json(HfDownloadResponse { model_id })))
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

    manager
        .download_model(
            format!("{}-model", model.id),
            model.url.to_string(),
            model.local_path(),
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
    let download_id = format!("{}-model", model_id);
    debug!(model_id = %model_id, download_id = %download_id, "Getting download progress");

    let manager = get_download_manager();

    let model_progress = manager
        .get_progress(&download_id)
        .ok_or_else(|| ErrorResponse::not_found("Download not found"))?;

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
    // Try featured model first
    if let Some(model) = get_local_model(&model_id) {
        let model_path = model.local_path();
        if !model_path.exists() {
            return Err(ErrorResponse::not_found("Model not downloaded"));
        }
        tokio::fs::remove_file(&model_path)
            .await
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete model: {}", e)))?;
        return Ok(StatusCode::OK);
    }

    // Try registry model
    let model_path = {
        let registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
        let entry = registry
            .get_model(&model_id)
            .ok_or_else(|| ErrorResponse::not_found("Model not found"))?;
        entry.local_path.clone()
    };

    if model_path.exists() {
        tokio::fs::remove_file(&model_path)
            .await
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete model: {}", e)))?;
    }

    // Remove from registry
    let mut registry = get_registry()
        .lock()
        .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
    registry
        .remove_model(&model_id)
        .map_err(|e| ErrorResponse::internal(format!("{}", e)))?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/local-inference/models/{model_id}/settings",
    responses(
        (status = 200, description = "Model settings", body = ModelSettings),
        (status = 404, description = "Model not found")
    )
)]
pub async fn get_model_settings(
    Path(model_id): Path<String>,
) -> Result<Json<ModelSettings>, ErrorResponse> {
    let registry = get_registry()
        .lock()
        .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

    if let Some(settings) = registry.get_model_settings(&model_id) {
        return Ok(Json(settings.clone()));
    }

    Err(ErrorResponse::not_found("Model not found"))
}

#[utoipa::path(
    put,
    path = "/local-inference/models/{model_id}/settings",
    request_body = ModelSettings,
    responses(
        (status = 200, description = "Settings updated", body = ModelSettings),
        (status = 404, description = "Model not found"),
        (status = 500, description = "Failed to save settings")
    )
)]
pub async fn update_model_settings(
    Path(model_id): Path<String>,
    Json(settings): Json<ModelSettings>,
) -> Result<Json<ModelSettings>, ErrorResponse> {
    let mut registry = get_registry()
        .lock()
        .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

    registry
        .update_model_settings(&model_id, settings.clone())
        .map_err(|e| ErrorResponse::not_found(format!("{}", e)))?;

    Ok(Json(settings))
}

pub fn routes(state: Arc<AppState>) -> Router {
    goose::dictation::download_manager::cleanup_partial_downloads(&Paths::in_data_dir("models"));

    Router::new()
        .route("/local-inference/models", get(list_local_models))
        .route("/local-inference/search", get(search_hf_models))
        .route(
            "/local-inference/repo/{author}/{repo}/files",
            get(get_repo_files),
        )
        .route("/local-inference/download", post(download_hf_model))
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
        .route(
            "/local-inference/models/{model_id}/settings",
            get(get_model_settings),
        )
        .route(
            "/local-inference/models/{model_id}/settings",
            axum::routing::put(update_model_settings),
        )
        .with_state(state)
}
