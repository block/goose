use anyhow::Result;
use goose::config::Config;
use goose::recipe::read_recipe_file_content::RecipeFile;
use goose::recipe::RECIPE_FILE_EXTENSIONS;

use super::github_recipe::{
    list_github_recipes, retrieve_recipe_from_github, retrieve_recipe_from_github_with_byte_limit,
    BoundedRecipeLoad, RecipeInfo, RecipeSource, GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY,
};
use goose::recipe::local_recipes::{
    list_local_recipes, load_local_recipe_file,
    load_local_recipe_file_with_byte_limit_and_consumption,
};

pub fn load_recipe_file(recipe_name: &str) -> Result<RecipeFile> {
    load_local_recipe_file(recipe_name).or_else(|e| {
        if let Some(recipe_repo_full_name) = configured_github_recipe_repo() {
            retrieve_recipe_from_github(recipe_name, &recipe_repo_full_name)
        } else {
            Err(e)
        }
    })
}

pub(crate) fn load_recipe_file_with_byte_limit(
    recipe_name: &str,
    max_bytes: usize,
) -> BoundedRecipeLoad {
    let github_repo = configured_github_recipe_repo();
    load_recipe_file_with_byte_limit_from_repo(recipe_name, max_bytes, github_repo.as_deref())
}

pub(crate) fn load_recipe_file_with_byte_limit_from_repo(
    recipe_name: &str,
    max_bytes: usize,
    github_repo: Option<&str>,
) -> BoundedRecipeLoad {
    let has_recipe_extension = RECIPE_FILE_EXTENSIONS
        .iter()
        .any(|extension| recipe_name.ends_with(&format!(".{extension}")));
    let local_result =
        load_local_recipe_file_with_byte_limit_and_consumption(recipe_name, max_bytes);
    let local_consumed_bytes = local_result.consumed_bytes;
    match local_result.recipe_file {
        Ok(recipe_file) => BoundedRecipeLoad {
            consumed_bytes: Some(local_consumed_bytes),
            recipe_file: Ok(recipe_file),
        },
        Err(error) if has_recipe_extension => BoundedRecipeLoad {
            recipe_file: Err(error),
            consumed_bytes: Some(local_consumed_bytes),
        },
        Err(error) => match github_repo {
            Some(recipe_repo_full_name) if local_consumed_bytes < max_bytes => {
                let remote_load = retrieve_recipe_from_github_with_byte_limit(
                    recipe_name,
                    recipe_repo_full_name,
                    max_bytes - local_consumed_bytes,
                );
                BoundedRecipeLoad {
                    recipe_file: remote_load.recipe_file,
                    consumed_bytes: remote_load
                        .consumed_bytes
                        .and_then(|remote_bytes| local_consumed_bytes.checked_add(remote_bytes)),
                }
            }
            Some(_) => BoundedRecipeLoad {
                recipe_file: Err(error),
                consumed_bytes: Some(local_consumed_bytes),
            },
            None => BoundedRecipeLoad {
                recipe_file: Err(error),
                consumed_bytes: Some(local_consumed_bytes),
            },
        },
    }
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
                path: path.display().to_string(),
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
