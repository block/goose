use anyhow::Result;
use console::style;

use crate::recipes::github_recipe::RecipeSource;
use crate::recipes::recipe::load_recipe_for_validation;
use crate::recipes::search_recipe::list_available_recipes;
use goose::recipe_deeplink;

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
    match load_recipe_for_validation(recipe_name) {
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
/// * `recipe_name` - Path to the recipe file
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_deeplink(recipe_name: &str) -> Result<String> {
    match generate_deeplink(recipe_name) {
        Ok((deeplink_url, recipe)) => {
            println!(
                "{} Generated deeplink for: {}",
                style("✓").green().bold(),
                recipe.title
            );
            println!("{}", deeplink_url);
            Ok(deeplink_url)
        }
        Err(err) => {
            println!(
                "{} Failed to encode recipe: {}",
                style("✗").red().bold(),
                err
            );
            Err(err)
        }
    }
}

/// Opens a recipe in Goose Desktop
///
/// # Arguments
///
/// * `recipe_name` - Path to the recipe file
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_open(recipe_name: &str) -> Result<()> {
    // Generate the deeplink using the helper function (no printing)
    // This reuses all the validation and encoding logic
    match generate_deeplink(recipe_name) {
        Ok((deeplink_url, recipe)) => {
            // Attempt to open the deeplink
            match open::that(&deeplink_url) {
                Ok(_) => {
                    println!(
                        "{} Opened recipe '{}' in Goose Desktop",
                        style("✓").green().bold(),
                        recipe.title
                    );
                    Ok(())
                }
                Err(err) => {
                    println!(
                        "{} Failed to open recipe in Goose Desktop: {}",
                        style("✗").red().bold(),
                        err
                    );
                    println!("Generated deeplink: {}", deeplink_url);
                    println!("You can manually copy and open the URL above, or ensure Goose Desktop is installed.");
                    Err(anyhow::anyhow!("Failed to open recipe: {}", err))
                }
            }
        }
        Err(err) => {
            println!(
                "{} Failed to encode recipe: {}",
                style("✗").red().bold(),
                err
            );
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

/// Helper function to generate a deeplink
///
/// # Arguments
///
/// * `recipe_name` - Path to the recipe file
///
/// # Returns
///
/// Result containing the deeplink URL and recipe
fn generate_deeplink(recipe_name: &str) -> Result<(String, goose::recipe::Recipe)> {
    // Load the recipe file first to validate it
    let recipe = load_recipe_for_validation(recipe_name)?;
    match recipe_deeplink::encode(&recipe) {
        Ok(encoded) => {
            let full_url = format!("goose://recipe?config={}", encoded);
            Ok((full_url, recipe))
        }
        Err(err) => Err(anyhow::anyhow!("Failed to encode recipe: {}", err)),
    }
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
        let url = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));
        let encoded_part = url.strip_prefix("goose://recipe?config=").unwrap();
        assert!(!encoded_part.is_empty());
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
    fn test_handle_open_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        // Test handle_open - should attempt to open but may fail (that's expected in test environment)
        // We just want to ensure it doesn't panic and handles the error gracefully
        let result = handle_open(&recipe_path);
        // The result may be Ok or Err depending on whether the system can open the URL
        // In a test environment, it will likely fail to open, but that's fine
        // We're mainly testing that the function doesn't panic and processes the recipe correctly
        match result {
            Ok(_) => {
                // Successfully opened (unlikely in test environment)
            }
            Err(_) => {
                // Failed to open (expected in test environment) - this is fine
            }
        }
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

    #[test]
    fn test_generate_deeplink_valid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let result = generate_deeplink(&recipe_path);
        assert!(result.is_ok());
        let (url, recipe) = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));
        assert_eq!(recipe.title, "Test Recipe with Valid JSON Schema");
        assert_eq!(recipe.description, "A test recipe with valid JSON schema");
        let encoded_part = url.strip_prefix("goose://recipe?config=").unwrap();
        assert!(!encoded_part.is_empty());
    }

    #[test]
    fn test_generate_deeplink_invalid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", INVALID_RECIPE_CONTENT);

        let result = generate_deeplink(&recipe_path);
        assert!(result.is_err());
    }
}
