use crate::recipes::print_recipe::{
    missing_parameters_command_line, print_parameters_with_values, print_recipe_explanation,
    print_required_parameters_for_template,
};
use crate::recipes::search_recipe::{retrieve_recipe_file, RecipeFile};
use crate::recipes::template_recipe::{
    parse_recipe_content, render_recipe_content_with_params, render_recipe_for_preview,
};
use anyhow::Result;
use console::style;
use goose::recipe::{Recipe, RecipeParameter, RecipeParameterRequirement};
use std::collections::{HashMap, HashSet};

pub const BUILT_IN_RECIPE_DIR_PARAM: &str = "recipe_dir";
pub const RECIPE_FILE_EXTENSIONS: &[&str] = &["yaml", "json"];

pub fn load_recipe_content_as_template(
    recipe_name: &str,
    params: Vec<(String, String)>,
) -> Result<String> {
    let RecipeFile {
        content: recipe_file_content,
        parent_dir: recipe_parent_dir,
        ..
    } = retrieve_recipe_file(recipe_name)?;
    let recipe_dir_str = recipe_parent_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Error getting recipe directory"))?;
    let recipe_parameters = validate_recipe_parameters(&recipe_file_content, recipe_dir_str)?;

    let (params_for_template, missing_params) =
        apply_values_to_parameters(&params, recipe_parameters, recipe_dir_str, true)?;

    if !missing_params.is_empty() {
        return Err(anyhow::anyhow!(
            "Please provide the following parameters in the command line: {}",
            missing_parameters_command_line(missing_params)
        ));
    }

    render_recipe_content_with_params(&recipe_file_content, &params_for_template)
}

fn validate_recipe_parameters(
    recipe_file_content: &str,
    recipe_dir_str: &str,
) -> Result<Option<Vec<RecipeParameter>>> {
    let (raw_recipe, template_variables) =
        parse_recipe_content(recipe_file_content, recipe_dir_str.to_string())?;
    let recipe_parameters = raw_recipe.parameters;
    validate_optional_parameters(&recipe_parameters)?;
    validate_parameters_in_template(&recipe_parameters, &template_variables)?;
    Ok(recipe_parameters)
}

pub fn load_recipe_as_template(recipe_name: &str, params: Vec<(String, String)>) -> Result<Recipe> {
    let rendered_content = load_recipe_content_as_template(recipe_name, params.clone())?;
    let recipe = Recipe::from_content(&rendered_content)?;

    // Display information about the loaded recipe
    println!(
        "{} {}",
        style("Loading recipe:").green().bold(),
        style(&recipe.title).green()
    );
    println!("{} {}", style("Description:").bold(), &recipe.description);

    if !params.is_empty() {
        println!("{}", style("Parameters used to load this recipe:").bold());
        print_parameters_with_values(params.into_iter().collect());
    }
    println!();
    Ok(recipe)
}

pub fn load_recipe(recipe_name: &str) -> Result<Recipe> {
    let RecipeFile {
        content: recipe_file_content,
        parent_dir: recipe_parent_dir,
        ..
    } = retrieve_recipe_file(recipe_name)?;
    let recipe_dir_str = recipe_parent_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Error getting recipe directory"))?;
    validate_recipe_parameters(&recipe_file_content, recipe_dir_str)?;
    let recipe = render_recipe_for_preview(
        &recipe_file_content,
        recipe_dir_str.to_string(),
        &HashMap::new(),
    )?;

    if let Some(response) = &recipe.response {
        if let Some(json_schema) = &response.json_schema {
            validate_json_schema(json_schema)?;
        }
    }

    Ok(recipe)
}

pub fn explain_recipe_with_parameters(
    recipe_name: &str,
    params: Vec<(String, String)>,
) -> Result<()> {
    let RecipeFile {
        content: recipe_file_content,
        parent_dir: recipe_parent_dir,
        ..
    } = retrieve_recipe_file(recipe_name)?;
    let recipe_dir_str = recipe_parent_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Error getting recipe directory"))?;
    let recipe_parameters = validate_recipe_parameters(&recipe_file_content, recipe_dir_str)?;

    let (params_for_template, missing_params) =
        apply_values_to_parameters(&params, recipe_parameters, recipe_dir_str, false)?;
    let recipe = render_recipe_for_preview(
        &recipe_file_content,
        recipe_dir_str.to_string(),
        &params_for_template,
    )?;
    print_recipe_explanation(&recipe);
    print_required_parameters_for_template(params_for_template, missing_params);

    Ok(())
}

fn validate_parameters_in_template(
    recipe_parameters: &Option<Vec<RecipeParameter>>,
    template_variables: &HashSet<String>,
) -> Result<()> {
    let mut template_variables = template_variables.clone();
    template_variables.remove(BUILT_IN_RECIPE_DIR_PARAM);

    let param_keys: HashSet<String> = recipe_parameters
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|p| p.key.clone())
        .collect();

    let missing_keys = template_variables
        .difference(&param_keys)
        .collect::<Vec<_>>();

    let extra_keys = param_keys
        .difference(&template_variables)
        .collect::<Vec<_>>();

    if missing_keys.is_empty() && extra_keys.is_empty() {
        return Ok(());
    }

    let mut message = String::new();

    if !missing_keys.is_empty() {
        message.push_str(&format!(
            "Missing definitions for parameters in the recipe file: {}.",
            missing_keys
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if !extra_keys.is_empty() {
        message.push_str(&format!(
            "\nUnnecessary parameter definitions: {}.",
            extra_keys
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Err(anyhow::anyhow!("{}", message.trim_end()))
}

fn validate_optional_parameters(parameters: &Option<Vec<RecipeParameter>>) -> Result<()> {
    let optional_params_without_default_values: Vec<String> = parameters
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .filter(|p| {
            matches!(p.requirement, RecipeParameterRequirement::Optional) && p.default.is_none()
        })
        .map(|p| p.key.clone())
        .collect();

    if optional_params_without_default_values.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Optional parameters missing default values in the recipe: {}. Please provide defaults.", optional_params_without_default_values.join(", ")))
    }
}

fn apply_values_to_parameters(
    user_params: &[(String, String)],
    recipe_parameters: Option<Vec<RecipeParameter>>,
    recipe_parent_dir: &str,
    enable_user_prompt: bool,
) -> Result<(HashMap<String, String>, Vec<String>)> {
    let mut param_map: HashMap<String, String> = user_params.iter().cloned().collect();
    param_map.insert(
        BUILT_IN_RECIPE_DIR_PARAM.to_string(),
        recipe_parent_dir.to_string(),
    );
    let mut missing_params: Vec<String> = Vec::new();
    for param in recipe_parameters.unwrap_or_default() {
        if !param_map.contains_key(&param.key) {
            match (&param.default, &param.requirement) {
                (Some(default), _) => param_map.insert(param.key.clone(), default.clone()),
                (None, RecipeParameterRequirement::UserPrompt) if enable_user_prompt => {
                    let input_value = cliclack::input(format!(
                        "Please enter {} ({})",
                        param.key, param.description
                    ))
                    .interact()?;
                    param_map.insert(param.key.clone(), input_value)
                }
                _ => {
                    missing_params.push(param.key.clone());
                    None
                }
            };
        }
    }
    Ok((param_map, missing_params))
}

fn validate_json_schema(schema: &serde_json::Value) -> Result<()> {
    match jsonschema::validator_for(schema) {
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow::anyhow!("JSON schema validation failed: {}", err)),
    }
}

#[cfg(test)]
mod tests;
