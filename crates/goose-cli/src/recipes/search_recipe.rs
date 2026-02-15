use anyhow::Result;
use goose::config::Config;
use goose::recipe::read_recipe_file_content::RecipeFile;
use goose::registry::manifest::RegistryEntryKind;
use goose::registry::sources::local::LocalRegistrySource;
use goose::registry::RegistryManager;

use super::github_recipe::{
    list_github_recipes, retrieve_recipe_from_github, RecipeInfo, RecipeSource,
    GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY,
};
use goose::recipe::local_recipes::{list_local_recipes, load_local_recipe_file};

pub fn load_recipe_file(recipe_name: &str) -> Result<RecipeFile> {
    load_local_recipe_file(recipe_name).or_else(|local_err| {
        if let Some(recipe_repo_full_name) = configured_github_recipe_repo() {
            retrieve_recipe_from_github(recipe_name, &recipe_repo_full_name)
        } else {
            Err(local_err)
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

/// Lists all available recipes from local paths, GitHub repositories, and the registry
pub fn list_available_recipes() -> Result<Vec<RecipeInfo>> {
    let mut recipes = Vec::new();

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
                path: path.display().to_string(),
                title: Some(recipe.title),
                description: Some(recipe.description),
            }
        }));
    }

    if let Some(repo) = configured_github_recipe_repo() {
        if let Ok(github_recipes) = list_github_recipes(&repo) {
            recipes.extend(github_recipes);
        }
    }

    if let Ok(registry_recipes) = list_registry_recipes() {
        let seen: std::collections::HashSet<String> =
            recipes.iter().map(|r| r.name.clone()).collect();
        recipes.extend(
            registry_recipes
                .into_iter()
                .filter(|r| !seen.contains(&r.name)),
        );
    }

    Ok(recipes)
}

fn list_registry_recipes() -> Result<Vec<RecipeInfo>> {
    let mut manager = RegistryManager::default();
    if let Ok(local) = LocalRegistrySource::from_default_paths() {
        manager.add_source(Box::new(local));
    }

    let entries = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { manager.search(None, Some(RegistryEntryKind::Recipe)).await })
    })?;

    Ok(entries
        .into_iter()
        .map(|entry| {
            let path = entry
                .local_path
                .as_ref()
                .map(|p| p.display().to_string())
                .or_else(|| entry.source_uri.clone())
                .unwrap_or_default();
            let title = Some(entry.name.clone());
            let description = if entry.description.is_empty() {
                None
            } else {
                Some(entry.description)
            };
            RecipeInfo {
                name: entry.name,
                source: RecipeSource::Registry,
                path,
                title,
                description,
            }
        })
        .collect())
}
