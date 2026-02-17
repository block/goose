//! A2A (Agent-to-Agent) Protocol compatibility layer for Goose.
//!
//! Provides bidirectional converters between Goose messages and A2A protocol
//! messages, plus an `AgentExecutor` implementation that bridges A2A requests
//! to Goose's `Agent::reply()` streaming interface.

pub mod card;
pub mod executor;
pub mod message;

pub use card::build_agent_card;
pub use executor::GooseAgentExecutor;
pub use message::{a2a_message_to_goose, goose_message_to_a2a};
