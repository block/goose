use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::{wrappers::SplitStream, StreamExt};

use crate::subprocess::SubprocessExt;

const DEFAULT_MAX_LINES: usize = 2000;
const DEFAULT_MAX_BYTES: usize = 50 * 1024;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShellParams {
    pub command: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

pub struct ShellTool;

impl ShellTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn shell(&self, params: ShellParams) -> CallToolResult {
        if params.command.trim().is_empty() {
            return CallToolResult::error(vec![Content::text(
                "Command cannot be empty.".to_string(),
            )]);
        }

        let execution = match run_command(&params.command, params.timeout_secs).await {
            Ok(execution) => execution,
            Err(error) => return CallToolResult::error(vec![Content::text(error)]),
        };

        let mut rendered = match render_output(&execution.output) {
            Ok(rendered) => rendered,
            Err(error) => return CallToolResult::error(vec![Content::text(error)]),
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
            return CallToolResult::error(vec![Content::text(rendered)]);
        }

        if execution.exit_code.unwrap_or(1) != 0 {
            rendered.push_str(&format!(
                "\n\nCommand exited with code {}",
                execution.exit_code.unwrap_or(1)
            ));
            return CallToolResult::error(vec![Content::text(rendered)]);
        }

        CallToolResult::success(vec![Content::text(rendered)])
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
) -> Result<ExecutionOutput, String> {
    let mut command = build_shell_command(command_line);
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TruncatedBy {
    Lines,
    Bytes,
}

#[derive(Debug)]
struct TruncationResult {
    content: String,
    truncated: bool,
    truncated_by: Option<TruncatedBy>,
    total_lines: usize,
    output_lines: usize,
    output_bytes: usize,
    last_line_partial: bool,
}

fn truncate_tail(content: &str, max_lines: usize, max_bytes: usize) -> TruncationResult {
    let total_bytes = content.len();
    let lines: Vec<&str> = content.split('\n').collect();
    let total_lines = lines.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            total_lines,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
        };
    }

    let mut output_lines = Vec::new();
    let mut output_bytes = 0usize;
    let mut truncated_by = TruncatedBy::Lines;
    let mut last_line_partial = false;

    for index in (0..lines.len()).rev() {
        if output_lines.len() >= max_lines {
            break;
        }

        let line = lines[index];
        let line_bytes = line.len() + usize::from(!output_lines.is_empty());
        if output_bytes + line_bytes > max_bytes {
            truncated_by = TruncatedBy::Bytes;
            if output_lines.is_empty() {
                let partial = truncate_string_to_bytes_from_end(line, max_bytes);
                output_lines.insert(0, partial);
                output_bytes = output_lines[0].len();
                last_line_partial = true;
            }
            break;
        }

        output_lines.insert(0, line.to_string());
        output_bytes += line_bytes;
    }

    if output_lines.len() >= max_lines && output_bytes <= max_bytes {
        truncated_by = TruncatedBy::Lines;
    }

    let content = output_lines.join("\n");
    let final_bytes = content.len();
    TruncationResult {
        content,
        truncated: true,
        truncated_by: Some(truncated_by),
        total_lines,
        output_lines: output_lines.len(),
        output_bytes: final_bytes,
        last_line_partial,
    }
}

fn truncate_string_to_bytes_from_end(text: &str, max_bytes: usize) -> String {
    let bytes = text.as_bytes();
    if bytes.len() <= max_bytes {
        return text.to_string();
    }

    let mut start = bytes.len().saturating_sub(max_bytes);
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }

    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn render_output(full_output: &str) -> Result<String, String> {
    let truncation = truncate_tail(full_output, DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);
    let mut rendered = if truncation.content.is_empty() {
        "(no output)".to_string()
    } else {
        truncation.content
    };

    if !truncation.truncated {
        return Ok(rendered);
    }

    let output_path = persist_full_output(full_output)?;
    let start_line = truncation
        .total_lines
        .saturating_sub(truncation.output_lines)
        .saturating_add(1);
    let end_line = truncation.total_lines;

    if truncation.last_line_partial {
        let last_line_size = full_output.lines().last().map(str::len).unwrap_or_default();
        rendered.push_str(&format!(
            "\n\n[Showing last {} of line {} (line is {}). Full output: {}]",
            format_size(truncation.output_bytes),
            end_line,
            format_size(last_line_size),
            output_path.display()
        ));
    } else if truncation.truncated_by == Some(TruncatedBy::Lines) {
        rendered.push_str(&format!(
            "\n\n[Showing lines {}-{} of {}. Full output: {}]",
            start_line,
            end_line,
            truncation.total_lines,
            output_path.display()
        ));
    } else {
        rendered.push_str(&format!(
            "\n\n[Showing lines {}-{} of {} ({} limit). Full output: {}]",
            start_line,
            end_line,
            truncation.total_lines,
            format_size(DEFAULT_MAX_BYTES),
            output_path.display()
        ));
    }

    Ok(rendered)
}

fn persist_full_output(output: &str) -> Result<PathBuf, String> {
    let temp_file = tempfile::NamedTempFile::new()
        .map_err(|error| format!("Failed to create temp file: {}", error))?;
    std::fs::write(temp_file.path(), output)
        .map_err(|error| format!("Failed to write temp output file: {}", error))?;
    let (_, path) = temp_file
        .keep()
        .map_err(|error| format!("Failed to persist temp output file: {}", error.error))?;
    Ok(path)
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
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

    #[test]
    fn truncate_tail_limits_lines() {
        let mut input = String::new();
        for index in 0..2500 {
            input.push_str(&format!("line {}\n", index));
        }

        let result = truncate_tail(&input, DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);
        assert!(result.truncated);
        assert!(result.output_lines <= DEFAULT_MAX_LINES);
        assert_eq!(result.truncated_by, Some(TruncatedBy::Lines));
    }
}
