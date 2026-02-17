//! Summon sub-modules — types, utilities, and source discovery for the summon extension.
//!
//! The main `SummonClient` implementation remains in `summon_extension.rs` (the parent module).
//! These sub-modules extract reusable data types and utility functions to reduce file size
//! and improve navigability.

pub mod types;
pub mod utils;

// Re-export key types for convenience
pub use types::{
    kind_plural, parse_agent_content, parse_frontmatter, parse_skill_content, BackgroundTask,
    CompletedTask, DelegateParams, Source, SourceKind,
};
pub use utils::{
    current_epoch_millis, is_session_id, max_background_tasks, round_duration, truncate,
};
