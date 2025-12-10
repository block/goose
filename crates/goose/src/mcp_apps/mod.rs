//! MCP Apps - Interactive User Interfaces for MCP (SEP-1865)
//!
//! This module implements the MCP Apps extension which enables MCP servers
//! to deliver interactive user interfaces to hosts. It provides:
//!
//! - UI Resource types with the `ui://` URI scheme
//! - Host context for theme, viewport, and platform information
//! - JSON-RPC message types for bidirectional UI-host communication
//! - CSP (Content Security Policy) configuration
//!
//! Reference: <https://github.com/modelcontextprotocol/ext-apps>

mod host;
mod types;

pub use host::*;
pub use types::*;

/// MCP Apps extension identifier as defined in SEP-1865
pub const EXTENSION_ID: &str = "io.modelcontextprotocol/ui";

/// MCP Apps MIME type for HTML content
pub const MIME_TYPE: &str = "text/html;profile=mcp-app";

/// The URI scheme for MCP Apps resources
pub const URI_SCHEME: &str = "ui://";

/// The metadata key for UI resource URI in tool metadata
pub const UI_RESOURCE_URI_KEY: &str = "ui/resourceUri";

/// Check if a URI is an MCP Apps UI resource
pub fn is_ui_resource_uri(uri: &str) -> bool {
    uri.starts_with(URI_SCHEME)
}

/// Check if a MIME type indicates MCP Apps HTML content
pub fn is_mcp_apps_mime_type(mime_type: &str) -> bool {
    mime_type == MIME_TYPE || mime_type == "text/html"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ui_resource_uri() {
        assert!(is_ui_resource_uri("ui://weather-server/dashboard"));
        assert!(is_ui_resource_uri("ui://sankey/diagram"));
        assert!(!is_ui_resource_uri("file:///tmp/test.html"));
        assert!(!is_ui_resource_uri("https://example.com"));
        assert!(!is_ui_resource_uri(""));
    }

    #[test]
    fn test_is_mcp_apps_mime_type() {
        assert!(is_mcp_apps_mime_type(MIME_TYPE));
        assert!(is_mcp_apps_mime_type("text/html"));
        assert!(!is_mcp_apps_mime_type("application/json"));
        assert!(!is_mcp_apps_mime_type("text/plain"));
    }
}
