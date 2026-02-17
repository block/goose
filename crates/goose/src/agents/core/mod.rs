//! Core agent runtime — the engine that powers all agent instances.
//!
//! This module groups the fundamental runtime components:
//! - `Agent` — the main agent struct (event loop, reply stream, tool execution)
//! - `AgentConfig` / `SessionConfig` — configuration types
//! - `PromptManager` — system prompt construction and management
//! - `RetryManager` — retry logic for failed tool calls and provider errors
//! - `ToolExecution` — tool call dispatch and result handling
//!
//! # Usage
//! ```rust,ignore
//! use goose::agents::core::{Agent, AgentConfig, SessionConfig};
//! ```

// Re-export core runtime types from their current locations
pub use super::agent::{Agent, AgentConfig, AgentEvent, ExtensionLoadResult};
pub use super::prompt_manager::PromptManager;
pub use super::retry::{RetryManager, RetryResult};
pub use super::specialist_config::{TaskConfig, DEFAULT_SUBAGENT_MAX_TURNS};
pub use super::specialist_handler::SPECIALIST_TOOL_REQUEST_TYPE;
pub use super::tool_execution::ToolCallResult;
pub use super::types::{FrontendTool, RetryConfig, SessionConfig, SharedProvider, SuccessCheck};
