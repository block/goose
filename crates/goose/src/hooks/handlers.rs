//! Hook Handlers - Execute commands and process results

use super::events::HookEvent;
use super::{exit_codes, HookOutput};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Matcher for filtering which events trigger a hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    /// Match specific tool names (for PreToolUse, PostToolUse, etc.)
    pub tool_names: Option<Vec<String>>,

    /// Match tool name patterns (regex)
    pub tool_pattern: Option<String>,

    /// Only match for specific session sources
    pub session_sources: Option<Vec<String>>,
}

impl HookMatcher {
    pub fn matches(&self, event: &HookEvent) -> bool {
        // Check tool name matching
        if let Some(ref tool_names) = self.tool_names {
            if let Some(event_tool) = event.tool_name() {
                if !tool_names.iter().any(|t| t == event_tool) {
                    return false;
                }
            }
        }

        // Check tool pattern matching
        if let Some(ref pattern) = self.tool_pattern {
            if let Some(event_tool) = event.tool_name() {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if !re.is_match(event_tool) {
                        return false;
                    }
                }
            }
        }

        true
    }
}

/// Handler configuration for a hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookHandler {
    /// Type of handler (command, script, prompt, agent)
    #[serde(rename = "type")]
    pub handler_type: HandlerType,

    /// Command to execute (for command type)
    pub command: Option<String>,

    /// Script path (for script type)
    pub script_path: Option<PathBuf>,

    /// Timeout for execution
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,

    /// Whether to run asynchronously
    #[serde(default)]
    pub async_execution: bool,

    /// Matcher for filtering events
    pub matcher: Option<HookMatcher>,
}

fn default_timeout_secs() -> u64 {
    60
}

/// Type of hook handler
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandlerType {
    Command,
    Script,
    Prompt,
    Agent,
}

/// Result from executing a hook handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub output: Option<HookOutput>,
    pub decision: HookDecision,
    pub timed_out: bool,
}

impl HookResult {
    pub fn is_success(&self) -> bool {
        self.exit_code == exit_codes::SUCCESS
    }

    pub fn is_blocking_error(&self) -> bool {
        self.exit_code == exit_codes::BLOCKING_ERROR
    }

    pub fn should_block(&self) -> bool {
        self.is_blocking_error()
            || self
                .output
                .as_ref()
                .map(|o| o.should_block())
                .unwrap_or(false)
            || matches!(self.decision, HookDecision::Block { .. })
    }

    pub fn get_additional_context(&self) -> Option<String> {
        // First check parsed output
        if let Some(ref output) = self.output {
            if let Some(ctx) = output.get_additional_context() {
                return Some(ctx.to_string());
            }
        }

        // Fall back to raw stdout for non-JSON output
        if !self.stdout.trim().is_empty() && self.output.is_none() {
            return Some(self.stdout.clone());
        }

        None
    }
}

/// Decision from a hook
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HookDecision {
    Continue,
    Approve { reason: String },
    Block { reason: String },
    Ask { reason: String },
}

impl Default for HookDecision {
    fn default() -> Self {
        HookDecision::Continue
    }
}

impl HookHandler {
    /// Check if this handler should run for the given event
    pub fn matches(&self, event: &HookEvent) -> bool {
        match &self.matcher {
            Some(matcher) => matcher.matches(event),
            None => true,
        }
    }

    /// Execute this hook handler
    pub async fn execute(&self, event: &HookEvent) -> Result<HookResult> {
        let start = std::time::Instant::now();

        let command = match self.handler_type {
            HandlerType::Command => self
                .command
                .as_ref()
                .ok_or_else(|| anyhow!("Command handler missing command"))?
                .clone(),
            HandlerType::Script => {
                let path = self
                    .script_path
                    .as_ref()
                    .ok_or_else(|| anyhow!("Script handler missing script_path"))?;
                format!("{}", path.display())
            }
            HandlerType::Prompt | HandlerType::Agent => {
                return Err(anyhow!("Prompt and Agent handlers not yet implemented"));
            }
        };

        let timeout = Duration::from_secs(self.timeout_secs);
        let input_json = serde_json::to_string(&event)?;

        // Execute command
        let result = execute_command(&command, &input_json, timeout).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok((exit_code, stdout, stderr, timed_out)) => {
                // Try to parse JSON output
                let output = parse_hook_output(&stdout);
                let decision = determine_decision(exit_code, &output);

                Ok(HookResult {
                    exit_code,
                    stdout,
                    stderr,
                    duration_ms,
                    output,
                    decision,
                    timed_out,
                })
            }
            Err(e) => Ok(HookResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: e.to_string(),
                duration_ms,
                output: None,
                decision: HookDecision::Continue,
                timed_out: false,
            }),
        }
    }
}

/// Execute a shell command with JSON input via stdin
async fn execute_command(
    command: &str,
    input: &str,
    timeout: Duration,
) -> Result<(i32, String, String, bool)> {
    let mut child = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).await?;
    }

    // Wait with timeout
    let result = tokio::time::timeout(timeout, async {
        let output = child.wait_with_output().await?;
        Ok::<_, std::io::Error>(output)
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            let exit_code = output.status.code().unwrap_or(1);
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok((exit_code, stdout, stderr, false))
        }
        Ok(Err(e)) => Err(anyhow!("Command execution failed: {}", e)),
        Err(_) => {
            // Timeout occurred - process may still be running but we can't kill it
            // since wait_with_output would have consumed child
            Ok((1, String::new(), "Command timed out".to_string(), true))
        }
    }
}

/// Parse hook output as JSON
fn parse_hook_output(stdout: &str) -> Option<HookOutput> {
    let trimmed = stdout.trim();
    if trimmed.starts_with('{') {
        serde_json::from_str(trimmed).ok()
    } else {
        None
    }
}

/// Determine the decision based on exit code and output
fn determine_decision(exit_code: i32, output: &Option<HookOutput>) -> HookDecision {
    // Check JSON output first
    if let Some(ref out) = output {
        if out.should_block() {
            return HookDecision::Block {
                reason: out
                    .reason
                    .clone()
                    .unwrap_or_else(|| "Blocked by hook".to_string()),
            };
        }
        if out.should_approve() {
            return HookDecision::Approve {
                reason: out
                    .reason
                    .clone()
                    .unwrap_or_else(|| "Approved by hook".to_string()),
            };
        }
    }

    // Fall back to exit code
    if exit_code == exit_codes::BLOCKING_ERROR {
        return HookDecision::Block {
            reason: "Hook returned blocking error (exit code 2)".to_string(),
        };
    }

    HookDecision::Continue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_matcher_tool_names() {
        let matcher = HookMatcher {
            tool_names: Some(vec!["Bash".to_string(), "Write".to_string()]),
            tool_pattern: None,
            session_sources: None,
        };

        let bash_event = HookEvent::PreToolUse {
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({}),
            tool_use_id: "id".to_string(),
            session_id: "test".to_string(),
            transcript_path: "/path".to_string(),
            cwd: "/cwd".to_string(),
            permission_mode: "default".to_string(),
        };
        assert!(matcher.matches(&bash_event));

        let read_event = HookEvent::PreToolUse {
            tool_name: "Read".to_string(),
            tool_input: serde_json::json!({}),
            tool_use_id: "id".to_string(),
            session_id: "test".to_string(),
            transcript_path: "/path".to_string(),
            cwd: "/cwd".to_string(),
            permission_mode: "default".to_string(),
        };
        assert!(!matcher.matches(&read_event));
    }

    #[test]
    fn test_hook_result_should_block() {
        let result = HookResult {
            exit_code: 2,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 100,
            output: None,
            decision: HookDecision::Continue,
            timed_out: false,
        };
        assert!(result.should_block());

        let result = HookResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 100,
            output: Some(HookOutput {
                decision: Some("block".to_string()),
                ..Default::default()
            }),
            decision: HookDecision::Continue,
            timed_out: false,
        };
        assert!(result.should_block());
    }

    #[test]
    fn test_parse_hook_output() {
        let json = r#"{"decision": "block", "reason": "Test reason"}"#;
        let output = parse_hook_output(json);
        assert!(output.is_some());
        assert!(output.unwrap().should_block());

        let non_json = "Just plain text";
        let output = parse_hook_output(non_json);
        assert!(output.is_none());
    }

    #[test]
    fn test_determine_decision() {
        // Exit code 2 should block
        let decision = determine_decision(2, &None);
        assert!(matches!(decision, HookDecision::Block { .. }));

        // JSON block should block
        let output = Some(HookOutput {
            decision: Some("block".to_string()),
            reason: Some("Test".to_string()),
            ..Default::default()
        });
        let decision = determine_decision(0, &output);
        assert!(matches!(decision, HookDecision::Block { .. }));

        // Success should continue
        let decision = determine_decision(0, &None);
        assert!(matches!(decision, HookDecision::Continue));
    }
}
