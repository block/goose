use super::hf_models::{
    self, register_resolved_model, resolve_local_model_selection, resolve_local_model_spec,
    resolve_model_spec, HfGgufFile, HfModelInfo, HfModelVariant,
};
use super::local_model_registry::{
    default_settings_for_model, featured_mmproj_spec, get_registry, model_id_from_repo,
    ChatTemplate, LocalModelEntry, LocalModelStorage, ModelDownloadStatus, ModelSettings,
    SamplingConfig, ToolCallingMode, FEATURED_MODELS,
};
use super::{
    available_inference_memory_bytes, builtin_chat_template_names, recommend_local_model,
    InferenceRuntime,
};
use crate::download_manager::{get_download_manager, DownloadProgress, DownloadStatus};
use crate::huggingface_auth;
use crate::paths::Paths;
use anyhow::{anyhow, Result};
use futures::future::join_all;
use goose_sdk_types::custom_requests::{
    LocalInferenceBuiltinChatTemplatesListResponse, LocalInferenceChatTemplate,
    LocalInferenceDownloadProgressDto, LocalInferenceDownloadState, LocalInferenceHfGgufFileDto,
    LocalInferenceHfModelInfoDto, LocalInferenceHfModelVariantDto,
    LocalInferenceHuggingFaceRepoVariantsResponse, LocalInferenceHuggingFaceSearchResponse,
    LocalInferenceModelDownloadRequest, LocalInferenceModelDownloadResponse,
    LocalInferenceModelDownloadStatusDto, LocalInferenceModelDto, LocalInferenceModelSettingsDto,
    LocalInferenceModelSettingsReadResponse, LocalInferenceModelSettingsUpdateResponse,
    LocalInferenceModelsListResponse, LocalInferenceSamplingConfig, LocalInferenceToolCallingMode,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

static MANAGEMENT_RUNTIME: OnceLock<Arc<InferenceRuntime>> = OnceLock::new();

#[derive(Clone)]
struct LocalModelSelection {
    repo_id: String,
    backend_id: String,
    variant_id: Option<String>,
}

pub async fn list_models() -> Result<LocalInferenceModelsListResponse> {
    ensure_featured_models_current().await?;

    let runtime = management_runtime()?;
    let recommended_id = recommend_local_model(&runtime);

    let loaded_model_ids = crate::loaded_model_ids()
        .await
        .map_err(|error| anyhow!(error.to_string()))?;
    let registry = get_registry()
        .lock()
        .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
    let mut models: Vec<LocalInferenceModelDto> = registry
        .list_models()
        .iter()
        .map(|entry| local_model_to_dto(entry, &recommended_id, &loaded_model_ids))
        .collect();

    models.sort_by(|a, b| {
        let a_downloaded = a.status.state == LocalInferenceDownloadState::Downloaded;
        let b_downloaded = b.status.state == LocalInferenceDownloadState::Downloaded;
        match (b_downloaded, a_downloaded) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.id.cmp(&b.id),
        }
    });

    Ok(LocalInferenceModelsListResponse { models })
}

pub async fn search_huggingface_models(
    query: String,
    limit: Option<usize>,
) -> Result<LocalInferenceHuggingFaceSearchResponse> {
    let limit = limit.unwrap_or(20).min(50);
    let models = hf_models::search_local_models(&query, limit)
        .await?
        .into_iter()
        .map(hf_model_info_to_dto)
        .collect();
    Ok(LocalInferenceHuggingFaceSearchResponse { models })
}

pub async fn huggingface_repo_variants(
    repo_id: String,
) -> Result<LocalInferenceHuggingFaceRepoVariantsResponse> {
    let variants = hf_models::get_repo_local_variants(&repo_id).await?;

    let runtime = management_runtime()?;
    let available_memory = available_inference_memory_bytes(&runtime);
    let gguf_variants: Vec<_> = variants
        .iter()
        .filter(|variant| variant.backend_id == "llamacpp")
        .map(|variant| hf_models::HfQuantVariant {
            quantization: variant.variant_id.clone(),
            size_bytes: variant.size_bytes,
            filename: variant.filename.clone().unwrap_or_default(),
            download_url: variant.download_url.clone().unwrap_or_default(),
            description: "",
            quality_rank: variant.quality_rank,
            sharded: variant.sharded,
        })
        .collect();
    let recommended_index = hf_models::recommend_variant(&gguf_variants, available_memory);

    let (downloaded_quants, downloaded_variants) = {
        let registry = get_registry()
            .lock()
            .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
        let models: Vec<_> = registry
            .list_models()
            .iter()
            .filter(|m| m.repo_id == repo_id && m.is_downloaded())
            .collect();
        (
            models.iter().map(|m| m.quantization.clone()).collect(),
            models.iter().map(|m| m.id.clone()).collect(),
        )
    };

    Ok(LocalInferenceHuggingFaceRepoVariantsResponse {
        variants: variants.into_iter().map(hf_model_variant_to_dto).collect(),
        recommended_index,
        available_memory_bytes: available_memory,
        downloaded_quants,
        downloaded_variants,
    })
}

pub async fn download_model(
    req: LocalInferenceModelDownloadRequest,
) -> Result<LocalInferenceModelDownloadResponse> {
    let selection = explicit_model_selection(&req)?;
    let model_id = local_model_id_from_request(&req, selection.as_ref()).await?;
    let download_id = format!("{}-model", model_id);
    let download_reserved = get_download_manager().reserve_download(DownloadProgress {
        model_id: download_id,
        status: DownloadStatus::Downloading,
        bytes_downloaded: 0,
        total_bytes: 0,
        progress_percent: 0.0,
        speed_bps: None,
        eta_seconds: None,
        error: None,
        task_exited: false,
    })?;
    if !download_reserved {
        return Ok(LocalInferenceModelDownloadResponse { model_id });
    }

    if let Err(error) = register_pending_download_model(&model_id, &req, selection.as_ref()) {
        mark_download_failed(&model_id, &error);
        return Err(error.context("Failed to register download"));
    }

    let spec = req.spec.clone();
    let selection_for_task = selection.clone();
    let model_id_for_task = model_id.clone();
    tokio::spawn(async move {
        let resolved = if let Some(selection) = selection_for_task {
            resolve_local_model_selection(
                &selection.repo_id,
                &selection.backend_id,
                selection.variant_id.as_deref(),
            )
            .await
        } else {
            resolve_local_model_spec(&spec).await
        };
        match resolved {
            Ok(resolved) => {
                if !model_download_completed(&model_id_for_task) {
                    return;
                }
                if let Err(error) = register_resolved_model(resolved, &spec) {
                    mark_download_failed(&model_id_for_task, error);
                }
            }
            Err(error) => mark_download_failed(&model_id_for_task, error),
        }
    });

    Ok(LocalInferenceModelDownloadResponse { model_id })
}

pub fn download_progress(model_id: &str) -> Result<Option<LocalInferenceDownloadProgressDto>> {
    Ok(get_download_manager()
        .get_progress(&format!("{}-model", model_id))
        .map(download_progress_to_dto))
}

pub fn cancel_download(model_id: &str) -> Result<()> {
    let manager = get_download_manager();
    manager.cancel_download(&format!("{}-model", model_id))?;
    let _ = manager.cancel_download(&format!("{}-mmproj", model_id));
    Ok(())
}

pub fn delete_model(model_id: &str) -> Result<()> {
    let mut registry = get_registry()
        .lock()
        .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
    if registry.get_model(model_id).is_none() {
        anyhow::bail!("Model not found");
    }
    registry.delete_model(model_id)
}

pub fn model_exists(model_id: &str) -> Result<bool> {
    let registry = get_registry()
        .lock()
        .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
    Ok(registry.get_model(model_id).is_some())
}

pub async fn evict_model(model_id: &str) -> Result<()> {
    crate::evict_model(model_id)
        .await
        .map(|_| ())
        .map_err(|error| anyhow!(error.to_string()))
}

pub fn get_model_settings(model_id: &str) -> Result<LocalInferenceModelSettingsReadResponse> {
    let registry = get_registry()
        .lock()
        .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
    let settings = registry
        .get_model_settings(model_id)
        .ok_or_else(|| anyhow!("Model not found"))?;
    Ok(LocalInferenceModelSettingsReadResponse {
        settings: model_settings_to_dto(settings),
    })
}

pub fn update_model_settings(
    model_id: &str,
    settings: LocalInferenceModelSettingsDto,
) -> Result<LocalInferenceModelSettingsUpdateResponse> {
    let settings = model_settings_from_dto(settings);
    let mut registry = get_registry()
        .lock()
        .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
    registry.update_model_settings(model_id, settings.clone())?;
    Ok(LocalInferenceModelSettingsUpdateResponse {
        settings: model_settings_to_dto(&settings),
    })
}

pub fn list_builtin_chat_templates() -> LocalInferenceBuiltinChatTemplatesListResponse {
    LocalInferenceBuiltinChatTemplatesListResponse {
        templates: builtin_chat_template_names(),
    }
}

fn management_runtime() -> Result<Arc<InferenceRuntime>> {
    if let Some(runtime) = MANAGEMENT_RUNTIME.get() {
        return Ok(runtime.clone());
    }

    let runtime = InferenceRuntime::get_or_init()?;
    match MANAGEMENT_RUNTIME.set(runtime.clone()) {
        Ok(()) => Ok(runtime),
        Err(_) => Ok(MANAGEMENT_RUNTIME
            .get()
            .expect("local inference management runtime initialized by another thread")
            .clone()),
    }
}

pub async fn ensure_featured_models_current() -> Result<()> {
    let mut mmproj_downloads_needed: Vec<(String, String, PathBuf)> = Vec::new();

    struct PendingResolve {
        spec: &'static str,
        repo_id: String,
        quantization: String,
        model_id: String,
    }
    let mut to_resolve = Vec::new();

    for featured in FEATURED_MODELS {
        let (repo_id, quantization) = match hf_models::parse_model_spec(featured.spec) {
            Ok(parts) => parts,
            Err(_) => continue,
        };

        let model_id = model_id_from_repo(&repo_id, &quantization);

        {
            let registry = get_registry()
                .lock()
                .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
            if let Some(existing) = registry.get_model(&model_id) {
                let needs_backfill = existing.mmproj_path.is_none() && featured.mmproj.is_some();
                let needs_size = existing.size_bytes == 0 && !existing.is_downloaded();
                let needs_download = existing.is_downloaded()
                    && featured.mmproj.is_some()
                    && !existing.mmproj_path.as_ref().is_some_and(|p| p.exists());

                if needs_download {
                    if let Some(mmproj) = featured.mmproj.as_ref() {
                        let path = mmproj.local_path();
                        let url = format!(
                            "https://huggingface.co/{}/resolve/main/{}",
                            mmproj.repo, mmproj.filename
                        );
                        mmproj_downloads_needed.push((model_id.clone(), url, path));
                    }
                }

                if !needs_backfill && !needs_size {
                    continue;
                }
            }
        }

        to_resolve.push(PendingResolve {
            spec: featured.spec,
            repo_id,
            quantization,
            model_id,
        });
    }

    let resolved: Vec<(PendingResolve, HfGgufFile)> =
        join_all(to_resolve.into_iter().map(|pending| async move {
            let hf_file = match resolve_model_spec(pending.spec).await {
                Ok((_repo, file)) => file,
                Err(_) => {
                    let filename = format!(
                        "{}-{}.gguf",
                        pending.repo_id.split('/').next_back().unwrap_or("model"),
                        pending.quantization
                    );
                    HfGgufFile {
                        filename: filename.clone(),
                        size_bytes: 0,
                        quantization: pending.quantization.to_string(),
                        download_url: format!(
                            "https://huggingface.co/{}/resolve/main/{}",
                            pending.repo_id, filename
                        ),
                    }
                }
            };
            (pending, hf_file)
        }))
        .await;

    let entries_to_add: Vec<LocalModelEntry> = resolved
        .into_iter()
        .map(|(pending, hf_file)| {
            let local_path = Paths::in_data_dir("models").join(&hf_file.filename);
            let settings = default_settings_for_model(&pending.model_id);
            LocalModelEntry {
                id: pending.model_id,
                repo_id: pending.repo_id,
                filename: hf_file.filename,
                quantization: pending.quantization,
                local_path,
                source_url: hf_file.download_url,
                backend_id: settings.backend_id.clone(),
                storage: LocalModelStorage::GooseManaged,
                settings,
                size_bytes: hf_file.size_bytes,
                mmproj_path: None,
                mmproj_source_url: None,
                mmproj_size_bytes: 0,
                mmproj_checked: false,
                shard_files: vec![],
            }
        })
        .collect();

    {
        let mut registry = get_registry()
            .lock()
            .map_err(|_| anyhow!("Failed to acquire registry lock"))?;

        if !entries_to_add.is_empty() {
            registry.sync_with_featured(entries_to_add);
        }

        for model in registry.list_models_mut() {
            model.enrich_with_featured_mmproj();
            if model.is_downloaded() {
                if let Some(mmproj) = featured_mmproj_spec(&model.id) {
                    let path = mmproj.local_path();
                    if !path.exists() {
                        let url = format!(
                            "https://huggingface.co/{}/resolve/main/{}",
                            mmproj.repo, mmproj.filename
                        );
                        mmproj_downloads_needed.push((model.id.clone(), url, path));
                    }
                }
            }
        }
        let _ = registry.save();
    }

    let dm = get_download_manager();
    let hf_token = huggingface_auth::resolve_token_async().await.ok().flatten();
    let mut started_paths = std::collections::HashSet::new();
    for (model_id, url, path) in mmproj_downloads_needed {
        if !path.exists() && started_paths.insert(path.clone()) {
            let download_id = format!("{}-mmproj", model_id);
            let dominated_by_active = dm
                .get_progress(&download_id)
                .is_some_and(|p| p.status == DownloadStatus::Downloading);
            if !dominated_by_active {
                tracing::info!(model_id = %model_id, "Auto-downloading vision encoder for existing model");
                if let Err(e) = dm
                    .download_model_with_bearer_token(
                        download_id,
                        url,
                        path,
                        hf_token.clone(),
                        None,
                    )
                    .await
                {
                    tracing::warn!(model_id = %model_id, error = %e, "Failed to start mmproj download");
                }
            }
        }
    }

    Ok(())
}

fn local_model_to_dto(
    entry: &LocalModelEntry,
    recommended_id: &str,
    loaded_model_ids: &HashSet<String>,
) -> LocalInferenceModelDto {
    let vision_capable = entry.settings.vision_capable;
    LocalInferenceModelDto {
        id: entry.id.clone(),
        repo_id: entry.repo_id.clone(),
        filename: entry.filename.clone(),
        quantization: entry.quantization.clone(),
        size_bytes: entry.file_size(),
        status: model_download_status_to_dto(entry.download_status()),
        recommended: recommended_id == entry.id,
        is_loaded: loaded_model_ids.contains(&entry.id),
        settings: model_settings_to_dto(&entry.settings),
        vision_capable,
        mmproj_status: vision_capable
            .then(|| model_download_status_to_dto(entry.mmproj_download_status())),
    }
}

fn model_download_status_to_dto(
    status: ModelDownloadStatus,
) -> LocalInferenceModelDownloadStatusDto {
    match status {
        ModelDownloadStatus::NotDownloaded => LocalInferenceModelDownloadStatusDto {
            state: LocalInferenceDownloadState::NotDownloaded,
            ..Default::default()
        },
        ModelDownloadStatus::Downloading {
            progress_percent,
            bytes_downloaded,
            total_bytes,
            speed_bps,
        } => LocalInferenceModelDownloadStatusDto {
            state: LocalInferenceDownloadState::Downloading,
            progress_percent: Some(progress_percent),
            bytes_downloaded: Some(bytes_downloaded),
            total_bytes: Some(total_bytes),
            speed_bps: Some(speed_bps),
        },
        ModelDownloadStatus::Downloaded => LocalInferenceModelDownloadStatusDto {
            state: LocalInferenceDownloadState::Downloaded,
            ..Default::default()
        },
    }
}

fn download_progress_to_dto(progress: DownloadProgress) -> LocalInferenceDownloadProgressDto {
    LocalInferenceDownloadProgressDto {
        model_id: progress.model_id,
        status: serde_json::to_value(progress.status)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| "unknown".to_string()),
        bytes_downloaded: progress.bytes_downloaded,
        total_bytes: progress.total_bytes,
        progress_percent: progress.progress_percent,
        speed_bps: progress.speed_bps,
        eta_seconds: progress.eta_seconds,
        error: progress.error,
        task_exited: progress.task_exited,
    }
}

fn hf_model_info_to_dto(model: HfModelInfo) -> LocalInferenceHfModelInfoDto {
    LocalInferenceHfModelInfoDto {
        repo_id: model.repo_id,
        author: model.author,
        model_name: model.model_name,
        downloads: model.downloads,
        gguf_files: model
            .gguf_files
            .into_iter()
            .map(|file| LocalInferenceHfGgufFileDto {
                filename: file.filename,
                size_bytes: file.size_bytes,
                quantization: file.quantization,
                download_url: file.download_url,
            })
            .collect(),
        variants: model
            .variants
            .into_iter()
            .map(hf_model_variant_to_dto)
            .collect(),
    }
}

fn hf_model_variant_to_dto(variant: HfModelVariant) -> LocalInferenceHfModelVariantDto {
    LocalInferenceHfModelVariantDto {
        variant_id: variant.variant_id,
        label: variant.label,
        backend_id: variant.backend_id,
        format: variant.format,
        model_id: variant.model_id,
        download_id: variant.download_id,
        size_bytes: variant.size_bytes,
        filename: variant.filename,
        download_url: variant.download_url,
        description: variant.description,
        quality_rank: variant.quality_rank,
        sharded: variant.sharded,
        supported: variant.supported,
        unsupported_reason: variant.unsupported_reason,
    }
}

pub fn model_settings_to_dto(settings: &ModelSettings) -> LocalInferenceModelSettingsDto {
    LocalInferenceModelSettingsDto {
        backend_id: settings.backend_id.clone(),
        context_size: settings.context_size,
        max_output_tokens: settings.max_output_tokens,
        draft_model: settings.draft_model.clone(),
        sampling: sampling_to_dto(&settings.sampling),
        repeat_penalty: settings.repeat_penalty,
        repeat_last_n: settings.repeat_last_n,
        frequency_penalty: settings.frequency_penalty,
        presence_penalty: settings.presence_penalty,
        n_batch: settings.n_batch,
        n_gpu_layers: settings.n_gpu_layers,
        use_mlock: settings.use_mlock,
        flash_attention: settings.flash_attention,
        n_threads: settings.n_threads,
        tool_calling: tool_calling_to_dto(settings.tool_calling),
        chat_template: chat_template_to_dto(&settings.chat_template),
        enable_thinking: settings.enable_thinking,
        vision_capable: settings.vision_capable,
        image_token_estimate: settings.image_token_estimate,
        mmproj_size_bytes: settings.mmproj_size_bytes,
    }
}

pub fn model_settings_from_dto(settings: LocalInferenceModelSettingsDto) -> ModelSettings {
    ModelSettings {
        backend_id: settings.backend_id,
        context_size: settings.context_size,
        max_output_tokens: settings.max_output_tokens,
        draft_model: settings.draft_model,
        sampling: sampling_from_dto(settings.sampling),
        repeat_penalty: settings.repeat_penalty,
        repeat_last_n: settings.repeat_last_n,
        frequency_penalty: settings.frequency_penalty,
        presence_penalty: settings.presence_penalty,
        n_batch: settings.n_batch,
        n_gpu_layers: settings.n_gpu_layers,
        use_mlock: settings.use_mlock,
        flash_attention: settings.flash_attention,
        n_threads: settings.n_threads,
        tool_calling: tool_calling_from_dto(settings.tool_calling),
        chat_template: chat_template_from_dto(settings.chat_template),
        enable_thinking: settings.enable_thinking,
        vision_capable: settings.vision_capable,
        image_token_estimate: settings.image_token_estimate,
        mmproj_size_bytes: settings.mmproj_size_bytes,
    }
}

fn sampling_to_dto(sampling: &SamplingConfig) -> LocalInferenceSamplingConfig {
    match sampling {
        SamplingConfig::Greedy => LocalInferenceSamplingConfig::Greedy,
        SamplingConfig::Temperature {
            temperature,
            top_k,
            top_p,
            min_p,
            seed,
        } => LocalInferenceSamplingConfig::Temperature {
            temperature: *temperature,
            top_k: *top_k,
            top_p: *top_p,
            min_p: *min_p,
            seed: *seed,
        },
        SamplingConfig::MirostatV2 { tau, eta, seed } => LocalInferenceSamplingConfig::MirostatV2 {
            tau: *tau,
            eta: *eta,
            seed: *seed,
        },
    }
}

fn sampling_from_dto(sampling: LocalInferenceSamplingConfig) -> SamplingConfig {
    match sampling {
        LocalInferenceSamplingConfig::Greedy => SamplingConfig::Greedy,
        LocalInferenceSamplingConfig::Temperature {
            temperature,
            top_k,
            top_p,
            min_p,
            seed,
        } => SamplingConfig::Temperature {
            temperature,
            top_k,
            top_p,
            min_p,
            seed,
        },
        LocalInferenceSamplingConfig::MirostatV2 { tau, eta, seed } => {
            SamplingConfig::MirostatV2 { tau, eta, seed }
        }
    }
}

fn tool_calling_to_dto(mode: ToolCallingMode) -> LocalInferenceToolCallingMode {
    match mode {
        ToolCallingMode::Auto => LocalInferenceToolCallingMode::Auto,
        ToolCallingMode::ForceNative => LocalInferenceToolCallingMode::ForceNative,
        ToolCallingMode::ForceEmulated => LocalInferenceToolCallingMode::ForceEmulated,
    }
}

fn tool_calling_from_dto(mode: LocalInferenceToolCallingMode) -> ToolCallingMode {
    match mode {
        LocalInferenceToolCallingMode::Auto => ToolCallingMode::Auto,
        LocalInferenceToolCallingMode::ForceNative => ToolCallingMode::ForceNative,
        LocalInferenceToolCallingMode::ForceEmulated => ToolCallingMode::ForceEmulated,
    }
}

fn chat_template_to_dto(template: &ChatTemplate) -> LocalInferenceChatTemplate {
    match template {
        ChatTemplate::Embedded => LocalInferenceChatTemplate::Embedded,
        ChatTemplate::Builtin { name } => {
            LocalInferenceChatTemplate::Builtin { name: name.clone() }
        }
        ChatTemplate::CustomInline { template } => LocalInferenceChatTemplate::CustomInline {
            template: template.clone(),
        },
    }
}

fn chat_template_from_dto(template: LocalInferenceChatTemplate) -> ChatTemplate {
    match template {
        LocalInferenceChatTemplate::Embedded => ChatTemplate::Embedded,
        LocalInferenceChatTemplate::Builtin { name } => ChatTemplate::Builtin { name },
        LocalInferenceChatTemplate::CustomInline { template } => {
            ChatTemplate::CustomInline { template }
        }
    }
}

fn explicit_model_selection(
    req: &LocalInferenceModelDownloadRequest,
) -> Result<Option<LocalModelSelection>> {
    if let Some(backend_id) = req.backend_id.as_deref() {
        let (repo_id, parsed_variant_id) = hf_models::parse_model_spec(&req.spec)
            .map(|(repo_id, quantization)| (repo_id, Some(quantization)))
            .unwrap_or_else(|_| (req.spec.clone(), None));
        let variant_id = req.variant_id.clone().or(parsed_variant_id);
        match backend_id {
            "mlx" | "llamacpp" => Ok(Some(LocalModelSelection {
                repo_id,
                backend_id: backend_id.to_string(),
                variant_id,
            })),
            _ => anyhow::bail!("Unknown local inference backend '{}'", backend_id),
        }
    } else {
        Ok(None)
    }
}

async fn local_model_id_from_request(
    req: &LocalInferenceModelDownloadRequest,
    selection: Option<&LocalModelSelection>,
) -> Result<String> {
    if let Some(selection) = selection {
        return match selection.backend_id.as_str() {
            "mlx" => Ok(selection.repo_id.clone()),
            "llamacpp" => {
                let quantization = selection.variant_id.as_deref().ok_or_else(|| {
                    anyhow!(
                        "llama.cpp model '{}' is missing a quantization",
                        selection.repo_id
                    )
                })?;
                Ok(model_id_from_repo(&selection.repo_id, quantization))
            }
            _ => anyhow::bail!("Unknown local inference backend '{}'", selection.backend_id),
        };
    }

    if let Ok((repo_id, quantization)) = hf_models::parse_model_spec(&req.spec) {
        return Ok(model_id_from_repo(&repo_id, &quantization));
    }

    let variants = hf_models::get_repo_local_variants(&req.spec).await?;
    let has_llamacpp = variants
        .iter()
        .any(|variant| variant.backend_id == "llamacpp");
    let mlx_variants: Vec<_> = variants
        .iter()
        .filter(|variant| variant.backend_id == "mlx")
        .collect();
    if mlx_variants.len() == 1 && !has_llamacpp {
        Ok(req.spec.clone())
    } else {
        anyhow::bail!(
            "Model spec '{}' is ambiguous; choose one of: {}",
            req.spec,
            variants
                .iter()
                .map(|variant| variant.download_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn mark_download_failed(model_id: &str, error: impl std::fmt::Display) {
    let manager = get_download_manager();
    let download_id = format!("{}-model", model_id);
    if manager.get_progress(&download_id).is_none() {
        manager.set_progress(DownloadProgress {
            model_id: download_id.clone(),
            status: DownloadStatus::Failed,
            bytes_downloaded: 0,
            total_bytes: 0,
            progress_percent: 0.0,
            speed_bps: None,
            eta_seconds: None,
            error: Some(error.to_string()),
            task_exited: true,
        });
        return;
    }

    manager.update_progress(&download_id, |progress| {
        if progress.status != DownloadStatus::Cancelled {
            progress.status = DownloadStatus::Failed;
            progress.error = Some(error.to_string());
        }
        progress.task_exited = true;
    });
}

fn model_download_completed(model_id: &str) -> bool {
    get_download_manager()
        .get_progress(&format!("{}-model", model_id))
        .is_some_and(|progress| progress.status == DownloadStatus::Completed)
}

fn register_pending_download_model(
    model_id: &str,
    req: &LocalInferenceModelDownloadRequest,
    selection: Option<&LocalModelSelection>,
) -> Result<()> {
    let (repo_id, backend_id, variant_id) = if let Some(selection) = selection {
        (
            selection.repo_id.clone(),
            selection.backend_id.clone(),
            selection
                .variant_id
                .clone()
                .unwrap_or_else(|| "default".to_string()),
        )
    } else if let Ok((repo_id, quantization)) = hf_models::parse_model_spec(&req.spec) {
        (repo_id, "llamacpp".to_string(), quantization)
    } else {
        (req.spec.clone(), "mlx".to_string(), "default".to_string())
    };

    let mut registry = get_registry()
        .lock()
        .map_err(|_| anyhow!("Failed to acquire registry lock"))?;
    if registry.has_model(model_id) {
        return Ok(());
    }

    let mut settings = default_settings_for_model(model_id);
    if backend_id != "llamacpp" {
        settings.backend_id = Some(backend_id.clone());
    }

    let filename = variant_id.clone();
    registry.add_model(LocalModelEntry {
        id: model_id.to_string(),
        repo_id,
        filename: filename.clone(),
        quantization: variant_id,
        local_path: Paths::in_data_dir("models").join(filename),
        source_url: req.spec.clone(),
        backend_id: settings.backend_id.clone(),
        storage: LocalModelStorage::HuggingFaceCache,
        settings,
        size_bytes: 0,
        mmproj_path: None,
        mmproj_source_url: None,
        mmproj_size_bytes: 0,
        mmproj_checked: false,
        shard_files: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_preserves_defaults() {
        let settings = ModelSettings::default();
        let dto = model_settings_to_dto(&settings);
        let round_trip = model_settings_from_dto(dto);
        assert_eq!(round_trip.repeat_penalty, settings.repeat_penalty);
        assert_eq!(round_trip.repeat_last_n, settings.repeat_last_n);
        assert_eq!(round_trip.enable_thinking, settings.enable_thinking);
        assert_eq!(
            round_trip.image_token_estimate,
            settings.image_token_estimate
        );
    }

    #[tokio::test]
    async fn explicit_llamacpp_selection_derives_quantized_model_id() {
        let req = LocalInferenceModelDownloadRequest {
            spec: "test/repo".to_string(),
            backend_id: Some("llamacpp".to_string()),
            variant_id: Some("Q4_K_M".to_string()),
        };
        let selection = explicit_model_selection(&req).unwrap();
        let model_id = local_model_id_from_request(&req, selection.as_ref())
            .await
            .unwrap();
        assert_eq!(model_id, "test/repo:Q4_K_M");
    }
}
