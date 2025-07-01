use std::path::PathBuf;

use anyhow::Result;
use goose::recipe::{Response, SubRecipe};

use crate::{cli::InputConfig, recipes::recipe::load_recipe_as_template, session::SessionSettings};

#[allow(clippy::type_complexity)]
pub fn extract_recipe_info_from_cli(
    recipe_name: String,
    params: Vec<(String, String)>,
    additional_sub_recipes: Vec<String>,
) -> Result<(
    InputConfig,
    Option<SessionSettings>,
    Option<Vec<SubRecipe>>,
    Option<Response>,
)> {
    let recipe = load_recipe_as_template(&recipe_name, params).unwrap_or_else(|err| {
        eprintln!("{}: {}", console::style("Error").red().bold(), err);
        std::process::exit(1);
    });
    let mut all_sub_recipes = recipe.sub_recipes.clone().unwrap_or_default();
    if !additional_sub_recipes.is_empty() {
        additional_sub_recipes.iter().for_each(|sub_recipe_path| {
            let path = convert_path(sub_recipe_path);
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let additional_sub_recipe: SubRecipe = SubRecipe {
                path: path.to_string_lossy().to_string(),
                name,
                values: None,
            };
            all_sub_recipes.push(additional_sub_recipe);
        });
    }
    Ok((
        InputConfig {
            contents: recipe.prompt,
            extensions_override: recipe.extensions,
            additional_system_prompt: recipe.instructions,
        },
        recipe.settings.map(|s| SessionSettings {
            goose_provider: s.goose_provider,
            goose_model: s.goose_model,
            temperature: s.temperature,
        }),
        Some(all_sub_recipes),
        recipe.response,
    ))
}

fn convert_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home_dir) = dirs::home_dir() {
            return home_dir.join(stripped);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_extract_recipe_info_from_cli_basic() {
        let (_temp_dir, recipe_path) = create_recipe();
        let params = vec![("name".to_string(), "my_value".to_string())];
        let recipe_name = recipe_path.to_str().unwrap().to_string();

        let (input_config, settings, sub_recipes, response) =
            extract_recipe_info_from_cli(recipe_name, params, Vec::new()).unwrap();

        assert_eq!(input_config.contents, Some("test_prompt".to_string()));
        assert_eq!(
            input_config.additional_system_prompt,
            Some("test_instructions my_value".to_string())
        );
        assert!(input_config.extensions_override.is_none());

        assert!(settings.is_some());
        let settings = settings.unwrap();
        assert_eq!(settings.goose_provider, Some("test_provider".to_string()));
        assert_eq!(settings.goose_model, Some("test_model".to_string()));
        assert_eq!(settings.temperature, Some(0.7));

        assert!(sub_recipes.is_some());
        let sub_recipes = sub_recipes.unwrap();
        assert!(sub_recipes.len() == 1);
        assert_eq!(sub_recipes[0].path, "existing_sub_recipe.yaml".to_string());
        assert_eq!(sub_recipes[0].name, "existing_sub_recipe".to_string());
        assert!(sub_recipes[0].values.is_none());
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(
            response.json_schema,
            Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"}
                }
            }))
        );
    }

    #[test]
    fn test_extract_recipe_info_from_cli_with_additional_sub_recipes() {
        let (_temp_dir, recipe_path) = create_recipe();
        let params = vec![("name".to_string(), "my_value".to_string())];
        let recipe_name = recipe_path.to_str().unwrap().to_string();
        let additional_sub_recipes = vec![
            "path/to/sub_recipe1.yaml".to_string(),
            "another/sub_recipe2.yaml".to_string(),
        ];

        let (input_config, settings, sub_recipes, response) =
            extract_recipe_info_from_cli(recipe_name, params, additional_sub_recipes).unwrap();

        assert_eq!(input_config.contents, Some("test_prompt".to_string()));
        assert_eq!(
            input_config.additional_system_prompt,
            Some("test_instructions my_value".to_string())
        );
        assert!(input_config.extensions_override.is_none());

        assert!(settings.is_some());
        let settings = settings.unwrap();
        assert_eq!(settings.goose_provider, Some("test_provider".to_string()));
        assert_eq!(settings.goose_model, Some("test_model".to_string()));
        assert_eq!(settings.temperature, Some(0.7));

        assert!(sub_recipes.is_some());
        let sub_recipes = sub_recipes.unwrap();
        assert!(sub_recipes.len() == 3);
        assert_eq!(sub_recipes[0].path, "existing_sub_recipe.yaml".to_string());
        assert_eq!(sub_recipes[0].name, "existing_sub_recipe".to_string());
        assert!(sub_recipes[0].values.is_none());
        assert_eq!(sub_recipes[1].path, "path/to/sub_recipe1.yaml".to_string());
        assert_eq!(sub_recipes[1].name, "sub_recipe1".to_string());
        assert!(sub_recipes[1].values.is_none());
        assert_eq!(sub_recipes[2].path, "another/sub_recipe2.yaml".to_string());
        assert_eq!(sub_recipes[2].name, "sub_recipe2".to_string());
        assert!(sub_recipes[2].values.is_none());
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(
            response.json_schema,
            Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"}
                }
            }))
        );
    }

    fn create_recipe() -> (TempDir, PathBuf) {
        let test_recipe_content = r#"
title: test_recipe
description: A test recipe
instructions: test_instructions {{name}}
prompt: test_prompt
parameters:
- key: name
  description: name
  input_type: string
  requirement: required
settings:
  goose_provider: test_provider
  goose_model: test_model
  temperature: 0.7
sub_recipes:
- path: existing_sub_recipe.yaml
  name: existing_sub_recipe        
response:
  json_schema:
    type: object
    properties:
      result:
        type: string
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path: std::path::PathBuf = temp_dir.path().join("test_recipe.yaml");

        std::fs::write(&recipe_path, test_recipe_content).unwrap();
        (temp_dir, recipe_path)
    }
}
