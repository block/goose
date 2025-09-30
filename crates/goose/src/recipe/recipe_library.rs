use crate::config::APP_STRATEGY;
use crate::recipe::Recipe;
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use serde_yaml;
use std::fs;
use std::path::PathBuf;

pub fn get_recipe_library_dir(is_global: bool) -> PathBuf {
    if is_global {
        choose_app_strategy(APP_STRATEGY.clone())
            .expect("goose requires a home dir")
            .config_dir()
            .join("recipes")
    } else {
        std::env::current_dir().unwrap().join(".goose/recipes")
    }
}

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
) -> Result<PathBuf> {
    let is_global_value = is_global.unwrap_or(true);
    // TODO: Lifei
    // check whether there is any existing file with same name, if return bad request
    // check whether there is any existing recipe has same title?, if yes, return bad request
    let default_file_path =
        get_recipe_library_dir(is_global_value).join(generate_recipe_filename(&recipe.title));
    let file_path_value = file_path.unwrap_or(default_file_path);
    let yaml_content = serde_yaml::to_string(&recipe)?;
    fs::write(&file_path_value, yaml_content)?;
    Ok(file_path_value)
}
