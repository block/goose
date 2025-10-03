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

pub fn list_recipes_from_library(is_global: bool) -> Result<Vec<(PathBuf, Recipe)>> {
    let path = get_recipe_library_dir(is_global);
    let mut recipes_with_path = Vec::new();
    if path.exists() {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            let extension = path.extension();

            if extension == Some("yaml".as_ref()) || extension == Some("json".as_ref()) {
                let Ok(recipe) = Recipe::from_file_path(&path) else {
                    continue;
                };
                recipes_with_path.push((path, recipe));
            }
        }
    }
    Ok(recipes_with_path)
}

pub fn list_all_recipes_from_library() -> Result<Vec<(PathBuf, Recipe)>> {
    let mut recipes_with_path = Vec::new();
    recipes_with_path.extend(list_recipes_from_library(true)?);
    recipes_with_path.extend(list_recipes_from_library(false)?);
    Ok(recipes_with_path)
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
    let all_recipes = list_all_recipes_from_library()?;

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
