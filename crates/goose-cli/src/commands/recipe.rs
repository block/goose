use anyhow::Result;
use base64::Engine;
use console::style;
use std::path::Path;

use crate::recipe::load_recipe;

/// Validates a recipe file
///
/// # Arguments
///
/// * `file_path` - Path to the recipe file to validate
///
/// # Returns
///
/// Result indicating success or failure
pub fn handle_validate<P: AsRef<Path>>(file_path: P) -> Result<()> {
    // Load and validate the recipe file
    match load_recipe(&file_path, false) {
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
pub fn handle_deeplink<P: AsRef<Path>>(file_path: P) -> Result<()> {
    // Load the recipe file first to validate it
    match load_recipe(&file_path, false) {
        Ok(recipe) => {
            if let Ok(recipe_json) = serde_json::to_string(&recipe) {
                let deeplink = base64::engine::general_purpose::STANDARD.encode(recipe_json);
                println!(
                    "{} Generated deeplink for: {}",
                    style("✓").green().bold(),
                    recipe.title
                );
                println!("goose://recipe?config={}", deeplink);
            }
            Ok(())
        }
        Err(err) => {
            println!("{} {}", style("✗").red().bold(), err);
            Err(err)
        }
    }
}
