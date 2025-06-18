use std::{collections::HashMap, fs, process::Command};

use anyhow::Result;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::{json, Map, Value};

use crate::recipe::{Recipe, RecipeParameter, RecipeParameterRequirement, SubRecipe};

pub const SUB_RECIPE_TOOL_NAME_PREFIX: &str = "subrecipe__run_sub_recipe";

pub fn create_sub_recipe_tool(sub_recipe: &SubRecipe) -> Tool {
    let input_schema = get_input_schema(sub_recipe).unwrap();
    Tool::new(
        format!("{}_{}", SUB_RECIPE_TOOL_NAME_PREFIX, sub_recipe.name),
        "Run a sub recipe.
        Use this tool when you need to run a sub-recipe.
        The sub recipe will be run with the provided parameters."
            .to_string(),
        input_schema,
        Some(ToolAnnotations {
            title: Some(format!("run sub recipe {}", sub_recipe.name)),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
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
    if let Some(params_with_value) = &sub_recipe.params {
        for param_with_value in params_with_value {
            sub_recipe_params_map.insert(
                param_with_value.name.clone(),
                param_with_value.value.clone(),
            );
        }
    }

    let parameter_definition = get_sub_recipe_parameter_definition(sub_recipe)?;
    if let Some(parameters) = parameter_definition {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for param in parameters {
            let mut description = param.description.clone();
            if sub_recipe_params_map.contains_key(&param.key) {
                description = format!("{}, currently the value is set to {}. If you want to change the value, please provide a new value.", description, sub_recipe_params_map.get(&param.key).unwrap());
            }
            properties.insert(
                param.key.clone(),
                json!({
                    "type": param.input_type.to_string(),
                    "description": description,
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

pub fn call_sub_recipe_tool(sub_recipe: &SubRecipe, params: Value) -> Result<String, String> {
    println!("======= params: {:?}", params);
    let mut sub_recipe_params = HashMap::<String, String>::new();
    if let Some(params_with_value) = &sub_recipe.params {
        for param_with_value in params_with_value {
            sub_recipe_params.insert(
                param_with_value.name.clone(),
                param_with_value.value.clone(),
            );
        }
    }
    println!(
        "======= existing sub_recipe_params: {:?}",
        sub_recipe_params
    );
    if let Some(params_map) = params.as_object() {
        for (key, value) in params_map {
            println!("======= key: {:?}, value: {:?}", key, value);
            sub_recipe_params.insert(
                key.to_string(),
                value.as_str().unwrap_or(&value.to_string()).to_string(),
            );
        }
    }
    println!(
        "======= overridden sub_recipe_params: {:?}",
        sub_recipe_params
    );
    let mut command = Command::new("goose");
    command.arg("run").arg("--recipe").arg(&sub_recipe.path);
    for (key, value) in sub_recipe_params {
        command.arg("--params");
        command.arg(format!("{}={}", key, value));
    }
    println!("======= command: {:?}", command);
    let output = command
        .output()
        .map_err(|e| format!("Failed to execute: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
