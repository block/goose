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
