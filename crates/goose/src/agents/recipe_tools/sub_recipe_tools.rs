use std::process::Command;

use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::{json, Value};

use crate::recipe::SubRecipe;

pub const SUB_RECIPE_TOOL_NAME_PREFIX: &str = "subrecipe__run_sub_recipe";

pub fn create_sub_recipe_tool(sub_recipe: &SubRecipe) -> Tool {
    Tool::new(
        format!("{}_{}", SUB_RECIPE_TOOL_NAME_PREFIX, sub_recipe.name),
        "Run a sub recipe.
        Use this tool when you need to run a sub-recipe.
        The sub recipe will be run with the provided parameters."
            .to_string(),
        get_sub_recipe_run_schema(),
        Some(ToolAnnotations {
            title: Some(format!("run sub recipe {}", sub_recipe.name)),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn run_sub_recipe_tool(sub_recipe: &SubRecipe, _params: Value) -> Result<String, String> {
    let mut command = Command::new("goose");
    command
        .arg("run")
        .arg("--recipe")
        .arg(&sub_recipe.path);
    if let Some(params) = &sub_recipe.params {
        for param in params {
            command.arg("--params");
            command.arg(format!("{}={}", param.name, param.value));
        }
    }
    let output = command
        .output()
        .map_err(|e| format!("Failed to execute: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
   

fn get_sub_recipe_run_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "params": {
                "type": "array",
                "description": "Parameters to override the existing parameters the sub-recipe",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "value": { "type": "string" }
                    }
                }
            }
        }
    })
}
