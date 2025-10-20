use std::sync::Arc;

use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::agents::ExtensionConfig;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing;

// TODO(Douwe): delete this file

#[derive(Serialize)]
struct ExtensionResponse {
    error: bool,
    message: Option<String>,
}

#[derive(Deserialize)]
struct AddExtensionRequest {
    session_id: String,
    #[serde(flatten)]
    config: ExtensionConfig,
}

async fn add_extension(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AddExtensionRequest>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    // Log the request for debugging
    tracing::info!(
        "Received extension request for session: {}",
        request.session_id
    );

    // If this is a Stdio extension that uses npx, check for Node.js installation
    #[cfg(target_os = "windows")]
    if let ExtensionConfig::Stdio { cmd, .. } = &request.config {
        if cmd.ends_with("npx.cmd") || cmd.ends_with("npx") {
            // Check if Node.js is installed in standard locations
            let node_exists = std::path::Path::new(r"C:\Program Files\nodejs\node.exe").exists()
                || std::path::Path::new(r"C:\Program Files (x86)\nodejs\node.exe").exists();

            if !node_exists {
                // Get the directory containing npx.cmd
                let cmd_path = std::path::Path::new(&cmd);
                let script_dir = cmd_path.parent().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

                // Run the Node.js installer script
                let install_script = script_dir.join("install-node.cmd");

                if install_script.exists() {
                    eprintln!("Installing Node.js...");
                    let output = std::process::Command::new(&install_script)
                        .arg("https://nodejs.org/dist/v23.10.0/node-v23.10.0-x64.msi")
                        .output()
                        .map_err(|e| {
                            eprintln!("Failed to run Node.js installer: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                    if !output.status.success() {
                        eprintln!(
                            "Failed to install Node.js: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        return Ok(Json(ExtensionResponse {
                            error: true,
                            message: Some(format!(
                                "Failed to install Node.js: {}",
                                String::from_utf8_lossy(&output.stderr)
                            )),
                        }));
                    }
                    eprintln!("Node.js installation completed");
                } else {
                    eprintln!(
                        "Node.js installer script not found at: {}",
                        install_script.display()
                    );
                    return Ok(Json(ExtensionResponse {
                        error: true,
                        message: Some("Node.js installer script not found".to_string()),
                    }));
                }
            }
        }
    }

    let agent = state.get_agent_for_route(request.session_id).await?;
    let response = agent.add_extension(request.config).await;

    // Respond with the result.
    match response {
        Ok(_) => Ok(Json(ExtensionResponse {
            error: false,
            message: None,
        })),
        Err(e) => {
            eprintln!("Failed to add extension configuration: {:?}", e);
            Ok(Json(ExtensionResponse {
                error: true,
                message: Some(format!(
                    "Failed to add extension configuration, error: {:?}",
                    e
                )),
            }))
        }
    }
}

#[derive(Deserialize)]
struct RemoveExtensionRequest {
    name: String,
    session_id: String,
}

/// Handler for removing an extension by name
async fn remove_extension(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RemoveExtensionRequest>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    let agent = state.get_agent_for_route(request.session_id).await?;

    match agent.remove_extension(&request.name).await {
        Ok(_) => Ok(Json(ExtensionResponse {
            error: false,
            message: None,
        })),
        Err(e) => Ok(Json(ExtensionResponse {
            error: true,
            message: Some(format!("Failed to remove extension: {:?}", e)),
        })),
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/extensions/add", post(add_extension))
        .route("/extensions/remove", post(remove_extension))
        .with_state(state)
}
