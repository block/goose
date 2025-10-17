use std::collections::HashMap;
use std::fs;
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use anyhow::Result;
use axum::http::StatusCode;

use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use goose::recipe::local_recipes::list_local_recipes;
use goose::recipe::validate_recipe::validate_recipe_template_from_content;
use goose::recipe::Recipe;
use serde_json;
use tracing::error;

pub struct RecipeValidationError {
    pub status: StatusCode,
    pub message: String,
}

pub struct RecipeManifestWithPath {
    pub id: String,
    pub recipe: Recipe,
    pub file_path: PathBuf,
    pub last_modified: String,
}

pub fn short_id_from_path(path: &str) -> String {
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

pub fn validate_recipe(recipe: &Recipe) -> Result<(), RecipeValidationError> {
    let recipe_json = serde_json::to_string(recipe).map_err(|err| {
        let message = err.to_string();
        error!("Failed to serialize recipe for validation: {}", message);
        RecipeValidationError {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    })?;

    validate_recipe_template_from_content(&recipe_json, None).map_err(|err| {
        let message = err.to_string();
        error!("Recipe validation failed: {}", message);
        RecipeValidationError {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    })?;

    Ok(())
}

pub async fn get_recipe_file_path_by_id(
    state: &AppState,
    id: &str,
) -> Result<PathBuf, ErrorResponse> {
    let cached_path = {
        let map = state.recipe_file_hash_map.lock().await;
        map.get(id).cloned()
    };

    if let Some(path) = cached_path {
        return Ok(path);
    }

    let recipe_manifest_with_paths = get_all_recipes_manifests().unwrap_or_default();
    let mut recipe_file_hash_map = HashMap::new();
    let mut resolved_path: Option<PathBuf> = None;

    for recipe_manifest_with_path in &recipe_manifest_with_paths {
        if recipe_manifest_with_path.id == id {
            resolved_path = Some(recipe_manifest_with_path.file_path.clone());
        }
        recipe_file_hash_map.insert(
            recipe_manifest_with_path.id.clone(),
            recipe_manifest_with_path.file_path.clone(),
        );
    }

    state.set_recipe_file_hash_map(recipe_file_hash_map).await;

    resolved_path.ok_or_else(|| ErrorResponse {
        message: format!("Recipe not found: {}", id),
        status: StatusCode::NOT_FOUND,
    })
}

pub async fn load_recipe_by_id(state: &AppState, id: &str) -> Result<Recipe, ErrorResponse> {
    let path = get_recipe_file_path_by_id(state, id).await?;

    Recipe::from_file_path(&path).map_err(|err| ErrorResponse {
        message: format!("Failed to load recipe: {}", err),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    })
}
