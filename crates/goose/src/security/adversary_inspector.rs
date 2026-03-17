use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::OnceLock;

use crate::agents::types::SharedProvider;
use crate::config::paths::Paths;
use crate::config::GooseMode;
use crate::conversation::message::{Message, MessageContent, ToolRequest};
use crate::conversation::Conversation;
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use crate::utils::safe_truncate;

const DEFAULT_RULES: &str = r#"BLOCK if the command:
- Exfiltrates data (curl/wget posting to unknown URLs, piping secrets out)
- Is destructive beyond the project scope (rm -rf /, modifying system files)
- Installs malware or runs obfuscated code
- Attempts to escalate privileges unnecessarily
- Downloads and executes untrusted remote scripts

ALLOW if the command is a normal development operation, even if it modifies files,
installs packages, runs tests, uses git, etc. Most commands are fine.
Err on the side of ALLOW — only block truly dangerous things."#;

const MAX_RECENT_USER_MESSAGES: usize = 4;

/// Adversary inspector that uses an LLM to review tool calls against user-defined rules.
///
/// Activated by placing an `adversary.md` file in the Goose config directory
/// (`~/.config/goose/adversary.md` on macOS/Linux). The file contains rules that
/// the LLM uses to decide whether to ALLOW or BLOCK each tool call.
///
/// If the file is absent, this inspector is disabled and does nothing.
/// If the LLM call fails, the inspector fails open (allows the tool call).
pub struct AdversaryInspector {
    provider: SharedProvider,
    rules: OnceLock<Option<String>>,
}

impl AdversaryInspector {
    pub fn new(provider: SharedProvider) -> Self {
        Self {
            provider,
            rules: OnceLock::new(),
        }
    }

    fn get_rules(&self) -> Option<&str> {
        self.rules
            .get_or_init(|| {
                let path = Paths::config_dir().join("adversary.md");
                if path.exists() {
                    match std::fs::read_to_string(&path) {
                        Ok(content) if !content.trim().is_empty() => {
                            tracing::info!(
                                "Adversary inspector loaded rules from {}",
                                path.display()
                            );
                            Some(content)
                        }
                        Ok(_) => {
                            tracing::info!(
                                "Adversary inspector using default rules (adversary.md is empty)"
                            );
                            Some(DEFAULT_RULES.to_string())
                        }
                        Err(e) => {
                            tracing::warn!("Failed to read adversary.md: {}. Using defaults.", e);
                            Some(DEFAULT_RULES.to_string())
                        }
                    }
                } else {
                    tracing::debug!("No adversary.md found, adversary inspector disabled");
                    None
                }
            })
            .as_deref()
    }

    fn format_tool_call(tool_request: &ToolRequest) -> String {
        match &tool_request.tool_call {
            Ok(tc) => {
                let mut s = format!("Tool: {}", tc.name);
                if let Some(args) = &tc.arguments {
                    if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                        s = format!("Tool: {} — command: {}", tc.name, cmd);
                    } else if let Ok(json) = serde_json::to_string_pretty(args) {
                        s.push_str("\nArguments: ");
                        s.push_str(&json);
                    }
                }
                s
            }
            Err(e) => format!("(malformed tool call: {})", e),
        }
    }

    fn extract_recent_user_messages(messages: &[Message], count: usize) -> Vec<String> {
        messages
            .iter()
            .rev()
            .filter(|m| m.role == rmcp::model::Role::User)
            .filter_map(|m| {
                let text: String = m
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        MessageContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            })
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    fn extract_original_task(messages: &[Message]) -> String {
        for msg in messages {
            if msg.role == rmcp::model::Role::User {
                let text: String = msg
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        MessageContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !text.is_empty() {
                    return safe_truncate(&text, 500);
                }
            }
        }
        "(unknown)".to_string()
    }

    async fn consult_llm(
        &self,
        session_id: &str,
        tool_description: &str,
        original_task: &str,
        recent_messages: &[String],
        rules: &str,
    ) -> Result<(bool, String)> {
        let provider_guard = self.provider.lock().await;
        let provider = match provider_guard.clone() {
            Some(p) => p,
            None => return Ok((true, "No provider available".to_string())),
        };
        drop(provider_guard);

        let history_section = if !recent_messages.is_empty() {
            let mut s = String::from("Recent user messages (oldest first):\n");
            for (i, msg) in recent_messages.iter().enumerate() {
                let truncated = safe_truncate(msg, 200);
                s.push_str(&format!("{}. {}\n", i + 1, truncated));
            }
            s.push('\n');
            s
        } else {
            String::new()
        };

        let user_message = format!(
            "The user's original task: {}\n\n\
             {}\
             The tool call to review:\n\
             ```\n\
             {}\n\
             ```\n\n\
             {}\n\n\
             Respond with a single word on the first line: ALLOW or BLOCK\n\
             Then on the next line, a brief reason.",
            original_task, history_section, tool_description, rules
        );

        let system_prompt = "You are a security reviewer. An AI coding agent is about to \
            execute a tool call. Your ONLY job: decide if this tool call is safe given the \
            user's task and rules. Respond with ALLOW or BLOCK on the first line, then a \
            brief reason on the next line.";

        let check_messages = vec![Message::new(
            rmcp::model::Role::User,
            Utc::now().timestamp(),
            vec![MessageContent::text(user_message)],
        )];
        let conversation = Conversation::new_unvalidated(check_messages);

        let model_config = provider.get_model_config();
        let (response, _usage) = provider
            .complete(
                &model_config,
                session_id,
                system_prompt,
                conversation.messages(),
                &[],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Adversary LLM call failed: {}", e))?;

        let output: String = response
            .content
            .iter()
            .filter_map(|c| match c {
                MessageContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let output = output.trim();
        let upper = output.to_uppercase();

        if upper.starts_with("BLOCK") || upper.contains("\nBLOCK") {
            let reason = output
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let reason = if reason.is_empty() {
                "Blocked by adversary".to_string()
            } else {
                reason
            };
            Ok((false, reason))
        } else {
            let reason = output
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            Ok((true, reason))
        }
    }
}

#[async_trait]
impl ToolInspector for AdversaryInspector {
    fn name(&self) -> &'static str {
        "adversary"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn is_enabled(&self) -> bool {
        self.get_rules().is_some()
    }

    async fn inspect(
        &self,
        session_id: &str,
        tool_requests: &[ToolRequest],
        messages: &[Message],
        _goose_mode: GooseMode,
    ) -> Result<Vec<InspectionResult>> {
        let rules = match self.get_rules() {
            Some(r) => r,
            None => return Ok(vec![]),
        };

        let original_task = Self::extract_original_task(messages);
        let recent_messages =
            Self::extract_recent_user_messages(messages, MAX_RECENT_USER_MESSAGES);

        let mut results = Vec::new();

        for request in tool_requests {
            let tool_description = Self::format_tool_call(request);

            tracing::debug!(
                tool_request_id = %request.id,
                "Adversary inspector reviewing tool call"
            );

            match self
                .consult_llm(
                    session_id,
                    &tool_description,
                    &original_task,
                    &recent_messages,
                    rules,
                )
                .await
            {
                Ok((true, reason)) => {
                    tracing::debug!(
                        tool_request_id = %request.id,
                        reason = %reason,
                        "Adversary: ALLOW"
                    );
                    results.push(InspectionResult {
                        tool_request_id: request.id.clone(),
                        action: InspectionAction::Allow,
                        reason: format!("Adversary: {}", reason),
                        confidence: 1.0,
                        inspector_name: self.name().to_string(),
                        finding_id: None,
                    });
                }
                Ok((false, reason)) => {
                    tracing::warn!(
                        tool_request_id = %request.id,
                        reason = %reason,
                        "Adversary: BLOCK"
                    );
                    results.push(InspectionResult {
                        tool_request_id: request.id.clone(),
                        action: InspectionAction::Deny,
                        reason: format!("🛡️ Adversary blocked: {}", reason),
                        confidence: 1.0,
                        inspector_name: self.name().to_string(),
                        finding_id: None,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        tool_request_id = %request.id,
                        error = %e,
                        "Adversary inspector failed, allowing tool call (fail-open)"
                    );
                    results.push(InspectionResult {
                        tool_request_id: request.id.clone(),
                        action: InspectionAction::Allow,
                        reason: format!("Adversary error (fail-open): {}", e),
                        confidence: 0.0,
                        inspector_name: self.name().to_string(),
                        finding_id: None,
                    });
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolRequestParams;
    use rmcp::object;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_format_tool_call_shell() {
        let request = ToolRequest {
            id: "req1".into(),
            tool_call: Ok(CallToolRequestParams::new("shell")
                .with_arguments(object!({"command": "rm -rf /"}))),
            metadata: None,
            tool_meta: None,
        };
        let formatted = AdversaryInspector::format_tool_call(&request);
        assert!(formatted.contains("shell"));
        assert!(formatted.contains("rm -rf /"));
    }

    #[test]
    fn test_format_tool_call_write() {
        let request = ToolRequest {
            id: "req2".into(),
            tool_call: Ok(CallToolRequestParams::new("write")
                .with_arguments(object!({"path": "/etc/passwd", "content": "hacked"}))),
            metadata: None,
            tool_meta: None,
        };
        let formatted = AdversaryInspector::format_tool_call(&request);
        assert!(formatted.contains("write"));
        assert!(formatted.contains("/etc/passwd"));
    }

    #[test]
    fn test_extract_original_task() {
        let messages = vec![
            Message::new(
                rmcp::model::Role::User,
                Utc::now().timestamp(),
                vec![MessageContent::text("Refactor the auth module")],
            ),
            Message::new(
                rmcp::model::Role::Assistant,
                Utc::now().timestamp(),
                vec![MessageContent::text("Sure, I'll start by...")],
            ),
        ];
        let task = AdversaryInspector::extract_original_task(&messages);
        assert_eq!(task, "Refactor the auth module");
    }

    #[test]
    fn test_extract_recent_user_messages() {
        let messages = vec![
            Message::new(
                rmcp::model::Role::User,
                Utc::now().timestamp(),
                vec![MessageContent::text("First message")],
            ),
            Message::new(
                rmcp::model::Role::Assistant,
                Utc::now().timestamp(),
                vec![MessageContent::text("Response")],
            ),
            Message::new(
                rmcp::model::Role::User,
                Utc::now().timestamp(),
                vec![MessageContent::text("Second message")],
            ),
            Message::new(
                rmcp::model::Role::User,
                Utc::now().timestamp(),
                vec![MessageContent::text("Third message")],
            ),
        ];
        let recent = AdversaryInspector::extract_recent_user_messages(&messages, 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0], "Second message");
        assert_eq!(recent[1], "Third message");
    }

    #[tokio::test]
    async fn test_disabled_when_no_rules_file() {
        let provider: SharedProvider = Arc::new(Mutex::new(None));
        let inspector = AdversaryInspector::new(provider);
        assert!(!inspector.is_enabled() || std::env::var("GOOSE_PATH_ROOT").is_ok());
    }

    #[tokio::test]
    async fn test_inspect_returns_empty_when_disabled() {
        let provider: SharedProvider = Arc::new(Mutex::new(None));
        let inspector = AdversaryInspector::new(provider);

        let request = ToolRequest {
            id: "req1".into(),
            tool_call: Ok(
                CallToolRequestParams::new("shell").with_arguments(object!({"command": "ls"}))
            ),
            metadata: None,
            tool_meta: None,
        };

        let results = inspector
            .inspect("test", &[request], &[], GooseMode::Auto)
            .await
            .unwrap();

        if !inspector.is_enabled() {
            assert!(results.is_empty());
        }
    }
}
