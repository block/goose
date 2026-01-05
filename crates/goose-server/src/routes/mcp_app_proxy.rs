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
}

const MCP_APP_PROXY_HTML: &str = include_str!("templates/mcp_app_proxy.html");

/// Build the outer sandbox CSP based on declared domains.
///
/// This CSP acts as a ceiling - the inner guest UI iframe cannot exceed these
/// permissions, even if it tried. This is the single source of truth for
/// security policy enforcement.
///
/// Based on the MCP Apps specification (ext-apps SEP):
/// <https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx>
fn build_outer_csp(connect_domains: &[String], resource_domains: &[String]) -> String {
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
         frame-src blob: data:; \
         object-src 'none'; \
         base-uri 'self'"
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
        ("resource_domains" = Option<String>, Query, description = "Comma-separated domains for resource loading")
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

    // Build the outer CSP based on declared domains
    let csp = build_outer_csp(&connect_domains, &resource_domains);

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

pub fn routes(secret_key: String) -> Router {
    Router::new()
        .route("/mcp-app-proxy", get(mcp_app_proxy))
        .with_state(secret_key)
}
