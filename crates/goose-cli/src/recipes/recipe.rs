use crate::recipes::print_recipe::{
    missing_parameters_command_line, print_recipe_explanation,
    print_required_parameters_for_template,
};
use crate::recipes::search_recipe::retrieve_recipe_file;
use crate::recipes::secret_discovery::{discover_recipe_secrets, SecretRequirement};
use anyhow::Result;
use goose::config::Config;
use goose::recipe::build_recipe::{
    apply_values_to_parameters, build_recipe_from_template, validate_recipe_parameters, RecipeError,
};
use goose::recipe::read_recipe_file_content::RecipeFile;
use goose::recipe::template_recipe::render_recipe_for_preview;
use goose::recipe::Recipe;
use serde_json::Value;
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
    match build_recipe_from_template(recipe_file, params, Some(create_user_prompt_callback())) {
        Ok(recipe) => {
            let secret_requirements = discover_recipe_secrets(&recipe);
            if let Err(e) = collect_missing_secrets(&secret_requirements) {
                eprintln!(
                    "Warning: Failed to collect some secrets: {}. Recipe will continue to run.",
                    e
                );
            }
            Ok(recipe)
        }
        Err(RecipeError::MissingParams { parameters }) => Err(anyhow::anyhow!(
            "Please provide the following parameters in the command line: {}",
            missing_parameters_command_line(parameters)
        )),
        Err(e) => Err(anyhow::anyhow!(e.to_string())),
    }
}

/// Collects missing secrets from the user interactively
///
/// This function checks if each required secret exists in the keyring.
/// For missing secrets, it prompts the user interactively and stores them
/// using the scoped key to prevent collisions.
///
/// # Arguments
/// * `requirements` - Vector of SecretRequirement objects to collect
///
/// # Returns
/// Result indicating success or failure of the collection process
pub fn collect_missing_secrets(requirements: &[SecretRequirement]) -> Result<()> {
    if requirements.is_empty() {
        return Ok(());
    }

    let config = Config::global();
    let mut missing_secrets = Vec::new();

    for req in requirements {
        match config.get_secret::<String>(&req.key) {
            Ok(_) => continue, // Secret exists
            Err(_) => missing_secrets.push(req),
        }
    }

    if missing_secrets.is_empty() {
        return Ok(());
    }

    println!(
        "🔐 This recipe uses {} secret(s) that are not yet configured (press ESC to skip any that are optional):",
        missing_secrets.len()
    );

    for req in &missing_secrets {
        println!("\n📋 Extension: {}", req.extension_name);
        println!("🔑 Secret: {}", req.key);

        let value = cliclack::password(format!(
            "Enter {} ({}) - press ESC to skip",
            req.key,
            req.description()
        ))
        .mask('▪')
        .interact()
        .unwrap_or_else(|_| String::new());

        if !value.trim().is_empty() {
            config.set_secret(&req.key, Value::String(value))?;
            println!("✅ Secret stored securely for {}", req.extension_name);
        } else {
            println!("⏭️  Skipped {} for {}", req.key, req.extension_name);
        }
    }

    if !missing_secrets.is_empty() {
        println!("\n🎉 Secret collection complete! Recipe execution will now continue.");
    }

    Ok(())
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
mod tests {
    use goose::recipe::{RecipeParameterInputType, RecipeParameterRequirement};

    use crate::recipes::recipe::load_recipe;

    mod load_recipe {
        use super::*;
        #[test]
        fn test_load_recipe_success() {
            let recipe_content = r#"{
                "version": "1.0.0",
                "title": "Test Recipe",
                "description": "A test recipe",
                "instructions": "Test instructions with {{ my_name }}",
                "parameters": [
                    {
                        "key": "my_name",
                        "input_type": "string",
                        "requirement": "required",
                        "description": "A test parameter"
                    }
                ]
            }"#;
            let temp_dir = tempfile::tempdir().unwrap();
            let recipe_path = temp_dir.path().join("test_recipe.json");
            std::fs::write(&recipe_path, recipe_content).unwrap();

            let params = vec![("my_name".to_string(), "value".to_string())];
            let recipe = load_recipe(recipe_path.to_str().unwrap(), params).unwrap();

            assert_eq!(recipe.title, "Test Recipe");
            assert_eq!(recipe.description, "A test recipe");
            assert_eq!(recipe.instructions.unwrap(), "Test instructions with value");
            // Verify parameters match recipe definition
            assert_eq!(recipe.parameters.as_ref().unwrap().len(), 1);
            let param = &recipe.parameters.as_ref().unwrap()[0];
            assert_eq!(param.key, "my_name");
            assert!(matches!(param.input_type, RecipeParameterInputType::String));
            assert!(matches!(
                param.requirement,
                RecipeParameterRequirement::Required
            ));
            assert_eq!(param.description, "A test parameter");
        }
    }
}
