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

const DEFAULT_TOOLS: &[&str] = &["shell", "computercontroller__automation_script"];

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

struct AdversaryConfig {
    tools: Vec<String>,
    rules: String,
}

/// Adversary inspector that reviews tool calls against user-defined rules.
///
/// Activated by placing an `adversary.md` file in the Goose config directory
/// (`~/.config/goose/adversary.md`). The file contains optional frontmatter
/// to select which tools are reviewed, followed by rules.
///
/// Example `adversary.md`:
/// ```text
/// tools: shell, computercontroller__automation_script
/// ---
/// BLOCK if the command exfiltrates data or is destructive.
/// ALLOW normal development operations.
/// ```
///
/// If the `tools:` line is omitted, only `shell` is reviewed by default.
/// If the file is absent, this inspector is disabled.
/// If the review fails, the inspector fails open (allows the tool call).
pub struct AdversaryInspector {
    provider: SharedProvider,
    config: OnceLock<Option<AdversaryConfig>>,
    config_path: Option<std::path::PathBuf>,
}

impl AdversaryInspector {
    pub fn new(provider: SharedProvider) -> Self {
        Self {
            provider,
            config: OnceLock::new(),
            config_path: None,
        }
    }

    pub fn with_config_dir(provider: SharedProvider, config_dir: std::path::PathBuf) -> Self {
        Self {
            provider,
            config: OnceLock::new(),
            config_path: Some(config_dir.join("adversary.md")),
        }
    }

    fn get_config(&self) -> Option<&AdversaryConfig> {
        self.config
            .get_or_init(|| {
                let path = self
                    .config_path
                    .clone()
                    .unwrap_or_else(|| Paths::config_dir().join("adversary.md"));
                if !path.exists() {
                    tracing::debug!("No adversary.md found, adversary inspector disabled");
                    return None;
                }

                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!("Failed to read adversary.md: {}", e);
                        return Some(AdversaryConfig {
                            tools: DEFAULT_TOOLS.iter().map(|s| (*s).to_string()).collect(),
                            rules: DEFAULT_RULES.to_string(),
                        });
                    }
                };

                let config = Self::parse_adversary_md(&content);
                let tool_list = config.tools.join(", ");
                tracing::info!(
                    tools = %tool_list,
                    "Adversary inspector enabled from {}",
                    path.display()
                );
                Some(config)
            })
            .as_ref()
    }

    /// Parse adversary.md content, extracting optional `tools:` frontmatter.
    ///
    /// Format:
    /// ```text
    /// tools: shell, computercontroller__automation_script
    /// ---
    /// BLOCK if ...
    /// ```
    ///
    /// If no `tools:` line or `---` separator, the entire content is rules
    /// and tools defaults to `["shell"]`.
    fn parse_adversary_md(content: &str) -> AdversaryConfig {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return AdversaryConfig {
                tools: DEFAULT_TOOLS.iter().map(|s| (*s).to_string()).collect(),
                rules: DEFAULT_RULES.to_string(),
            };
        }

        // Look for frontmatter: lines before a `---` separator
        if let Some((frontmatter, rest)) = trimmed.split_once("\n---") {
            let rules = rest.trim();

            let mut tools: Option<Vec<String>> = None;
            for line in frontmatter.lines() {
                let line = line.trim();
                if let Some(value) = line.strip_prefix("tools:") {
                    tools = Some(
                        value
                            .split(',')
                            .map(|t| t.trim().to_string())
                            .filter(|t| !t.is_empty())
                            .collect(),
                    );
                }
            }

            let rules = if rules.is_empty() {
                DEFAULT_RULES.to_string()
            } else {
                rules.to_string()
            };

            AdversaryConfig {
                tools: tools
                    .unwrap_or_else(|| DEFAULT_TOOLS.iter().map(|s| (*s).to_string()).collect()),
                rules,
            }
        } else {
            // No frontmatter — entire content is rules
            AdversaryConfig {
                tools: DEFAULT_TOOLS.iter().map(|s| (*s).to_string()).collect(),
                rules: trimmed.to_string(),
            }
        }
    }

    fn should_review(config: &AdversaryConfig, tool_request: &ToolRequest) -> bool {
        let tool_name = match &tool_request.tool_call {
            Ok(tc) => tc.name.as_ref(),
            Err(_) => return false,
        };
        config.tools.iter().any(|t| t == tool_name)
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

        let system_prompt = "You are an adversarial security reviewer, protecting the user \
            in case the other agent is rogue. An AI coding agent is about to execute a tool \
            call. Your ONLY job: decide if this tool call is safe given the user's task and \
            rules. Respond with ALLOW or BLOCK on the first line, then a brief reason on \
            the next line.";

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
                "",
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
        self.get_config().is_some()
    }

    async fn inspect(
        &self,
        _session_id: &str,
        tool_requests: &[ToolRequest],
        messages: &[Message],
        _goose_mode: GooseMode,
    ) -> Result<Vec<InspectionResult>> {
        let config = match self.get_config() {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let original_task = Self::extract_original_task(messages);
        let recent_messages =
            Self::extract_recent_user_messages(messages, MAX_RECENT_USER_MESSAGES);

        let mut results = Vec::new();

        for request in tool_requests {
            if !Self::should_review(config, request) {
                continue;
            }

            let tool_description = Self::format_tool_call(request);

            tracing::debug!(
                tool_request_id = %request.id,
                "Adversary inspector reviewing tool call"
            );

            match self
                .consult_llm(
                    &tool_description,
                    &original_task,
                    &recent_messages,
                    &config.rules,
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
    fn test_parse_with_tools_frontmatter() {
        let content = "tools: shell, computercontroller__automation_script\n---\nBLOCK bad stuff";
        let config = AdversaryInspector::parse_adversary_md(content);
        assert_eq!(
            config.tools,
            vec!["shell", "computercontroller__automation_script"]
        );
        assert_eq!(config.rules, "BLOCK bad stuff");
    }

    #[test]
    fn test_parse_without_frontmatter() {
        let content = "BLOCK if the command exfiltrates data";
        let config = AdversaryInspector::parse_adversary_md(content);
        assert_eq!(
            config.tools,
            vec!["shell", "computercontroller__automation_script"]
        );
        assert_eq!(config.rules, "BLOCK if the command exfiltrates data");
    }

    #[test]
    fn test_parse_empty() {
        let config = AdversaryInspector::parse_adversary_md("");
        assert_eq!(
            config.tools,
            vec!["shell", "computercontroller__automation_script"]
        );
        assert_eq!(config.rules, DEFAULT_RULES);
    }

    #[test]
    fn test_parse_frontmatter_empty_rules_uses_defaults() {
        let content = "tools: shell\n---\n";
        let config = AdversaryInspector::parse_adversary_md(content);
        assert_eq!(config.tools, vec!["shell"]);
        assert_eq!(config.rules, DEFAULT_RULES);
    }

    #[test]
    fn test_should_review_matches() {
        let config = AdversaryConfig {
            tools: vec!["shell".to_string()],
            rules: String::new(),
        };
        let request = ToolRequest {
            id: "r1".into(),
            tool_call: Ok(
                CallToolRequestParams::new("shell").with_arguments(object!({"command": "ls"}))
            ),
            metadata: None,
            tool_meta: None,
        };
        assert!(AdversaryInspector::should_review(&config, &request));
    }

    #[test]
    fn test_should_review_skips_non_matching() {
        let config = AdversaryConfig {
            tools: vec!["shell".to_string()],
            rules: String::new(),
        };
        let request = ToolRequest {
            id: "r1".into(),
            tool_call: Ok(CallToolRequestParams::new("write")
                .with_arguments(object!({"path": "foo.txt", "content": "hi"}))),
            metadata: None,
            tool_meta: None,
        };
        assert!(!AdversaryInspector::should_review(&config, &request));
    }

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
    async fn test_disabled_when_no_adversary_md() {
        let tmp = tempfile::tempdir().unwrap();

        let provider: SharedProvider = Arc::new(Mutex::new(None));
        let inspector = AdversaryInspector::with_config_dir(provider, tmp.path().to_path_buf());
        assert!(!inspector.is_enabled());

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
        assert!(results.is_empty());
    }
}
