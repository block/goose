use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::message::Message;
use goose::recipe::Recipe;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct AuthorRequest {
    #[serde(default)]
    contact: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateRecipeResponse {
    recipe: Option<Recipe>,
    error: Option<String>,
}

/// Create a Recipe configuration from the current state of an agent
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
    let recipe_result = agent.create_recipe(request.messages).await;

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

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/recipe/create", post(create_recipe))
        .with_state(state)
}
