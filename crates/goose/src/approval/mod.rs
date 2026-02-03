use serde::{Deserialize, Serialize};

pub mod environment;
pub mod presets;

pub use environment::{detect_environment, is_running_in_ci, is_running_in_docker};
pub use presets::{ApprovalPreset, AutopilotMode, ParanoidMode, SafeMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Environment {
    #[default]
    RealFilesystem,
    DockerSandbox,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    pub environment: Environment,
    pub working_dir: String,
    pub session_id: Option<String>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_environment(mut self, env: Environment) -> Self {
        self.environment = env;
        self
    }

    pub fn with_working_dir(mut self, dir: &str) -> Self {
        self.working_dir = dir.to_string();
        self
    }

    pub fn in_sandbox(&self) -> bool {
        matches!(self.environment, Environment::DockerSandbox)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub command: String,
    pub reason: String,
    pub risk_level: RiskLevel,
    pub matched_patterns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Safe => write!(f, "Safe"),
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approved,
    RequiresApproval(ApprovalRequest),
    Blocked(String),
}

pub trait ApprovalPolicy: Send + Sync {
    fn name(&self) -> &str;

    fn evaluate(&self, cmd: &str, context: &ExecutionContext) -> ApprovalDecision;

    fn description(&self) -> &str {
        "No description"
    }
}

pub struct ApprovalManager {
    policy: Box<dyn ApprovalPolicy>,
}

impl ApprovalManager {
    pub fn new(policy: Box<dyn ApprovalPolicy>) -> Self {
        Self { policy }
    }

    pub fn from_preset(preset: ApprovalPreset) -> Self {
        Self {
            policy: preset.into_policy(),
        }
    }

    pub fn evaluate(&self, cmd: &str, context: &ExecutionContext) -> ApprovalDecision {
        self.policy.evaluate(cmd, context)
    }

    pub fn policy_name(&self) -> &str {
        self.policy.name()
    }

    pub fn policy_description(&self) -> &str {
        self.policy.description()
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::from_preset(ApprovalPreset::Safe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context() {
        let ctx = ExecutionContext::new()
            .with_environment(Environment::DockerSandbox)
            .with_working_dir("/workspace");

        assert!(ctx.in_sandbox());
        assert_eq!(ctx.working_dir, "/workspace");
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe < RiskLevel::Low);
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }
}
