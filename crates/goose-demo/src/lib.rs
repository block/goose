//! goose2 - A minimal AI agent built on ACP and rig
//!
//! This crate implements an AI agent that:
//! - Exposes itself via ACP (Agent Client Protocol) for client communication
//! - Provides tools via Extensions (native Rust or MCP servers)
//! - Uses rig for LLM provider abstraction
//! - Persists sessions to SQLite for durability
//!
//! ## Architecture
//!
//! ```text
//! Client <--ACP--> Server
//!                    |
//!                    +-- ExtensionCatalog (global, from config)
//!                    |
//!                    v
//!                 Session (cached in memory)
//!                    |
//!                    +-- messages (persisted to DB)
//!                    +-- EnabledExtensions (session-scoped)
//!                    +-- Model
//!                    |
//!                    v
//!               Agent Loop
//!                    |
//!                    +-- builtin tools (enable/disable extensions)
//!                    +-- extension tools
//!                    +-- dynamic preamble generation
//!                    +-- streaming completion
//! ```
//!
//! ## Extensions
//!
//! Extensions provide tools to the agent. They can be:
//! - **Native**: Built directly in Rust (e.g., DevelopExtension)
//! - **MCP**: Backed by an MCP server process
//!
//! The agent can dynamically enable/disable extensions at runtime using
//! built-in `platform__enable_extension` and `platform__disable_extension` tools.

mod agent_loop;
pub mod db;
pub mod error;
pub mod extension;
pub mod mcp;
pub mod notifier;
pub mod provider;
pub mod server;
pub mod session;

pub use db::Database;
pub use error::{Error, Result};
pub use server::Server;
pub use session::Session;
