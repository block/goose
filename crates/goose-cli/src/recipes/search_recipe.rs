use anyhow::{anyhow, Result};
use goose::config::Config;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::recipes::recipe::RECIPE_FILE_EXTENSIONS;

use super::github_recipe::{retrieve_recipe_from_github, GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY};

const GOOSE_RECIPE_PATH_ENV_VAR: &str = "GOOSE_RECIPE_PATH";

pub struct RecipeFile {
    pub content: String,
    pub parent_dir: PathBuf,
    pub file_path: PathBuf,
}

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

fn convert_path_with_tilde_expansion(path: &Path) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Some(home_dir) = dirs::home_dir() {
                return home_dir.join(stripped);
            }
        }
    }
    PathBuf::from(path)
}

fn read_recipe_file<P: AsRef<Path>>(recipe_path: P) -> Result<RecipeFile> {
    let raw_path = recipe_path.as_ref();
    let path = convert_path_with_tilde_expansion(raw_path);

    let content = fs::read_to_string(&path)
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

    Ok(RecipeFile {
        content,
        parent_dir,
        file_path: canonical,
    })
}
