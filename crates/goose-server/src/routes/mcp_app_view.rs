//! MCP App View - Secure Context Endpoint
//!
//! This module provides endpoints for serving MCP App HTML content from a real URL
//! instead of using `srcdoc` iframes. This gives MCP Apps a proper origin and
//! secure context, which is required for:
//! - Web Payments SDK (Square, Stripe, etc.)
//! - WebAuthn / Passkeys
//! - Certain OAuth flows
//! - Any API that checks `window.isSecureContext`
//!
//! ## How it works
//!
//! 1. Frontend POSTs MCP App HTML + metadata to `/mcp-app-view`
//! 2. Backend stores it temporarily and returns a unique token
//! 3. Frontend creates iframe with `src="/mcp-app-view/{token}"`
//! 4. Backend serves the HTML with proper CSP headers
//! 5. The iframe now has a real origin (http://localhost:PORT) which is a secure context
//!
//! ## Security
//!
//! - Tokens are single-use (deleted after first GET)
//! - Tokens expire after 60 seconds
//! - Secret key required for POST
//! - CSP headers enforced based on declared domains

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
struct CspConfig {
    connect_domains: Option<Vec<String>>,
    resource_domains: Option<Vec<String>>,
    frame_domains: Option<Vec<String>>,
    base_uri_domains: Option<Vec<String>>,
}

/// Permissions configuration from _meta.ui.permissions
#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionsConfig {
    camera: Option<bool>,
    microphone: Option<bool>,
    geolocation: Option<bool>,
    clipboard_write: Option<bool>,
}

/// In-memory store for pending MCP App content
#[derive(Clone, Default)]
pub struct McpAppViewStore {
    pending: Arc<RwLock<HashMap<String, PendingApp>>>,
}

impl McpAppViewStore {
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

    /// Retrieve and remove HTML content by token (single-use)
    async fn take(&self, token: &str) -> Option<PendingApp> {
        let mut pending = self.pending.write().await;

        // Clean up expired tokens
        pending.retain(|_, app| app.created_at.elapsed() < TOKEN_TTL);

        pending.remove(token)
    }
}

/// Shared state for the MCP App View routes
#[derive(Clone)]
struct AppState {
    secret_key: String,
    store: McpAppViewStore,
}

/// Request body for POST /mcp-app-view
#[derive(Deserialize)]
struct CreateViewRequest {
    secret: String,
    html: String,
    #[serde(default)]
    csp: CspConfig,
    #[serde(default)]
    permissions: PermissionsConfig,
}

/// Response body for POST /mcp-app-view
#[derive(Serialize)]
struct CreateViewResponse {
    token: String,
    url: String,
}

/// POST /mcp-app-view - Store HTML and get a token
async fn create_view(
    State(state): State<AppState>,
    result: Result<Json<CreateViewRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let req = match result {
        Ok(Json(req)) => req,
        Err(e) => {
            tracing::error!("MCP App View JSON parse error: {}", e);
            return (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)).into_response();
        }
    };

    if req.secret != state.secret_key {
        tracing::warn!(
            "MCP App View auth failed: received secret length={}, expected length={}",
            req.secret.len(),
            state.secret_key.len()
        );
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let token = state.store.store(req.html, req.csp, req.permissions).await;

    Json(CreateViewResponse {
        url: format!("/mcp-app-view/{}", token),
        token,
    })
    .into_response()
}

/// Build CSP header based on declared domains
fn build_csp(csp: &CspConfig, permissions: &PermissionsConfig) -> String {
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
        .map(|d| format!("frame-src {}", d.join(" ")))
        .unwrap_or_else(|| "frame-src 'none'".to_string());

    let base_uris = csp
        .base_uri_domains
        .as_ref()
        .map(|d| format!(" {}", d.join(" ")))
        .unwrap_or_default();

    // Build Permission-Policy header value (not CSP, but we'll return it separately)
    let _ = permissions; // Used for Permission-Policy header, not CSP

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

/// GET /mcp-app-view/{token} - Serve the HTML content
async fn get_view(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    let Some(app) = state.store.take(&token).await else {
        return (StatusCode::NOT_FOUND, "Token not found or expired").into_response();
    };

    // Check if token has expired
    if app.created_at.elapsed() >= TOKEN_TTL {
        return (StatusCode::NOT_FOUND, "Token expired").into_response();
    }

    let csp = build_csp(&app.csp, &app.permissions);
    let permission_policy = build_permission_policy(&app.permissions);

    // Build response with headers
    let mut response = Html(app.html).into_response();
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

pub fn routes(secret_key: String, store: McpAppViewStore) -> Router {
    let state = AppState { secret_key, store };

    Router::new()
        .route("/mcp-app-view", post(create_view))
        .route("/mcp-app-view/{token}", get(get_view))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = McpAppViewStore::new();

        let token = store
            .store(
                "<html>test</html>".to_string(),
                CspConfig::default(),
                PermissionsConfig::default(),
            )
            .await;

        let app = store.take(&token).await;
        assert!(app.is_some());
        assert_eq!(app.unwrap().html, "<html>test</html>");

        // Token should be consumed (single-use)
        let app2 = store.take(&token).await;
        assert!(app2.is_none());
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let store = McpAppViewStore::new();
        let app = store.take("invalid-token").await;
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

        let result = build_csp(&csp, &PermissionsConfig::default());
        assert!(result.contains("connect-src 'self' https://api.example.com"));
        assert!(result.contains("https://cdn.example.com"));
        assert!(result.contains("frame-src 'none'"));
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
