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

    if let Some(ref exts) = recipe.extensions {
        task_config.extensions = exts.clone();
    }

    if let Some(ref settings) = recipe.settings {
        let new_provider = match (
            &settings.goose_provider,
            &settings.goose_model,
            settings.temperature,
        ) {
            (Some(provider), Some(model), temp) => {
                let config = ModelConfig::new_or_fail(model).with_temperature(temp);
                Some((provider.clone(), config))
            }
            (Some(_), None, _) => {
                return Err("Recipe specifies provider but no model".to_string());
            }
            (None, model_or_temp, _)
                if model_or_temp.is_some() || settings.temperature.is_some() =>
            {
                let provider_name = task_config.provider.get_name().to_string();
                let mut config = task_config.provider.get_model_config();

                if let Some(model) = &settings.goose_model {
                    config.model_name = model.clone();
                }
                if let Some(temp) = settings.temperature {
                    config = config.with_temperature(Some(temp));
                }

                Some((provider_name, config))
            }
            _ => None,
        };

        if let Some((provider_name, model_config)) = new_provider {
            task_config.provider = providers::create(&provider_name, model_config)
                .await
                .map_err(|e| format!("Failed to create provider '{}': {}", provider_name, e))?;
        }
    }

    tokio::select! {
        result = run_complete_subagent_task(recipe, task_config, return_last_only, task.id.clone()) => {
            result.map(|text| serde_json::json!({"result": text}))
                  .map_err(|e| format!("Recipe execution failed: {}", e))
        }
        _ = cancellation_token.cancelled() => {
            Err("Task cancelled".to_string())
        }
    }
}
