#![allow(dead_code)]
#![allow(clippy::type_complexity)]
#![allow(clippy::field_reassign_with_default)]

mod download_task;
mod model_id;
mod progress;
mod storage;

pub use download_task::{DownloadError, DownloadProgress, DownloadStatus, DownloadTask};
pub use model_id::{ModelIdError, ModelIdentifier};
pub use storage::{LocalModelInfo, ModelMetadata, StorageManager};

use futures::StreamExt;
use progress::ProgressTracker;
use std::collections::HashMap;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::providers::api_client::{ApiClient, AuthMethod};

pub struct DownloadManager {
    active_downloads: Arc<RwLock<HashMap<String, Arc<DownloadTask>>>>,
    storage: Arc<StorageManager>,
    max_concurrent_downloads: usize,
    default_hf_token: Option<String>,
}

impl DownloadManager {
    pub fn new() -> Result<Self, DownloadError> {
        let max_concurrent = env::var("GOOSE_MAX_CONCURRENT_DOWNLOADS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        let default_hf_token = env::var("GOOSE_HF_TOKEN").ok();

        let storage = Arc::new(StorageManager::new()?);

        Ok(Self {
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            storage,
            max_concurrent_downloads: max_concurrent,
            default_hf_token,
        })
    }

    pub async fn start_download(
        &self,
        model_id: String,
        auth_token: Option<String>,
    ) -> Result<Arc<DownloadTask>, DownloadError> {
        // Parse and normalize model ID
        let identifier = ModelIdentifier::parse(&model_id)
            .map_err(|e| DownloadError::InvalidModelId(e.to_string()))?;

        // Check concurrent download limit
        let active_count = self.active_downloads.read().await.len();
        if active_count >= self.max_concurrent_downloads {
            return Err(DownloadError::ConcurrentLimitExceeded(
                self.max_concurrent_downloads,
            ));
        }

        // Check if already downloading
        if self
            .active_downloads
            .read()
            .await
            .contains_key(&identifier.normalized)
        {
            return Err(DownloadError::AlreadyDownloading(model_id));
        }

        // Check if already downloaded
        if self.storage.model_exists(&identifier.normalized).await {
            return Err(DownloadError::AlreadyExists(model_id));
        }

        // Create download task
        let task = Arc::new(DownloadTask::new(
            model_id.clone(),
            identifier.normalized.clone(),
        ));

        // Spawn background download task
        let task_clone = task.clone();
        let storage = self.storage.clone();
        let token = auth_token.or_else(|| self.default_hf_token.clone());
        let identifier_clone = identifier.clone();

        let handle = tokio::spawn(async move {
            *task_clone.status.write().await = DownloadStatus::Downloading;

            let result = Self::download_model_file(
                &identifier_clone,
                token,
                task_clone.progress.clone(),
                task_clone.cancel_token.clone(),
                storage.clone(),
            )
            .await;

            match result {
                Ok(local_info) => {
                    *task_clone.status.write().await = DownloadStatus::Completed;
                    Ok(local_info)
                }
                Err(e) => {
                    if matches!(e, DownloadError::Cancelled) {
                        *task_clone.status.write().await = DownloadStatus::Cancelled;
                    } else {
                        *task_clone.status.write().await = DownloadStatus::Failed;
                    }
                    Err(e)
                }
            }
        });

        *task.handle.write().await = Some(handle);

        // Store in active downloads
        self.active_downloads
            .write()
            .await
            .insert(identifier.normalized.clone(), task.clone());

        Ok(task)
    }

    async fn download_model_file(
        identifier: &ModelIdentifier,
        auth_token: Option<String>,
        progress: Arc<RwLock<DownloadProgress>>,
        cancel_token: tokio_util::sync::CancellationToken,
        storage: Arc<StorageManager>,
    ) -> Result<LocalModelInfo, DownloadError> {
        // Check for partial file to resume
        let resume_from = if storage.partial_exists(&identifier.normalized) {
            storage.get_partial_size(&identifier.normalized).unwrap_or(0)
        } else {
            0
        };

        let tracker = ProgressTracker::new(resume_from);

        // Build HuggingFace URL
        let url = identifier.to_download_url("model.gguf");

        // Create ApiClient with optional Bearer token
        let auth = match auth_token {
            Some(token) => AuthMethod::BearerToken(token),
            None => AuthMethod::NoAuth,
        };

        let client = ApiClient::new(url, auth)?;

        // Build request with Range header for resume
        let mut builder = client.request(None, "");
        if resume_from > 0 {
            builder = builder.header("Range", &format!("bytes={}-", resume_from))?;
        }

        // Send request
        let response = builder.response_get().await?;
        let status = response.status();

        // Check for auth errors
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(DownloadError::AuthenticationRequired);
        }

        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(DownloadError::Network(format!(
                "HTTP error: {}",
                status
            )));
        }

        // Get total size from Content-Length or Content-Range
        let total_bytes = if status == reqwest::StatusCode::PARTIAL_CONTENT {
            // Parse Content-Range: bytes start-end/total
            response
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split('/').nth(1))
                .and_then(|s| s.parse::<u64>().ok())
        } else if status.is_success() {
            response
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
        } else {
            None
        };

        // If we expected resume but got 200 OK, server doesn't support range requests
        if status == reqwest::StatusCode::OK && resume_from > 0 {
            tracing::warn!(
                "Server doesn't support resumable downloads for {}, restarting from beginning",
                identifier
            );
            storage.delete_partial(&identifier.normalized)?;
        }

        // Open partial file for writing (create or append)
        let partial_path = storage.get_partial_path(&identifier.normalized);
        let mut file = OpenOptions::new()
            .create(true)
            .append(status == reqwest::StatusCode::PARTIAL_CONTENT)
            .write(true)
            .truncate(status == reqwest::StatusCode::OK) // Truncate if restarting
            .open(&partial_path)?;

        // Stream response to file
        let mut stream = response.bytes_stream();
        let mut downloaded = if status == reqwest::StatusCode::PARTIAL_CONTENT {
            resume_from
        } else {
            0
        };

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if cancel_token.is_cancelled() {
                storage.delete_partial(&identifier.normalized)?;
                return Err(DownloadError::Cancelled);
            }

            let chunk = chunk_result?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            // Update progress
            tracker.update(downloaded, total_bytes).await;
            *progress.write().await = tracker.to_progress().await;
        }

        // Flush and sync
        file.flush()?;
        file.sync_all()?;
        drop(file);

        // Rename to final name
        let final_path = storage.finalize_download(&identifier.normalized)?;

        // Add to metadata
        storage
            .add_model_metadata(identifier, &final_path, None)
            .await?;

        Ok(LocalModelInfo {
            model_id: identifier.original.clone(),
            normalized_id: identifier.normalized.clone(),
            file_path: final_path.clone(),
            file_size: downloaded,
            downloaded_at: chrono::Utc::now(),
            checksum: None,
            metadata: ModelMetadata {
                model_id: identifier.original.clone(),
                organization: identifier.organization.clone(),
                model_name: identifier.model_name.clone(),
                variant: identifier.variant.clone(),
                file_name: "model.gguf".to_string(),
                file_size: downloaded,
                checksum: None,
                downloaded_at: chrono::Utc::now(),
            },
        })
    }

    pub async fn cancel_download(&self, model_id: &str) -> Result<(), DownloadError> {
        let identifier = ModelIdentifier::parse(model_id)
            .map_err(|e| DownloadError::InvalidModelId(e.to_string()))?;

        let task = self
            .active_downloads
            .write()
            .await
            .remove(&identifier.normalized)
            .ok_or_else(|| DownloadError::NotFound(model_id.to_string()))?;

        // Cancel the task
        task.cancel_token.cancel();

        // Wait for task to complete (with timeout)
        if let Some(handle) = task.handle.write().await.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
        }

        // Clean up partial file
        self.storage.delete_partial(&identifier.normalized)?;

        Ok(())
    }

    pub async fn get_progress(&self, model_id: &str) -> Result<DownloadProgress, DownloadError> {
        let identifier = ModelIdentifier::parse(model_id)
            .map_err(|e| DownloadError::InvalidModelId(e.to_string()))?;

        let task = {
            let downloads = self.active_downloads.read().await;
            downloads
                .get(&identifier.normalized)
                .cloned()
                .ok_or_else(|| DownloadError::NotFound(model_id.to_string()))?
        };

        let progress = task.progress.read().await.clone();
        Ok(progress)
    }

    pub async fn list_local_models(&self) -> Result<Vec<LocalModelInfo>, DownloadError> {
        Ok(self.storage.list_models().await)
    }

    pub async fn list_active_downloads(&self) -> Result<Vec<Arc<DownloadTask>>, DownloadError> {
        let downloads = self.active_downloads.read().await;
        Ok(downloads.values().cloned().collect())
    }

    pub async fn get_model_info(&self, model_id: &str) -> Result<ModelInfoResponse, DownloadError> {
        let identifier = ModelIdentifier::parse(model_id)
            .map_err(|e| DownloadError::InvalidModelId(e.to_string()))?;

        // Check if actively downloading
        let downloads = self.active_downloads.read().await;
        if let Some(task) = downloads.get(&identifier.normalized) {
            let status = task.status.read().await.clone();
            let progress = task.progress.read().await.clone();

            return Ok(ModelInfoResponse {
                model_id: task.model_id.clone(),
                normalized_id: task.normalized_id.clone(),
                status,
                file_size: progress.total_bytes,
                downloaded_at: None,
                progress: Some(progress),
            });
        }

        // Check if already downloaded
        if let Some(metadata) = self.storage.get_model_metadata(&identifier.normalized).await {
            return Ok(ModelInfoResponse {
                model_id: metadata.model_id,
                normalized_id: identifier.normalized,
                status: DownloadStatus::Completed,
                file_size: Some(metadata.file_size),
                downloaded_at: Some(metadata.downloaded_at),
                progress: None,
            });
        }

        Err(DownloadError::NotFound(model_id.to_string()))
    }

    pub async fn cleanup_completed(&self) {
        let mut downloads = self.active_downloads.write().await;
        downloads.retain(|_, task| {
            let status = task.status.try_read();
            match status {
                Ok(s) => !matches!(*s, DownloadStatus::Completed | DownloadStatus::Failed),
                Err(_) => true,
            }
        });
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfoResponse {
    pub model_id: String,
    pub normalized_id: String,
    pub status: DownloadStatus,
    pub file_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloaded_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<DownloadProgress>,
}
