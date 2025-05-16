use anyhow::{anyhow, Result};
use goose::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

use super::github_recipe::{retrieve_recipe_from_github, GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY};

pub fn retrieve_recipe_file(recipe_name: &str) -> Result<(String, PathBuf)> {
    // If recipe_name ends with yaml or json, treat it as a direct path
    if recipe_name.ends_with(".yaml") || recipe_name.ends_with(".json") {
        let path = PathBuf::from(recipe_name);
        return read_recipe_file(path);
    }

    // First check current directory
    let current_dir = std::env::current_dir()?;
    if let Ok((content, recipe_parent_dir)) = read_recipe_in_dir(&current_dir, recipe_name) {
        return Ok((content, recipe_parent_dir));
    }
    read_recipe_in_dir(&current_dir, recipe_name).or_else(|e| {
        if let Some(recipe_repo_full_name) = configured_github_recipe_repo() {
            retrieve_recipe_from_github(recipe_name, &recipe_repo_full_name)
        } else {
            Err(e)
        }
    })
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

fn read_recipe_in_dir(dir: &Path, recipe_name: &str) -> Result<(String, PathBuf)> {
    for ext in &["yaml", "json"] {
        let recipe_path = dir.join(format!("{}.{}", recipe_name, ext));
        match read_recipe_file(recipe_path) {
            Ok((content, recipe_parent_dir)) => return Ok((content, recipe_parent_dir)),
            Err(_) => continue,
        }
    }
    Err(anyhow!(format!(
        "No {}.yaml or {}.json recipe file found in current directory.",
        recipe_name, recipe_name
    )))
}
