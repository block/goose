//! Hook Manager - Central coordination for hook execution

use super::events::HookEvent;
use super::handlers::{HookDecision, HookHandler, HookResult};
use super::logging::HookLogger;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for a hook event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEventConfig {
    pub handlers: Vec<HookHandler>,
}

/// Overall hook configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookConfig {
    #[serde(rename = "Setup")]
    pub setup: Option<Vec<HookEventConfig>>,

    #[serde(rename = "SessionStart")]
    pub session_start: Option<Vec<HookEventConfig>>,

    #[serde(rename = "UserPromptSubmit")]
    pub user_prompt_submit: Option<Vec<HookEventConfig>>,

    #[serde(rename = "PreToolUse")]
    pub pre_tool_use: Option<Vec<HookEventConfig>>,

    #[serde(rename = "PermissionRequest")]
    pub permission_request: Option<Vec<HookEventConfig>>,

    #[serde(rename = "PostToolUse")]
    pub post_tool_use: Option<Vec<HookEventConfig>>,

    #[serde(rename = "PostToolUseFailure")]
    pub post_tool_use_failure: Option<Vec<HookEventConfig>>,

    #[serde(rename = "Notification")]
    pub notification: Option<Vec<HookEventConfig>>,

    #[serde(rename = "SubagentStart")]
    pub subagent_start: Option<Vec<HookEventConfig>>,

    #[serde(rename = "SubagentStop")]
    pub subagent_stop: Option<Vec<HookEventConfig>>,

    #[serde(rename = "Stop")]
    pub stop: Option<Vec<HookEventConfig>>,

    #[serde(rename = "PreCompact")]
    pub pre_compact: Option<Vec<HookEventConfig>>,

    #[serde(rename = "SessionEnd")]
    pub session_end: Option<Vec<HookEventConfig>>,
}

impl HookConfig {
    pub fn get_handlers(&self, event_name: &str) -> Vec<&HookHandler> {
        let configs = match event_name {
            "Setup" => &self.setup,
            "SessionStart" => &self.session_start,
            "UserPromptSubmit" => &self.user_prompt_submit,
            "PreToolUse" => &self.pre_tool_use,
            "PermissionRequest" => &self.permission_request,
            "PostToolUse" => &self.post_tool_use,
            "PostToolUseFailure" => &self.post_tool_use_failure,
            "Notification" => &self.notification,
            "SubagentStart" => &self.subagent_start,
            "SubagentStop" => &self.subagent_stop,
            "Stop" => &self.stop,
            "PreCompact" => &self.pre_compact,
            "SessionEnd" => &self.session_end,
            _ => return vec![],
        };

        configs
            .as_ref()
            .map(|c| c.iter().flat_map(|ec| ec.handlers.iter()).collect())
            .unwrap_or_default()
    }
}

/// Central manager for hook execution
pub struct HookManager {
    config: Arc<RwLock<HookConfig>>,
    logger: Arc<HookLogger>,
    run_id: String,
    enabled: bool,
}

impl HookManager {
    pub fn new(run_id: impl Into<String>, log_dir: impl Into<PathBuf>) -> Self {
        let run_id = run_id.into();
        let log_dir = log_dir.into();
        Self {
            config: Arc::new(RwLock::new(HookConfig::default())),
            logger: Arc::new(HookLogger::new(&run_id, log_dir)),
            run_id,
            enabled: true,
        }
    }

    pub fn with_config(mut self, config: HookConfig) -> Self {
        self.config = Arc::new(RwLock::new(config));
        self
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Load configuration from a settings file
    pub async fn load_config(&self, path: &PathBuf) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: HookConfig = serde_json::from_str(&content)?;
        let mut current = self.config.write().await;
        *current = config;
        Ok(())
    }

    /// Fire a hook event and get all results
    pub async fn fire(&self, event: HookEvent) -> Vec<HookResult> {
        if !self.enabled {
            return vec![];
        }

        let event_name = event.event_name();
        let config = self.config.read().await;
        let handlers: Vec<HookHandler> = config
            .get_handlers(event_name)
            .into_iter()
            .filter(|h| h.matches(&event))
            .cloned()
            .collect();
        drop(config);

        if handlers.is_empty() {
            return vec![];
        }

        // Log hook start
        let _ = self.logger.log_event_start(&event).await;

        // Execute all handlers in parallel
        let mut results = Vec::new();
        let mut handles = Vec::new();

        for handler in handlers {
            let event_clone = event.clone();
            let logger = self.logger.clone();

            if handler.async_execution {
                // Fire and forget for async handlers
                tokio::spawn(async move {
                    let result = handler.execute(&event_clone).await;
                    if let Ok(ref r) = result {
                        let _ = logger.log_handler_result(&event_clone, r).await;
                    }
                });
            } else {
                // Collect sync handler futures
                let handle = tokio::spawn(async move {
                    let result = handler.execute(&event_clone).await;
                    (event_clone, result)
                });
                handles.push(handle);
            }
        }

        // Wait for sync handlers
        #[allow(clippy::collapsible_match)]
        for handle in handles {
            if let Ok((evt, result)) = handle.await {
                if let Ok(r) = result {
                    let _ = self.logger.log_handler_result(&evt, &r).await;
                    results.push(r);
                }
            }
        }

        // Log hook end
        let _ = self.logger.log_event_end(&event, &results).await;

        results
    }

    /// Check if any result indicates blocking
    pub fn should_block(&self, results: &[HookResult]) -> bool {
        results.iter().any(|r| r.should_block())
    }

    /// Get combined additional context from results
    pub fn get_context(&self, results: &[HookResult]) -> Option<String> {
        let contexts: Vec<String> = results
            .iter()
            .filter_map(|r| r.get_additional_context())
            .collect();

        if contexts.is_empty() {
            None
        } else {
            Some(contexts.join("\n"))
        }
    }

    /// Get the blocking reason from results
    pub fn get_block_reason(&self, results: &[HookResult]) -> Option<String> {
        for result in results {
            if let HookDecision::Block { reason } = &result.decision {
                return Some(reason.clone());
            }
            if result.is_blocking_error() {
                return Some(result.stderr.clone());
            }
        }
        None
    }

    /// Fire PreToolUse hook and check if tool should be blocked
    pub async fn check_pre_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        session_id: &str,
        cwd: &str,
    ) -> (bool, Option<String>, Option<String>) {
        let event = HookEvent::PreToolUse {
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_use_id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
            permission_mode: "default".to_string(),
        };

        let results = self.fire(event).await;
        let should_block = self.should_block(&results);
        let block_reason = self.get_block_reason(&results);
        let context = self.get_context(&results);

        (should_block, block_reason, context)
    }

    /// Fire PostToolUse hook
    pub async fn fire_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        tool_response: &serde_json::Value,
        session_id: &str,
        cwd: &str,
    ) -> Vec<HookResult> {
        let event = HookEvent::PostToolUse {
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_response: tool_response.clone(),
            tool_use_id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
            permission_mode: "default".to_string(),
        };

        self.fire(event).await
    }

    /// Fire Stop hook and check if stop should be blocked
    pub async fn check_stop(&self, session_id: &str, cwd: &str) -> (bool, Option<String>) {
        let event = HookEvent::Stop {
            stop_hook_active: true,
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
            permission_mode: "default".to_string(),
        };

        let results = self.fire(event).await;
        let should_block = self.should_block(&results);
        let block_reason = self.get_block_reason(&results);

        (should_block, block_reason)
    }

    /// Fire Setup hook (on init or maintenance)
    pub async fn fire_setup(&self, trigger: &str, session_id: &str, cwd: &str) -> Vec<HookResult> {
        let event = HookEvent::Setup {
            trigger: trigger.to_string(),
            session_id: session_id.to_string(),
            cwd: cwd.to_string(),
        };
        self.fire(event).await
    }

    /// Fire SessionStart hook
    #[allow(clippy::too_many_arguments)]
    pub async fn fire_session_start(
        &self,
        source: super::events::SessionSource,
        session_id: &str,
        transcript_path: &str,
        cwd: &str,
        permission_mode: &str,
        model: Option<String>,
        agent_type: Option<String>,
    ) -> Vec<HookResult> {
        let event = HookEvent::SessionStart {
            source,
            session_id: session_id.to_string(),
            transcript_path: transcript_path.to_string(),
            cwd: cwd.to_string(),
            permission_mode: permission_mode.to_string(),
            model,
            agent_type,
        };
        self.fire(event).await
    }

    /// Fire UserPromptSubmit hook
    pub async fn fire_user_prompt_submit(
        &self,
        prompt: &str,
        session_id: &str,
        cwd: &str,
    ) -> (bool, Option<String>) {
        let event = HookEvent::UserPromptSubmit {
            prompt: prompt.to_string(),
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
            permission_mode: "default".to_string(),
        };
        let results = self.fire(event).await;
        let should_block = self.should_block(&results);
        let block_reason = self.get_block_reason(&results);
        (should_block, block_reason)
    }

    /// Fire PermissionRequest hook
    pub async fn fire_permission_request(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        session_id: &str,
        cwd: &str,
    ) -> Vec<HookResult> {
        let event = HookEvent::PermissionRequest {
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_use_id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
            permission_mode: "default".to_string(),
        };
        self.fire(event).await
    }

    /// Fire PostToolUseFailure hook
    pub async fn fire_post_tool_use_failure(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
        error: &str,
        session_id: &str,
        cwd: &str,
    ) -> Vec<HookResult> {
        let event = HookEvent::PostToolUseFailure {
            tool_name: tool_name.to_string(),
            tool_input: tool_input.clone(),
            tool_use_id: uuid::Uuid::new_v4().to_string(),
            error: error.to_string(),
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
            permission_mode: "default".to_string(),
        };
        self.fire(event).await
    }

    /// Fire Notification hook
    pub async fn fire_notification(
        &self,
        message: &str,
        session_id: &str,
        cwd: &str,
    ) -> Vec<HookResult> {
        let event = HookEvent::Notification {
            message: message.to_string(),
            session_id: session_id.to_string(),
            cwd: cwd.to_string(),
        };
        self.fire(event).await
    }

    /// Fire SubagentStart hook
    pub async fn fire_subagent_start(
        &self,
        agent_id: &str,
        agent_type: &str,
        session_id: &str,
        cwd: &str,
    ) -> Vec<HookResult> {
        let event = HookEvent::SubagentStart {
            agent_id: agent_id.to_string(),
            agent_type: agent_type.to_string(),
            session_id: session_id.to_string(),
            cwd: cwd.to_string(),
        };
        self.fire(event).await
    }

    /// Fire SubagentStop hook and check if stop should be blocked
    pub async fn check_subagent_stop(
        &self,
        agent_id: &str,
        session_id: &str,
        cwd: &str,
    ) -> (bool, Option<String>) {
        let event = HookEvent::SubagentStop {
            agent_id: agent_id.to_string(),
            stop_hook_active: true,
            session_id: session_id.to_string(),
            cwd: cwd.to_string(),
        };
        let results = self.fire(event).await;
        let should_block = self.should_block(&results);
        let block_reason = self.get_block_reason(&results);
        (should_block, block_reason)
    }

    /// Fire PreCompact hook
    pub async fn fire_pre_compact(
        &self,
        trigger: super::events::CompactTrigger,
        custom_instructions: Option<String>,
        session_id: &str,
        cwd: &str,
    ) -> Vec<HookResult> {
        let event = HookEvent::PreCompact {
            trigger,
            custom_instructions,
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
        };
        self.fire(event).await
    }

    /// Fire SessionEnd hook
    pub async fn fire_session_end(
        &self,
        reason: super::events::SessionEndReason,
        session_id: &str,
        cwd: &str,
    ) -> Vec<HookResult> {
        let event = HookEvent::SessionEnd {
            reason,
            session_id: session_id.to_string(),
            transcript_path: String::new(),
            cwd: cwd.to_string(),
            permission_mode: "default".to_string(),
        };
        self.fire(event).await
    }

    /// Get the run ID
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Get statistics about hook executions
    pub async fn get_stats(&self) -> HookStats {
        self.logger.get_stats().await
    }
}

/// Statistics about hook executions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookStats {
    pub total_executions: usize,
    pub successful: usize,
    pub blocked: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub by_event: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::super::handlers::HandlerType;
    use super::*;

    #[test]
    fn test_hook_config_get_handlers() {
        let config = HookConfig {
            pre_tool_use: Some(vec![HookEventConfig {
                handlers: vec![HookHandler {
                    handler_type: HandlerType::Command,
                    command: Some("echo test".to_string()),
                    script_path: None,
                    timeout_secs: 60,
                    async_execution: false,
                    matcher: None,
                }],
            }]),
            ..Default::default()
        };

        let handlers = config.get_handlers("PreToolUse");
        assert_eq!(handlers.len(), 1);

        let handlers = config.get_handlers("PostToolUse");
        assert_eq!(handlers.len(), 0);
    }

    #[tokio::test]
    async fn test_hook_manager_disabled() {
        let mut manager = HookManager::new("test-run", std::env::temp_dir());
        manager.disable();

        let event = HookEvent::UserPromptSubmit {
            prompt: "test".to_string(),
            session_id: "session".to_string(),
            transcript_path: "/path".to_string(),
            cwd: "/cwd".to_string(),
            permission_mode: "default".to_string(),
        };

        let results = manager.fire(event).await;
        assert!(results.is_empty());
    }

    #[test]
    fn test_hook_manager_should_block() {
        let manager = HookManager::new("test-run", std::env::temp_dir());

        let results = vec![HookResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 100,
            output: None,
            decision: HookDecision::Continue,
            timed_out: false,
        }];
        assert!(!manager.should_block(&results));

        let results = vec![HookResult {
            exit_code: 2,
            stdout: String::new(),
            stderr: "Blocked".to_string(),
            duration_ms: 100,
            output: None,
            decision: HookDecision::Block {
                reason: "Test".to_string(),
            },
            timed_out: false,
        }];
        assert!(manager.should_block(&results));
    }

    #[test]
    fn test_hook_manager_get_context() {
        let manager = HookManager::new("test-run", std::env::temp_dir());

        let results = vec![
            HookResult {
                exit_code: 0,
                stdout: "Context 1".to_string(),
                stderr: String::new(),
                duration_ms: 100,
                output: None,
                decision: HookDecision::Continue,
                timed_out: false,
            },
            HookResult {
                exit_code: 0,
                stdout: "Context 2".to_string(),
                stderr: String::new(),
                duration_ms: 100,
                output: None,
                decision: HookDecision::Continue,
                timed_out: false,
            },
        ];

        let context = manager.get_context(&results);
        assert!(context.is_some());
        assert!(context.unwrap().contains("Context 1"));
    }
}
