pub mod auth;

use std::sync::Arc;

use agent_client_protocol_http::{AcpHttpServer, CorsOptions, ServerOptions};
use axum::{
    http::{header, HeaderName, Method},
    routing::get,
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::acp::server::GooseAgentConnection;
use crate::acp::server_factory::AcpServer;

fn acp_http_options() -> ServerOptions {
    ServerOptions {
        path: "/acp".to_string(),
        cors: CorsOptions::allow_any_origin(),
        health_endpoint: false,
    }
}

/// CORS for the auxiliary routes (`/health`, `/status`, MCP app proxy) served by
/// `goose serve`. The ACP routes get their CORS from `AcpHttpServer`; this also
/// allows the `x-secret-key` auth header the proxy routes rely on.
fn aux_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-secret-key"),
        ])
}

/// The bare ACP HTTP/WebSocket router (POST/GET/DELETE on `/acp`), without auth
/// or goose-specific auxiliary routes.
pub fn create_acp_router(server: Arc<AcpServer>) -> Router {
    AcpHttpServer::new(move || GooseAgentConnection::new(server.clone()))
        .with_options(acp_http_options())
        .into_router()
}

async fn health() -> &'static str {
    "ok"
}

/// The full standalone ACP server router used by `goose serve`: ACP transport,
/// optional token auth, health/status endpoints, and the MCP app proxy.
pub fn create_router(server: Arc<AcpServer>, secret_key: String, require_token: bool) -> Router {
    let mut acp_routes = create_acp_router(server);
    if require_token {
        acp_routes = acp_routes.layer(axum::middleware::from_fn_with_state(
            secret_key.clone(),
            auth::check_acp_token,
        ));
    }

    let aux_routes = Router::new()
        .route("/health", get(health))
        .route("/status", get(health))
        .merge(super::mcp_app_proxy::routes(secret_key))
        .layer(aux_cors_layer());

    acp_routes.merge(aux_routes)
}
