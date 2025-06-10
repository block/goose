use anyhow::Result;
use base64::Engine;
use console::style;

use crate::recipes::recipe::load_recipe;

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
pub fn handle_deeplink(recipe_name: &str) -> Result<()> {
    // Load the recipe file first to validate it
    match load_recipe(recipe_name) {
        Ok(recipe) => {
            if let Ok(recipe_json) = serde_json::to_string(&recipe) {
                let deeplink = base64::engine::general_purpose::STANDARD.encode(recipe_json);
                println!(
                    "{} Generated deeplink for: {}",
                    style("✓").green().bold(),
                    recipe.title
                );
                let url_safe = urlencoding::encode(&deeplink);
                println!("goose://recipe?config={}", url_safe);
            }
            Ok(())
        }
        Err(err) => {
            println!("{} {}", style("✗").red().bold(), err);
            Err(err)
        }
    }
}
