use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use crate::agents::sub_recipe_execution_tool::types::{Task, TaskResult};

// Process a single task based on its type
pub async fn process_task(task: &Task, timeout_seconds: u64) -> TaskResult {
    let task_clone = task.clone();
    let timeout_duration = Duration::from_secs(timeout_seconds);

    // Execute with timeout
    match timeout(timeout_duration, execute_task(task_clone)).await {
        Ok(Ok(data)) => TaskResult {
            task_id: task.id.clone(),
            status: "success".to_string(),
            data: Some(data),
            error: None,
        },
        Ok(Err(error)) => TaskResult {
            task_id: task.id.clone(),
            status: "failed".to_string(),
            data: None,
            error: Some(error),
        },
        Err(_) => TaskResult {
            task_id: task.id.clone(),
            status: "failed".to_string(),
            data: None,
            error: Some("Task timeout".to_string()),
        },
    }
}

async fn execute_task(task: Task) -> Result<Value, String> {
    let mut output_identifier = task.id.clone();
    let mut command = if task.task_type == "sub_recipe" {
        let sub_recipe = task.payload.get("sub_recipe").unwrap();
        let sub_recipe_name = sub_recipe.get("name").unwrap().as_str().unwrap();
        let path = sub_recipe.get("recipe_path").unwrap().as_str().unwrap();
        let command_parameters = sub_recipe.get("command_parameters").unwrap();
        output_identifier = format!("sub-recipe {}", sub_recipe_name);
        let mut cmd = Command::new("goose");
        cmd.arg("run").arg("--recipe").arg(path);
        if let Some(params_map) = command_parameters.as_object() {
            for (key, value) in params_map {
                let key_str = key.to_string();
                let value_str = value.as_str().unwrap_or(&value.to_string()).to_string();
                cmd.arg("--params")
                    .arg(format!("{}={}", key_str, value_str));
            }
        }
        cmd
    } else {
        let text = task
            .payload
            .get("text_instruction")
            .unwrap()
            .as_str()
            .unwrap();
        let mut cmd = Command::new("goose");
        cmd.arg("run").arg("--text").arg(text);
        cmd
    };

    // Configure to capture stdout
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // Spawn the child process
    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn goose: {}", e))?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Spawn background tasks to read from stdout and stderr
    let output_identifier_clone = output_identifier.clone();
    let stdout_task = tokio::spawn(async move {
        let mut buffer = String::new();
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            println!("[{}] {}", output_identifier_clone, line);
            buffer.push_str(&line);
            buffer.push('\n');
        }
        buffer
    });

    let stderr_task = tokio::spawn(async move {
        let mut buffer = String::new();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            eprintln!("[stderr for {}] {}", output_identifier, line);
            buffer.push_str(&line);
            buffer.push('\n');
        }
        buffer
    });

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for process: {}", e))?;

    let stdout_output = stdout_task.await.unwrap();
    let stderr_output = stderr_task.await.unwrap();

    if status.success() {
        Ok(Value::String(stdout_output))
    } else {
        Err(format!("Command failed:\n{}", stderr_output))
    }
}
