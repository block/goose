use axum::{
    extract::Path,
    routing::{delete, get, put},
    Json, Router,
};
use goose::prompt_template::{get_template, list_templates, reset_template, save_template, Template};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct PromptsListResponse {
    pub prompts: Vec<Template>,
}

#[derive(Serialize, ToSchema)]
pub struct PromptContentResponse {
    pub name: String,
    pub content: String,
    pub default_content: String,
    pub is_customized: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct SavePromptRequest {
    pub content: String,
}

#[utoipa::path(
    get,
    path = "/config/prompts",
    responses(
        (status = 200, description = "List of all available prompts", body = PromptsListResponse)
    )
)]
pub async fn get_prompts() -> Json<PromptsListResponse> {
    Json(PromptsListResponse {
        prompts: list_templates(),
    })
}

#[utoipa::path(
    get,
    path = "/config/prompts/{name}",
    params(
        ("name" = String, Path, description = "Prompt template name (e.g., system.md)")
    ),
    responses(
        (status = 200, description = "Prompt content retrieved successfully", body = PromptContentResponse),
        (status = 404, description = "Prompt not found")
    )
)]
pub async fn get_prompt(
    Path(name): Path<String>,
) -> Result<Json<PromptContentResponse>, StatusCode> {
    let template = get_template(&name).ok_or(StatusCode::NOT_FOUND)?;

    let content = template.user_content.as_ref().unwrap_or(&template.default_content);

    Ok(Json(PromptContentResponse {
        name: template.name,
        content: content.clone(),
        default_content: template.default_content,
        is_customized: template.is_customized,
    }))
}

#[utoipa::path(
    put,
    path = "/config/prompts/{name}",
    params(
        ("name" = String, Path, description = "Prompt template name (e.g., system.md)")
    ),
    request_body = SavePromptRequest,
    responses(
        (status = 200, description = "Prompt saved successfully", body = String),
        (status = 404, description = "Prompt not found"),
        (status = 500, description = "Failed to save prompt")
    )
)]
pub async fn save_prompt(
    Path(name): Path<String>,
    Json(request): Json<SavePromptRequest>,
) -> Result<Json<String>, StatusCode> {
    save_template(&name, &request.content).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            StatusCode::NOT_FOUND
        } else {
            tracing::error!("Failed to save prompt {}: {}", name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(format!("Saved prompt: {}", name)))
}

#[utoipa::path(
    delete,
    path = "/config/prompts/{name}",
    params(
        ("name" = String, Path, description = "Prompt template name (e.g., system.md)")
    ),
    responses(
        (status = 200, description = "Prompt reset to default successfully", body = String),
        (status = 404, description = "Prompt not found"),
        (status = 500, description = "Failed to reset prompt")
    )
)]
pub async fn reset_prompt(Path(name): Path<String>) -> Result<Json<String>, StatusCode> {
    reset_template(&name).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            StatusCode::NOT_FOUND
        } else {
            tracing::error!("Failed to reset prompt {}: {}", name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(format!("Reset prompt to default: {}", name)))
}

pub fn routes() -> Router {
    Router::new()
        .route("/config/prompts", get(get_prompts))
        .route("/config/prompts/{name}", get(get_prompt))
        .route("/config/prompts/{name}", put(save_prompt))
        .route("/config/prompts/{name}", delete(reset_prompt))
}
