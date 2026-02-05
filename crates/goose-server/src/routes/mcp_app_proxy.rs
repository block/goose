//! MCP App Proxy - Secure Context with Sandboxed Iframe
//!
//! This module provides endpoints for serving MCP App HTML content using a
//! double-iframe architecture that provides both:
//! - Secure context (real URL origin)
//! - Sandbox isolation (restricted iframe capabilities)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  Host (Electron/Browser)                                │
//! │  ┌───────────────────────────────────────────────────┐  │
//! │  │  Outer iframe (src="/mcp-app-proxy/{token}")      │  │  ← Real URL = Secure Context
//! │  │  CSP + Permissions-Policy headers from server     │  │
//! │  │  ┌─────────────────────────────────────────────┐  │  │
//! │  │  │  mcp_app_proxy.html (bridge template)       │  │  │  ← Message forwarding
//! │  │  │  ┌───────────────────────────────────────┐  │  │  │
//! │  │  │  │  Inner iframe (sandboxed)             │  │  │  │  ← sandbox="allow-scripts allow-same-origin"
//! │  │  │  │  src="/mcp-app-proxy/{token}/content" │  │  │  │
//! │  │  │  │  ┌─────────────────────────────────┐  │  │  │  │
//! │  │  │  │  │  Actual MCP App HTML            │  │  │  │  │
//! │  │  │  │  └─────────────────────────────────┘  │  │  │  │
//! │  │  │  └───────────────────────────────────────┘  │  │  │
//! │  │  └─────────────────────────────────────────────┘  │  │
//! │  └───────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why double iframe?
//!
//! 1. **Outer iframe** has a real URL, giving it a proper origin and secure context.
//!    This is required for Web Payments SDK, WebAuthn, certain OAuth flows, etc.
//!
//! 2. **Inner iframe** is sandboxed, restricting what the MCP App can do.
//!    This is required by the MCP Apps spec for security.
//!
//! 3. **Bridge template** forwards postMessage between host and inner iframe,
//!    maintaining the auditable communication channel.
//!
//! ## How it works
//!
//! 1. Frontend POSTs MCP App HTML + metadata to `/mcp-app-proxy`
//! 2. Backend stores it and returns a unique token
//! 3. Frontend creates iframe with `src="/mcp-app-proxy/{token}"`
//! 4. Backend serves the bridge template with CSP headers
//! 5. Bridge template creates sandboxed inner iframe with `src="/mcp-app-proxy/{token}/content"`
//! 6. Backend serves the actual MCP App HTML to the inner iframe
//!
//! ## Security
//!
//! - Tokens expire after 60 seconds
//! - Secret key required for POST
//! - CSP headers enforced on outer iframe
//! - Inner iframe is sandboxed with minimal permissions

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
use uuid::Uuid;

/// How long a token is valid before it expires
const TOKEN_TTL: Duration = Duration::from_secs(60);

/// The bridge template HTML (loaded at compile time)
const BRIDGE_TEMPLATE: &str = include_str!("templates/mcp_app_proxy.html");

/// Stored MCP App content waiting to be served
#[derive(Clone)]
struct PendingApp {
    html: String,
    csp: CspConfig,
    permissions: PermissionsConfig,
    created_at: Instant,
}

/// CSP configuration from _meta.ui.csp
#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CspConfig {
    pub connect_domains: Option<Vec<String>>,
    pub resource_domains: Option<Vec<String>>,
    pub frame_domains: Option<Vec<String>>,
    pub base_uri_domains: Option<Vec<String>>,
}

/// Permissions configuration from _meta.ui.permissions
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsConfig {
    pub camera: Option<bool>,
    pub microphone: Option<bool>,
    pub geolocation: Option<bool>,
    pub clipboard_write: Option<bool>,
}

/// In-memory store for pending MCP App content
#[derive(Clone, Default)]
pub struct McpAppProxyStore {
    pending: Arc<RwLock<HashMap<String, PendingApp>>>,
}

impl McpAppProxyStore {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store HTML content and return a token
    async fn store(&self, html: String, csp: CspConfig, permissions: PermissionsConfig) -> String {
        let token = Uuid::new_v4().to_string();
        let app = PendingApp {
            html,
            csp,
            permissions,
            created_at: Instant::now(),
        };

        let mut pending = self.pending.write().await;

        // Clean up expired tokens while we have the lock
        pending.retain(|_, app| app.created_at.elapsed() < TOKEN_TTL);

        pending.insert(token.clone(), app);
        token
    }

    /// Get app by token (does not consume - needed for double fetch)
    async fn get(&self, token: &str) -> Option<PendingApp> {
        let pending = self.pending.read().await;

        pending.get(token).and_then(|app| {
            if app.created_at.elapsed() < TOKEN_TTL {
                Some(app.clone())
            } else {
                None
            }
        })
    }

    /// Remove expired tokens (called periodically)
    async fn cleanup(&self) {
        let mut pending = self.pending.write().await;
        pending.retain(|_, app| app.created_at.elapsed() < TOKEN_TTL);
    }
}

/// Shared state for the MCP App Proxy routes
#[derive(Clone)]
struct AppState {
    secret_key: String,
    store: McpAppProxyStore,
}

/// Request body for POST /mcp-app-proxy
#[derive(Deserialize)]
struct CreateProxyRequest {
    secret: String,
    html: String,
    #[serde(default)]
    csp: CspConfig,
    #[serde(default)]
    permissions: PermissionsConfig,
}

/// Response body for POST /mcp-app-proxy
#[derive(Serialize)]
struct CreateProxyResponse {
    token: String,
    url: String,
}

/// POST /mcp-app-proxy - Store HTML and get a token
async fn create_proxy(
    State(state): State<AppState>,
    result: Result<Json<CreateProxyRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let req = match result {
        Ok(Json(req)) => req,
        Err(e) => {
            tracing::error!("MCP App Proxy JSON parse error: {}", e);
            return (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)).into_response();
        }
    };

    if req.secret != state.secret_key {
        tracing::warn!(
            "MCP App Proxy auth failed: received secret length={}, expected length={}",
            req.secret.len(),
            state.secret_key.len()
        );
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    // Cleanup expired tokens
    state.store.cleanup().await;

    let token = state.store.store(req.html, req.csp, req.permissions).await;

    Json(CreateProxyResponse {
        url: format!("/mcp-app-proxy/{}", token),
        token,
    })
    .into_response()
}

/// Build CSP header based on declared domains
fn build_csp(csp: &CspConfig) -> String {
    let resources = csp
        .resource_domains
        .as_ref()
        .map(|d| format!(" {}", d.join(" ")))
        .unwrap_or_default();

    let connections = csp
        .connect_domains
        .as_ref()
        .map(|d| format!(" {}", d.join(" ")))
        .unwrap_or_default();

    let frame_src = csp
        .frame_domains
        .as_ref()
        .filter(|d| !d.is_empty())
        .map(|d| format!("frame-src 'self' {}", d.join(" ")))
        .unwrap_or_else(|| "frame-src 'self'".to_string()); // Allow 'self' for inner iframe

    let base_uris = csp
        .base_uri_domains
        .as_ref()
        .map(|d| format!(" {}", d.join(" ")))
        .unwrap_or_default();

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

/// Build Permission-Policy header based on declared permissions
fn build_permission_policy(permissions: &PermissionsConfig) -> Option<String> {
    let mut policies = Vec::new();

    if permissions.camera.unwrap_or(false) {
        policies.push("camera=(self)");
    }
    if permissions.microphone.unwrap_or(false) {
        policies.push("microphone=(self)");
    }
    if permissions.geolocation.unwrap_or(false) {
        policies.push("geolocation=(self)");
    }
    if permissions.clipboard_write.unwrap_or(false) {
        policies.push("clipboard-write=(self)");
    }

    if policies.is_empty() {
        None
    } else {
        Some(policies.join(", "))
    }
}

/// GET /mcp-app-proxy/{token} - Serve the bridge template
async fn get_proxy(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    let Some(app) = state.store.get(&token).await else {
        return (StatusCode::NOT_FOUND, "Token not found or expired").into_response();
    };

    let csp = build_csp(&app.csp);
    let permission_policy = build_permission_policy(&app.permissions);

    // Build the bridge HTML with the content URL and permissions injected
    let content_url = format!("/mcp-app-proxy/{}/content", token);
    let permissions_json =
        serde_json::to_string(&app.permissions).unwrap_or_else(|_| "null".to_string());

    let bridge_html = BRIDGE_TEMPLATE
        .replace("{{CONTENT_URL}}", &content_url)
        .replace("{{PERMISSIONS_JSON}}", &permissions_json);

    // Build response with headers
    let mut response = Html(bridge_html).into_response();
    let headers = response.headers_mut();

    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        csp.parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("referrer-policy"),
        "no-referrer".parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        "nosniff".parse().unwrap(),
    );

    if let Some(pp) = permission_policy {
        headers.insert(
            header::HeaderName::from_static("permissions-policy"),
            pp.parse().unwrap(),
        );
    }

    response
}

/// GET /mcp-app-proxy/{token}/content - Serve the actual MCP App HTML
async fn get_content(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    let Some(app) = state.store.get(&token).await else {
        return (StatusCode::NOT_FOUND, "Token not found or expired").into_response();
    };

    // Build CSP for the content iframe - this is where the actual restrictions matter!
    // The outer iframe's CSP doesn't cascade to iframes loading from different URLs.
    let csp = build_csp(&app.csp);
    let permission_policy = build_permission_policy(&app.permissions);

    let mut response = Html(app.html).into_response();
    let headers = response.headers_mut();

    // Apply CSP to restrict what the MCP App can load
    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        csp.parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("referrer-policy"),
        "no-referrer".parse().unwrap(),
    );
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        "nosniff".parse().unwrap(),
    );

    if let Some(pp) = permission_policy {
        headers.insert(
            header::HeaderName::from_static("permissions-policy"),
            pp.parse().unwrap(),
        );
    }

    response
}

pub fn routes(secret_key: String, store: McpAppProxyStore) -> Router {
    let state = AppState { secret_key, store };

    Router::new()
        .route("/mcp-app-proxy", post(create_proxy))
        .route("/mcp-app-proxy/{token}", get(get_proxy))
        .route("/mcp-app-proxy/{token}/content", get(get_content))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = McpAppProxyStore::new();

        let token = store
            .store(
                "<html>test</html>".to_string(),
                CspConfig::default(),
                PermissionsConfig::default(),
            )
            .await;

        // Can retrieve multiple times (not single-use)
        let app = store.get(&token).await;
        assert!(app.is_some());
        assert_eq!(app.unwrap().html, "<html>test</html>");

        let app2 = store.get(&token).await;
        assert!(app2.is_some());
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let store = McpAppProxyStore::new();
        let app = store.get("invalid-token").await;
        assert!(app.is_none());
    }

    #[test]
    fn test_build_csp() {
        let csp = CspConfig {
            connect_domains: Some(vec!["https://api.example.com".to_string()]),
            resource_domains: Some(vec!["https://cdn.example.com".to_string()]),
            frame_domains: None,
            base_uri_domains: None,
        };

        let result = build_csp(&csp);
        assert!(result.contains("connect-src 'self' https://api.example.com"));
        assert!(result.contains("https://cdn.example.com"));
        // Should allow 'self' for inner iframe
        assert!(result.contains("frame-src 'self'"));
    }

    #[test]
    fn test_build_permission_policy() {
        let permissions = PermissionsConfig {
            camera: Some(true),
            microphone: Some(false),
            geolocation: Some(true),
            clipboard_write: None,
        };

        let result = build_permission_policy(&permissions);
        assert!(result.is_some());
        let policy = result.unwrap();
        assert!(policy.contains("camera=(self)"));
        assert!(policy.contains("geolocation=(self)"));
        assert!(!policy.contains("microphone"));
    }
}
