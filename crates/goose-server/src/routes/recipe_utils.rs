use std::fs;
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use anyhow::Result;

use goose::recipe::local_recipes::list_local_recipes;
use goose::recipe::Recipe;

pub struct RecipeManifestWithPath {
    pub id: String,
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

pub fn get_all_recipes_manifests() -> Result<Vec<RecipeManifestWithPath>> {
    let recipes_with_path = list_local_recipes()?;
    let mut recipe_manifests_with_path = Vec::new();
    for (file_path, recipe) in recipes_with_path {
        let Ok(last_modified) = fs::metadata(file_path.clone())
            .map(|m| chrono::DateTime::<chrono::Utc>::from(m.modified().unwrap()).to_rfc3339())
        else {
            continue;
        };

        let manifest_with_path = RecipeManifestWithPath {
            id: short_id_from_path(file_path.to_string_lossy().as_ref()),
            recipe,
            file_path,
            last_modified,
        };
        recipe_manifests_with_path.push(manifest_with_path);
    }
    recipe_manifests_with_path.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(recipe_manifests_with_path)
}
