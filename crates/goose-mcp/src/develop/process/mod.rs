//! Process management: shell execution with state tracking.
//!
//! This module provides:
//! - `shell`: Execute commands with automatic env/cwd persistence
//! - `process_list`: List tracked processes
//! - `process_output`: Query process output with slicing/grep
//! - `process_status`: Check if process is running/exited
//! - `process_await`: Wait for process completion
//! - `process_kill`: Terminate a process
//! - `process_input`: Send text to process stdin

mod buffer;
mod manager;
mod shell;
mod types;

pub use manager::ProcessManager;
pub use types::{
    AwaitResult, KillResult, OutputQuery, ProcessId, ProcessInfo, ProcessStatus, SpawnResult,
};

use std::sync::Arc;
use std::time::Duration;

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

/// Maximum await timeout (5 minutes).
const MAX_AWAIT_TIMEOUT_SECS: u64 = 300;

// ============================================================================
// Tool Parameter Types
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShellParams {
    /// The shell command to execute.
    command: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProcessOutputParams {
    /// Process ID (e.g., "proc01").
    id: String,
    /// Start line index. Supports negative values for tail (e.g., -30 for last 30 lines).
    #[serde(default)]
    start: Option<i64>,
    /// End line index. Supports negative values.
    #[serde(default)]
    end: Option<i64>,
    /// Filter to lines matching this pattern.
    #[serde(default)]
    grep: Option<String>,
    /// Lines of context before each grep match.
    #[serde(default)]
    before: Option<usize>,
    /// Lines of context after each grep match.
    #[serde(default)]
    after: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProcessIdParams {
    /// Process ID (e.g., "proc01").
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProcessAwaitParams {
    /// Process ID (e.g., "proc01").
    id: String,
    /// Timeout in seconds (required, max 300).
    timeout_secs: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProcessInputParams {
    /// Process ID (e.g., "proc01").
    id: String,
    /// Text to send to stdin.
    text: String,
}

// ============================================================================
// Tool Implementations
// ============================================================================

/// Tools for process management, to be registered with the MCP server.
pub struct ProcessTools {
    manager: Arc<ProcessManager>,
}

impl ProcessTools {
    pub fn new(manager: Arc<ProcessManager>) -> Self {
        Self { manager }
    }

    /// Execute a shell command.
    pub fn shell(&self, params: ShellParams) -> CallToolResult {
        match self.manager.spawn(&params.command) {
            Ok(SpawnResult::Completed { output, exit_code }) => {
                let text = if exit_code == 0 {
                    output
                } else {
                    format!("{}\n(exit: {})", output, exit_code)
                };
                CallToolResult::success(vec![Content::text(text)])
            }
            Ok(SpawnResult::Promoted {
                id,
                output_preview,
                lines_omitted,
            }) => {
                let status = self.manager.status(&id.0).unwrap_or(ProcessStatus::Running);
                let status_str = match status {
                    ProcessStatus::Running => "running".to_string(),
                    ProcessStatus::Exited(code) => format!("exit: {}", code),
                    ProcessStatus::Killed => "killed".to_string(),
                };

                let _ = lines_omitted; // May use later for different formatting
                let text = format!("Process: {} ({})\n\n{}", id, status_str, output_preview);
                CallToolResult::success(vec![Content::text(text)])
            }
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    /// List all tracked processes.
    pub fn process_list(&self) -> CallToolResult {
        let procs = self.manager.list();

        if procs.is_empty() {
            return CallToolResult::success(vec![Content::text("No tracked processes.")]);
        }

        let lines: Vec<String> = procs
            .iter()
            .map(|p| format!("{}, {}, {}", p.id, p.command_display(80), p.status))
            .collect();

        CallToolResult::success(vec![Content::text(lines.join("\n"))])
    }

    /// Get process output with optional filtering.
    pub fn process_output(&self, params: ProcessOutputParams) -> CallToolResult {
        let query = OutputQuery {
            start: params.start,
            end: params.end,
            grep: params.grep,
            before: params.before,
            after: params.after,
        };

        match self.manager.output(&params.id, query) {
            Ok(output) => CallToolResult::success(vec![Content::text(output)]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    /// Get process status.
    pub fn process_status(&self, params: ProcessIdParams) -> CallToolResult {
        match self.manager.status(&params.id) {
            Ok(status) => CallToolResult::success(vec![Content::text(status.to_string())]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    /// Wait for process completion.
    pub fn process_await(&self, params: ProcessAwaitParams) -> CallToolResult {
        let timeout_secs = params.timeout_secs.min(MAX_AWAIT_TIMEOUT_SECS);
        let timeout = Duration::from_secs(timeout_secs);

        match self.manager.await_completion(&params.id, timeout) {
            Ok(AwaitResult::Completed { output, exit_code }) => {
                let text = if exit_code == 0 {
                    output
                } else {
                    format!("{}\n(exit: {})", output, exit_code)
                };
                CallToolResult::success(vec![Content::text(text)])
            }
            Ok(AwaitResult::TimedOut { output_so_far }) => {
                let text = format!(
                    "{}\n\n(timed out after {}s, process still running)",
                    output_so_far, timeout_secs
                );
                CallToolResult::success(vec![Content::text(text)])
            }
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    /// Kill a process.
    pub fn process_kill(&self, params: ProcessIdParams) -> CallToolResult {
        match self.manager.kill(&params.id) {
            Ok(KillResult::Killed) => CallToolResult::success(vec![Content::text(format!(
                "Process {} killed.",
                params.id
            ))]),
            Ok(KillResult::AlreadyExited(code)) => CallToolResult::success(vec![Content::text(
                format!("Process {} already exited with code {}.", params.id, code),
            )]),
            Ok(KillResult::AlreadyKilled) => CallToolResult::success(vec![Content::text(format!(
                "Process {} was already killed.",
                params.id
            ))]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    /// Send input to process stdin.
    pub fn process_input(&self, params: ProcessInputParams) -> CallToolResult {
        match self.manager.send(&params.id, &params.text) {
            Ok(()) => CallToolResult::success(vec![Content::text(format!(
                "Sent {} bytes to {}.",
                params.text.len(),
                params.id
            ))]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }
}
