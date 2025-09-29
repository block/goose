use std::sync::Arc;

use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::agents::{extension::Envs, ExtensionConfig};
use http::StatusCode;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use tracing;

/// Enum representing the different types of extension configuration requests.
#[derive(Deserialize)]
#[serde(tag = "type")]
enum ExtensionConfigRequest {
    /// Server-Sent Events (SSE) extension.
    #[serde(rename = "sse")]
    Sse {
        /// The name to identify this extension
        name: String,
        /// The URI endpoint for the SSE extension.
        uri: String,
        #[serde(default)]
        /// Map of environment variable key to values.
        envs: Envs,
        /// List of environment variable keys. The server will fetch their values from the keyring.
        #[serde(default)]
        env_keys: Vec<String>,
        timeout: Option<u64>,
    },
    /// Standard I/O (stdio) extension.
    #[serde(rename = "stdio")]
    Stdio {
        /// The name to identify this extension
        name: String,
        /// The command to execute.
        cmd: String,
        /// Arguments for the command.
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        /// Map of environment variable key to values.
        envs: Envs,
        /// List of environment variable keys. The server will fetch their values from the keyring.
        #[serde(default)]
        env_keys: Vec<String>,
        timeout: Option<u64>,
    },
    /// Built-in extension that is part of the goose binary.
    #[serde(rename = "builtin")]
    Builtin {
        /// The name of the built-in extension.
        name: String,
        display_name: Option<String>,
        timeout: Option<u64>,
    },
    /// Streamable HTTP extension using MCP Streamable HTTP specification.
    #[serde(rename = "streamable_http")]
    StreamableHttp {
        /// The name to identify this extension
        name: String,
        /// The URI endpoint for the streamable HTTP extension.
        uri: String,
        #[serde(default)]
        /// Map of environment variable key to values.
        envs: Envs,
        /// List of environment variable keys. The server will fetch their values from the keyring.
        #[serde(default)]
        env_keys: Vec<String>,
        /// Custom headers to include in requests.
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
        timeout: Option<u64>,
    },
    /// Frontend extension that provides tools to be executed by the frontend.
    #[serde(rename = "frontend")]
    Frontend {
        /// The name to identify this extension
        name: String,
        /// The tools provided by this extension
        tools: Vec<Tool>,
        /// Optional instructions for using the tools
        instructions: Option<String>,
    },
}

/// Response structure for adding an extension.
///
/// - `error`: Indicates whether an error occurred (`true`) or not (`false`).
/// - `message`: Provides detailed error information when `error` is `true`.
#[derive(Serialize)]
struct ExtensionResponse {
    error: bool,
    message: Option<String>,
}

/// Request structure for adding an extension, combining session_id with the extension config
#[derive(Deserialize)]
struct AddExtensionRequest {
    session_id: String,
    #[serde(flatten)]
    config: ExtensionConfigRequest,
}

/// Handler for adding a new extension configuration.
async fn add_extension(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AddExtensionRequest>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    // Log the request for debugging
    tracing::info!(
        "Received extension request for session: {}",
        request.session_id
    );

    let session_id = request.session_id.clone();
    let extension_request = request.config;

    // If this is a Stdio extension that uses npx, check for Node.js installation
    #[cfg(target_os = "windows")]
    if let ExtensionConfigRequest::Stdio { cmd, .. } = &extension_request {
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

    // Construct ExtensionConfig with Envs populated from keyring based on provided env_keys.
    let extension_config: ExtensionConfig = match extension_request {
        ExtensionConfigRequest::Sse {
            name,
            uri,
            envs,
            env_keys,
            timeout,
        } => ExtensionConfig::Sse {
            name,
            uri,
            envs,
            env_keys,
            description: None,
            timeout,
            bundled: None,
            available_tools: Vec::new(),
        },
        ExtensionConfigRequest::StreamableHttp {
            name,
            uri,
            envs,
            env_keys,
            headers,
            timeout,
        } => ExtensionConfig::StreamableHttp {
            name,
            uri,
            envs,
            env_keys,
            headers,
            description: None,
            timeout,
            bundled: None,
            available_tools: Vec::new(),
        },
        ExtensionConfigRequest::Stdio {
            name,
            cmd,
            args,
            envs,
            env_keys,
            timeout,
        } => {
            // TODO: We can uncomment once bugs are fixed. Check allowlist for Stdio extensions
            // if !is_command_allowed(&cmd, &args) {
            //     return Ok(Json(ExtensionResponse {
            //         error: true,
            //         message: Some(format!(
            //             "Extension '{}' is not in the allowed extensions list. Command: '{} {}'. If you require access please ask your administrator to update the allowlist.",
            //             args.join(" "),
            //             cmd, args.join(" ")
            //         )),
            //     }));
            // }

            ExtensionConfig::Stdio {
                name,
                cmd,
                args,
                description: None,
                envs,
                env_keys,
                timeout,
                bundled: None,
                available_tools: Vec::new(),
            }
        }
        ExtensionConfigRequest::Builtin {
            name,
            display_name,
            timeout,
        } => ExtensionConfig::Builtin {
            name,
            display_name,
            timeout,
            bundled: None,
            description: None,
            available_tools: Vec::new(),
        },
        ExtensionConfigRequest::Frontend {
            name,
            tools,
            instructions,
        } => ExtensionConfig::Frontend {
            name,
            tools,
            instructions,
            bundled: None,
            available_tools: Vec::new(),
        },
    };

    let agent = state.get_agent_for_route(session_id).await?;
    let response = agent.add_extension(extension_config).await;

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
