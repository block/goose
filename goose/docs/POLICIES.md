# Goose Policies / Rule Engine

## Overview

The Policies module provides a YAML-based rule engine for policy enforcement and action automation. It supports 26 condition types, 11 action types, hot-reload capabilities, and dry-run mode.

## Features

- **YAML-Based Rules**: Define policies in human-readable YAML format
- **26 Condition Types**: String, numeric, temporal, collection, and logical conditions
- **11 Action Types**: Block, warn, log, notify, approval workflows, and more
- **Hot-Reload**: Update policies without restarting
- **Dry-Run Mode**: Test policies without enforcement
- **Severity Ordering**: Critical rules evaluated first

## Quick Start

```rust
use goose::policies::{
    PolicyEngine, PolicyConfig, Event, EventType
};

// Create policy engine
let config = PolicyConfig {
    enabled: true,
    policy_dir: PathBuf::from("policies"),
    hot_reload_enabled: true,
    ..Default::default()
};

let engine = PolicyEngine::with_config(config);

// Load policies from directory
engine.load_policies().await?;

// Evaluate an event
let event = Event::new(EventType::ToolExecution)
    .with_data("tool_name", "bash")
    .with_data("command", "rm -rf /tmp/test");

let decision = engine.evaluate(&event).await?;

match decision.decision {
    Decision::Allow => { /* proceed */ },
    Decision::AllowWithWarning => { /* proceed with caution */ },
    Decision::Deny => { /* block */ },
    Decision::RequireApproval => { /* request approval */ },
}
```

## YAML Policy Format

### Basic Structure

```yaml
version: "1.0"
name: "security-policies"
description: "Security policy rules for the agent"

rules:
  - id: "unique-rule-id"
    description: "Human-readable description"
    enabled: true
    severity: critical  # critical, high, medium, low
    event_types:
      - tool_execution
      - resource_access
    conditions:
      - type: contains
        field: "command"
        value: "rm -rf"
        case_sensitive: true
    actions:
      - type: block
        reason: "Dangerous command blocked"
```

### Event Types

| Event Type | Description |
|------------|-------------|
| `tool_execution` | Tool/function is being executed |
| `message_received` | Message received from user |
| `message_sent` | Message being sent to user |
| `session_start` | Session is starting |
| `session_end` | Session is ending |
| `permission_request` | Permission is being requested |
| `resource_access` | Resource is being accessed |
| `all` | Match all event types |

### Severity Levels

| Severity | Priority | Use Case |
|----------|----------|----------|
| `critical` | Highest | Immediate threats, data loss |
| `high` | High | Security violations |
| `medium` | Medium | Policy violations |
| `low` | Lowest | Informational |

## Condition Types (26)

### String Conditions

```yaml
# Contains
- type: contains
  field: "command"
  value: "rm -rf"
  case_sensitive: true

# Matches (regex)
- type: matches
  field: "email"
  pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"

# Equals
- type: equals
  field: "status"
  value: "active"

# StartsWith
- type: starts_with
  field: "path"
  value: "/etc/"
  case_sensitive: true

# EndsWith
- type: ends_with
  field: "file"
  value: ".exe"
  case_sensitive: false

# IsEmpty
- type: is_empty
  field: "description"

# IsNotEmpty
- type: is_not_empty
  field: "user_id"
```

### Numeric Conditions

```yaml
# GreaterThan
- type: greater_than
  field: "amount"
  value: 1000

# GreaterThanOrEqual
- type: greater_than_or_equal
  field: "count"
  value: 10

# LessThan
- type: less_than
  field: "retry_count"
  value: 5

# LessThanOrEqual
- type: less_than_or_equal
  field: "priority"
  value: 3

# Between
- type: between
  field: "score"
  min: 0.5
  max: 1.0
```

### Collection Conditions

```yaml
# InList
- type: in_list
  field: "extension"
  values: ["exe", "bat", "cmd", "ps1"]

# NotInList
- type: not_in_list
  field: "status"
  values: ["banned", "suspended"]

# HasKey
- type: has_key
  field: "metadata"
  key: "approval_id"

# HasLength
- type: has_length
  field: "items"
  min: 1
  max: 100

# ArrayContains
- type: array_contains
  field: "tags"
  value: "sensitive"
```

### Temporal Conditions

```yaml
# Before
- type: before
  field: "expires_at"
  datetime: "2025-01-01T00:00:00Z"

# After
- type: after
  field: "created_at"
  datetime: "2024-01-01T00:00:00Z"

# WithinLast
- type: within_last
  field: "last_login"
  duration: "24h"  # 24 hours
```

### Logical Conditions

```yaml
# And (all must match)
- type: and
  conditions:
    - type: contains
      field: "command"
      value: "sudo"
    - type: equals
      field: "user_role"
      value: "admin"

# Or (any must match)
- type: or
  conditions:
    - type: equals
      field: "status"
      value: "blocked"
    - type: equals
      field: "status"
      value: "suspended"

# Not
- type: not
  condition:
    type: equals
    field: "verified"
    value: true
```

### Special Conditions

```yaml
# Always matches
- type: always

# Never matches
- type: never

# Custom condition
- type: custom
  name: "my_custom_check"
  params:
    threshold: 0.9
```

## Action Types (11)

### Block

```yaml
- type: block
  reason: "Action blocked by security policy"
```

### Warn

```yaml
- type: warn
  message: "This action may have unintended consequences"
```

### Log

```yaml
- type: log
  level: "warn"  # error, warn, info, debug, trace
  message: "Suspicious activity detected: {tool_name}"
```

### Notify

```yaml
- type: notify
  channel: "slack"
  message: "Alert: {user_id} attempted {action}"
```

### RequireApproval

```yaml
- type: require_approval
  approvers:
    - "admin@company.com"
    - "@security-team"
```

### Modify

```yaml
- type: modify
  field: "arguments.timeout"
  value: 30
```

### RateLimit

```yaml
- type: rate_limit
  max_requests: 100
  window_seconds: 3600
  key: "user_id"
```

### Delay

```yaml
- type: delay
  milliseconds: 5000
```

### AddMetadata

```yaml
- type: add_metadata
  key: "policy_version"
  value: "1.0"
```

### Webhook

```yaml
- type: webhook
  url: "https://api.example.com/events"
  method: "POST"
  headers:
    Authorization: "Bearer {token}"
```

### Custom

```yaml
- type: custom
  name: "my_custom_action"
  params:
    setting: "value"
```

## Complete Example

```yaml
version: "1.0"
name: "enterprise-security"
description: "Enterprise security policies"

rules:
  # Block dangerous shell commands
  - id: "block-dangerous-commands"
    description: "Block potentially destructive shell commands"
    enabled: true
    severity: critical
    event_types:
      - tool_execution
    conditions:
      - type: matches
        field: "tool_name"
        pattern: "^(bash|shell|execute)$"
      - type: or
        conditions:
          - type: matches
            field: "arguments.command"
            pattern: "rm\\s+-rf\\s+/"
          - type: matches
            field: "arguments.command"
            pattern: "dd\\s+if=/dev"
          - type: contains
            field: "arguments.command"
            value: "mkfs"
            case_sensitive: false
    actions:
      - type: block
        reason: "Destructive command blocked by security policy"
      - type: log
        level: "error"
        message: "Blocked dangerous command: {arguments.command}"
      - type: notify
        channel: "security"
        message: "Security alert: Destructive command attempted"

  # Require approval for deployments
  - id: "require-deployment-approval"
    description: "Require approval for production deployments"
    enabled: true
    severity: high
    event_types:
      - tool_execution
    conditions:
      - type: in_list
        field: "tool_name"
        values: ["deploy", "kubectl", "terraform_apply"]
      - type: equals
        field: "environment"
        value: "production"
    actions:
      - type: require_approval
        approvers:
          - "@devops"
          - "@security"
      - type: log
        level: "info"
        message: "Deployment approval requested for {tool_name}"

  # Audit all tool executions
  - id: "audit-all-tools"
    description: "Log all tool executions for audit"
    enabled: true
    severity: low
    event_types:
      - tool_execution
    conditions:
      - type: always
    actions:
      - type: log
        level: "info"
        message: "Tool executed: {tool_name} by {user_id}"
      - type: add_metadata
        key: "audit_timestamp"
        value: "{timestamp}"
```

## Programmatic Usage

### Creating Rules in Code

```rust
use goose::policies::{
    Rule, RuleSet, Condition, Action, Severity, EventType
};

let mut rule_set = RuleSet::new("my-policies");

rule_set.add_rule(
    Rule::new("block-rm", "Block rm commands")
        .with_severity(Severity::Critical)
        .for_event_type(EventType::ToolExecution)
        .with_condition(Condition::Contains {
            field: "command".to_string(),
            value: "rm -rf".to_string(),
            case_sensitive: true,
        })
        .with_action(Action::Block {
            reason: "Dangerous command blocked".to_string(),
        })
);

engine.rule_engine().add_rule_set(rule_set).await;
```

### Hot-Reload

```rust
use goose::policies::PolicyWatcher;

// Create watcher
let mut watcher = PolicyWatcher::new(PathBuf::from("policies"))?;

// Handle changes
while let Some(event) = watcher.recv().await {
    match event {
        PolicyChangeEvent::Modified(path) => {
            engine.load_policy_file(&path).await?;
        }
        PolicyChangeEvent::Removed(path) => {
            // Handle removal
        }
        PolicyChangeEvent::Error(err) => {
            log::error!("Policy watch error: {}", err);
        }
    }
}
```

### Dry-Run Mode

```rust
let config = PolicyConfig {
    enabled: true,
    dry_run: true,  // Log but don't enforce
    ..Default::default()
};

let engine = PolicyEngine::with_config(config);

let decision = engine.evaluate(&event).await?;
// decision.dry_run == true
// Actions logged but not executed
```

## Configuration

### PolicyConfig

```rust
pub struct PolicyConfig {
    /// Enable/disable policy enforcement
    pub enabled: bool,

    /// Policy directory path
    pub policy_dir: PathBuf,

    /// Enable hot-reload of policies
    pub hot_reload_enabled: bool,

    /// Hot-reload check interval (seconds)
    pub reload_interval_secs: u64,

    /// Fail mode on evaluation errors
    pub fail_mode: PolicyFailMode,

    /// Maximum evaluation time per rule (ms)
    pub max_rule_eval_time_ms: u64,

    /// Enable dry-run mode
    pub dry_run: bool,
}
```

### Fail Modes

```rust
pub enum PolicyFailMode {
    FailClosed,  // Block on errors (safer)
    FailOpen,    // Allow on errors (more permissive)
}
```

## Testing

```bash
# Run policies unit tests
cargo test --package goose policies::

# Run integration tests
cargo test --package goose --test policies_integration_test
```

## Performance

- Target: < 5ms average rule evaluation
- Regex patterns are cached
- Rules evaluated in parallel where possible
- Severity ordering ensures critical rules checked first

## See Also

- [Enterprise Integration Action Plan](07_ENTERPRISE_INTEGRATION_ACTION_PLAN.md)
- [Comprehensive Audit Report](08_COMPREHENSIVE_AUDIT_REPORT.md)
