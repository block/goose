use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use axum::routing::get;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::conversation::{message::Message, Conversation};
use goose::recipe::recipe_library;
use goose::recipe::Recipe;
use goose::recipe_deeplink;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::recipe_utils::get_all_recipes_manifests;
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
    session_id: String,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveRecipeToFileRequest {
    recipe: Recipe,
    id: Option<String>,
    is_global: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SaveRecipeToFileErrorResponse {
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecipeManifestResponse {
    name: String,
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
) -> Result<Json<CreateRecipeResponse>, StatusCode> {
    tracing::info!(
        "Recipe creation request received with {} messages",
        request.messages.len()
    );

    let agent = state.get_agent_for_route(request.session_id).await?;

    // Create base recipe from agent state and messages
    let recipe_result = agent
        .create_recipe(Conversation::new_unvalidated(request.messages))
        .await;

    match recipe_result {
        Ok(mut recipe) => {
            recipe.title = request.title;
            recipe.description = request.description;
            if request.activities.is_some() {
                recipe.activities = request.activities
            };

            if let Some(author_req) = request.author {
                recipe.author = Some(goose::recipe::Author {
                    contact: author_req.contact,
                    metadata: author_req.metadata,
                });
            }

            Ok(Json(CreateRecipeResponse {
                recipe: Some(recipe),
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!("Error details: {:?}", e);
            Err(StatusCode::BAD_REQUEST)
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
) -> Result<Json<ListRecipeResponse>, StatusCode> {
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
                recipe: recipe_manifest_with_path.recipe.clone(),
                id: id.clone(),
                last_modified: recipe_manifest_with_path.last_modified.clone(),
            }
        })
        .collect::<Vec<RecipeManifestResponse>>();
    state.set_recipe_file_hash_map(recipe_file_hash_map).await;

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
    Json(request): Json<DeleteRecipeRequest>,
) -> StatusCode {
    let recipe_file_hash_map = state.recipe_file_hash_map.lock().await;
    let file_path = match recipe_file_hash_map.get(&request.id) {
        Some(path) => path,
        None => return StatusCode::NOT_FOUND,
    };

    if fs::remove_file(file_path).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::NO_CONTENT
}

#[utoipa::path(
    post,
    path = "/recipes/save_to_file",
    request_body = SaveRecipeToFileRequest,
    responses(
        (status = 204, description = "Recipe saved to file successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error", body = SaveRecipeToFileErrorResponse)
    ),
    tag = "Recipe Management"
)]
async fn save_recipe_to_file(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SaveRecipeToFileRequest>,
) -> Result<StatusCode, (StatusCode, Json<SaveRecipeToFileErrorResponse>)> {
    let file_path = match request.id {
        Some(id) => state.recipe_file_hash_map.lock().await.get(&id).cloned(),
        None => None,
    };

    match recipe_library::save_recipe_to_file(request.recipe, request.is_global, file_path) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SaveRecipeToFileErrorResponse {
                message: e.to_string(),
            }),
        )),
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/recipes/create", post(create_recipe))
        .route("/recipes/encode", post(encode_recipe))
        .route("/recipes/decode", post(decode_recipe))
        .route("/recipes/scan", post(scan_recipe))
        .route("/recipes/list", get(list_recipes))
        .route("/recipes/delete", post(delete_recipe))
        .route("/recipes/save_to_file", post(save_recipe_to_file))
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
