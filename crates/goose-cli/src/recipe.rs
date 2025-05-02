use anyhow::{Context, Result};
use console::style;
use goose::recipe::Recipe;
use minijinja::UndefinedBehavior;
use std::{collections::HashMap, path::Path};

/// Loads and validates a recipe from a YAML or JSON file
///
/// # Arguments
///
/// * `path` - Path to the recipe file (YAML or JSON)
/// * `log`  - whether to log information about the recipe or not
/// * `params` - optional parameters to render the recipe with
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
pub fn load_recipe<P: AsRef<Path>>(
    path: P,
    log: bool,
    params: Option<Vec<(String, String)>>,
) -> Result<Recipe> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!("recipe file not found: {}", path.display()));
    }
    // Read file content
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read recipe file: {}", path.display()))?;
    // Check if any parameters were provided
    let rendered_content = match params {
        None => content,
        Some(params) => render_content_with_params(&content, &params)?,
    };

    // Determine file format based on extension and parse accordingly
    let recipe: Recipe = if let Some(extension) = path.extension() {
        match extension.to_str().unwrap_or("").to_lowercase().as_str() {
            "json" => serde_json::from_str(&rendered_content)
                .with_context(|| format!("Failed to parse JSON recipe file: {}", path.display()))?,
            "yaml" => serde_yaml::from_str(&rendered_content)
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

fn render_content_with_params(content: &str, params: &[(String, String)]) -> Result<String> {
    // Turn params into HashMap
    let param_map: HashMap<String, String> = params.iter().cloned().collect();

    // Create a minijinja environment and context
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let template = env.template_from_str(content)
        .map_err(|_| anyhow::anyhow!("Failed to render recipe, please check if the recipe has proper syntax for variables: eg: {{ variable_name }}"))?;

    // Render the template with the parameters
    template.render(param_map).map_err(|_| {
        anyhow::anyhow!(
            "Failed to render the recipe - please check if all required parameters are provided"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_content_with_params() {
        // Test basic parameter substitution
        let content = "Hello {{ name }}!";
        let params = vec![("name".to_string(), "World".to_string())];
        let result = render_content_with_params(content, &params).unwrap();
        assert_eq!(result, "Hello World!");

        // Test multiple parameters
        let content = "{{ greeting }} {{ name }}!";
        let params = vec![
            ("greeting".to_string(), "Hi".to_string()),
            ("name".to_string(), "Alice".to_string()),
        ];
        let result = render_content_with_params(content, &params).unwrap();
        assert_eq!(result, "Hi Alice!");

        // Test missing parameter results in error
        let content = "Hello {{ missing }}!";
        let params = vec![];
        let err = render_content_with_params(content, &params).unwrap_err();
        assert!(err
            .to_string()
            .contains("please check if all required parameters"));

        // Test invalid template syntax results in error
        let content = "Hello {{ unclosed";
        let params = vec![];
        let err = render_content_with_params(content, &params).unwrap_err();
        assert!(err
            .to_string()
            .contains("please check if the recipe has proper syntax"));
    }
}
