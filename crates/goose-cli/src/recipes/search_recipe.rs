use anyhow::{anyhow, Result};
use goose::config::Config;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::recipes::recipe::RECIPE_FILE_EXTENSIONS;

use super::github_recipe::{retrieve_recipe_from_github, GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY};

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
            println!("{}", e);
            retrieve_recipe_from_github(recipe_name, &recipe_repo_full_name)
        } else {
            Err(e)
        }
    })
}

fn retrieve_recipe_from_local_path(recipe_name: &str) -> Result<(String, PathBuf)> {
    println!(
        "📦 Looking for recipe \"{}\" using paths from environment variable: {}",
        recipe_name, GOOSE_RECIPE_PATH_ENV_VAR
    );
    let recipe_path_env = match env::var(GOOSE_RECIPE_PATH_ENV_VAR) {
        Ok(val) => val,
        Err(_) => {
            return Err(anyhow!(
                "ℹ️  Environment variable {} is not set. You can set it to search for the recipe in the paths you specify in the environment variable. eg: GOOSE_RECIPE_PATH=/path/to/path1:/path/to/path2",
                GOOSE_RECIPE_PATH_ENV_VAR
            ));
        }
    };

    let path_separator = if cfg!(windows) { ';' } else { ':' };

    let search_dirs: Vec<PathBuf> = recipe_path_env
        .split(path_separator)
        .map(PathBuf::from)
        .collect();
    println!("🔍 GOOSE_RECIPE_PATH includes:");
    for path in &search_dirs {
        println!("  - {}", path.display());
    }

    for dir in &search_dirs {
        for ext in RECIPE_FILE_EXTENSIONS {
            let candidate = dir.join(recipe_name).join(format!("recipe.{}", ext));
            if candidate.exists() && candidate.is_file() {
                println!("⬇️  Retrieve recipe from path: {}", candidate.display());
                return read_recipe_file(candidate);
            }
        }
    }
    Err(anyhow!("ℹ️  Failed to retrieve {}/recipe.yaml or {}/recipe.json in the paths from environment variable: {}", recipe_name, recipe_name, GOOSE_RECIPE_PATH_ENV_VAR))
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
