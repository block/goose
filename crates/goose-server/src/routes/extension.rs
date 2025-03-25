use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::OnceLock;

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
        display_name: Option<String>,
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
                description: None,
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
            // Check allowlist for Stdio extensions
            if !is_command_allowed(&cmd) {
                return Ok(Json(ExtensionResponse {
                    error: true,
                    message: Some(format!(
                        "Command '{}' is not in the allowed extensions list",
                        cmd
                    )),
                }));
            }

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
                description: None,
                envs: Envs::new(env_map),
                timeout,
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
        },
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

/// Structure representing the allowed extensions from the YAML file
#[derive(Deserialize, Debug, Clone)]
struct AllowedExtensions {
    extensions: Vec<ExtensionAllowlistEntry>,
}

/// Structure representing an individual extension entry in the allowlist
#[derive(Deserialize, Debug, Clone)]
struct ExtensionAllowlistEntry {
    #[allow(dead_code)]
    id: String,
    command: String,
}

// Global cache for the allowed extensions
static ALLOWED_EXTENSIONS: OnceLock<Option<AllowedExtensions>> = OnceLock::new();

/// Fetches and parses the allowed extensions from the URL specified in GOOSE_ALLOWLIST env var
fn fetch_allowed_extensions() -> Option<AllowedExtensions> {
    match env::var("GOOSE_ALLOWLIST") {
        Err(_) => {
            // Environment variable not set, no allowlist to enforce
            None
        }
        Ok(url) => match reqwest::blocking::get(&url) {
            Err(e) => {
                eprintln!("Failed to fetch allowlist: {}", e);
                None
            }
            Ok(response) if !response.status().is_success() => {
                eprintln!("Failed to fetch allowlist, status: {}", response.status());
                None
            }
            Ok(response) => match response.text() {
                Err(e) => {
                    eprintln!("Failed to read allowlist response: {}", e);
                    None
                }
                Ok(text) => match serde_yaml::from_str::<AllowedExtensions>(&text) {
                    Ok(allowed) => Some(allowed),
                    Err(e) => {
                        eprintln!("Failed to parse allowlist YAML: {}", e);
                        None
                    }
                },
            },
        },
    }
}

/// Gets the cached allowed extensions or fetches them if not yet cached
fn get_allowed_extensions() -> &'static Option<AllowedExtensions> {
    ALLOWED_EXTENSIONS.get_or_init(fetch_allowed_extensions)
}

/// Checks if a command is allowed based on the allowlist
fn is_command_allowed(cmd: &str) -> bool {
    is_command_allowed_with_allowlist(cmd, get_allowed_extensions())
}

/// Implementation of command allowlist checking that takes an explicit allowlist parameter
/// This makes it easier to test without relying on global state
fn is_command_allowed_with_allowlist(
    cmd: &str,
    allowed_extensions: &Option<AllowedExtensions>,
) -> bool {
    match allowed_extensions {
        // No allowlist configured, allow all commands
        None => true,

        // Empty allowlist, allow all commands
        Some(extensions) if extensions.extensions.is_empty() => true,

        // Check against the allowlist
        Some(extensions) => {
            // Extract the base command name (last part of the path)
            let cmd_base = Path::new(cmd)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(cmd);

            // Check if the command is in the allowlist
            extensions
                .extensions
                .iter()
                .any(|entry| cmd_base.contains(&entry.command))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Create a test allowlist with the given commands
    fn create_test_allowlist(commands: &[&str]) -> Option<AllowedExtensions> {
        if commands.is_empty() {
            return Some(AllowedExtensions { extensions: vec![] });
        }

        let entries = commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| ExtensionAllowlistEntry {
                id: format!("test-{}", i),
                command: cmd.to_string(),
            })
            .collect();

        Some(AllowedExtensions {
            extensions: entries,
        })
    }

    #[test]
    fn test_command_allowed_when_matching() {
        let allowlist = create_test_allowlist(&["uvx mcp_slack", "uvx mcp_github"]);

        // Test with full paths
        assert!(is_command_allowed_with_allowlist(
            "/Users/username/path/to/uvx mcp_slack",
            &allowlist
        ));
        assert!(is_command_allowed_with_allowlist(
            "/opt/local/bin/uvx mcp_github",
            &allowlist
        ));

        // Test with just the command
        assert!(is_command_allowed_with_allowlist(
            "uvx mcp_slack",
            &allowlist
        ));
        assert!(is_command_allowed_with_allowlist(
            "uvx mcp_github",
            &allowlist
        ));
    }

    #[test]
    fn test_command_not_allowed_when_not_matching() {
        let allowlist = create_test_allowlist(&["uvx mcp_slack", "uvx mcp_github"]);

        // These should not be allowed
        assert!(!is_command_allowed_with_allowlist(
            "/Users/username/path/to/uvx mcp_malicious",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist(
            "uvx mcp_unauthorized",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist("/bin/bash", &allowlist));
    }

    #[test]
    fn test_all_commands_allowed_when_no_allowlist() {
        // Empty allowlist should allow all commands
        let empty_allowlist = create_test_allowlist(&[]);
        assert!(is_command_allowed_with_allowlist(
            "any_command_should_be_allowed",
            &empty_allowlist
        ));

        // No allowlist should allow all commands
        assert!(is_command_allowed_with_allowlist(
            "any_command_should_be_allowed",
            &None
        ));
    }

    #[test]
    fn test_fetch_allowed_extensions_from_url() {
        // Start a mock server - we need to use a blocking approach since fetch_allowed_extensions is blocking
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        let server_url = format!("http://127.0.0.1:{}", port);
        let server_path = "/allowed_extensions.yaml";

        // Define the mock response
        let yaml_content = r#"extensions:
  - id: slack
    command: uvx mcp_slack
  - id: github
    command: uvx mcp_github
"#;

        // Spawn a thread to handle the request
        let handle = std::thread::spawn(move || {
            let (stream, _) = server.accept().unwrap();
            let mut buf_reader = std::io::BufReader::new(&stream);
            let mut request_line = String::new();
            std::io::BufRead::read_line(&mut buf_reader, &mut request_line).unwrap();

            // Very simple HTTP response
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/yaml\r\n\r\n{}",
                yaml_content.len(),
                yaml_content
            );

            let mut writer = std::io::BufWriter::new(&stream);
            std::io::Write::write_all(&mut writer, response.as_bytes()).unwrap();
            std::io::Write::flush(&mut writer).unwrap();
        });

        // Set the environment variable to point to our mock server
        env::set_var("GOOSE_ALLOWLIST", format!("{}{}", server_url, server_path));

        // Give the server a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Call the function that fetches from the URL
        let allowed_extensions = fetch_allowed_extensions();

        // Verify the result
        assert!(allowed_extensions.is_some());
        let extensions = allowed_extensions.unwrap();
        assert_eq!(extensions.extensions.len(), 2);
        assert_eq!(extensions.extensions[0].id, "slack");
        assert_eq!(extensions.extensions[0].command, "uvx mcp_slack");
        assert_eq!(extensions.extensions[1].id, "github");
        assert_eq!(extensions.extensions[1].command, "uvx mcp_github");

        // Clean up
        env::remove_var("GOOSE_ALLOWLIST");

        // Wait for the server thread to complete
        handle.join().unwrap();
    }
}
