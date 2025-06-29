use anyhow::{anyhow, Result};
use goose::config::Config;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::recipes::recipe::RECIPE_FILE_EXTENSIONS;

use super::github_recipe::{
    list_github_recipes, retrieve_recipe_from_github, RecipeInfo, RecipeSource,
    GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY,
};

const GOOSE_RECIPE_PATH_ENV_VAR: &str = "GOOSE_RECIPE_PATH";

pub fn retrieve_recipe_file(recipe_name: &str) -> Result<(String, PathBuf)> {
    // If recipe_name ends with yaml or json, treat it as a direct file path
    if RECIPE_FILE_EXTENSIONS
        .iter()
        .any(|ext| recipe_name.ends_with(&format!(".{}", ext)))
    {
        let path = PathBuf::from(recipe_name);
        return read_recipe_file(path);
    }
    retrieve_recipe_from_local_path(recipe_name).or_else(|e| {
        if let Some(recipe_repo_full_name) = configured_github_recipe_repo() {
            retrieve_recipe_from_github(recipe_name, &recipe_repo_full_name)
        } else {
            Err(e)
        }
    })
}

fn read_recipe_in_dir(dir: &Path, recipe_name: &str) -> Result<(String, PathBuf)> {
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

fn retrieve_recipe_from_local_path(recipe_name: &str) -> Result<(String, PathBuf)> {
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

fn read_recipe_file<P: AsRef<Path>>(recipe_path: P) -> Result<(String, PathBuf)> {
    let path = recipe_path.as_ref();

    let content = fs::read_to_string(path)
        .map_err(|e| anyhow!("Failed to read recipe file {}: {}", path.display(), e))?;

    let canonical = path.canonicalize().map_err(|e| {
        anyhow!(
            "Failed to resolve absolute path for {}: {}",
            path.display(),
            e
        )
    })?;

    let parent_dir = canonical
        .parent()
        .ok_or_else(|| anyhow!("Resolved path has no parent: {}", canonical.display()))?
        .to_path_buf();

    Ok((content, parent_dir))
}

/// Lists all available recipes from local paths and GitHub repositories
pub fn list_available_recipes() -> Result<Vec<RecipeInfo>> {
    let mut recipes = Vec::new();

    // Search local recipes
    if let Ok(local_recipes) = discover_local_recipes() {
        recipes.extend(local_recipes);
    }

    // Search GitHub recipes if configured
    if let Some(repo) = configured_github_recipe_repo() {
        if let Ok(github_recipes) = list_github_recipes(&repo) {
            recipes.extend(github_recipes);
        }
    }

    Ok(recipes)
}

fn discover_local_recipes() -> Result<Vec<RecipeInfo>> {
    let mut recipes = Vec::new();
    let mut search_dirs = vec![PathBuf::from(".")];

    // Add GOOSE_RECIPE_PATH directories
    if let Ok(recipe_path_env) = env::var(GOOSE_RECIPE_PATH_ENV_VAR) {
        let path_separator = if cfg!(windows) { ';' } else { ':' };
        let recipe_path_env_dirs: Vec<PathBuf> = recipe_path_env
            .split(path_separator)
            .map(PathBuf::from)
            .collect();
        search_dirs.extend(recipe_path_env_dirs);
    }

    for dir in search_dirs {
        if let Ok(dir_recipes) = scan_directory_for_recipes(&dir) {
            recipes.extend(dir_recipes);
        }
    }

    Ok(recipes)
}

fn scan_directory_for_recipes(dir: &Path) -> Result<Vec<RecipeInfo>> {
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
                    if let Ok(recipe_info) = create_local_recipe_info(&path) {
                        recipes.push(recipe_info);
                    }
                }
            }
        }
    }

    Ok(recipes)
}

fn create_local_recipe_info(path: &Path) -> Result<RecipeInfo> {
    let content = fs::read_to_string(path)?;
    let recipe_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_string_lossy()
        .to_string();
    let (recipe, _) = crate::recipes::template_recipe::parse_recipe_content(&content, recipe_dir)?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let path_str = path.to_string_lossy().to_string();

    Ok(RecipeInfo {
        name,
        source: RecipeSource::Local,
        path: path_str,
        title: Some(recipe.title),
        description: Some(recipe.description),
    })
}
