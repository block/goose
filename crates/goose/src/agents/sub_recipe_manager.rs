use mcp_core::{Content, Tool, ToolError};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    agents::recipe_tools::sub_recipe_tools::{
        call_sub_recipe_tool, create_sub_recipe_tool, SUB_RECIPE_TOOL_NAME_PREFIX,
    },
    recipe::SubRecipe,
};

#[derive(Debug, Clone)]
pub struct SubRecipeManager {
    pub sub_recipe_tools: HashMap<String, Tool>,
    pub sub_recipes: HashMap<String, SubRecipe>,
}

impl Default for SubRecipeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubRecipeManager {
    pub fn new() -> Self {
        Self {
            sub_recipe_tools: HashMap::new(),
            sub_recipes: HashMap::new(),
        }
    }

    pub fn add_sub_recipe_tools(&mut self, sub_recipes_to_add: Vec<SubRecipe>) {
        for sub_recipe in sub_recipes_to_add {
            let sub_recipe_name = sub_recipe.name.clone();
            let tool = create_sub_recipe_tool(&sub_recipe);
            self.sub_recipe_tools.insert(sub_recipe_name.clone(), tool);
            self.sub_recipes.insert(sub_recipe_name, sub_recipe);
        }
    }

    pub fn is_sub_recipe_tool(&self, tool_name: &str) -> bool {
        tool_name.starts_with(SUB_RECIPE_TOOL_NAME_PREFIX)
    }

    pub async fn run_sub_recipe(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<Vec<Content>, ToolError> {
        let sub_recipe_name = tool_name
            .strip_prefix(SUB_RECIPE_TOOL_NAME_PREFIX)
            .and_then(|s| s.strip_prefix("_"))
            .ok_or_else(|| {
                ToolError::InvalidParameters(format!(
                    "Invalid sub-recipe tool name format: {}",
                    tool_name
                ))
            })?;

        let sub_recipe = self.sub_recipes.get(sub_recipe_name).ok_or_else(|| {
            ToolError::InvalidParameters(format!("Sub-recipe '{}' not found", sub_recipe_name))
        })?;

        let output = call_sub_recipe_tool(sub_recipe, params).map_err(|e| {
            ToolError::ExecutionError(format!("Sub-recipe execution failed: {}", e))
        })?;
        Ok(vec![Content::text(output)])
    }
}
