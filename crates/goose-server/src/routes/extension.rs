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
            if !is_command_allowed(&cmd, &args) {
                return Ok(Json(ExtensionResponse {
                    error: true,
                    message: Some(format!(
                        "Extension '{}' is not in the allowed extensions list. Command: '{} {}'. If you require access please ask your administrator to update the allowlist.",                        
                        args.join(" "),
                        cmd, args.join(" ")
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
fn is_command_allowed(cmd: &str, args: &[String]) -> bool {
    is_command_allowed_with_allowlist(&make_full_cmd(cmd, args), get_allowed_extensions())
}

fn make_full_cmd(cmd: &str, args: &[String]) -> String {
    // trim each arg string to remove any leading/trailing whitespace
    let args_trimmed = args.iter().map(|arg| arg.trim()).collect::<Vec<&str>>();

    format!("{} {}", cmd.trim(), args_trimmed.join(" ").trim())
}

/// Normalizes a command name by removing common executable extensions (.exe, .cmd, .bat)
/// This makes the allowlist more portable across different operating systems
fn normalize_command_name(cmd: &str) -> String {
    cmd.replace(".exe", "")
        .replace(".cmd", "")
        .replace(".bat", "")
        .to_string()
}

/// Implementation of command allowlist checking that takes an explicit allowlist parameter
/// This makes it easier to test without relying on global state
fn is_command_allowed_with_allowlist(
    cmd: &str,
    allowed_extensions: &Option<AllowedExtensions>,
) -> bool {
    println!("\n\n\n\n\n\n------------\n\nChecking command: {}", cmd);
    // Extract the first part of the command (before any spaces)
    let first_part = cmd.split_whitespace().next().unwrap_or(cmd);
    println!("First part: {}", first_part);

    // Extract the base command name (last part of the path)
    let cmd_base_with_ext = Path::new(first_part)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(first_part);

    // Normalize the command name by removing extensions like .exe or .cmd
    let cmd_base = normalize_command_name(cmd_base_with_ext);

    // Special case: Always allow commands ending with "/goosed" or equal to "goosed"
    // But still enforce that it's in the same directory as the current executable
    if cmd_base == "goosed" {
        // Only allow exact matches (no arguments)
        if cmd == first_part {
            // For absolute paths, check that it's in the same directory as the current executable
            if (first_part.contains('/') || first_part.contains('\\'))
                && !first_part.starts_with("./")
            {
                let current_exe = std::env::current_exe().unwrap();
                let current_exe_dir = current_exe.parent().unwrap();
                let expected_path = current_exe_dir.join("goosed").to_str().unwrap().to_string();

                // Normalize both paths before comparing
                let normalized_cmd_path = normalize_command_name(first_part);
                let normalized_expected_path = normalize_command_name(&expected_path);

                if normalized_cmd_path == normalized_expected_path {
                    return true;
                }
                // If the path doesn't match, don't allow it
                println!("Goosed not in expected directory: {}", cmd);
                println!("Expected path: {}", expected_path);
                return false;
            } else {
                // For non-path goosed or relative paths, allow it
                return true;
            }
        }
        return false;
    }

    println!("Allowed extensions: {:?}", allowed_extensions);

    match allowed_extensions {
        // No allowlist configured, allow all commands
        None => true,

        // Empty allowlist, allow all commands
        Some(extensions) if extensions.extensions.is_empty() => true,

        // Check against the allowlist
        Some(extensions) => {
            // Check that the command exists as a peer command to current executable directory
            // Only apply this check if the command includes a path separator
            let current_exe = std::env::current_exe().unwrap();
            let current_exe_dir = current_exe.parent().unwrap();
            let expected_path = current_exe_dir
                .join(&cmd_base)
                .to_str()
                .unwrap()
                .to_string();

            // Normalize both paths before comparing
            let normalized_cmd_path = normalize_command_name(first_part);

            if (first_part.contains('/') || first_part.contains('\\'))
                && normalized_cmd_path != expected_path
            {
                println!("Command not in expected directory: {}", cmd);
                return false;
            }

            //let cmd_to_check= cmd.replace(current_exe_dir.to_str(), "").to_string();

            // remove current_exe_dir + "/" from the cmd to clean it up
            let path_to_trim = format!("{}/", current_exe_dir.to_str().unwrap());

            // now remove this to make it clean
            let cmd_to_check = cmd.replace(&path_to_trim, "");

            println!("Command to check: {}", cmd_to_check);

            // Normalize the command before comparing with allowlist entries
            let normalized_cmd = normalize_command_name(&cmd_to_check);

            println!("Normalized command: {}", normalized_cmd);

            extensions.extensions.iter().any(|entry| {
                let normalized_entry = normalize_command_name(&entry.command);
                normalized_cmd == normalized_entry
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_normalize_command_name() {
        // Test removing .exe extension
        assert_eq!(normalize_command_name("goosed.exe"), "goosed");
        assert_eq!(
            normalize_command_name("/path/to/goosed.exe"),
            "/path/to/goosed"
        );

        // Test removing .cmd extension
        assert_eq!(normalize_command_name("script.cmd"), "script");
        assert_eq!(
            normalize_command_name("/path/to/script.cmd"),
            "/path/to/script"
        );

        assert_eq!(normalize_command_name("batch.bat"), "batch");
        assert_eq!(
            normalize_command_name("/path/to/batch.bat thing"),
            "/path/to/batch thing"
        );

        // Test with no extension
        assert_eq!(normalize_command_name("goosed"), "goosed");
        assert_eq!(normalize_command_name("/path/to/goosed"), "/path/to/goosed");
    }

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
    fn test_make_full() {
        assert_eq!(
            make_full_cmd("uvx", &vec!["mcp_slack".to_string()]),
            "uvx mcp_slack"
        );
        assert_eq!(
            make_full_cmd("uvx", &vec!["mcp_slack ".to_string()]),
            "uvx mcp_slack"
        );
        assert_eq!(
            make_full_cmd(
                "uvx",
                &vec!["mcp_slack".to_string(), "--verbose".to_string()]
            ),
            "uvx mcp_slack --verbose"
        );
        assert_eq!(
            make_full_cmd(
                "uvx",
                &vec!["mcp_slack".to_string(), " --verbose".to_string()]
            ),
            "uvx mcp_slack --verbose"
        );
    }

    #[test]
    fn test_command_allowed_when_matching() {
        let allowlist = create_test_allowlist(&[
            "uvx something",
            "uvx mcp_slack",
            "npx mcp_github",
            "minecraft",
        ]);

        // Test with exact command matches
        assert!(is_command_allowed_with_allowlist(
            "uvx something",
            &allowlist
        ));

        // Test with exact command matches
        assert!(is_command_allowed_with_allowlist("minecraft", &allowlist));

        assert!(is_command_allowed_with_allowlist(
            "uvx mcp_slack",
            &allowlist
        ));
        assert!(is_command_allowed_with_allowlist(
            "npx mcp_github",
            &allowlist
        ));

        // Get the current executable directory for reference
        let current_exe = std::env::current_exe().unwrap();
        let current_exe_dir = current_exe.parent().unwrap();

        // Create a full path command that would be in the current executable directory
        // For testing purposes, we'll use a direct path to the command in the allowlist
        let full_path_cmd = current_exe_dir
            .join("uvx my_mcp")
            .to_str()
            .unwrap()
            .to_string();

        // Create a test allowlist with the command name (without path)
        let path_test_allowlist = create_test_allowlist(&["uvx my_mcp"]);

        // This should be allowed because the path is correct and the base command matches
        println!(
            "Current executable directory: {}",
            current_exe_dir.to_str().unwrap()
        );
        println!("Path test allowlist: {:?}", path_test_allowlist);
        assert!(is_command_allowed_with_allowlist(
            &full_path_cmd,
            &path_test_allowlist
        ));

        // Test with additional arguments - should NOT match because we require exact matches
        assert!(!is_command_allowed_with_allowlist(
            "uvx mcp_slack --verbose --flag=value",
            &allowlist
        ));

        // Test with a path that doesn't match the current directory - should fail
        assert!(!is_command_allowed_with_allowlist(
            "/Users/username/path/to/uvx mcp_slack",
            &allowlist
        ));

        // These should NOT match with exact matching
        assert!(!is_command_allowed_with_allowlist(
            "uvx other_command",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist(
            "prefix_npx mcp_github",
            &allowlist
        ));
    }

    #[test]
    fn test_command_not_allowed_when_not_matching() {
        let allowlist =
            create_test_allowlist(&["uvx something", "uvx mcp_slack", "npx mcp_github"]);

        // These should not be allowed
        assert!(!is_command_allowed_with_allowlist(
            "/Users/username/path/to/uvx_malicious",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist(
            "unauthorized_command",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist("/bin/bash", &allowlist));
        assert!(!is_command_allowed_with_allowlist(
            "uvx unauthorized",
            &allowlist
        ));
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
    fn test_goosed_special_case() {
        // Create a restrictive allowlist that doesn't include goosed
        let allowlist = create_test_allowlist(&["uvx mcp_slack"]);

        // Get the current executable directory for goosed path testing
        let current_exe = std::env::current_exe().unwrap();
        let current_exe_dir = current_exe.parent().unwrap();
        let goosed_path = current_exe_dir.join("goosed").to_str().unwrap().to_string();
        let goosed_exe_path = current_exe_dir
            .join("goosed.exe")
            .to_str()
            .unwrap()
            .to_string();

        // This should be allowed because it's goosed in the correct directory
        assert!(is_command_allowed_with_allowlist(&goosed_path, &allowlist));

        // This should also be allowed because it's goosed.exe in the correct directory
        assert!(is_command_allowed_with_allowlist(
            &goosed_exe_path,
            &allowlist
        ));

        // These should NOT be allowed because they're in the wrong directory
        assert!(!is_command_allowed_with_allowlist(
            "/usr/local/bin/goosed",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist(
            "/Users/username/path/to/goosed",
            &allowlist
        ));

        // Commands with arguments should NOT be allowed - we require exact matches
        assert!(!is_command_allowed_with_allowlist(
            "/Users/username/path/to/goosed --flag value",
            &allowlist
        ));

        // Simple goosed without path should be allowed
        assert!(is_command_allowed_with_allowlist("./goosed", &allowlist));
        assert!(is_command_allowed_with_allowlist("goosed", &allowlist));

        // These should NOT be allowed because they don't end with "/goosed"
        assert!(!is_command_allowed_with_allowlist(
            "/usr/local/bin/goosed-extra",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist(
            "/usr/local/bin/not-goosed",
            &allowlist
        ));
        assert!(!is_command_allowed_with_allowlist(
            "goosed-extra",
            &allowlist
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
