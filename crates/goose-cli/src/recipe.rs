use anyhow::{Context, Result};
use console::style;
use std::path::Path;

use goose::recipe::Recipe;

/// Loads and validates a recipe from a YAML or JSON file
///
/// # Arguments
///
/// * `path` - Path to the recipe file (YAML or JSON)
/// * `log`  - whether to log information about the recipe or not
///
/// # Returns
///
/// The parsed recipe struct if successful
///
/// # Errors
///
/// Returns an error if:
/// - The file doesn't exist
/// - The file can't be read
/// - The YAML/JSON is invalid
/// - The required fields are missing
pub fn load_recipe<P: AsRef<Path>>(path: P, log: bool) -> Result<Recipe> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!("recipe file not found: {}", path.display()));
    }
    // Read file content
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read recipe file: {}", path.display()))?;

    // Determine file format based on extension and parse accordingly
    let recipe: Recipe = if let Some(extension) = path.extension() {
        match extension.to_str().unwrap_or("").to_lowercase().as_str() {
            "json" => serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON recipe file: {}", path.display()))?,
            "yaml" => serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML recipe file: {}", path.display()))?,
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported file format for recipe file: {}. Expected .yaml or .json",
                    path.display()
                ))
            }
        }
    } else {
        return Err(anyhow::anyhow!(
            "File has no extension: {}. Expected .yaml or .json",
            path.display()
        ));
    };

    if log {
        // Display information about the loaded recipe
        println!(
            "{} {}",
            style("Loading recipe:").green().bold(),
            style(&recipe.title).green()
        );
        println!("{} {}", style("Description:").dim(), &recipe.description);

        println!(); // Add a blank line for spacing
    }

    Ok(recipe)
}
