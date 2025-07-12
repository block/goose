use anyhow::Result;
use base64::Engine;
use console::style;
use serde_json;

use crate::recipes::github_recipe::RecipeSource;
use crate::recipes::recipe::load_recipe;
use crate::recipes::search_recipe::list_available_recipes;

/// Validates a recipe file
///
/// # Arguments
///
/// * `file_path` - Path to the recipe file to validate
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_validate(recipe_name: &str) -> Result<()> {
    // Load and validate the recipe file
    match load_recipe(recipe_name) {
        Ok(_) => {
            println!("{} recipe file is valid", style("✓").green().bold());
            Ok(())
        }
        Err(err) => {
            println!("{} {}", style("✗").red().bold(), err);
            Err(err)
        }
    }
}

/// Generates a deeplink for a recipe file
///
/// # Arguments
///
/// * `file_path` - Path to the recipe file
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_deeplink(recipe_name: &str) -> Result<String> {
    // Load the recipe file first to validate it
    match load_recipe(recipe_name) {
        Ok(recipe) => {
            let mut full_url = String::new();
            if let Ok(recipe_json) = serde_json::to_string(&recipe) {
                let deeplink = base64::engine::general_purpose::STANDARD.encode(recipe_json);
                println!(
                    "{} Generated deeplink for: {}",
                    style("✓").green().bold(),
                    recipe.title
                );
                let url_safe = urlencoding::encode(&deeplink);
                full_url = format!("goose://recipe?config={}", url_safe);
                println!("{}", full_url);
            }
            Ok(full_url)
        }
        Err(err) => {
            println!("{} {}", style("✗").red().bold(), err);
            Err(err)
        }
    }
}

/// Lists all available recipes from local paths and GitHub repositories
///
/// # Arguments
///
/// * `format` - Output format ("text" or "json")
/// * `verbose` - Whether to show detailed information
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_list(format: &str, verbose: bool) -> Result<()> {
    let recipes = match list_available_recipes() {
        Ok(recipes) => recipes,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to list recipes: {}", e));
        }
    };

    match format {
        "json" => {
            println!("{}", serde_json::to_string(&recipes)?);
        }
        _ => {
            if recipes.is_empty() {
                println!("No recipes found");
                return Ok(());
            } else {
                println!("Available recipes:");
                for recipe in recipes {
                    let source_info = match recipe.source {
                        RecipeSource::Local => format!("local: {}", recipe.path),
                        RecipeSource::GitHub => format!("github: {}", recipe.path),
                    };

                    let description = if let Some(desc) = &recipe.description {
                        if desc.is_empty() {
                            "(none)"
                        } else {
                            desc
                        }
                    } else {
                        "(none)"
                    };

                    let output = format!("{} - {} - {}", recipe.name, description, source_info);
                    if verbose {
                        println!("  {}", output);
                        if let Some(title) = &recipe.title {
                            println!("    Title: {}", title);
                        }
                        println!("    Path: {}", recipe.path);
                    } else {
                        println!("{}", output);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_recipe_file(dir: &TempDir, filename: &str, content: &str) -> String {
        let file_path = dir.path().join(filename);
        fs::write(&file_path, content).expect("Failed to write test recipe file");
        file_path.to_string_lossy().into_owned()
    }

    const VALID_RECIPE_CONTENT: &str = r#"
title: "Test Recipe with Valid JSON Schema"
description: "A test recipe with valid JSON schema"
prompt: "Test prompt content"
instructions: "Test instructions"
response:
  json_schema:
    type: object
    properties:
      result:
        type: string
        description: "The result"
      count:
        type: number
        description: "A count value"
    required:
      - result
"#;

    const INVALID_RECIPE_CONTENT: &str = r#"
title: "Test Recipe"
description: "A test recipe for deeplink generation"
prompt: "Test prompt content {{ name }}"
instructions: "Test instructions"
"#;

    const RECIPE_WITH_INVALID_JSON_SCHEMA: &str = r#"
title: "Test Recipe with Invalid JSON Schema"
description: "A test recipe with invalid JSON schema"
prompt: "Test prompt content"
instructions: "Test instructions"
response:
  json_schema:
    type: invalid_type
    properties:
      result:
        type: unknown_type
    required: "should_be_array_not_string"
"#;

    #[test]
    fn test_handle_deeplink_valid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let result = handle_deeplink(&recipe_path);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("goose://recipe?config=eyJ2ZXJzaW9uIjoiMS4wLjAiLCJ0aXRsZSI6IlRlc3QgUmVjaXBlIHdpdGggVmFsaWQgSlNPTiBTY2hlbWEiLCJkZXNjcmlwdGlvbiI6IkEgdGVzdCByZWNpcGUgd2l0aCB2YWxpZCBKU09OIHNjaGVtYSIsImluc3RydWN0aW9ucyI6IlRlc3QgaW5zdHJ1Y3Rpb25zIiwicHJvbXB0IjoiVGVzdCBwcm9tcHQgY29udGVudCIsInJlc3BvbnNlIjp7Impzb25fc2NoZW1hIjp7InByb3BlcnRpZXMiOnsiY291bnQiOnsiZGVzY3JpcHRpb24iOiJBIGNvdW50IHZhbHVlIiwidHlwZSI6Im51bWJlciJ9LCJyZXN1bHQiOnsiZGVzY3JpcHRpb24iOiJUaGUgcmVzdWx0IiwidHlwZSI6InN0cmluZyJ9fSwicmVxdWlyZWQiOlsicmVzdWx0Il0sInR5cGUiOiJvYmplY3QifX19"));
    }

    #[test]
    fn test_handle_deeplink_invalid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", INVALID_RECIPE_CONTENT);
        let result = handle_deeplink(&recipe_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_validation_valid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let result = handle_validate(&recipe_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_validation_invalid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", INVALID_RECIPE_CONTENT);
        let result = handle_validate(&recipe_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_validation_recipe_with_invalid_json_schema() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path = create_test_recipe_file(
            &temp_dir,
            "test_recipe.yaml",
            RECIPE_WITH_INVALID_JSON_SCHEMA,
        );

        let result = handle_validate(&recipe_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("JSON schema validation failed"));
    }
}
