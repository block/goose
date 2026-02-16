//! Extension management routes for live MCP connection lifecycle.
//!
//! These routes manage the runtime `ExtensionRegistry` — the shared singleton
//! that holds active MCP connections. Unlike `/config/extensions` (which manages
//! persisted extension configs), these routes manage **live** connections.
//!
//! Routes:
//!   GET    /extensions/live          — list active MCP connections
//!   GET    /extensions/live/:name    — get details for one extension
//!   POST   /extensions/live/:name/connect    — connect (start) an extension
//!   DELETE /extensions/live/:name/disconnect  — disconnect (stop) an extension

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use serde::Serialize;

use crate::state::AppState;

/// Summary of a live MCP extension connection
#[derive(Debug, Serialize)]
pub struct LiveExtensionInfo {
    pub name: String,
    pub connected: bool,
}

/// Response for listing live extensions
#[derive(Debug, Serialize)]
pub struct LiveExtensionsResponse {
    pub extensions: Vec<LiveExtensionInfo>,
    pub count: usize,
}

/// GET /extensions/live — list all active MCP connections
async fn list_live_extensions(State(state): State<Arc<AppState>>) -> Json<LiveExtensionsResponse> {
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

/// GET /extensions/live/:name — check if a specific extension is connected
async fn get_live_extension(
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

/// DELETE /extensions/live/:name/disconnect — disconnect an extension
async fn disconnect_extension(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let disconnected = state.extension_registry.disconnect(&name).await;
    if disconnected {
        // Invalidate tool caches since an extension was removed
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
