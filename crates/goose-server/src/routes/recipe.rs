use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::get;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::conversation::{message::Message, Conversation};
use goose::recipe::build_recipe::{build_recipe_from_file, RecipeError};
use goose::recipe::Recipe;
use goose::recipe_deeplink;

use http::HeaderMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::recipe_utils::{
    create_temp_deeplink_recipe_file, find_recipe_file_path_by_id, get_all_recipes_manifests,
    save_recipe_file_hash_map,
};
use crate::routes::utils::verify_secret_key;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRecipeRequest {
    messages: Vec<Message>,
    // Required metadata
    title: String,
    description: String,
    // Optional fields
    #[serde(default)]
    activities: Option<Vec<String>>,
    #[serde(default)]
    author: Option<AuthorRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthorRequest {
    #[serde(default)]
    contact: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateRecipeResponse {
    recipe: Option<Recipe>,
    error: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EncodeRecipeRequest {
    recipe: Recipe,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EncodeRecipeResponse {
    deeplink: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DecodeRecipeRequest {
    deeplink: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DecodeRecipeResponse {
    recipe: Recipe,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ScanRecipeRequest {
    recipe: Recipe,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScanRecipeResponse {
    has_security_warnings: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecipeManifestResponse {
    name: String,
    #[serde(rename = "isGlobal")]
    is_global: bool,
    recipe: Recipe,
    #[serde(rename = "lastModified")]
    last_modified: String,
    id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteRecipeRequest {
    id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListRecipeResponse {
    recipe_manifest_responses: Vec<RecipeManifestResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoadRecipeRequest {
    id: String,
    parameters: Option<Vec<(String, String)>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoadRecipeResponse {
    recipe: Recipe,
}

#[utoipa::path(
    post,
    path = "/recipes/create",
    request_body = CreateRecipeRequest,
    responses(
        (status = 200, description = "Recipe created successfully", body = CreateRecipeResponse),
        (status = 400, description = "Bad request"),
        (status = 412, description = "Precondition failed - Agent not available"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Recipe Management"
)]
/// Create a Recipe configuration from the current session
async fn create_recipe(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateRecipeRequest>,
) -> Result<Json<CreateRecipeResponse>, (StatusCode, Json<CreateRecipeResponse>)> {
    tracing::info!(
        "Recipe creation request received with {} messages",
        request.messages.len()
    );

    let error_response = CreateRecipeResponse {
        recipe: None,
        error: Some("Missing agent".to_string()),
    };
    let agent = state.get_agent().await.map_err(|e| {
        tracing::error!("Failed to get agent for recipe creation: {}", e);
        (StatusCode::PRECONDITION_FAILED, Json(error_response))
    })?;

    tracing::debug!("Agent retrieved successfully, creating recipe from conversation");

    // Create base recipe from agent state and messages
    let recipe_result = agent
        .create_recipe(Conversation::new_unvalidated(request.messages))
        .await;

    match recipe_result {
        Ok(mut recipe) => {
            tracing::info!("Recipe created successfully with title: '{}'", recipe.title);

            // Update with user-provided metadata
            recipe.title = request.title;
            recipe.description = request.description;
            if request.activities.is_some() {
                recipe.activities = request.activities
            };

            // Add author if provided
            if let Some(author_req) = request.author {
                recipe.author = Some(goose::recipe::Author {
                    contact: author_req.contact,
                    metadata: author_req.metadata,
                });
            }

            tracing::debug!("Recipe metadata updated, returning success response");

            Ok(Json(CreateRecipeResponse {
                recipe: Some(recipe),
                error: None,
            }))
        }
        Err(e) => {
            // Log the detailed error for debugging
            tracing::error!("Recipe creation failed: {}", e);
            tracing::error!("Error details: {:?}", e);

            // Return 400 Bad Request with error message
            let error_message = format!("Recipe creation failed: {}", e);
            let error_response = CreateRecipeResponse {
                recipe: None,
                error: Some(error_message),
            };
            Err((StatusCode::BAD_REQUEST, Json(error_response)))
        }
    }
}

#[utoipa::path(
    post,
    path = "/recipes/encode",
    request_body = EncodeRecipeRequest,
    responses(
        (status = 200, description = "Recipe encoded successfully", body = EncodeRecipeResponse),
        (status = 400, description = "Bad request")
    ),
    tag = "Recipe Management"
)]
async fn encode_recipe(
    Json(request): Json<EncodeRecipeRequest>,
) -> Result<Json<EncodeRecipeResponse>, StatusCode> {
    match recipe_deeplink::encode(&request.recipe) {
        Ok(encoded) => Ok(Json(EncodeRecipeResponse { deeplink: encoded })),
        Err(err) => {
            tracing::error!("Failed to encode recipe: {}", err);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[utoipa::path(
    post,
    path = "/recipes/decode",
    request_body = DecodeRecipeRequest,
    responses(
        (status = 200, description = "Recipe decoded successfully", body = DecodeRecipeResponse),
        (status = 400, description = "Bad request")
    ),
    tag = "Recipe Management"
)]
async fn decode_recipe(
    Json(request): Json<DecodeRecipeRequest>,
) -> Result<Json<DecodeRecipeResponse>, StatusCode> {
    match recipe_deeplink::decode(&request.deeplink) {
        Ok(recipe) => Ok(Json(DecodeRecipeResponse { recipe })),
        Err(err) => {
            tracing::error!("Failed to decode deeplink: {}", err);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[utoipa::path(
    post,
    path = "/recipes/scan",
    request_body = ScanRecipeRequest,
    responses(
        (status = 200, description = "Recipe scanned successfully", body = ScanRecipeResponse),
    ),
    tag = "Recipe Management"
)]
async fn scan_recipe(
    Json(request): Json<ScanRecipeRequest>,
) -> Result<Json<ScanRecipeResponse>, StatusCode> {
    let has_security_warnings = request.recipe.check_for_security_warnings();

    Ok(Json(ScanRecipeResponse {
        has_security_warnings,
    }))
}

#[utoipa::path(
    get,
    path = "/recipes/list",
    responses(
        (status = 200, description = "Get recipe list successfully", body = ListRecipeResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Recipe Management"
)]
async fn list_recipes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ListRecipeResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let recipe_manifest_with_paths = get_all_recipes_manifests().unwrap();
    let mut recipe_file_hash_map = HashMap::new();
    let recipe_manifest_responses = recipe_manifest_with_paths
        .iter()
        .map(|recipe_manifest_with_path| {
            let id = &recipe_manifest_with_path.id;
            let file_path = recipe_manifest_with_path.file_path.clone();
            recipe_file_hash_map.insert(id.clone(), file_path);
            RecipeManifestResponse {
                name: recipe_manifest_with_path.name.clone(),
                is_global: recipe_manifest_with_path.is_global,
                recipe: recipe_manifest_with_path.recipe.clone(),
                id: id.clone(),
                last_modified: recipe_manifest_with_path.last_modified.clone(),
            }
        })
        .collect::<Vec<RecipeManifestResponse>>();

    if let Err(e) = save_recipe_file_hash_map(&recipe_file_hash_map) {
        tracing::error!("Failed to save recipe file hash map to temp file: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(ListRecipeResponse {
        recipe_manifest_responses,
    }))
}

#[utoipa::path(
    post,
    path = "/recipes/delete",
    request_body = DeleteRecipeRequest,
    responses(
        (status = 204, description = "Recipe deleted successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Recipe not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Recipe Management"
)]
async fn delete_recipe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<DeleteRecipeRequest>,
) -> StatusCode {
    if verify_secret_key(&headers, &state).is_err() {
        return StatusCode::UNAUTHORIZED;
    }
    let file_path = match find_recipe_file_path_by_id(&request.id) {
        Ok(path) => path,
        Err(e) => {
            tracing::error!("Failed to find recipe file path: {}", e);
            return StatusCode::NOT_FOUND;
        }
    };

    if fs::remove_file(file_path).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::NO_CONTENT
}

#[utoipa::path(
    post,
    path = "/recipes/load",
    request_body = LoadRecipeRequest,
    responses(
        (status = 200, description = "Recipe loaded successfully", body = LoadRecipeResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Recipe not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Recipe Management"
)]
async fn load_recipe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<LoadRecipeRequest>,
) -> Result<Json<LoadRecipeResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let file_path: PathBuf = match request.id.contains("deeplink_") {
        true => {
            let deeplink = request.id.strip_prefix("deeplink_").unwrap_or(&request.id);
            let recipe_content =
                recipe_deeplink::decode_deeplink_to_string(deeplink).map_err(|e| {
                    tracing::error!("Failed to decode recipe deeplink: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            create_temp_deeplink_recipe_file(&recipe_content).map_err(|e| {
                tracing::error!("Failed to create temp deeplink recipe file: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
        false => find_recipe_file_path_by_id(&request.id).map_err(|e| {
            tracing::error!("Failed to find recipe file path: {}", e);
            StatusCode::NOT_FOUND
        })?,
    };
    let recipe = match build_recipe_from_file(&file_path, request.parameters.unwrap_or_default()) {
        Ok(recipe) => recipe,
        Err(RecipeError::MissingParams { raw_recipe, .. }) => *raw_recipe,
        Err(e) => {
            tracing::error!("Failed to parse recipe: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    Ok(Json(LoadRecipeResponse { recipe }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/recipes/create", post(create_recipe))
        .route("/recipes/encode", post(encode_recipe))
        .route("/recipes/decode", post(decode_recipe))
        .route("/recipes/scan", post(scan_recipe))
        .route("/recipes/list", get(list_recipes))
        .route("/recipes/delete", post(delete_recipe))
        .route("/recipes/load", post(load_recipe))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::recipe::Recipe;

    #[tokio::test]
    async fn test_decode_and_encode_recipe() {
        let original_recipe = Recipe::builder()
            .title("Test Recipe")
            .description("A test recipe")
            .instructions("Test instructions")
            .build()
            .unwrap();
        let encoded = recipe_deeplink::encode(&original_recipe).unwrap();

        let request = DecodeRecipeRequest {
            deeplink: encoded.clone(),
        };
        let response = decode_recipe(Json(request)).await;

        assert!(response.is_ok());
        let decoded = response.unwrap().0.recipe;
        assert_eq!(decoded.title, original_recipe.title);
        assert_eq!(decoded.description, original_recipe.description);
        assert_eq!(decoded.instructions, original_recipe.instructions);

        let encode_request = EncodeRecipeRequest { recipe: decoded };
        let encode_response = encode_recipe(Json(encode_request)).await;

        assert!(encode_response.is_ok());
        let encoded_again = encode_response.unwrap().0.deeplink;
        assert!(!encoded_again.is_empty());
        assert_eq!(encoded, encoded_again);
    }
}
