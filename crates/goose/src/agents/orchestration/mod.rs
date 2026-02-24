//! Orchestration — delegation, sub-agent management, and multi-agent coordination.
//!
//! This module groups delegation and orchestration components:
//! - `DelegationStrategy` — routing strategy for task delegation (in-process, ACP, A2A)
//! - `Dispatcher` — execution backend abstraction (in-process, A2A, composite)
//! - `SubagentExecutionTool` — notification events for sub-agent progress
//!
//! # Usage
//! ```rust,ignore
//! use goose::agents::orchestration::{DelegationStrategy, Dispatcher, CompositeDispatcher};
//! ```

pub use super::delegation::DelegationStrategy;
pub use super::dispatch::{
    A2ADispatcher, CompositeDispatcher, DispatchEvent, DispatchResult, DispatchStatus, Dispatcher,
    InProcessDispatcher,
};
pub use super::subagent_execution_tool;

pub mod memory;

// pub(crate) re-exports — accessible within the crate only
// Specialist handler is pub(crate) in the flat namespace; when code migrates
// to use orchestration::specialist_handler, remove the #[allow].
#[allow(unused_imports)]
pub(crate) use super::specialist_handler;
