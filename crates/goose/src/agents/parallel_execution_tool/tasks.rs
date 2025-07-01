use crate::agents::parallel_execution_tool::types::{Task, TaskResult};
use serde_json::{json, Value};
use tokio::{
    process::Command,
    time::{timeout, Duration},
};

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
    let run_command_output = if task.task_type == "sub_recipe" {
        let sub_recipe = task.payload.get("sub_recipe").unwrap();
        let name = sub_recipe.get("name").unwrap().as_str().unwrap();
        let path = sub_recipe.get("recipe_path").unwrap().as_str().unwrap();
        let command_parameters = sub_recipe.get("command_parameters").unwrap();
        let mut command = Command::new("goose");
        command.arg("run").arg("--recipe").arg(path);
        if let Some(params_map) = command_parameters.as_object() {
            for (key, value) in params_map {
                let key_str = key.to_string();
                let value_str = value.as_str().unwrap_or(&value.to_string()).to_string();
                command
                    .arg("--params")
                    .arg(format!("{}={}", key_str, value_str));
            }
        }
        command
            .output()
            .await
            .map_err(|e| format!("Failed to run goose: {}", e))?
    } else {
        Command::new("goose")
            .arg("run")
            .arg("--text")
            .arg(
                task.payload
                    .get("text_instruction")
                    .unwrap()
                    .as_str()
                    .unwrap(),
            )
            .output()
            .await
            .map_err(|e| format!("Failed to run goose: {}", e))?
    };
    // Check for success
    if !run_command_output.status.success() {
        let stderr = String::from_utf8_lossy(&run_command_output.stderr);
        return Err(format!("Goose command failed: {}", stderr));
    }

    // Parse stdout as string
    let stdout = String::from_utf8_lossy(&run_command_output.stdout);

    // Wrap output in JSON
    Ok(json!({
        "output": stdout.trim(),
    }))
}