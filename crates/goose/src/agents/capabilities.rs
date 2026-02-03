//! Agent Capabilities - Integration of Phase 7 modules with core Agent
//!
//! Provides a unified interface for accessing:
//! - Hook system (lifecycle events)
//! - Task management (DAG execution)
//! - Validators (code quality checks)
//! - Tool search (dynamic discovery)
//! - Compaction (context management)
//! - Skills (installable modules)
//! - Team coordination (builder/validator)

use crate::compaction::{CompactionConfig, CompactionManager};
use crate::hooks::{HookManager, SessionEndReason, SessionSource};
use crate::skills::{SkillManager, SkillPack};
use crate::tasks::{Task, TaskGraph, TaskGraphConfig};
use crate::tools::{ToolSearchConfig, ToolSearchTool};
use crate::validators::{ValidationContext, ValidationResult, ValidatorRegistry};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unified capabilities for an enhanced agent
pub struct AgentCapabilities {
    pub hooks: Arc<HookManager>,
    pub tasks: Arc<RwLock<TaskGraph>>,
    pub validators: Arc<ValidatorRegistry>,
    pub tools: Arc<RwLock<ToolSearchTool>>,
    pub compaction: Arc<RwLock<CompactionManager>>,
    pub skills: Arc<RwLock<SkillManager>>,
    config: CapabilitiesConfig,
}

/// Configuration for agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    pub hooks_enabled: bool,
    pub tasks_enabled: bool,
    pub validators_enabled: bool,
    pub tool_search_enabled: bool,
    pub compaction_enabled: bool,
    pub skills_enabled: bool,
    pub log_dir: PathBuf,
    pub skills_dir: PathBuf,
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            hooks_enabled: true,
            tasks_enabled: true,
            validators_enabled: true,
            tool_search_enabled: true,
            compaction_enabled: true,
            skills_enabled: true,
            log_dir: PathBuf::from(".goose/logs"),
            skills_dir: PathBuf::from(".goose/skills"),
        }
    }
}

impl AgentCapabilities {
    pub fn new(session_id: &str, config: CapabilitiesConfig) -> Self {
        let hooks = Arc::new(HookManager::new(session_id, &config.log_dir));
        let tasks = Arc::new(RwLock::new(TaskGraph::new(TaskGraphConfig::default())));
        let validators = Arc::new(ValidatorRegistry::new());
        let tools = Arc::new(RwLock::new(
            ToolSearchTool::new(ToolSearchConfig::default()),
        ));
        let compaction = Arc::new(RwLock::new(CompactionManager::new(
            CompactionConfig::default(),
        )));
        let skills = Arc::new(RwLock::new(SkillManager::new()));

        Self {
            hooks,
            tasks,
            validators,
            tools,
            compaction,
            skills,
            config,
        }
    }

    /// Initialize capabilities with default configuration
    pub fn with_defaults(session_id: &str) -> Self {
        Self::new(session_id, CapabilitiesConfig::default())
    }

    /// Fire session start hook
    pub async fn on_session_start(&self, cwd: &str, model: Option<String>) {
        if self.config.hooks_enabled {
            self.hooks
                .fire_session_start(
                    SessionSource::Startup,
                    self.hooks.run_id(),
                    "",
                    cwd,
                    "default",
                    model,
                    None,
                )
                .await;
        }
    }

    /// Fire session end hook
    pub async fn on_session_end(&self, cwd: &str, reason: SessionEndReason) {
        if self.config.hooks_enabled {
            self.hooks
                .fire_session_end(reason, self.hooks.run_id(), cwd)
                .await;
        }
    }

    /// Check if a tool call should be blocked
    pub async fn check_tool_permission(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        cwd: &str,
    ) -> (bool, Option<String>) {
        if !self.config.hooks_enabled {
            return (false, None);
        }

        let (blocked, reason, _context) = self
            .hooks
            .check_pre_tool_use(tool_name, tool_input, self.hooks.run_id(), cwd)
            .await;

        (blocked, reason)
    }

    /// Fire post tool use hook
    pub async fn on_tool_complete(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        tool_response: &serde_json::Value,
        cwd: &str,
    ) {
        if self.config.hooks_enabled {
            self.hooks
                .fire_post_tool_use(
                    tool_name,
                    tool_input,
                    tool_response,
                    self.hooks.run_id(),
                    cwd,
                )
                .await;
        }
    }

    /// Fire tool failure hook
    pub async fn on_tool_failure(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        error: &str,
        cwd: &str,
    ) {
        if self.config.hooks_enabled {
            self.hooks
                .fire_post_tool_use_failure(tool_name, tool_input, error, self.hooks.run_id(), cwd)
                .await;
        }
    }

    /// Check if agent stop should be blocked (done gate)
    pub async fn check_stop_gate(&self, cwd: &str) -> (bool, Option<String>) {
        if !self.config.hooks_enabled {
            return (false, None);
        }

        self.hooks.check_stop(self.hooks.run_id(), cwd).await
    }

    /// Create a new task in the task graph
    pub async fn create_task(&self, id: &str, description: &str) -> Result<String> {
        if !self.config.tasks_enabled {
            return Ok(id.to_string());
        }

        let tasks = self.tasks.write().await;
        let task = Task::new(id, description);
        tasks.create(task).await
    }

    /// Run validators on the working directory
    pub async fn validate(&self, cwd: &str) -> Vec<ValidationResult> {
        if !self.config.validators_enabled {
            return vec![];
        }

        let context = ValidationContext::new(cwd);

        self.validators.validate_all(&context).await
    }

    /// Search for relevant tools
    pub async fn search_tools(&self, query: &str) -> Vec<crate::tools::ToolDefinition> {
        if !self.config.tool_search_enabled {
            return vec![];
        }

        let mut tools = self.tools.write().await;
        tools.get_relevant_tools(query)
    }

    /// Check if compaction is needed
    pub fn should_compact(&self, current_tokens: usize, max_tokens: usize) -> bool {
        if !self.config.compaction_enabled {
            return false;
        }

        // Use synchronous check - compaction manager doesn't need async for this
        let threshold = 0.85;
        (current_tokens as f32 / max_tokens as f32) >= threshold
    }

    /// Load a skill pack
    pub async fn load_skill(&self, skill: SkillPack) -> Result<()> {
        if !self.config.skills_enabled {
            return Ok(());
        }

        let mut skills = self.skills.write().await;
        skills.install(skill)
    }

    /// Get gates for pre-complete validation
    pub async fn get_pre_complete_gates(&self, skill_name: &str) -> Vec<String> {
        if !self.config.skills_enabled {
            return vec![];
        }

        let skills = self.skills.read().await;
        skills
            .get_gates(skill_name)
            .map(|g| g.pre_complete.clone())
            .unwrap_or_default()
    }

    /// Get hook statistics
    pub async fn get_hook_stats(&self) -> crate::hooks::HookStats {
        self.hooks.get_stats().await
    }

    /// Fire notification hook
    pub async fn notify(&self, message: &str, cwd: &str) {
        if self.config.hooks_enabled {
            self.hooks
                .fire_notification(message, self.hooks.run_id(), cwd)
                .await;
        }
    }

    /// Fire subagent start hook
    pub async fn on_subagent_start(&self, agent_id: &str, agent_type: &str, cwd: &str) {
        if self.config.hooks_enabled {
            self.hooks
                .fire_subagent_start(agent_id, agent_type, self.hooks.run_id(), cwd)
                .await;
        }
    }

    /// Check subagent stop gate
    pub async fn check_subagent_stop(&self, agent_id: &str, cwd: &str) -> (bool, Option<String>) {
        if !self.config.hooks_enabled {
            return (false, None);
        }

        self.hooks
            .check_subagent_stop(agent_id, self.hooks.run_id(), cwd)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capabilities_creation() {
        let caps = AgentCapabilities::with_defaults("test-session");
        assert!(caps.config.hooks_enabled);
        assert!(caps.config.tasks_enabled);
    }

    #[tokio::test]
    async fn test_tool_permission_check() {
        let caps = AgentCapabilities::with_defaults("test-session");
        let (blocked, _reason) = caps
            .check_tool_permission("Bash", &serde_json::json!({"command": "ls"}), "/tmp")
            .await;
        // No hooks configured, should not block
        assert!(!blocked);
    }

    #[tokio::test]
    async fn test_stop_gate_check() {
        let caps = AgentCapabilities::with_defaults("test-session");
        let (blocked, _reason) = caps.check_stop_gate("/tmp").await;
        // No hooks configured, should not block
        assert!(!blocked);
    }

    #[test]
    fn test_should_compact() {
        let caps = AgentCapabilities::with_defaults("test-session");

        // Below threshold
        assert!(!caps.should_compact(8000, 10000));

        // Above threshold
        assert!(caps.should_compact(9000, 10000));
    }
}
