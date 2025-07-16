use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

use crate::recipe::SubRecipe;

pub fn prepare_command_params(
    sub_recipe: &SubRecipe,
    params_from_tool_call: Vec<Value>,
) -> Result<Vec<HashMap<String, String>>> {
    let base_params = sub_recipe.values.clone().unwrap_or_default();

    if params_from_tool_call.is_empty() {
        return Ok(vec![base_params]);
    }

    let result = params_from_tool_call
        .into_iter()
        .map(|tool_param| {
            let mut param_map = base_params.clone();
            if let Some(param_obj) = tool_param.as_object() {
                for (key, value) in param_obj {
                    let value_str = value
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(|| value.to_string());
                    param_map.entry(key.clone()).or_insert(value_str);
                }
            }
            param_map
        })
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests;
