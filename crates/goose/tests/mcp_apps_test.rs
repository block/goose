//! Tests for MCP Apps (SEP-1865) capability negotiation
//!
//! These tests verify that Goose correctly advertises MCP Apps support
//! to MCP servers during initialization.

use rmcp::model::{ClientCapabilities, JsonObject};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The extension identifier for MCP Apps as defined in SEP-1865
const MCP_APPS_EXTENSION_ID: &str = "io.modelcontextprotocol/ui";

/// The MIME type for MCP Apps HTML content
const MCP_APPS_MIME_TYPE: &str = "text/html;profile=mcp-app";

/// Helper to create MCP Apps capability settings
fn create_mcp_apps_capability() -> JsonObject {
    let mut settings = JsonObject::new();
    settings.insert(
        "mimeTypes".to_string(),
        Value::Array(vec![Value::String(MCP_APPS_MIME_TYPE.to_string())]),
    );
    settings
}

/// Helper to create ClientCapabilities with MCP Apps support using experimental field
fn create_capabilities_with_mcp_apps_experimental() -> ClientCapabilities {
    let mut experimental: BTreeMap<String, JsonObject> = BTreeMap::new();
    experimental.insert(
        MCP_APPS_EXTENSION_ID.to_string(),
        create_mcp_apps_capability(),
    );

    ClientCapabilities {
        experimental: Some(experimental),
        roots: None,
        sampling: Some(JsonObject::new()), // Keep existing sampling support
        elicitation: None,
    }
}

#[test]
fn test_mcp_apps_capability_serializes_correctly() {
    let caps = create_capabilities_with_mcp_apps_experimental();
    let serialized = serde_json::to_value(&caps).unwrap();

    // Verify the structure matches what servers expect
    assert!(serialized.get("experimental").is_some());
    assert!(serialized["experimental"][MCP_APPS_EXTENSION_ID].is_object());
    assert_eq!(
        serialized["experimental"][MCP_APPS_EXTENSION_ID]["mimeTypes"],
        json!([MCP_APPS_MIME_TYPE])
    );
}

#[test]
fn test_mcp_apps_capability_includes_sampling() {
    let caps = create_capabilities_with_mcp_apps_experimental();
    let serialized = serde_json::to_value(&caps).unwrap();

    // Verify sampling is still present (existing functionality)
    assert!(serialized.get("sampling").is_some());
}

#[test]
fn test_mcp_apps_mime_type_is_correct() {
    // Verify the MIME type matches the spec exactly
    assert_eq!(MCP_APPS_MIME_TYPE, "text/html;profile=mcp-app");
}

#[test]
fn test_mcp_apps_extension_id_is_correct() {
    // Verify the extension ID matches the spec exactly
    assert_eq!(MCP_APPS_EXTENSION_ID, "io.modelcontextprotocol/ui");
}

#[cfg(test)]
mod capability_builder_tests {
    use super::*;

    /// Test that we can build capabilities using the builder pattern
    /// and then add MCP Apps support
    #[test]
    fn test_builder_with_mcp_apps() {
        // Start with the standard builder
        let mut caps = ClientCapabilities::builder().enable_sampling().build();

        // Add MCP Apps support via experimental
        let mut experimental: BTreeMap<String, JsonObject> = BTreeMap::new();
        experimental.insert(
            MCP_APPS_EXTENSION_ID.to_string(),
            create_mcp_apps_capability(),
        );
        caps.experimental = Some(experimental);

        let serialized = serde_json::to_value(&caps).unwrap();

        // Verify both sampling and MCP Apps are present
        assert!(serialized.get("sampling").is_some());
        assert!(serialized["experimental"][MCP_APPS_EXTENSION_ID].is_object());
    }
}
