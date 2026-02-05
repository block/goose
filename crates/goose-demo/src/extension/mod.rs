//! Extension system for goose2
//!
//! Extensions provide tools to the agent. They can be:
//! - **Native**: Built directly in Rust (e.g., DevelopExtension)
//! - **MCP**: Backed by an MCP server process
//!
//! The system has two layers:
//! - `ExtensionCatalog`: Global, shared across sessions, loaded from config
//! - `EnabledExtensions`: Session-scoped, owns running extension instances

mod builtin;
mod catalog;
mod config;
mod enabled;
mod mcp;
mod preamble;

pub use builtin::{builtin_tools, is_builtin_tool, DISABLE_EXTENSION, ENABLE_EXTENSION};
pub use catalog::ExtensionCatalog;
pub use config::{ExtensionConfig, ExtensionKind};
pub use enabled::{EnabledExtensions, ToolIndex};
pub use mcp::McpExtension;
pub use preamble::generate_preamble;

use async_trait::async_trait;
use rig::completion::ToolDefinition;
use serde_json::{Map, Value};

use crate::Result;

/// An extension provides tools to the agent.
///
/// Extensions are the abstraction layer between the agent and tool providers.
/// They can be backed by MCP servers or implemented natively in Rust.
///
/// The `#[async_trait]` macro is required to make this trait dyn-compatible
/// while still supporting async methods.
#[async_trait]
pub trait Extension: Send + Sync {
    /// Unique name for this extension (e.g., "develop", "browser")
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Instructions to include in the system prompt when this extension is enabled.
    /// These help the agent understand how to use the extension's tools effectively.
    fn instructions(&self) -> Option<&str> {
        None
    }

    /// List all tools provided by this extension.
    /// Tool names should be prefixed with `{extension_name}__` to avoid collisions.
    async fn list_tools(&self) -> Result<Vec<ToolDefinition>>;

    /// Call a tool by name.
    ///
    /// The `name` parameter is the full tool name including the extension prefix.
    async fn call_tool(&self, name: &str, arguments: Option<Map<String, Value>>) -> Result<String>;
}
