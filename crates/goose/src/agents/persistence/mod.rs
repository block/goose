//! Persistence module for LangGraph-style checkpointing and state management
//!
//! Provides:
//! - Checkpoint trait for pluggable storage backends
//! - MemoryCheckpointer for testing and short-lived sessions
//! - SqliteCheckpointer for production use with durable storage
//! - CheckpointMetadata for tracking checkpoint history

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod memory;
pub mod sqlite;

pub use memory::MemoryCheckpointer;
pub use sqlite::SqliteCheckpointer;

/// Unique identifier for a checkpoint
pub type CheckpointId = String;

/// Unique identifier for a thread/conversation
pub type ThreadId = String;

/// Checkpoint represents a saved state that can be resumed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier for this checkpoint
    pub checkpoint_id: CheckpointId,
    /// Thread/conversation this checkpoint belongs to
    pub thread_id: ThreadId,
    /// Parent checkpoint ID for branching history
    pub parent_id: Option<CheckpointId>,
    /// The serialized state data
    pub state: serde_json::Value,
    /// Checkpoint metadata
    pub metadata: CheckpointMetadata,
    /// When this checkpoint was created
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(thread_id: impl Into<String>, state: serde_json::Value) -> Self {
        let checkpoint_id = uuid::Uuid::new_v4().to_string();
        Self {
            checkpoint_id,
            thread_id: thread_id.into(),
            parent_id: None,
            state,
            metadata: CheckpointMetadata::default(),
            created_at: Utc::now(),
        }
    }

    /// Set the parent checkpoint ID
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Set custom metadata
    pub fn with_metadata(mut self, metadata: CheckpointMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a tag to this checkpoint
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    /// Set a custom label for this checkpoint
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.metadata.label = Some(label.into());
        self
    }
}

/// Metadata associated with a checkpoint
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Human-readable label for this checkpoint
    pub label: Option<String>,
    /// Tags for categorizing/filtering checkpoints
    pub tags: Vec<String>,
    /// Custom key-value metadata
    pub custom: HashMap<String, serde_json::Value>,
    /// Step number in the workflow when checkpoint was created
    pub step: Option<usize>,
    /// State name when checkpoint was created (e.g., "Code", "Test", "Fix")
    pub state_name: Option<String>,
    /// Number of iterations completed
    pub iteration: Option<usize>,
    /// Whether this is an automatic or manual checkpoint
    pub auto: bool,
}

impl CheckpointMetadata {
    /// Create metadata for a specific workflow step
    pub fn for_step(step: usize, state_name: impl Into<String>) -> Self {
        Self {
            step: Some(step),
            state_name: Some(state_name.into()),
            auto: true,
            ..Default::default()
        }
    }

    /// Mark as a manual checkpoint
    pub fn manual(mut self) -> Self {
        self.auto = false;
        self
    }
}

/// Summary information about a checkpoint (without full state data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSummary {
    pub checkpoint_id: CheckpointId,
    pub thread_id: ThreadId,
    pub parent_id: Option<CheckpointId>,
    pub metadata: CheckpointMetadata,
    pub created_at: DateTime<Utc>,
}

impl From<&Checkpoint> for CheckpointSummary {
    fn from(cp: &Checkpoint) -> Self {
        Self {
            checkpoint_id: cp.checkpoint_id.clone(),
            thread_id: cp.thread_id.clone(),
            parent_id: cp.parent_id.clone(),
            metadata: cp.metadata.clone(),
            created_at: cp.created_at,
        }
    }
}

/// Trait for checkpoint storage backends
///
/// Implementations must be thread-safe and async-compatible.
#[async_trait::async_trait]
pub trait Checkpointer: Send + Sync {
    /// Save a checkpoint
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()>;

    /// Load the most recent checkpoint for a thread
    async fn load(&self, thread_id: &str) -> Result<Option<Checkpoint>>;

    /// Load a specific checkpoint by ID
    async fn load_by_id(&self, checkpoint_id: &str) -> Result<Option<Checkpoint>>;

    /// List all checkpoints for a thread (most recent first)
    async fn list(&self, thread_id: &str) -> Result<Vec<CheckpointSummary>>;

    /// Delete a specific checkpoint
    async fn delete(&self, checkpoint_id: &str) -> Result<bool>;

    /// Delete all checkpoints for a thread
    async fn delete_thread(&self, thread_id: &str) -> Result<usize>;

    /// Get the checkpoint history (ancestors) for a checkpoint
    async fn get_history(&self, checkpoint_id: &str) -> Result<Vec<CheckpointSummary>> {
        let mut history = Vec::new();
        let mut current_id = Some(checkpoint_id.to_string());

        while let Some(id) = current_id {
            if let Some(cp) = self.load_by_id(&id).await? {
                let summary = CheckpointSummary::from(&cp);
                current_id = cp.parent_id;
                history.push(summary);
            } else {
                break;
            }
        }

        Ok(history)
    }
}

/// Configuration for checkpointing behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Whether checkpointing is enabled
    pub enabled: bool,
    /// Automatically checkpoint on state transitions
    pub auto_checkpoint: bool,
    /// Maximum number of checkpoints to retain per thread (0 = unlimited)
    pub max_checkpoints: usize,
    /// Checkpoint on every N iterations (0 = disabled)
    pub checkpoint_interval: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_checkpoint: true,
            max_checkpoints: 100,
            checkpoint_interval: 1,
        }
    }
}

/// Manager for handling checkpoint operations with the StateGraph
pub struct CheckpointManager {
    checkpointer: Arc<dyn Checkpointer>,
    config: CheckpointConfig,
    current_thread: RwLock<Option<ThreadId>>,
    last_checkpoint: RwLock<Option<CheckpointId>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager with the given backend
    pub fn new(checkpointer: Arc<dyn Checkpointer>) -> Self {
        Self {
            checkpointer,
            config: CheckpointConfig::default(),
            current_thread: RwLock::new(None),
            last_checkpoint: RwLock::new(None),
        }
    }

    /// Create with custom configuration
    pub fn with_config(checkpointer: Arc<dyn Checkpointer>, config: CheckpointConfig) -> Self {
        Self {
            checkpointer,
            config,
            current_thread: RwLock::new(None),
            last_checkpoint: RwLock::new(None),
        }
    }

    /// Create a manager with in-memory storage (for testing)
    pub fn in_memory() -> Self {
        Self::new(Arc::new(MemoryCheckpointer::new()))
    }

    /// Create a manager with SQLite storage
    pub async fn sqlite(path: impl AsRef<Path>) -> Result<Self> {
        let checkpointer = SqliteCheckpointer::new(path).await?;
        Ok(Self::new(Arc::new(checkpointer)))
    }

    /// Set the current thread ID
    pub async fn set_thread(&self, thread_id: impl Into<String>) {
        let mut current = self.current_thread.write().await;
        *current = Some(thread_id.into());
    }

    /// Get the current thread ID
    pub async fn thread_id(&self) -> Option<ThreadId> {
        self.current_thread.read().await.clone()
    }

    /// Create a checkpoint for the current state
    pub async fn checkpoint<S: Serialize>(
        &self,
        state: &S,
        metadata: Option<CheckpointMetadata>,
    ) -> Result<CheckpointId> {
        if !self.config.enabled {
            anyhow::bail!("Checkpointing is disabled");
        }

        let thread_id = self
            .current_thread
            .read()
            .await
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let state_json = serde_json::to_value(state)?;
        let parent_id = self.last_checkpoint.read().await.clone();

        let mut checkpoint = Checkpoint::new(&thread_id, state_json);
        if let Some(parent) = parent_id {
            checkpoint = checkpoint.with_parent(parent);
        }
        if let Some(meta) = metadata {
            checkpoint = checkpoint.with_metadata(meta);
        }

        let checkpoint_id = checkpoint.checkpoint_id.clone();
        self.checkpointer.save(&checkpoint).await?;

        // Update last checkpoint
        let mut last = self.last_checkpoint.write().await;
        *last = Some(checkpoint_id.clone());

        // Prune old checkpoints if needed
        if self.config.max_checkpoints > 0 {
            self.prune_old_checkpoints(&thread_id).await?;
        }

        Ok(checkpoint_id)
    }

    /// Resume from the most recent checkpoint for the current thread
    pub async fn resume<S: for<'de> Deserialize<'de>>(&self) -> Result<Option<S>> {
        let thread_id = match self.current_thread.read().await.clone() {
            Some(id) => id,
            None => return Ok(None),
        };

        if let Some(checkpoint) = self.checkpointer.load(&thread_id).await? {
            let state: S = serde_json::from_value(checkpoint.state)?;
            let mut last = self.last_checkpoint.write().await;
            *last = Some(checkpoint.checkpoint_id);
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Resume from a specific checkpoint
    pub async fn resume_from<S: for<'de> Deserialize<'de>>(
        &self,
        checkpoint_id: &str,
    ) -> Result<Option<S>> {
        if let Some(checkpoint) = self.checkpointer.load_by_id(checkpoint_id).await? {
            let state: S = serde_json::from_value(checkpoint.state)?;

            // Update current thread
            let mut current = self.current_thread.write().await;
            *current = Some(checkpoint.thread_id);

            // Update last checkpoint
            let mut last = self.last_checkpoint.write().await;
            *last = Some(checkpoint.checkpoint_id);

            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// List all checkpoints for the current thread
    pub async fn list_checkpoints(&self) -> Result<Vec<CheckpointSummary>> {
        let thread_id = match self.current_thread.read().await.clone() {
            Some(id) => id,
            None => return Ok(vec![]),
        };
        self.checkpointer.list(&thread_id).await
    }

    /// Get the checkpoint history leading to the current checkpoint
    pub async fn history(&self) -> Result<Vec<CheckpointSummary>> {
        let checkpoint_id = match self.last_checkpoint.read().await.clone() {
            Some(id) => id,
            None => return Ok(vec![]),
        };
        self.checkpointer.get_history(&checkpoint_id).await
    }

    /// Prune old checkpoints to stay within the configured limit
    async fn prune_old_checkpoints(&self, thread_id: &str) -> Result<()> {
        let summaries = self.checkpointer.list(thread_id).await?;

        if summaries.len() > self.config.max_checkpoints {
            // Delete oldest checkpoints (list is most recent first)
            for summary in summaries.iter().skip(self.config.max_checkpoints) {
                self.checkpointer.delete(&summary.checkpoint_id).await?;
            }
        }

        Ok(())
    }

    /// Get the underlying checkpointer
    pub fn checkpointer(&self) -> &Arc<dyn Checkpointer> {
        &self.checkpointer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestState {
        counter: i32,
        message: String,
    }

    #[tokio::test]
    async fn test_checkpoint_creation() {
        let state = serde_json::json!({"counter": 1, "message": "test"});
        let checkpoint = Checkpoint::new("thread-1", state.clone())
            .with_label("Initial state")
            .with_tag("test");

        assert_eq!(checkpoint.thread_id, "thread-1");
        assert!(checkpoint.parent_id.is_none());
        assert_eq!(checkpoint.metadata.label, Some("Initial state".to_string()));
        assert!(checkpoint.metadata.tags.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_checkpoint_manager_basic() {
        let manager = CheckpointManager::in_memory();
        manager.set_thread("test-thread").await;

        let state = TestState {
            counter: 42,
            message: "Hello".to_string(),
        };

        // Create checkpoint
        let cp_id = manager.checkpoint(&state, None).await.unwrap();
        assert!(!cp_id.is_empty());

        // Resume
        let restored: Option<TestState> = manager.resume().await.unwrap();
        assert_eq!(restored, Some(state));
    }

    #[tokio::test]
    async fn test_checkpoint_history() {
        let manager = CheckpointManager::in_memory();
        manager.set_thread("test-thread").await;

        // Create chain of checkpoints
        for i in 0..5 {
            let state = TestState {
                counter: i,
                message: format!("Step {}", i),
            };
            manager.checkpoint(&state, None).await.unwrap();
        }

        // Check history
        let history = manager.history().await.unwrap();
        assert_eq!(history.len(), 5);
    }

    #[tokio::test]
    async fn test_checkpoint_pruning() {
        let config = CheckpointConfig {
            enabled: true,
            auto_checkpoint: true,
            max_checkpoints: 3,
            checkpoint_interval: 1,
        };
        let manager = CheckpointManager::with_config(Arc::new(MemoryCheckpointer::new()), config);
        manager.set_thread("test-thread").await;

        // Create more checkpoints than the limit
        for i in 0..10 {
            let state = TestState {
                counter: i,
                message: format!("Step {}", i),
            };
            manager.checkpoint(&state, None).await.unwrap();
        }

        // Should only have 3 checkpoints
        let list = manager.list_checkpoints().await.unwrap();
        assert_eq!(list.len(), 3);
    }
}
