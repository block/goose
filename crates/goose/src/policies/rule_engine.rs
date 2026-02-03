//! Rule Engine
//!
//! Core rule evaluation engine supporting YAML-based policy definitions.

use super::actions::Action;
use super::conditions::{Condition, ConditionContext, ConditionEvaluator};
use super::errors::PolicyError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Rule severity levels
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Low severity
    Low = 0,
    /// Medium severity
    #[default]
    Medium = 1,
    /// High severity
    High = 2,
    /// Critical severity
    Critical = 3,
}

impl Severity {
    /// Get numeric priority (higher is more severe)
    pub fn priority(&self) -> u8 {
        *self as u8
    }
}

/// Event types that rules can match against
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Tool execution event
    ToolExecution,
    /// Message received
    MessageReceived,
    /// Message sent
    MessageSent,
    /// Session started
    SessionStart,
    /// Session ended
    SessionEnd,
    /// Permission request
    PermissionRequest,
    /// Resource access
    ResourceAccess,
    /// Custom event type
    Custom(String),
    /// Match all events
    All,
}

impl EventType {
    /// Check if this event type matches another
    pub fn matches(&self, other: &EventType) -> bool {
        match (self, other) {
            (EventType::All, _) | (_, EventType::All) => true,
            (EventType::Custom(a), EventType::Custom(b)) => a == b,
            (a, b) => a == b,
        }
    }
}

/// An event to be evaluated against rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event type
    pub event_type: EventType,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event data (field values)
    pub data: HashMap<String, Value>,
    /// Event metadata
    pub metadata: HashMap<String, Value>,
}

impl Event {
    /// Create a new event
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            data: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a data field
    pub fn with_data(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), v);
        }
        self
    }

    /// Get a field value by path (supports dot notation)
    pub fn get_field(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return None;
        }

        // Try data first, then metadata
        let value = if let Some(v) = self.data.get(parts[0]) {
            Some(v)
        } else {
            self.metadata.get(parts[0])
        };

        if parts.len() == 1 {
            return value;
        }

        // Navigate nested path
        let mut current = value?;
        for part in &parts[1..] {
            match current {
                Value::Object(obj) => {
                    current = obj.get(*part)?;
                }
                Value::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        current = arr.get(idx)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(current)
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new(EventType::Custom("default".to_string()))
    }
}

/// A rule set containing multiple rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    /// Rule set version
    pub version: String,
    /// Rule set name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Rules in this set
    pub rules: Vec<Rule>,
    /// Metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl RuleSet {
    /// Create a new rule set
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: "1.0".to_string(),
            name: name.into(),
            description: None,
            rules: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Get enabled rules
    pub fn enabled_rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter().filter(|r| r.enabled)
    }
}

/// A single rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique rule identifier
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// Whether the rule is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Rule severity
    #[serde(default)]
    pub severity: Severity,
    /// Event types this rule applies to
    pub event_types: Vec<EventType>,
    /// Conditions that must match
    pub conditions: Vec<Condition>,
    /// Actions to execute on match
    pub actions: Vec<Action>,
    /// Rule metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

fn default_enabled() -> bool {
    true
}

impl Rule {
    /// Create a new rule
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            enabled: true,
            severity: Severity::Medium,
            event_types: Vec::new(), // Start empty, will default to All if none added
            conditions: Vec::new(),
            actions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Add event type
    pub fn for_event_type(mut self, event_type: EventType) -> Self {
        self.event_types.push(event_type);
        self
    }

    /// Add condition
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add action
    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Check if rule applies to event type
    /// If no event types are specified, matches all events
    pub fn applies_to(&self, event_type: &EventType) -> bool {
        if self.event_types.is_empty() {
            return true; // No restrictions = matches all
        }
        self.event_types.iter().any(|t| t.matches(event_type))
    }
}

/// Result of a rule match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    /// Rule ID that matched
    pub rule_id: String,
    /// Rule severity
    pub severity: Severity,
    /// Actions to execute
    pub actions: Vec<Action>,
    /// Evidence from condition evaluation
    pub evidence: Vec<String>,
}

/// Result of rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluationResult {
    /// Whether any rules matched
    pub matched: bool,
    /// Rule matches
    pub matches: Vec<RuleMatch>,
    /// Evaluation timestamp
    pub evaluated_at: DateTime<Utc>,
}

impl RuleEvaluationResult {
    /// Create an empty (no match) result
    pub fn no_match() -> Self {
        Self {
            matched: false,
            matches: vec![],
            evaluated_at: Utc::now(),
        }
    }
}

/// Rule evaluation engine
pub struct RuleEngine {
    /// Loaded rule sets
    rule_sets: RwLock<Vec<RuleSet>>,
    /// Condition evaluator
    evaluator: ConditionEvaluator,
}

impl RuleEngine {
    /// Create a new rule engine
    pub fn new() -> Self {
        Self {
            rule_sets: RwLock::new(Vec::new()),
            evaluator: ConditionEvaluator::new(),
        }
    }

    /// Add a rule set
    pub async fn add_rule_set(&self, rule_set: RuleSet) {
        let mut rule_sets = self.rule_sets.write().await;
        rule_sets.push(rule_set);
    }

    /// Remove a rule set by name
    pub async fn remove_rule_set(&self, name: &str) -> bool {
        let mut rule_sets = self.rule_sets.write().await;
        let initial_len = rule_sets.len();
        rule_sets.retain(|rs| rs.name != name);
        rule_sets.len() < initial_len
    }

    /// Clear all rule sets
    pub async fn clear(&self) {
        let mut rule_sets = self.rule_sets.write().await;
        rule_sets.clear();
    }

    /// Get all rule sets
    pub async fn get_rule_sets(&self) -> Vec<RuleSet> {
        let rule_sets = self.rule_sets.read().await;
        rule_sets.clone()
    }

    /// Evaluate an event against all rules
    pub async fn evaluate(&self, event: &Event) -> Result<RuleEvaluationResult, PolicyError> {
        let rule_sets = self.rule_sets.read().await;
        let mut matches = Vec::new();

        for rule_set in rule_sets.iter() {
            for rule in rule_set.enabled_rules() {
                // Check if rule applies to this event type
                if !rule.applies_to(&event.event_type) {
                    continue;
                }

                // Evaluate conditions
                let context = ConditionContext {
                    event: event.clone(),
                };

                match self
                    .evaluate_rule_conditions(&rule.conditions, &context)
                    .await
                {
                    Ok(result) if result.matched => {
                        matches.push(RuleMatch {
                            rule_id: rule.id.clone(),
                            severity: rule.severity,
                            actions: rule.actions.clone(),
                            evidence: result.evidence,
                        });
                    }
                    Ok(_) => {
                        // Conditions didn't match
                    }
                    Err(e) => {
                        tracing::warn!("Error evaluating rule {}: {}", rule.id, e);
                    }
                }
            }
        }

        // Sort by severity (highest first)
        matches.sort_by_key(|m| std::cmp::Reverse(m.severity.priority()));

        Ok(RuleEvaluationResult {
            matched: !matches.is_empty(),
            matches,
            evaluated_at: Utc::now(),
        })
    }

    /// Evaluate rule conditions (implicit AND)
    async fn evaluate_rule_conditions(
        &self,
        conditions: &[Condition],
        context: &ConditionContext,
    ) -> Result<super::conditions::ConditionResult, PolicyError> {
        if conditions.is_empty() {
            // No conditions = always match
            return Ok(super::conditions::ConditionResult {
                matched: true,
                evidence: vec!["No conditions defined".to_string()],
            });
        }

        let mut evidence = Vec::new();

        for condition in conditions {
            let result = self.evaluator.evaluate(condition, context).await?;

            if !result.matched {
                return Ok(super::conditions::ConditionResult {
                    matched: false,
                    evidence: vec![],
                });
            }

            evidence.extend(result.evidence);
        }

        Ok(super::conditions::ConditionResult {
            matched: true,
            evidence,
        })
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }

    #[test]
    fn test_event_type_matching() {
        assert!(EventType::All.matches(&EventType::ToolExecution));
        assert!(EventType::ToolExecution.matches(&EventType::All));
        assert!(EventType::ToolExecution.matches(&EventType::ToolExecution));
        assert!(!EventType::ToolExecution.matches(&EventType::MessageReceived));
    }

    #[test]
    fn test_event_creation() {
        let event = Event::new(EventType::ToolExecution)
            .with_data("tool_name", "bash")
            .with_data("arguments", serde_json::json!({"command": "ls"}));

        assert_eq!(event.event_type, EventType::ToolExecution);
        assert!(event.data.contains_key("tool_name"));
    }

    #[test]
    fn test_event_get_field() {
        let event = Event::new(EventType::ToolExecution)
            .with_data("tool_name", "bash")
            .with_data("arguments", serde_json::json!({"command": "ls -la"}));

        assert_eq!(
            event.get_field("tool_name"),
            Some(&Value::String("bash".to_string()))
        );
        assert_eq!(
            event.get_field("arguments.command"),
            Some(&Value::String("ls -la".to_string()))
        );
        assert!(event.get_field("nonexistent").is_none());
    }

    #[test]
    fn test_rule_set_creation() {
        let mut rule_set = RuleSet::new("test-rules");
        rule_set.add_rule(Rule::new("test-rule-1", "Test rule"));

        assert_eq!(rule_set.name, "test-rules");
        assert_eq!(rule_set.rules.len(), 1);
    }

    #[test]
    fn test_rule_builder() {
        let rule = Rule::new("test", "Test rule")
            .with_severity(Severity::High)
            .for_event_type(EventType::ToolExecution)
            .with_action(Action::Block {
                reason: "Blocked".to_string(),
            });

        assert_eq!(rule.id, "test");
        assert_eq!(rule.severity, Severity::High);
        assert!(rule.applies_to(&EventType::ToolExecution));
        assert_eq!(rule.actions.len(), 1);
    }

    #[test]
    fn test_rule_applies_to() {
        let rule = Rule::new("test", "Test")
            .for_event_type(EventType::ToolExecution)
            .for_event_type(EventType::ResourceAccess);

        assert!(rule.applies_to(&EventType::ToolExecution));
        assert!(rule.applies_to(&EventType::ResourceAccess));
        assert!(!rule.applies_to(&EventType::MessageReceived));
    }

    #[tokio::test]
    async fn test_rule_engine_empty() {
        let engine = RuleEngine::new();
        let event = Event::new(EventType::ToolExecution);

        let result = engine.evaluate(&event).await.unwrap();
        assert!(!result.matched);
        assert!(result.matches.is_empty());
    }

    #[tokio::test]
    async fn test_rule_engine_add_remove() {
        let engine = RuleEngine::new();

        engine.add_rule_set(RuleSet::new("test-1")).await;
        engine.add_rule_set(RuleSet::new("test-2")).await;

        let rule_sets = engine.get_rule_sets().await;
        assert_eq!(rule_sets.len(), 2);

        assert!(engine.remove_rule_set("test-1").await);
        let rule_sets = engine.get_rule_sets().await;
        assert_eq!(rule_sets.len(), 1);

        engine.clear().await;
        let rule_sets = engine.get_rule_sets().await;
        assert!(rule_sets.is_empty());
    }

    #[tokio::test]
    async fn test_rule_evaluation_basic() {
        let engine = RuleEngine::new();

        // Create a rule that matches all tool executions
        let mut rule_set = RuleSet::new("test");
        rule_set.add_rule(
            Rule::new("block-all-tools", "Block all tool executions")
                .for_event_type(EventType::ToolExecution)
                .with_action(Action::Block {
                    reason: "All tools blocked".to_string(),
                }),
        );

        engine.add_rule_set(rule_set).await;

        let event = Event::new(EventType::ToolExecution).with_data("tool_name", "bash");

        let result = engine.evaluate(&event).await.unwrap();
        assert!(result.matched);
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].rule_id, "block-all-tools");
    }

    #[tokio::test]
    async fn test_rule_evaluation_no_match_event_type() {
        let engine = RuleEngine::new();

        let mut rule_set = RuleSet::new("test");
        rule_set.add_rule(
            Rule::new("block-tools", "Block tools")
                .for_event_type(EventType::ToolExecution)
                .with_action(Action::Block {
                    reason: "Blocked".to_string(),
                }),
        );

        engine.add_rule_set(rule_set).await;

        // Different event type
        let event = Event::new(EventType::MessageReceived);

        let result = engine.evaluate(&event).await.unwrap();
        assert!(!result.matched);
    }

    #[tokio::test]
    async fn test_rule_evaluation_disabled_rule() {
        let engine = RuleEngine::new();

        let mut rule = Rule::new("disabled-rule", "Disabled rule")
            .for_event_type(EventType::ToolExecution)
            .with_action(Action::Block {
                reason: "Blocked".to_string(),
            });
        rule.enabled = false;

        let mut rule_set = RuleSet::new("test");
        rule_set.add_rule(rule);

        engine.add_rule_set(rule_set).await;

        let event = Event::new(EventType::ToolExecution);
        let result = engine.evaluate(&event).await.unwrap();
        assert!(!result.matched);
    }
}
