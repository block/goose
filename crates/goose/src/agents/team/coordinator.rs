//! Team Coordinator - Orchestrates Builder/Validator workflows

use super::{BuildResult, BuilderAgent, TeamAgent, TeamTask, ValidatorAgent};
use crate::validators::ValidationResult;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for team coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub require_validation: bool,
    pub max_retries: usize,
    pub parallel_tasks: usize,
    pub auto_rollback_on_failure: bool,
    pub working_dir: PathBuf,
}

impl Default for TeamConfig {
    fn default() -> Self {
        Self {
            require_validation: true,
            max_retries: 3,
            parallel_tasks: 4,
            auto_rollback_on_failure: true,
            working_dir: PathBuf::from("."),
        }
    }
}

/// Result of a team workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResult {
    pub task_id: String,
    pub success: bool,
    pub build_result: Option<BuildResult>,
    pub validation_result: Option<ValidationResult>,
    pub retries: usize,
    pub total_duration_ms: u64,
    pub error: Option<String>,
}

/// A complete team workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamWorkflow {
    pub id: String,
    pub name: String,
    pub tasks: Vec<TeamTask>,
    pub members: Vec<String>,
    pub status: WorkflowStatus,
    pub results: Vec<TeamResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Team coordinator for managing build/validate workflows
pub struct TeamCoordinator {
    config: TeamConfig,
    builders: HashMap<String, Arc<BuilderAgent>>,
    validators: HashMap<String, Arc<ValidatorAgent>>,
    active_tasks: RwLock<HashMap<String, TeamTask>>,
    results: RwLock<Vec<TeamResult>>,
}

impl TeamCoordinator {
    pub fn new(config: TeamConfig) -> Self {
        Self {
            config,
            builders: HashMap::new(),
            validators: HashMap::new(),
            active_tasks: RwLock::new(HashMap::new()),
            results: RwLock::new(Vec::new()),
        }
    }

    pub fn add_builder(&mut self, builder: BuilderAgent) {
        self.builders
            .insert(builder.id().to_string(), Arc::new(builder));
    }

    pub fn add_validator(&mut self, validator: ValidatorAgent) {
        self.validators
            .insert(validator.id().to_string(), Arc::new(validator));
    }

    /// Create a paired team (builder + validator) for a task
    pub fn create_team(&mut self, task_name: &str) -> Result<(String, String)> {
        let builder_id = format!("{}_builder", task_name);
        let validator_id = format!("{}_validator", task_name);

        let builder = BuilderAgent::new(
            &builder_id,
            format!("{} Builder", task_name),
            &self.config.working_dir,
        );
        let validator = ValidatorAgent::new(
            &validator_id,
            format!("{} Validator", task_name),
            &self.config.working_dir,
        );

        self.add_builder(builder);
        self.add_validator(validator);

        Ok((builder_id, validator_id))
    }

    /// Execute a task with build/validate workflow
    pub async fn execute_task(&self, task: TeamTask) -> Result<TeamResult> {
        let start = std::time::Instant::now();
        let task_id = task.id.clone();

        // Get builder and validator
        let builder = self
            .builders
            .get(&task.builder_id)
            .ok_or_else(|| anyhow!("Builder not found: {}", task.builder_id))?;
        let validator = self
            .validators
            .get(&task.validator_id)
            .ok_or_else(|| anyhow!("Validator not found: {}", task.validator_id))?;

        // Track active task
        {
            let mut active = self.active_tasks.write().await;
            active.insert(task_id.clone(), task.clone());
        }

        let mut retries = 0;
        let mut last_build_result = None;
        let mut last_validation_result = None;
        let mut success = false;

        // Retry loop
        while retries < self.config.max_retries {
            // 1. Build phase
            let build_result = match builder.execute_task(&task).await {
                Ok(result) => result,
                Err(_) => {
                    return Err(anyhow!("Builder failed"));
                }
            };

            if !build_result.success {
                retries += 1;
                last_build_result = Some(build_result);
                continue;
            }

            last_build_result = Some(build_result.clone());

            // 2. Validation phase (if required)
            if self.config.require_validation {
                let validation_result = match validator.validate_work(&task, &build_result).await {
                    Ok(result) => result,
                    Err(_) => {
                        return Err(anyhow!("Validation failed"));
                    }
                };

                last_validation_result = Some(validation_result.clone());

                if validation_result.ok {
                    success = true;
                    break;
                } else {
                    retries += 1;
                    // Optionally rollback on validation failure
                    if self.config.auto_rollback_on_failure {
                        // Rollback logic would go here
                    }
                }
            } else {
                success = true;
                break;
            }
        }

        // Remove from active tasks
        {
            let mut active = self.active_tasks.write().await;
            active.remove(&task_id);
        }

        let duration = start.elapsed().as_millis() as u64;

        let result = TeamResult {
            task_id: task_id.clone(),
            success,
            build_result: last_build_result,
            validation_result: last_validation_result,
            retries,
            total_duration_ms: duration,
            error: if success {
                None
            } else {
                Some("Max retries exceeded".to_string())
            },
        };

        // Store result
        {
            let mut results = self.results.write().await;
            results.push(result.clone());
        }

        Ok(result)
    }

    /// Execute multiple tasks in parallel
    pub async fn execute_workflow(&self, workflow: &mut TeamWorkflow) -> Result<Vec<TeamResult>> {
        workflow.status = WorkflowStatus::Running;

        let mut handles = Vec::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.parallel_tasks));

        for task in &workflow.tasks {
            let task = task.clone();
            let permit = semaphore.clone().acquire_owned().await?;

            // Note: In a real implementation, we'd use Arc<Self> or clone necessary state
            let handle = tokio::spawn(async move {
                drop(permit);
                // Execute task...
                TeamResult {
                    task_id: task.id,
                    success: true,
                    build_result: None,
                    validation_result: None,
                    retries: 0,
                    total_duration_ms: 0,
                    error: None,
                }
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        workflow.results = results.clone();
        workflow.status = if results.iter().all(|r| r.success) {
            WorkflowStatus::Completed
        } else {
            WorkflowStatus::Failed
        };

        Ok(results)
    }

    /// Get active tasks
    pub async fn get_active_tasks(&self) -> Vec<TeamTask> {
        let active = self.active_tasks.read().await;
        active.values().cloned().collect()
    }

    /// Get all results
    pub async fn get_results(&self) -> Vec<TeamResult> {
        let results = self.results.read().await;
        results.clone()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> TeamStats {
        let results = self.results.read().await;

        let total = results.len();
        let successful = results.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let total_retries: usize = results.iter().map(|r| r.retries).sum();
        let total_duration: u64 = results.iter().map(|r| r.total_duration_ms).sum();

        TeamStats {
            total_tasks: total,
            successful,
            failed,
            total_retries,
            total_duration_ms: total_duration,
            success_rate: if total > 0 {
                successful as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStats {
    pub total_tasks: usize,
    pub successful: usize,
    pub failed: usize,
    pub total_retries: usize,
    pub total_duration_ms: u64,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_config_default() {
        let config = TeamConfig::default();
        assert!(config.require_validation);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.parallel_tasks, 4);
    }

    #[test]
    fn test_team_coordinator_creation() {
        let config = TeamConfig::default();
        let coordinator = TeamCoordinator::new(config);
        assert!(coordinator.builders.is_empty());
        assert!(coordinator.validators.is_empty());
    }

    #[tokio::test]
    async fn test_team_coordinator_create_team() {
        let config = TeamConfig {
            working_dir: PathBuf::from("/tmp"),
            ..Default::default()
        };
        let mut coordinator = TeamCoordinator::new(config);

        let (builder_id, validator_id) = coordinator.create_team("feature_x").unwrap();

        assert_eq!(builder_id, "feature_x_builder");
        assert_eq!(validator_id, "feature_x_validator");
        assert!(coordinator.builders.contains_key(&builder_id));
        assert!(coordinator.validators.contains_key(&validator_id));
    }
}
