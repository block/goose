use std::fs;
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};

use crate::recipe::recipe_manifest_metadata::RecipeManifestMetadata;
use goose::config::APP_STRATEGY;
use goose::recipe::read_recipe_file_content::read_recipe_file;
use goose::recipe::Recipe;

pub struct RecipeManifestWithPath {
    pub id: String,
    pub recipe_metadata: RecipeManifestMetadata,
    pub recipe: Recipe,
    pub file_path: PathBuf,
    pub last_modified: String,
}

fn short_id_from_path(path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let h = hasher.finish();
    format!("{:016x}", h)
}

fn load_recipes_from_path(path: &PathBuf) -> Result<Vec<RecipeManifestWithPath>> {
    let mut recipe_manifests_with_path = Vec::new();
    if path.exists() {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            if path.extension() == Some("yaml".as_ref()) {
                let Ok(recipe_file) = read_recipe_file(path.clone()) else {
                    continue;
                };
                let Ok(recipe) = Recipe::from_content(&recipe_file.content) else {
                    continue;
                };
                let Ok(recipe_metadata) = RecipeManifestMetadata::from_yaml_file(&path) else {
                    continue;
                };
                let Ok(last_modified) = fs::metadata(path.clone()).map(|m| {
                    chrono::DateTime::<chrono::Utc>::from(m.modified().unwrap()).to_rfc3339()
                }) else {
                    continue;
                };

                let manifest_with_path = RecipeManifestWithPath {
                    id: short_id_from_path(recipe_file.file_path.to_string_lossy().as_ref()),
                    recipe_metadata,
                    recipe,
                    file_path: recipe_file.file_path,
                    last_modified,
                };
                recipe_manifests_with_path.push(manifest_with_path);
            }
        }
    }
    Ok(recipe_manifests_with_path)
}

fn get_all_recipes_manifests() -> Result<Vec<RecipeManifestWithPath>> {
    let current_dir = std::env::current_dir()?;
    let local_recipe_path = current_dir.join(".goose/recipes");

    let global_recipe_path = choose_app_strategy(APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .config_dir()
        .join("recipes");

    let mut recipe_manifests_with_path = Vec::new();

    recipe_manifests_with_path.extend(load_recipes_from_path(&local_recipe_path)?);
    recipe_manifests_with_path.extend(load_recipes_from_path(&global_recipe_path)?);

    Ok(recipe_manifests_with_path)
}

pub fn list_sorted_recipe_manifests() -> Result<Vec<RecipeManifestWithPath>> {
    let mut recipe_manifests_with_path = get_all_recipes_manifests()?;
    recipe_manifests_with_path.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(recipe_manifests_with_path)
}
