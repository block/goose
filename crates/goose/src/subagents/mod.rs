//! Subagents Module - Task spawning and parallel execution
//!
//! Provides:
//! - Subagent spawning with isolated contexts
//! - Task queuing and parallel execution
//! - Result aggregation and reporting
//! - Hook integration for subagent lifecycle

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Subagent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentConfig {
    pub id: String,
    pub name: String,
    pub agent_type: SubagentType,
    pub instructions: String,
    pub max_turns: Option<usize>,
    pub timeout_secs: Option<u64>,
    pub inherit_context: bool,
    pub extensions: Vec<String>,
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Subagent".to_string(),
            agent_type: SubagentType::Task,
            instructions: String::new(),
            max_turns: Some(50),
            timeout_secs: Some(300),
            inherit_context: true,
            extensions: vec![],
        }
    }
}

impl SubagentConfig {
    pub fn new(instructions: impl Into<String>) -> Self {
        Self {
            instructions: instructions.into(),
            ..Default::default()
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_type(mut self, agent_type: SubagentType) -> Self {
        self.agent_type = agent_type;
        self
    }

    pub fn with_max_turns(mut self, turns: usize) -> Self {
        self.max_turns = Some(turns);
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }
}

/// Types of subagents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubagentType {
    Task,
    Research,
    Code,
    Test,
    Review,
    Deploy,
}

impl std::fmt::Display for SubagentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubagentType::Task => write!(f, "task"),
            SubagentType::Research => write!(f, "research"),
            SubagentType::Code => write!(f, "code"),
            SubagentType::Test => write!(f, "test"),
            SubagentType::Review => write!(f, "review"),
            SubagentType::Deploy => write!(f, "deploy"),
        }
    }
}

/// Status of a spawned subagent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubagentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

/// Result from a subagent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentResult {
    pub id: String,
    pub name: String,
    pub status: SubagentStatus,
    pub summary: String,
    pub artifacts: Vec<SubagentArtifact>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

impl SubagentResult {
    pub fn success(id: &str, name: &str, summary: String, duration_ms: u64) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            status: SubagentStatus::Completed,
            summary,
            artifacts: vec![],
            duration_ms,
            error: None,
        }
    }

    pub fn failure(id: &str, name: &str, error: String, duration_ms: u64) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            status: SubagentStatus::Failed,
            summary: String::new(),
            artifacts: vec![],
            duration_ms,
            error: Some(error),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == SubagentStatus::Completed
    }

    pub fn with_artifacts(mut self, artifacts: Vec<SubagentArtifact>) -> Self {
        self.artifacts = artifacts;
        self
    }
}

/// Artifact produced by a subagent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentArtifact {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub content: String,
}

/// Types of artifacts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactType {
    File,
    Code,
    Report,
    Data,
    Log,
}

/// A spawned subagent task
#[derive(Debug)]
pub struct SpawnedSubagent {
    pub config: SubagentConfig,
    pub status: SubagentStatus,
    pub result: Option<SubagentResult>,
    started_at: Option<std::time::Instant>,
}

impl SpawnedSubagent {
    pub fn new(config: SubagentConfig) -> Self {
        Self {
            config,
            status: SubagentStatus::Pending,
            result: None,
            started_at: None,
        }
    }

    pub fn start(&mut self) {
        self.status = SubagentStatus::Running;
        self.started_at = Some(std::time::Instant::now());
    }

    pub fn complete(&mut self, result: SubagentResult) {
        self.status = result.status;
        self.result = Some(result);
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started_at
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Manager for spawning and tracking subagents
pub struct SubagentSpawner {
    subagents: Arc<RwLock<HashMap<String, SpawnedSubagent>>>,
    max_concurrent: usize,
    running_count: Arc<RwLock<usize>>,
}

impl Default for SubagentSpawner {
    fn default() -> Self {
        Self::new(4)
    }
}

impl SubagentSpawner {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            subagents: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent,
            running_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Spawn a new subagent
    pub async fn spawn(&self, config: SubagentConfig) -> Result<String> {
        // Check concurrency limit
        {
            let count = self.running_count.read().await;
            if *count >= self.max_concurrent {
                return Err(anyhow::anyhow!(
                    "Maximum concurrent subagents ({}) reached",
                    self.max_concurrent
                ));
            }
        }

        let id = config.id.clone();
        let subagent = SpawnedSubagent::new(config);

        {
            let mut subagents = self.subagents.write().await;
            subagents.insert(id.clone(), subagent);
        }

        Ok(id)
    }

    /// Mark a subagent as running
    pub async fn start(&self, id: &str) -> Result<()> {
        let mut subagents = self.subagents.write().await;
        if let Some(subagent) = subagents.get_mut(id) {
            subagent.start();
            let mut count = self.running_count.write().await;
            *count += 1;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Subagent not found: {}", id))
        }
    }

    /// Complete a subagent with result
    pub async fn complete(&self, id: &str, result: SubagentResult) -> Result<()> {
        let mut subagents = self.subagents.write().await;
        if let Some(subagent) = subagents.get_mut(id) {
            subagent.complete(result);
            let mut count = self.running_count.write().await;
            if *count > 0 {
                *count -= 1;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Subagent not found: {}", id))
        }
    }

    /// Get subagent status
    pub async fn get_status(&self, id: &str) -> Option<SubagentStatus> {
        let subagents = self.subagents.read().await;
        subagents.get(id).map(|s| s.status)
    }

    /// Get subagent result
    pub async fn get_result(&self, id: &str) -> Option<SubagentResult> {
        let subagents = self.subagents.read().await;
        subagents.get(id).and_then(|s| s.result.clone())
    }

    /// Get all running subagents
    pub async fn get_running(&self) -> Vec<String> {
        let subagents = self.subagents.read().await;
        subagents
            .iter()
            .filter(|(_, s)| s.status == SubagentStatus::Running)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get summary of all subagents
    pub async fn get_summary(&self) -> SubagentSummary {
        let subagents = self.subagents.read().await;
        let mut summary = SubagentSummary::default();

        for subagent in subagents.values() {
            match subagent.status {
                SubagentStatus::Pending => summary.pending += 1,
                SubagentStatus::Running => summary.running += 1,
                SubagentStatus::Completed => summary.completed += 1,
                SubagentStatus::Failed => summary.failed += 1,
                SubagentStatus::Cancelled => summary.cancelled += 1,
                SubagentStatus::TimedOut => summary.timed_out += 1,
            }
        }

        summary.total = subagents.len();
        summary
    }

    /// Cancel a running subagent
    pub async fn cancel(&self, id: &str) -> Result<()> {
        let mut subagents = self.subagents.write().await;
        if let Some(subagent) = subagents.get_mut(id) {
            if subagent.status == SubagentStatus::Running {
                subagent.status = SubagentStatus::Cancelled;
                let mut count = self.running_count.write().await;
                if *count > 0 {
                    *count -= 1;
                }
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Subagent not found: {}", id))
        }
    }

    /// Cancel all running subagents
    pub async fn cancel_all(&self) {
        let mut subagents = self.subagents.write().await;
        for subagent in subagents.values_mut() {
            if subagent.status == SubagentStatus::Running {
                subagent.status = SubagentStatus::Cancelled;
            }
        }
        let mut count = self.running_count.write().await;
        *count = 0;
    }
}

/// Summary of subagent states
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubagentSummary {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub timed_out: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subagent_spawner() {
        let spawner = SubagentSpawner::new(2);

        let config = SubagentConfig::new("Test task");
        let id = spawner.spawn(config).await.unwrap();

        assert!(spawner.get_status(&id).await.is_some());
        assert_eq!(
            spawner.get_status(&id).await.unwrap(),
            SubagentStatus::Pending
        );
    }

    #[tokio::test]
    async fn test_subagent_lifecycle() {
        let spawner = SubagentSpawner::new(2);

        let config = SubagentConfig::new("Test task").with_name("TestAgent");
        let id = spawner.spawn(config).await.unwrap();

        spawner.start(&id).await.unwrap();
        assert_eq!(
            spawner.get_status(&id).await.unwrap(),
            SubagentStatus::Running
        );

        let result = SubagentResult::success(&id, "TestAgent", "Task completed".to_string(), 100);
        spawner.complete(&id, result).await.unwrap();

        assert_eq!(
            spawner.get_status(&id).await.unwrap(),
            SubagentStatus::Completed
        );
        assert!(spawner.get_result(&id).await.is_some());
    }

    #[tokio::test]
    async fn test_max_concurrent() {
        let spawner = SubagentSpawner::new(1);

        let config1 = SubagentConfig::new("Task 1");
        let id1 = spawner.spawn(config1).await.unwrap();
        spawner.start(&id1).await.unwrap();

        // Second spawn should fail due to limit
        let config2 = SubagentConfig::new("Task 2");
        let result = spawner.spawn(config2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subagent_summary() {
        let spawner = SubagentSpawner::new(4);

        let config1 = SubagentConfig::new("Task 1");
        let id1 = spawner.spawn(config1).await.unwrap();
        spawner.start(&id1).await.unwrap();

        let config2 = SubagentConfig::new("Task 2");
        let _id2 = spawner.spawn(config2).await.unwrap();

        let summary = spawner.get_summary().await;
        assert_eq!(summary.total, 2);
        assert_eq!(summary.running, 1);
        assert_eq!(summary.pending, 1);
    }
}
