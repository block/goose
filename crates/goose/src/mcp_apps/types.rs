//! MCP Apps type definitions (SEP-1865)
//!
//! This module defines the types used for MCP Apps interactive UI support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// ============================================================================
// UI Resource Types
// ============================================================================

/// Content Security Policy configuration for UI resources.
///
/// Servers declare which external origins their UI needs to access.
/// Hosts use this to enforce appropriate CSP headers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CspConfig {
    /// Origins for network requests (fetch/XHR/WebSocket).
    /// Maps to CSP `connect-src` directive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connect_domains: Vec<String>,

    /// Origins for static resources (images, scripts, stylesheets, fonts).
    /// Wildcard subdomains supported: `https://*.example.com`
    /// Maps to CSP `img-src`, `script-src`, `style-src`, `font-src` directives.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_domains: Vec<String>,
}

/// UI resource metadata for security and rendering configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UIResourceMeta {
    /// Content Security Policy configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csp: Option<CspConfig>,

    /// Dedicated origin for the widget's sandbox.
    /// Useful when widgets need dedicated origins for API key allowlists
    /// or cross-origin isolation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Visual boundary preference.
    /// `true` requests a visible border (host decides styling).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}

// ============================================================================
// Host Context Types
// ============================================================================

/// Color theme preference.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

/// How the UI is currently displayed.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    #[default]
    Inline,
    Fullscreen,
    Pip,
}

/// Platform type for responsive design.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Web,
    #[default]
    Desktop,
    Mobile,
}

/// Viewport dimensions available to the UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
}

/// Device capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct DeviceCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touch: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<bool>,
}

/// Safe area boundaries in pixels (for notched displays, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SafeAreaInsets {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

/// Host context provided to UI iframes during initialization.
///
/// All fields are optional. Hosts SHOULD provide relevant context.
/// Guest UIs SHOULD handle missing fields gracefully.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostContext {
    /// Current color theme preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,

    /// How the UI is currently displayed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_mode: Option<DisplayMode>,

    /// Display modes the host supports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_display_modes: Vec<DisplayMode>,

    /// Current and maximum dimensions available to the UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,

    /// User's language/region preference (BCP 47, e.g., "en-US").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    /// User's timezone (IANA, e.g., "America/New_York").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,

    /// Host application identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Platform type for responsive design.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,

    /// Device capabilities such as touch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_capabilities: Option<DeviceCapabilities>,

    /// Safe area boundaries in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_area_insets: Option<SafeAreaInsets>,
}

// ============================================================================
// Host Capabilities
// ============================================================================

/// Capabilities that the host supports for MCP Apps.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    /// MIME types the host can render.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mime_types: Vec<String>,

    /// Whether the host supports the `ui/open-link` request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_link: Option<bool>,

    /// Whether the host supports the `ui/message` request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<bool>,
}

/// Host information provided during initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct HostInfo {
    pub name: String,
    pub version: String,
}

// ============================================================================
// JSON-RPC Message Types
// ============================================================================

/// Initialize request from UI to Host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpUiInitializeParams {
    pub protocol_version: String,
}

/// Initialize response from Host to UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpUiInitializeResult {
    pub protocol_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_capabilities: Option<HostCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_info: Option<HostInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_context: Option<HostContext>,
}

/// Request to open an external URL (UI → Host).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct OpenLinkParams {
    pub url: String,
}

/// Message content type for ui/message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageContent {
    Text { text: String },
}

/// Message role for ui/message.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

/// Request to send a message to the chat (UI → Host).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UiMessageParams {
    /// The session ID to add the message to
    pub session_id: String,
    /// The role of the message sender
    pub role: MessageRole,
    /// The content of the message
    pub content: MessageContent,
}

/// Tool input notification (Host → UI).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolInputParams {
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Tool result notification (Host → UI).
/// Uses the standard MCP CallToolResult type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    pub content: Vec<serde_json::Value>,
}

/// Tool cancelled notification (Host → UI).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCancelledParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// ============================================================================
// JSON-RPC Method Names
// ============================================================================

/// JSON-RPC method names for MCP Apps protocol.
pub mod methods {
    /// Initialize request (UI → Host)
    pub const INITIALIZE: &str = "initialize";

    /// Open external link request (UI → Host)
    pub const OPEN_LINK: &str = "ui/open-link";

    /// Send message to chat request (UI → Host)
    pub const MESSAGE: &str = "ui/message";

    /// Tool input notification (Host → UI)
    pub const TOOL_INPUT: &str = "ui/notifications/tool-input";

    /// Partial tool input notification (Host → UI)
    pub const TOOL_INPUT_PARTIAL: &str = "ui/notifications/tool-input-partial";

    /// Tool result notification (Host → UI)
    pub const TOOL_RESULT: &str = "ui/notifications/tool-result";

    /// Tool cancelled notification (Host → UI)
    pub const TOOL_CANCELLED: &str = "ui/tool-cancelled";
}

// ============================================================================
// CSP Generation
// ============================================================================

impl CspConfig {
    /// Generate a Content Security Policy string from the configuration.
    ///
    /// This follows the SEP-1865 specification for CSP construction:
    /// - connectDomains -> connect-src
    /// - resourceDomains -> img-src, font-src, media-src, script-src, style-src
    ///
    /// Per the spec, media-src includes 'data:' for inline audio/video.
    pub fn to_csp_string(&self) -> String {
        let connect_src = if self.connect_domains.is_empty() {
            String::new()
        } else {
            format!(" {}", self.connect_domains.join(" "))
        };

        let resource_domains = if self.resource_domains.is_empty() {
            String::new()
        } else {
            format!(" {}", self.resource_domains.join(" "))
        };

        // Per SEP-1865 spec: media-src includes 'data:' like img-src
        format!(
            "default-src 'none'; \
             script-src 'self' 'unsafe-inline'{resource_domains}; \
             style-src 'self' 'unsafe-inline'{resource_domains}; \
             connect-src 'self'{connect_src}; \
             img-src 'self' data:{resource_domains}; \
             font-src 'self'{resource_domains}; \
             media-src 'self' data:{resource_domains}; \
             frame-src 'none'; \
             object-src 'none'; \
             base-uri 'self';"
        )
    }
}

/// Generate the default restrictive CSP when no configuration is provided.
pub fn default_csp() -> String {
    CspConfig::default().to_csp_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csp_config_default() {
        let csp = CspConfig::default();
        let csp_string = csp.to_csp_string();
        assert!(csp_string.contains("default-src 'none'"));
        assert!(csp_string.contains("script-src 'self' 'unsafe-inline'"));
        assert!(csp_string.contains("connect-src 'self'"));
        assert!(!csp_string.contains("api.example.com"));
    }

    #[test]
    fn test_csp_config_with_domains() {
        let csp = CspConfig {
            connect_domains: vec![
                "https://api.weather.com".to_string(),
                "wss://realtime.service.com".to_string(),
            ],
            resource_domains: vec!["https://cdn.jsdelivr.net".to_string()],
        };
        let csp_string = csp.to_csp_string();

        // Verify connect domains in connect-src
        assert!(csp_string
            .contains("connect-src 'self' https://api.weather.com wss://realtime.service.com"));

        // Verify resource domains in all resource directives
        assert!(csp_string.contains("img-src 'self' data: https://cdn.jsdelivr.net"));
        assert!(csp_string.contains("font-src 'self' https://cdn.jsdelivr.net"));
        assert!(csp_string.contains("media-src 'self' data: https://cdn.jsdelivr.net"));
        assert!(csp_string.contains("script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net"));
        assert!(csp_string.contains("style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net"));

        // Verify security directives
        assert!(csp_string.contains("frame-src 'none'"));
        assert!(csp_string.contains("object-src 'none'"));
        assert!(csp_string.contains("base-uri 'self'"));
    }

    #[test]
    fn test_csp_all_directives_present() {
        let csp = CspConfig::default();
        let csp_string = csp.to_csp_string();

        // All CSP directives should be present
        assert!(csp_string.contains("default-src"));
        assert!(csp_string.contains("script-src"));
        assert!(csp_string.contains("style-src"));
        assert!(csp_string.contains("connect-src"));
        assert!(csp_string.contains("img-src"));
        assert!(csp_string.contains("font-src"));
        assert!(csp_string.contains("media-src"));
        assert!(csp_string.contains("frame-src"));
        assert!(csp_string.contains("object-src"));
        assert!(csp_string.contains("base-uri"));
    }

    #[test]
    fn test_host_context_serialization() {
        let context = HostContext {
            theme: Some(Theme::Dark),
            display_mode: Some(DisplayMode::Inline),
            viewport: Some(Viewport {
                width: 400,
                height: 300,
                max_width: None,
                max_height: None,
            }),
            ..Default::default()
        };

        let json = serde_json::to_value(&context).unwrap();
        assert_eq!(json["theme"], "dark");
        assert_eq!(json["displayMode"], "inline");
        assert_eq!(json["viewport"]["width"], 400);
    }

    #[test]
    fn test_host_context_deserialization() {
        let json = serde_json::json!({
            "theme": "dark",
            "displayMode": "fullscreen",
            "locale": "en-US",
            "platform": "desktop"
        });

        let context: HostContext = serde_json::from_value(json).unwrap();
        assert_eq!(context.theme, Some(Theme::Dark));
        assert_eq!(context.display_mode, Some(DisplayMode::Fullscreen));
        assert_eq!(context.locale, Some("en-US".to_string()));
        assert_eq!(context.platform, Some(Platform::Desktop));
    }

    #[test]
    fn test_ui_resource_meta_serialization() {
        let meta = UIResourceMeta {
            csp: Some(CspConfig {
                connect_domains: vec!["https://api.example.com".to_string()],
                resource_domains: vec![],
            }),
            domain: Some("https://widget.example.com".to_string()),
            prefers_border: Some(true),
        };

        let json = serde_json::to_value(&meta).unwrap();
        assert!(json["csp"]["connectDomains"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("https://api.example.com")));
        assert_eq!(json["domain"], "https://widget.example.com");
        assert_eq!(json["prefersBorder"], true);
    }

    #[test]
    fn test_initialize_result() {
        let result = McpUiInitializeResult {
            protocol_version: "2025-06-18".to_string(),
            host_capabilities: Some(HostCapabilities {
                mime_types: vec![super::super::MIME_TYPE.to_string()],
                open_link: Some(true),
                message: Some(true),
            }),
            host_info: Some(HostInfo {
                name: "goose".to_string(),
                version: "1.0.0".to_string(),
            }),
            host_context: Some(HostContext {
                theme: Some(Theme::Dark),
                ..Default::default()
            }),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["protocolVersion"], "2025-06-18");
        assert_eq!(json["hostInfo"]["name"], "goose");
    }

    #[test]
    fn test_open_link_params() {
        let params = OpenLinkParams {
            url: "https://example.com".to_string(),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["url"], "https://example.com");
    }

    #[test]
    fn test_ui_message_params() {
        let params = UiMessageParams {
            session_id: "test-session-123".to_string(),
            role: MessageRole::User,
            content: MessageContent::Text {
                text: "Hello, world!".to_string(),
            },
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["sessionId"], "test-session-123");
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"]["type"], "text");
        assert_eq!(json["content"]["text"], "Hello, world!");
    }

    #[test]
    fn test_tool_input_params() {
        let mut arguments = HashMap::new();
        arguments.insert("location".to_string(), serde_json::json!("San Francisco"));
        arguments.insert("units".to_string(), serde_json::json!("celsius"));

        let params = ToolInputParams { arguments };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["arguments"]["location"], "San Francisco");
        assert_eq!(json["arguments"]["units"], "celsius");
    }

    #[test]
    fn test_tool_cancelled_params() {
        let params = ToolCancelledParams {
            reason: Some("User cancelled".to_string()),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["reason"], "User cancelled");

        let params_no_reason = ToolCancelledParams { reason: None };
        let json_no_reason = serde_json::to_value(&params_no_reason).unwrap();
        assert!(json_no_reason.get("reason").is_none());
    }
}
