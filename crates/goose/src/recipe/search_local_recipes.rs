use anyhow::{anyhow, Result};
use etcetera::{choose_app_strategy, AppStrategy};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::APP_STRATEGY;
use crate::recipe::read_recipe_file_content::{read_recipe_file, RecipeFile};
use crate::recipe::Recipe;
use crate::recipe::RECIPE_FILE_EXTENSIONS;

const GOOSE_RECIPE_PATH_ENV_VAR: &str = "GOOSE_RECIPE_PATH";

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

fn local_recipe_dirs() -> Vec<PathBuf> {
    let mut local_dirs = vec![PathBuf::from(".")];

    if let Ok(recipe_path_env) = env::var(GOOSE_RECIPE_PATH_ENV_VAR) {
        let path_separator = if cfg!(windows) { ';' } else { ':' };
        local_dirs.extend(recipe_path_env.split(path_separator).map(PathBuf::from));
    }
    local_dirs.push(get_recipe_library_dir(true));
    local_dirs.push(get_recipe_library_dir(false));

    local_dirs
}

pub fn retrieve_local_recipe_file(recipe_name: &str) -> Result<RecipeFile> {
    if RECIPE_FILE_EXTENSIONS
        .iter()
        .any(|ext| recipe_name.ends_with(&format!(".{}", ext)))
    {
        let path = PathBuf::from(recipe_name);
        return read_recipe_file(path);
    }

    if is_file_path(recipe_name) || is_file_name(recipe_name) {
        return Err(anyhow!(
            "Recipe file {} is not a json or yaml file",
            recipe_name
        ));
    }

    let search_dirs = local_recipe_dirs();
    for dir in &search_dirs {
        if let Ok(result) = read_recipe_in_dir(dir, recipe_name) {
            return Ok(result);
        }
    }

    let search_dirs_str = search_dirs
        .iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join(":");
    Err(anyhow!(
        "ℹ️  Failed to retrieve {}.yaml or {}.json in {}",
        recipe_name,
        recipe_name,
        search_dirs_str
    ))
}

pub fn list_local_recipes() -> Result<Vec<(PathBuf, Recipe)>> {
    let mut recipes = Vec::new();
    for dir in local_recipe_dirs() {
        if let Ok(dir_recipes) = scan_directory_for_recipes(&dir) {
            recipes.extend(dir_recipes);
        }
    }

    Ok(recipes)
}

fn is_file_path(recipe_name: &str) -> bool {
    recipe_name.contains('/')
        || recipe_name.contains('\\')
        || recipe_name.starts_with('~')
        || recipe_name.starts_with('.')
}

fn is_file_name(recipe_name: &str) -> bool {
    Path::new(recipe_name).extension().is_some()
}

fn read_recipe_in_dir(dir: &Path, recipe_name: &str) -> Result<RecipeFile> {
    for ext in RECIPE_FILE_EXTENSIONS {
        let recipe_path = dir.join(format!("{}.{}", recipe_name, ext));
        if let Ok(result) = read_recipe_file(recipe_path) {
            return Ok(result);
        }
    }
    Err(anyhow!(format!(
        "No {}.yaml or {}.json recipe file found in directory: {}",
        recipe_name,
        recipe_name,
        dir.display()
    )))
}

fn scan_directory_for_recipes(dir: &Path) -> Result<Vec<(PathBuf, Recipe)>> {
    let mut recipes = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Ok(recipes);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if RECIPE_FILE_EXTENSIONS.contains(&extension.to_string_lossy().as_ref()) {
                    if let Ok(recipe) = Recipe::from_file_path(&path) {
                        recipes.push((path.clone(), recipe));
                    }
                }
            }
        }
    }

    Ok(recipes)
}
