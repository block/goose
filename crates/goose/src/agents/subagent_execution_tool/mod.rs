//! Subagent execution notification types.
//!
//! This module contains types used for task execution notifications in the CLI.
//! The actual subagent execution is now handled by the unified `subagent` tool
//! in `crate::agents::subagent_tool`.

pub mod notification_events;

// Re-export commonly used types for backward compatibility
pub mod lib {
    pub use super::notification_events::TaskStatus;
}
