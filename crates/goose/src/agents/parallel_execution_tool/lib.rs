pub use crate::agents::parallel_execution_tool::types::{Task, TaskResult, Config, ExecutionResponse, ExecutionStats};
pub use crate::agents::parallel_execution_tool::executor::parallel_execute;

use serde_json::Value;

pub async fn llm_parallel_execute(input: Value) -> Result<Value, String> {
    let tasks: Vec<Task> = serde_json::from_value(
        input.get("tasks")
            .ok_or("Missing tasks field")?
            .clone()
    ).map_err(|e| format!("Failed to parse tasks: {}", e))?;
    
    let config: Config = if let Some(config_value) = input.get("config") {
        serde_json::from_value(config_value.clone())
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        Config::default()
    };
    
    // Execute tasks
    let response = parallel_execute(tasks, config).await;
    
    // Convert response to JSON
    serde_json::to_value(response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}