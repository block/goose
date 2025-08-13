use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::conversation::{message::Message, Conversation};
use goose::recipe::Recipe;
use goose::recipe_deeplink;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    let error_response = CreateRecipeResponse {
        recipe: None,
        error: Some("Missing agent".to_string()),
    };
    let agent = state
        .get_agent()
        .await
        .map_err(|_| (StatusCode::PRECONDITION_FAILED, Json(error_response)))?;

    // Create base recipe from agent state and messages
    let recipe_result = agent
        .create_recipe(Conversation::new_unvalidated(request.messages))
        .await;

    match recipe_result {
        Ok(mut recipe) => {
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

            Ok(Json(CreateRecipeResponse {
                recipe: Some(recipe),
                error: None,
            }))
        }
        Err(e) => {
            // Return 400 Bad Request with error message
            let error_response = CreateRecipeResponse {
                recipe: None,
                error: Some(e.to_string()),
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

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/recipes/create", post(create_recipe))
        .route("/recipes/encode", post(encode_recipe))
        .route("/recipes/decode", post(decode_recipe))
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
