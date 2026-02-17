use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;

use super::storage::LocalModelInfo;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Invalid model ID format: {0}")]
    InvalidModelId(String),

    #[error("Model already exists: {0}")]
    AlreadyExists(String),

    #[error("Model is already being downloaded: {0}")]
    AlreadyDownloading(String),

    #[error("Concurrent download limit exceeded (max: {0})")]
    ConcurrentLimitExceeded(usize),

    #[error("Download not found: {0}")]
    NotFound(String),

    #[error("Download was cancelled")]
    Cancelled,

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication required for private model")]
    AuthenticationRequired,

    #[error("Checksum verification failed")]
    ChecksumMismatch,

    #[error("Failed to parse model metadata: {0}")]
    MetadataError(String),

    #[error("API client error: {0}")]
    ApiClient(#[from] anyhow::Error),
}

// Convert reqwest errors to DownloadError
impl From<reqwest::Error> for DownloadError {
    fn from(err: reqwest::Error) -> Self {
        DownloadError::Network(err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_sec: f64,
    pub eta_seconds: Option<u64>,
    pub resumed_from_bytes: u64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_updated: DateTime<Utc>,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            downloaded_bytes: 0,
            total_bytes: None,
            speed_bytes_per_sec: 0.0,
            eta_seconds: None,
            resumed_from_bytes: 0,
            last_updated: Utc::now(),
        }
    }
}

impl DownloadProgress {
    pub fn percentage(&self) -> Option<f64> {
        self.total_bytes
            .map(|total| {
                if total > 0 {
                    (self.downloaded_bytes as f64 / total as f64) * 100.0
                } else {
                    0.0
                }
            })
    }
}

pub struct DownloadTask {
    pub model_id: String,
    pub normalized_id: String,
    pub status: Arc<RwLock<DownloadStatus>>,
    pub progress: Arc<RwLock<DownloadProgress>>,
    pub handle: Arc<RwLock<Option<JoinHandle<Result<LocalModelInfo, DownloadError>>>>>,
    pub cancel_token: CancellationToken,
    pub started_at: DateTime<Utc>,
}

impl DownloadTask {
    pub fn new(model_id: String, normalized_id: String) -> Self {
        Self {
            model_id,
            normalized_id,
            status: Arc::new(RwLock::new(DownloadStatus::Pending)),
            progress: Arc::new(RwLock::new(DownloadProgress::default())),
            handle: Arc::new(RwLock::new(None)),
            cancel_token: CancellationToken::new(),
            started_at: Utc::now(),
        }
    }

    pub async fn is_active(&self) -> bool {
        let status = self.status.read().await;
        matches!(
            *status,
            DownloadStatus::Pending | DownloadStatus::Downloading | DownloadStatus::Verifying
        )
    }

    pub async fn is_finished(&self) -> bool {
        let status = self.status.read().await;
        matches!(
            *status,
            DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_progress_percentage() {
        let mut progress = DownloadProgress::default();
        progress.downloaded_bytes = 50;
        progress.total_bytes = Some(100);
        assert_eq!(progress.percentage(), Some(50.0));

        progress.downloaded_bytes = 75;
        assert_eq!(progress.percentage(), Some(75.0));

        progress.total_bytes = None;
        assert_eq!(progress.percentage(), None);
    }

    #[test]
    fn test_download_progress_percentage_zero_total() {
        let mut progress = DownloadProgress::default();
        progress.total_bytes = Some(0);
        assert_eq!(progress.percentage(), Some(0.0));
    }

    #[tokio::test]
    async fn test_download_task_new() {
        let task = DownloadTask::new(
            "org/model".to_string(),
            "org_model".to_string(),
        );
        assert_eq!(task.model_id, "org/model");
        assert_eq!(task.normalized_id, "org_model");
        assert_eq!(*task.status.read().await, DownloadStatus::Pending);
        assert!(task.is_active().await);
        assert!(!task.is_finished().await);
    }

    #[tokio::test]
    async fn test_download_task_status_transitions() {
        let task = DownloadTask::new(
            "org/model".to_string(),
            "org_model".to_string(),
        );

        *task.status.write().await = DownloadStatus::Downloading;
        assert!(task.is_active().await);

        *task.status.write().await = DownloadStatus::Completed;
        assert!(!task.is_active().await);
        assert!(task.is_finished().await);
    }
}
