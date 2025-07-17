use crate::session;
use mcp_core::{Tool, ToolResult};
use rmcp::model::Content;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Type alias for the tool result channel receiver
pub type ToolResultReceiver = Arc<Mutex<mpsc::Receiver<(String, ToolResult<Vec<Content>>)>>>;

/// Default timeout for retry operations (5 minutes)
pub const DEFAULT_RETRY_TIMEOUT_SECONDS: u64 = 300;

/// Default timeout for cleanup operations (10 minutes - longer for cleanup tasks)
pub const DEFAULT_CLEANUP_TIMEOUT_SECONDS: u64 = 600;

/// Configuration for retry logic in recipe execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts before giving up
    pub max_retries: u32,
    /// List of success checks to validate recipe completion
    pub checks: Vec<SuccessCheck>,
    /// Optional shell command to run on failure for cleanup
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
    /// Timeout in seconds for individual shell commands (default: 300 seconds)
    /// Can also be configured globally via GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS environment variable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    /// Timeout in seconds for cleanup commands (default: 600 seconds)
    /// Can also be configured globally via GOOSE_RECIPE_CLEANUP_TIMEOUT_SECONDS environment variable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_timeout_seconds: Option<u64>,
}

/// A single success check to validate recipe completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCheck {
    /// The type of success check to perform
    #[serde(rename = "type")]
    pub check_type: SuccessCheckType,
    /// The command or instruction for the success check
    pub command: String,
}

/// Types of success checks that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuccessCheckType {
    /// Execute a shell command and check its exit status
    #[serde(alias = "shell")]
    Shell,
}

/// A frontend tool that will be executed by the frontend rather than an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendTool {
    pub name: String,
    pub tool: Tool,
}

/// Session configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Unique identifier for the session
    pub id: session::Identifier,
    /// Working directory for the session
    pub working_dir: PathBuf,
    /// ID of the schedule that triggered this session, if any
    pub schedule_id: Option<String>,
    /// Execution mode for scheduled jobs: "foreground" or "background"
    pub execution_mode: Option<String>,
    /// Maximum number of turns (iterations) allowed without user input
    pub max_turns: Option<u32>,
    /// Retry configuration for automated validation and recovery
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_config: Option<RetryConfig>,
}
