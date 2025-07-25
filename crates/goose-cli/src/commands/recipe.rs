use anyhow::Result;
use arboard::Clipboard;
use console::style;

use crate::recipes::github_recipe::RecipeSource;
use crate::recipes::recipe::load_recipe_for_validation;
use crate::recipes::search_recipe::list_available_recipes;
use goose::recipe_deeplink;

/// Trait for clipboard operations to enable mocking in tests
pub trait ClipboardProvider {
    fn set_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>>;
}

/// Real clipboard implementation
pub struct RealClipboard {
    clipboard: Clipboard,
}

impl RealClipboard {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(RealClipboard {
            clipboard: Clipboard::new()?,
        })
    }
}

impl ClipboardProvider for RealClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.clipboard.set_text(text)?;
        Ok(())
    }
}

/// Mock clipboard for testing
#[cfg(test)]
pub struct MockClipboard {
    pub should_fail: bool,
    pub last_text: std::rc::Rc<std::cell::RefCell<Option<String>>>,
}

#[cfg(test)]
impl MockClipboard {
    pub fn new() -> Self {
        MockClipboard {
            should_fail: false,
            last_text: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }

    pub fn new_failing() -> Self {
        MockClipboard {
            should_fail: true,
            last_text: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }

    pub fn get_last_text(&self) -> Option<String> {
        self.last_text.borrow().clone()
    }
}

#[cfg(test)]
impl ClipboardProvider for MockClipboard {
    fn set_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.should_fail {
            Err("Mock clipboard failure".into())
        } else {
            *self.last_text.borrow_mut() = Some(text.to_string());
            Ok(())
        }
    }
}

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
/// * `file_path` - Path to the recipe file
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_deeplink(recipe_name: &str) -> Result<String> {
    handle_deeplink_with_clipboard(recipe_name, RealClipboard::new)
}

/// Generates a deeplink for a recipe file with a custom clipboard provider
///
/// # Arguments
///
/// * `recipe_name` - Path to the recipe file
/// * `clipboard_factory` - Function that creates a clipboard provider
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_deeplink_with_clipboard<F, C>(
    recipe_name: &str,
    clipboard_factory: F,
) -> Result<String>
where
    F: FnOnce() -> Result<C, Box<dyn std::error::Error>>,
    C: ClipboardProvider,
{
    // Load the recipe file first to validate it
    match load_recipe_for_validation(recipe_name) {
        Ok(recipe) => match recipe_deeplink::encode(&recipe) {
            Ok(encoded) => {
                println!(
                    "{} Generated deeplink for: {}",
                    style("✓").green().bold(),
                    recipe.title
                );
                let full_url = format!("goose://recipe?config={}", encoded);
                println!("{}", full_url);

                // Copy to clipboard
                match clipboard_factory() {
                    Ok(mut clipboard) => match clipboard.set_text(&full_url) {
                        Ok(_) => {
                            println!("{} Deeplink copied to clipboard", style("📋").cyan().bold());
                        }
                        Err(e) => {
                            println!(
                                "{} Failed to copy to clipboard: {}",
                                style("⚠").yellow().bold(),
                                e
                            );
                        }
                    },
                    Err(e) => {
                        println!(
                            "{} Failed to access clipboard: {}",
                            style("⚠").yellow().bold(),
                            e
                        );
                    }
                }

                Ok(full_url)
            }
            Err(err) => {
                println!(
                    "{} Failed to encode recipe: {}",
                    style("✗").red().bold(),
                    err
                );
                Err(anyhow::anyhow!("Failed to encode recipe: {}", err))
            }
        },
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
        let url = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));
        let encoded_part = url.strip_prefix("goose://recipe?config=").unwrap();
        assert!(encoded_part.len() > 0);
    }

    #[test]
    fn test_handle_deeplink_clipboard_success() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let mock_clipboard = MockClipboard::new();
        let clipboard_state = mock_clipboard.last_text.clone();

        let result = handle_deeplink_with_clipboard(&recipe_path, || Ok(mock_clipboard));

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));

        // Now we can check the clipboard state!
        assert_eq!(clipboard_state.borrow().as_ref(), Some(&url));
    }

    #[test]
    fn test_handle_deeplink_clipboard_failure() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let mock_clipboard = MockClipboard::new_failing();
        let clipboard_state = mock_clipboard.last_text.clone();

        let result = handle_deeplink_with_clipboard(&recipe_path, || Ok(mock_clipboard));

        // Should still succeed even if clipboard fails
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));

        // Clipboard should remain empty since the operation failed
        assert_eq!(clipboard_state.borrow().as_ref(), None);
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
