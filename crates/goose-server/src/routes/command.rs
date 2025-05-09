use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

// Request for executing a command
#[derive(Deserialize)]
pub struct CommandRequest {
    command: String,
}

// Response with command execution results
#[derive(Serialize)]
pub struct CommandResponse {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    error: Option<String>,
}

// Allow list of commands that can be executed
const ALLOWED_COMMANDS: [&str; 2] = [
    "ps",    // Process status
    "pgrep", // Process grep
];

// Handler for executing a command
async fn execute_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, StatusCode> {
    // Verify the secret key
    verify_secret_key(&headers, &state)?;

    // Extract the base command (first word)
    let cmd_parts: Vec<&str> = req.command.split_whitespace().collect();
    if cmd_parts.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let base_cmd = cmd_parts[0];

    // Security check: Validate against allowed commands
    if !ALLOWED_COMMANDS.contains(&base_cmd) {
        return Ok(Json(CommandResponse {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            error: Some(format!("Command '{}' is not allowed", base_cmd)),
        }));
    }

    // Execute the command with a timeout
    match execute_with_timeout(&req.command, Duration::from_secs(5)).await {
        Ok(result) => Ok(Json(result)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Execute a command with timeout
async fn execute_with_timeout(cmd: &str, duration: Duration) -> Result<CommandResponse, String> {
    // Use tokio's timeout to limit execution time
    let result = timeout(duration, async {
        // On Unix-like systems, use sh to execute the command
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(&["/C", cmd]).output()
        } else {
            Command::new("sh").arg("-c").arg(cmd).output()
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                CommandResponse {
                    stdout,
                    stderr,
                    exit_code: output.status.code(),
                    error: None,
                }
            }
            Err(e) => CommandResponse {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error: Some(format!("Failed to execute command: {}", e)),
            },
        }
    })
    .await;

    match result {
        Ok(response) => Ok(response),
        Err(_) => Ok(CommandResponse {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            error: Some("Command execution timed out".to_string()),
        }),
    }
}

// Register routes
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/command", post(execute_command))
        .with_state(state)
}
