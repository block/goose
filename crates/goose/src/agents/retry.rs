use anyhow::Result;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::subprocess::SubprocessExt;

use crate::agents::types::SessionConfig;
use crate::agents::types::{
    RetryConfig, SuccessCheck, DEFAULT_ON_FAILURE_TIMEOUT_SECONDS, DEFAULT_RETRY_TIMEOUT_SECONDS,
};
use crate::config::Config;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::tool_monitor::RepetitionInspector;

/// Result of a retry logic evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum RetryResult {
    /// No retry configuration or session available, retry logic skipped
    Skipped,
    /// Maximum retry attempts reached, cannot retry further
    MaxAttemptsReached,
    /// Success checks passed, no retry needed
    SuccessChecksPassed,
    /// Retry is needed and will be performed
    Retried,
}

/// Environment variable for configuring retry timeout globally
const GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS: &str = "GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS";

/// Environment variable for configuring on_failure timeout globally
const GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS: &str = "GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS";

/// Maximum number of characters to retain in retry failure feedback to prevent context overflow.
/// 20,000 chars is roughly 5k-10k tokens depending on the model/content.
const MAX_RETRY_OUTPUT_CHARS: usize = 20_000;

/// Manages retry state and operations for agent execution
#[derive(Debug)]
pub struct RetryManager {
    /// Current number of retry attempts
    attempts: Arc<Mutex<u32>>,
    /// Optional repetition inspector for reset operations
    repetition_inspector: Option<Arc<Mutex<Option<RepetitionInspector>>>>,
}

impl Default for RetryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryManager {
    /// Create a new retry manager
    pub fn new() -> Self {
        Self {
            attempts: Arc::new(Mutex::new(0)),
            repetition_inspector: None,
        }
    }

    /// Create a new retry manager with repetition inspector
    pub fn with_repetition_inspector(
        repetition_inspector: Arc<Mutex<Option<RepetitionInspector>>>,
    ) -> Self {
        Self {
            attempts: Arc::new(Mutex::new(0)),
            repetition_inspector: Some(repetition_inspector),
        }
    }

    /// Reset the retry attempts counter to 0
    pub async fn reset_attempts(&self) {
        let mut attempts = self.attempts.lock().await;
        *attempts = 0;

        // Reset repetition inspector if available
        if let Some(inspector) = &self.repetition_inspector {
            if let Some(inspector) = inspector.lock().await.as_mut() {
                inspector.reset();
            }
        }
    }

    /// Increment the retry attempts counter and return the new value
    pub async fn increment_attempts(&self) -> u32 {
        let mut attempts = self.attempts.lock().await;
        *attempts += 1;
        *attempts
    }

    /// Get the current retry attempts count
    pub async fn get_attempts(&self) -> u32 {
        *self.attempts.lock().await
    }

    /// Reset status for retry: clear message history and final output tool state
    async fn reset_status_for_retry(
        messages: &mut Conversation,
        initial_messages: &[Message],
        final_output_tool: &Arc<Mutex<Option<crate::agents::final_output_tool::FinalOutputTool>>>,
    ) {
        *messages = Conversation::new_unvalidated(initial_messages.to_vec());
        info!("Reset message history to initial state for retry");

        let mut guard = final_output_tool.lock().await;
        if let Some(fot) = guard.as_mut() {
            fot.final_output = None;
            info!("Cleared final output tool state for retry");
        }
    }

    pub async fn handle_retry_logic(
        &self,
        messages: &mut Conversation,
        session_config: &SessionConfig,
        initial_messages: &[Message],
        final_output_tool: &Arc<Mutex<Option<crate::agents::final_output_tool::FinalOutputTool>>>,
    ) -> Result<RetryResult> {
        let Some(retry_config) = &session_config.retry_config else {
            return Ok(RetryResult::Skipped);
        };

        let check_result = execute_success_checks(&retry_config.checks, retry_config).await?;

        if let SuccessCheckResult::Passed = check_result {
            info!("All success checks passed, no retry needed");
            return Ok(RetryResult::SuccessChecksPassed);
        }

        let current_attempts = self.get_attempts().await;
        if current_attempts >= retry_config.max_retries {
            let error_msg = Message::assistant().with_text(format!(
                "Maximum retry attempts ({}) exceeded. Unable to complete the task successfully.",
                retry_config.max_retries
            ));
            messages.push(error_msg);
            warn!(
                "Maximum retry attempts ({}) exceeded",
                retry_config.max_retries
            );
            #[cfg(feature = "telemetry")]
            crate::posthog::emit_error(
                "retry_max_exceeded",
                &format!("Max retries ({}) exceeded", retry_config.max_retries),
            );
            return Ok(RetryResult::MaxAttemptsReached);
        }

        let on_failure_details = if let Some(on_failure_cmd) = &retry_config.on_failure {
            Some(execute_on_failure_command(on_failure_cmd, retry_config).await?)
        } else {
            None
        };

        if retry_config.reset_context {
            if let Some(ref details) = on_failure_details {
                if !details.exit_status.success() {
                    return Err(anyhow::anyhow!(
                        "On_failure command '{}' exited with status {}, stderr: {}",
                        details.command,
                        details.exit_status,
                        details.stderr.trim()
                    ));
                }
            }
            Self::reset_status_for_retry(messages, initial_messages, final_output_tool).await;
        } else {
            use std::fmt::Write;
            let mut failure_message = String::new();

            let mut remaining_budget = MAX_RETRY_OUTPUT_CHARS;

            if let SuccessCheckResult::Failed(details) = check_result {
                let _ = writeln!(
                    failure_message,
                    "Validation failed: command '{}' exited with status {}",
                    details.command, details.exit_status
                );
                append_output_details(
                    &mut failure_message,
                    "Validation",
                    &details.stdout,
                    &details.stderr,
                    &mut remaining_budget,
                );
            }

            if let Some(details) = on_failure_details {
                let status_str = if details.exit_status.success() {
                    "successfully"
                } else {
                    "with failure"
                };
                let _ = writeln!(
                    failure_message,
                    "\nRecovery command '{}' executed {}.",
                    details.command, status_str
                );
                append_output_details(
                    &mut failure_message,
                    "Recovery",
                    &details.stdout,
                    &details.stderr,
                    &mut remaining_budget,
                );
            }

            failure_message
                .push_str("\nPlease analyze the outputs above and try again to achieve the goal.");
            messages.push(
                Message::user()
                    .with_text(failure_message.trim())
                    .agent_only(),
            );
            info!(
                "Added comprehensive failure context to conversation history for retry (no reset)"
            );
        }

        let new_attempts = self.increment_attempts().await;
        info!("Incrementing retry attempts to {}", new_attempts);

        Ok(RetryResult::Retried)
    }
}

/// Helper function to append stdout/stderr to a failure message with truncation.
fn append_output_details(
    message: &mut String,
    prefix: &str,
    stdout: &str,
    stderr: &str,
    budget: &mut usize,
) {
    use crate::agents::large_response_handler::write_large_text_to_file;
    use crate::utils::truncate_keep_tail;
    use std::fmt::Write;

    if stdout.is_empty() && stderr.is_empty() {
        return;
    }

    let _ = writeln!(message, "{} output:", prefix);
    for (label, text) in [("stdout", stdout), ("stderr", stderr)] {
        if text.is_empty() {
            continue;
        }

        if *budget == 0 {
            let file_note = match write_large_text_to_file(text) {
                Ok(path) => format!(" [Full {} saved to: {}]", label, path),
                Err(_) => String::new(),
            };
            let _ = writeln!(
                message,
                "  {label}:\n[... omitted: global cap reached ...]{file_note}"
            );
            continue;
        }

        let (kept, omitted) = truncate_keep_tail(text, *budget);
        *budget = budget.saturating_sub(kept.chars().count());

        if omitted > 0 {
            let file_note = match write_large_text_to_file(text) {
                Ok(path) => format!("\n[Full {} saved to: {}]", label, path),
                Err(_) => String::new(),
            };
            let _ = writeln!(
                message,
                "  {label}:\n[... truncated {omitted} chars ...]{file_note}\n{kept}"
            );
        } else {
            let _ = writeln!(message, "  {label}:\n{kept}");
        }
    }
}

/// Get the configured timeout duration for retry operations
/// retry_config.timeout_seconds -> env var -> default
fn get_retry_timeout(retry_config: &RetryConfig) -> Duration {
    let timeout_seconds = retry_config
        .timeout_seconds
        .or_else(|| {
            let config = Config::global();
            config.get_param(GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS).ok()
        })
        .unwrap_or(DEFAULT_RETRY_TIMEOUT_SECONDS);

    Duration::from_secs(timeout_seconds)
}

/// Get the configured timeout duration for on_failure operations
/// retry_config.on_failure_timeout_seconds -> env var -> default
fn get_on_failure_timeout(retry_config: &RetryConfig) -> Duration {
    let timeout_seconds = retry_config
        .on_failure_timeout_seconds
        .or_else(|| {
            let config = Config::global();
            config
                .get_param(GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS)
                .ok()
        })
        .unwrap_or(DEFAULT_ON_FAILURE_TIMEOUT_SECONDS);

    Duration::from_secs(timeout_seconds)
}

/// Detailed result of a command execution
#[derive(Debug, Clone)]
pub struct ExecutionDetails {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_status: std::process::ExitStatus,
}

impl ExecutionDetails {
    pub fn from_output(command: String, output: std::process::Output) -> Self {
        Self {
            command,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_status: output.status,
        }
    }
}

/// Result of executing success checks
#[derive(Debug)]
pub enum SuccessCheckResult {
    /// All checks passed
    Passed,
    /// A check failed with details
    Failed(ExecutionDetails),
}

/// Execute all success checks and return detailed result
pub async fn execute_success_checks(
    checks: &[SuccessCheck],
    retry_config: &RetryConfig,
) -> Result<SuccessCheckResult> {
    let timeout = get_retry_timeout(retry_config);

    for check in checks {
        match check {
            SuccessCheck::Shell { command } => {
                let result = execute_shell_command(command, timeout).await?;
                if !result.status.success() {
                    warn!(
                        "Success check failed: command '{}' exited with status {}, stderr: {}",
                        command,
                        result.status,
                        String::from_utf8_lossy(&result.stderr)
                    );
                    return Ok(SuccessCheckResult::Failed(ExecutionDetails::from_output(
                        command.clone(),
                        result,
                    )));
                }
                info!(
                    "Success check passed: command '{}' completed successfully",
                    command
                );
            }
        }
    }
    Ok(SuccessCheckResult::Passed)
}

/// Execute a shell command with cross-platform compatibility and mandatory timeout
pub async fn execute_shell_command(
    command: &str,
    timeout: std::time::Duration,
) -> Result<std::process::Output> {
    debug!(
        "Executing shell command with timeout {:?}: {}",
        timeout, command
    );

    let future = async {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", command]);
            cmd.env("GOOSE_TERMINAL", "1");
            cmd.env("AGENT", "goose");
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", command]);
            cmd.env("GOOSE_TERMINAL", "1");
            cmd.env("AGENT", "goose");
            cmd
        };

        cmd.set_no_window();

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await?;

        debug!(
            "Shell command completed with status: {}, stdout: {}, stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(output)
    };

    match tokio::time::timeout(timeout, future).await {
        Ok(result) => result,
        Err(_) => {
            let error_msg = format!("Shell command timed out after {:?}: {}", timeout, command);
            warn!("{}", error_msg);
            Err(anyhow::anyhow!("{}", error_msg))
        }
    }
}

/// Execute an on_failure command and return its details even if it fails
pub async fn execute_on_failure_command(
    command: &str,
    retry_config: &RetryConfig,
) -> Result<ExecutionDetails> {
    let timeout = get_on_failure_timeout(retry_config);
    info!(
        "Executing on_failure command with timeout {:?}: {}",
        timeout, command
    );

    match execute_shell_command(command, timeout).await {
        Ok(output) => {
            let details = ExecutionDetails::from_output(command.to_string(), output);
            if !details.exit_status.success() {
                warn!(
                    "On_failure command failed with status {}: {}",
                    details.exit_status, command
                );
            } else {
                info!("On_failure command completed successfully: {}", command);
            }
            Ok(details)
        }
        Err(e) => {
            warn!("On_failure command execution error: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::SuccessCheck;

    #[tokio::test]
    async fn test_execute_success_checks_all_pass() {
        let checks = vec![
            SuccessCheck::Shell {
                command: "echo 'test'".to_string(),
            },
            SuccessCheck::Shell {
                command: "true".to_string(),
            },
        ];
        let retry_config = RetryConfig::default();

        let result = execute_success_checks(&checks, &retry_config).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SuccessCheckResult::Passed));
    }

    #[tokio::test]
    async fn test_execute_success_checks_one_fails() {
        let checks = vec![
            SuccessCheck::Shell {
                command: "echo 'test'".to_string(),
            },
            SuccessCheck::Shell {
                command: "false".to_string(),
            },
        ];
        let retry_config = RetryConfig::default();

        let result = execute_success_checks(&checks, &retry_config).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SuccessCheckResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_execute_shell_command_success() {
        let result = execute_shell_command("echo 'hello world'", Duration::from_secs(30)).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("hello world"));
    }

    #[tokio::test]
    async fn test_execute_shell_command_failure() {
        let result = execute_shell_command("false", Duration::from_secs(30)).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.status.success());
    }

    #[tokio::test]
    async fn test_execute_on_failure_command_success() {
        let retry_config = RetryConfig::default();
        let result = execute_on_failure_command("echo 'cleanup'", &retry_config).await;
        assert!(result.is_ok());
        assert!(result.unwrap().exit_status.success());
    }

    #[tokio::test]
    async fn test_execute_on_failure_command_failure() {
        let retry_config = RetryConfig::default();
        let result = execute_on_failure_command("false", &retry_config).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().exit_status.success());
    }

    #[tokio::test]
    async fn test_shell_command_timeout() {
        let timeout = std::time::Duration::from_millis(100);
        let result = if cfg!(target_os = "windows") {
            execute_shell_command("timeout /t 1", timeout).await
        } else {
            execute_shell_command("sleep 1", timeout).await
        };

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_retry_timeout_uses_config_default() {
        let retry_config = RetryConfig {
            timeout_seconds: None,
            ..Default::default()
        };

        let timeout = get_retry_timeout(&retry_config);
        assert_eq!(timeout, Duration::from_secs(DEFAULT_RETRY_TIMEOUT_SECONDS));
    }

    #[tokio::test]
    async fn test_get_retry_timeout_uses_retry_config() {
        let retry_config = RetryConfig {
            timeout_seconds: Some(120),
            ..Default::default()
        };

        let timeout = get_retry_timeout(&retry_config);
        assert_eq!(timeout, Duration::from_secs(120));
    }

    #[tokio::test]
    async fn test_get_on_failure_timeout_uses_config_default() {
        let retry_config = RetryConfig {
            on_failure_timeout_seconds: None,
            ..Default::default()
        };

        let timeout = get_on_failure_timeout(&retry_config);
        assert_eq!(
            timeout,
            Duration::from_secs(DEFAULT_ON_FAILURE_TIMEOUT_SECONDS)
        );
    }

    #[tokio::test]
    async fn test_get_on_failure_timeout_uses_retry_config() {
        let retry_config = RetryConfig {
            on_failure_timeout_seconds: Some(900),
            ..Default::default()
        };

        let timeout = get_on_failure_timeout(&retry_config);
        assert_eq!(timeout, Duration::from_secs(900));
    }

    #[tokio::test]
    async fn test_on_failure_timeout_different_from_retry_timeout() {
        let retry_config = RetryConfig {
            timeout_seconds: Some(60),
            on_failure_timeout_seconds: Some(300),
            ..Default::default()
        };

        let retry_timeout = get_retry_timeout(&retry_config);
        let on_failure_timeout = get_on_failure_timeout(&retry_config);

        assert_eq!(retry_timeout, Duration::from_secs(60));
        assert_eq!(on_failure_timeout, Duration::from_secs(300));
        assert_ne!(retry_timeout, on_failure_timeout);
    }
}
