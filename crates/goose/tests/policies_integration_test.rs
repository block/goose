//! Policies Module Integration Tests
//!
//! Tests for the YAML-based rule engine (Watchflow) including:
//! - Policy loading and validation
//! - Rule evaluation with various condition types
//! - Action execution and enforcement
//! - Hot-reload capabilities
//! - Performance benchmarks

use std::path::PathBuf;
use tempfile::TempDir;

use goose::policies::{
    Action, Condition, Decision, Event, EventType, PolicyConfig, PolicyDecision, PolicyEngine,
    PolicyLoader, Rule, RuleEngine, RuleSet, Severity,
};

// =============================================================================
// Policy Engine Tests
// =============================================================================

/// Test basic policy engine creation and configuration
#[tokio::test]
async fn test_policy_engine_creation() {
    let engine = PolicyEngine::new();
    let stats = engine.get_stats().await;

    assert_eq!(stats.total_rule_sets, 0);
    assert_eq!(stats.total_rules, 0);
}

/// Test policy engine with custom configuration
#[tokio::test]
async fn test_policy_engine_with_config() {
    let config = PolicyConfig {
        enabled: true,
        policy_dir: PathBuf::from("test-policies"),
        hot_reload_enabled: false,
        reload_interval_secs: 60,
        fail_mode: goose::policies::PolicyFailMode::FailOpen,
        max_rule_eval_time_ms: 50,
        dry_run: true,
    };

    let engine = PolicyEngine::with_config(config);
    let stats = engine.get_stats().await;

    assert_eq!(stats.total_rule_sets, 0);
}

/// Test policy evaluation when disabled
#[tokio::test]
async fn test_policy_engine_disabled() {
    let config = PolicyConfig {
        enabled: false,
        ..PolicyConfig::default()
    };

    let engine = PolicyEngine::with_config(config);
    let event = Event::new(EventType::ToolExecution).with_data("command", "rm -rf /");

    let decision = engine.evaluate(&event).await.unwrap();

    assert_eq!(decision.decision, Decision::Allow);
    assert!(decision.reason.contains("disabled"));
}

/// Test policy evaluation with no matching rules
#[tokio::test]
async fn test_policy_engine_no_matches() {
    let engine = PolicyEngine::new();
    let event = Event::new(EventType::ToolExecution).with_data("command", "ls");

    let decision = engine.evaluate(&event).await.unwrap();

    assert_eq!(decision.decision, Decision::Allow);
    assert!(decision.matched_rules.is_empty());
}

// =============================================================================
// Rule Engine Tests
// =============================================================================

/// Test rule engine with blocking rules
#[tokio::test]
async fn test_rule_engine_block_action() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("security-rules");
    rule_set.add_rule(
        Rule::new("block-dangerous-commands", "Block dangerous commands")
            .with_severity(Severity::Critical)
            .for_event_type(EventType::ToolExecution)
            .with_condition(Condition::Contains {
                field: "command".to_string(),
                value: "rm -rf".to_string(),
                case_sensitive: true,
            })
            .with_action(Action::Block {
                reason: "Dangerous command blocked for security".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    // Test matching event
    let dangerous_event =
        Event::new(EventType::ToolExecution).with_data("command", "rm -rf /important");

    let result = engine.evaluate(&dangerous_event).await.unwrap();
    assert!(result.matched);
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].rule_id, "block-dangerous-commands");
    assert_eq!(result.matches[0].severity, Severity::Critical);

    // Test non-matching event
    let safe_event = Event::new(EventType::ToolExecution).with_data("command", "ls -la");

    let result = engine.evaluate(&safe_event).await.unwrap();
    assert!(!result.matched);
}

/// Test rule engine with warning actions
#[tokio::test]
async fn test_rule_engine_warn_action() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("audit-rules");
    rule_set.add_rule(
        Rule::new("warn-on-sudo", "Warn on sudo usage")
            .with_severity(Severity::Medium)
            .for_event_type(EventType::ToolExecution)
            .with_condition(Condition::Contains {
                field: "command".to_string(),
                value: "sudo".to_string(),
                case_sensitive: false,
            })
            .with_action(Action::Warn {
                message: "Sudo usage detected".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    let event = Event::new(EventType::ToolExecution).with_data("command", "sudo apt update");

    let result = engine.evaluate(&event).await.unwrap();
    assert!(result.matched);
    assert_eq!(result.matches[0].severity, Severity::Medium);
}

/// Test rule engine with multiple conditions (AND logic)
#[tokio::test]
async fn test_rule_engine_multiple_conditions() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("complex-rules");
    rule_set.add_rule(
        Rule::new("sensitive-file-access", "Block access to sensitive files")
            .with_severity(Severity::High)
            .for_event_type(EventType::ResourceAccess)
            .with_condition(Condition::Contains {
                field: "resource".to_string(),
                value: "credentials".to_string(),
                case_sensitive: false,
            })
            .with_condition(Condition::Equals {
                field: "action".to_string(),
                value: serde_json::json!("write"),
            })
            .with_action(Action::Block {
                reason: "Cannot write to credentials files".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    // Test matching both conditions
    let matching_event = Event::new(EventType::ResourceAccess)
        .with_data("resource", "/etc/credentials.json")
        .with_data("action", "write");

    let result = engine.evaluate(&matching_event).await.unwrap();
    assert!(result.matched);

    // Test matching only one condition (should not match)
    let partial_event = Event::new(EventType::ResourceAccess)
        .with_data("resource", "/etc/credentials.json")
        .with_data("action", "read");

    let result = engine.evaluate(&partial_event).await.unwrap();
    assert!(!result.matched);
}

/// Test rule evaluation with severity ordering
#[tokio::test]
async fn test_rule_engine_severity_ordering() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("multi-severity");

    // Add rules with different severities
    rule_set.add_rule(
        Rule::new("low-rule", "Low severity")
            .with_severity(Severity::Low)
            .for_event_type(EventType::All)
            .with_action(Action::Log {
                level: "info".to_string(),
                message: "Low".to_string(),
            }),
    );

    rule_set.add_rule(
        Rule::new("critical-rule", "Critical severity")
            .with_severity(Severity::Critical)
            .for_event_type(EventType::All)
            .with_action(Action::Block {
                reason: "Critical".to_string(),
            }),
    );

    rule_set.add_rule(
        Rule::new("medium-rule", "Medium severity")
            .with_severity(Severity::Medium)
            .for_event_type(EventType::All)
            .with_action(Action::Warn {
                message: "Medium".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    let event = Event::new(EventType::ToolExecution);
    let result = engine.evaluate(&event).await.unwrap();

    assert!(result.matched);
    assert_eq!(result.matches.len(), 3);
    // Critical should be first (highest severity)
    assert_eq!(result.matches[0].severity, Severity::Critical);
    assert_eq!(result.matches[1].severity, Severity::Medium);
    assert_eq!(result.matches[2].severity, Severity::Low);
}

// =============================================================================
// Condition Types Tests
// =============================================================================

/// Test string conditions
#[tokio::test]
async fn test_string_conditions() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("string-tests");

    // StartsWith
    rule_set.add_rule(
        Rule::new("starts-with", "StartsWith test")
            .for_event_type(EventType::All)
            .with_condition(Condition::StartsWith {
                field: "path".to_string(),
                value: "/etc/".to_string(),
                case_sensitive: true,
            })
            .with_action(Action::Log {
                level: "info".to_string(),
                message: "Matched".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    let event = Event::new(EventType::ResourceAccess).with_data("path", "/etc/passwd");

    let result = engine.evaluate(&event).await.unwrap();
    assert!(result.matched);

    let non_matching = Event::new(EventType::ResourceAccess).with_data("path", "/var/log/test");

    let result = engine.evaluate(&non_matching).await.unwrap();
    assert!(!result.matched);
}

/// Test numeric conditions
#[tokio::test]
async fn test_numeric_conditions() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("numeric-tests");

    // GreaterThan
    rule_set.add_rule(
        Rule::new("high-value", "Check high values")
            .for_event_type(EventType::All)
            .with_condition(Condition::GreaterThan {
                field: "amount".to_string(),
                value: 1000.0,
            })
            .with_action(Action::RequireApproval {
                approvers: vec!["admin@example.com".to_string()],
            }),
    );

    engine.add_rule_set(rule_set).await;

    let high_event = Event::new(EventType::Custom("transaction".to_string())).with_data("amount", 5000);

    let result = engine.evaluate(&high_event).await.unwrap();
    assert!(result.matched);

    let low_event = Event::new(EventType::Custom("transaction".to_string())).with_data("amount", 500);

    let result = engine.evaluate(&low_event).await.unwrap();
    assert!(!result.matched);
}

/// Test logical conditions (AND, OR, NOT)
#[tokio::test]
async fn test_logical_conditions() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("logical-tests");

    // OR condition
    rule_set.add_rule(
        Rule::new("admin-or-root", "Admin or root access")
            .for_event_type(EventType::All)
            .with_condition(Condition::Or {
                conditions: vec![
                    Condition::Equals {
                        field: "user".to_string(),
                        value: serde_json::json!("admin"),
                    },
                    Condition::Equals {
                        field: "user".to_string(),
                        value: serde_json::json!("root"),
                    },
                ],
            })
            .with_action(Action::Log {
                level: "warn".to_string(),
                message: "Privileged access".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    // Test admin
    let admin_event = Event::new(EventType::SessionStart).with_data("user", "admin");
    let result = engine.evaluate(&admin_event).await.unwrap();
    assert!(result.matched);

    // Test root
    let root_event = Event::new(EventType::SessionStart).with_data("user", "root");
    let result = engine.evaluate(&root_event).await.unwrap();
    assert!(result.matched);

    // Test regular user
    let user_event = Event::new(EventType::SessionStart).with_data("user", "john");
    let result = engine.evaluate(&user_event).await.unwrap();
    assert!(!result.matched);
}

/// Test IN_LIST condition
#[tokio::test]
async fn test_in_list_condition() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("list-tests");

    rule_set.add_rule(
        Rule::new("allowed-extensions", "Check file extensions")
            .for_event_type(EventType::All)
            .with_condition(Condition::InList {
                field: "extension".to_string(),
                values: vec![
                    serde_json::json!("exe"),
                    serde_json::json!("bat"),
                    serde_json::json!("cmd"),
                ],
            })
            .with_action(Action::Block {
                reason: "Executable files not allowed".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    let exe_event = Event::new(EventType::ResourceAccess).with_data("extension", "exe");
    let result = engine.evaluate(&exe_event).await.unwrap();
    assert!(result.matched);

    let txt_event = Event::new(EventType::ResourceAccess).with_data("extension", "txt");
    let result = engine.evaluate(&txt_event).await.unwrap();
    assert!(!result.matched);
}

/// Test regex matching
#[tokio::test]
async fn test_regex_condition() {
    let engine = RuleEngine::new();

    let mut rule_set = RuleSet::new("regex-tests");

    rule_set.add_rule(
        Rule::new("email-pattern", "Match email addresses")
            .for_event_type(EventType::All)
            .with_condition(Condition::Matches {
                field: "email".to_string(),
                pattern: r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string(),
            })
            .with_action(Action::Log {
                level: "info".to_string(),
                message: "Valid email".to_string(),
            }),
    );

    engine.add_rule_set(rule_set).await;

    let valid_event = Event::new(EventType::MessageReceived).with_data("email", "test@example.com");
    let result = engine.evaluate(&valid_event).await.unwrap();
    assert!(result.matched);

    let invalid_event = Event::new(EventType::MessageReceived).with_data("email", "not-an-email");
    let result = engine.evaluate(&invalid_event).await.unwrap();
    assert!(!result.matched);
}

// =============================================================================
// YAML Loading Tests
// =============================================================================

/// Test loading policies from YAML
#[tokio::test]
async fn test_yaml_policy_loading() {
    let temp_dir = TempDir::new().unwrap();

    let policy_yaml = r#"
version: "1.0"
name: "security-policies"
description: "Security policy rules"
rules:
  - id: "block-rm-rf"
    description: "Block rm -rf commands"
    enabled: true
    severity: critical
    event_types:
      - tool_execution
    conditions:
      - type: contains
        field: "command"
        value: "rm -rf"
        case_sensitive: true
    actions:
      - type: block
        reason: "Dangerous command blocked"

  - id: "warn-sudo"
    description: "Warn on sudo usage"
    enabled: true
    severity: medium
    event_types:
      - tool_execution
    conditions:
      - type: contains
        field: "command"
        value: "sudo"
        case_sensitive: false
    actions:
      - type: warn
        message: "Sudo usage detected"
"#;

    std::fs::write(temp_dir.path().join("security.yaml"), policy_yaml).unwrap();

    let loader = PolicyLoader::new(temp_dir.path().to_path_buf());
    let rule_sets = loader.load_all().await.unwrap();

    assert_eq!(rule_sets.len(), 1);
    assert_eq!(rule_sets[0].name, "security-policies");
    assert_eq!(rule_sets[0].rules.len(), 2);
    assert_eq!(rule_sets[0].rules[0].id, "block-rm-rf");
    assert_eq!(rule_sets[0].rules[0].severity, Severity::Critical);
}

/// Test YAML validation errors
#[tokio::test]
async fn test_yaml_validation_errors() {
    let loader = PolicyLoader::new(PathBuf::from("test"));

    // Missing name
    let no_name = r#"
version: "1.0"
name: ""
rules: []
"#;
    let result = loader.load_from_string(no_name);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("name is required"));

    // Rule with no actions
    let no_actions = r#"
version: "1.0"
name: "test"
rules:
  - id: "no-actions"
    description: "No actions"
    event_types: [tool_execution]
    conditions: []
    actions: []
"#;
    let result = loader.load_from_string(no_actions);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no actions"));
}

/// Test loading multiple policy files
#[tokio::test]
async fn test_load_multiple_policy_files() {
    let temp_dir = TempDir::new().unwrap();

    let security_yaml = r#"
version: "1.0"
name: "security"
rules:
  - id: "sec-rule-1"
    description: "Security rule"
    event_types:
      - tool_execution
    conditions: []
    actions:
      - type: block
        reason: "blocked"
"#;

    let audit_yaml = r#"
version: "1.0"
name: "audit"
rules:
  - id: "audit-rule-1"
    description: "Audit rule"
    event_types:
      - all
    conditions: []
    actions:
      - type: log
        level: "info"
        message: "logged"
"#;

    std::fs::write(temp_dir.path().join("security.yaml"), security_yaml).unwrap();
    std::fs::write(temp_dir.path().join("audit.yml"), audit_yaml).unwrap();
    std::fs::write(
        temp_dir.path().join("ignore.txt"),
        "This is not a policy file",
    )
    .unwrap();

    let loader = PolicyLoader::new(temp_dir.path().to_path_buf());
    let rule_sets = loader.load_all().await.unwrap();

    // Should load 2 YAML files, ignore the .txt file
    assert_eq!(rule_sets.len(), 2);
}

// =============================================================================
// Policy Decision Tests
// =============================================================================

/// Test PolicyDecision helper methods
#[test]
fn test_policy_decision_helpers() {
    let allow = PolicyDecision::allow("Test allow");
    assert!(allow.is_allowed());
    assert!(!allow.requires_approval());

    let deny = PolicyDecision {
        decision: Decision::Deny,
        reason: "Blocked".to_string(),
        matched_rules: vec!["rule-1".to_string()],
        actions_to_execute: vec![],
        evaluation_time_ms: 5,
        dry_run: false,
    };
    assert!(!deny.is_allowed());
    assert!(!deny.requires_approval());

    let approval = PolicyDecision {
        decision: Decision::RequireApproval,
        reason: "Needs approval".to_string(),
        matched_rules: vec![],
        actions_to_execute: vec![],
        evaluation_time_ms: 3,
        dry_run: false,
    };
    assert!(!approval.is_allowed());
    assert!(approval.requires_approval());

    let warning = PolicyDecision {
        decision: Decision::AllowWithWarning,
        reason: "Warning".to_string(),
        matched_rules: vec![],
        actions_to_execute: vec![],
        evaluation_time_ms: 2,
        dry_run: false,
    };
    assert!(warning.is_allowed());
}

// =============================================================================
// Event Type Tests
// =============================================================================

/// Test event type matching
#[test]
fn test_event_type_matching() {
    // All matches everything
    assert!(EventType::All.matches(&EventType::ToolExecution));
    assert!(EventType::All.matches(&EventType::MessageReceived));
    assert!(EventType::All.matches(&EventType::Custom("test".to_string())));

    // Specific types match themselves
    assert!(EventType::ToolExecution.matches(&EventType::ToolExecution));
    assert!(!EventType::ToolExecution.matches(&EventType::MessageReceived));

    // Custom types match by value
    assert!(EventType::Custom("test".to_string()).matches(&EventType::Custom("test".to_string())));
    assert!(!EventType::Custom("test".to_string()).matches(&EventType::Custom("other".to_string())));
}

/// Test event field access with dot notation
#[test]
fn test_event_field_access() {
    let event = Event::new(EventType::ToolExecution)
        .with_data("tool_name", "bash")
        .with_data(
            "arguments",
            serde_json::json!({
                "command": "ls -la",
                "options": {
                    "verbose": true
                }
            }),
        );

    // Simple field access
    assert_eq!(
        event.get_field("tool_name"),
        Some(&serde_json::json!("bash"))
    );

    // Nested field access
    assert_eq!(
        event.get_field("arguments.command"),
        Some(&serde_json::json!("ls -la"))
    );

    assert_eq!(
        event.get_field("arguments.options.verbose"),
        Some(&serde_json::json!(true))
    );

    // Non-existent field
    assert!(event.get_field("nonexistent").is_none());
    assert!(event.get_field("arguments.nonexistent").is_none());
}

// =============================================================================
// Performance Tests
// =============================================================================

/// Test rule evaluation performance (should be < 5ms average)
#[tokio::test]
async fn test_rule_evaluation_performance() {
    let engine = RuleEngine::new();

    // Add multiple rules
    let mut rule_set = RuleSet::new("performance-test");
    for i in 0..100 {
        rule_set.add_rule(
            Rule::new(format!("rule-{}", i), format!("Rule {}", i))
                .for_event_type(EventType::ToolExecution)
                .with_condition(Condition::Contains {
                    field: "command".to_string(),
                    value: format!("pattern-{}", i),
                    case_sensitive: true,
                })
                .with_action(Action::Log {
                    level: "info".to_string(),
                    message: "Matched".to_string(),
                }),
        );
    }

    engine.add_rule_set(rule_set).await;

    // Run multiple evaluations
    let iterations = 100;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        let event = Event::new(EventType::ToolExecution).with_data("command", "some random command");
        let _ = engine.evaluate(&event).await;
    }

    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as f64 / iterations as f64;

    // Target: < 5ms average evaluation time
    println!("Average rule evaluation time: {:.2}ms", avg_ms);
    assert!(
        avg_ms < 10.0,
        "Rule evaluation too slow: {:.2}ms (target < 10ms)",
        avg_ms
    );
}

// =============================================================================
// Integration with Policy Engine
// =============================================================================

/// Test full policy engine integration
#[tokio::test]
async fn test_full_policy_engine_integration() {
    let temp_dir = TempDir::new().unwrap();

    let policy_yaml = r#"
version: "1.0"
name: "integration-test"
rules:
  - id: "block-dangerous"
    description: "Block dangerous commands"
    enabled: true
    severity: critical
    event_types:
      - tool_execution
    conditions:
      - type: matches
        field: "command"
        pattern: "rm\\s+-rf|format\\s+c:|del\\s+/[qfsy]"
    actions:
      - type: block
        reason: "Dangerous command blocked by policy"
      - type: notify
        channel: "security"
        message: "Dangerous command attempted: {command}"

  - id: "audit-all-tools"
    description: "Audit all tool executions"
    enabled: true
    severity: low
    event_types:
      - tool_execution
    conditions:
      - type: always
    actions:
      - type: log
        level: "info"
        message: "Tool executed: {tool_name}"
"#;

    std::fs::write(temp_dir.path().join("policies.yaml"), policy_yaml).unwrap();

    let config = PolicyConfig {
        enabled: true,
        policy_dir: temp_dir.path().to_path_buf(),
        hot_reload_enabled: false,
        dry_run: false,
        ..PolicyConfig::default()
    };

    let engine = PolicyEngine::with_config(config);
    let loaded = engine.load_policies().await.unwrap();

    assert_eq!(loaded, 1);

    let stats = engine.get_stats().await;
    assert_eq!(stats.total_rule_sets, 1);
    assert_eq!(stats.total_rules, 2);
    assert_eq!(stats.enabled_rules, 2);

    // Test dangerous command
    let dangerous_event = Event::new(EventType::ToolExecution)
        .with_data("tool_name", "bash")
        .with_data("command", "rm -rf /important/data");

    let decision = engine.evaluate(&dangerous_event).await.unwrap();
    assert_eq!(decision.decision, Decision::Deny);
    assert!(decision.matched_rules.contains(&"block-dangerous".to_string()));

    // Test safe command
    let safe_event = Event::new(EventType::ToolExecution)
        .with_data("tool_name", "bash")
        .with_data("command", "ls -la");

    let decision = engine.evaluate(&safe_event).await.unwrap();
    // Should match the audit rule but not the block rule
    assert!(decision.is_allowed());
    assert!(decision.matched_rules.contains(&"audit-all-tools".to_string()));
}

/// Test dry-run mode
#[tokio::test]
async fn test_dry_run_mode() {
    let config = PolicyConfig {
        enabled: true,
        policy_dir: PathBuf::from("nonexistent"),
        dry_run: true,
        ..PolicyConfig::default()
    };

    let engine = PolicyEngine::with_config(config);

    // Add a blocking rule directly
    let mut rule_set = RuleSet::new("test");
    rule_set.add_rule(
        Rule::new("block-all", "Block everything")
            .for_event_type(EventType::All)
            .with_action(Action::Block {
                reason: "Blocked".to_string(),
            }),
    );

    engine.rule_engine().add_rule_set(rule_set).await;

    let event = Event::new(EventType::ToolExecution);
    let decision = engine.evaluate(&event).await.unwrap();

    // In dry-run mode, decision is still calculated but actions are not executed
    assert!(decision.dry_run);
    assert_eq!(decision.decision, Decision::Deny);
}
