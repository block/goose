use std::collections::HashSet;
use std::fs;
use std::sync::Arc;

use anyhow::Result;
use rmcp::model::{Tool, ToolAnnotations};
use serde_json::{json, Map, Value};

use crate::agents::subagent_execution_tool::lib::{ExecutionMode, Task};
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::recipe::{Recipe, RecipeParameter, RecipeParameterRequirement, SubRecipe};

use super::param_utils::prepare_command_params;

pub const SUB_RECIPE_TASK_TOOL_NAME_PREFIX: &str = "subrecipe__create_task";

pub fn create_sub_recipe_task_tool(sub_recipe: &SubRecipe) -> Tool {
    let input_schema = get_input_schema(sub_recipe).unwrap();

    Tool::new(
        format!("{}_{}", SUB_RECIPE_TASK_TOOL_NAME_PREFIX, sub_recipe.name),
        format!(
            "Create one or more tasks to run the '{}' sub recipe. \
            Provide an array of parameter sets in the 'task_parameters' field:\n\
            - For a single task: provide an array with one parameter set\n\
            - For multiple tasks: provide an array with multiple parameter sets, each with different values\n\n\
            Each task will run the same sub recipe but with different parameter values. \
            This is useful when you need to execute the same sub recipe multiple times with varying inputs. \
            After creating the tasks and execution_mode is provided, pass them to the task executor to run these tasks",
            sub_recipe.name
        ),
        Arc::new(input_schema.as_object().unwrap().clone())
    ).annotate(ToolAnnotations {
        title: Some(format!(
            "create multiple sub recipe tasks for {}",
            sub_recipe.name
        )),
        read_only_hint: Some(false),
        destructive_hint: Some(true),
        idempotent_hint: Some(false),
        open_world_hint: Some(true),
    })
}

fn extract_task_parameters(params: &Value) -> Vec<Value> {
    params
        .get("task_parameters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn create_tasks_from_params(
    sub_recipe: &SubRecipe,
    command_params: &[std::collections::HashMap<String, String>],
) -> Vec<Task> {
    let tasks: Vec<Task> = command_params
        .iter()
        .map(|task_command_param| {
            let payload = json!({
                "sub_recipe": {
                    "name": sub_recipe.name.clone(),
                    "command_parameters": task_command_param,
                    "recipe_path": sub_recipe.path.clone(),
                    "sequential_when_repeated": sub_recipe.sequential_when_repeated
                }
            });
            Task {
                id: uuid::Uuid::new_v4().to_string(),
                task_type: "sub_recipe".to_string(),
                payload,
            }
        })
        .collect();

    tasks
}

fn create_task_execution_payload(tasks: &[Task], sub_recipe: &SubRecipe) -> Value {
    let execution_mode = if tasks.len() == 1 || sub_recipe.sequential_when_repeated {
        ExecutionMode::Sequential
    } else {
        ExecutionMode::Parallel
    };
    let task_ids: Vec<String> = tasks.iter().map(|task| task.id.clone()).collect();
    json!({
        "task_ids": task_ids,
        "execution_mode": execution_mode,
    })
}

pub async fn create_sub_recipe_task(
    sub_recipe: &SubRecipe,
    params: Value,
    tasks_manager: &TasksManager,
) -> Result<String> {
    let task_params_array = extract_task_parameters(&params);
    let command_params = prepare_command_params(sub_recipe, task_params_array.clone())?;
    let tasks = create_tasks_from_params(sub_recipe, &command_params);
    let task_execution_payload = create_task_execution_payload(&tasks, sub_recipe);

    let tasks_json = serde_json::to_string(&task_execution_payload)
        .map_err(|e| anyhow::anyhow!("Failed to serialize task list: {}", e))?;
    tasks_manager.save_tasks(tasks.clone()).await;
    Ok(tasks_json)
}

fn get_sub_recipe_parameter_definition(
    sub_recipe: &SubRecipe,
) -> Result<Option<Vec<RecipeParameter>>> {
    let content = fs::read_to_string(sub_recipe.path.clone())
        .map_err(|e| anyhow::anyhow!("Failed to read recipe file {}: {}", sub_recipe.path, e))?;
    let recipe = Recipe::from_content(&content)?;
    Ok(recipe.parameters)
}

fn get_params_with_values(sub_recipe: &SubRecipe) -> HashSet<String> {
    let mut sub_recipe_params_with_values = HashSet::<String>::new();
    if let Some(params_with_value) = &sub_recipe.values {
        for param_name in params_with_value.keys() {
            sub_recipe_params_with_values.insert(param_name.clone());
        }
    }
    sub_recipe_params_with_values
}

fn create_input_schema(param_properties: Map<String, Value>, param_required: Vec<String>) -> Value {
    let mut properties = Map::new();
    if !param_properties.is_empty() {
        properties.insert(
            "task_parameters".to_string(),
            json!({
                "type": "array",
                "description": "Array of parameter sets for creating tasks. \
                    For a single task, provide an array with one element. \
                    For multiple tasks, provide an array with multiple elements, each with different parameter values. \
                    If there is no parameter set, provide an empty array.",
                "items": {
                    "type": "object",
                    "properties": param_properties,
                    "required": param_required
                },
            })
        );
    }
    json!({
        "type": "object",
        "properties": properties,
    })
}

fn get_input_schema(sub_recipe: &SubRecipe) -> Result<Value> {
    let sub_recipe_params_with_values = get_params_with_values(sub_recipe);

    let parameter_definition = get_sub_recipe_parameter_definition(sub_recipe)?;

    let mut param_properties = Map::new();
    let mut param_required = Vec::new();

    if let Some(parameters) = parameter_definition {
        for param in parameters {
            if sub_recipe_params_with_values.contains(&param.key.clone()) {
                continue;
            }
            param_properties.insert(
                param.key.clone(),
                json!({
                    "type": param.input_type.to_string(),
                    "description": param.description.clone(),
                }),
            );
            if !matches!(param.requirement, RecipeParameterRequirement::Optional) {
                param_required.push(param.key);
            }
        }
    }
    Ok(create_input_schema(param_properties, param_required))
}

#[cfg(test)]
mod tests;
