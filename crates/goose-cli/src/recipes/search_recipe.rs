use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::github_recipe::retrieve_recipe_from_github;

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
pub fn retrieve_recipe_file(recipe_name: &str) -> Result<String> {
    // If recipe_name ends with yaml or json, treat it as a direct path
    if recipe_name.ends_with(".yaml") || recipe_name.ends_with(".json") {
        let path = PathBuf::from(recipe_name);
        return read_recipe_file(path);
    }

    // First check current directory
    let current_dir = std::env::current_dir()?;
    match read_recipe_in_dir(&current_dir, recipe_name) {
        Ok(content_with_file_extension) => return Ok(content_with_file_extension),
        Err(e) => {
            if !is_block_internal()? {
                return Err(e);
            }
        }
    }
    let recipe_repo_full_name = "squareup/goose-recipes";
    // Try to retrieve from GitHub as a fallback
    retrieve_recipe_from_github(recipe_name, recipe_repo_full_name)
}

fn read_recipe_file<P: AsRef<Path>>(recipe_path: P) -> Result<String> {
    let path = recipe_path.as_ref();

    if path.exists() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read recipe file: {}", path.display()))?;
        Ok(content)
    } else {
        Err(anyhow!("Recipe file not found: {}", path.display()))
    }
}

fn read_recipe_in_dir(dir: &Path, recipe_name: &str) -> Result<String> {
    for ext in &["yaml", "json"] {
        let recipe_path = dir.join(format!("{}.{}", recipe_name, ext));
        match read_recipe_file(recipe_path) {
            Ok(content) => return Ok(content),
            Err(_) => continue,
        }
    }
    Err(anyhow!(
        "No recipe.yaml or recipe.json file found in current directory."
    ))
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
