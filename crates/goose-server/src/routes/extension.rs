use std::env;
use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;

use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::agents::{extension::Envs, ExtensionConfig};
use http::{HeaderMap, StatusCode};
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

/// Handler for adding a new extension configuration.
async fn add_extension(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    raw: axum::extract::Json<serde_json::Value>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Log the raw request for debugging
    tracing::info!(
        "Received extension request: {}",
        serde_json::to_string_pretty(&raw.0).unwrap()
    );

    // Try to parse into our enum
    let request: ExtensionConfigRequest = match serde_json::from_value(raw.0.clone()) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to parse extension request: {}", e);
            tracing::error!(
                "Raw request was: {}",
                serde_json::to_string_pretty(&raw.0).unwrap()
            );
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    // If this is a Stdio extension that uses npx, check for Node.js installation
    #[cfg(target_os = "windows")]
    if let ExtensionConfigRequest::Stdio { cmd, .. } = &request {
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
    let extension_config: ExtensionConfig = match request {
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
        },
    };

    // Get a reference to the agent
    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;
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
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(name): Json<String>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Get a reference to the agent
    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;
    match agent.remove_extension(&name).await {
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

/// Registers the extension management routes with the Axum router.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/extensions/add", post(add_extension))
        .route("/extensions/remove", post(remove_extension))
        .with_state(state)
}

/// Structure representing the allowed extensions from the YAML file
#[derive(Deserialize, Debug, Clone)]
struct AllowedExtensions {
    #[allow(dead_code)]
    extensions: Vec<ExtensionAllowlistEntry>,
}

/// Structure representing an individual extension entry in the allowlist
#[derive(Deserialize, Debug, Clone)]
struct ExtensionAllowlistEntry {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    command: String,
}

// Global cache for the allowed extensions
#[allow(dead_code)]
static ALLOWED_EXTENSIONS: OnceLock<Option<AllowedExtensions>> = OnceLock::new();

/// Fetches and parses the allowed extensions from the URL specified in GOOSE_ALLOWLIST env var
#[allow(dead_code)]
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
#[allow(dead_code)]
fn get_allowed_extensions() -> &'static Option<AllowedExtensions> {
    ALLOWED_EXTENSIONS.get_or_init(fetch_allowed_extensions)
}

/// Checks if a command is allowed based on the allowlist
#[allow(dead_code)]
fn is_command_allowed(cmd: &str, args: &[String]) -> bool {
    // Check if bypass is enabled
    if let Ok(bypass_value) = env::var("GOOSE_ALLOWLIST_BYPASS") {
        if bypass_value.to_lowercase() == "true" {
            // Bypass the allowlist check
            println!("Allowlist check bypassed due to GOOSE_ALLOWLIST_BYPASS=true");
            return true;
        }
    }

    // Proceed with normal allowlist check
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
        .replace(" -y ", " ")
        .replace(" -y", "")
        .replace("-y ", "")
        .to_string()
}

/// Implementation of command allowlist checking that takes an explicit allowlist parameter
/// This makes it easier to test without relying on global state
fn is_command_allowed_with_allowlist(
    cmd: &str,
    allowed_extensions: &Option<AllowedExtensions>,
) -> bool {
    // Extract the first part of the command (before any spaces)
    let first_part = cmd.split_whitespace().next().unwrap_or(cmd);

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

    match allowed_extensions {
        // No allowlist configured, allow all commands
        None => true,

        // Empty allowlist, allow all commands
        Some(extensions) if extensions.extensions.is_empty() => true,

        // Check against the allowlist
        Some(extensions) => {
            // Strip out the Goose app resources/bin prefix if present (handle both macOS and Windows paths)
            let mut cmd_to_check = cmd.to_string();
            let mut is_goose_path = false;

            // Check for macOS-style Goose.app path
            if cmd_to_check.contains("Goose.app/Contents/Resources/bin/") {
                if let Some(idx) = cmd_to_check.find("Goose.app/Contents/Resources/bin/") {
                    cmd_to_check = cmd_to_check
                        [(idx + "Goose.app/Contents/Resources/bin/".len())..]
                        .to_string();
                    is_goose_path = true;
                }
            }
            // Check for Windows-style Goose path with resources\bin
            else if cmd_to_check.to_lowercase().contains("\\resources\\bin\\")
                || cmd_to_check.contains("/resources/bin/")
            {
                // Also handle forward slashes
                if let Some(idx) = cmd_to_check
                    .to_lowercase()
                    .rfind("\\resources\\bin\\")
                    .or_else(|| cmd_to_check.rfind("/resources/bin/"))
                {
                    let path_len = if cmd_to_check.contains("/resources/bin/") {
                        "/resources/bin/".len()
                    } else {
                        "\\resources\\bin\\".len()
                    };
                    cmd_to_check = cmd_to_check[(idx + path_len)..].to_string();
                    is_goose_path = true;
                }
            }

            // Only check current directory for non-Goose paths
            if !is_goose_path {
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
                    && !cmd_to_check.contains("Goose.app/Contents/Resources/bin/")
                {
                    println!("Command not in expected directory: {}", cmd);
                    return false;
                }

                // Remove current_exe_dir + "/" from the cmd to clean it up
                let path_to_trim = format!("{}/", current_exe_dir.to_str().unwrap());
                cmd_to_check = cmd_to_check.replace(&path_to_trim, "");
            }

            println!("Command to check after path trimming: {}", cmd_to_check);

            // Remove @version suffix from command parts, but preserve scoped npm packages
            let parts: Vec<&str> = cmd_to_check.split_whitespace().collect();
            let mut cleaned_parts: Vec<String> = Vec::new();

            for part in parts {
                if part.contains('@') && !part.starts_with('@') {
                    // This is likely a package with a version suffix, like "package@1.0.0"
                    // Keep only the part before the @ symbol
                    if let Some(base_part) = part.split('@').next() {
                        cleaned_parts.push(base_part.to_string());
                    } else {
                        cleaned_parts.push(part.to_string());
                    }
                } else {
                    // Either no @ symbol or it's a scoped package (starts with @)
                    cleaned_parts.push(part.to_string());
                }
            }

            // Reconstruct the command without version suffixes
            cmd_to_check = cleaned_parts.join(" ");

            println!("Command to check after @version removal: {}", cmd_to_check);

            // Normalize the command before comparing with allowlist entries
            let normalized_cmd = normalize_command_name(&cmd_to_check);

            println!("Final normalized command: {}", normalized_cmd);

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

        assert_eq!(normalize_command_name("npx -y thing"), "npx thing");
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
            "npx -y @mic/mcp_mic",
            "npx -y @mic/mcp_mic2@latest",
            "npx @mic/mcp_mic3",
            "npx @mic/mcp_mic4@latest",
            "executor thing",
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

        assert!(is_command_allowed_with_allowlist(
            "npx -y mcp_github",
            &allowlist
        ));

        assert!(is_command_allowed_with_allowlist(
            "executor thing",
            &allowlist
        ));

        assert!(!is_command_allowed_with_allowlist(
            "executor thing2",
            &allowlist
        ));

        assert!(!is_command_allowed_with_allowlist(
            "executor2 thing",
            &allowlist
        ));

        assert!(is_command_allowed_with_allowlist(
            "npx -y @mic/mcp_mic",
            &allowlist
        ));

        assert!(is_command_allowed_with_allowlist(
            "npx -y @mic/mcp_mic2@latest",
            &allowlist
        ));

        assert!(is_command_allowed_with_allowlist(
            "npx -y @mic/mcp_mic3",
            &allowlist
        ));

        assert!(is_command_allowed_with_allowlist(
            "npx -y @mic/mcp_mic4@latest",
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
    fn test_command_allowed_simple() {
        let allowlist = create_test_allowlist(&[
            "uvx something",
            "uvx mcp_slack",
            "npx mcp_github",
            "minecraft",
        ]);

        // Test with version, anything @version can be stripped when matching
        assert!(is_command_allowed_with_allowlist(
            "npx -y mcp_github@latest",
            &allowlist
        ));
    }

    #[test]
    fn test_command_allowed_flexible() {
        let allowlist = create_test_allowlist(&[
            "uvx something",
            "uvx mcp_slack",
            "npx -y mcp_github",
            "npx -y mcp_hammer start",
            "minecraft",
        ]);

        // Test with version, anything @version can be stripped when matching
        assert!(is_command_allowed_with_allowlist(
            "uvx something@1.0.13",
            &allowlist
        ));

        // Test with shim path - 'Goose.app/Contents/Resources/bin/' and before can be stripped to get the command to match
        assert!(is_command_allowed_with_allowlist(
            "/private/var/folders/fq/rd_cb6/T/AppTranslocation/EA0195/d/Goose.app/Contents/Resources/bin/uvx something",
            &allowlist
        ));

        // Test with shim path & latest version
        assert!(is_command_allowed_with_allowlist(
            "/private/var/folders/fq/rd_cb6/T/AppTranslocation/EA0195/d/Goose.app/Contents/Resources/bin/uvx something@latest",
            &allowlist
        ));

        // Test with exact command matches
        assert!(is_command_allowed_with_allowlist(
            "uvx something",
            &allowlist
        ));

        // Test with -y added, it is allowed (ie doesn't matter if we see a -y in there)
        assert!(is_command_allowed_with_allowlist(
            "npx -y mcp_github@latest",
            &allowlist
        ));

        // Test with -y added, and a version and parameter, it is allowed (npx mcp_hammer start is allowed)
        assert!(is_command_allowed_with_allowlist(
            "npx -y mcp_hammer@latest start",
            &allowlist
        ));

        // Test with shim path & latest version
        assert!(is_command_allowed_with_allowlist(
            "/private/var/folders/fq/rd_cb6/T/AppTranslocation/EA0195/d/Goose.app/Contents/Resources/bin/npx -y mcp_hammer@latest start",
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
    fn test_windows_paths() {
        let allowlist = create_test_allowlist(&["uvx mcp_snowflake", "uvx mcp_test"]);

        // Test various Windows path formats
        let test_paths = vec![
            // Standard Windows path
            r"C:\Users\MaxNovich\Downloads\Goose-1.0.17\resources\bin\uvx.exe",
            // Path with different casing
            r"C:\Users\MaxNovich\Downloads\Goose-1.0.17\Resources\Bin\uvx.exe",
            // Path with forward slashes
            r"C:/Users/MaxNovich/Downloads/Goose-1.0.17/resources/bin/uvx.exe",
            // Path with spaces
            r"C:\Program Files\Goose 1.0.17\resources\bin\uvx.exe",
            // Path with version numbers
            r"C:\Users\MaxNovich\Downloads\Goose-1.0.17-block.202504072238-76ffe-win32-x64\Goose-1.0.17-block.202504072238-76ffe-win32-x64\resources\bin\uvx.exe",
        ];

        for path in test_paths {
            // Test with @latest version
            let cmd = format!("{} mcp_snowflake@latest", path);
            assert!(
                is_command_allowed_with_allowlist(&cmd, &allowlist),
                "Failed for path: {}",
                path
            );

            // Test with specific version
            let cmd_version = format!("{} mcp_test@1.2.3", path);
            assert!(
                is_command_allowed_with_allowlist(&cmd_version, &allowlist),
                "Failed for path with version: {}",
                path
            );
        }

        // Test invalid paths that should be rejected
        let invalid_paths = vec![
            // Path without resources\bin
            r"C:\Users\MaxNovich\Downloads\uvx.exe",
            // Path with modified resources\bin
            r"C:\Users\MaxNovich\Downloads\Goose-1.0.17\resources_modified\bin\uvx.exe",
            // Path with extra components
            r"C:\Users\MaxNovich\Downloads\Goose-1.0.17\resources\bin\extra\uvx.exe",
        ];

        for path in invalid_paths {
            let cmd = format!("{} mcp_snowflake@latest", path);
            assert!(
                !is_command_allowed_with_allowlist(&cmd, &allowlist),
                "Should have rejected path: {}",
                path
            );
        }
    }

    #[test]
    fn test_windows_uvx_path() {
        let allowlist = create_test_allowlist(&["uvx mcp_snowflake"]);

        // Test Windows-style path with uvx.exe
        let windows_path = r"C:\Users\MaxNovich\Downloads\Goose-1.0.17-block.202504072238-76ffe-win32-x64\Goose-1.0.17-block.202504072238-76ffe-win32-x64\resources\bin\uvx.exe";
        let cmd = format!("{} mcp_snowflake@latest", windows_path);

        // This should be allowed because it's a valid uvx command in the Goose resources/bin directory
        assert!(is_command_allowed_with_allowlist(&cmd, &allowlist));

        // Test with different casing and backslashes
        let windows_path_alt = r"c:\Users\MaxNovich\Downloads\Goose-1.0.17-block.202504072238-76ffe-win32-x64\Goose-1.0.17-block.202504072238-76ffe-win32-x64\Resources\Bin\uvx.exe";
        let cmd_alt = format!("{} mcp_snowflake@latest", windows_path_alt);
        assert!(is_command_allowed_with_allowlist(&cmd_alt, &allowlist));
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

    #[test]
    fn test_allowlist_bypass() {
        // We need to directly test is_command_allowed_with_allowlist with our test allowlist
        // since get_allowed_extensions() might return None in the test environment

        // Create a restrictive allowlist
        let allowlist = create_test_allowlist(&["uvx mcp_slack"]);

        // Command not in allowlist
        let cmd = "uvx unauthorized_command";

        // Without bypass, command should be denied with our test allowlist
        assert!(!is_command_allowed_with_allowlist(cmd, &allowlist));

        // Set the bypass environment variable
        env::set_var("GOOSE_ALLOWLIST_BYPASS", "true");

        // With bypass enabled, any command should be allowed regardless of allowlist
        assert!(is_command_allowed(
            "uvx",
            &vec!["unauthorized_command".to_string()]
        ));

        // Test case insensitivity
        env::set_var("GOOSE_ALLOWLIST_BYPASS", "TRUE");
        assert!(is_command_allowed(
            "uvx",
            &vec!["unauthorized_command".to_string()]
        ));

        // Clean up
        env::remove_var("GOOSE_ALLOWLIST_BYPASS");

        // Create a mock function to test with allowlist and bypass
        let test_with_allowlist_and_bypass = |bypass_value: &str, expected: bool| {
            if bypass_value.is_empty() {
                env::remove_var("GOOSE_ALLOWLIST_BYPASS");
            } else {
                env::set_var("GOOSE_ALLOWLIST_BYPASS", bypass_value);
            }

            // This is what we're testing - a direct call that simulates what happens in is_command_allowed
            let result = if let Ok(bypass) = env::var("GOOSE_ALLOWLIST_BYPASS") {
                if bypass.to_lowercase() == "true" {
                    true
                } else {
                    is_command_allowed_with_allowlist(cmd, &allowlist)
                }
            } else {
                is_command_allowed_with_allowlist(cmd, &allowlist)
            };

            assert_eq!(
                result,
                expected,
                "With GOOSE_ALLOWLIST_BYPASS={}, expected allowed={}",
                if bypass_value.is_empty() {
                    "not set"
                } else {
                    bypass_value
                },
                expected
            );
        };

        // Test various bypass values
        test_with_allowlist_and_bypass("true", true);
        test_with_allowlist_and_bypass("TRUE", true);
        test_with_allowlist_and_bypass("True", true);
        test_with_allowlist_and_bypass("false", false);
        test_with_allowlist_and_bypass("0", false);
        test_with_allowlist_and_bypass("", false);

        // Final cleanup
        env::remove_var("GOOSE_ALLOWLIST_BYPASS");
    }
}
