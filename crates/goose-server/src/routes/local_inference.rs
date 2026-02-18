use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use goose::config::paths::Paths;
use goose::providers::local_inference::{
    hf_models::{get_repo_gguf_files, resolve_model_spec, search_gguf_models, HfGgufFile},
    local_model_registry::{
        display_name_from_repo, get_registry, is_featured_model, model_id_from_repo,
        parse_model_spec, LocalModelEntry, ModelDownloadStatus as RegistryDownloadStatus,
        ModelSettings, FEATURED_MODELS,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sysinfo::System;
use tracing::debug;
use utoipa::ToSchema;

/// Download status for local models
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

/// Response for a local model
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalModelResponse {
    pub id: String,
    pub display_name: String,
    pub repo_id: String,
    pub filename: String,
    pub quantization: String,
    pub size_bytes: u64,
    pub status: ModelDownloadStatus,
    pub recommended: bool,
    pub settings: ModelSettings,
}

fn handle_internal_error<T: std::fmt::Display>(msg: T) -> ErrorResponse {
    ErrorResponse::internal(format!("{}", msg))
}

/// Recommend the best model based on available system memory
fn recommend_model_id() -> Option<String> {
    let mut sys = System::new_all();
    sys.refresh_memory();
    let available_memory = sys.total_memory();

    // Rough model sizes based on GGUF Q4_K_M quantization
    // Return the largest model that fits in ~80% of available memory
    let target_memory = (available_memory as f64 * 0.8) as u64;

    // Model sizes in bytes (approximate)
    let model_sizes: &[(&str, u64)] = &[
        ("bartowski/Mistral-Small-24B-Instruct-2501-GGUF:Q4_K_M", 15_000_000_000),
        ("bartowski/Hermes-2-Pro-Mistral-7B-GGUF:Q4_K_M", 5_000_000_000),
        ("bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M", 2_000_000_000),
        ("bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M", 1_000_000_000),
    ];

    for (spec, size) in model_sizes {
        if target_memory >= *size {
            if let Some((repo_id, quant)) = parse_model_spec(spec) {
                return Some(model_id_from_repo(repo_id, quant));
            }
        }
    }

    // Default to smallest
    parse_model_spec(FEATURED_MODELS[0])
        .map(|(repo_id, quant)| model_id_from_repo(repo_id, quant))
}

/// Ensure featured models are in the registry.
/// Fetches metadata from HuggingFace for any missing featured models.
async fn ensure_featured_models_in_registry() -> Result<(), ErrorResponse> {
    let mut entries_to_add = Vec::new();

    for spec in FEATURED_MODELS {
        let (repo_id, quantization) = match parse_model_spec(spec) {
            Some(parts) => parts,
            None => continue,
        };

        let model_id = model_id_from_repo(repo_id, quantization);

        // Check if already in registry
        {
            let registry = get_registry()
                .lock()
                .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
            if registry.has_model(&model_id) {
                continue;
            }
        }

        // Fetch metadata from HuggingFace
        let hf_file = match resolve_model_spec(spec).await {
            Ok((_repo, file)) => file,
            Err(_) => {
                // Create a basic entry without HF metadata
                let filename = format!(
                    "{}-{}.gguf",
                    repo_id.split('/').last().unwrap_or("model"),
                    quantization
                );
                HfGgufFile {
                    filename: filename.clone(),
                    size_bytes: 0,
                    quantization: quantization.to_string(),
                    download_url: format!(
                        "https://huggingface.co/{}/resolve/main/{}",
                        repo_id, filename
                    ),
                }
            }
        };

        let local_path = Paths::in_data_dir("models").join(&hf_file.filename);

        entries_to_add.push(LocalModelEntry {
            id: model_id,
            display_name: display_name_from_repo(repo_id, quantization),
            repo_id: repo_id.to_string(),
            filename: hf_file.filename,
            quantization: quantization.to_string(),
            local_path,
            source_url: hf_file.download_url,
            settings: ModelSettings::default(),
            size_bytes: hf_file.size_bytes,
        });
    }

    // Add entries and sync registry
    if !entries_to_add.is_empty() {
        let mut registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
        registry.sync_with_featured(entries_to_add);
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/local-inference/models",
    responses(
        (status = 200, description = "List of available local LLM models", body = Vec<LocalModelResponse>)
    )
)]
pub async fn list_local_models() -> Result<Json<Vec<LocalModelResponse>>, ErrorResponse> {
    // Ensure featured models are in registry
    ensure_featured_models_in_registry().await?;

    let recommended_id = recommend_model_id();

    let registry = get_registry()
        .lock()
        .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

    let mut models: Vec<LocalModelResponse> = Vec::new();

    for entry in registry.list_models() {
        let goose_status: RegistryDownloadStatus = entry.download_status();

        let status = match goose_status {
            RegistryDownloadStatus::NotDownloaded => ModelDownloadStatus::NotDownloaded,
            RegistryDownloadStatus::Downloading {
                progress_percent,
                bytes_downloaded,
                total_bytes,
                speed_bps,
            } => ModelDownloadStatus::Downloading {
                progress_percent,
                bytes_downloaded,
                total_bytes,
                speed_bps: Some(speed_bps),
            },
            RegistryDownloadStatus::Downloaded => ModelDownloadStatus::Downloaded,
        };

        // Get actual file size if downloaded and entry size is 0
        let size_bytes = if entry.size_bytes > 0 {
            entry.size_bytes
        } else if entry.local_path.exists() {
            std::fs::metadata(&entry.local_path)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };

        models.push(LocalModelResponse {
            id: entry.id.clone(),
            display_name: entry.display_name.clone(),
            repo_id: entry.repo_id.clone(),
            filename: entry.filename.clone(),
            quantization: entry.quantization.clone(),
            size_bytes,
            status,
            recommended: recommended_id.as_deref() == Some(&entry.id),
            settings: entry.settings.clone(),
        });
    }

    // Sort: downloaded first, then by display_name
    models.sort_by(|a, b| {
        let a_downloaded = matches!(a.status, ModelDownloadStatus::Downloaded);
        let b_downloaded = matches!(b.status, ModelDownloadStatus::Downloaded);
        match (b_downloaded, a_downloaded) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.display_name.cmp(&b.display_name),
        }
    });

    Ok(Json(models))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HfSearchResult {
    pub id: String,
    pub author: String,
    pub name: String,
    pub downloads: u64,
    pub likes: u64,
}

#[utoipa::path(
    get,
    path = "/local-inference/search",
    params(
        ("q" = String, Query, description = "Search query")
    ),
    responses(
        (status = 200, description = "Search results", body = Vec<HfSearchResult>)
    )
)]
pub async fn search_hf_models(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<HfSearchResult>>, ErrorResponse> {
    let query = params.get("q").cloned().unwrap_or_default();

    let results = search_gguf_models(&query, 20)
        .await
        .map_err(|e| ErrorResponse::internal(format!("HF search failed: {}", e)))?;

    let search_results: Vec<HfSearchResult> = results
        .into_iter()
        .map(|m| HfSearchResult {
            id: m.repo_id.clone(),
            author: m.author,
            name: m.model_name,
            downloads: m.downloads,
            likes: 0, // HfModelInfo doesn't have likes
        })
        .collect();

    Ok(Json(search_results))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HfQuantVariant {
    pub filename: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub download_url: String,
    pub quality_rank: u32,
}

#[utoipa::path(
    get,
    path = "/local-inference/repo/{author}/{repo}/files",
    responses(
        (status = 200, description = "GGUF files in the repo", body = Vec<HfQuantVariant>)
    )
)]
pub async fn get_repo_files(
    Path((author, repo)): Path<(String, String)>,
) -> Result<Json<Vec<HfQuantVariant>>, ErrorResponse> {
    let repo_id = format!("{}/{}", author, repo);
    let files = get_repo_gguf_files(&repo_id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to fetch files: {}", e)))?;

    let variants: Vec<HfQuantVariant> = files
        .into_iter()
        .map(|f| HfQuantVariant {
            filename: f.filename,
            size_bytes: f.size_bytes,
            quantization: f.quantization,
            download_url: f.download_url,
            quality_rank: 0,
        })
        .collect();

    Ok(Json(variants))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DownloadModelRequest {
    /// Model spec like "bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M"
    pub spec: Option<String>,
    /// Alternative: provide repo_id and filename separately
    pub repo_id: Option<String>,
    pub filename: Option<String>,
}

#[utoipa::path(
    post,
    path = "/local-inference/download",
    request_body = DownloadModelRequest,
    responses(
        (status = 202, description = "Download started", body = String),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn download_hf_model(
    Json(req): Json<DownloadModelRequest>,
) -> Result<(StatusCode, Json<String>), ErrorResponse> {
    let (repo_id, quantization, hf_file) = if let Some(spec) = &req.spec {
        // Parse spec like "bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M"
        let (_repo, file) = resolve_model_spec(spec)
            .await
            .map_err(|e| ErrorResponse::bad_request(format!("Invalid spec: {}", e)))?;

        let (repo, quant) = parse_model_spec(spec)
            .ok_or_else(|| ErrorResponse::bad_request("Invalid spec format"))?;

        (repo.to_string(), quant.to_string(), file)
    } else if let (Some(repo_id), Some(filename)) = (&req.repo_id, &req.filename) {
        // Get file info from repo
        let files = get_repo_gguf_files(repo_id)
            .await
            .map_err(|e| ErrorResponse::internal(format!("Failed to fetch files: {}", e)))?;

        let file = files
            .into_iter()
            .find(|f| f.filename == *filename)
            .ok_or_else(|| ErrorResponse::not_found("File not found in repo"))?;

        let quantization = file.quantization.clone();
        (repo_id.clone(), quantization, file)
    } else {
        return Err(ErrorResponse::bad_request(
            "Must provide either 'spec' or both 'repo_id' and 'filename'",
        ));
    };

    let model_id = model_id_from_repo(&repo_id, &quantization);
    let local_path = Paths::in_data_dir("models").join(&hf_file.filename);
    let download_url = hf_file.download_url.clone();

    // Create registry entry
    let entry = LocalModelEntry {
        id: model_id.clone(),
        display_name: display_name_from_repo(&repo_id, &quantization),
        repo_id: repo_id.to_string(),
        filename: hf_file.filename,
        quantization: quantization.to_string(),
        local_path: local_path.clone(),
        source_url: download_url.clone(),
        settings: ModelSettings::default(),
        size_bytes: hf_file.size_bytes,
    };

    {
        let mut registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;
        registry
            .add_model(entry)
            .map_err(|e| handle_internal_error(e))?;
    }

    // Start download
    let dm = goose::dictation::download_manager::get_download_manager();
    dm.download_model(
        format!("{}-model", model_id),
        download_url,
        local_path,
        None,
        None,
    )
    .await
    .map_err(|e| ErrorResponse::internal(format!("Download failed: {}", e)))?;

    Ok((StatusCode::ACCEPTED, Json(model_id)))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DownloadProgressResponse {
    pub model_id: String,
    pub status: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub progress_percent: f32,
    pub speed_bps: Option<u64>,
    pub eta_seconds: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/local-inference/models/{model_id}/download",
    responses(
        (status = 200, description = "Download progress", body = DownloadProgressResponse),
        (status = 404, description = "No active download")
    )
)]
pub async fn get_local_model_download_progress(
    Path(model_id): Path<String>,
) -> Result<Json<DownloadProgressResponse>, ErrorResponse> {
    let model_id = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model_id encoding"))?
        .into_owned();

    let download_id = format!("{}-model", model_id);
    debug!(
        model_id = %model_id,
        download_id = %download_id,
        "Getting download progress"
    );

    let dm = goose::dictation::download_manager::get_download_manager();

    if let Some(progress) = dm.get_progress(&download_id) {
        return Ok(Json(DownloadProgressResponse {
            model_id,
            status: format!("{:?}", progress.status).to_lowercase(),
            bytes_downloaded: progress.bytes_downloaded,
            total_bytes: progress.total_bytes,
            progress_percent: progress.progress_percent,
            speed_bps: progress.speed_bps,
            eta_seconds: progress.eta_seconds,
        }));
    }

    // Check if the model file exists (download completed)
    let registry = get_registry()
        .lock()
        .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

    if let Some(entry) = registry.get_model(&model_id) {
        if entry.local_path.exists() {
            let size = std::fs::metadata(&entry.local_path)
                .map(|m| m.len())
                .unwrap_or(0);
            return Ok(Json(DownloadProgressResponse {
                model_id,
                status: "completed".to_string(),
                bytes_downloaded: size,
                total_bytes: size,
                progress_percent: 100.0,
                speed_bps: None,
                eta_seconds: None,
            }));
        }
    }

    Err(ErrorResponse::not_found("No active download"))
}

#[utoipa::path(
    delete,
    path = "/local-inference/models/{model_id}/download",
    responses(
        (status = 200, description = "Download cancelled"),
        (status = 404, description = "No active download")
    )
)]
pub async fn cancel_local_model_download(
    Path(model_id): Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    let model_id = urlencoding::decode(&model_id)
        .map_err(|_| ErrorResponse::bad_request("Invalid model_id encoding"))?
        .into_owned();

    let download_id = format!("{}-model", model_id);
    let dm = goose::dictation::download_manager::get_download_manager();
    let _ = dm.cancel_download(&download_id);

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

    let local_path = {
        let registry = get_registry()
            .lock()
            .map_err(|_| ErrorResponse::internal("Failed to acquire registry lock"))?;

        let entry = registry
            .get_model(&model_id)
            .ok_or_else(|| ErrorResponse::not_found("Model not found"))?;

        entry.local_path.clone()
    };

    // Delete the file
    if local_path.exists() {
        std::fs::remove_file(&local_path)
            .map_err(|e| ErrorResponse::internal(format!("Failed to delete: {}", e)))?;
    }

    // Remove from registry if not a featured model
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
        let s: ModelSettings = settings.clone();
        return Ok(Json(s));
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
