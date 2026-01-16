//! Shared types for process management.

use std::time::Instant;

/// Unique identifier for a managed process (e.g., "proc01", "proc02").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessId(pub String);

impl ProcessId {
    pub fn new(n: u32) -> Self {
        Self(format!("proc{:02}", n))
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ProcessId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Status of a managed process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    Exited(i32),
    Killed,
}

impl std::fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessStatus::Running => write!(f, "RUNNING"),
            ProcessStatus::Exited(code) => write!(f, "EXITED({})", code),
            ProcessStatus::Killed => write!(f, "KILLED"),
        }
    }
}

/// Query parameters for retrieving process output.
#[derive(Debug, Clone, Default)]
pub struct OutputQuery {
    /// Start line index (Python slice semantics, negative = from end).
    pub start: Option<i64>,
    /// End line index (Python slice semantics, negative = from end).
    pub end: Option<i64>,
    /// Filter to lines matching this pattern.
    pub grep: Option<String>,
    /// Lines of context before each grep match.
    pub before: Option<usize>,
    /// Lines of context after each grep match.
    pub after: Option<usize>,
}

/// Result from spawning a command.
#[derive(Debug)]
pub enum SpawnResult {
    /// Command completed quickly, here's the full output.
    Completed { output: String, exit_code: i32 },
    /// Command promoted to process manager (took too long or large output).
    Promoted {
        id: ProcessId,
        output_preview: String,
        lines_omitted: usize,
    },
}

/// Result from awaiting a process.
#[derive(Debug)]
pub enum AwaitResult {
    /// Process completed within timeout.
    Completed { output: String, exit_code: i32 },
    /// Process still running after timeout.
    TimedOut { output_so_far: String },
}

/// Result from killing a process.
#[derive(Debug)]
pub enum KillResult {
    /// Process was running and has been killed.
    Killed,
    /// Process had already exited.
    AlreadyExited(i32),
    /// Process was already killed.
    AlreadyKilled,
}

/// Info about a process for listing.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub id: ProcessId,
    pub command: String,
    pub status: ProcessStatus,
    pub started_at: Instant,
}

impl ProcessInfo {
    /// Format command for display, truncated to max_len.
    pub fn command_display(&self, max_len: usize) -> String {
        if self.command.len() <= max_len {
            self.command.clone()
        } else {
            // Use chars to safely handle UTF-8
            let truncate_len = max_len.saturating_sub(3);
            let truncated: String = self.command.chars().take(truncate_len).collect();
            format!("{}...", truncated)
        }
    }
}
