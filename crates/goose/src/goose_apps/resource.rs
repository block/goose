use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Content Security Policy metadata for MCP Apps
/// Specifies allowed domains for network connections and resource loading
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CspMetadata {
    /// Domains allowed for connect-src (fetch, XHR, WebSocket)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_domains: Option<Vec<String>>,
    /// Domains allowed for resource loading (scripts, styles, images, fonts, media)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_domains: Option<Vec<String>>,
}

/// UI-specific metadata for MCP resources
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiMetadata {
    /// Content Security Policy configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csp: Option<CspMetadata>,
    /// Preferred domain for the app (used for CORS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Whether the app prefers to have a border around it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}

/// Resource metadata containing UI configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMetadata {
    /// UI-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiMetadata>,
}

/// MCP App Resource
/// Represents a UI resource that can be rendered in an MCP App
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpAppResource {
    /// URI of the resource (must use ui:// scheme)
    pub uri: String,
    /// Human-readable name of the resource
    pub name: String,
    /// Optional description of what this resource does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type (should be "text/html;profile=mcp-app" for MCP Apps)
    pub mime_type: String,
    /// Text content of the resource (HTML for MCP Apps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64-encoded binary content (alternative to text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
    /// Resource metadata including UI configuration
    #[serde(skip_serializing_if = "Option::is_none", rename = "_meta")]
    pub meta: Option<ResourceMetadata>,
}

impl McpAppResource {
    pub fn new_html(uri: String, name: String, html: String) -> Self {
        Self {
            uri,
            name,
            description: None,
            mime_type: "text/html;profile=mcp-app".to_string(),
            text: Some(html),
            blob: None,
            meta: None,
        }
    }

    pub fn new_html_with_csp(uri: String, name: String, html: String, csp: CspMetadata) -> Self {
        Self {
            uri,
            name,
            description: None,
            mime_type: "text/html;profile=mcp-app".to_string(),
            text: Some(html),
            blob: None,
            meta: Some(ResourceMetadata {
                ui: Some(UiMetadata {
                    csp: Some(csp),
                    domain: None,
                    prefers_border: None,
                }),
            }),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_ui_metadata(mut self, ui_metadata: UiMetadata) -> Self {
        if let Some(meta) = &mut self.meta {
            meta.ui = Some(ui_metadata);
        } else {
            self.meta = Some(ResourceMetadata {
                ui: Some(ui_metadata),
            });
        }
        self
    }
}
