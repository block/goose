use tokio::time::{timeout, Duration};
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
    Ok(json!({
        "data": "my data",
    }))
}

// Compute task processor
async fn process_compute_task(task: &Task) -> Result<Value, String> {
    // Simulate CPU-bound work
    tokio::task::yield_now().await;
    
    if let Some(numbers) = task.payload.get("numbers").and_then(|v| v.as_array()) {
        let sum: f64 = numbers
            .iter()
            .filter_map(|v| v.as_f64())
            .sum();
        
        Ok(json!({
            "result": sum,
            "operation": "sum"
        }))
    } else {
        Err("Invalid compute task payload".to_string())
    }
}

// Fetch task processor (simulated)
async fn process_fetch_task(task: &Task) -> Result<Value, String> {
    if let Some(url) = task.payload.get("url").and_then(|v| v.as_str()) {
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // In real implementation, you would use reqwest here:
        // let response = reqwest::get(url).await?;
        
        Ok(json!({
            "url": url,
            "status": 200,
            "data": "Simulated response data"
        }))
    } else {
        Err("Invalid fetch task payload".to_string())
    }
}

// Transform task processor
async fn process_transform_task(task: &Task) -> Result<Value, String> {
    if let Some(data) = task.payload.get("data") {
        if let Some(operation) = task.payload.get("operation").and_then(|v| v.as_str()) {
            match operation {
                "uppercase" => {
                    if let Some(text) = data.as_str() {
                        Ok(json!({
                            "result": text.to_uppercase()
                        }))
                    } else {
                        Err("Data must be string for uppercase".to_string())
                    }
                }
                "lowercase" => {
                    if let Some(text) = data.as_str() {
                        Ok(json!({
                            "result": text.to_lowercase()
                        }))
                    } else {
                        Err("Data must be string for lowercase".to_string())
                    }
                }
                _ => Err(format!("Unknown transform operation: {}", operation)),
            }
        } else {
            Err("Missing operation in transform task".to_string())
        }
    } else {
        Err("Invalid transform task payload".to_string())
    }
}