use anyhow::Result;
use console::style;

use crate::recipes::print_recipe::{
    missing_parameters_command_line, print_parameters_with_values, print_recipe_explanation,
    print_required_parameters_for_template,
};
use crate::recipes::search_recipe::retrieve_recipe_file;
use goose::recipe::{Recipe, RecipeParameter, RecipeParameterRequirement};
use minijinja::{Environment, Error, Template, UndefinedBehavior};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub const BUILT_IN_RECIPE_DIR_PARAM: &str = "recipe_dir";
pub const RECIPE_FILE_EXTENSIONS: &[&str] = &["yaml", "json"];
/// Loads, validates a recipe from a YAML or JSON file, and renders it with the given parameters
///
/// # Arguments
///
/// * `path` - Path to the recipe file (YAML or JSON)
/// * `params` - parameters to render the recipe with
///
/// # Returns
///
/// The rendered recipe if successful
///
/// # Errors
///
/// Returns an error if:
/// - Recipe is not valid
/// - The required fields are missing
pub fn load_recipe_as_template(recipe_name: &str, params: Vec<(String, String)>) -> Result<Recipe> {
    let (recipe_file_content, recipe_parent_dir) = retrieve_recipe_file(recipe_name)?;

    let recipe = validate_recipe_file_parameters(&recipe_file_content)?;

    let (params_for_template, missing_params) =
        apply_values_to_parameters(&params, recipe.parameters, recipe_parent_dir, true)?;
    if !missing_params.is_empty() {
        return Err(anyhow::anyhow!(
            "Please provide the following parameters in the command line: {}",
            missing_parameters_command_line(missing_params)
        ));
    }

    let rendered_content = render_content_with_params(&recipe_file_content, &params_for_template)?;

    let recipe = parse_recipe_content(&rendered_content)?;

    // Display information about the loaded recipe
    println!(
        "{} {}",
        style("Loading recipe:").green().bold(),
        style(&recipe.title).green()
    );
    println!("{} {}", style("Description:").bold(), &recipe.description);

    if !params_for_template.is_empty() {
        println!("{}", style("Parameters used to load this recipe:").bold());
        print_parameters_with_values(params_for_template);
    }
    println!();
    Ok(recipe)
}

/// Loads and validates a recipe from a YAML or JSON file
///
/// # Arguments
///
/// * `path` - Path to the recipe file (YAML or JSON)
/// * `params` - optional parameters to render the recipe with
///
/// # Returns
///
/// The parsed recipe struct if successful
///
/// # Errors
///
/// Returns an error if:
/// - The file doesn't exist
/// - The file can't be read
/// - The YAML/JSON is invalid
/// - The parameter definition does not match the template variables in the recipe file
pub fn load_recipe(recipe_name: &str) -> Result<Recipe> {
    let (recipe_file_content, _) = retrieve_recipe_file(recipe_name)?;

    validate_recipe_file_parameters(&recipe_file_content)
}

pub fn explain_recipe_with_parameters(
    recipe_name: &str,
    params: Vec<(String, String)>,
) -> Result<()> {
    let (recipe_file_content, recipe_parent_dir) = retrieve_recipe_file(recipe_name)?;

    let raw_recipe = validate_recipe_file_parameters(&recipe_file_content)?;
    print_recipe_explanation(&raw_recipe);
    let recipe_parameters = raw_recipe.parameters;
    let (params_for_template, missing_params) =
        apply_values_to_parameters(&params, recipe_parameters, recipe_parent_dir, false)?;
    print_required_parameters_for_template(params_for_template, missing_params);

    Ok(())
}

fn validate_recipe_file_parameters(recipe_file_content: &str) -> Result<Recipe> {
    let recipe_from_recipe_file: Recipe = parse_recipe_content(recipe_file_content)?;
    validate_optional_parameters(&recipe_from_recipe_file)?;
    validate_parameters_in_template(&recipe_from_recipe_file.parameters, recipe_file_content)?;
    Ok(recipe_from_recipe_file)
}

fn validate_parameters_in_template(
    recipe_parameters: &Option<Vec<RecipeParameter>>,
    recipe_file_content: &str,
) -> Result<()> {
    let mut template_variables = extract_template_variables(recipe_file_content)?;
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

fn validate_optional_parameters(recipe: &Recipe) -> Result<()> {
    let optional_params_without_default_values: Vec<String> = recipe
        .parameters
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

pub fn parse_recipe_content(content: &str) -> Result<Recipe> {
    if serde_json::from_str::<JsonValue>(content).is_ok() {
        Ok(serde_json::from_str(content)?)
    } else if serde_yaml::from_str::<YamlValue>(content).is_ok() {
        Ok(serde_yaml::from_str(content)?)
    } else {
        Err(anyhow::anyhow!(
            "Unsupported file format for recipe file. Expected .yaml or .json"
        ))
    }
}

fn extract_template_variables(template_str: &str) -> Result<HashSet<String>> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);

    let template = env
        .template_from_str(template_str)
        .map_err(|e: Error| anyhow::anyhow!("Invalid template syntax: {}", e.to_string()))?;

    Ok(template.undeclared_variables(true))
}

fn apply_values_to_parameters(
    user_params: &[(String, String)],
    recipe_parameters: Option<Vec<RecipeParameter>>,
    recipe_parent_dir: PathBuf,
    enable_user_prompt: bool,
) -> Result<(HashMap<String, String>, Vec<String>)> {
    let mut param_map: HashMap<String, String> = user_params.iter().cloned().collect();
    let recipe_parent_dir_str = recipe_parent_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in recipe_dir"))?;
    param_map.insert(
        BUILT_IN_RECIPE_DIR_PARAM.to_string(),
        recipe_parent_dir_str.to_string(),
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

fn render_content_with_params(content: &str, params: &HashMap<String, String>) -> Result<String> {
    // Create a minijinja environment and context
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let template: Template<'_, '_> = env
        .template_from_str(content)
        .map_err(|e: Error| anyhow::anyhow!("Invalid template syntax: {}", e.to_string()))?;

    // Render the template with the parameters
    template.render(params).map_err(|e: Error| {
        anyhow::anyhow!(
            "Failed to render the recipe {} - please check if all required parameters are provided",
            e.to_string()
        )
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use goose::recipe::{RecipeParameterInputType, RecipeParameterRequirement};
    use tempfile::TempDir;

    use super::*;

    fn setup_recipe_file(instructions_and_parameters: &str) -> (TempDir, PathBuf) {
        let recipe_content = format!(
            r#"{{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
            {}
        }}"#,
            instructions_and_parameters
        );
        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path: std::path::PathBuf = temp_dir.path().join("test_recipe.json");
        std::fs::write(&recipe_path, recipe_content).unwrap();
        (temp_dir, recipe_path)
    }

    #[test]
    fn test_render_content_with_params() {
        // Test basic parameter substitution
        let content = "Hello {{ name }}!";
        let mut params = HashMap::new();
        params.insert("name".to_string(), "World".to_string());
        let result = render_content_with_params(content, &params).unwrap();
        assert_eq!(result, "Hello World!");

        // Test empty parameter substitution
        let content = "Hello {{ empty }}!";
        let mut params = HashMap::new();
        params.insert("empty".to_string(), "".to_string());
        let result = render_content_with_params(content, &params).unwrap();
        assert_eq!(result, "Hello !");

        // Test multiple parameters
        let content = "{{ greeting }} {{ name }}!";
        let mut params = HashMap::new();
        params.insert("greeting".to_string(), "Hi".to_string());
        params.insert("name".to_string(), "Alice".to_string());
        let result = render_content_with_params(content, &params).unwrap();
        assert_eq!(result, "Hi Alice!");

        // Test missing parameter results in error
        let content = "Hello {{ missing }}!";
        let params = HashMap::new();
        let err = render_content_with_params(content, &params).unwrap_err();
        assert!(err
            .to_string()
            .contains("please check if all required parameters"));

        // Test invalid template syntax results in error
        let content = "Hello {{ unclosed";
        let params = HashMap::new();
        let err = render_content_with_params(content, &params).unwrap_err();
        assert!(err.to_string().contains("Invalid template syntax"));
    }

    #[test]
    fn test_load_recipe_as_template_success() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ my_name }}",
            "parameters": [
                {
                    "key": "my_name",
                    "input_type": "string",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ]"#;

        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

        let params = vec![("my_name".to_string(), "value".to_string())];
        let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), params).unwrap();

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

    #[test]
    fn test_load_recipe_as_template_success_variable_in_prompt() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions",
            "prompt": "My prompt {{ my_name }}",
            "parameters": [
                {
                    "key": "my_name",
                    "input_type": "string",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ]"#;

        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

        let params = vec![("my_name".to_string(), "value".to_string())];
        let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), params).unwrap();

        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.description, "A test recipe");
        assert_eq!(recipe.instructions.unwrap(), "Test instructions");
        assert_eq!(recipe.prompt.unwrap(), "My prompt value");
        let param = &recipe.parameters.as_ref().unwrap()[0];
        assert_eq!(param.key, "my_name");
        assert!(matches!(param.input_type, RecipeParameterInputType::String));
        assert!(matches!(
            param.requirement,
            RecipeParameterRequirement::Required
        ));
        assert_eq!(param.description, "A test parameter");
    }

    #[test]
    fn test_load_recipe_as_template_wrong_parameters_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ expected_param1 }} {{ expected_param2 }}",
            "parameters": [
                {
                    "key": "wrong_param_key",
                    "input_type": "string",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

        let load_recipe_result = load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new());
        assert!(load_recipe_result.is_err());
        let err = load_recipe_result.unwrap_err();
        println!("{}", err.to_string());
        assert!(err
            .to_string()
            .contains("Unnecessary parameter definitions: wrong_param_key."));
        assert!(err
            .to_string()
            .contains("Missing definitions for parameters in the recipe file:"));
        assert!(err.to_string().contains("expected_param1"));
        assert!(err.to_string().contains("expected_param2"));
    }

    #[test]
    fn test_load_recipe_as_template_with_default_values_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ param_with_default }} {{ param_without_default }}",
            "parameters": [
                {
                    "key": "param_with_default",
                    "input_type": "string",
                    "requirement": "optional",
                    "default": "my_default_value",
                    "description": "A test parameter"
                },
                {
                    "key": "param_without_default",
                    "input_type": "string",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let params = vec![("param_without_default".to_string(), "value1".to_string())];

        let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), params).unwrap();

        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.description, "A test recipe");
        assert_eq!(
            recipe.instructions.unwrap(),
            "Test instructions with my_default_value value1"
        );
    }

    #[test]
    fn test_load_recipe_as_template_optional_parameters_with_empty_default_values_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ optional_param }}",
            "parameters": [
                {
                    "key": "optional_param",
                    "input_type": "string",
                    "requirement": "optional",
                    "description": "A test parameter",
                    "default": "",
                }
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

        let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new()).unwrap();
        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.description, "A test recipe");
        assert_eq!(recipe.instructions.unwrap(), "Test instructions with ");
    }

    #[test]
    fn test_load_recipe_as_template_optional_parameters_without_default_values_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ optional_param }}",
            "parameters": [
                {
                    "key": "optional_param",
                    "input_type": "string",
                    "requirement": "optional",
                    "description": "A test parameter"
                }
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

        let load_recipe_result = load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new());
        assert!(load_recipe_result.is_err());
        let err = load_recipe_result.unwrap_err();
        println!("{}", err.to_string());
        assert!(err.to_string().contains(
            "Optional parameters missing default values in the recipe: optional_param. Please provide defaults."
        ));
    }

    #[test]
    fn test_load_recipe_as_template_wrong_input_type_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ param }}",
            "parameters": [
                {
                    "key": "param",
                    "input_type": "some_invalid_type",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ]"#;
        let params = vec![("param".to_string(), "value".to_string())];
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

        let load_recipe_result = load_recipe_as_template(recipe_path.to_str().unwrap(), params);
        assert!(load_recipe_result.is_err());
        let err = load_recipe_result.unwrap_err();
        assert!(err
            .to_string()
            .contains("unknown variant `some_invalid_type`"));
    }

    #[test]
    fn test_load_recipe_as_template_success_without_parameters() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions"
            "#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

        let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new()).unwrap();
        assert_eq!(recipe.instructions.unwrap(), "Test instructions");
        assert!(recipe.parameters.is_none());
    }
}
