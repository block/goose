//! Orchestration — delegation, sub-agent management, and multi-agent coordination.
//!
//! This module groups delegation and orchestration components:
//! - `DelegationStrategy` — routing strategy for task delegation (in-process, ACP, A2A)
//! - `SubagentExecutionTool` — notification events for sub-agent progress
//!
//! # Usage
//! ```rust,ignore
//! use goose::agents::orchestration::DelegationStrategy;
//! ```

pub use super::delegation::DelegationStrategy;
pub use super::subagent_execution_tool;

// pub(crate) re-exports — accessible within the crate only
// Specialist handler is pub(crate) in the flat namespace; when code migrates
// to use orchestration::specialist_handler, remove the #[allow].
#[allow(unused_imports)]
pub(crate) use super::specialist_handler;
