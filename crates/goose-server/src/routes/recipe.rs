use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::routing::get;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::recipe::local_recipes;
use goose::recipe::validate_recipe::validate_recipe_template_from_content;
use goose::recipe::Recipe;
use goose::recipe_deeplink;
use goose::session::SessionManager;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_path_to_error::deserialize as deserialize_with_path;
use utoipa::ToSchema;

fn format_json_rejection_message(rejection: &JsonRejection) -> String {
    match rejection {
        JsonRejection::JsonDataError(err) => {
            format!("Request body validation failed: {}", clean_data_error(err))
        }
        JsonRejection::JsonSyntaxError(err) => format!("Invalid JSON payload: {}", err.body_text()),
        JsonRejection::MissingJsonContentType(err) => err.body_text(),
        JsonRejection::BytesRejection(err) => err.body_text(),
        _ => rejection.body_text(),
    }
}

fn clean_data_error(err: &axum::extract::rejection::JsonDataError) -> String {
    let message = err.body_text();
    message
        .strip_prefix("Failed to deserialize the JSON body into the target type: ")
        .map(|s| s.to_string())
        .unwrap_or_else(|| message.to_string())
}

use crate::routes::errors::ErrorResponse;
use crate::routes::recipe_utils::{
    get_all_recipes_manifests, get_recipe_file_path_by_id, validate_recipe, RecipeValidationError,
};
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRecipeRequest {
    session_id: String,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveRecipeRequest {
    recipe: Recipe,
    id: Option<String>,
}
#[derive(Debug, Deserialize, ToSchema)]
pub struct ParseRecipeRequest {
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParseRecipeResponse {
    pub recipe: Recipe,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecipeManifestResponse {
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
async fn create_recipe(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateRecipeRequest>,
) -> Result<Json<CreateRecipeResponse>, StatusCode> {
    tracing::info!(
        "Recipe creation request received for session_id: {}",
        request.session_id
    );

    let session = match SessionManager::get_session(&request.session_id, true).await {
        Ok(session) => session,
        Err(e) => {
            tracing::error!("Failed to get session: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let conversation = match session.conversation {
        Some(conversation) => conversation,
        None => {
            let error_message = "Session has no conversation".to_string();
            let error_response = CreateRecipeResponse {
                recipe: None,
                error: Some(error_message),
            };
            return Ok(Json(error_response));
        }
    };

    let agent = state.get_agent_for_route(request.session_id).await?;

    let recipe_result = agent.create_recipe(conversation).await;

    match recipe_result {
        Ok(mut recipe) => {
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
            let error_response = CreateRecipeResponse {
                recipe: None,
                error: Some(format!("Failed to create recipe: {}", e)),
            };
            Ok(Json(error_response))
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
        Ok(recipe) => match validate_recipe(&recipe) {
            Ok(_) => Ok(Json(DecodeRecipeResponse { recipe })),
            Err(RecipeValidationError { status, .. }) => Err(status),
        },
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
    let recipe_manifest_with_paths = get_all_recipes_manifests().unwrap_or_default();
    let mut recipe_file_hash_map = HashMap::new();
    let recipe_manifest_responses = recipe_manifest_with_paths
        .iter()
        .map(|recipe_manifest_with_path| {
            let id = &recipe_manifest_with_path.id;
            let file_path = recipe_manifest_with_path.file_path.clone();
            recipe_file_hash_map.insert(id.clone(), file_path);
            RecipeManifestResponse {
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
    let file_path = match get_recipe_file_path_by_id(state.as_ref(), &request.id).await {
        Ok(path) => path,
        Err(err) => return err.status,
    };

    if fs::remove_file(file_path).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::NO_CONTENT
}

#[utoipa::path(
    post,
    path = "/recipes/save",
    request_body = SaveRecipeRequest,
    responses(
        (status = 204, description = "Recipe saved to file successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Recipe Management"
)]
async fn save_recipe(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<StatusCode, ErrorResponse> {
    let Json(raw_json) = payload.map_err(json_rejection_to_error_response)?;
    let request = deserialize_save_recipe_request(raw_json)?;
    ensure_recipe_valid(&request.recipe)?;

    let file_path = match request.id.as_ref() {
        Some(id) => Some(get_recipe_file_path_by_id(state.as_ref(), id).await?),
        None => None,
    };

    match local_recipes::save_recipe_to_file(request.recipe, file_path) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(ErrorResponse {
            message: e.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }),
    }
}

fn json_rejection_to_error_response(rejection: JsonRejection) -> ErrorResponse {
    ErrorResponse {
        message: format_json_rejection_message(&rejection),
        status: StatusCode::BAD_REQUEST,
    }
}

fn ensure_recipe_valid(recipe: &Recipe) -> Result<(), ErrorResponse> {
    if let Err(err) = validate_recipe(recipe) {
        return Err(ErrorResponse {
            message: err.message,
            status: err.status,
        });
    }

    Ok(())
}

fn deserialize_save_recipe_request(value: Value) -> Result<SaveRecipeRequest, ErrorResponse> {
    let payload = value.to_string();
    let mut deserializer = serde_json::Deserializer::from_str(&payload);
    let result: Result<SaveRecipeRequest, _> = deserialize_with_path(&mut deserializer);
    result.map_err(|err| {
        let path = err.path().to_string();
        let inner = err.into_inner();
        let message = if path.is_empty() {
            format!("Save recipe validation failed: {}", inner)
        } else {
            format!(
                "save recipe validation failed at {}: {}",
                path.trim_start_matches('.'),
                inner
            )
        };
        ErrorResponse {
            message,
            status: StatusCode::BAD_REQUEST,
        }
    })
}

#[utoipa::path(
    post,
    path = "/recipes/parse",
    request_body = ParseRecipeRequest,
    responses(
        (status = 200, description = "Recipe parsed successfully", body = ParseRecipeResponse),
        (status = 400, description = "Bad request - Invalid recipe format", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Recipe Management"
)]
async fn parse_recipe(
    Json(request): Json<ParseRecipeRequest>,
) -> Result<Json<ParseRecipeResponse>, ErrorResponse> {
    let recipe = validate_recipe_template_from_content(&request.content, None).map_err(|e| {
        ErrorResponse {
            message: format!("Invalid recipe format: {}", e),
            status: StatusCode::BAD_REQUEST,
        }
    })?;

    Ok(Json(ParseRecipeResponse { recipe }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/recipes/create", post(create_recipe))
        .route("/recipes/encode", post(encode_recipe))
        .route("/recipes/decode", post(decode_recipe))
        .route("/recipes/scan", post(scan_recipe))
        .route("/recipes/list", get(list_recipes))
        .route("/recipes/delete", post(delete_recipe))
        .route("/recipes/save", post(save_recipe))
        .route("/recipes/parse", post(parse_recipe))
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
