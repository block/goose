use crate::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use goose::config::PermissionManager;
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use goose::model::ModelConfig;
use goose::providers::create;
use goose::recipe::{Recipe, Response};
use goose::session;
use goose::session::SessionMetadata;
use goose::{
    agents::{extension::ToolInfo, extension_manager::get_parameter_names},
    config::permission::PermissionLevel,
};
use goose::{config::Config, recipe::SubRecipe};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::error;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ExtendPromptRequest {
    extension: String,
    session_id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ExtendPromptResponse {
    success: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddSubRecipesRequest {
    sub_recipes: Vec<SubRecipe>,
    session_id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AddSubRecipesResponse {
    success: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateProviderRequest {
    provider: String,
    model: Option<String>,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SessionConfigRequest {
    response: Option<Response>,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GetToolsQuery {
    extension_name: Option<String>,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateRouterToolSelectorRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StartAgentRequest {
    working_dir: String,
    recipe: Option<Recipe>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResumeAgentRequest {
    session_id: String,
}

// This is the same as SessionHistoryResponse
#[derive(Serialize, utoipa::ToSchema)]
pub struct StartAgentResponse {
    session_id: String,
    metadata: SessionMetadata,
    messages: Vec<Message>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    error: String,
}

#[utoipa::path(
    post,
    path = "/agent/start",
    request_body = StartAgentRequest,
    responses(
        (status = 200, description = "Agent started successfully", body = StartAgentResponse),
        (status = 400, description = "Bad request - invalid working directory"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error")
    )
)]
async fn start_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartAgentRequest>,
) -> Result<Json<StartAgentResponse>, StatusCode> {
    let session_id = session::generate_session_id();
    let counter = state.session_counter.fetch_add(1, Ordering::SeqCst) + 1;

    let metadata = SessionMetadata {
        working_dir: PathBuf::from(&payload.working_dir),
        description: format!("New session {}", counter),
        schedule_id: None,
        message_count: 0,
        total_tokens: Some(0),
        input_tokens: Some(0),
        output_tokens: Some(0),
        accumulated_total_tokens: Some(0),
        accumulated_input_tokens: Some(0),
        accumulated_output_tokens: Some(0),
        extension_data: Default::default(),
        recipe: payload.recipe,
    };

    let session_path = match session::get_path(session::Identifier::Name(session_id.clone())) {
        Ok(path) => path,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let conversation = Conversation::empty();
    session::storage::save_messages_with_metadata(&session_path, &metadata, &conversation)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StartAgentResponse {
        session_id,
        metadata,
        messages: conversation.messages().clone(),
    }))
}

#[utoipa::path(
    post,
    path = "/agent/resume",
    request_body = ResumeAgentRequest,
    responses(
        (status = 200, description = "Agent started successfully", body = StartAgentResponse),
        (status = 400, description = "Bad request - invalid working directory"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error")
    )
)]
async fn resume_agent(
    Json(payload): Json<ResumeAgentRequest>,
) -> Result<Json<StartAgentResponse>, StatusCode> {
    let session_path =
        match session::get_path(session::Identifier::Name(payload.session_id.clone())) {
            Ok(path) => path,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

    let metadata = session::read_metadata(&session_path).map_err(|_| StatusCode::NOT_FOUND)?;

    let conversation = match session::read_messages(&session_path) {
        Ok(messages) => messages,
        Err(e) => {
            error!("Failed to read session messages: {:?}", e);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    Ok(Json(StartAgentResponse {
        session_id: payload.session_id.clone(),
        metadata,
        messages: conversation.messages().clone(),
    }))
}

#[utoipa::path(
    post,
    path = "/agent/add_sub_recipes",
    request_body = AddSubRecipesRequest,
    responses(
        (status = 200, description = "Added sub recipes to agent successfully", body = AddSubRecipesResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
    ),
)]
async fn add_sub_recipes(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddSubRecipesRequest>,
) -> Result<Json<AddSubRecipesResponse>, StatusCode> {
    let agent = match state
        .get_session_agent(Some(payload.session_id.clone()))
        .await
    {
        Ok(agent) => agent,
        Err(e) => {
            tracing::error!("Failed to get session agent: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    agent.add_sub_recipes(payload.sub_recipes.clone()).await;
    Ok(Json(AddSubRecipesResponse { success: true }))
}

#[utoipa::path(
    post,
    path = "/agent/prompt",
    request_body = ExtendPromptRequest,
    responses(
        (status = 200, description = "Extended system prompt successfully", body = ExtendPromptResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
    ),
)]
async fn extend_prompt(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ExtendPromptRequest>,
) -> Result<Json<ExtendPromptResponse>, StatusCode> {
    let agent = match state
        .get_session_agent(Some(payload.session_id.clone()))
        .await
    {
        Ok(agent) => agent,
        Err(e) => {
            tracing::error!("Failed to get session agent: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    agent.extend_system_prompt(payload.extension.clone()).await;
    Ok(Json(ExtendPromptResponse { success: true }))
}

#[utoipa::path(
    get,
    path = "/agent/tools",
    params(
        ("extension_name" = Option<String>, Query, description = "Optional extension name to filter tools"),
        ("session_id" = String, Query, description = "Required session ID to scope tools to a specific session")
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
    Query(query): Query<GetToolsQuery>,
) -> Result<Json<Vec<ToolInfo>>, StatusCode> {
    let config = Config::global();
    let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
    let agent = match state
        .get_session_agent(Some(query.session_id.clone()))
        .await
    {
        Ok(agent) => agent,
        Err(e) => {
            tracing::error!("Failed to get session agent: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
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
    request_body = UpdateProviderRequest,
    responses(
        (status = 200, description = "Provider updated successfully"),
        (status = 400, description = "Bad request - missing or invalid parameters"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_agent_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateProviderRequest>,
) -> Result<StatusCode, impl IntoResponse> {
    let agent = match state
        .get_session_agent(Some(payload.session_id.clone()))
        .await
    {
        Ok(agent) => agent,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session agent: {}", e),
            ))
        }
    };
    let config = Config::global();
    let model = match payload
        .model
        .or_else(|| config.get_param("GOOSE_MODEL").ok())
    {
        Some(m) => m,
        None => return Err((StatusCode::BAD_REQUEST, "No model specified".to_string())),
    };

    let model_config = ModelConfig::new(&model).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid model config: {}", e),
        )
    })?;

    let new_provider = create(&payload.provider, model_config).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to create provider: {}", e),
        )
    })?;

    agent
        .update_provider(new_provider)
        .await
        .map_err(|_e| (StatusCode::INTERNAL_SERVER_ERROR, String::new()))?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/update_router_tool_selector",
    request_body = UpdateRouterToolSelectorRequest,
    responses(
        (status = 200, description = "Tool selection strategy updated successfully", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_router_tool_selector(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRouterToolSelectorRequest>,
) -> Result<Json<String>, Json<ErrorResponse>> {
    let agent = match state
        .get_session_agent(Some(payload.session_id.clone()))
        .await
    {
        Ok(agent) => agent,
        Err(e) => {
            return Err(Json(ErrorResponse {
                error: format!("Failed to get session agent: {}", e),
            }))
        }
    };
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
    request_body = SessionConfigRequest,
    responses(
        (status = 200, description = "Session config updated successfully", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_session_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SessionConfigRequest>,
) -> Result<Json<String>, Json<ErrorResponse>> {
    let agent = match state
        .get_session_agent(Some(payload.session_id.clone()))
        .await
    {
        Ok(agent) => agent,
        Err(e) => {
            return Err(Json(ErrorResponse {
                error: format!("Failed to get session agent: {}", e),
            }))
        }
    };
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
        .route("/agent/start", post(start_agent))
        .route("/agent/resume", post(resume_agent))
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
