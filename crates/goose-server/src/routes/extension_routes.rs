//! Extension management routes for live MCP connection lifecycle.
//!
//! These routes manage the runtime `ExtensionRegistry` — the shared singleton
//! that holds active MCP connections. Unlike `/config/extensions` (which manages
//! persisted extension configs), these routes manage **live** connections.
//!
//! Routes:
//!   GET    /extensions/live          — list active MCP connections
//!   GET    /extensions/live/:name    — get details for one extension
//!   DELETE /extensions/live/:name/disconnect  — disconnect (stop) an extension

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

/// Summary of a live MCP extension connection
#[derive(Debug, Serialize, ToSchema)]
pub struct LiveExtensionInfo {
    /// Extension name (e.g. "developer", "memory", "fetch")
    pub name: String,
    /// Whether the MCP connection is active
    pub connected: bool,
}

/// Response for listing live extensions
#[derive(Debug, Serialize, ToSchema)]
pub struct LiveExtensionsResponse {
    /// List of active extensions
    pub extensions: Vec<LiveExtensionInfo>,
    /// Total count
    pub count: usize,
}

/// List all active MCP connections
#[utoipa::path(
    get,
    path = "/extensions/live",
    responses(
        (status = 200, description = "List of active MCP extension connections", body = LiveExtensionsResponse)
    ),
    tag = "extensions"
)]
pub async fn list_live_extensions(
    State(state): State<Arc<AppState>>,
) -> Json<LiveExtensionsResponse> {
    let names = state.extension_registry.list_names().await;
    let count = names.len();
    let extensions = names
        .into_iter()
        .map(|name| LiveExtensionInfo {
            name,
            connected: true,
        })
        .collect();

    Json(LiveExtensionsResponse { extensions, count })
}

/// Check if a specific extension is connected
#[utoipa::path(
    get,
    path = "/extensions/live/{name}",
    params(
        ("name" = String, Path, description = "Extension name to check")
    ),
    responses(
        (status = 200, description = "Extension connection status", body = LiveExtensionInfo),
        (status = 404, description = "Extension not found")
    ),
    tag = "extensions"
)]
pub async fn get_live_extension(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<LiveExtensionInfo>, StatusCode> {
    let exists = state.extension_registry.contains(&name).await;
    if exists {
        Ok(Json(LiveExtensionInfo {
            name,
            connected: true,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Disconnect (stop) an active MCP extension
#[utoipa::path(
    delete,
    path = "/extensions/live/{name}/disconnect",
    params(
        ("name" = String, Path, description = "Extension name to disconnect")
    ),
    responses(
        (status = 200, description = "Extension disconnected successfully"),
        (status = 404, description = "Extension not found or already disconnected")
    ),
    tag = "extensions"
)]
pub async fn disconnect_extension(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let disconnected = state.extension_registry.disconnect(&name).await;
    if disconnected {
        Ok(Json(serde_json::json!({
            "name": name,
            "status": "disconnected"
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/extensions/live", get(list_live_extensions))
        .route("/extensions/live/{name}", get(get_live_extension))
        .route(
            "/extensions/live/{name}/disconnect",
            delete(disconnect_extension),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_extension_info_serialization() {
        let info = LiveExtensionInfo {
            name: "developer".to_string(),
            connected: true,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["name"], "developer");
        assert_eq!(json["connected"], true);
    }

    #[test]
    fn test_live_extensions_response_serialization() {
        let response = LiveExtensionsResponse {
            extensions: vec![
                LiveExtensionInfo {
                    name: "developer".to_string(),
                    connected: true,
                },
                LiveExtensionInfo {
                    name: "memory".to_string(),
                    connected: true,
                },
            ],
            count: 2,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["count"], 2);
        assert_eq!(json["extensions"].as_array().unwrap().len(), 2);
    }
}
