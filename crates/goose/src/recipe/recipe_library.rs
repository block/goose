use crate::recipe::search_local_recipes::{discover_local_recipes, get_recipe_library_dir};
use crate::recipe::Recipe;
use serde_yaml;
use std::fs;
use std::path::PathBuf;

fn generate_recipe_filename(title: &str) -> String {
    let base_name = title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("-");

    let filename = if base_name.is_empty() {
        "untitled-recipe".to_string()
    } else {
        base_name
    };
    format!("{}.yaml", filename)
}

pub fn save_recipe_to_file(
    recipe: Recipe,
    is_global: Option<bool>,
    file_path: Option<PathBuf>,
) -> anyhow::Result<PathBuf> {
    let is_global_value = is_global.unwrap_or(true);

    let default_file_path =
        get_recipe_library_dir(is_global_value).join(generate_recipe_filename(&recipe.title));

    let file_path_value = match file_path {
        Some(path) => path,
        None => {
            if default_file_path.exists() {
                return Err(anyhow::anyhow!(
                    "Recipe file already exists at: {:?}",
                    default_file_path
                ));
            }
            default_file_path
        }
    };
    let all_recipes = discover_local_recipes()?;

    for (existing_path, existing_recipe) in &all_recipes {
        if existing_recipe.title == recipe.title && existing_path != &file_path_value {
            return Err(anyhow::anyhow!(
                "Recipe with title '{}' already exists",
                recipe.title
            ));
        }
    }

    let yaml_content = serde_yaml::to_string(&recipe)?;
    fs::write(&file_path_value, yaml_content)?;
    Ok(file_path_value)
}
