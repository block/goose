//! Team Agents Module - Builder/Validator Pairing
//!
//! Implements Claude Code-style team-based workflows:
//! - Builder agent: Full tool access, implements features
//! - Validator agent: Read-only, verifies work
//! - Mandatory pairing enforcement
//! - Validator has authority to fail/rollback

mod builder;
mod coordinator;
mod validator;

pub use builder::BuilderAgent;
pub use coordinator::{TeamConfig, TeamCoordinator, TeamResult, TeamWorkflow};
pub use validator::ValidatorAgent;

use crate::validators::ValidationResult;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Role of an agent in a team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    Builder,
    Validator,
    Orchestrator,
    Reviewer,
}

/// Capabilities available to team members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamCapabilities {
    pub can_write: bool,
    pub can_edit: bool,
    pub can_execute: bool,
    pub can_read: bool,
    pub can_search: bool,
    pub can_spawn_subagents: bool,
}

impl TeamCapabilities {
    pub fn builder() -> Self {
        Self {
            can_write: true,
            can_edit: true,
            can_execute: true,
            can_read: true,
            can_search: true,
            can_spawn_subagents: true,
        }
    }

    pub fn validator() -> Self {
        Self {
            can_write: false,
            can_edit: false,
            can_execute: true, // Can run tests
            can_read: true,
            can_search: true,
            can_spawn_subagents: false,
        }
    }

    pub fn reviewer() -> Self {
        Self {
            can_write: false,
            can_edit: false,
            can_execute: false,
            can_read: true,
            can_search: true,
            can_spawn_subagents: false,
        }
    }
}

/// Team member definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub name: String,
    pub role: TeamRole,
    pub capabilities: TeamCapabilities,
    pub validators: Vec<String>,
    pub acceptance_criteria: Vec<String>,
}

impl TeamMember {
    pub fn builder(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role: TeamRole::Builder,
            capabilities: TeamCapabilities::builder(),
            validators: vec!["rust".to_string(), "no_todos".to_string()],
            acceptance_criteria: Vec::new(),
        }
    }

    pub fn validator(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role: TeamRole::Validator,
            capabilities: TeamCapabilities::validator(),
            validators: Vec::new(),
            acceptance_criteria: Vec::new(),
        }
    }

    pub fn with_validators(mut self, validators: Vec<String>) -> Self {
        self.validators = validators;
        self
    }

    pub fn with_acceptance_criteria(mut self, criteria: Vec<String>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }
}

/// Task assignment for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask {
    pub id: String,
    pub description: String,
    pub builder_id: String,
    pub validator_id: String,
    pub status: TeamTaskStatus,
    pub build_result: Option<BuildResult>,
    pub validation_result: Option<ValidationResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TeamTaskStatus {
    #[default]
    Pending,
    Building,
    Built,
    Validating,
    Validated,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<String>,
    pub changed_files: Vec<String>,
    pub duration_ms: u64,
}

/// Trait for team agents
#[async_trait]
pub trait TeamAgent: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn role(&self) -> TeamRole;
    fn capabilities(&self) -> &TeamCapabilities;

    async fn execute_task(&self, task: &TeamTask) -> Result<BuildResult>;
    async fn validate_work(
        &self,
        task: &TeamTask,
        build_result: &BuildResult,
    ) -> Result<ValidationResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_member_builder() {
        let builder = TeamMember::builder("b1", "Feature Builder");
        assert_eq!(builder.role, TeamRole::Builder);
        assert!(builder.capabilities.can_write);
        assert!(builder.capabilities.can_edit);
    }

    #[test]
    fn test_team_member_validator() {
        let validator = TeamMember::validator("v1", "Code Reviewer");
        assert_eq!(validator.role, TeamRole::Validator);
        assert!(!validator.capabilities.can_write);
        assert!(!validator.capabilities.can_edit);
        assert!(validator.capabilities.can_read);
    }

    #[test]
    fn test_team_capabilities() {
        let builder_caps = TeamCapabilities::builder();
        let validator_caps = TeamCapabilities::validator();

        assert!(builder_caps.can_write);
        assert!(!validator_caps.can_write);
        assert!(validator_caps.can_execute); // Can run tests
    }
}
