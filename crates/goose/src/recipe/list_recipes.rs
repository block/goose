use std::fs;
use std::hash::DefaultHasher;
use std::path::PathBuf;
use std::hash::{Hash, Hasher};

use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};

use crate::config::APP_STRATEGY;
use crate::recipe::recipe_manifest::RecipeManifest;

pub struct RecipeManifestWithPath {
    pub id: String,
    pub manifest: RecipeManifest,
    pub file_path: PathBuf,
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
                let absolute_path = path.canonicalize()?;
                if let Ok(recipe_manifest) = RecipeManifest::from_yaml_file(&path) {
                    let manifest_with_path = RecipeManifestWithPath {
                        id: short_id_from_path(&absolute_path.to_string_lossy()),
                        manifest: recipe_manifest,
                        file_path: absolute_path,
                    };
                    recipe_manifests_with_path.push(manifest_with_path);
                }
            }
        }
    }
    Ok(recipe_manifests_with_path)
}

fn get_all_recipes_manifests() -> Result<Vec<RecipeManifestWithPath>> {
    let current_dir = std::env::current_dir()?;
    let local_recipe_path = current_dir.join(".goose/recipes");

    let global_recipe_path = choose_app_strategy(APP_STRATEGY.clone())
        .map(|strategy| strategy.in_config_dir("recipes"))
        .unwrap_or_else(|_| PathBuf::from("~/.config/goose/recipes"));

    let mut recipe_manifests_with_path = Vec::new();

    recipe_manifests_with_path.extend(load_recipes_from_path(&local_recipe_path)?);
    recipe_manifests_with_path.extend(load_recipes_from_path(&global_recipe_path)?);

    Ok(recipe_manifests_with_path)
}

pub fn list_sorted_recipe_manifests() -> Result<Vec<RecipeManifestWithPath>> {
    let mut recipe_manifests_with_path = get_all_recipes_manifests()?;
    recipe_manifests_with_path
        .sort_by(|a, b| b.manifest.last_modified.cmp(&a.manifest.last_modified));
    Ok(recipe_manifests_with_path)
}
