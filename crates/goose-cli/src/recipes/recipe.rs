use crate::recipes::print_recipe::{
    missing_parameters_command_line, print_recipe_explanation,
    print_required_parameters_for_template,
};
use crate::recipes::search_recipe::retrieve_recipe_file;
use anyhow::Result;
use goose::recipe::build_recipe::{
    apply_values_to_parameters, build_recipe_from_template, validate_recipe_parameters,
};
use goose::recipe::read_recipe_file_content::RecipeFile;
use goose::recipe::template_recipe::render_recipe_for_preview;
use goose::recipe::Recipe;
use std::collections::HashMap;

pub const RECIPE_FILE_EXTENSIONS: &[&str] = &["yaml", "json"];

fn create_user_prompt_callback() -> impl Fn(&str, &str) -> Result<String> {
    |key: &str, description: &str| -> Result<String> {
        let input_value =
            cliclack::input(format!("Please enter {} ({})", key, description)).interact()?;
        Ok(input_value)
    }
}

fn load_recipe_file_with_dir(recipe_name: &str) -> Result<(RecipeFile, String)> {
    let recipe_file = retrieve_recipe_file(recipe_name)?;
    let recipe_dir_str = recipe_file
        .parent_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Error getting recipe directory"))?
        .to_string();
    Ok((recipe_file, recipe_dir_str))
}

pub fn load_recipe(recipe_name: &str, params: Vec<(String, String)>) -> Result<Recipe> {
    let recipe_file = retrieve_recipe_file(recipe_name)?;
    let (recipe_opt, missing_params) =
        build_recipe_from_template(recipe_file, params, Some(create_user_prompt_callback()))?;

    if !missing_params.is_empty() {
        return Err(anyhow::anyhow!(
            "Please provide the following parameters in the command line: {}",
            missing_parameters_command_line(missing_params)
        ));
    }

    recipe_opt.ok_or_else(|| anyhow::anyhow!("Failed to build recipe"))
}

pub fn render_recipe_as_yaml(recipe_name: &str, params: Vec<(String, String)>) -> Result<()> {
    let recipe = load_recipe(recipe_name, params)?;
    match serde_yaml::to_string(&recipe) {
        Ok(yaml_content) => {
            println!("{}", yaml_content);
            Ok(())
        }
        Err(_) => {
            eprintln!("Failed to serialize recipe to YAML");
            std::process::exit(1);
        }
    }
}

pub fn load_recipe_for_validation(recipe_name: &str) -> Result<Recipe> {
    let (recipe_file, recipe_dir_str) = load_recipe_file_with_dir(recipe_name)?;
    let recipe_file_content = &recipe_file.content;
    validate_recipe_parameters(recipe_file_content, &recipe_dir_str)?;
    let recipe = render_recipe_for_preview(
        recipe_file_content,
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

pub fn explain_recipe(recipe_name: &str, params: Vec<(String, String)>) -> Result<()> {
    let (recipe_file, recipe_dir_str) = load_recipe_file_with_dir(recipe_name)?;
    let recipe_file_content = &recipe_file.content;
    let recipe_parameters = validate_recipe_parameters(recipe_file_content, &recipe_dir_str)?;

    let (params_for_template, missing_params) = apply_values_to_parameters(
        &params,
        recipe_parameters,
        &recipe_dir_str,
        None::<fn(&str, &str) -> Result<String>>,
    )?;
    let recipe = render_recipe_for_preview(
        recipe_file_content,
        recipe_dir_str.to_string(),
        &params_for_template,
    )?;
    print_recipe_explanation(&recipe);
    print_required_parameters_for_template(params_for_template, missing_params);

    Ok(())
}

fn validate_json_schema(schema: &serde_json::Value) -> Result<()> {
    match jsonschema::validator_for(schema) {
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow::anyhow!("JSON schema validation failed: {}", err)),
    }
}

#[cfg(test)]
mod tests;
