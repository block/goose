use crate::routes::config_management::resolve_provider_model_info;
use crate::routes::errors::ErrorResponse;
use crate::routes::recipe_utils::{
    apply_recipe_to_agent, build_recipe_with_parameter_values, load_recipe_by_id, validate_recipe,
};
use crate::state::AppState;
use axum::response::IntoResponse;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use goose::agents::{Container, ExtensionLoadResult};

use goose::agents::ExtensionConfig;
use goose::config::resolve_extensions_for_new_session;
use goose::config::{Config, GooseMode};
use goose::providers::create;
use goose::recipe::Recipe;
use goose::recipe_deeplink;
use goose::session::session_manager::SessionType;
use goose::session::{EnabledExtensionsState, ExtensionState, Session};
use goose::{
    agents::{extension::ToolInfo, extension_manager::get_parameter_names},
    config::permission::PermissionLevel,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::error;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateFromSessionRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateProviderRequest {
    provider: String,
    model: Option<String>,
    session_id: String,
    context_limit: Option<usize>,
    request_params: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateSessionRequest {
    session_id: String,
    goose_mode: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GetToolsQuery {
    extension_name: Option<String>,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StartAgentRequest {
    working_dir: String,
    #[serde(default)]
    recipe: Option<Recipe>,
    #[serde(default)]
    recipe_id: Option<String>,
    #[serde(default)]
    recipe_deeplink: Option<String>,
    #[serde(default)]
    extension_overrides: Option<Vec<ExtensionConfig>>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StopAgentRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RestartAgentRequest {
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkingDirRequest {
    session_id: String,
    working_dir: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResumeAgentRequest {
    session_id: String,
    load_model_and_extensions: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddExtensionRequest {
    session_id: String,
    config: ExtensionConfig,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RemoveExtensionRequest {
    name: String,
    session_id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetContainerRequest {
    session_id: String,
    container_id: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ResumeAgentResponse {
    pub session: Session,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_results: Option<Vec<ExtensionLoadResult>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RestartAgentResponse {
    pub extension_results: Vec<ExtensionLoadResult>,
}

#[utoipa::path(
    post,
    path = "/agent/start",
    request_body = StartAgentRequest,
    responses(
        (status = 200, description = "Agent started successfully", body = Session),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[allow(clippy::too_many_lines)]
async fn start_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartAgentRequest>,
) -> Result<Json<Session>, ErrorResponse> {
    #[cfg(feature = "telemetry")]
    goose::posthog::set_session_context("desktop", false);

    let StartAgentRequest {
        working_dir,
        recipe,
        recipe_id,
        recipe_deeplink,
        extension_overrides,
    } = payload;

    let original_recipe = if let Some(deeplink) = recipe_deeplink {
        match recipe_deeplink::decode(&deeplink) {
            Ok(recipe) => Some(recipe),
            Err(err) => {
                error!("Failed to decode recipe deeplink: {}", err);
                #[cfg(feature = "telemetry")]
                goose::posthog::emit_error("recipe_deeplink_decode_failed", &err.to_string());
                return Err(ErrorResponse {
                    message: err.to_string(),
                    status: StatusCode::BAD_REQUEST,
                });
            }
        }
    } else if let Some(id) = recipe_id {
        match load_recipe_by_id(state.as_ref(), &id).await {
            Ok(recipe) => Some(recipe),
            Err(err) => return Err(err),
        }
    } else {
        recipe
    };

    if let Some(ref recipe) = original_recipe {
        if let Err(err) = validate_recipe(recipe) {
            return Err(ErrorResponse {
                message: err.message,
                status: err.status,
            });
        }
    }

    let name = "New Chat".to_string();

    let manager = state.session_manager();
    let config = Config::global();
    let current_mode = config.get_goose_mode().unwrap_or_default();

    let mut session = manager
        .create_session(
            PathBuf::from(&working_dir),
            name,
            SessionType::User,
            current_mode,
        )
        .await
        .map_err(|err| {
            error!("Failed to create session: {}", err);
            #[cfg(feature = "telemetry")]
            goose::posthog::emit_error("session_create_failed", &err.to_string());
            ErrorResponse {
                message: format!("Failed to create session: {}", err),
                status: StatusCode::BAD_REQUEST,
            }
        })?;

    let recipe_extensions = original_recipe
        .as_ref()
        .and_then(|r| r.extensions.as_deref());
    let has_extension_overrides = extension_overrides.is_some();
    let mut extensions_to_use =
        resolve_extensions_for_new_session(recipe_extensions, extension_overrides);
    if recipe_extensions.is_none() && !has_extension_overrides {
        extensions_to_use.extend(goose::plugins::mcp_servers::enabled_plugin_mcp_servers(
            Some(&PathBuf::from(&working_dir)),
        ));
    }

    let mut extension_data = session.extension_data.clone();
    let extensions_state = EnabledExtensionsState::new(extensions_to_use);
    if let Err(e) = extensions_state.to_extension_data(&mut extension_data) {
        tracing::warn!("Failed to initialize session with extensions: {}", e);
    } else {
        manager
            .update(&session.id)
            .extension_data(extension_data.clone())
            .apply()
            .await
            .map_err(|err| {
                error!("Failed to save initial extension state: {}", err);
                ErrorResponse {
                    message: format!("Failed to save initial extension state: {}", err),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
    }

    if let Some(recipe) = original_recipe {
        let mut update = manager.update(&session.id).recipe(Some(recipe.clone()));

        if let Some(ref settings) = recipe.settings {
            if let Some(ref provider) = settings.goose_provider {
                update = update.provider_name(provider);

                if let Some(ref model) = settings.goose_model {
                    if let Ok(model_config) =
                        goose::model_config::model_config_from_user_config(provider, model)
                    {
                        update = update.model_config(model_config);
                    }
                }
            }
        }

        update.apply().await.map_err(|err| {
            error!("Failed to update session with recipe: {}", err);
            ErrorResponse {
                message: format!("Failed to update session with recipe: {}", err),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
    }

    // Refetch session to get all updates
    session = manager
        .get_session(&session.id, false)
        .await
        .map_err(|err| {
            error!("Failed to get updated session: {}", err);
            ErrorResponse {
                message: format!("Failed to get updated session: {}", err),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    // Eagerly start loading extensions in the background
    let session_for_spawn = session.clone();
    let state_for_spawn = state.clone();
    let session_id_for_task = session.id.clone();
    let task = tokio::spawn(async move {
        match state_for_spawn
            .get_agent(session_for_spawn.id.clone())
            .await
        {
            Ok(agent) => {
                let results = agent.load_extensions_from_session(&session_for_spawn).await;
                tracing::debug!(
                    "Background extension loading completed for session {}",
                    session_for_spawn.id
                );
                results
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create agent for background extension loading: {}",
                    e
                );
                vec![]
            }
        }
    });

    state
        .set_extension_loading_task(session_id_for_task, task)
        .await;

    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/agent/resume",
    request_body = ResumeAgentRequest,
    responses(
        (status = 200, description = "Agent started successfully", body = ResumeAgentResponse),
        (status = 400, description = "Bad request - invalid working directory"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error")
    )
)]
async fn resume_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ResumeAgentRequest>,
) -> Result<Json<ResumeAgentResponse>, ErrorResponse> {
    #[cfg(feature = "telemetry")]
    goose::posthog::set_session_context("desktop", true);

    let session = state
        .session_manager()
        .get_session(&payload.session_id, true)
        .await
        .map_err(|err| {
            error!("Failed to resume session {}: {}", payload.session_id, err);
            #[cfg(feature = "telemetry")]
            goose::posthog::emit_error("session_resume_failed", &err.to_string());
            ErrorResponse {
                message: format!("Failed to resume session: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    let (extension_results, session) = if payload.load_model_and_extensions {
        let agent = state
            .get_agent_for_route(payload.session_id.clone())
            .await
            .map_err(|code| ErrorResponse {
                message: "Failed to get agent for route".into(),
                status: code,
            })?;

        if !state.has_extension_loading_task(&payload.session_id).await {
            let session_for_task = session.clone();
            let agent_for_task = agent.clone();
            let session_id_for_task = payload.session_id.clone();
            let task = tokio::spawn(async move {
                agent_for_task
                    .load_extensions_from_session(&session_for_task)
                    .await
            });
            state
                .set_extension_loading_task(session_id_for_task, task)
                .await;
        }

        let provider_changed = agent
            .restore_provider_from_session(&session)
            .await
            .map_err(|e| ErrorResponse {
                message: e.to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

        let session = if provider_changed {
            state
                .session_manager()
                .get_session(&payload.session_id, true)
                .await
                .map_err(|err| ErrorResponse {
                    message: format!("Failed to re-fetch session: {}", err),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                })?
        } else {
            session
        };

        let extension_results = match state.take_extension_loading_task(&payload.session_id).await {
            Ok(Some(results)) => {
                tracing::debug!(
                    "Using background extension loading results for session {}",
                    payload.session_id
                );
                state
                    .remove_extension_loading_task(&payload.session_id)
                    .await;
                results
            }
            Ok(None) => {
                tracing::debug!(
                    "Extension loading task for session {} was already consumed",
                    payload.session_id
                );
                vec![]
            }
            Err(e) => {
                state
                    .remove_extension_loading_task(&payload.session_id)
                    .await;
                tracing::warn!(
                    "Background extension loading failed for session {}, retrying synchronously: {}",
                    payload.session_id,
                    e
                );
                agent.load_extensions_from_session(&session).await
            }
        };

        (Some(extension_results), session)
    } else {
        (None, session)
    };

    Ok(Json(ResumeAgentResponse {
        session,
        extension_results,
    }))
}

#[utoipa::path(
    post,
    path = "/agent/update_from_session",
    request_body = UpdateFromSessionRequest,
    responses(
        (status = 200, description = "Update agent from session data successfully"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
    ),
)]
async fn update_from_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateFromSessionRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let agent = state
        .get_agent_for_route(payload.session_id.clone())
        .await
        .map_err(|status| ErrorResponse {
            message: format!("Failed to get agent: {}", status),
            status,
        })?;
    let session = state
        .session_manager()
        .get_session(&payload.session_id, false)
        .await
        .map_err(|err| ErrorResponse {
            message: format!("Failed to get session: {}", err),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    if let Some(recipe) = session.recipe {
        if session.session_type == SessionType::Scheduled {
            if let Some(prompt) = apply_recipe_to_agent(&agent, &recipe, true).await {
                agent
                    .extend_system_prompt("recipe".to_string(), prompt)
                    .await;
            }
        } else {
            match build_recipe_with_parameter_values(
                &recipe,
                session.user_recipe_values.unwrap_or_default(),
            )
            .await
            {
                Ok(Some(recipe)) => {
                    if let Some(prompt) = apply_recipe_to_agent(&agent, &recipe, true).await {
                        agent
                            .extend_system_prompt("recipe".to_string(), prompt)
                            .await;
                    }
                }
                Ok(None) => {
                    // Recipe has missing parameters
                }
                Err(e) => {
                    return Err(ErrorResponse {
                        message: e.to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                    });
                }
            }
        }
    }

    Ok(StatusCode::OK)
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
    let session_id = query.session_id;
    let agent = state.get_agent_for_route(session_id.clone()).await?;
    let goose_mode = agent.goose_mode().await;
    let permission_manager = agent.config.permission_manager.clone();

    let mut tools: Vec<ToolInfo> = agent
        .list_tools(&session_id, query.extension_name)
        .await
        .into_iter()
        .map(|tool| {
            let permission = permission_manager
                .get_user_permission(&tool.name)
                .or_else(|| {
                    if goose_mode == GooseMode::SmartApprove {
                        permission_manager.get_smart_approve_permission(&tool.name)
                    } else if goose_mode == GooseMode::Approve {
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
            .with_input_schema(serde_json::Value::Object(
                tool.input_schema.as_ref().clone(),
            ))
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
) -> Result<(), impl IntoResponse> {
    let agent = state
        .get_agent_for_route(payload.session_id.clone())
        .await
        .map_err(|e| (e, "No agent for session id".to_owned()))?;

    let config = Config::global();
    let model = match payload.model.or_else(|| config.get_goose_model().ok()) {
        Some(m) => m,
        None => {
            return Err((StatusCode::BAD_REQUEST, "No model specified".to_owned()));
        }
    };

    let mut model_config =
        goose::model_config::model_config_from_user_config(&payload.provider, &model)
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid model config: {}", e),
                )
            })?
            .with_context_limit(payload.context_limit);

    if let Some(request_params) = payload.request_params {
        model_config = model_config.with_merged_request_params(request_params);
    }
    let model_info = resolve_provider_model_info(&payload.provider, &model)
        .await
        .map_err(|e| (e.status, e.message))?;
    model_config.reasoning = Some(model_info.reasoning);

    let extensions =
        EnabledExtensionsState::for_session(state.session_manager(), &payload.session_id, config)
            .await;

    let new_provider = create(&payload.provider, extensions).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to create {} provider: {}", &payload.provider, e),
        )
    })?;

    agent
        .update_provider(new_provider, model_config, &payload.session_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update provider: {}", e),
            )
        })?;

    // Propagate session mode to the new provider
    let mode = agent.goose_mode().await;
    agent
        .update_goose_mode(mode, &payload.session_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to propagate mode to provider: {}", e),
            )
        })?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/agent/update_session",
    request_body = UpdateSessionRequest,
    responses(
        (status = 200, description = "Session updated"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal error")
    )
)]
async fn update_session(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateSessionRequest>,
) -> Result<(), (StatusCode, String)> {
    let agent = state
        .get_agent_for_route(payload.session_id.clone())
        .await
        .map_err(|e| (e, "No agent for session id".to_owned()))?;

    if let Some(mode_str) = payload.goose_mode {
        let mode: GooseMode = mode_str.parse().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid mode: {}", mode_str),
            )
        })?;

        agent
            .update_goose_mode(mode, &payload.session_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update mode: {}", e),
                )
            })?;
    }

    Ok(())
}

#[utoipa::path(
    post,
    path = "/agent/add_extension",
    request_body = AddExtensionRequest,
    responses(
        (status = 200, description = "Extension added", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn agent_add_extension(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AddExtensionRequest>,
) -> Result<StatusCode, ErrorResponse> {
    #[cfg(feature = "telemetry")]
    let extension_name = request.config.name();

    ensure_extensions_loaded(&state, &request.session_id).await?;

    let agent = state.get_agent(request.session_id.clone()).await?;

    agent
        .add_extension(request.config, &request.session_id)
        .await
        .map_err(|e| {
            #[cfg(feature = "telemetry")]
            goose::posthog::emit_error(
                "extension_add_failed",
                &format!("{}: {}", extension_name, e),
            );
            ErrorResponse::internal(format!("Failed to add extension: {}", e))
        })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/remove_extension",
    request_body = RemoveExtensionRequest,
    responses(
        (status = 200, description = "Extension removed", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn agent_remove_extension(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RemoveExtensionRequest>,
) -> Result<StatusCode, ErrorResponse> {
    ensure_extensions_loaded(&state, &request.session_id).await?;

    let agent = state.get_agent(request.session_id.clone()).await?;

    agent
        .remove_extension(&request.name, &request.session_id)
        .await
        .map_err(|e| {
            error!("Failed to remove extension: {}", e);
            ErrorResponse {
                message: format!("Failed to remove extension: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/set_container",
    request_body = SetContainerRequest,
    responses(
        (status = 200, description = "Container set successfully"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn set_container(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SetContainerRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let agent = state.get_agent(request.session_id.clone()).await?;

    let container = request.container_id.map(Container::new);
    agent.set_container(container).await;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/agent/stop",
    request_body = StopAgentRequest,
    responses(
        (status = 200, description = "Agent stopped successfully", body = String),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn stop_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StopAgentRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let session_id = payload.session_id;
    state
        .agent_manager
        .remove_session(&session_id)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to stop agent for session {}: {}", session_id, e),
            status: StatusCode::NOT_FOUND,
        })?;

    Ok(StatusCode::OK)
}

async fn restart_agent_internal(
    state: &Arc<AppState>,
    session_id: &str,
    session: &Session,
) -> Result<Vec<ExtensionLoadResult>, ErrorResponse> {
    state
        .agent_manager
        .remove_session_if_loaded(session_id)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to remove in-memory agent for session {session_id}: {e}"),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let agent = state
        .get_agent_for_route(session_id.to_string())
        .await
        .map_err(|code| ErrorResponse {
            message: "Failed to create new agent during restart".into(),
            status: code,
        })?;

    let provider_future = agent.restore_provider_from_session(session);
    let extensions_future = agent.load_extensions_from_session(session);

    let (provider_result, extension_results) = tokio::join!(provider_future, extensions_future);
    provider_result.map_err(|e| ErrorResponse {
        message: e.to_string(),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    if let Some(ref recipe) = session.recipe {
        if session.session_type == SessionType::Scheduled {
            if let Some(prompt) = apply_recipe_to_agent(&agent, recipe, true).await {
                agent
                    .extend_system_prompt("recipe".to_string(), prompt)
                    .await;
            }
        } else {
            match build_recipe_with_parameter_values(
                recipe,
                session.user_recipe_values.clone().unwrap_or_default(),
            )
            .await
            {
                Ok(Some(recipe)) => {
                    if let Some(prompt) = apply_recipe_to_agent(&agent, &recipe, true).await {
                        agent
                            .extend_system_prompt("recipe".to_string(), prompt)
                            .await;
                    }
                }
                Ok(None) => {
                    // Recipe has missing parameters
                }
                Err(e) => {
                    return Err(ErrorResponse {
                        message: e.to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                    });
                }
            }
        }
    }

    Ok(extension_results)
}

#[utoipa::path(
    post,
    path = "/agent/restart",
    request_body = RestartAgentRequest,
    responses(
        (status = 200, description = "Agent restarted successfully", body = RestartAgentResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn restart_agent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RestartAgentRequest>,
) -> Result<Json<RestartAgentResponse>, ErrorResponse> {
    let session_id = payload.session_id.clone();

    let session = state
        .session_manager()
        .get_session(&session_id, false)
        .await
        .map_err(|err| {
            error!("Failed to get session during restart: {}", err);
            ErrorResponse {
                message: format!("Failed to get session: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    let extension_results = restart_agent_internal(&state, &session_id, &session).await?;

    Ok(Json(RestartAgentResponse { extension_results }))
}

#[utoipa::path(
    post,
    path = "/agent/update_working_dir",
    request_body = UpdateWorkingDirRequest,
    responses(
        (status = 200, description = "Working directory updated and agent restarted successfully"),
        (status = 400, description = "Bad request - invalid directory path"),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_working_dir(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateWorkingDirRequest>,
) -> Result<StatusCode, ErrorResponse> {
    let session_id = payload.session_id.clone();
    let working_dir = payload.working_dir.trim();

    if working_dir.is_empty() {
        return Err(ErrorResponse {
            message: "Working directory cannot be empty".into(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let path = PathBuf::from(working_dir);
    if !path.exists() || !path.is_dir() {
        return Err(ErrorResponse {
            message: "Invalid directory path".into(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    // Update the session's working directory
    state
        .session_manager()
        .update(&session_id)
        .working_dir(path)
        .apply()
        .await
        .map_err(|e| {
            error!("Failed to update session working directory: {}", e);
            ErrorResponse {
                message: format!("Failed to update working directory: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    // Get the updated session and restart the agent
    let session = state
        .session_manager()
        .get_session(&session_id, false)
        .await
        .map_err(|err| {
            error!("Failed to get session after working dir update: {}", err);
            ErrorResponse {
                message: format!("Failed to get session: {}", err),
                status: StatusCode::NOT_FOUND,
            }
        })?;

    restart_agent_internal(&state, &session_id, &session).await?;

    Ok(StatusCode::OK)
}

async fn ensure_extensions_loaded(state: &AppState, session_id: &str) -> Result<(), ErrorResponse> {
    match state.take_extension_loading_task(session_id).await {
        Ok(Some(_)) => {
            tracing::debug!(
                "Awaited background extension loading for session {} before serving request",
                session_id
            );
            state.remove_extension_loading_task(session_id).await;
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(e) => {
            state.remove_extension_loading_task(session_id).await;
            tracing::warn!(
                "Background extension loading failed for session {}, retrying synchronously: {}",
                session_id,
                e
            );
            let session = state
                .session_manager()
                .get_session(session_id, false)
                .await
                .map_err(|err| ErrorResponse {
                    message: format!(
                        "Failed to get session after extension loading failed: {}",
                        err
                    ),
                    status: StatusCode::NOT_FOUND,
                })?;
            let agent = state
                .get_agent(session_id.to_string())
                .await
                .map_err(|err| {
                    ErrorResponse::internal(format!(
                        "Failed to get agent after extension loading failed: {}",
                        err
                    ))
                })?;
            agent.load_extensions_from_session(&session).await;
            Ok(())
        }
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/agent/start", post(start_agent))
        .route("/agent/resume", post(resume_agent))
        .route("/agent/restart", post(restart_agent))
        .route("/agent/update_working_dir", post(update_working_dir))
        .route("/agent/tools", get(get_tools))
        .route("/agent/update_provider", post(update_agent_provider))
        .route("/agent/update_session", post(update_session))
        .route("/agent/update_from_session", post(update_from_session))
        .route("/agent/add_extension", post(agent_add_extension))
        .route("/agent/remove_extension", post(agent_remove_extension))
        .route("/agent/set_container", post(set_container))
        .route("/agent/stop", post(stop_agent))
        .with_state(state)
}
