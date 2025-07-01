use tokio::{process::Command, time::{timeout, Duration}};
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
    println!("Executing task: {:?}", task);
    
    let output = Command::new("goose")
        .arg("run") 
        .arg("--text")
        .arg(task.payload.to_string())
        .output()
        .await
        .map_err(|e| format!("Failed to run goose: {}", e))?;

    // Check for success
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Goose command failed: {}", stderr));
    }

    // Parse stdout as string
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Wrap output in JSON
    Ok(json!({
        "output": stdout.trim(),
    }))
}