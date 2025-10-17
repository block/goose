use anyhow::Result;
use goose::config::Config;
use goose::recipe::read_recipe_file_content::RecipeFile;

use super::github_recipe::{
    list_github_recipes, retrieve_recipe_from_github, RecipeInfo, RecipeSource,
    GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY,
};
use goose::recipe::local_recipes::{list_local_recipes, load_local_recipe_file};

pub fn load_recipe_file(recipe_name: &str) -> Result<RecipeFile> {
    load_local_recipe_file(recipe_name).or_else(|e| {
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

/// Lists all available recipes from local paths and GitHub repositories
pub fn list_available_recipes() -> Result<Vec<RecipeInfo>> {
    let mut recipes = Vec::new();

    // Search local recipes
    if let Ok(local_recipes) = list_local_recipes() {
        recipes.extend(local_recipes.into_iter().map(|(path, recipe)| {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            RecipeInfo {
                name,
                source: RecipeSource::Local,
                path: path.to_string_lossy().to_string(),
                title: Some(recipe.title),
                description: Some(recipe.description),
            }
        }));
    }

    // Search GitHub recipes if configured
    if let Some(repo) = configured_github_recipe_repo() {
        if let Ok(github_recipes) = list_github_recipes(&repo) {
            recipes.extend(github_recipes);
        }
    }

    Ok(recipes)
}
