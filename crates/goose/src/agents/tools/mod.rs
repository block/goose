//! Tool management — registry, filtering, inspection, and scheduling.
//!
//! This module groups tool-related components:
//! - `ToolRegistry` — tracks available tools across all extensions
//! - `ToolFilter` — filters tools based on context and permissions
//! - `FinalOutputTool` — special tool for structured agent output
//! - `ScheduleTool` — scheduled task execution
//!
//! # Usage
//! ```rust,ignore
//! use goose::agents::tools::{filter_tools, ToolRegistry};
//! ```

pub use super::execute_commands::COMPACT_TRIGGERS;
pub use super::final_output_tool::{FINAL_OUTPUT_CONTINUATION_MESSAGE, FINAL_OUTPUT_TOOL_NAME};
pub use super::tool_filter::filter_tools;
pub use super::tool_registry::ToolRegistry;
