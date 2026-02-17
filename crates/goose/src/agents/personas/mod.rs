//! Agent persona definitions — data-driven configurations that define agent behavior.
//!
//! Each persona is a "class definition" that becomes an "instance" when spawned
//! via AgentPool or SummonExtension. Personas define:
//! - System prompt / instructions
//! - Tool groups (which MCP extensions to load)
//! - Modes (universal modes like ask/plan/write/review)
//! - Capabilities (what the agent can do)
//!
//! # Usage
//! ```rust,ignore
//! use goose::agents::personas::GooseAgent;
//! let slot = GooseAgent::agent_slot();
//! ```

// Re-export persona types from their current locations
pub use super::developer_agent::DeveloperAgent;
pub use super::goose_agent::GooseAgent;
pub use super::orchestrator_agent::OrchestratorAgent;
pub use super::pm_agent::PmAgent;
pub use super::qa_agent::QaAgent;
pub use super::research_agent::ResearchAgent;
pub use super::security_agent::SecurityAgent;
