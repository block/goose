use std::collections::HashMap;
use std::fs;
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use anyhow::Result;
use axum::http::StatusCode;
use goose::agents::Agent;
use goose::prompt_template::render_template;
use goose::recipe::build_recipe::{build_recipe_from_template, RecipeError};
use goose::recipe::local_recipes::{get_recipe_library_dir, list_local_recipes};
use goose::recipe::validate_recipe::validate_recipe_template_from_content;
use goose::recipe::Recipe;
use serde::Serialize;
use serde_json::Value;
use tracing::error;
use utoipa::ToSchema;

pub struct RecipeValidationError {
    pub status: StatusCode,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecipeManifest {
    pub id: String,
    pub recipe: Recipe,
    #[schema(value_type = String)]
    pub file_path: PathBuf,
    pub last_modified: String,
    pub schedule_cron: Option<String>,
    pub slash_command: Option<String>,
}

pub fn short_id_from_path(path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let h = hasher.finish();
    format!("{:016x}", h)
}

pub fn get_all_recipes_manifests() -> Result<Vec<RecipeManifest>> {
    let recipes_with_path = list_local_recipes()?;
    let mut recipe_manifests_with_path = Vec::new();
    for (file_path, recipe) in recipes_with_path {
        let Ok(last_modified) = fs::metadata(file_path.clone())
            .map(|m| chrono::DateTime::<chrono::Utc>::from(m.modified().unwrap()).to_rfc3339())
        else {
            continue;
        };

        let manifest_with_path = RecipeManifest {
            id: short_id_from_path(file_path.to_string_lossy().as_ref()),
            recipe,
            file_path,
            last_modified,
            schedule_cron: None,
            slash_command: None,
        };
        recipe_manifests_with_path.push(manifest_with_path);
    }
    recipe_manifests_with_path.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(recipe_manifests_with_path)
}

pub fn validate_recipe(recipe: &Recipe) -> Result<(), RecipeValidationError> {
    let recipe_yaml = recipe.to_yaml().map_err(|err| {
        let message = err.to_string();
        error!("Failed to serialize recipe for validation: {}", message);
        RecipeValidationError {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    })?;

    validate_recipe_template_from_content(&recipe_yaml, None).map_err(|err| {
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

pub async fn build_recipe_with_parameter_values(
    original_recipe: &Recipe,
    user_recipe_values: HashMap<String, String>,
) -> Result<Option<Recipe>> {
    let recipe_content = original_recipe.to_yaml()?;

    let recipe_dir = get_recipe_library_dir(true);
    let params = user_recipe_values.into_iter().collect();

    let recipe = match build_recipe_from_template(
        recipe_content,
        &recipe_dir,
        params,
        None::<fn(&str, &str) -> Result<String, anyhow::Error>>,
    ) {
        Ok(recipe) => Some(recipe),
        Err(RecipeError::MissingParams { .. }) => None,
        Err(e) => return Err(anyhow::anyhow!(e)),
    };

    Ok(recipe)
}

pub async fn apply_recipe_to_agent(
    agent: &Arc<Agent>,
    recipe: &Recipe,
    include_final_output_tool: bool,
) -> Option<String> {
    agent
        .apply_recipe_components(recipe.response.clone(), include_final_output_tool)
        .await;

    recipe.instructions.as_ref().map(|instructions| {
        let mut context: HashMap<&str, Value> = HashMap::new();
        context.insert("recipe_instructions", Value::String(instructions.clone()));
        render_template("desktop_recipe_instruction.md", &context).expect("Prompt should render")
    })
}
