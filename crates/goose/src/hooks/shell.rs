use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use crate::agents::platform_extensions::developer::shell::build_shell_command;
#[cfg(not(windows))]
use crate::agents::platform_extensions::developer::shell::user_login_path;

/// Output from a hook command execution.
pub struct HookCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

/// Run a hook command as a direct subprocess.
///
/// Deadlock-safe: stdout and stderr are drained concurrently via spawned tasks,
/// and both drains start BEFORE stdin is written. This prevents circular deadlock
/// when the child echoes input back to stdout/stderr before consuming all stdin.
///
/// The child is placed in its own process group (unix) so terminal SIGINT does not
/// kill it — the cancellation token is the intended shutdown path.
pub async fn run_hook_command(
    command_line: &str,
    stdin_data: Option<&str>,
    timeout_secs: u64,
    working_dir: &Path,
    cancel_token: CancellationToken,
) -> Result<HookCommandOutput, String> {
    // Guard zero timeout — default to 10 minutes
    let timeout = if timeout_secs == 0 { 600 } else { timeout_secs };

    let mut command = build_shell_command(command_line);
    command.current_dir(working_dir);

    // Inherit the user's full login shell PATH (not the minimal desktop-app PATH)
    #[cfg(not(windows))]
    if let Some(path) = user_login_path() {
        command.env("PATH", path);
    }

    // Isolate hook into its own process group so Ctrl+C (SIGINT) doesn't kill it.
    // The cancellation token is the intended shutdown path for hooks.
    #[cfg(unix)]
    command.process_group(0);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    if stdin_data.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn hook command: {}", e))?;

    // Take ALL handles before spawning any tasks
    let stdin_handle = child.stdin.take();
    let stdout_handle = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr_handle = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    // Spawn stdout drain FIRST (before stdin write to prevent circular deadlock)
    let stdout_task = tokio::spawn(async move {
        let mut output = String::new();
        let mut reader = stdout_handle;
        let _ = reader.read_to_string(&mut output).await;
        output
    });

    // Spawn stderr drain concurrently
    let stderr_task = tokio::spawn(async move {
        let mut output = String::new();
        let mut reader = stderr_handle;
        let _ = reader.read_to_string(&mut output).await;
        output
    });

    // Write stdin data concurrently with drains.
    // Wrapped in a timeout to prevent hanging if the child stops reading stdin.
    let stdin_data_owned = stdin_data.map(|s| s.to_string());
    let stdin_task = tokio::spawn(async move {
        if let Some(data) = stdin_data_owned {
            if let Some(mut stdin) = stdin_handle {
                let _ =
                    tokio::time::timeout(Duration::from_secs(30), stdin.write_all(data.as_bytes()))
                        .await;
                drop(stdin); // Close stdin so child sees EOF
            }
        }
    });

    // Wait for child with timeout + cancellation
    let (exit_code, timed_out) = tokio::select! {
        result = tokio::time::timeout(Duration::from_secs(timeout), child.wait()) => {
            match result {
                Ok(Ok(status)) => (status.code(), false),
                Ok(Err(e)) => {
                    return Err(format!("Failed waiting on hook command: {}", e));
                }
                Err(_) => {
                    // Timeout — kill the process
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    (None, true)
                }
            }
        }
        _ = cancel_token.cancelled() => {
            // Cancellation — kill the process
            let _ = child.start_kill();
            let _ = child.wait().await;
            (None, true)
        }
    };

    // Collect output from drain tasks.
    // Use a secondary timeout to prevent hanging if grandchild processes hold pipe FDs open.
    let drain_timeout = Duration::from_secs(5);
    let stdout_output = tokio::time::timeout(drain_timeout, stdout_task)
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or_default();
    let stderr_output = tokio::time::timeout(drain_timeout, stderr_task)
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or_default();

    // Best-effort wait for stdin task to finish
    let _ = tokio::time::timeout(Duration::from_secs(1), stdin_task).await;

    Ok(HookCommandOutput {
        stdout: stdout_output,
        stderr: stderr_output,
        exit_code,
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[cfg(not(windows))]
    #[tokio::test]
    async fn hook_receives_stdin_and_returns_stdout() {
        // Real subprocess: reads JSON from stdin, echoes a field back to stdout
        let dir = tempfile::tempdir().unwrap();
        let output = run_hook_command(
            r#"jq -r '.hook_event_name'"#,
            Some(r#"{"hook_event_name":"PreToolUse","session_id":"s1"}"#),
            10,
            dir.path(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stdout.trim(), "PreToolUse");
        assert!(output.stderr.is_empty());
        assert!(!output.timed_out);
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn hook_exit_2_captures_stderr_separately() {
        // Real subprocess: writes to stderr and exits 2 (Claude Code block protocol)
        let dir = tempfile::tempdir().unwrap();
        let output = run_hook_command(
            "echo 'blocked: rm not allowed' >&2; exit 2",
            None,
            10,
            dir.path(),
            CancellationToken::new(),
        )
        .await
        .unwrap();

        assert_eq!(output.exit_code, Some(2));
        assert!(
            output.stderr.contains("rm not allowed"),
            "stderr should contain the block reason, got: {:?}",
            output.stderr
        );
        // stdout should be empty — Claude Code ignores stdout on exit 2
        assert!(output.stdout.trim().is_empty());
    }
}
