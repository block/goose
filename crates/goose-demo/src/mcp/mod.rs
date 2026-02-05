//! MCP (Model Context Protocol) integration
//!
//! This module handles connections to MCP servers and bridging
//! MCP tools to rig's tool system.

mod connection;
mod bridge;

pub use connection::McpConnection;
pub use bridge::McpToolBridge;
