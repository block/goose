use crate::recipes::print_recipe::{
    missing_parameters_command_line, print_parameters_with_values, print_recipe_explanation,
    print_required_parameters_for_template,
};
use crate::recipes::search_recipe::retrieve_recipe_file;
use anyhow::Result;
use console::style;
use goose::recipe::build_recipe::{
    apply_values_to_parameters, build_recipe_from_template, validate_recipe_parameters, render_recipe_template
};
use goose::recipe::read_recipe_file_content::RecipeFile;
use goose::recipe::template_recipe::{render_recipe_for_preview};
use goose::recipe::Recipe;
use std::collections::HashMap;

pub const BUILT_IN_RECIPE_DIR_PARAM: &str = "recipe_dir";
pub const RECIPE_FILE_EXTENSIONS: &[&str] = &["yaml", "json"];

fn create_user_prompt_callback() -> impl Fn(&str, &str) -> Result<String> {
    |key: &str, description: &str| -> Result<String> {
        let input_value = cliclack::input(format!("Please enter {} ({})", key, description))
            .interact()?;
        Ok(input_value)
    }
}

pub fn render_recipe_template_content(
    recipe_name: &str,
    params: Vec<(String, String)>,
) -> Result<String> {
    let recipe_file = retrieve_recipe_file(recipe_name)?;
    let (rendered_content, missing_params) = render_recipe_template(recipe_file, params, Some(create_user_prompt_callback()))?;
    
    if !missing_params.is_empty() {
        return Err(anyhow::anyhow!(
            "Please provide the following parameters in the command line: {}",
            missing_parameters_command_line(missing_params)
        ));
    }
    
    Ok(rendered_content)
}

pub fn load_and_render_recipe(recipe_name: &str, params: Vec<(String, String)>) -> Result<Recipe> {
    let rendered_content = render_recipe_template_content(&recipe_name, params.clone())?;
    
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

pub fn load_recipe_for_validation(recipe_name: &str) -> Result<Recipe> {
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

    let (params_for_template, missing_params) = apply_values_to_parameters(
        &params,
        recipe_parameters,
        recipe_dir_str,
        None::<fn(&str, &str) -> Result<String>>,
    )?;
    let recipe = render_recipe_for_preview(
        &recipe_file_content,
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
