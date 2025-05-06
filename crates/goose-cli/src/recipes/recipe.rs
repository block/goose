use anyhow::Result;
use console::style;

use goose::recipe::{Recipe, RecipeParameter};
use minijinja::{Environment, Template, UndefinedBehavior};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::{HashMap, HashSet};

use crate::recipes::search_recipe::retrieve_recipe_file;

/// Loads and validates a recipe from a YAML or JSON file
///
/// # Arguments
///
/// * `path` - Path to the recipe file (YAML or JSON)
/// * `log`  - whether to log information about the recipe or not
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
/// - The required fields are missing
pub fn load_recipe(
    recipe_name: &str,
    log: bool,
    params: Option<Vec<(String, String)>>,
) -> Result<Recipe> {
    let content = retrieve_recipe_file(recipe_name)?;
    let recipe_from_recipe_file: Recipe = parse_recipe_content(&content)?;

    let recipe_parameters: &Vec<RecipeParameter> =
        validate_recipe_file_parameters(&recipe_from_recipe_file)?;

    let rendered_content = match params {
        None => content,
        Some(user_params) => {
            let user_params_with_defaults = apply_default_values(&user_params, recipe_parameters);
            render_content_with_params(&content, &user_params_with_defaults)?
        }
    };

    let recipe = parse_recipe_content(&rendered_content)?;
    if log {
        // Display information about the loaded recipe
        println!(
            "{} {}",
            style("Loading recipe:").green().bold(),
            style(&recipe.title).green()
        );
        println!("{} {}", style("Description:").dim(), &recipe.description);

        println!(); // Add a blank line for spacing
    }

    Ok(recipe)
}

fn validate_recipe_file_parameters(recipe: &Recipe) -> Result<&Vec<RecipeParameter>> {
    let template_variables = extract_template_variables(&recipe.instructions.as_ref().unwrap())?;
    let param_keys: HashSet<String> = recipe
        .parameters
        .as_ref()
        .unwrap()
        .iter()
        .map(|p| p.key.clone())
        .collect();

    let missing_keys: HashSet<_> = template_variables.difference(&param_keys).collect();
    let extra_keys: HashSet<_> = param_keys.difference(&template_variables).collect();
    if missing_keys.is_empty() && extra_keys.is_empty() {
        return Ok(recipe.parameters.as_ref().unwrap());
    }
    let mut message = String::new();
    if !missing_keys.is_empty() {
        message.push_str(&format!(
            "Missing definitions for parameters: {:?}\n",
            missing_keys
        ));
    }
    if !extra_keys.is_empty() {
        message.push_str(&format!(
            "Unexpected parameter definitions: {:?}\n",
            extra_keys
        ));
    }
    Err(anyhow::anyhow!("{}", message.trim_end()))
}

fn parse_recipe_content(content: &str) -> Result<Recipe> {
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
        .map_err(|_| anyhow::anyhow!("Invalid template syntax"))?;

    Ok(template.undeclared_variables(true))
}

fn apply_default_values(
    user_params: &[(String, String)],
    recipe_parameters: &Vec<RecipeParameter>,
) -> HashMap<String, String> {
    let mut param_map: HashMap<String, String> = user_params.iter().cloned().collect();
    for param in recipe_parameters {
        if let (false, Some(default)) = (param_map.contains_key(&param.key), &param.default) {
            param_map.insert(param.key.clone(), default.clone());
        }
    }
    param_map
}

fn render_content_with_params(content: &str, params: &HashMap<String, String>) -> Result<String> {
    // Create a minijinja environment and context
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let template: Template<'_, '_> = env.template_from_str(content)
        .map_err(|_| anyhow::anyhow!("Failed to render recipe, please check if the recipe has proper syntax for variables: eg: {{ variable_name }}"))?;

    // Render the template with the parameters
    template.render(params).map_err(|_| {
        anyhow::anyhow!(
            "Failed to render the recipe - please check if all required parameters are provided"
        )
    })
}

#[cfg(test)]
mod tests {
    use goose::recipe::RecipeParameterRequirement;

    use super::*;

    #[test]
    fn test_render_content_with_params() {
        // Test basic parameter substitution
        let content = "Hello {{ name }}!";
        let mut params = HashMap::new();
        params.insert("name".to_string(), "World".to_string());
        let result = render_content_with_params(content, &params).unwrap();
        assert_eq!(result, "Hello World!");

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
        assert!(err
            .to_string()
            .contains("please check if the recipe has proper syntax"));
    }

    #[test]
    fn test_load_recipe_success() {
        // Mock retrieve_recipe_file by creating a test recipe file
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

        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path = temp_dir.path().join("test_recipe.json");
        std::fs::write(&recipe_path, recipe_content).unwrap();

        // Test loading recipe with parameters
        let params = vec![("my_name".to_string(), "value".to_string())];
        let recipe = load_recipe(recipe_path.to_str().unwrap(), false, Some(params)).unwrap();

        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.description, "A test recipe");
        assert_eq!(recipe.instructions.unwrap(), "Test instructions with value");
        // Verify parameters match recipe definition
        assert_eq!(recipe.parameters.as_ref().unwrap().len(), 1);
        let param = &recipe.parameters.as_ref().unwrap()[0];
        assert_eq!(param.key, "my_name");
        assert_eq!(param.input_type, "string");
        assert!(matches!(
            param.requirement,
            RecipeParameterRequirement::Required
        ));
        assert_eq!(param.description, "A test parameter");
    }

    #[test]
    fn test_load_recipe_wrong_parameters() {
        // Mock retrieve_recipe_file by creating a test recipe file
        let recipe_content = r#"{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
            "instructions": "Test instructions with {{ expected_param1 }} {{ expected_param2 }}",
            "parameters": [
                {
                    "key": "wrong_param_key",
                    "input_type": "string",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ]
        }"#;

        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path = temp_dir.path().join("test_recipe.json");
        std::fs::write(&recipe_path, recipe_content).unwrap();

        // Test loading recipe with parameters
        let params = vec![("my_name".to_string(), "value".to_string())];
        let load_recipe_result = load_recipe(recipe_path.to_str().unwrap(), false, Some(params));
        assert!(load_recipe_result.is_err());
        let err = load_recipe_result.unwrap_err();
        println!("{}", err.to_string());
        assert!(err
            .to_string()
            .contains("Unexpected parameter definitions: {\"wrong_param_key\"}"));
        assert!(err
            .to_string()
            .contains("Missing definitions for parameters:"));
        assert!(err.to_string().contains("expected_param1"));
        assert!(err.to_string().contains("expected_param2"));
    }

    #[test]
    fn test_load_recipe_with_default_values() {
        // Mock retrieve_recipe_file by creating a test recipe file
        let recipe_content = r#"{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
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
            ]
        }"#;

        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path = temp_dir.path().join("test_recipe.json");
        std::fs::write(&recipe_path, recipe_content).unwrap();

        // Test loading recipe with parameters
        let params = vec![("param_without_default".to_string(), "value1".to_string())];
        let recipe = load_recipe(recipe_path.to_str().unwrap(), false, Some(params)).unwrap();

        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.description, "A test recipe");
        assert_eq!(
            recipe.instructions.unwrap(),
            "Test instructions with my_default_value value1"
        );
    }
}
