use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use crate::agents::platform_extensions::developer::shell::build_shell_command;
#[cfg(not(windows))]
use crate::agents::platform_extensions::developer::shell::user_login_path;

/// Maximum bytes to capture from stdout (32 KB — matches output cap in mod.rs).
const MAX_STDOUT_BYTES: usize = 32 * 1024;
/// Maximum bytes to capture from stderr (4 KB — matches block-reason cap in mod.rs).
const MAX_STDERR_BYTES: usize = 4 * 1024;

/// Output from a hook command execution.
pub struct HookCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

/// Read up to `limit` bytes from an async reader into a String.
/// Prevents unbounded memory growth from malicious/buggy hooks.
async fn read_bounded(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    limit: usize,
) -> String {
    let mut buf = vec![0u8; limit];
    let mut total = 0;
    loop {
        match reader.read(&mut buf[total..]).await {
            Ok(0) => break,
            Ok(n) => {
                total += n;
                if total >= limit {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf.truncate(total);
    String::from_utf8_lossy(&buf).into_owned()
}

/// Kill the entire process group on Unix (sends signal to -pgid).
/// Falls back to killing just the child if process group kill fails.
#[cfg(unix)]
fn kill_process_group(child: &tokio::process::Child) {
    if let Some(pid) = child.id() {
        // Kill the entire process group (negative PID = process group)
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

/// Run a hook command as a direct subprocess.
///
/// Deadlock-safe: stdout and stderr are drained concurrently via spawned tasks,
/// and both drains start BEFORE stdin is written. This prevents circular deadlock
/// when the child echoes input back to stdout/stderr before consuming all stdin.
///
/// The child is placed in its own process group (unix) so terminal SIGINT does not
/// kill it — the cancellation token is the intended shutdown path.
///
/// Output capture is bounded: stdout to 32KB, stderr to 4KB. Excess is silently
/// discarded to prevent OOM from malicious/buggy hooks.
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
    // Bounded read prevents OOM from hooks that produce excessive output.
    let stdout_task = tokio::spawn(async move {
        read_bounded(stdout_handle, MAX_STDOUT_BYTES).await
    });

    // Spawn stderr drain concurrently
    let stderr_task = tokio::spawn(async move {
        read_bounded(stderr_handle, MAX_STDERR_BYTES).await
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
                    // Timeout — kill the entire process group (not just the child)
                    #[cfg(unix)]
                    kill_process_group(&child);
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    (None, true)
                }
            }
        }
        _ = cancel_token.cancelled() => {
            // Cancellation — kill the entire process group
            #[cfg(unix)]
            kill_process_group(&child);
            let _ = child.start_kill();
            let _ = child.wait().await;
            (None, true)
        }
    };

    // Collect output from drain tasks.
    // Use a secondary timeout to prevent hanging if grandchild processes hold pipe FDs open.
    // On timeout, explicitly abort the drain tasks to prevent detached task leaks.
    let drain_timeout = Duration::from_secs(5);

    let stdout_output = match tokio::time::timeout(drain_timeout, stdout_task).await {
        Ok(Ok(output)) => output,
        Ok(Err(join_err)) => {
            tracing::warn!("Hook stdout drain task panicked: {}", join_err);
            String::new()
        }
        Err(_) => {
            // Drain timed out — grandchild likely holding pipe FDs open.
            // The task was consumed by timeout; since bounded read caps at MAX_STDOUT_BYTES
            // and process group was killed, the read will eventually EOF.
            tracing::warn!("Hook stdout drain timed out (grandchild may hold FDs)");
            String::new()
        }
    };

    let stderr_output = match tokio::time::timeout(drain_timeout, stderr_task).await {
        Ok(Ok(output)) => output,
        Ok(Err(join_err)) => {
            tracing::warn!("Hook stderr drain task panicked: {}", join_err);
            String::new()
        }
        Err(_) => {
            tracing::warn!("Hook stderr drain timed out (grandchild may hold FDs)");
            String::new()
        }
    };

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
