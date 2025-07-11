use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

use crate::recipe::SubRecipe;

pub fn extract_run_params(
    sub_recipe: &SubRecipe,
) -> (HashMap<String, String>, Vec<HashMap<String, String>>) {
    let base_params = sub_recipe.values.clone().unwrap_or_default();

    let run_params = sub_recipe
        .executions
        .as_ref()
        .and_then(|e| e.runs.as_ref())
        .map(|runs| {
            runs.iter()
                .map(|run| {
                    let mut params = base_params.clone();
                    if let Some(run_values) = &run.values {
                        params.extend(run_values.clone());
                    }
                    params
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (base_params, run_params)
}

pub fn validate_param_counts(
    run_params: &[HashMap<String, String>],
    params_from_tool_call: &[Value],
) -> Result<()> {
    let has_run_params = !run_params.is_empty();
    let multiple_params_from_tool_call = params_from_tool_call.len() > 1;
    let count_mismatch = run_params.len() != params_from_tool_call.len();

    if has_run_params && multiple_params_from_tool_call && count_mismatch {
        return Err(anyhow::anyhow!(
            "The number of runs in the sub recipe ({}) does not match the number of task parameters ({})",
            run_params.len(),
            params_from_tool_call.len()
        ));
    }
    Ok(())
}

pub fn prepare_base_params(
    base_params: &HashMap<String, String>,
    run_params: &[HashMap<String, String>],
    params_from_tool_call: &[Value],
) -> Vec<HashMap<String, String>> {
    if run_params.is_empty() {
        vec![base_params.clone(); params_from_tool_call.len()]
    } else {
        run_params.to_vec()
    }
}

pub fn prepare_tool_params(
    params_from_tool_call: &[Value],
    run_params: &[HashMap<String, String>],
) -> Vec<Value> {
    if params_from_tool_call.len() == 1 && run_params.len() > 1 {
        vec![params_from_tool_call[0].clone(); run_params.len()]
    } else {
        params_from_tool_call.to_vec()
    }
}

pub fn merge_parameters(
    tool_params: Vec<Value>,
    base_params: Vec<HashMap<String, String>>,
) -> Vec<HashMap<String, String>> {
    tool_params
        .into_iter()
        .zip(base_params)
        .map(|(tool_param, mut base_param_map)| {
            if let Some(param_obj) = tool_param.as_object() {
                for (key, value) in param_obj {
                    let value_str = value
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(|| value.to_string());
                    base_param_map.entry(key.clone()).or_insert(value_str);
                }
            }
            base_param_map
        })
        .collect()
}

pub fn prepare_command_params(
    sub_recipe: &SubRecipe,
    params_from_tool_call: Vec<Value>,
) -> Result<Vec<HashMap<String, String>>> {
    let (base_params, run_params) = extract_run_params(sub_recipe);

    if params_from_tool_call.is_empty() {
        return Ok(run_params);
    }

    validate_param_counts(&run_params, &params_from_tool_call)?;

    let base_params_for_merging =
        prepare_base_params(&base_params, &run_params, &params_from_tool_call);
    let tool_params_for_merging = prepare_tool_params(&params_from_tool_call, &run_params);

    Ok(merge_parameters(
        tool_params_for_merging,
        base_params_for_merging,
    ))
}

#[cfg(test)]
mod tests;
