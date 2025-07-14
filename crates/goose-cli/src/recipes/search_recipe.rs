use anyhow::{anyhow, Result};
use goose::config::Config;
use goose::recipe::read_recipe_file_content::{read_recipe_file, RecipeFile};
use std::env;
use std::path::{Path, PathBuf};

use crate::recipes::recipe::RECIPE_FILE_EXTENSIONS;

use super::github_recipe::{retrieve_recipe_from_github, GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY};

const GOOSE_RECIPE_PATH_ENV_VAR: &str = "GOOSE_RECIPE_PATH";

pub fn retrieve_recipe_file(recipe_name: &str) -> Result<RecipeFile> {
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
    retrieve_recipe_from_local_path(recipe_name).or_else(|e| {
        if let Some(recipe_repo_full_name) = configured_github_recipe_repo() {
            retrieve_recipe_from_github(recipe_name, &recipe_repo_full_name)
        } else {
            Err(e)
        }
    })
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

fn retrieve_recipe_from_local_path(recipe_name: &str) -> Result<RecipeFile> {
    let mut search_dirs = vec![PathBuf::from(".")];
    if let Ok(recipe_path_env) = env::var(GOOSE_RECIPE_PATH_ENV_VAR) {
        let path_separator = if cfg!(windows) { ';' } else { ':' };
        let recipe_path_env_dirs: Vec<PathBuf> = recipe_path_env
            .split(path_separator)
            .map(PathBuf::from)
            .collect();
        search_dirs.extend(recipe_path_env_dirs);
    }
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

fn configured_github_recipe_repo() -> Option<String> {
    let config = Config::global();
    match config.get_param(GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY) {
        Ok(Some(recipe_repo_full_name)) => Some(recipe_repo_full_name),
        _ => None,
    }
}
