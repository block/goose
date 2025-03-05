use std::collections::HashMap;

use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::{
    agents::{extension::Envs, ExtensionConfig},
    config::Config,
};
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

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
        timeout: Option<u64>,
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

/// Handler for adding a new extension configuration.
async fn add_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ExtensionConfigRequest>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    // Verify the presence and validity of the secret key.
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Load the configuration
    let config = Config::global();

    // Initialize a vector to collect any missing keys.
    let mut missing_keys = Vec::new();

    // Construct ExtensionConfig with Envs populated from keyring based on provided env_keys.
    let extension_config: ExtensionConfig = match request {
        ExtensionConfigRequest::Sse {
            name,
            uri,
            env_keys,
            timeout,
        } => {
            let mut env_map = HashMap::new();
            for key in env_keys {
                match config.get_secret(&key) {
                    Ok(value) => {
                        env_map.insert(key, value);
                    }
                    Err(_) => {
                        missing_keys.push(key);
                    }
                }
            }

            if !missing_keys.is_empty() {
                return Ok(Json(ExtensionResponse {
                    error: true,
                    message: Some(format!(
                        "Missing secrets for keys: {}",
                        missing_keys.join(", ")
                    )),
                }));
            }

            ExtensionConfig::Sse {
                name,
                uri,
                envs: Envs::new(env_map),
                timeout,
            }
        }
        ExtensionConfigRequest::Stdio {
            name,
            cmd,
            args,
            env_keys,
            timeout,
        } => {
            let mut env_map = HashMap::new();
            for key in env_keys {
                match config.get_secret(&key) {
                    Ok(value) => {
                        env_map.insert(key, value);
                    }
                    Err(_) => {
                        missing_keys.push(key);
                    }
                }
            }

            if !missing_keys.is_empty() {
                return Ok(Json(ExtensionResponse {
                    error: true,
                    message: Some(format!(
                        "Missing secrets for keys: {}",
                        missing_keys.join(", ")
                    )),
                }));
            }

            ExtensionConfig::Stdio {
                name,
                cmd,
                args,
                envs: Envs::new(env_map),
                timeout,
            }
        }
        ExtensionConfigRequest::Builtin { name, timeout } => {
            ExtensionConfig::Builtin { name, timeout }
        }
    };

    // Acquire a lock on the agent and attempt to add the extension.
    let mut agent = state.agent.write().await;
    let agent = agent.as_mut().ok_or(StatusCode::PRECONDITION_REQUIRED)?;
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

/// Handler for removing an extension by name
async fn remove_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(name): Json<String>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    // Verify the presence and validity of the secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Acquire a lock on the agent and attempt to remove the extension
    let mut agent = state.agent.write().await;
    let agent = agent.as_mut().ok_or(StatusCode::PRECONDITION_REQUIRED)?;
    agent.remove_extension(&name).await;

    Ok(Json(ExtensionResponse {
        error: false,
        message: None,
    }))
}

/// Registers the extension management routes with the Axum router.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/extensions/add", post(add_extension))
        .route("/extensions/remove", post(remove_extension))
        .with_state(state)
}
