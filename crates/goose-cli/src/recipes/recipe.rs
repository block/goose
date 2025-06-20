use crate::recipes::print_recipe::{
    missing_parameters_command_line, print_parameters_with_values, print_recipe_explanation,
    print_required_parameters_for_template,
};
use crate::recipes::search_recipe::retrieve_recipe_file;
use anyhow::Result;
use console::style;
use goose::recipe::{Recipe, RecipeParameter, RecipeParameterRequirement};
use minijinja::{Environment, Error, UndefinedBehavior};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const BUILT_IN_RECIPE_DIR_PARAM: &str = "recipe_dir";
pub const RECIPE_FILE_EXTENSIONS: &[&str] = &["yaml", "json"];

pub fn load_recipe_content_as_template(
    recipe_name: &str,
    params: Vec<(String, String)>,
) -> Result<String> {
    let (recipe_file_content, recipe_parent_dir) = retrieve_recipe_file(recipe_name)?;
    let recipe_parameters = extract_parameters_from_content(&recipe_file_content)?;

    validate_optional_parameters(&recipe_parameters)?;
    validate_parameters_in_template(&recipe_parameters, &recipe_file_content)?;

    let (params_for_template, missing_params) =
        apply_values_to_parameters(&params, recipe_parameters, recipe_parent_dir, true)?;

    if !missing_params.is_empty() {
        return Err(anyhow::anyhow!(
            "Please provide the following parameters in the command line: {}",
            missing_parameters_command_line(missing_params)
        ));
    }

    render_content_with_params(&recipe_file_content, &params_for_template)
}

pub fn load_recipe_as_template(recipe_name: &str, params: Vec<(String, String)>) -> Result<Recipe> {
    let rendered_content = load_recipe_content_as_template(recipe_name, params.clone())?;
    let recipe = parse_recipe_content(&rendered_content)?;

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

fn extract_parameters_from_content(content: &str) -> Result<Option<Vec<RecipeParameter>>> {
    let lines = content.lines();
    let mut params_block = String::new();
    let mut collecting = false;

    for line in lines {
        if line.starts_with("parameters:") {
            collecting = true;
        }
        if collecting {
            if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                let parameters: Vec<RecipeParameter> = serde_yaml::from_str(&params_block)
                    .map_err(|e| anyhow::anyhow!("Failed to parse parameters block: {}", e))?;
                return Ok(Some(parameters));
            }
            params_block.push_str(line);
            params_block.push('\n');
        }
    }

    // If we didn't find a parameter block it might be because it is defined in json style or some such:
    if serde_yaml::from_str::<serde_yaml::Value>(content).is_err() {
        return Ok(None);
    }

    let recipe: Recipe = serde_yaml::from_str(content)
        .map_err(|e| anyhow::anyhow!("Valid YAML but invalid Recipe structure: {}", e))?;
    Ok(recipe.parameters)
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

fn parse_recipe_content(content: &str) -> Result<Recipe> {
    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("Recipe content is empty"));
    }

    serde_yaml::from_str(content)
        .map_err(|e| anyhow::anyhow!("Failed to parse recipe content: {}", e))
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
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);

    if let Some(recipe_dir) = params.get("recipe_dir") {
        let recipe_dir = recipe_dir.clone();
        env.set_loader(move |name| {
            let path = Path::new(&recipe_dir).join(name);
            match std::fs::read_to_string(&path) {
                Ok(content) => Ok(Some(content)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    "could not read template",
                )
                .with_source(e)),
            }
        });
    }

    let template = env
        .template_from_str(content)
        .map_err(|e| anyhow::anyhow!("Invalid template syntax: {}", e))?;

    template
        .render(params)
        .map_err(|e| anyhow::anyhow!("Failed to render the recipe {}", e))
}

fn validate_recipe_file_parameters(recipe_file_content: &str) -> Result<Recipe> {
    let recipe_from_recipe_file: Recipe = parse_recipe_content(recipe_file_content)?;
    let parameters = extract_parameters_from_content(recipe_file_content)?;
    validate_optional_parameters(&parameters)?;
    validate_parameters_in_template(&parameters, recipe_file_content)?;
    Ok(recipe_from_recipe_file)
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
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path: std::path::PathBuf = temp_dir.path().join("test_recipe.yaml");

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
        let error_msg = err.to_string();
        assert!(error_msg.contains("Failed to render the recipe"));

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
        assert!(err.to_string().to_lowercase().contains("missing"));
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
        let err_msg = err.to_string();
        eprint!("Error: {}", err_msg);
        assert!(err_msg.contains("unknown variant `some_invalid_type`"));
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

    #[test]
    fn test_template_inheritance() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path();

        let parent_content = [
            "version: 1.0.0",
            "title: Parent",
            "description: Parent recipe",
            "prompt: |",
            "  {% block prompt -%}",
            "  What is the capital of France?",
            "  {%- endblock -%}",
        ]
        .join("\n");

        let parent_path = temp_path.join("parent.yaml");
        std::fs::write(&parent_path, parent_content).unwrap();

        let child_content = [
            "{% extends \"parent.yaml\" -%}",
            "{%- block prompt -%}",
            "  What is the capital of Germany?",
            "{%- endblock -%}",
        ]
        .join("\n");

        let child_path = temp_path.join("child.yaml");
        std::fs::write(&child_path, child_content).unwrap();

        let params = vec![];
        let result = load_recipe_as_template(child_path.to_str().unwrap(), params);

        assert!(result.is_ok());
        let recipe = result.unwrap();

        assert_eq!(recipe.title, "Parent");
        assert_eq!(recipe.description, "Parent recipe");
        assert_eq!(
            recipe.prompt.unwrap().trim(),
            "What is the capital of Germany?"
        );
    }
}
