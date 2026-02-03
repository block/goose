//! Status Line Module - Real-time feedback for agent operations
//!
//! Provides ephemeral status updates that show:
//! - Current operation (reading, writing, executing)
//! - Progress indicators
//! - File paths and tool names
//! - Elapsed time for long operations

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Status update types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatusType {
    Reading,
    Writing,
    Executing,
    Thinking,
    Validating,
    Searching,
    Compacting,
    Waiting,
    Complete,
    Error,
}

impl std::fmt::Display for StatusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusType::Reading => write!(f, "Reading"),
            StatusType::Writing => write!(f, "Writing"),
            StatusType::Executing => write!(f, "Executing"),
            StatusType::Thinking => write!(f, "Thinking"),
            StatusType::Validating => write!(f, "Validating"),
            StatusType::Searching => write!(f, "Searching"),
            StatusType::Compacting => write!(f, "Compacting"),
            StatusType::Waiting => write!(f, "Waiting"),
            StatusType::Complete => write!(f, "Complete"),
            StatusType::Error => write!(f, "Error"),
        }
    }
}

/// A status line update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub status_type: StatusType,
    pub message: String,
    pub detail: Option<String>,
    pub progress: Option<Progress>,
    #[serde(skip)]
    pub started_at: Option<Instant>,
}

impl StatusUpdate {
    pub fn new(status_type: StatusType, message: impl Into<String>) -> Self {
        Self {
            status_type,
            message: message.into(),
            detail: None,
            progress: None,
            started_at: Some(Instant::now()),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_progress(mut self, current: usize, total: usize) -> Self {
        self.progress = Some(Progress { current, total });
        self
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.started_at.map(|s| s.elapsed())
    }

    pub fn format(&self) -> String {
        let mut parts = vec![format!("{}", self.status_type)];

        parts.push(self.message.clone());

        if let Some(ref detail) = self.detail {
            parts.push(format!("({})", detail));
        }

        if let Some(ref progress) = self.progress {
            parts.push(format!("[{}/{}]", progress.current, progress.total));
        }

        if let Some(elapsed) = self.elapsed() {
            if elapsed.as_secs() > 0 {
                parts.push(format!("{}s", elapsed.as_secs()));
            }
        }

        parts.join(" ")
    }
}

/// Progress indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub current: usize,
    pub total: usize,
}

impl Progress {
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.current as f32 / self.total as f32) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }
}

/// Status line manager for tracking current operation
pub struct StatusLine {
    current: Arc<RwLock<Option<StatusUpdate>>>,
    history: Arc<RwLock<Vec<StatusUpdate>>>,
    max_history: usize,
    callbacks: Arc<RwLock<Vec<Box<dyn Fn(&StatusUpdate) + Send + Sync>>>>,
}

impl Default for StatusLine {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusLine {
    pub fn new() -> Self {
        Self {
            current: Arc::new(RwLock::new(None)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// Set the current status
    pub async fn set(&self, update: StatusUpdate) {
        // Archive previous status
        let mut current = self.current.write().await;
        if let Some(prev) = current.take() {
            let mut history = self.history.write().await;
            history.push(prev);
            if history.len() > self.max_history {
                history.remove(0);
            }
        }

        // Notify callbacks
        let callbacks = self.callbacks.read().await;
        for callback in callbacks.iter() {
            callback(&update);
        }

        *current = Some(update);
    }

    /// Clear the current status
    pub async fn clear(&self) {
        let mut current = self.current.write().await;
        if let Some(prev) = current.take() {
            let mut history = self.history.write().await;
            history.push(prev);
            if history.len() > self.max_history {
                history.remove(0);
            }
        }
    }

    /// Get the current status
    pub async fn get(&self) -> Option<StatusUpdate> {
        self.current.read().await.clone()
    }

    /// Get formatted current status
    pub async fn format(&self) -> Option<String> {
        self.current.read().await.as_ref().map(|u| u.format())
    }

    /// Register a callback for status updates
    pub async fn on_update<F>(&self, callback: F)
    where
        F: Fn(&StatusUpdate) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Convenience: Set reading status
    pub async fn reading(&self, path: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Reading, path)).await;
    }

    /// Convenience: Set writing status
    pub async fn writing(&self, path: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Writing, path)).await;
    }

    /// Convenience: Set executing status
    pub async fn executing(&self, command: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Executing, command))
            .await;
    }

    /// Convenience: Set thinking status
    pub async fn thinking(&self, message: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Thinking, message))
            .await;
    }

    /// Convenience: Set validating status
    pub async fn validating(&self, target: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Validating, target))
            .await;
    }

    /// Convenience: Set searching status
    pub async fn searching(&self, query: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Searching, query))
            .await;
    }

    /// Convenience: Set compacting status
    pub async fn compacting(&self) {
        self.set(StatusUpdate::new(StatusType::Compacting, "context"))
            .await;
    }

    /// Convenience: Set complete status
    pub async fn complete(&self, message: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Complete, message))
            .await;
    }

    /// Convenience: Set error status
    pub async fn error(&self, message: impl Into<String>) {
        self.set(StatusUpdate::new(StatusType::Error, message))
            .await;
    }

    /// Get history
    pub async fn get_history(&self) -> Vec<StatusUpdate> {
        self.history.read().await.clone()
    }
}

/// Status line for tool execution with progress
pub struct ToolExecutionStatus {
    status_line: Arc<StatusLine>,
    tool_name: String,
    started_at: Instant,
}

impl ToolExecutionStatus {
    pub fn new(status_line: Arc<StatusLine>, tool_name: impl Into<String>) -> Self {
        Self {
            status_line,
            tool_name: tool_name.into(),
            started_at: Instant::now(),
        }
    }

    pub async fn start(&self) {
        self.status_line.executing(&self.tool_name).await;
    }

    pub async fn update(&self, message: impl Into<String>) {
        let update = StatusUpdate::new(StatusType::Executing, &self.tool_name).with_detail(message);
        self.status_line.set(update).await;
    }

    pub async fn complete(&self) {
        let elapsed = self.started_at.elapsed();
        let message = format!("{} ({}ms)", self.tool_name, elapsed.as_millis());
        self.status_line.complete(message).await;
    }

    pub async fn fail(&self, error: impl Into<String>) {
        let message = format!("{}: {}", self.tool_name, error.into());
        self.status_line.error(message).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_update_format() {
        let update = StatusUpdate::new(StatusType::Reading, "file.rs");
        let formatted = update.format();
        assert!(formatted.contains("Reading"));
        assert!(formatted.contains("file.rs"));
    }

    #[test]
    fn test_status_update_with_detail() {
        let update =
            StatusUpdate::new(StatusType::Executing, "cargo build").with_detail("release mode");
        let formatted = update.format();
        assert!(formatted.contains("Executing"));
        assert!(formatted.contains("cargo build"));
        assert!(formatted.contains("release mode"));
    }

    #[test]
    fn test_progress() {
        let progress = Progress {
            current: 50,
            total: 100,
        };
        assert_eq!(progress.percentage(), 50.0);
        assert!(!progress.is_complete());

        let complete = Progress {
            current: 100,
            total: 100,
        };
        assert!(complete.is_complete());
    }

    #[tokio::test]
    async fn test_status_line_set_and_get() {
        let status_line = StatusLine::new();
        status_line.reading("test.rs").await;

        let current = status_line.get().await;
        assert!(current.is_some());
        assert_eq!(current.unwrap().status_type, StatusType::Reading);
    }

    #[tokio::test]
    async fn test_status_line_history() {
        let status_line = StatusLine::new();
        status_line.reading("file1.rs").await;
        status_line.writing("file2.rs").await;
        status_line.executing("cargo build").await;

        let history = status_line.get_history().await;
        assert_eq!(history.len(), 2);
    }

    #[tokio::test]
    async fn test_tool_execution_status() {
        let status_line = Arc::new(StatusLine::new());
        let tool_status = ToolExecutionStatus::new(status_line.clone(), "Bash");

        tool_status.start().await;
        let current = status_line.get().await;
        assert!(current.is_some());
        assert_eq!(current.unwrap().status_type, StatusType::Executing);

        tool_status.complete().await;
        let current = status_line.get().await;
        assert!(current.is_some());
        assert_eq!(current.unwrap().status_type, StatusType::Complete);
    }
}
