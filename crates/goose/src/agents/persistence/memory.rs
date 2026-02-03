//! In-memory checkpoint storage for testing and short-lived sessions

use super::{Checkpoint, CheckpointId, CheckpointSummary, Checkpointer, ThreadId};
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// In-memory checkpointer for testing and ephemeral sessions
///
/// All data is lost when the process exits. Use SqliteCheckpointer for persistence.
pub struct MemoryCheckpointer {
    /// Map of checkpoint_id -> Checkpoint
    checkpoints: RwLock<HashMap<CheckpointId, Checkpoint>>,
    /// Map of thread_id -> Vec<checkpoint_id> (ordered by creation time, newest last)
    thread_index: RwLock<HashMap<ThreadId, Vec<CheckpointId>>>,
}

impl MemoryCheckpointer {
    /// Create a new in-memory checkpointer
    pub fn new() -> Self {
        Self {
            checkpoints: RwLock::new(HashMap::new()),
            thread_index: RwLock::new(HashMap::new()),
        }
    }

    /// Get the total number of checkpoints stored
    pub async fn len(&self) -> usize {
        self.checkpoints.read().await.len()
    }

    /// Check if there are any checkpoints stored
    pub async fn is_empty(&self) -> bool {
        self.checkpoints.read().await.is_empty()
    }

    /// Clear all checkpoints
    pub async fn clear(&self) {
        self.checkpoints.write().await.clear();
        self.thread_index.write().await.clear();
    }
}

impl Default for MemoryCheckpointer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Checkpointer for MemoryCheckpointer {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let mut checkpoints = self.checkpoints.write().await;
        let mut index = self.thread_index.write().await;

        // Store checkpoint
        checkpoints.insert(checkpoint.checkpoint_id.clone(), checkpoint.clone());

        // Update thread index
        index
            .entry(checkpoint.thread_id.clone())
            .or_default()
            .push(checkpoint.checkpoint_id.clone());

        Ok(())
    }

    async fn load(&self, thread_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self.checkpoints.read().await;
        let index = self.thread_index.read().await;

        // Get the most recent checkpoint for this thread
        if let Some(checkpoint_ids) = index.get(thread_id) {
            if let Some(latest_id) = checkpoint_ids.last() {
                return Ok(checkpoints.get(latest_id).cloned());
            }
        }

        Ok(None)
    }

    async fn load_by_id(&self, checkpoint_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self.checkpoints.read().await;
        Ok(checkpoints.get(checkpoint_id).cloned())
    }

    async fn list(&self, thread_id: &str) -> Result<Vec<CheckpointSummary>> {
        let checkpoints = self.checkpoints.read().await;
        let index = self.thread_index.read().await;

        let mut summaries = Vec::new();

        if let Some(checkpoint_ids) = index.get(thread_id) {
            // Return in reverse order (most recent first)
            for id in checkpoint_ids.iter().rev() {
                if let Some(cp) = checkpoints.get(id) {
                    summaries.push(CheckpointSummary::from(cp));
                }
            }
        }

        Ok(summaries)
    }

    async fn delete(&self, checkpoint_id: &str) -> Result<bool> {
        let mut checkpoints = self.checkpoints.write().await;
        let mut index = self.thread_index.write().await;

        if let Some(cp) = checkpoints.remove(checkpoint_id) {
            // Remove from thread index
            if let Some(ids) = index.get_mut(&cp.thread_id) {
                ids.retain(|id| id != checkpoint_id);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn delete_thread(&self, thread_id: &str) -> Result<usize> {
        let mut checkpoints = self.checkpoints.write().await;
        let mut index = self.thread_index.write().await;

        let count = if let Some(ids) = index.remove(thread_id) {
            let count = ids.len();
            for id in ids {
                checkpoints.remove(&id);
            }
            count
        } else {
            0
        };

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::persistence::CheckpointMetadata;

    #[tokio::test]
    async fn test_memory_checkpointer_save_load() {
        let cp = MemoryCheckpointer::new();

        let checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": 1}));
        cp.save(&checkpoint).await.unwrap();

        let loaded = cp.load("thread-1").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().thread_id, "thread-1");
    }

    #[tokio::test]
    async fn test_memory_checkpointer_load_by_id() {
        let cp = MemoryCheckpointer::new();

        let checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": 1}));
        let id = checkpoint.checkpoint_id.clone();
        cp.save(&checkpoint).await.unwrap();

        let loaded = cp.load_by_id(&id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().checkpoint_id, id);
    }

    #[tokio::test]
    async fn test_memory_checkpointer_list() {
        let cp = MemoryCheckpointer::new();

        // Create multiple checkpoints
        for i in 0..5 {
            let mut checkpoint = Checkpoint::new("thread-1", serde_json::json!({ "count": i }));
            checkpoint.metadata = CheckpointMetadata::for_step(i, "Test");
            cp.save(&checkpoint).await.unwrap();
        }

        let list = cp.list("thread-1").await.unwrap();
        assert_eq!(list.len(), 5);

        // Should be most recent first
        assert_eq!(list[0].metadata.step, Some(4));
        assert_eq!(list[4].metadata.step, Some(0));
    }

    #[tokio::test]
    async fn test_memory_checkpointer_delete() {
        let cp = MemoryCheckpointer::new();

        let checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": 1}));
        let id = checkpoint.checkpoint_id.clone();
        cp.save(&checkpoint).await.unwrap();

        assert_eq!(cp.len().await, 1);

        let deleted = cp.delete(&id).await.unwrap();
        assert!(deleted);
        assert_eq!(cp.len().await, 0);
    }

    #[tokio::test]
    async fn test_memory_checkpointer_delete_thread() {
        let cp = MemoryCheckpointer::new();

        // Create checkpoints for two threads
        for i in 0..5 {
            cp.save(&Checkpoint::new(
                "thread-1",
                serde_json::json!({ "count": i }),
            ))
            .await
            .unwrap();
            cp.save(&Checkpoint::new(
                "thread-2",
                serde_json::json!({ "count": i }),
            ))
            .await
            .unwrap();
        }

        assert_eq!(cp.len().await, 10);

        let deleted = cp.delete_thread("thread-1").await.unwrap();
        assert_eq!(deleted, 5);
        assert_eq!(cp.len().await, 5);

        // thread-2 should still exist
        let list = cp.list("thread-2").await.unwrap();
        assert_eq!(list.len(), 5);
    }

    #[tokio::test]
    async fn test_memory_checkpointer_clear() {
        let cp = MemoryCheckpointer::new();

        for i in 0..5 {
            cp.save(&Checkpoint::new(
                "thread-1",
                serde_json::json!({ "count": i }),
            ))
            .await
            .unwrap();
        }

        assert_eq!(cp.len().await, 5);
        cp.clear().await;
        assert!(cp.is_empty().await);
    }

    #[tokio::test]
    async fn test_memory_checkpointer_parent_chain() {
        let cp = MemoryCheckpointer::new();

        // Create a chain of checkpoints
        let mut parent_id: Option<String> = None;
        for i in 0..3 {
            let mut checkpoint = Checkpoint::new("thread-1", serde_json::json!({ "count": i }));
            if let Some(pid) = parent_id {
                checkpoint = checkpoint.with_parent(pid);
            }
            parent_id = Some(checkpoint.checkpoint_id.clone());
            cp.save(&checkpoint).await.unwrap();
        }

        // Get history from the last checkpoint
        let history = cp.get_history(&parent_id.unwrap()).await.unwrap();
        assert_eq!(history.len(), 3);
    }
}
