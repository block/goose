use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::timeout;
use std::process::Stdio;
use std::time::Duration;
use serde_json::{json, Value};

use crate::agents::parallel_execution_tool::types::{Task, TaskResult};

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
    println!("=======Executing task: {:?}", task);

    let mut command = if task.task_type == "sub_recipe" {
        let sub_recipe = task.payload.get("sub_recipe").unwrap();
        let name = sub_recipe.get("name").unwrap().as_str().unwrap();
        let path = sub_recipe.get("recipe_path").unwrap().as_str().unwrap();
        let command_parameters = sub_recipe.get("command_parameters").unwrap();
        let mut cmd = Command::new("goose");
        cmd.arg("run").arg("--recipe").arg(path);
        if let Some(params_map) = command_parameters.as_object() {
            for (key, value) in params_map {
                let key_str = key.to_string();
                let value_str = value.as_str().unwrap_or(&value.to_string()).to_string();
                cmd.arg("--params").arg(format!("{}={}", key_str, value_str));
            }
        }
        cmd
    } else {
        let text = task.payload.get("text_instruction").unwrap().as_str().unwrap();
        let mut cmd = Command::new("goose");
        cmd.arg("run").arg("--text").arg(text);
        cmd
    };

    // Configure to capture stdout
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // Spawn the child process
    let mut child = command.spawn().map_err(|e| format!("Failed to spawn goose: {}", e))?;

    // Pipe the stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        println!("--- Goose output ---");
        while let Ok(Some(line)) = lines.next_line().await {
            println!("{}", line);
        }
    }

    // Await final status
    let status = child.wait().await.map_err(|e| format!("Failed to wait on goose: {}", e))?;
    if !status.success() {
        return Err(format!("Goose command failed with exit code: {:?}", status.code()));
    }

    Ok(json!({ "output": "Goose command completed." }))
}