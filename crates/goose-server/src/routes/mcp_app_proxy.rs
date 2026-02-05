//! MCP App Proxy - Serves MCP App HTML in a secure double-iframe architecture.
//!
//! This module provides a secure way to render MCP App HTML content by:
//! 1. Accepting HTML via POST and storing it temporarily with a token
//! 2. Serving a bridge template (outer iframe) that provides a secure context
//! 3. Serving the actual HTML content (inner iframe) in a sandboxed iframe
//!
//! The double-iframe architecture provides:
//! - Secure context (real URL origin) for Web APIs like Web Payments
//! - Sandbox isolation for the actual MCP App content
//! - CSP enforcement via HTTP headers on both iframes

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

/// HTML template for the bridge (outer iframe)
const BRIDGE_TEMPLATE: &str = include_str!("templates/mcp_app_proxy.html");

/// How long tokens remain valid
const TOKEN_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Stored data for each token
struct StoredApp {
    html: String,
    csp: CspConfig,
    permissions: PermissionsConfig,
    created_at: Instant,
}

/// In-memory store for HTML content, keyed by token
#[derive(Clone, Default)]
pub struct McpAppProxyStore {
    apps: Arc<RwLock<HashMap<String, StoredApp>>>,
}

impl McpAppProxyStore {
    pub fn new() -> Self {
        Self::default()
    }

    async fn store(&self, html: String, csp: CspConfig, permissions: PermissionsConfig) -> String {
        let token = Uuid::new_v4().to_string();
        let mut apps = self.apps.write().await;

        // Clean up expired tokens
        apps.retain(|_, app| app.created_at.elapsed() < TOKEN_TTL);

        apps.insert(
            token.clone(),
            StoredApp {
                html,
                csp,
                permissions,
                created_at: Instant::now(),
            },
        );
        token
    }

    async fn get(&self, token: &str) -> Option<(String, CspConfig, PermissionsConfig)> {
        let apps = self.apps.read().await;
        apps.get(token).and_then(|app| {
            if app.created_at.elapsed() < TOKEN_TTL {
                Some((app.html.clone(), app.csp.clone(), app.permissions.clone()))
            } else {
                None
            }
        })
    }
}

/// CSP configuration from the MCP server
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CspConfig {
    #[serde(default)]
    pub connect_domains: Vec<String>,
    #[serde(default)]
    pub resource_domains: Vec<String>,
    #[serde(default)]
    pub frame_domains: Vec<String>,
    #[serde(default)]
    pub base_uri_domains: Vec<String>,
}

/// Permissions configuration from the MCP server
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsConfig {
    #[serde(default)]
    pub camera: bool,
    #[serde(default)]
    pub microphone: bool,
    #[serde(default)]
    pub geolocation: bool,
    #[serde(default)]
    pub clipboard_write: bool,
}

/// Request body for creating a new MCP App proxy
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProxyRequest {
    pub html: String,
    #[serde(default)]
    pub csp: CspConfig,
    #[serde(default)]
    pub permissions: PermissionsConfig,
}

/// Response from creating a new MCP App proxy
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateProxyResponse {
    pub token: String,
}

/// Build CSP header string from configuration
/// Matches the style from main branch's build_outer_csp function
fn build_csp(csp: &CspConfig) -> String {
    let resources = if csp.resource_domains.is_empty() {
        String::new()
    } else {
        format!(" {}", csp.resource_domains.join(" "))
    };

    let connections = if csp.connect_domains.is_empty() {
        String::new()
    } else {
        format!(" {}", csp.connect_domains.join(" "))
    };

    // For frame-src, we need 'self' to allow the inner iframe
    let frame_src = if csp.frame_domains.is_empty() {
        "frame-src 'self'".to_string()
    } else {
        format!("frame-src 'self' {}", csp.frame_domains.join(" "))
    };

    let base_uris = if csp.base_uri_domains.is_empty() {
        String::new()
    } else {
        format!(" {}", csp.base_uri_domains.join(" "))
    };

    format!(
        "default-src 'none'; \
         script-src 'self' 'unsafe-inline'{resources}; \
         script-src-elem 'self' 'unsafe-inline'{resources}; \
         style-src 'self' 'unsafe-inline'{resources}; \
         style-src-elem 'self' 'unsafe-inline'{resources}; \
         connect-src 'self'{connections}; \
         img-src 'self' data: blob:{resources}; \
         font-src 'self'{resources}; \
         media-src 'self' data: blob:{resources}; \
         {frame_src}; \
         object-src 'none'; \
         base-uri 'self'{base_uris}"
    )
}

/// Build Permissions-Policy header string from configuration
fn build_permissions_policy(permissions: &PermissionsConfig) -> String {
    let mut policies = Vec::new();

    if permissions.camera {
        policies.push("camera=(self)");
    } else {
        policies.push("camera=()");
    }

    if permissions.microphone {
        policies.push("microphone=(self)");
    } else {
        policies.push("microphone=()");
    }

    if permissions.geolocation {
        policies.push("geolocation=(self)");
    } else {
        policies.push("geolocation=()");
    }

    if permissions.clipboard_write {
        policies.push("clipboard-write=(self)");
    } else {
        policies.push("clipboard-write=()");
    }

    policies.join(", ")
}

// === Route Handlers ===

/// POST /mcp-app-proxy - Store HTML and return a token
#[utoipa::path(
    post,
    path = "/mcp-app-proxy",
    request_body = CreateProxyRequest,
    responses(
        (status = 200, description = "Token for accessing the proxy", body = CreateProxyResponse),
    )
)]
async fn create_proxy(
    State(store): State<McpAppProxyStore>,
    Json(request): Json<CreateProxyRequest>,
) -> Json<CreateProxyResponse> {
    let token = store
        .store(request.html, request.csp, request.permissions)
        .await;
    Json(CreateProxyResponse { token })
}

/// GET /mcp-app-proxy/:token - Serve the bridge template (outer iframe)
#[utoipa::path(
    get,
    path = "/mcp-app-proxy/{token}",
    params(
        ("token" = String, Path, description = "Token from create_proxy")
    ),
    responses(
        (status = 200, description = "Bridge HTML page", content_type = "text/html"),
        (status = 404, description = "Token not found or expired"),
    )
)]
async fn get_proxy(State(store): State<McpAppProxyStore>, Path(token): Path<String>) -> Response {
    let Some((_, csp, permissions)) = store.get(&token).await else {
        return (StatusCode::NOT_FOUND, "Token not found or expired").into_response();
    };

    // Build the content URL for the inner iframe
    let content_url = format!("/mcp-app-proxy/{}/content", token);

    // Serialize permissions to JSON for the template
    let permissions_json = serde_json::to_string(&permissions).unwrap_or_else(|_| "{}".to_string());

    // Replace placeholders in the bridge template
    let html = BRIDGE_TEMPLATE
        .replace("{{CONTENT_URL}}", &content_url)
        .replace("{{PERMISSIONS_JSON}}", &permissions_json);

    // Build security headers
    let csp_header = build_csp(&csp);
    let permissions_policy = build_permissions_policy(&permissions);

    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::HeaderName::from_static("referrer-policy"),
                "no-referrer",
            ),
            (
                header::HeaderName::from_static("content-security-policy"),
                csp_header.as_str(),
            ),
            (
                header::HeaderName::from_static("permissions-policy"),
                permissions_policy.as_str(),
            ),
        ],
        Html(html),
    )
        .into_response()
}

/// GET /mcp-app-proxy/:token/content - Serve the actual MCP App HTML (inner iframe)
#[utoipa::path(
    get,
    path = "/mcp-app-proxy/{token}/content",
    params(
        ("token" = String, Path, description = "Token from create_proxy")
    ),
    responses(
        (status = 200, description = "MCP App HTML content", content_type = "text/html"),
        (status = 404, description = "Token not found or expired"),
    )
)]
async fn get_content(State(store): State<McpAppProxyStore>, Path(token): Path<String>) -> Response {
    let Some((html, csp, permissions)) = store.get(&token).await else {
        return (StatusCode::NOT_FOUND, "Token not found or expired").into_response();
    };

    // Build security headers (same as outer iframe)
    let csp_header = build_csp(&csp);
    let permissions_policy = build_permissions_policy(&permissions);

    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::HeaderName::from_static("referrer-policy"),
                "no-referrer",
            ),
            (
                header::HeaderName::from_static("content-security-policy"),
                csp_header.as_str(),
            ),
            (
                header::HeaderName::from_static("permissions-policy"),
                permissions_policy.as_str(),
            ),
        ],
        Html(html),
    )
        .into_response()
}

/// Configure routes for the MCP App Proxy
/// Note: secret_key is kept for API compatibility but not used (auth handled elsewhere)
pub fn routes(_secret_key: String, store: McpAppProxyStore) -> Router {
    Router::new()
        .route("/mcp-app-proxy", post(create_proxy))
        .route("/mcp-app-proxy/{token}", get(get_proxy))
        .route("/mcp-app-proxy/{token}/content", get(get_content))
        .with_state(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_csp_empty() {
        let csp = CspConfig::default();
        let result = build_csp(&csp);
        assert!(result.contains("default-src 'none'"));
        assert!(result.contains("frame-src 'self'"));
    }

    #[test]
    fn test_build_csp_with_domains() {
        let csp = CspConfig {
            connect_domains: vec!["https://api.example.com".to_string()],
            resource_domains: vec!["https://cdn.example.com".to_string()],
            frame_domains: vec!["https://embed.example.com".to_string()],
            base_uri_domains: vec![],
        };
        let result = build_csp(&csp);
        assert!(result.contains("connect-src 'self' https://api.example.com"));
        assert!(result.contains("https://cdn.example.com"));
        assert!(result.contains("frame-src 'self' https://embed.example.com"));
    }

    #[test]
    fn test_build_permissions_policy() {
        let permissions = PermissionsConfig {
            camera: true,
            microphone: false,
            geolocation: false,
            clipboard_write: true,
        };
        let result = build_permissions_policy(&permissions);
        assert!(result.contains("camera=(self)"));
        assert!(result.contains("microphone=()"));
        assert!(result.contains("geolocation=()"));
        assert!(result.contains("clipboard-write=(self)"));
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = McpAppProxyStore::new();
        let html = "<html>test</html>".to_string();
        let csp = CspConfig::default();
        let permissions = PermissionsConfig::default();

        let token = store.store(html.clone(), csp, permissions).await;
        let retrieved = store.get(&token).await;

        assert!(retrieved.is_some());
        let (retrieved_html, _, _) = retrieved.unwrap();
        assert_eq!(retrieved_html, html);
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let store = McpAppProxyStore::new();
        let retrieved = store.get("invalid-token").await;
        assert!(retrieved.is_none());
    }
}
