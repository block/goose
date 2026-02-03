//! Policy Loader
//!
//! Loads policy files from YAML and supports hot-reload.

use super::errors::PolicyError;
use super::rule_engine::RuleSet;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Policy file loader
pub struct PolicyLoader {
    /// Base directory for policy files
    policy_dir: PathBuf,
}

impl PolicyLoader {
    /// Create a new loader with the given policy directory
    pub fn new(policy_dir: PathBuf) -> Self {
        Self { policy_dir }
    }

    /// Load all policy files from the directory
    pub async fn load_all(&self) -> Result<Vec<RuleSet>, PolicyError> {
        if !self.policy_dir.exists() {
            return Ok(Vec::new());
        }

        let mut rule_sets = Vec::new();

        let entries = std::fs::read_dir(&self.policy_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if self.is_policy_file(&path) {
                match self.load_file(&path).await {
                    Ok(rule_set) => {
                        tracing::info!("Loaded policy file: {:?}", path);
                        rule_sets.push(rule_set);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load policy file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(rule_sets)
    }

    /// Load a single policy file
    pub async fn load_file(&self, path: &Path) -> Result<RuleSet, PolicyError> {
        let content = tokio::fs::read_to_string(path).await?;
        self.parse_content(&content, path)
    }

    /// Load from string content
    pub fn load_from_string(&self, content: &str) -> Result<RuleSet, PolicyError> {
        self.parse_content(content, Path::new("<string>"))
    }

    /// Parse policy content
    fn parse_content(&self, content: &str, source: &Path) -> Result<RuleSet, PolicyError> {
        let rule_set: RuleSet = serde_yaml::from_str(content).map_err(|e| {
            PolicyError::YamlParseError(serde_yaml::Error::from(e))
        })?;

        // Validate the rule set
        self.validate_rule_set(&rule_set, source)?;

        Ok(rule_set)
    }

    /// Validate a rule set
    fn validate_rule_set(&self, rule_set: &RuleSet, source: &Path) -> Result<(), PolicyError> {
        if rule_set.name.is_empty() {
            return Err(PolicyError::config(format!(
                "Rule set name is required in {:?}",
                source
            )));
        }

        // Check for duplicate rule IDs within the set
        let mut seen_ids = std::collections::HashSet::new();
        for rule in &rule_set.rules {
            if rule.id.is_empty() {
                return Err(PolicyError::config(format!(
                    "Rule ID is required in rule set '{}' ({:?})",
                    rule_set.name, source
                )));
            }

            if !seen_ids.insert(&rule.id) {
                return Err(PolicyError::config(format!(
                    "Duplicate rule ID '{}' in rule set '{}' ({:?})",
                    rule.id, rule_set.name, source
                )));
            }

            // Validate that rules have at least one event type
            if rule.event_types.is_empty() {
                return Err(PolicyError::config(format!(
                    "Rule '{}' has no event types in rule set '{}' ({:?})",
                    rule.id, rule_set.name, source
                )));
            }

            // Validate that rules have at least one action
            if rule.actions.is_empty() {
                return Err(PolicyError::config(format!(
                    "Rule '{}' has no actions in rule set '{}' ({:?})",
                    rule.id, rule_set.name, source
                )));
            }
        }

        Ok(())
    }

    /// Check if a path is a valid policy file
    fn is_policy_file(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        match path.extension().and_then(|e| e.to_str()) {
            Some("yaml") | Some("yml") => true,
            _ => false,
        }
    }

    /// Get the policy directory
    pub fn policy_dir(&self) -> &Path {
        &self.policy_dir
    }
}

/// Watches for policy file changes and triggers reloads
pub struct PolicyWatcher {
    /// Channel for receiving file change events
    rx: mpsc::Receiver<PolicyChangeEvent>,
    /// The watcher handle (kept alive)
    _watcher: RecommendedWatcher,
}

/// Policy change event
#[derive(Debug, Clone)]
pub enum PolicyChangeEvent {
    /// A policy file was created or modified
    Modified(PathBuf),
    /// A policy file was removed
    Removed(PathBuf),
    /// An error occurred
    Error(String),
}

impl PolicyWatcher {
    /// Create a new policy watcher
    pub fn new(policy_dir: PathBuf) -> Result<Self, PolicyError> {
        let (tx, rx) = mpsc::channel(100);

        let tx_clone = tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    for path in event.paths {
                        // Only process YAML files
                        if let Some(ext) = path.extension() {
                            if ext == "yaml" || ext == "yml" {
                                let change_event = match event.kind {
                                    notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                        PolicyChangeEvent::Modified(path.clone())
                                    }
                                    notify::EventKind::Remove(_) => {
                                        PolicyChangeEvent::Removed(path.clone())
                                    }
                                    _ => continue,
                                };

                                if tx_clone.blocking_send(change_event).is_err() {
                                    tracing::warn!("Failed to send policy change event");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx_clone.blocking_send(PolicyChangeEvent::Error(e.to_string()));
                }
            }
        }).map_err(|e| PolicyError::Internal(e.to_string()))?;

        // Start watching the policy directory
        if policy_dir.exists() {
            watcher.watch(&policy_dir, RecursiveMode::Recursive)
                .map_err(|e| PolicyError::Internal(e.to_string()))?;
        }

        Ok(Self {
            rx,
            _watcher: watcher,
        })
    }

    /// Receive the next change event
    pub async fn recv(&mut self) -> Option<PolicyChangeEvent> {
        self.rx.recv().await
    }
}

/// Policy schema for validation
#[derive(Debug, Clone)]
pub struct PolicySchema {
    /// Schema version
    pub version: String,
    /// Supported event types
    pub event_types: Vec<String>,
    /// Supported condition types
    pub condition_types: Vec<String>,
    /// Supported action types
    pub action_types: Vec<String>,
}

impl Default for PolicySchema {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            event_types: vec![
                "tool_execution".to_string(),
                "message_received".to_string(),
                "message_sent".to_string(),
                "session_start".to_string(),
                "session_end".to_string(),
                "permission_request".to_string(),
                "resource_access".to_string(),
                "all".to_string(),
            ],
            condition_types: vec![
                "contains".to_string(),
                "matches".to_string(),
                "equals".to_string(),
                "starts_with".to_string(),
                "ends_with".to_string(),
                "is_empty".to_string(),
                "is_not_empty".to_string(),
                "greater_than".to_string(),
                "greater_than_or_equal".to_string(),
                "less_than".to_string(),
                "less_than_or_equal".to_string(),
                "between".to_string(),
                "in_list".to_string(),
                "not_in_list".to_string(),
                "has_key".to_string(),
                "has_length".to_string(),
                "array_contains".to_string(),
                "before".to_string(),
                "after".to_string(),
                "within_last".to_string(),
                "and".to_string(),
                "or".to_string(),
                "not".to_string(),
                "always".to_string(),
                "never".to_string(),
                "custom".to_string(),
            ],
            action_types: vec![
                "block".to_string(),
                "warn".to_string(),
                "log".to_string(),
                "notify".to_string(),
                "require_approval".to_string(),
                "modify".to_string(),
                "rate_limit".to_string(),
                "delay".to_string(),
                "add_metadata".to_string(),
                "webhook".to_string(),
                "custom".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_policy_yaml() -> &'static str {
        r#"
version: "1.0"
name: "test-policies"
description: "Test policy rules"
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
"#
    }

    fn invalid_policy_no_name() -> &'static str {
        r#"
version: "1.0"
name: ""
rules: []
"#
    }

    fn invalid_policy_no_actions() -> &'static str {
        r#"
version: "1.0"
name: "test"
rules:
  - id: "test-rule"
    description: "Test"
    event_types:
      - tool_execution
    conditions: []
    actions: []
"#
    }

    #[test]
    fn test_loader_creation() {
        let loader = PolicyLoader::new(PathBuf::from("policies"));
        assert_eq!(loader.policy_dir(), Path::new("policies"));
    }

    #[test]
    fn test_parse_valid_policy() {
        let loader = PolicyLoader::new(PathBuf::from("policies"));
        let result = loader.load_from_string(sample_policy_yaml());

        assert!(result.is_ok());
        let rule_set = result.unwrap();
        assert_eq!(rule_set.name, "test-policies");
        assert_eq!(rule_set.rules.len(), 1);
        assert_eq!(rule_set.rules[0].id, "block-rm-rf");
    }

    #[test]
    fn test_parse_invalid_no_name() {
        let loader = PolicyLoader::new(PathBuf::from("policies"));
        let result = loader.load_from_string(invalid_policy_no_name());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("name is required"));
    }

    #[test]
    fn test_parse_invalid_no_actions() {
        let loader = PolicyLoader::new(PathBuf::from("policies"));
        let result = loader.load_from_string(invalid_policy_no_actions());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("no actions"));
    }

    #[tokio::test]
    async fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let policy_path = temp_dir.path().join("test.yaml");
        std::fs::write(&policy_path, sample_policy_yaml()).unwrap();

        let loader = PolicyLoader::new(temp_dir.path().to_path_buf());
        let result = loader.load_file(&policy_path).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_all() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple policy files
        std::fs::write(temp_dir.path().join("policy1.yaml"), sample_policy_yaml()).unwrap();
        std::fs::write(temp_dir.path().join("policy2.yml"), sample_policy_yaml()).unwrap();
        std::fs::write(temp_dir.path().join("not-a-policy.txt"), "text file").unwrap();

        let loader = PolicyLoader::new(temp_dir.path().to_path_buf());
        let result = loader.load_all().await;

        assert!(result.is_ok());
        let rule_sets = result.unwrap();
        assert_eq!(rule_sets.len(), 2);
    }

    #[tokio::test]
    async fn test_load_all_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let loader = PolicyLoader::new(temp_dir.path().to_path_buf());

        let result = loader.load_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_load_nonexistent_dir() {
        let loader = PolicyLoader::new(PathBuf::from("/nonexistent/path/policies"));
        let result = loader.load_all().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_is_policy_file() {
        let _loader = PolicyLoader::new(PathBuf::from("policies"));

        // These would need actual files, so we just test the extension logic
        // by checking the extension extraction
        assert!(Path::new("test.yaml")
            .extension()
            .map_or(false, |e| e == "yaml" || e == "yml"));
        assert!(Path::new("test.yml")
            .extension()
            .map_or(false, |e| e == "yaml" || e == "yml"));
        assert!(!Path::new("test.txt")
            .extension()
            .map_or(false, |e| e == "yaml" || e == "yml"));
    }

    #[test]
    fn test_policy_schema_default() {
        let schema = PolicySchema::default();

        assert_eq!(schema.version, "1.0");
        assert!(!schema.event_types.is_empty());
        assert!(!schema.condition_types.is_empty());
        assert!(!schema.action_types.is_empty());

        // Verify key types are present
        assert!(schema.event_types.contains(&"tool_execution".to_string()));
        assert!(schema.condition_types.contains(&"contains".to_string()));
        assert!(schema.action_types.contains(&"block".to_string()));
    }

    #[test]
    fn test_duplicate_rule_ids() {
        let yaml = r#"
version: "1.0"
name: "test"
rules:
  - id: "same-id"
    description: "First rule"
    event_types: [tool_execution]
    conditions: []
    actions:
      - type: block
        reason: "test"
  - id: "same-id"
    description: "Second rule"
    event_types: [tool_execution]
    conditions: []
    actions:
      - type: block
        reason: "test"
"#;

        let loader = PolicyLoader::new(PathBuf::from("policies"));
        let result = loader.load_from_string(yaml);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Duplicate rule ID"));
    }
}
