//! Action Types and Execution
//!
//! Implements policy actions that can be executed when rules match.

use super::errors::PolicyError;
use super::rule_engine::{Event, RuleMatch};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Action types that can be executed when a rule matches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Block the action
    Block {
        reason: String,
    },

    /// Warn but allow
    Warn {
        message: String,
    },

    /// Log an event
    Log {
        level: String,
        message: String,
    },

    /// Send a notification
    Notify {
        channel: String,
        message: String,
    },

    /// Require human approval
    RequireApproval {
        approvers: Vec<String>,
    },

    /// Modify a field value
    Modify {
        field: String,
        value: Value,
    },

    /// Rate limit
    RateLimit {
        /// Maximum requests
        max_requests: u64,
        /// Time window in seconds
        window_secs: u64,
    },

    /// Delay execution
    Delay {
        /// Delay in milliseconds
        delay_ms: u64,
    },

    /// Add metadata to the event
    AddMetadata {
        key: String,
        value: Value,
    },

    /// Execute a webhook
    Webhook {
        url: String,
        #[serde(default)]
        method: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },

    /// Custom action
    Custom {
        name: String,
        #[serde(default)]
        params: HashMap<String, Value>,
    },
}

impl Action {
    /// Create a block action
    pub fn block(reason: impl Into<String>) -> Self {
        Self::Block {
            reason: reason.into(),
        }
    }

    /// Create a warn action
    pub fn warn(message: impl Into<String>) -> Self {
        Self::Warn {
            message: message.into(),
        }
    }

    /// Create a log action
    pub fn log(level: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Log {
            level: level.into(),
            message: message.into(),
        }
    }

    /// Create a notify action
    pub fn notify(channel: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Notify {
            channel: channel.into(),
            message: message.into(),
        }
    }

    /// Create a require approval action
    pub fn require_approval(approvers: Vec<String>) -> Self {
        Self::RequireApproval { approvers }
    }

    /// Check if this action blocks the event
    pub fn is_blocking(&self) -> bool {
        matches!(self, Action::Block { .. } | Action::RequireApproval { .. })
    }

    /// Get the action type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Action::Block { .. } => "block",
            Action::Warn { .. } => "warn",
            Action::Log { .. } => "log",
            Action::Notify { .. } => "notify",
            Action::RequireApproval { .. } => "require_approval",
            Action::Modify { .. } => "modify",
            Action::RateLimit { .. } => "rate_limit",
            Action::Delay { .. } => "delay",
            Action::AddMetadata { .. } => "add_metadata",
            Action::Webhook { .. } => "webhook",
            Action::Custom { .. } => "custom",
        }
    }
}

/// Context for action execution
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// The original event
    pub event: Event,
    /// Rules that matched
    pub matched_rules: Vec<RuleMatch>,
}

/// Result of action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Action type
    pub action_type: String,
    /// Result message
    pub message: String,
    /// Additional data
    #[serde(default)]
    pub data: HashMap<String, Value>,
}

impl ActionResult {
    /// Create a successful result
    pub fn success(action_type: &str, message: impl Into<String>) -> Self {
        Self {
            success: true,
            action_type: action_type.to_string(),
            message: message.into(),
            data: HashMap::new(),
        }
    }

    /// Create a failed result
    pub fn failure(action_type: &str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            action_type: action_type.to_string(),
            message: message.into(),
            data: HashMap::new(),
        }
    }

    /// Add data to the result
    pub fn with_data(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
        self
    }
}

/// Action executor
pub struct ActionExecutor {
    /// Rate limiter state
    rate_limits: std::sync::RwLock<HashMap<String, RateLimitState>>,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    count: u64,
    window_start: std::time::Instant,
    window_secs: u64,
}

impl ActionExecutor {
    /// Create a new action executor
    pub fn new() -> Self {
        Self {
            rate_limits: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Execute an action
    pub async fn execute(
        &self,
        action: &Action,
        context: &ActionContext,
    ) -> Result<ActionResult, PolicyError> {
        match action {
            Action::Block { reason } => self.execute_block(reason),

            Action::Warn { message } => self.execute_warn(message, context),

            Action::Log { level, message } => self.execute_log(level, message, context),

            Action::Notify { channel, message } => self.execute_notify(channel, message, context),

            Action::RequireApproval { approvers } => self.execute_require_approval(approvers),

            Action::Modify { field, value } => self.execute_modify(field, value),

            Action::RateLimit {
                max_requests,
                window_secs,
            } => self.execute_rate_limit(*max_requests, *window_secs, context),

            Action::Delay { delay_ms } => self.execute_delay(*delay_ms).await,

            Action::AddMetadata { key, value } => self.execute_add_metadata(key, value),

            Action::Webhook {
                url,
                method,
                headers,
            } => self.execute_webhook(url, method, headers, context).await,

            Action::Custom { name, params } => self.execute_custom(name, params, context),
        }
    }

    fn execute_block(&self, reason: &str) -> Result<ActionResult, PolicyError> {
        tracing::info!("Blocking action: {}", reason);
        Ok(ActionResult::success("block", reason))
    }

    fn execute_warn(&self, message: &str, context: &ActionContext) -> Result<ActionResult, PolicyError> {
        let formatted = self.format_message(message, context);
        tracing::warn!("Policy warning: {}", formatted);
        Ok(ActionResult::success("warn", formatted))
    }

    fn execute_log(
        &self,
        level: &str,
        message: &str,
        context: &ActionContext,
    ) -> Result<ActionResult, PolicyError> {
        let formatted = self.format_message(message, context);

        match level.to_lowercase().as_str() {
            "error" => tracing::error!("Policy: {}", formatted),
            "warn" | "warning" => tracing::warn!("Policy: {}", formatted),
            "info" => tracing::info!("Policy: {}", formatted),
            "debug" => tracing::debug!("Policy: {}", formatted),
            "trace" => tracing::trace!("Policy: {}", formatted),
            _ => tracing::info!("Policy: {}", formatted),
        }

        Ok(ActionResult::success("log", formatted))
    }

    fn execute_notify(
        &self,
        channel: &str,
        message: &str,
        context: &ActionContext,
    ) -> Result<ActionResult, PolicyError> {
        let formatted = self.format_message(message, context);
        tracing::info!("Notification to {}: {}", channel, formatted);

        // In a real implementation, this would send to Slack, email, etc.
        Ok(ActionResult::success("notify", formatted)
            .with_data("channel", channel))
    }

    fn execute_require_approval(&self, approvers: &[String]) -> Result<ActionResult, PolicyError> {
        tracing::info!("Requiring approval from: {:?}", approvers);
        Ok(ActionResult::success("require_approval", "Approval required")
            .with_data("approvers", approvers))
    }

    fn execute_modify(&self, field: &str, value: &Value) -> Result<ActionResult, PolicyError> {
        tracing::debug!("Modifying field {}: {:?}", field, value);
        Ok(ActionResult::success("modify", format!("Modified field {}", field))
            .with_data("field", field)
            .with_data("value", value.clone()))
    }

    fn execute_rate_limit(
        &self,
        max_requests: u64,
        window_secs: u64,
        context: &ActionContext,
    ) -> Result<ActionResult, PolicyError> {
        // Generate a key based on event type and some identifier
        let key = format!(
            "rate_limit:{:?}",
            context.event.event_type
        );

        let now = std::time::Instant::now();

        let mut limits = self.rate_limits.write().unwrap();
        let state = limits.entry(key.clone()).or_insert_with(|| RateLimitState {
            count: 0,
            window_start: now,
            window_secs,
        });

        // Reset window if expired
        if now.duration_since(state.window_start).as_secs() > state.window_secs {
            state.count = 0;
            state.window_start = now;
        }

        state.count += 1;

        if state.count > max_requests {
            Ok(ActionResult::failure(
                "rate_limit",
                format!("Rate limit exceeded: {} requests in {} seconds", max_requests, window_secs),
            ))
        } else {
            Ok(ActionResult::success(
                "rate_limit",
                format!("Request {}/{} in window", state.count, max_requests),
            ))
        }
    }

    async fn execute_delay(&self, delay_ms: u64) -> Result<ActionResult, PolicyError> {
        tracing::debug!("Delaying for {}ms", delay_ms);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        Ok(ActionResult::success("delay", format!("Delayed {}ms", delay_ms)))
    }

    fn execute_add_metadata(&self, key: &str, value: &Value) -> Result<ActionResult, PolicyError> {
        tracing::debug!("Adding metadata {}: {:?}", key, value);
        Ok(ActionResult::success("add_metadata", format!("Added metadata {}", key))
            .with_data(key, value.clone()))
    }

    async fn execute_webhook(
        &self,
        url: &str,
        method: &str,
        _headers: &HashMap<String, String>,
        _context: &ActionContext,
    ) -> Result<ActionResult, PolicyError> {
        // In a real implementation, this would make an HTTP request
        tracing::info!("Webhook {} {}", method, url);
        Ok(ActionResult::success("webhook", format!("Webhook sent to {}", url)))
    }

    fn execute_custom(
        &self,
        name: &str,
        _params: &HashMap<String, Value>,
        _context: &ActionContext,
    ) -> Result<ActionResult, PolicyError> {
        // Custom actions are not implemented by default
        Err(PolicyError::action(format!(
            "Custom action '{}' not implemented",
            name
        )))
    }

    /// Format a message template with event data
    fn format_message(&self, template: &str, context: &ActionContext) -> String {
        let mut result = template.to_string();

        // Replace {field} placeholders with actual values
        for (key, value) in &context.event.data {
            let placeholder = format!("{{{}}}", key);
            let replacement = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => serde_json::to_string(value).unwrap_or_default(),
            };
            result = result.replace(&placeholder, &replacement);
        }

        // Replace nested placeholders like {arguments.command}
        for (key, value) in &context.event.data {
            if let Value::Object(obj) = value {
                for (nested_key, nested_value) in obj {
                    let placeholder = format!("{{{}.{}}}", key, nested_key);
                    let replacement = match nested_value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => serde_json::to_string(nested_value).unwrap_or_default(),
                    };
                    result = result.replace(&placeholder, &replacement);
                }
            }
        }

        result
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policies::rule_engine::EventType;

    fn create_test_context() -> ActionContext {
        let event = Event::new(EventType::ToolExecution)
            .with_data("tool_name", "bash")
            .with_data("command", "rm -rf /tmp/test")
            .with_data("arguments", serde_json::json!({"cmd": "test-cmd"}));

        ActionContext {
            event,
            matched_rules: vec![],
        }
    }

    #[test]
    fn test_action_builders() {
        let block = Action::block("Blocked reason");
        assert!(block.is_blocking());
        assert_eq!(block.type_name(), "block");

        let warn = Action::warn("Warning message");
        assert!(!warn.is_blocking());
        assert_eq!(warn.type_name(), "warn");

        let approve = Action::require_approval(vec!["admin".to_string()]);
        assert!(approve.is_blocking());
    }

    #[tokio::test]
    async fn test_execute_block() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::block("Test block");

        let result = executor.execute(&action, &context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.action_type, "block");
    }

    #[tokio::test]
    async fn test_execute_warn() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::warn("Test warning for {tool_name}");

        let result = executor.execute(&action, &context).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("bash"));
    }

    #[tokio::test]
    async fn test_execute_log() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::log("info", "Logged: {tool_name}");

        let result = executor.execute(&action, &context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_execute_notify() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::notify("slack", "Alert: {tool_name}");

        let result = executor.execute(&action, &context).await.unwrap();
        assert!(result.success);
        assert!(result.data.contains_key("channel"));
    }

    #[tokio::test]
    async fn test_execute_require_approval() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::require_approval(vec!["admin".to_string(), "security".to_string()]);

        let result = executor.execute(&action, &context).await.unwrap();
        assert!(result.success);
        assert!(result.data.contains_key("approvers"));
    }

    #[tokio::test]
    async fn test_execute_rate_limit() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::RateLimit {
            max_requests: 2,
            window_secs: 60,
        };

        // First two requests should succeed
        let result1 = executor.execute(&action, &context).await.unwrap();
        assert!(result1.success);

        let result2 = executor.execute(&action, &context).await.unwrap();
        assert!(result2.success);

        // Third request should fail (rate limited)
        let result3 = executor.execute(&action, &context).await.unwrap();
        assert!(!result3.success);
    }

    #[tokio::test]
    async fn test_execute_delay() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::Delay { delay_ms: 10 };

        let start = std::time::Instant::now();
        let result = executor.execute(&action, &context).await.unwrap();
        let elapsed = start.elapsed();

        assert!(result.success);
        assert!(elapsed.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_execute_add_metadata() {
        let executor = ActionExecutor::new();
        let context = create_test_context();
        let action = Action::AddMetadata {
            key: "processed".to_string(),
            value: Value::Bool(true),
        };

        let result = executor.execute(&action, &context).await.unwrap();
        assert!(result.success);
        assert!(result.data.contains_key("processed"));
    }

    #[test]
    fn test_format_message() {
        let executor = ActionExecutor::new();
        let context = create_test_context();

        let result = executor.format_message("Tool: {tool_name}, Cmd: {command}", &context);
        assert_eq!(result, "Tool: bash, Cmd: rm -rf /tmp/test");
    }

    #[test]
    fn test_format_message_nested() {
        let executor = ActionExecutor::new();
        let context = create_test_context();

        let result = executor.format_message("Arg: {arguments.cmd}", &context);
        assert_eq!(result, "Arg: test-cmd");
    }

    #[test]
    fn test_action_result_builder() {
        let result = ActionResult::success("test", "Test message")
            .with_data("key1", "value1")
            .with_data("key2", 42);

        assert!(result.success);
        assert_eq!(result.action_type, "test");
        assert!(result.data.contains_key("key1"));
        assert!(result.data.contains_key("key2"));
    }
}
