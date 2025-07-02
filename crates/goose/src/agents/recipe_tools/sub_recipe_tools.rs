use std::{collections::HashMap, fs};

use anyhow::Result;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::{json, Map, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::recipe::{Recipe, RecipeParameter, RecipeParameterRequirement, SubRecipe};

pub const SUB_RECIPE_TOOL_NAME_PREFIX: &str = "subrecipe__run_";

pub fn create_sub_recipe_tool(sub_recipe: &SubRecipe) -> Tool {
    let input_schema = get_input_schema(sub_recipe).unwrap();
    Tool::new(
        format!("{}_{}", SUB_RECIPE_TOOL_NAME_PREFIX, sub_recipe.name),
        "Run a sub recipe.
        Use this tool when you need to run a sub-recipe.
        The sub recipe will be run with the provided parameters 
        and return the output of the sub recipe."
            .to_string(),
        input_schema,
        Some(ToolAnnotations {
            title: Some(format!("run sub recipe {}", sub_recipe.name)),
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: true,
        }),
    )
}

fn get_sub_recipe_parameter_definition(
    sub_recipe: &SubRecipe,
) -> Result<Option<Vec<RecipeParameter>>> {
    let content = fs::read_to_string(sub_recipe.path.clone())
        .map_err(|e| anyhow::anyhow!("Failed to read recipe file {}: {}", sub_recipe.path, e))?;
    let recipe = Recipe::from_content(&content)?;
    Ok(recipe.parameters)
}

fn get_input_schema(sub_recipe: &SubRecipe) -> Result<Value> {
    let mut sub_recipe_params_map = HashMap::<String, String>::new();
    if let Some(params_with_value) = &sub_recipe.values {
        for (param_name, param_value) in params_with_value {
            sub_recipe_params_map.insert(param_name.clone(), param_value.clone());
        }
    }
    let parameter_definition = get_sub_recipe_parameter_definition(sub_recipe)?;
    if let Some(parameters) = parameter_definition {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for param in parameters {
            if sub_recipe_params_map.contains_key(&param.key) {
                continue;
            }
            properties.insert(
                param.key.clone(),
                json!({
                    "type": param.input_type.to_string(),
                    "description": param.description.clone(),
                }),
            );
            if !matches!(param.requirement, RecipeParameterRequirement::Optional) {
                required.push(param.key);
            }
        }
        Ok(json!({
            "type": "object",
            "properties": properties,
            "required": required
        }))
    } else {
        Ok(json!({
            "type": "object",
            "properties": {}
        }))
    }
}

fn prepare_command_params(
    sub_recipe: &SubRecipe,
    params_from_tool_call: Value,
) -> Result<HashMap<String, String>> {
    let mut sub_recipe_params = HashMap::<String, String>::new();
    if let Some(params_with_value) = &sub_recipe.values {
        for (param_name, param_value) in params_with_value {
            sub_recipe_params.insert(param_name.clone(), param_value.clone());
        }
    }
    if let Some(params_map) = params_from_tool_call.as_object() {
        for (key, value) in params_map {
            sub_recipe_params.insert(
                key.to_string(),
                value.as_str().unwrap_or(&value.to_string()).to_string(),
            );
        }
    }
    Ok(sub_recipe_params)
}

pub async fn run_sub_recipe(sub_recipe: &SubRecipe, params: Value) -> Result<String> {
    let command_params = prepare_command_params(sub_recipe, params)?;

    let mut command = Command::new("goose");
    command.arg("run").arg("--recipe").arg(&sub_recipe.path);

    for (key, value) in command_params {
        command.arg("--params").arg(format!("{}={}", key, value));
    }

    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn: {}", e))?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let stdout_sub_recipe_name = sub_recipe.name.clone();
    let stderr_sub_recipe_name = sub_recipe.name.clone();

    // Spawn background tasks to read from stdout and stderr
    let stdout_task = tokio::spawn(async move {
        let mut buffer = String::new();
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            println!("[sub-recipe {}] {}", stdout_sub_recipe_name, line);
            buffer.push_str(&line);
            buffer.push('\n');
        }
        buffer
    });

    let stderr_task = tokio::spawn(async move {
        let mut buffer = String::new();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            eprintln!(
                "[stderr for sub-recipe {}] {}",
                stderr_sub_recipe_name, line
            );
            buffer.push_str(&line);
            buffer.push('\n');
        }
        buffer
    });

    let status = child
        .wait()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to wait for process: {}", e))?;

    let stdout_output = stdout_task.await.unwrap();
    let stderr_output = stderr_task.await.unwrap();

    if status.success() {
        Ok(stdout_output)
    } else {
        Err(anyhow::anyhow!("Command failed:\n{}", stderr_output))
    }
}

#[cfg(test)]
mod tests;
