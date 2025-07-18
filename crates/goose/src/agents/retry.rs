use anyhow::Result;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::agents::types::{
    RetryConfig, SuccessCheck, SuccessCheckType, DEFAULT_CLEANUP_TIMEOUT_SECONDS,
    DEFAULT_RETRY_TIMEOUT_SECONDS,
};
use crate::config::Config;

/// Environment variable for configuring retry timeout globally
const GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS: &str = "GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS";

/// Environment variable for configuring cleanup timeout globally
const GOOSE_RECIPE_CLEANUP_TIMEOUT_SECONDS: &str = "GOOSE_RECIPE_CLEANUP_TIMEOUT_SECONDS";

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

/// Get the configured timeout duration for cleanup operations
/// retry_config.cleanup_timeout_seconds -> env var -> default
fn get_cleanup_timeout(retry_config: &RetryConfig) -> Duration {
    let timeout_seconds = retry_config
        .cleanup_timeout_seconds
        .or_else(|| {
            let config = Config::global();
            config.get_param(GOOSE_RECIPE_CLEANUP_TIMEOUT_SECONDS).ok()
        })
        .unwrap_or(DEFAULT_CLEANUP_TIMEOUT_SECONDS);

    Duration::from_secs(timeout_seconds)
}

/// Execute all success checks and return true if all pass
pub async fn execute_success_checks(
    checks: &[SuccessCheck],
    retry_config: &RetryConfig,
) -> Result<bool> {
    let timeout = get_retry_timeout(retry_config);

    for check in checks {
        match check.check_type {
            SuccessCheckType::Shell => {
                let result = execute_shell_command(&check.command, timeout).await?;
                if !result.status.success() {
                    warn!(
                        "Success check failed: command '{}' exited with status {}, stderr: {}",
                        check.command,
                        result.status,
                        String::from_utf8_lossy(&result.stderr)
                    );
                    return Ok(false);
                }
                info!(
                    "Success check passed: command '{}' completed successfully",
                    check.command
                );
            }
        }
    }
    Ok(true)
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
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .kill_on_drop(true)
                .output()
                .await?
        } else {
            Command::new("sh")
                .args(["-c", command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .kill_on_drop(true)
                .output()
                .await?
        };

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

/// Execute an on_failure command and return an error if it fails
pub async fn execute_on_failure_command(command: &str, retry_config: &RetryConfig) -> Result<()> {
    let timeout = get_cleanup_timeout(retry_config);
    info!(
        "Executing on_failure command with timeout {:?}: {}",
        timeout, command
    );

    let output = match execute_shell_command(command, timeout).await {
        Ok(output) => output,
        Err(e) => {
            if e.to_string().contains("timed out") {
                let error_msg = format!(
                    "On_failure command timed out after {:?}: {}",
                    timeout, command
                );
                warn!("{}", error_msg);
                return Err(anyhow::anyhow!(error_msg));
            } else {
                warn!("On_failure command execution error: {}", e);
                return Err(e);
            }
        }
    };

    if !output.status.success() {
        let error_msg = format!(
            "On_failure command failed: command '{}' exited with status {}, stderr: {}",
            command,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        warn!("{}", error_msg);
        return Err(anyhow::anyhow!(error_msg));
    } else {
        info!("On_failure command completed successfully: {}", command);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::{SuccessCheck, SuccessCheckType};

    fn create_test_retry_config() -> RetryConfig {
        RetryConfig {
            max_retries: 3,
            checks: vec![],
            on_failure: None,
            timeout_seconds: Some(60),
            cleanup_timeout_seconds: Some(120),
        }
    }

    #[tokio::test]
    async fn test_execute_success_checks_all_pass() {
        let checks = vec![
            SuccessCheck {
                check_type: SuccessCheckType::Shell,
                command: "echo 'test'".to_string(),
            },
            SuccessCheck {
                check_type: SuccessCheckType::Shell,
                command: "true".to_string(),
            },
        ];
        let retry_config = create_test_retry_config();

        let result = execute_success_checks(&checks, &retry_config).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_execute_success_checks_one_fails() {
        let checks = vec![
            SuccessCheck {
                check_type: SuccessCheckType::Shell,
                command: "echo 'test'".to_string(),
            },
            SuccessCheck {
                check_type: SuccessCheckType::Shell,
                command: "false".to_string(),
            },
        ];
        let retry_config = create_test_retry_config();

        let result = execute_success_checks(&checks, &retry_config).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
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
        let retry_config = create_test_retry_config();
        let result = execute_on_failure_command("echo 'cleanup'", &retry_config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_on_failure_command_failure() {
        let retry_config = create_test_retry_config();
        let result = execute_on_failure_command("false", &retry_config).await;
        assert!(result.is_err());
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
            max_retries: 1,
            checks: vec![],
            on_failure: None,
            timeout_seconds: None,
            cleanup_timeout_seconds: None,
        };

        let timeout = get_retry_timeout(&retry_config);
        assert_eq!(timeout, Duration::from_secs(DEFAULT_RETRY_TIMEOUT_SECONDS));
    }

    #[tokio::test]
    async fn test_get_retry_timeout_uses_retry_config() {
        let retry_config = RetryConfig {
            max_retries: 1,
            checks: vec![],
            on_failure: None,
            timeout_seconds: Some(120),
            cleanup_timeout_seconds: None,
        };

        let timeout = get_retry_timeout(&retry_config);
        assert_eq!(timeout, Duration::from_secs(120));
    }

    #[tokio::test]
    async fn test_get_cleanup_timeout_uses_config_default() {
        let retry_config = RetryConfig {
            max_retries: 1,
            checks: vec![],
            on_failure: None,
            timeout_seconds: None,
            cleanup_timeout_seconds: None,
        };

        let timeout = get_cleanup_timeout(&retry_config);
        assert_eq!(
            timeout,
            Duration::from_secs(DEFAULT_CLEANUP_TIMEOUT_SECONDS)
        );
    }

    #[tokio::test]
    async fn test_get_cleanup_timeout_uses_retry_config() {
        let retry_config = RetryConfig {
            max_retries: 1,
            checks: vec![],
            on_failure: None,
            timeout_seconds: None,
            cleanup_timeout_seconds: Some(900),
        };

        let timeout = get_cleanup_timeout(&retry_config);
        assert_eq!(timeout, Duration::from_secs(900));
    }

    #[tokio::test]
    async fn test_cleanup_timeout_different_from_retry_timeout() {
        let retry_config = RetryConfig {
            max_retries: 1,
            checks: vec![],
            on_failure: None,
            timeout_seconds: Some(60),
            cleanup_timeout_seconds: Some(300),
        };

        let retry_timeout = get_retry_timeout(&retry_config);
        let cleanup_timeout = get_cleanup_timeout(&retry_config);

        assert_eq!(retry_timeout, Duration::from_secs(60));
        assert_eq!(cleanup_timeout, Duration::from_secs(300));
        assert_ne!(retry_timeout, cleanup_timeout);
    }
}
