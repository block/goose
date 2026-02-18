use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use goose::config::paths::Paths;
use goose::providers::local_inference::hf_models::{self, HfModelInfo};
use goose::providers::local_inference::local_model_registry::{
    display_name_from_repo, get_featured_by_id, get_registry, is_featured_model,
    model_id_from_repo, LocalModelEntry, ModelSettings, ModelTier, FeaturedModel,
    FEATURED_MODELS,
};
use goose::providers::local_inference::{available_inference_memory_bytes, InferenceRuntime};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Download status for a local model (mirrors goose::providers::local_inference::local_model_registry::ModelDownloadStatus)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "state")]
pub enum ModelDownloadStatus {
    NotDownloaded,
    Downloading {
        progress_percent: f32,
        bytes_downloaded: u64,
        total_bytes: u64,
        speed_bps: Option<u64>,
    },
    Downloaded,
}

/// Response for a single local model
#[derive(Debug, Serialize, ToSchema)]
pub struct LocalModelResponse {
    pub id: String,
    pub display_name: String,
    pub repo_id: String,
    pub filename: String,
    pub quantization: String,
    pub size_bytes: u64,
    #[schema(inline)]
    pub status: ModelDownloadStatus,
    pub recommended: bool,
    #[schema(nullable)]
    pub tier: Option<ModelTier>,
    pub context_limit: Option<u32>,
    pub settings: ModelSettings,
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

/// Ensure all recommended models are in the registry (with metadata from HuggingFace if needed).
/// This is called on list_local_models to populate the registry with recommended models.
async fn ensure_recommended_models_in_registry() -> Result<(), ErrorResponse> {
    // First pass: collect specs that need to be fetched (without holding lock across await)
    let specs_to_fetch: Vec<&'static FeaturedModel> = {
        let registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

        FEATURED_MODELS
            .iter()
            .filter(|rec| {
                let parts: Vec<&str> = rec.spec.rsplitn(2, ':').collect();
                if parts.len() != 2 {
                    return false;
                }
                let model_id = model_id_from_repo(parts[1], parts[0]);
                !registry.has_model(&model_id)
            })
            .collect()
    }; // Lock released here

    // Fetch info and add entries one by one (lock is not held across await)
    for rec in specs_to_fetch {
        let parts: Vec<&str> = rec.spec.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let (repo_id, quantization) = (parts[1], parts[0]);
        let model_id = model_id_from_repo(repo_id, quantization);

        // Try to get file info from HuggingFace (for size_bytes and download URL)
        let (filename, download_url, size_bytes) =
            match hf_models::resolve_model_spec(rec.spec).await {
                Ok((_, file_info)) => (
                    file_info.filename,
                    file_info.download_url,
                    file_info.size_bytes,
                ),
                Err(_) => {
                    // If HF lookup fails, create a placeholder with estimated filename
                    let estimated_filename = format!(
                        "{}-{}.gguf",
                        repo_id.split('/').next_back().unwrap_or("model"),
                        quantization
                    );
                    let download_url = format!(
                        "https://huggingface.co/{}/resolve/main/{}",
                        repo_id, estimated_filename
                    );
                    // Use the size from FEATURED_MODELS as fallback
                    (estimated_filename, download_url, rec.size_bytes)
                }
            };

        let local_path = Paths::in_data_dir("models").join(&filename);

        let entry = LocalModelEntry {
            id: model_id,
            display_name: rec.display_name.to_string(),
            repo_id: repo_id.to_string(),
            filename,
            quantization: quantization.to_string(),
            local_path,
            source_url: download_url,
            settings: ModelSettings::default(),
            size_bytes,
        };

        // Re-acquire lock to add entry (ignore errors - might already exist due to race)
        if let Ok(mut registry) = get_registry().lock() {
            let _ = registry.add_model(entry);
        }
    }

    Ok(())
}

/// Recommend a model based on available memory
fn recommend_model_id(runtime: &InferenceRuntime) -> Option<String> {
    let available_memory = available_inference_memory_bytes(runtime);

    // Simple heuristic: pick the largest model that fits in ~80% of available memory
    // Rough model sizes: Tiny ~1GB, Small ~2GB, Medium ~5GB, Large ~15GB
    let target_memory = (available_memory as f64 * 0.8) as u64;

    let model_sizes = [
        (ModelTier::Large, 15_000_000_000u64),
        (ModelTier::Medium, 5_000_000_000u64),
        (ModelTier::Small, 2_000_000_000u64),
        (ModelTier::Tiny, 1_000_000_000u64),
    ];

    for (tier, size) in model_sizes {
        if target_memory >= size {
            // Find a recommended model with this tier
            for rec in FEATURED_MODELS {
                if rec.tier == tier {
                    let parts: Vec<&str> = rec.spec.rsplitn(2, ':').collect();
                    if parts.len() == 2 {
                        return Some(model_id_from_repo(parts[1], parts[0]));
                    }
                }
            }
        }
    }

    // Default to smallest
    FEATURED_MODELS.first().map(|rec| {
        let parts: Vec<&str> = rec.spec.rsplitn(2, ':').collect();
        if parts.len() == 2 {
            model_id_from_repo(parts[1], parts[0])
        } else {
            String::new()
        }
    })
}

#[utoipa::path(
    get,
    path = "/local-inference/models",
    responses(
        (status = 200, description = "List of available local LLM models", body = Vec<LocalModelResponse>)
    )
)]
pub async fn list_local_models() -> Result<Json<Vec<LocalModelResponse>>, ErrorResponse> {
    // Ensure recommended models are in registry
    ensure_recommended_models_in_registry().await?;

    let runtime = InferenceRuntime::get_or_init();
    let recommended_id = recommend_model_id(&runtime);

    let registry = get_registry()
        .lock()
        .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

    let mut models: Vec<LocalModelResponse> = Vec::new();

    for entry in registry.list_models() {
        let is_recommended = is_featured_model(&entry.id);
        let goose_status = entry.download_status();

        // Convert from goose's ModelDownloadStatus to our local one
        use goose::providers::local_inference::local_model_registry::ModelDownloadStatus as GooseStatus;
        let is_downloaded = matches!(goose_status, GooseStatus::Downloaded);

        // Filter: keep if downloaded OR recommended
        if !is_downloaded && !is_recommended {
            continue;
        }

        let status = match goose_status {
            GooseStatus::NotDownloaded => ModelDownloadStatus::NotDownloaded,
            GooseStatus::Downloading {
                progress_percent,
                bytes_downloaded,
                total_bytes,
                speed_bps,
            } => ModelDownloadStatus::Downloading {
                progress_percent,
                bytes_downloaded,
                total_bytes,
                speed_bps,
            },
            GooseStatus::Downloaded => ModelDownloadStatus::Downloaded,
        };

        // Get tier and context_limit from recommended info if available
        let rec_info = get_featured_by_id(&entry.id);
        let tier = rec_info.map(|r| r.tier);
        let context_limit = rec_info.map(|r| r.context_limit);

        models.push(LocalModelResponse {
            id: entry.id.clone(),
            display_name: entry.display_name.clone(),
            repo_id: entry.repo_id.clone(),
            filename: entry.filename.clone(),
            quantization: entry.quantization.clone(),
            size_bytes: if entry.size_bytes > 0 {
                entry.size_bytes
            } else if entry.local_path.exists() {
                entry.file_size()
            } else if let Some(rec) = rec_info {
                rec.size_bytes
            } else {
                0
            },
            status,
            recommended: recommended_id.as_deref() == Some(&entry.id),
            tier,
            context_limit,
            settings: entry.settings.clone(),
        });
    }

    // Sort: recommended models first (by tier), then others by name
    models.sort_by(|a, b| match (&a.tier, &b.tier) {
        (Some(ta), Some(tb)) => ta.cmp(tb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.display_name.cmp(&b.display_name),
    });

    Ok(Json(models))
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
        ("limit" = Option<usize>, Query, description = "Max results (default 20)")
    ),
    responses(
        (status = 200, description = "Search results", body = Vec<HfModelInfo>)
    )
)]
pub async fn search_hf_models(
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<HfModelInfo>>, ErrorResponse> {
    let limit = params.limit.unwrap_or(20);
    let results = hf_models::search_gguf_models(&params.q, limit)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Search failed: {}", e)))?;
    Ok(Json(results))
}

#[derive(Debug, Serialize, ToSchema)]
#[aliases(RepoVariantsResponse = RepoVariantsResponse)]
pub struct RepoVariantsResponse {
    #[schema(value_type = Vec<HfQuantVariant>)]
    pub variants: Vec<hf_models::HfQuantVariant>,
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
    Path((author, repo)): Path<(String, String)>,
) -> Result<Json<RepoVariantsResponse>, ErrorResponse> {
    let repo_id = format!("{}/{}", author, repo);
    let variants = hf_models::get_repo_gguf_variants(&repo_id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to fetch repo files: {}", e)))?;

    let runtime = InferenceRuntime::get_or_init();
    let available_memory = available_inference_memory_bytes(&runtime);
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
    let (repo_id, filename, quantization, download_url, size_bytes) = if let Some(spec) = &req.spec
    {
        let (repo_id, file) = hf_models::resolve_model_spec(spec)
            .await
            .map_err(|e| ErrorResponse::bad_request(format!("{}", e)))?;
        (
            repo_id,
            file.filename,
            file.quantization,
            file.download_url,
            file.size_bytes,
        )
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
            0,
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
        size_bytes,
    };

    {
        let mut registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
        registry
            .add_model(entry.clone())
            .map_err(|e| ErrorResponse::internal(format!("{}", e)))?;
    }

    // Use the dictation download manager for now (it handles direct URL downloads)
    // The model_id format for the download manager is "{model_id}-model"
    let dm = goose::dictation::download_manager::get_download_manager();
    dm.download_model(
        format!("{}-model", model_id),
        download_url,
        local_path,
        None,
        None,
    )
    .await
    .map_err(convert_error)?;

    Ok((StatusCode::ACCEPTED, Json(HfDownloadResponse { model_id })))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DownloadProgressResponse {
    pub model_id: String,
    pub status: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub speed_bps: Option<u64>,
    pub eta_seconds: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/local-inference/models/{model_id}/download",
    responses(
        (status = 200, description = "Download progress", body = DownloadProgressResponse),
        (status = 404, description = "Download not found")
    )
)]
pub async fn get_local_model_download_progress(
    Path(model_id): Path<String>,
) -> Result<Json<DownloadProgressResponse>, ErrorResponse> {
    let model_id = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model_id encoding"))?
        .into_owned();

    let dm = goose::dictation::download_manager::get_download_manager();
    let download_id = format!("{}-model", model_id);
    tracing::debug!("Getting download progress for model_id: {}, download_id: {}", model_id, download_id);

    let progress = dm
        .get_progress(&download_id)
        .ok_or_else(|| ErrorResponse::not_found("Download not found"))?;

    // Status enum has serde(rename_all = "lowercase"), but format!("{:?}") uses Debug
    // which gives "Downloading" instead of "downloading". Use lowercase for consistency.
    Ok(Json(DownloadProgressResponse {
        model_id,
        status: format!("{:?}", progress.status).to_lowercase(),
        bytes_downloaded: progress.bytes_downloaded,
        total_bytes: progress.total_bytes,
        speed_bps: progress.speed_bps,
        eta_seconds: progress.eta_seconds,
    }))
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
    let model_id = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model_id encoding"))?
        .into_owned();

    let dm = goose::dictation::download_manager::get_download_manager();
    let download_id = format!("{}-model", model_id);

    dm.cancel_download(&download_id)
        .map_err(convert_error)?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/local-inference/models/{model_id}",
    responses(
        (status = 200, description = "Model deleted"),
        (status = 404, description = "Model not found")
    )
)]
pub async fn delete_local_model(Path(model_id): Path<String>) -> Result<StatusCode, ErrorResponse> {
    let model_id = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model_id encoding"))?
        .into_owned();

    // Get model path from registry
    let model_path = {
        let registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

        registry
            .get_model(&model_id)
            .map(|m| m.local_path.clone())
            .ok_or_else(|| ErrorResponse::not_found("Model not found in registry"))?
    };

    // Delete the file if it exists
    if model_path.exists() {
        tokio::fs::remove_file(&model_path)
            .await
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete model: {}", e)))?;
    }

    // Remove from registry (unless it's a recommended model - just mark as not downloaded)
    if !is_featured_model(&model_id) {
        let mut registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
        registry
            .remove_model(&model_id)
            .map_err(|e| ErrorResponse::internal(format!("{}", e)))?;
    }

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
    let model_id = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model_id encoding"))?
        .into_owned();

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
    let model_id = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model_id encoding"))?
        .into_owned();

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
