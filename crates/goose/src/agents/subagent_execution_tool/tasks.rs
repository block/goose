use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;
use crate::agents::subagent_execution_tool::task_types::{Task, TaskResult, TaskStatus};
use crate::agents::subagent_task_config::TaskConfig;

pub async fn process_task(
    task: &Task,
    _task_execution_tracker: Arc<TaskExecutionTracker>,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> TaskResult {
    match handle_recipe_task(task.clone(), task_config, cancellation_token).await {
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

async fn handle_recipe_task(
    task: Task,
    mut task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    use crate::agents::subagent_handler::run_complete_subagent_task;
    use crate::model::ModelConfig;
    use crate::providers;

    let recipe = task.payload.recipe;
    let return_last_only = task.payload.return_last_only;

    // Apply recipe extensions if specified
    // - None: inherit parent extensions (default)
    // - Some([]): explicitly no extensions
    // - Some([...]): use only specified extensions
    if let Some(ref exts) = recipe.extensions {
        task_config.extensions = exts.clone();
    }

    // Apply recipe provider settings if specified
    if let Some(ref settings) = recipe.settings {
        // Validate: provider requires model
        if settings.goose_provider.is_some() && settings.goose_model.is_none() {
            return Err("Recipe specifies provider but no model".to_string());
        }

        // Apply settings based on what's specified
        if let (Some(provider), Some(model)) = (&settings.goose_provider, &settings.goose_model) {
            // Full replacement: new provider and model
            let model_config =
                ModelConfig::new_or_fail(model).with_temperature(settings.temperature);
            task_config.provider = providers::create(provider, model_config)
                .await
                .map_err(|e| format!("Failed to create provider '{}': {}", provider, e))?;
        } else if settings.goose_model.is_some() || settings.temperature.is_some() {
            // Partial override: wrap existing provider
            let mut model_config = task_config.provider.get_model_config();
            if let Some(model) = &settings.goose_model {
                model_config.model_name = model.clone();
            }
            if let Some(temp) = settings.temperature {
                model_config = model_config.with_temperature(Some(temp));
            }
            task_config.provider = Arc::new(providers::ModelOverrideProvider::new(
                task_config.provider.clone(),
                model_config,
            ));
        }
        // else: no changes needed, keep current provider
    }

    let result = tokio::select! {
        result = run_complete_subagent_task(
            recipe,
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
