use mcp_core::{Content, Tool, ToolError};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    agents::{
        recipe_tools::sub_recipe_tools::{
            create_sub_recipe_tool, run_sub_recipe, SUB_RECIPE_TOOL_NAME_PREFIX,
        },
        tool_execution::ToolCallResult,
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
            let sub_recipe_key = format!(
                "{}_{}",
                SUB_RECIPE_TOOL_NAME_PREFIX,
                sub_recipe.name.clone()
            );
            let tool = create_sub_recipe_tool(&sub_recipe);
            self.sub_recipe_tools.insert(sub_recipe_key.clone(), tool);
            self.sub_recipes.insert(sub_recipe_key.clone(), sub_recipe);
        }
    }

    pub fn is_sub_recipe_tool(&self, tool_name: &str) -> bool {
        self.sub_recipe_tools.contains_key(tool_name)
    }

    pub async fn dispatch_sub_recipe_tool_call(
        &self,
        tool_name: &str,
        params: Value,
    ) -> ToolCallResult {
        let result = self.call_sub_recipe_tool(tool_name, params).await;
        match result {
            Ok(call_result) => ToolCallResult::from(Ok(call_result)),
            Err(e) => ToolCallResult::from(Err(ToolError::ExecutionError(e.to_string()))),
        }
    }

    async fn call_sub_recipe_tool(
        &self,
        tool_name: &str,
        params: Value,
    ) -> Result<Vec<Content>, ToolError> {
        let sub_recipe = self.sub_recipes.get(tool_name).ok_or_else(|| {
            let sub_recipe_name = tool_name
                .strip_prefix(SUB_RECIPE_TOOL_NAME_PREFIX)
                .and_then(|s| s.strip_prefix("_"))
                .ok_or_else(|| {
                    ToolError::InvalidParameters(format!(
                        "Invalid sub-recipe tool name format: {}",
                        tool_name
                    ))
                })
                .unwrap();

            ToolError::InvalidParameters(format!("Sub-recipe '{}' not found", sub_recipe_name))
        })?;

        let output = run_sub_recipe(sub_recipe, params).await.map_err(|e| {
            ToolError::ExecutionError(format!("Sub-recipe execution failed: {}", e))
        })?;
        Ok(vec![Content::text(output)])
    }
}
