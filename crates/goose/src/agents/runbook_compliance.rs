// Phase 7 Extension: Runbook Compliance System
// Ensures agents execute documentation as binding contracts, not suggestions

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use tokio::fs;

/// A parsed step from a runbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookStep {
    pub number: usize,
    pub description: String,
    pub command: Option<String>,
    pub expected_result: String,
    pub failure_action: String,
    pub preconditions: Vec<String>,
}

/// Success criteria for runbook completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    pub description: String,
    pub check_command: Option<String>,
    pub expected_output: Option<String>,
    pub manual_verification: bool,
}

/// Current execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub current_step: usize,
    pub total_steps: usize,
    pub started_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    pub status: ExecutionStatus,
    pub failures: Vec<StepFailure>,
    pub completed_steps: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    NotStarted,
    InProgress,
    Blocked,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepFailure {
    pub step: usize,
    pub error: String,
    pub retry_count: usize,
    pub timestamp: DateTime<Utc>,
}

/// Main runbook compliance enforcer
pub struct RunbookCompliance {
    runbook_path: PathBuf,
    success_path: PathBuf,
    progress_path: PathBuf,
    state_path: PathBuf,
    log_path: PathBuf,
    max_retries: usize,
}

impl RunbookCompliance {
    pub fn new(work_dir: &Path) -> Self {
        let docs = work_dir.join("docs");
        let artifacts = work_dir.join("artifacts");

        Self {
            runbook_path: docs.join("RUNBOOK.md"),
            success_path: docs.join("SUCCESS.md"),
            progress_path: docs.join("PROGRESS.md"),
            state_path: artifacts.join("run-state.json"),
            log_path: artifacts.join("run-log.txt"),
            max_retries: 3,
        }
    }

    /// Parse RUNBOOK.md into executable steps
    pub async fn parse_runbook(&self) -> Result<Vec<RunbookStep>> {
        let content = fs::read_to_string(&self.runbook_path)
            .await
            .context("Failed to read RUNBOOK.md")?;

        let mut steps = Vec::new();
        let mut current_step: Option<RunbookStep> = None;
        let mut step_number = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Detect step headers (numbered lines)
            if let Some(desc) = trimmed.strip_prefix(|c: char| c.is_numeric()) {
                if let Some(desc) = desc.strip_prefix('.').or_else(|| desc.strip_prefix(')')) {
                    // Save previous step
                    if let Some(step) = current_step.take() {
                        steps.push(step);
                    }

                    step_number += 1;
                    current_step = Some(RunbookStep {
                        number: step_number,
                        description: desc.trim().to_string(),
                        command: None,
                        expected_result: String::new(),
                        failure_action: "Stop and report error".to_string(),
                        preconditions: Vec::new(),
                    });
                }
            } else if let Some(step) = current_step.as_mut() {
                // Parse step details
                if trimmed.starts_with("Command:") || trimmed.starts_with("```bash") {
                    if let Some(cmd_line) = trimmed.strip_prefix("Command:") {
                        step.command = Some(cmd_line.trim().to_string());
                    }
                } else if trimmed.starts_with("Expected:") {
                    step.expected_result =
                        trimmed.strip_prefix("Expected:").unwrap().trim().to_string();
                } else if trimmed.starts_with("If fails:") {
                    step.failure_action =
                        trimmed.strip_prefix("If fails:").unwrap().trim().to_string();
                } else if trimmed.starts_with("Requires:") {
                    let req = trimmed.strip_prefix("Requires:").unwrap().trim().to_string();
                    step.preconditions.push(req);
                } else if !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && step.command.is_none()
                {
                    // If we see code block content after ```bash
                    if !trimmed.starts_with("```") && !trimmed.starts_with("##") {
                        step.command = Some(trimmed.to_string());
                    }
                }
            }
        }

        // Don't forget the last step
        if let Some(step) = current_step {
            steps.push(step);
        }

        Ok(steps)
    }

    /// Parse SUCCESS.md criteria
    pub async fn parse_success_criteria(&self) -> Result<Vec<SuccessCriteria>> {
        let content = fs::read_to_string(&self.success_path)
            .await
            .context("Failed to read SUCCESS.md")?;

        let mut criteria = Vec::new();
        let mut current: Option<SuccessCriteria> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with('-') || trimmed.starts_with('*') {
                // Save previous criterion
                if let Some(crit) = current.take() {
                    criteria.push(crit);
                }

                let desc = trimmed.trim_start_matches('-').trim_start_matches('*').trim();
                current = Some(SuccessCriteria {
                    description: desc.to_string(),
                    check_command: None,
                    expected_output: None,
                    manual_verification: false,
                });
            } else if let Some(crit) = current.as_mut() {
                if trimmed.starts_with("Check:") {
                    crit.check_command =
                        Some(trimmed.strip_prefix("Check:").unwrap().trim().to_string());
                } else if trimmed.starts_with("Expect:") {
                    crit.expected_output =
                        Some(trimmed.strip_prefix("Expect:").unwrap().trim().to_string());
                } else if trimmed.contains("manual") || trimmed.contains("verify manually") {
                    crit.manual_verification = true;
                }
            }
        }

        if let Some(crit) = current {
            criteria.push(crit);
        }

        Ok(criteria)
    }

    /// Load or initialize execution state
    pub async fn load_state(&self) -> Result<ExecutionState> {
        if self.state_path.exists() {
            let content = fs::read_to_string(&self.state_path).await?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(ExecutionState {
                current_step: 0,
                total_steps: 0,
                started_at: Utc::now(),
                last_update: Utc::now(),
                status: ExecutionStatus::NotStarted,
                failures: Vec::new(),
                completed_steps: Vec::new(),
            })
        }
    }

    /// Save execution state
    pub async fn save_state(&self, state: &ExecutionState) -> Result<()> {
        // Ensure artifacts dir exists
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(state)?;
        fs::write(&self.state_path, json).await?;
        Ok(())
    }

    /// Append to execution log
    pub async fn log(&self, message: &str) -> Result<()> {
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let timestamp = Utc::now().to_rfc3339();
        let entry = format!("[{}] {}\n", timestamp, message);

        // Append to log
        if self.log_path.exists() {
            let mut existing = fs::read_to_string(&self.log_path).await?;
            existing.push_str(&entry);
            fs::write(&self.log_path, existing).await?;
        } else {
            fs::write(&self.log_path, entry).await?;
        }

        Ok(())
    }

    /// Update PROGRESS.md with current status
    pub async fn update_progress(&self, state: &ExecutionState, steps: &[RunbookStep]) -> Result<()> {
        let mut content = String::new();
        content.push_str("# Execution Progress\n\n");
        content.push_str(&format!("Status: {:?}\n", state.status));
        content.push_str(&format!("Started: {}\n", state.started_at.format("%Y-%m-%d %H:%M:%S")));
        content.push_str(&format!("Last Update: {}\n\n", state.last_update.format("%Y-%m-%d %H:%M:%S")));
        content.push_str(&format!("Progress: {}/{} steps completed\n\n", state.completed_steps.len(), state.total_steps));

        content.push_str("## Steps\n\n");
        for step in steps {
            let status = if state.completed_steps.contains(&step.number) {
                "✅"
            } else if step.number == state.current_step {
                "🔄"
            } else {
                "⏳"
            };

            content.push_str(&format!("{}. {} {}\n", step.number, status, step.description));
        }

        if !state.failures.is_empty() {
            content.push_str("\n## Failures\n\n");
            for failure in &state.failures {
                content.push_str(&format!(
                    "- Step {}: {} (retries: {})\n",
                    failure.step, failure.error, failure.retry_count
                ));
            }
        }

        if let Some(parent) = self.progress_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&self.progress_path, content).await?;
        Ok(())
    }

    /// Check if runbook exists and is non-empty
    pub async fn has_runbook(&self) -> bool {
        if !self.runbook_path.exists() {
            return false;
        }

        if let Ok(content) = fs::read_to_string(&self.runbook_path).await {
            !content.trim().is_empty()
        } else {
            false
        }
    }

    /// Check if all success criteria are met
    pub async fn verify_success(&self) -> Result<bool> {
        let criteria = self.parse_success_criteria().await?;

        for crit in criteria {
            if crit.manual_verification {
                self.log(&format!(
                    "Manual verification required: {}",
                    crit.description
                ))
                .await?;
                return Ok(false);
            }

            if let Some(check_cmd) = crit.check_command {
                // This would need integration with the shell executor
                self.log(&format!(
                    "Need to verify: {} with command: {}",
                    crit.description, check_cmd
                ))
                .await?;
                // For now, mark as needing verification
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get retry count for a step
    pub fn get_retry_count(&self, state: &ExecutionState, step_num: usize) -> usize {
        state
            .failures
            .iter()
            .filter(|f| f.step == step_num)
            .map(|f| f.retry_count)
            .max()
            .unwrap_or(0)
    }

    /// Record a step failure
    pub fn record_failure(
        &self,
        state: &mut ExecutionState,
        step_num: usize,
        error: String,
    ) {
        let retry_count = self.get_retry_count(state, step_num) + 1;
        state.failures.push(StepFailure {
            step: step_num,
            error,
            retry_count,
            timestamp: Utc::now(),
        });
    }

    /// Mark a step as completed
    pub fn complete_step(&self, state: &mut ExecutionState, step_num: usize) {
        if !state.completed_steps.contains(&step_num) {
            state.completed_steps.push(step_num);
        }
        state.last_update = Utc::now();
    }

    /// Check if should retry a failed step
    pub fn should_retry(&self, state: &ExecutionState, step_num: usize) -> bool {
        self.get_retry_count(state, step_num) < self.max_retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parse_simple_runbook() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir_all(&docs).await.unwrap();

        let runbook_content = r#"
# Setup

1. Install dependencies
   Command: cargo build
   Expected: Successfully compiled
   If fails: Check Rust version

2. Run tests
   Command: cargo test
   Expected: All tests pass
"#;

        fs::write(docs.join("RUNBOOK.md"), runbook_content)
            .await
            .unwrap();

        let compliance = RunbookCompliance::new(temp.path());
        let steps = compliance.parse_runbook().await.unwrap();

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].number, 1);
        assert_eq!(steps[0].description, "Install dependencies");
        assert_eq!(steps[0].command, Some("cargo build".to_string()));
    }

    #[tokio::test]
    async fn test_state_persistence() {
        let temp = TempDir::new().unwrap();
        let compliance = RunbookCompliance::new(temp.path());

        let state = ExecutionState {
            current_step: 1,
            total_steps: 5,
            started_at: Utc::now(),
            last_update: Utc::now(),
            status: ExecutionStatus::InProgress,
            failures: Vec::new(),
            completed_steps: vec![],
        };

        compliance.save_state(&state).await.unwrap();
        let loaded = compliance.load_state().await.unwrap();

        assert_eq!(loaded.current_step, 1);
        assert_eq!(loaded.total_steps, 5);
        assert_eq!(loaded.status, ExecutionStatus::InProgress);
    }
}
