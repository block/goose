//! MCP Apps routes for host context and UI communication
//!
//! This module provides API endpoints for MCP Apps (SEP-1865) support:
//! - Host context endpoint for UI iframe initialization
//! - Message handling for UI → Host communication
//! - Proxy endpoint with dynamic CSP based on resource metadata

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use goose::conversation::message::Message;
use goose::mcp_apps::{
    create_initialize_result, default_csp, CspConfig, DisplayMode, HostContextBuilder,
    McpUiInitializeResult, MessageContent, MessageRole, Platform, Theme, UiMessageParams,
};
use goose::session::SessionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::state::AppState;

/// HTML template for MCP Apps proxy with CSP placeholder
const MCP_APPS_PROXY_HTML: &str = include_str!("templates/mcp_apps_proxy.html");

/// CSP placeholder in the HTML template
const CSP_PLACEHOLDER: &str = "{{CSP_CONTENT}}";

/// Query parameters for host context request
#[derive(Debug, Deserialize, ToSchema)]
pub struct HostContextQuery {
    /// Theme preference (light or dark)
    #[serde(default)]
    pub theme: Option<String>,
    /// Display mode (inline, fullscreen, pip)
    #[serde(default)]
    pub display_mode: Option<String>,
    /// Viewport width
    #[serde(default)]
    pub viewport_width: Option<u32>,
    /// Viewport height
    #[serde(default)]
    pub viewport_height: Option<u32>,
    /// User locale (e.g., "en-US")
    #[serde(default)]
    pub locale: Option<String>,
    /// User timezone (e.g., "America/New_York")
    #[serde(default)]
    pub time_zone: Option<String>,
}

/// Response for ui/message request
#[derive(Debug, Serialize, ToSchema)]
pub struct UiMessageResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Get the MCP Apps host context for UI iframe initialization
///
/// This endpoint returns the host context that UI iframes need during
/// their initialization phase. The context includes theme, viewport,
/// platform info, and host capabilities.
#[utoipa::path(
    get,
    path = "/mcp-apps/context",
    params(
        ("theme" = Option<String>, Query, description = "Theme preference (light or dark)"),
        ("display_mode" = Option<String>, Query, description = "Display mode (inline, fullscreen, pip)"),
        ("viewport_width" = Option<u32>, Query, description = "Viewport width in pixels"),
        ("viewport_height" = Option<u32>, Query, description = "Viewport height in pixels"),
        ("locale" = Option<String>, Query, description = "User locale (BCP 47, e.g., 'en-US')"),
        ("time_zone" = Option<String>, Query, description = "User timezone (IANA, e.g., 'America/New_York')"),
    ),
    responses(
        (status = 200, description = "MCP Apps host context", body = McpUiInitializeResult),
    ),
    tag = "mcp-apps"
)]
pub async fn get_host_context(
    Query(params): Query<HostContextQuery>,
) -> Json<McpUiInitializeResult> {
    let mut builder = HostContextBuilder::new();

    // Set theme
    if let Some(theme_str) = params.theme {
        let theme = match theme_str.to_lowercase().as_str() {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        };
        builder = builder.theme(theme);
    } else {
        builder = builder.theme(Theme::Light);
    }

    // Set display mode
    if let Some(mode_str) = params.display_mode {
        let mode = match mode_str.to_lowercase().as_str() {
            "fullscreen" => DisplayMode::Fullscreen,
            "pip" => DisplayMode::Pip,
            _ => DisplayMode::Inline,
        };
        builder = builder.display_mode(mode);
    } else {
        builder = builder.display_mode(DisplayMode::Inline);
    }

    // Set viewport
    if let (Some(width), Some(height)) = (params.viewport_width, params.viewport_height) {
        builder = builder.viewport(width, height);
    }

    // Set locale
    if let Some(locale) = params.locale {
        builder = builder.locale(locale);
    }

    // Set timezone
    if let Some(tz) = params.time_zone {
        builder = builder.time_zone(tz);
    }

    // Always set platform to Desktop for goose-server
    builder = builder.platform(Platform::Desktop);

    let context = builder.build();
    let result = create_initialize_result(context);

    Json(result)
}

/// Handle ui/message request from UI iframe
///
/// This endpoint receives messages from UI iframes that want to add
/// content to the chat conversation. The message is added to the
/// current session's conversation.
#[utoipa::path(
    post,
    path = "/mcp-apps/message",
    request_body = UiMessageParams,
    responses(
        (status = 200, description = "Message added successfully", body = UiMessageResponse),
        (status = 400, description = "Invalid message format"),
        (status = 500, description = "Failed to add message"),
    ),
    tag = "mcp-apps"
)]
pub async fn handle_ui_message(
    State(_state): State<Arc<AppState>>,
    Json(params): Json<UiMessageParams>,
) -> impl IntoResponse {
    // Extract the text from the message content
    let text = match &params.content {
        MessageContent::Text { text } => text.clone(),
    };

    tracing::info!(
        session_id = %params.session_id,
        role = ?params.role,
        text = %text,
        "Received ui/message from MCP Apps UI"
    );

    // Create the appropriate message based on role
    let message = match params.role {
        MessageRole::User => Message::user().with_text(&text),
        MessageRole::Assistant => Message::assistant().with_text(&text),
    };

    // Add the message to the session
    match SessionManager::add_message(&params.session_id, &message).await {
        Ok(()) => {
            tracing::info!(
                session_id = %params.session_id,
                "Successfully added message to session"
            );
            (
                StatusCode::OK,
                Json(UiMessageResponse {
                    success: true,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %params.session_id,
                error = %e,
                "Failed to add message to session"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UiMessageResponse {
                    success: false,
                    error: Some(format!("Failed to add message: {}", e)),
                }),
            )
        }
    }
}

/// Query parameters for MCP Apps proxy request
#[derive(Debug, Deserialize, ToSchema)]
pub struct McpAppsProxyQuery {
    /// Secret key for authentication
    pub secret: String,
    /// Comma-separated list of connect domains for CSP
    #[serde(default)]
    pub connect_domains: Option<String>,
    /// Comma-separated list of resource domains for CSP
    #[serde(default)]
    pub resource_domains: Option<String>,
}

/// MCP Apps proxy endpoint with dynamic CSP
///
/// This endpoint serves the MCP Apps proxy HTML with CSP headers
/// dynamically generated based on the resource's metadata.
/// The CSP is constructed from the connect_domains and resource_domains
/// parameters, which should come from the MCP server's resource _meta.csp.
#[utoipa::path(
    get,
    path = "/mcp-apps-proxy",
    params(
        ("secret" = String, Query, description = "Secret key for authentication"),
        ("connect_domains" = Option<String>, Query, description = "Comma-separated connect domains for CSP"),
        ("resource_domains" = Option<String>, Query, description = "Comma-separated resource domains for CSP"),
    ),
    responses(
        (status = 200, description = "MCP Apps proxy HTML page", content_type = "text/html"),
        (status = 401, description = "Unauthorized - invalid or missing secret"),
    ),
    tag = "mcp-apps"
)]
async fn mcp_apps_proxy(
    axum::extract::State(secret_key): axum::extract::State<String>,
    Query(params): Query<McpAppsProxyQuery>,
) -> Response {
    // Validate secret key
    if params.secret != secret_key {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    // Build CSP config from query parameters
    let csp_config = CspConfig {
        connect_domains: params
            .connect_domains
            .map(|s| s.split(',').map(|d| d.trim().to_string()).collect())
            .unwrap_or_default(),
        resource_domains: params
            .resource_domains
            .map(|s| s.split(',').map(|d| d.trim().to_string()).collect())
            .unwrap_or_default(),
    };

    // Generate CSP string - use default if no domains provided
    let csp_string =
        if csp_config.connect_domains.is_empty() && csp_config.resource_domains.is_empty() {
            default_csp()
        } else {
            csp_config.to_csp_string()
        };

    // Replace placeholder in HTML template
    let html = MCP_APPS_PROXY_HTML.replace(CSP_PLACEHOLDER, &csp_string);

    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::HeaderName::from_static("referrer-policy"),
                "no-referrer",
            ),
        ],
        Html(html),
    )
        .into_response()
}

/// Configure MCP Apps routes
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/mcp-apps/context", get(get_host_context))
        .route("/mcp-apps/message", post(handle_ui_message))
        .with_state(state)
}

/// Configure MCP Apps proxy route (requires secret key)
pub fn proxy_routes(secret_key: String) -> Router {
    Router::new()
        .route("/mcp-apps-proxy", get(mcp_apps_proxy))
        .with_state(secret_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_context_query_defaults() {
        let query: HostContextQuery = serde_json::from_str("{}").unwrap();
        assert!(query.theme.is_none());
        assert!(query.display_mode.is_none());
        assert!(query.viewport_width.is_none());
        assert!(query.viewport_height.is_none());
    }

    #[test]
    fn test_host_context_query_with_values() {
        let query: HostContextQuery = serde_json::from_str(
            r#"{
                "theme": "dark",
                "display_mode": "fullscreen",
                "viewport_width": 800,
                "viewport_height": 600,
                "locale": "en-US",
                "time_zone": "America/New_York"
            }"#,
        )
        .unwrap();

        assert_eq!(query.theme, Some("dark".to_string()));
        assert_eq!(query.display_mode, Some("fullscreen".to_string()));
        assert_eq!(query.viewport_width, Some(800));
        assert_eq!(query.viewport_height, Some(600));
        assert_eq!(query.locale, Some("en-US".to_string()));
        assert_eq!(query.time_zone, Some("America/New_York".to_string()));
    }

    #[test]
    fn test_ui_message_response_serialization() {
        let response = UiMessageResponse {
            success: true,
            error: None,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["success"], true);
        assert!(json.get("error").is_none());

        let response_with_error = UiMessageResponse {
            success: false,
            error: Some("Test error".to_string()),
        };
        let json = serde_json::to_value(&response_with_error).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Test error");
    }

    #[test]
    fn test_mcp_apps_proxy_query_parsing() {
        let query: McpAppsProxyQuery = serde_json::from_str(
            r#"{
                "secret": "test-secret",
                "connect_domains": "https://api.example.com,wss://ws.example.com",
                "resource_domains": "https://cdn.example.com"
            }"#,
        )
        .unwrap();

        assert_eq!(query.secret, "test-secret");
        assert_eq!(
            query.connect_domains,
            Some("https://api.example.com,wss://ws.example.com".to_string())
        );
        assert_eq!(
            query.resource_domains,
            Some("https://cdn.example.com".to_string())
        );
    }

    #[test]
    fn test_mcp_apps_proxy_query_defaults() {
        let query: McpAppsProxyQuery =
            serde_json::from_str(r#"{"secret": "test-secret"}"#).unwrap();

        assert_eq!(query.secret, "test-secret");
        assert!(query.connect_domains.is_none());
        assert!(query.resource_domains.is_none());
    }

    #[test]
    fn test_csp_generation_from_query() {
        // Test with domains
        let csp_config = CspConfig {
            connect_domains: vec![
                "https://api.example.com".to_string(),
                "wss://ws.example.com".to_string(),
            ],
            resource_domains: vec!["https://cdn.example.com".to_string()],
        };
        let csp = csp_config.to_csp_string();

        // Verify connect domains
        assert!(csp.contains("https://api.example.com"));
        assert!(csp.contains("wss://ws.example.com"));

        // Verify resource domains in all resource directives
        assert!(csp.contains("img-src 'self' data: https://cdn.example.com"));
        assert!(csp.contains("font-src 'self' https://cdn.example.com"));
        assert!(csp.contains("media-src 'self' https://cdn.example.com"));
        assert!(csp.contains("script-src 'self' 'unsafe-inline' https://cdn.example.com"));
        assert!(csp.contains("style-src 'self' 'unsafe-inline' https://cdn.example.com"));

        // Test default (empty) CSP
        let default = default_csp();
        assert!(default.contains("default-src 'none'"));
        assert!(default.contains("script-src 'self' 'unsafe-inline'"));
        assert!(default.contains("media-src 'self'"));
    }

    #[test]
    fn test_html_template_has_csp_placeholder() {
        assert!(
            MCP_APPS_PROXY_HTML.contains(CSP_PLACEHOLDER),
            "HTML template must contain CSP placeholder"
        );
    }
}
