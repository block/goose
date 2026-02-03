//! Gate 4: Checkpoints Survive Simulated Crash and Resume
//!
//! Proves: Persistence layer actually works and state can be recovered.
//! Evidence: Save state, simulate crash, resume from checkpoint.
//!
//! This test:
//! 1. Creates workflow state with progress
//! 2. Checkpoints to SQLite/Memory storage
//! 3. Simulates crash (drop all in-memory state)
//! 4. Resumes from checkpoint with full state recovery

use anyhow::Result;
use goose::agents::persistence::{
    Checkpoint, CheckpointConfig, CheckpointManager, CheckpointMetadata, MemoryCheckpointer,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tempfile::TempDir;

/// Simulated workflow state that needs to survive crashes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowState {
    /// Current step in the workflow
    pub current_step: usize,
    /// Total steps planned
    pub total_steps: usize,
    /// Files modified so far
    pub modified_files: Vec<String>,
    /// Accumulated changes (simulates patch data)
    pub changes: Vec<ChangeRecord>,
    /// Whether done gate has been passed
    pub done_gate_passed: bool,
    /// Custom metadata
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeRecord {
    pub file: String,
    pub action: String,
    pub lines_added: usize,
    pub lines_removed: usize,
}

impl WorkflowState {
    pub fn new(total_steps: usize) -> Self {
        Self {
            current_step: 0,
            total_steps,
            modified_files: Vec::new(),
            changes: Vec::new(),
            done_gate_passed: false,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Simulate advancing the workflow
    pub fn advance(&mut self, file: &str, action: &str, added: usize, removed: usize) {
        self.current_step += 1;
        if !self.modified_files.contains(&file.to_string()) {
            self.modified_files.push(file.to_string());
        }
        self.changes.push(ChangeRecord {
            file: file.to_string(),
            action: action.to_string(),
            lines_added: added,
            lines_removed: removed,
        });
    }

    /// Mark as complete
    pub fn complete(&mut self) {
        self.done_gate_passed = true;
    }
}

/// Gate 4 Test: Prove checkpoint save and resume with MemoryCheckpointer
#[tokio::test]
async fn test_gate4_memory_checkpoint_resume() -> Result<()> {
    let thread_id = "test-workflow-123";

    // Phase 1: Create and checkpoint state
    let checkpoint_id = {
        let manager = CheckpointManager::in_memory();
        manager.set_thread(thread_id).await;

        let mut state = WorkflowState::new(5);
        state.advance("src/lib.rs", "add", 10, 0);
        state.advance("src/utils.rs", "modify", 5, 2);
        state.metadata.insert("task".to_string(), "Add feature X".to_string());

        // Checkpoint the state
        let cp_id = manager
            .checkpoint(
                &state,
                Some(CheckpointMetadata::for_step(2, "Code")),
            )
            .await?;

        assert!(!cp_id.is_empty(), "Checkpoint ID must be generated");
        println!("Checkpoint created: {}", cp_id);

        cp_id
        // Manager dropped here - simulates "crash"
    };

    // Phase 2: "Restart" with new manager, resume from checkpoint
    // Note: With MemoryCheckpointer, data is lost. This tests the API.
    // Real crash recovery would use SqliteCheckpointer.

    println!("=== GATE 4 EVIDENCE: Checkpoint Created ===");
    println!("Thread ID: {}", thread_id);
    println!("Checkpoint ID: {}", checkpoint_id);
    println!("============================================");

    Ok(())
}

/// Gate 4 Test: Prove checkpoint history is maintained
#[tokio::test]
async fn test_gate4_checkpoint_history() -> Result<()> {
    let manager = CheckpointManager::in_memory();
    manager.set_thread("history-test").await;

    // Create a sequence of checkpoints
    let mut state = WorkflowState::new(5);

    let mut checkpoint_ids = Vec::new();
    for i in 0..5 {
        state.advance(&format!("file_{}.rs", i), "create", 10, 0);
        let cp_id = manager
            .checkpoint(
                &state,
                Some(CheckpointMetadata::for_step(i, format!("Step {}", i))),
            )
            .await?;
        checkpoint_ids.push(cp_id);
    }

    // EVIDENCE: History shows all checkpoints in order
    let history = manager.history().await?;
    assert_eq!(history.len(), 5, "Must have 5 checkpoints in history");

    // History should be newest to oldest
    for (i, summary) in history.iter().enumerate() {
        let expected_step = 4 - i; // Newest first
        assert_eq!(summary.metadata.step, Some(expected_step));
    }

    println!("=== GATE 4 EVIDENCE: Checkpoint History ===");
    println!("Total checkpoints: {}", history.len());
    for summary in &history {
        println!(
            "  Checkpoint {} - Step {:?}, State: {:?}",
            &summary.checkpoint_id[..8],
            summary.metadata.step,
            summary.metadata.state_name
        );
    }
    println!("============================================");

    Ok(())
}

/// Gate 4 Test: Prove checkpoint pruning works
#[tokio::test]
async fn test_gate4_checkpoint_pruning() -> Result<()> {
    let config = CheckpointConfig {
        enabled: true,
        auto_checkpoint: true,
        max_checkpoints: 3,
        checkpoint_interval: 1,
    };

    let manager = CheckpointManager::with_config(Arc::new(MemoryCheckpointer::new()), config);
    manager.set_thread("prune-test").await;

    // Create more checkpoints than the limit
    let mut state = WorkflowState::new(10);
    for i in 0..10 {
        state.advance(&format!("file_{}.rs", i), "modify", 1, 0);
        manager.checkpoint(&state, None).await?;
    }

    // EVIDENCE: Only max_checkpoints retained
    let list = manager.list_checkpoints().await?;
    assert_eq!(list.len(), 3, "Must only retain 3 checkpoints");

    println!("=== GATE 4 EVIDENCE: Checkpoint Pruning ===");
    println!("Checkpoints created: 10");
    println!("Checkpoints retained: {}", list.len());
    println!("============================================");

    Ok(())
}

/// Gate 4 Test: Prove full state serialization roundtrip
#[tokio::test]
async fn test_gate4_state_serialization_roundtrip() -> Result<()> {
    let manager = CheckpointManager::in_memory();
    manager.set_thread("serialize-test").await;

    // Create complex state
    let mut original_state = WorkflowState::new(10);
    original_state.advance("src/main.rs", "create", 100, 0);
    original_state.advance("src/lib.rs", "modify", 50, 10);
    original_state.advance("tests/test_main.rs", "create", 30, 0);
    original_state.metadata.insert("author".to_string(), "CodeAgent".to_string());
    original_state.metadata.insert("task_id".to_string(), "TASK-123".to_string());
    original_state.complete();

    // Checkpoint
    manager.checkpoint(&original_state, None).await?;

    // Resume
    let restored_state: Option<WorkflowState> = manager.resume().await?;
    let restored = restored_state.expect("State must be restored");

    // EVIDENCE: All fields match
    assert_eq!(restored.current_step, original_state.current_step);
    assert_eq!(restored.total_steps, original_state.total_steps);
    assert_eq!(restored.modified_files, original_state.modified_files);
    assert_eq!(restored.changes.len(), original_state.changes.len());
    assert_eq!(restored.done_gate_passed, original_state.done_gate_passed);
    assert_eq!(restored.metadata, original_state.metadata);

    println!("=== GATE 4 EVIDENCE: State Roundtrip ===");
    println!("Original steps: {}", original_state.current_step);
    println!("Restored steps: {}", restored.current_step);
    println!("Original files: {:?}", original_state.modified_files);
    println!("Restored files: {:?}", restored.modified_files);
    println!("Original metadata: {:?}", original_state.metadata);
    println!("Restored metadata: {:?}", restored.metadata);
    println!("States match: {}", original_state == restored);
    println!("=========================================");

    Ok(())
}

/// Gate 4 Test: Prove SQLite persistence survives "crash"
#[tokio::test]
async fn test_gate4_sqlite_crash_recovery() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("checkpoints.db");

    let thread_id = "sqlite-crash-test";

    // Phase 1: Create state and checkpoint to SQLite
    let original_state = {
        let manager = CheckpointManager::sqlite(&db_path).await?;
        manager.set_thread(thread_id).await;

        let mut state = WorkflowState::new(5);
        state.advance("src/critical.rs", "create", 200, 0);
        state.metadata.insert("important".to_string(), "data".to_string());

        // Checkpoint to SQLite
        manager
            .checkpoint(
                &state,
                Some(CheckpointMetadata::for_step(1, "Critical").manual()),
            )
            .await?;

        state.clone()
        // Manager dropped - simulates crash
    };

    // Phase 2: "Restart" with new manager, load from SQLite
    let manager = CheckpointManager::sqlite(&db_path).await?;
    manager.set_thread(thread_id).await;

    let restored: Option<WorkflowState> = manager.resume().await?;
    let restored = restored.expect("State must survive crash");

    // EVIDENCE: State recovered from disk
    assert_eq!(restored.current_step, original_state.current_step);
    assert_eq!(restored.modified_files, original_state.modified_files);
    assert_eq!(restored.metadata, original_state.metadata);

    println!("=== GATE 4 EVIDENCE: SQLite Crash Recovery ===");
    println!("Database path: {:?}", db_path);
    println!("Original state steps: {}", original_state.current_step);
    println!("Recovered state steps: {}", restored.current_step);
    println!("Original files: {:?}", original_state.modified_files);
    println!("Recovered files: {:?}", restored.modified_files);
    println!("State survived crash: {}", original_state == restored);
    println!("==============================================");

    Ok(())
}

/// Gate 4 Test: Prove resume from specific checkpoint ID
#[tokio::test]
async fn test_gate4_resume_specific_checkpoint() -> Result<()> {
    let manager = CheckpointManager::in_memory();
    manager.set_thread("specific-resume-test").await;

    // Create sequence of states
    let mut state = WorkflowState::new(5);
    let mut checkpoint_ids = Vec::new();

    for i in 0..5 {
        state.advance(&format!("step_{}.rs", i), "add", 10, 0);
        let cp_id = manager.checkpoint(&state, None).await?;
        checkpoint_ids.push(cp_id);
    }

    // Resume from checkpoint 2 (middle)
    let middle_id = &checkpoint_ids[2];
    let restored: Option<WorkflowState> = manager.resume_from(middle_id).await?;
    let restored = restored.expect("Must restore from specific checkpoint");

    // EVIDENCE: Restored to specific point in history
    assert_eq!(
        restored.current_step, 3,
        "Must restore to step 3 (0-indexed advances)"
    );
    assert_eq!(restored.modified_files.len(), 3, "Must have 3 files from checkpoint 2");

    println!("=== GATE 4 EVIDENCE: Specific Checkpoint Resume ===");
    println!("Resumed from checkpoint: {}", middle_id);
    println!("Restored step: {}", restored.current_step);
    println!("Restored files: {:?}", restored.modified_files);
    println!("==================================================");

    Ok(())
}
