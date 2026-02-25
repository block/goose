use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ProxyQuery {
    secret: String,
    /// Comma-separated list of domains for connect-src (fetch, XHR, WebSocket)
    connect_domains: Option<String>,
    /// Comma-separated list of domains for resource loading (scripts, styles, images, fonts, media)
    resource_domains: Option<String>,
    /// Comma-separated list of origins for nested iframes (frame-src)
    frame_domains: Option<String>,
    /// Comma-separated list of allowed base URIs (base-uri)
    base_uri_domains: Option<String>,
}

const MCP_APP_PROXY_HTML: &str = include_str!("templates/mcp_app_proxy.html");
const MCP_APP_SDK_JS: &str = include_str!("templates/mcp_app_sdk.js");

/// Build the outer sandbox CSP based on declared domains.
///
/// This CSP acts as a ceiling - the inner guest UI iframe cannot exceed these
/// permissions, even if it tried. This is the single source of truth for
/// security policy enforcement.
///
/// Based on the MCP Apps specification (ext-apps SEP):
/// <https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx>
fn build_outer_csp(
    connect_domains: &[String],
    resource_domains: &[String],
    frame_domains: &[String],
    base_uri_domains: &[String],
) -> String {
    let resources = if resource_domains.is_empty() {
        String::new()
    } else {
        format!(" {}", resource_domains.join(" "))
    };

    let connections = if connect_domains.is_empty() {
        String::new()
    } else {
        format!(" {}", connect_domains.join(" "))
    };

    let frame_src = if frame_domains.is_empty() {
        "frame-src 'none'".to_string()
    } else {
        format!("frame-src {}", frame_domains.join(" "))
    };

    let base_uris = if base_uri_domains.is_empty() {
        String::new()
    } else {
        format!(" {}", base_uri_domains.join(" "))
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

/// Parse comma-separated domains, filtering out empty strings
fn parse_domains(domains: Option<&String>) -> Vec<String> {
    domains
        .map(|d| {
            d.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[utoipa::path(
    get,
    path = "/mcp-app-proxy",
    params(
        ("secret" = String, Query, description = "Secret key for authentication"),
        ("connect_domains" = Option<String>, Query, description = "Comma-separated domains for connect-src"),
        ("resource_domains" = Option<String>, Query, description = "Comma-separated domains for resource loading"),
        ("frame_domains" = Option<String>, Query, description = "Comma-separated origins for nested iframes (frame-src)"),
        ("base_uri_domains" = Option<String>, Query, description = "Comma-separated allowed base URIs (base-uri)")
    ),
    responses(
        (status = 200, description = "MCP App proxy HTML page", content_type = "text/html"),
        (status = 401, description = "Unauthorized - invalid or missing secret"),
    )
)]
async fn mcp_app_proxy(
    axum::extract::State(secret_key): axum::extract::State<String>,
    Query(params): Query<ProxyQuery>,
) -> Response {
    if params.secret != secret_key {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    // Parse domains from query params
    let connect_domains = parse_domains(params.connect_domains.as_ref());
    let resource_domains = parse_domains(params.resource_domains.as_ref());
    let frame_domains = parse_domains(params.frame_domains.as_ref());
    let base_uri_domains = parse_domains(params.base_uri_domains.as_ref());

    // Build the outer CSP based on declared domains
    let csp = build_outer_csp(
        &connect_domains,
        &resource_domains,
        &frame_domains,
        &base_uri_domains,
    );

    // Replace the CSP placeholder in the HTML template
    let html = MCP_APP_PROXY_HTML.replace("{{OUTER_CSP}}", &csp);

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

/// Serve the MCP Apps SDK (`@modelcontextprotocol/ext-apps`) as a JavaScript module.
///
/// Guest apps inside the sandbox can load this via:
///   `<script type="module">import { App } from '/mcp-app-sdk.js';</script>`
///
/// This works because the sandbox iframe has `allow-same-origin` and the CSP
/// includes `script-src 'self'`, allowing script loads from the goosed origin.
async fn mcp_app_sdk() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        MCP_APP_SDK_JS,
    )
        .into_response()
}

pub fn routes(secret_key: String) -> Router {
    Router::new()
        .route("/mcp-app-proxy", get(mcp_app_proxy))
        .route("/mcp-app-sdk.js", get(mcp_app_sdk))
        .with_state(secret_key)
}
