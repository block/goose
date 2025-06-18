use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

use crate::recipe::SubRecipe;

pub const SUB_RECIPE_TOOL_NAME_PREFIX: &str = "subrecipe__run_sub_recipe";

pub const SUB_RECIPE_RUN_SCHEMA: &str = json!({
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
});

pub fn create_sub_recipe_tool(sub_recipe: SubRecipe) -> Tool {
    Tool::new(
        format!("{}_{}", SUB_RECIPE_TOOL_NAME_PREFIX, sub_recipe.name),
        "Run a sub recipe.
        Use this tool when you need to run a sub-recipe.
        The sub recipe will be run with the provided parameters.".to_string(),
        SUB_RECIPE_RUN_SCHEMA.to_string(),
        Some(ToolAnnotations {
            title: Some(format!("run sub recipe {}", sub_recipe.name)),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn run_tool(sub_recipe: SubRecipe, params: Vec<SubRecipeParams>) -> Result<String, String> {
    let tool = create_sub_recipe_tool(sub_recipe);
    let result = tool.call(params);
    Ok(result)
}