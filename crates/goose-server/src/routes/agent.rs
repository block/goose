use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use goose::config::PermissionManager;
use goose::model::ModelConfig;
use goose::providers::create;
use goose::recipe::Response;
use goose::{
    agents::{extension::ToolInfo, extension_manager::get_parameter_names},
    config::permission::PermissionLevel,
};
use goose::{config::Config, recipe::SubRecipe};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize)]
struct VersionsResponse {
    available_versions: Vec<String>,
    default_version: String,
}

#[derive(Deserialize)]
struct ExtendPromptRequest {
    extension: String,
}

#[derive(Serialize)]
struct ExtendPromptResponse {
    success: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddSubRecipesRequest {
    sub_recipes: Vec<SubRecipe>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AddSubRecipesResponse {
    success: bool,
}

#[derive(Deserialize)]
struct ProviderFile {
    name: String,
    description: String,
    models: Vec<String>,
    required_keys: Vec<String>,
}

#[derive(Serialize)]
struct ProviderDetails {
    name: String,
    description: String,
    models: Vec<String>,
    required_keys: Vec<String>,
}

#[derive(Serialize)]
struct ProviderList {
    id: String,
    details: ProviderDetails,
}

#[derive(Deserialize)]
struct UpdateProviderRequest {
    provider: String,
    model: Option<String>,
}

#[derive(Deserialize)]
struct SessionConfigRequest {
    response: Option<Response>,
}

#[derive(Deserialize)]
pub struct GetToolsQuery {
    extension_name: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn get_versions() -> Json<VersionsResponse> {
    let versions = ["goose".to_string()];
    let default_version = "goose".to_string();

    Json(VersionsResponse {
        available_versions: versions.iter().map(|v| v.to_string()).collect(),
        default_version,
    })
}

#[utoipa::path(
    post,
    path = "/agent/add_sub_recipes",
    request_body = AddSubRecipesRequest,
    responses(
        (status = 200, description = "added sub recipes to agent successfully", body = AddSubRecipesResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
    ),
)]
async fn add_sub_recipes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<AddSubRecipesRequest>,
) -> Result<Json<AddSubRecipesResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;
    agent.add_sub_recipes(payload.sub_recipes.clone()).await;
    Ok(Json(AddSubRecipesResponse { success: true }))
}

async fn extend_prompt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<ExtendPromptRequest>,
) -> Result<Json<ExtendPromptResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;
    agent.extend_system_prompt(payload.extension.clone()).await;
    Ok(Json(ExtendPromptResponse { success: true }))
}

async fn list_providers() -> Json<Vec<ProviderList>> {
    let contents = include_str!("providers_and_keys.json");

    let providers: HashMap<String, ProviderFile> =
        serde_json::from_str(contents).expect("Failed to parse providers_and_keys.json");

    let response: Vec<ProviderList> = providers
        .into_iter()
        .map(|(id, provider)| ProviderList {
            id,
            details: ProviderDetails {
                name: provider.name,
                description: provider.description,
                models: provider.models,
                required_keys: provider.required_keys,
            },
        })
        .collect();

    // Return the response as JSON.
    Json(response)
}

#[utoipa::path(
    get,
    path = "/agent/tools",
    params(
        ("extension_name" = Option<String>, Query, description = "Optional extension name to filter tools")
    ),
    responses(
        (status = 200, description = "Tools retrieved successfully", body = Vec<ToolInfo>),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_tools(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<GetToolsQuery>,
) -> Result<Json<Vec<ToolInfo>>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();
    let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;
    let permission_manager = PermissionManager::default();

    let mut tools: Vec<ToolInfo> = agent
        .list_tools(query.extension_name)
        .await
        .into_iter()
        .map(|tool| {
            let permission = permission_manager
                .get_user_permission(&tool.name)
                .or_else(|| {
                    if goose_mode == "smart_approve" {
                        permission_manager.get_smart_approve_permission(&tool.name)
                    } else if goose_mode == "approve" {
                        Some(PermissionLevel::AskBefore)
                    } else {
                        None
                    }
                });

            ToolInfo::new(
                &tool.name,
                tool.description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default(),
                get_parameter_names(&tool),
                permission,
            )
        })
        .collect::<Vec<ToolInfo>>();
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(tools))
}

#[utoipa::path(
    post,
    path = "/agent/update_provider",
    responses(
        (status = 200, description = "Update provider completed", body = String),
        (status = 400, description = "Bad request - missing or invalid parameters"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_agent_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProviderRequest>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    let config = Config::global();
    let model = match payload
        .model
        .or_else(|| config.get_param("GOOSE_MODEL").ok())
    {
        Some(m) => m,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    let model_config = ModelConfig::new(&model).map_err(|_| StatusCode::BAD_REQUEST)?;

    let new_provider =
        create(&payload.provider, model_config).map_err(|_| StatusCode::BAD_REQUEST)?;
    agent
        .update_provider(new_provider)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/update_router_tool_selector",
    responses(
        (status = 200, description = "Tool selection strategy updated successfully", body = String),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_router_tool_selector(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<String>, Json<ErrorResponse>> {
    verify_secret_key(&headers, &state).map_err(|_| {
        Json(ErrorResponse {
            error: "Unauthorized - Invalid or missing API key".to_string(),
        })
    })?;

    let agent = state.get_agent().await.map_err(|e| {
        tracing::error!("Failed to get agent: {}", e);
        Json(ErrorResponse {
            error: format!("Failed to get agent: {}", e),
        })
    })?;

    agent
        .update_router_tool_selector(None, Some(true))
        .await
        .map_err(|e| {
            tracing::error!("Failed to update tool selection strategy: {}", e);
            Json(ErrorResponse {
                error: format!("Failed to update tool selection strategy: {}", e),
            })
        })?;

    Ok(Json(
        "Tool selection strategy updated successfully".to_string(),
    ))
}

#[utoipa::path(
    post,
    path = "/agent/session_config",
    responses(
        (status = 200, description = "Session config updated successfully", body = String),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_session_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<SessionConfigRequest>,
) -> Result<Json<String>, Json<ErrorResponse>> {
    verify_secret_key(&headers, &state).map_err(|_| {
        Json(ErrorResponse {
            error: "Unauthorized - Invalid or missing API key".to_string(),
        })
    })?;

    let agent = state.get_agent().await.map_err(|e| {
        tracing::error!("Failed to get agent: {}", e);
        Json(ErrorResponse {
            error: format!("Failed to get agent: {}", e),
        })
    })?;

    if let Some(response) = payload.response {
        agent.add_final_output_tool(response).await;

        tracing::info!("Added final output tool with response config");
        Ok(Json(
            "Session config updated with final output tool".to_string(),
        ))
    } else {
        Ok(Json("Nothing provided to update.".to_string()))
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/agent/versions", get(get_versions))
        .route("/agent/providers", get(list_providers))
        .route("/agent/prompt", post(extend_prompt))
        .route("/agent/tools", get(get_tools))
        .route("/agent/update_provider", post(update_agent_provider))
        .route(
            "/agent/update_router_tool_selector",
            post(update_router_tool_selector),
        )
        .route("/agent/session_config", post(update_session_config))
        .route("/agent/add_sub_recipes", post(add_sub_recipes))
        .with_state(state)
}
