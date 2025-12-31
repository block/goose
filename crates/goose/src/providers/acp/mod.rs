//! ACP (Agent Client Protocol) provider implementation
//!
//! This provider acts as an ACP client, spawning an ACP agent process
//! (like claude-code-acp) and communicating with it via the ACP protocol.

mod provider;

pub use provider::AcpProvider;
