use anyhow::Result;
use base64::Engine;
use console::style;
use serde_json;

use crate::recipes::recipe::load_recipe;
use crate::recipes::search_recipe::list_available_recipes;
use crate::recipes::github_recipe::RecipeSource;

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
