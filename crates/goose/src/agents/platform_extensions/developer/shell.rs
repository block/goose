use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::{wrappers::SplitStream, StreamExt};

use crate::subprocess::SubprocessExt;

const OUTPUT_LIMIT_LINES: usize = 2000;
const OUTPUT_LIMIT_BYTES: usize = 50_000;
const OUTPUT_PREVIEW_LINES: usize = 50;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShellParams {
    pub command: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Resolve the user's full PATH by running a login shell.
///
/// When goosed is launched from a desktop app (e.g. Electron), it may inherit
/// a minimal PATH like `/usr/bin:/bin`. This function spawns a login shell to
/// source the user's profile and recover the full PATH.
#[cfg(not(windows))]
fn resolve_login_shell_path() -> Option<String> {
    let shell = if PathBuf::from("/bin/bash").is_file() {
        "/bin/bash".to_string()
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
    };

    std::process::Command::new(&shell)
        .args(["-l", "-i", "-c", "echo $PATH"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                // Take the last non-empty line — interactive shells may emit
                // extra output from profile scripts before our echo.
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
                    .filter(|path| !path.is_empty())
            } else {
                None
            }
        })
}

/// Returns the user's full login shell PATH, resolved once and cached.
#[cfg(not(windows))]
fn user_login_path() -> Option<&'static str> {
    static CACHED: OnceLock<Option<String>> = OnceLock::new();
    CACHED.get_or_init(resolve_login_shell_path).as_deref()
}

pub struct ShellTool;

impl ShellTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn shell(&self, params: ShellParams) -> CallToolResult {
        self.shell_with_cwd(params, None).await
    }

    pub async fn shell_with_cwd(
        &self,
        params: ShellParams,
        working_dir: Option<&std::path::Path>,
    ) -> CallToolResult {
        if params.command.trim().is_empty() {
            return CallToolResult::error(vec![Content::text(
                "Command cannot be empty.".to_string(),
            )
            .with_priority(0.0)]);
        }

        let execution = match run_command(&params.command, params.timeout_secs, working_dir).await {
            Ok(execution) => execution,
            Err(error) => {
                return CallToolResult::error(vec![Content::text(error).with_priority(0.0)])
            }
        };

        let mut rendered = match render_output(&execution.output) {
            Ok(rendered) => rendered,
            Err(error) => {
                return CallToolResult::error(vec![Content::text(error).with_priority(0.0)])
            }
        };

        if execution.timed_out {
            if let Some(timeout_secs) = params.timeout_secs {
                rendered.push_str(&format!(
                    "\n\nCommand timed out after {} seconds",
                    timeout_secs
                ));
            } else {
                rendered.push_str("\n\nCommand timed out");
            }
            return CallToolResult::error(vec![Content::text(rendered).with_priority(0.0)]);
        }

        if execution.exit_code.unwrap_or(1) != 0 {
            rendered.push_str(&format!(
                "\n\nCommand exited with code {}",
                execution.exit_code.unwrap_or(1)
            ));
            return CallToolResult::error(vec![Content::text(rendered).with_priority(0.0)]);
        }

        CallToolResult::success(vec![Content::text(rendered).with_priority(0.0)])
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

struct ExecutionOutput {
    output: String,
    exit_code: Option<i32>,
    timed_out: bool,
}

async fn run_command(
    command_line: &str,
    timeout_secs: Option<u64>,
    working_dir: Option<&std::path::Path>,
) -> Result<ExecutionOutput, String> {
    let mut command = build_shell_command(command_line);
    if let Some(path) = working_dir {
        command.current_dir(path);
    }

    #[cfg(not(windows))]
    if let Some(path) = user_login_path() {
        command.env("PATH", path);
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.stdin(Stdio::null());

    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to spawn shell command: {}", error))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    let output_task = tokio::spawn(async move { collect_merged_output(stdout, stderr).await });

    let mut timed_out = false;
    let exit_code = if let Some(timeout_secs) = timeout_secs.filter(|value| *value > 0) {
        match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await {
            Ok(wait_result) => wait_result
                .map_err(|error| format!("Failed waiting on shell command: {}", error))?
                .code(),
            Err(_) => {
                timed_out = true;
                let _ = child.start_kill();
                let _ = child.wait().await;
                None
            }
        }
    } else {
        child
            .wait()
            .await
            .map_err(|error| format!("Failed waiting on shell command: {}", error))?
            .code()
    };

    let output = output_task
        .await
        .map_err(|error| format!("Failed to collect shell output: {}", error))?
        .map_err(|error| format!("Failed to collect shell output: {}", error))?;

    Ok(ExecutionOutput {
        output,
        exit_code,
        timed_out,
    })
}

fn build_shell_command(command_line: &str) -> tokio::process::Command {
    #[cfg(windows)]
    let mut command = {
        let mut command = tokio::process::Command::new("cmd");
        command.arg("/C").arg(command_line);
        command
    };

    #[cfg(not(windows))]
    let mut command = {
        let shell = if PathBuf::from("/bin/bash").is_file() {
            "/bin/bash".to_string()
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
        };
        let mut command = tokio::process::Command::new(shell);
        command.arg("-c").arg(command_line);
        command
    };

    command.set_no_window();
    command
}

async fn collect_merged_output(
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
) -> Result<String, std::io::Error> {
    let stdout = BufReader::new(stdout);
    let stderr = BufReader::new(stderr);
    let stdout = SplitStream::new(stdout.split(b'\n')).map(|line| ("stdout", line));
    let stderr = SplitStream::new(stderr.split(b'\n')).map(|line| ("stderr", line));
    let mut merged = stdout.merge(stderr);

    let mut output = String::new();
    while let Some((_stream, line)) = merged.next().await {
        let mut line = line?;
        line.push(b'\n');
        output.push_str(&String::from_utf8_lossy(&line));
    }

    Ok(output.trim_end_matches('\n').to_string())
}

fn render_output(full_output: &str) -> Result<String, String> {
    if full_output.is_empty() {
        return Ok("(no output)".to_string());
    }

    let lines: Vec<&str> = full_output.split('\n').collect();
    let total_lines = lines.len();
    let total_bytes = full_output.len();

    let exceeded_lines = total_lines > OUTPUT_LIMIT_LINES;
    let exceeded_bytes = total_bytes > OUTPUT_LIMIT_BYTES;

    if !exceeded_lines && !exceeded_bytes {
        return Ok(full_output.to_string());
    }

    let output_path = save_full_output(full_output)?;

    let preview_start = total_lines.saturating_sub(OUTPUT_PREVIEW_LINES);
    let preview = lines[preview_start..].join("\n");

    let reason = if exceeded_lines {
        format!("Output exceeded {OUTPUT_LIMIT_LINES} line limit ({total_lines} lines total).")
    } else {
        format!(
            "Output exceeded {} byte limit ({total_bytes} bytes total).",
            OUTPUT_LIMIT_BYTES
        )
    };

    Ok(format!(
        "{preview}\n\n[{reason} Full output saved to {path}. \
         Read it with shell commands like `head`, `tail`, or `sed -n '100,200p'` \
         up to 2000 lines at a time.]",
        path = output_path.display(),
    ))
}

fn output_buffer_path() -> Result<PathBuf, String> {
    static PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
    let mut guard = PATH.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
    if let Some(path) = guard.as_ref() {
        return Ok(path.clone());
    }
    let temp_file =
        tempfile::NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {e}"))?;
    let (_, path) = temp_file
        .keep()
        .map_err(|e| format!("Failed to persist temp file: {}", e.error))?;
    *guard = Some(path.clone());
    Ok(path)
}

fn save_full_output(output: &str) -> Result<PathBuf, String> {
    let path = output_buffer_path()?;
    std::fs::write(&path, output).map_err(|e| format!("Failed to write output buffer: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::RawContent;

    fn extract_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(text) => &text.text,
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn shell_executes_command() {
        let tool = ShellTool::new();
        let result = tool
            .shell(ShellParams {
                command: "echo hello".to_string(),
                timeout_secs: None,
            })
            .await;

        assert_eq!(result.is_error, Some(false));
        assert!(extract_text(&result).contains("hello"));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn shell_returns_error_for_non_zero_exit() {
        let tool = ShellTool::new();
        let result = tool
            .shell(ShellParams {
                command: "echo fail && exit 7".to_string(),
                timeout_secs: None,
            })
            .await;

        assert_eq!(result.is_error, Some(true));
        assert!(extract_text(&result).contains("Command exited with code 7"));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn shell_uses_working_dir_for_relative_execution() {
        let dir = tempfile::tempdir().unwrap();
        let tool = ShellTool::new();
        let result = tool
            .shell_with_cwd(
                ShellParams {
                    command: "pwd".to_string(),
                    timeout_secs: None,
                },
                Some(dir.path()),
            )
            .await;

        assert_eq!(result.is_error, Some(false));
        let observed = std::fs::canonicalize(extract_text(&result)).unwrap();
        let expected = std::fs::canonicalize(dir.path()).unwrap();
        assert_eq!(observed, expected);
    }

    #[test]
    fn render_output_returns_full_output_when_under_limit() {
        let input = (0..100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let rendered = render_output(&input).unwrap();
        assert_eq!(rendered, input);
    }

    #[test]
    fn render_output_shows_empty_message() {
        let rendered = render_output("").unwrap();
        assert_eq!(rendered, "(no output)");
    }

    #[test]
    fn render_output_truncates_when_lines_exceeded() {
        let input = (0..2500)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let rendered = render_output(&input).unwrap();
        let (preview, metadata) = rendered.split_once("\n\n[").unwrap();

        assert_eq!(preview.lines().count(), OUTPUT_PREVIEW_LINES);
        assert!(preview.starts_with("line 2450"));
        assert!(preview.contains("line 2499"));
        assert!(metadata.contains("2000 line limit"));
        assert!(metadata.contains("2500 lines total"));
        assert!(metadata.contains("Full output saved to"));
        assert!(metadata.contains("head"));
        assert!(metadata.contains("sed -n"));
    }

    #[test]
    fn render_output_truncates_when_bytes_exceeded() {
        let long_line = "x".repeat(1000);
        let input = (0..100)
            .map(|_| long_line.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(input.len() > OUTPUT_LIMIT_BYTES);
        assert!(input.lines().count() <= OUTPUT_LIMIT_LINES);

        let rendered = render_output(&input).unwrap();
        let (_preview, metadata) = rendered.split_once("\n\n[").unwrap();

        assert!(metadata.contains("byte limit"));
        assert!(metadata.contains("bytes total"));
        assert!(metadata.contains("Full output saved to"));
    }

    #[test]
    fn save_full_output_reuses_same_path() {
        let path1 = save_full_output("first").unwrap();
        let path2 = save_full_output("second").unwrap();
        assert_eq!(path1, path2);
        // Note: we intentionally don't assert file content here because
        // parallel tests (render_output_truncates_*) share the same static
        // temp file and can overwrite the content between our write and read.
    }
}
