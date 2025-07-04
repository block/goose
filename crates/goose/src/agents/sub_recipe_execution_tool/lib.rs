use crate::agents::sub_recipe_execution_tool::executor::execute_single_task;
pub use crate::agents::sub_recipe_execution_tool::executor::parallel_execute;
pub use crate::agents::sub_recipe_execution_tool::types::{
    Config, ExecutionResponse, ExecutionStats, Task, TaskResult,
};

use serde_json::Value;

pub async fn execute_tasks(input: Value, execution_mode: &str) -> Result<Value, String> {
    let tasks: Vec<Task> =
        serde_json::from_value(input.get("tasks").ok_or("Missing tasks field")?.clone())
            .map_err(|e| format!("Failed to parse tasks: {}", e))?;

    let config: Config = if let Some(config_value) = input.get("config") {
        serde_json::from_value(config_value.clone())
            .map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        Config::default()
    };
    let task_count = tasks.len();
    match execution_mode {
        "sequential" => {
            if task_count == 1 {
                let response = execute_single_task(&tasks[0], config).await;
                serde_json::to_value(response)
                    .map_err(|e| format!("Failed to serialize response: {}", e))
            } else {
                Err("Sequential execution mode requires exactly one task".to_string())
            }
        }
        "parallel" => {
            let response = parallel_execute(tasks, config).await;
            serde_json::to_value(response)
                .map_err(|e| format!("Failed to serialize response: {}", e))
        }
        _ => Err("Invalid execution mode".to_string()),
    }
}
