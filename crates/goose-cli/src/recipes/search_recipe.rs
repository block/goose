use anyhow::{anyhow, Result};
use dirs::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

use super::github_recipe::{download_github_recipe, GOOSE_RECIPE_GITHUB_HTTP_URL};

// use crate::recipes::github_recipe::download_github_recipe;

/// Searches for a recipe file locally and on GitHub if not found
///
/// # Arguments
///
/// * `recipe_name` - Name of the recipe to search for (without extension) or the full path to the recipe file
///
/// # Returns
///
/// The path to the recipe file if found
pub fn find_recipe_file(recipe_name: &str) -> Result<PathBuf> {
    // If recipe_name ends with yaml or json, treat it as a direct path
    if recipe_name.ends_with(".yaml") || recipe_name.ends_with(".json") {
        let path = PathBuf::from(recipe_name);
        if path.exists() {
            return Ok(path);
        } else {
            return Err(anyhow!("Recipe file not found: {}", path.display()));
        }
    }
    // First check current directory
    let current_dir = std::env::current_dir()?;
    if let Some(path) = check_recipe_in_dir(&current_dir, recipe_name) {
        return Ok(path);
    }
    // Get home directory, return error if not found
    let home = home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    // Check ~/.goose/recipes directory
    let recipes_dir = home.join(".goose").join("recipes");

    // Create recipes directory if it doesn't exist
    if !recipes_dir.exists() {
        fs::create_dir_all(&recipes_dir)?;
    }
    if let Some(path) = check_recipe_in_dir(&recipes_dir, recipe_name) {
        return Ok(path);
    }

    let error_message = format!(
        "No {}.yaml, or {}.json file found in current directory, {} directory",
        recipe_name,
        recipe_name,
        recipes_dir.display()
    );
    if !is_block_internal()? {
        return Err(anyhow!(error_message.clone()));
    }
    // Try to download from GitHub as a fallback
    match download_github_recipe(recipe_name, &recipes_dir) {
        Ok(download_path) => Ok(download_path),
        Err(_) => {
            let github_directory = format!("{}/{}", GOOSE_RECIPE_GITHUB_HTTP_URL, recipe_name);
            Err(anyhow!(
                "{}\n  No recipe.yaml or recipe.json file found in github directory {}",
                error_message,
                github_directory
            ))
        }
    }
}

/// Checks if a recipe exists in the given directory with either yaml or json extension
fn check_recipe_in_dir(dir: &Path, recipe_name: &str) -> Option<PathBuf> {
    for ext in &["yaml", "json"] {
        let recipe_path = dir.join(format!("{}.{}", recipe_name, ext));
        if recipe_path.exists() {
            return Some(recipe_path);
        }
    }
    None
}

fn is_block_internal() -> Result<bool> {
    if let Ok(host) = std::env::var("DATABRICKS_HOST") {
        if host.contains("block-lakehouse-production") {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_recipe_in_dir() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test recipe files
        fs::write(dir_path.join("test_recipe.yaml"), "test yaml content").unwrap();
        fs::write(dir_path.join("test_recipe2.json"), "test json content").unwrap();

        // Test finding existing yaml recipe
        let result = check_recipe_in_dir(dir_path, "test_recipe");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), dir_path.join("test_recipe.yaml"));

        // Test finding existing json recipe
        let result = check_recipe_in_dir(dir_path, "test_recipe2");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), dir_path.join("test_recipe2.json"));

        // Test non-existent recipe returns None
        let result = check_recipe_in_dir(dir_path, "nonexistent");
        assert!(result.is_none());
    }
}
