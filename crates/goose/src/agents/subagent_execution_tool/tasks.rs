use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;
use crate::agents::subagent_execution_tool::task_types::{Task, TaskResult, TaskStatus, TaskType};
use crate::agents::subagent_task_config::TaskConfig;

pub async fn process_task(
    task: &Task,
    task_execution_tracker: Arc<TaskExecutionTracker>,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> TaskResult {
    match get_task_result(
        task.clone(),
        task_execution_tracker,
        task_config,
        cancellation_token,
    )
    .await
    {
        Ok(data) => TaskResult {
            task_id: task.id.clone(),
            status: TaskStatus::Completed,
            data: Some(data),
            error: None,
        },
        Err(error) => TaskResult {
            task_id: task.id.clone(),
            status: TaskStatus::Failed,
            data: None,
            error: Some(error),
        },
    }
}

/// Load a SubRecipe from file and convert to Recipe object
async fn load_sub_recipe(task: &Task) -> Result<crate::recipe::Recipe, String> {
    use crate::recipe::build_recipe::build_recipe_from_template;
    use crate::recipe::local_recipes::load_local_recipe_file;
    
    let path = task.get_sub_recipe_path()
        .ok_or("Missing path in sub_recipe payload")?;
    let params = task.get_command_parameters()
        .map(|m| m.iter().map(|(k,v)| (k.to_string(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();
    
    let recipe_file = load_local_recipe_file(path).map_err(|e| e.to_string())?;
    build_recipe_from_template(recipe_file, params, None::<fn(&str, &str) -> Result<String, anyhow::Error>>).map_err(|e| e.to_string())
}

async fn get_task_result(
    task: Task,
    _task_execution_tracker: Arc<TaskExecutionTracker>,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    // Convert SubRecipe to InlineRecipe if needed
    let task = match task.task_type {
        TaskType::SubRecipe => {
            let recipe = load_sub_recipe(&task).await?;
            Task {
                id: task.id,
                task_type: TaskType::InlineRecipe,
                payload: serde_json::json!({
                    "recipe": recipe,
                    "return_last_only": false
                }),
            }
        }
        TaskType::InlineRecipe => task,
    };
    
    // Single execution path for all recipes
    handle_inline_recipe_task(task, task_config, cancellation_token).await
}

async fn handle_inline_recipe_task(
    task: Task,
    mut task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    use crate::agents::subagent_handler::run_complete_subagent_task;
    use crate::recipe::Recipe;

    let recipe_value = task
        .payload
        .get("recipe")
        .ok_or_else(|| "Missing recipe in inline_recipe task payload".to_string())?;

    let recipe: Recipe = serde_json::from_value(recipe_value.clone())
        .map_err(|e| format!("Invalid recipe in payload: {}", e))?;

    let return_last_only = task
        .payload
        .get("return_last_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if let Some(exts) = recipe.extensions {
        if !exts.is_empty() {
            task_config.extensions = exts.clone();
        }
    }

    let instruction = recipe
        .instructions
        .or(recipe.prompt)
        .ok_or_else(|| "No instructions or prompt in recipe".to_string())?;

    let result = tokio::select! {
        result = run_complete_subagent_task(
            instruction,
            task_config,
            return_last_only,
        ) => result,
        _ = cancellation_token.cancelled() => {
            return Err("Task cancelled".to_string());
        }
    };

    match result {
        Ok(result_text) => Ok(serde_json::json!({
            "result": result_text
        })),
        Err(e) => {
            let error_msg = format!("Inline recipe execution failed: {}", e);
            Err(error_msg)
        }
    }
}
