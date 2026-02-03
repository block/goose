//! Policies Module
//!
//! YAML-based rule engine for policy enforcement and action automation.
//! Supports 18+ condition types, multiple action types, and hot-reload capabilities.
//!
//! This module provides:
//! - YAML-based rule definition and loading
//! - Flexible condition evaluation (string, numeric, temporal, logical)
//! - Configurable actions (block, warn, notify, require approval)
//! - Hot-reload support for runtime policy updates

pub mod actions;
pub mod conditions;
pub mod errors;
pub mod loader;
pub mod rule_engine;

pub use actions::{Action, ActionContext, ActionExecutor, ActionResult};
pub use conditions::{Condition, ConditionContext, ConditionEvaluator, ConditionResult};
pub use errors::PolicyError;
pub use loader::{PolicyLoader, PolicyWatcher};
pub use rule_engine::{
    Event, EventType, Rule, RuleEngine, RuleEvaluationResult, RuleMatch, RuleSet, Severity,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// Policy engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Enable/disable policy enforcement
    pub enabled: bool,
    /// Policy directory path
    pub policy_dir: PathBuf,
    /// Enable hot-reload of policies
    pub hot_reload_enabled: bool,
    /// Hot-reload check interval in seconds
    pub reload_interval_secs: u64,
    /// Fail mode on evaluation errors
    pub fail_mode: PolicyFailMode,
    /// Maximum evaluation time per rule (ms)
    pub max_rule_eval_time_ms: u64,
    /// Enable dry-run mode (log but don't enforce)
    pub dry_run: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy_dir: PathBuf::from("policies"),
            hot_reload_enabled: true,
            reload_interval_secs: 30,
            fail_mode: PolicyFailMode::FailClosed,
            max_rule_eval_time_ms: 100,
            dry_run: false,
        }
    }
}

/// Fail mode for policy evaluation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyFailMode {
    /// Block on errors (safer)
    #[default]
    FailClosed,
    /// Allow through on errors (more permissive)
    FailOpen,
}

/// Main policy engine orchestrator
pub struct PolicyEngine {
    config: PolicyConfig,
    rule_engine: Arc<RuleEngine>,
    action_executor: Arc<ActionExecutor>,
    loader: Arc<PolicyLoader>,
}

impl PolicyEngine {
    /// Create new policy engine with default configuration
    pub fn new() -> Self {
        Self::with_config(PolicyConfig::default())
    }

    /// Create new policy engine with custom configuration
    pub fn with_config(config: PolicyConfig) -> Self {
        let rule_engine = Arc::new(RuleEngine::new());
        let action_executor = Arc::new(ActionExecutor::new());
        let loader = Arc::new(PolicyLoader::new(config.policy_dir.clone()));

        Self {
            config,
            rule_engine,
            action_executor,
            loader,
        }
    }

    /// Load policies from configured directory
    pub async fn load_policies(&self) -> Result<usize, PolicyError> {
        let rule_sets = self.loader.load_all().await?;
        let count = rule_sets.len();

        for rule_set in rule_sets {
            self.rule_engine.add_rule_set(rule_set).await;
        }

        Ok(count)
    }

    /// Load a single policy file
    pub async fn load_policy_file(&self, path: &std::path::Path) -> Result<(), PolicyError> {
        let rule_set = self.loader.load_file(path).await?;
        self.rule_engine.add_rule_set(rule_set).await;
        Ok(())
    }

    /// Evaluate an event against all policies
    pub async fn evaluate(&self, event: &Event) -> Result<PolicyDecision, PolicyError> {
        if !self.config.enabled {
            return Ok(PolicyDecision::allow("Policies disabled"));
        }

        let start = std::time::Instant::now();

        // Evaluate rules
        let result = self.rule_engine.evaluate(event).await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if !result.matched {
            return Ok(PolicyDecision {
                decision: Decision::Allow,
                reason: "No policy rules matched".to_string(),
                matched_rules: vec![],
                actions_to_execute: vec![],
                evaluation_time_ms: duration_ms,
                dry_run: self.config.dry_run,
            });
        }

        // Determine decision based on highest severity match
        let (decision, reason) = self.determine_decision(&result);

        // Collect actions to execute
        let actions_to_execute: Vec<Action> = result
            .matches
            .iter()
            .flat_map(|m| m.actions.clone())
            .collect();

        let policy_decision = PolicyDecision {
            decision,
            reason,
            matched_rules: result.matches.iter().map(|m| m.rule_id.clone()).collect(),
            actions_to_execute: actions_to_execute.clone(),
            evaluation_time_ms: duration_ms,
            dry_run: self.config.dry_run,
        };

        // Execute actions if not in dry-run mode
        if !self.config.dry_run {
            let action_context = ActionContext {
                event: event.clone(),
                matched_rules: result.matches.clone(),
            };

            for action in &actions_to_execute {
                if let Err(e) = self.action_executor.execute(action, &action_context).await {
                    tracing::warn!("Action execution failed: {}", e);
                }
            }
        }

        Ok(policy_decision)
    }

    /// Determine the final decision based on rule matches
    fn determine_decision(&self, result: &RuleEvaluationResult) -> (Decision, String) {
        // Find the highest severity blocking action
        for rule_match in &result.matches {
            for action in &rule_match.actions {
                match action {
                    Action::Block { reason } => {
                        return (Decision::Deny, reason.clone());
                    }
                    Action::RequireApproval { approvers } => {
                        return (
                            Decision::RequireApproval,
                            format!("Requires approval from: {}", approvers.join(", ")),
                        );
                    }
                    _ => continue,
                }
            }
        }

        // No blocking actions, allow with warning
        let warnings: Vec<String> = result
            .matches
            .iter()
            .flat_map(|m| {
                m.actions.iter().filter_map(|a| {
                    if let Action::Warn { message } = a {
                        Some(message.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        if !warnings.is_empty() {
            (Decision::AllowWithWarning, warnings.join("; "))
        } else {
            (Decision::Allow, "Allowed by policy".to_string())
        }
    }

    /// Get rule engine reference
    pub fn rule_engine(&self) -> &Arc<RuleEngine> {
        &self.rule_engine
    }

    /// Get loader reference
    pub fn loader(&self) -> &Arc<PolicyLoader> {
        &self.loader
    }

    /// Reload all policies
    pub async fn reload(&self) -> Result<usize, PolicyError> {
        self.rule_engine.clear().await;
        self.load_policies().await
    }

    /// Get policy statistics
    pub async fn get_stats(&self) -> PolicyStats {
        let rule_sets = self.rule_engine.get_rule_sets().await;
        let total_rules: usize = rule_sets.iter().map(|rs| rs.rules.len()).sum();
        let enabled_rules: usize = rule_sets
            .iter()
            .flat_map(|rs| rs.rules.iter())
            .filter(|r| r.enabled)
            .count();

        PolicyStats {
            total_rule_sets: rule_sets.len(),
            total_rules,
            enabled_rules,
            disabled_rules: total_rules - enabled_rules,
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Policy decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Final decision
    pub decision: Decision,
    /// Reason for the decision
    pub reason: String,
    /// IDs of rules that matched
    pub matched_rules: Vec<String>,
    /// Actions that were/will be executed
    pub actions_to_execute: Vec<Action>,
    /// Time taken for evaluation (ms)
    pub evaluation_time_ms: u64,
    /// Whether this was a dry-run
    pub dry_run: bool,
}

impl PolicyDecision {
    /// Create an allow decision
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            decision: Decision::Allow,
            reason: reason.into(),
            matched_rules: vec![],
            actions_to_execute: vec![],
            evaluation_time_ms: 0,
            dry_run: false,
        }
    }

    /// Check if the decision allows the action
    pub fn is_allowed(&self) -> bool {
        matches!(self.decision, Decision::Allow | Decision::AllowWithWarning)
    }

    /// Check if approval is required
    pub fn requires_approval(&self) -> bool {
        matches!(self.decision, Decision::RequireApproval)
    }
}

/// Decision types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Allow the action
    Allow,
    /// Allow but with warnings
    AllowWithWarning,
    /// Deny the action
    Deny,
    /// Require human approval
    RequireApproval,
}

/// Policy statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyStats {
    /// Number of loaded rule sets
    pub total_rule_sets: usize,
    /// Total number of rules
    pub total_rules: usize,
    /// Number of enabled rules
    pub enabled_rules: usize,
    /// Number of disabled rules
    pub disabled_rules: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_config_default() {
        let config = PolicyConfig::default();
        assert!(config.enabled);
        assert!(config.hot_reload_enabled);
        assert_eq!(config.reload_interval_secs, 30);
    }

    #[test]
    fn test_policy_decision_allow() {
        let decision = PolicyDecision::allow("Test reason");
        assert!(decision.is_allowed());
        assert!(!decision.requires_approval());
    }

    #[tokio::test]
    async fn test_policy_engine_creation() {
        let engine = PolicyEngine::new();
        assert!(engine.config.enabled);
    }

    #[tokio::test]
    async fn test_policy_stats() {
        let engine = PolicyEngine::new();
        let stats = engine.get_stats().await;

        assert_eq!(stats.total_rule_sets, 0);
        assert_eq!(stats.total_rules, 0);
    }
}
