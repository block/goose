//! Extension management — MCP server lifecycle, tool dispatch, and configuration.
//!
//! This module groups all extension-related components:
//! - `ExtensionManager` — manages MCP extension connections and tool dispatch
//! - `ExtensionConfig` — configuration for different extension types (Builtin, Stdio, SSE, etc.)
//! - `ExtensionRegistry` — shared registry of available extensions across sessions
//! - `McpClient` — MCP protocol client for communicating with extension servers
//!
//! # Usage
//! ```rust,ignore
//! use goose::agents::extensions::{ExtensionManager, ExtensionConfig, ExtensionRegistry};
//! ```

// Core extension types (all pub)
pub use super::extension::PlatformExtensionContext;
pub use super::extension::PLATFORM_EXTENSIONS;
pub use super::extension::{Envs, ExtensionConfig, ExtensionError, ExtensionInfo, ExtensionResult};
pub use super::extension_malware_check;
pub use super::extension_manager::ExtensionManager;
pub use super::extension_manager::{get_parameter_names, get_tool_owner};
pub use super::extension_registry::ExtensionRegistry;

// MCP client
pub use super::mcp_client::{Error as McpError, McpClient, McpClientTrait};

// Platform tools
pub use super::platform_tools;
pub use super::extension_manager_extension;
