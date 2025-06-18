use std::process::Stdio;

use mcp_core::{protocol::{JsonRpcMessage, JsonRpcNotification}, Content, Role, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{io::{AsyncBufReadExt, BufReader}, process::Command, sync::mpsc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubRecipeParameter {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubRecipeAttributes {
    pub path: String,
    pub params: Vec<SubRecipeParameter>,
    pub name: String,
}

pub async fn run_sub_recipe_command(
    sub_recipe_attributes: &SubRecipeAttributes,
    notifier: mpsc::Sender<JsonRpcMessage>,
) -> Result<Vec<Content>, ToolError> {
    let mut command = Command::new("goose");
    command.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .arg("run")
        .arg("--recipe")
        .arg(&sub_recipe_attributes.name);

    for param in &sub_recipe_attributes.params {
        command.arg(format!("--params={}={}", param.name, param.value));
    }

    let mut child = command.spawn().map_err(|e| ToolError::ExecutionError(format!("Failed to execute: {e}")))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    let output_task = tokio::spawn(async move {
        let mut combined_output = String::new();

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let mut stdout_done = false;
        let mut stderr_done = false;

        loop {
            tokio::select! {
                n = stdout_reader.read_until(b'\n', &mut stdout_buf), if !stdout_done => {
                    if n? == 0 {
                        stdout_done = true;
                    } else {
                        let line = String::from_utf8_lossy(&stdout_buf);

                        notifier.try_send(JsonRpcMessage::Notification(JsonRpcNotification {
                            jsonrpc: "2.0".to_string(),
                            method: "notifications/message".to_string(),
                            params: Some(json!({
                                "data": {
                                    "type": "run-sub-recipe",
                                    "stream": "stdout",
                                    "output": line.to_string(),
                                    "show_message_history": true,
                                }
                            })),
                        })).ok();

                        combined_output.push_str(&line);
                        stdout_buf.clear();
                    }
                }

                n = stderr_reader.read_until(b'\n', &mut stderr_buf), if !stderr_done => {
                    if n? == 0 {
                        stderr_done = true;
                    } else {
                        let line = String::from_utf8_lossy(&stderr_buf);

                        notifier.try_send(JsonRpcMessage::Notification(JsonRpcNotification {
                            jsonrpc: "2.0".to_string(),
                            method: "notifications/message".to_string(),
                            params: Some(json!({
                                "data": {
                                    "type": "run-sub-recipe",
                                    "stream": "stderr",
                                    "output": line.to_string(),
                                }
                            })),
                        })).ok();

                        combined_output.push_str(&line);
                        stderr_buf.clear();
                    }
                }

                else => break,
            }

            if stdout_done && stderr_done {
                break;
            }
        }
        Ok::<_, std::io::Error>(combined_output)
    });

    child
        .wait()
        .await
        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

    let output_str = match output_task.await {
        Ok(result) => result.map_err(|e| ToolError::ExecutionError(e.to_string()))?,
        Err(e) => return Err(ToolError::ExecutionError(e.to_string())),
    };
    Ok(vec![
        Content::text(output_str.clone()).with_audience(vec![Role::Assistant]),
        Content::text(output_str)
            .with_audience(vec![Role::User])
            .with_priority(0.0),
    ])
}

pub const SUB_RECIPE_RUN_SCHEMA: &str = r#"{
    "type": "object",
    "properties": {
        "params": {
            "type": "array",
            "description": "Parameters to override the existing parameters the sub-recipe",
            "items": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "value": { "type": "string" }
                }
            }
        }
    }
}"#;

pub const SUB_RECIPE_RUN_DESCRIPTION: &str = r#"
A tool for running a sub-recipe.
When you are given a sub-recipe, you should first read the sub-recipe file and understand the parameters that are required to run the sub-recipe.
Using params section of the sub-recipe in the main recipe as parameters to run the sub-recipe. If the required parameters of the sub-recipe are not provided, use the context to fill in the parameters.

Example usage:
Run a sub-recipe: {"parameters": [{"name": "date", "value": "2025-06-17"}]}
"#;
