use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;
use crate::agents::subagent_execution_tool::task_types::{Task, TaskResult, TaskStatus};
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

async fn get_task_result(
    task: Task,
    _task_execution_tracker: Arc<TaskExecutionTracker>,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    // All tasks now contain recipes directly - no conversion needed
    handle_recipe_task(task, task_config, cancellation_token).await
}

async fn handle_recipe_task(
    task: Task,
    mut task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    use crate::agents::subagent_handler::run_complete_subagent_task;
    use crate::model::ModelConfig;
    use crate::providers;
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

    // Apply recipe extensions if specified
    if let Some(exts) = recipe.extensions {
        if !exts.is_empty() {
            task_config.extensions = exts.clone();
        }
    }

    // Apply recipe provider settings if specified
    if let Some(settings) = recipe.settings {
        match (settings.goose_provider, settings.goose_model) {
            // Both provider and model specified - create new provider
            (Some(provider), Some(model)) => {
                let model_config =
                    ModelConfig::new_or_fail(&model).with_temperature(settings.temperature);
                task_config.provider = providers::create(&provider, model_config)
                    .map_err(|e| format!("Failed to create provider '{}': {}", provider, e))?;
            }
            // Only model specified - wrap parent provider with override
            (None, Some(model)) => {
                let mut model_config = task_config.provider.get_model_config();
                model_config.model_name = model.clone();
                if let Some(temp) = settings.temperature {
                    model_config = model_config.with_temperature(Some(temp));
                }

                // Wrap the parent provider with our override
                task_config.provider = Arc::new(providers::ModelOverrideProvider::new(
                    task_config.provider.clone(),
                    model_config,
                ));
            }
            // Only provider specified - error
            (Some(_), None) => {
                return Err("Recipe specifies provider but no model".to_string());
            }
            // Neither specified - just apply temperature if present
            (None, None) => {
                if let Some(temp) = settings.temperature {
                    let mut model_config = task_config.provider.get_model_config();
                    model_config = model_config.with_temperature(Some(temp));

                    task_config.provider = Arc::new(providers::ModelOverrideProvider::new(
                        task_config.provider.clone(),
                        model_config,
                    ));
                }
            }
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
