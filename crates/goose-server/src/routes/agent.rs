use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use goose::config::Config;
use goose::config::PermissionManager;
use goose::model::ModelConfig;
use goose::providers::create;
use goose::recipe::Response;
use goose::{
    agents::{extension::ToolInfo, extension_manager::get_parameter_names},
    config::permission::PermissionLevel,
};
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
    settings: Option<SessionSettings>,
}

#[derive(Deserialize)]
struct SessionSettings {
    goose_provider: Option<String>,
    goose_model: Option<String>,
    temperature: Option<f32>,
}

#[derive(Deserialize)]
pub struct GetToolsQuery {
    extension_name: Option<String>,
}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    error: String,
}

async fn authenticate_and_get_agent(
    headers: &HeaderMap,
    state: &Arc<AppState>,
) -> Result<Arc<goose::agents::Agent>, Json<ErrorResponse>> {
    verify_secret_key(headers, state).map_err(|_| {
        Json(ErrorResponse {
            error: "Unauthorized - Invalid or missing API key".to_string(),
        })
    })?;

    state.get_agent().await.map_err(|e| {
        tracing::error!("Failed to get agent: {}", e);
        Json(ErrorResponse {
            error: format!("Failed to get agent: {}", e),
        })
    })
}

fn create_model_config_from_settings(
    settings: Option<SessionSettings>,
) -> Result<Option<(String, ModelConfig)>, Json<ErrorResponse>> {
    let Some(settings) = settings else {
        return Ok(None);
    };

    let has_model = settings.goose_model.is_some();
    let has_provider = settings.goose_provider.is_some();
    if has_model != has_provider {
        return Err(Json(ErrorResponse {
            error: "Both goose_model and goose_provider must be specified together, or neither"
                .to_string(),
        }));
    }

    if settings.goose_provider.is_some()
        || settings.goose_model.is_some()
        || settings.temperature.is_some()
    {
        let config = Config::global();

        let provider_name = settings
            .goose_provider
            .or_else(|| config.get_param("GOOSE_PROVIDER").ok())
            .ok_or_else(|| {
                Json(ErrorResponse {
                    error: "No provider specified".to_string(),
                })
            })?;

        let model_name = settings
            .goose_model
            .or_else(|| config.get_param("GOOSE_MODEL").ok())
            .ok_or_else(|| {
                Json(ErrorResponse {
                    error: "No model specified".to_string(),
                })
            })?;

        let model_config = ModelConfig::new(model_name).with_temperature(settings.temperature);
        Ok(Some((provider_name, model_config)))
    } else {
        Ok(None)
    }
}

async fn get_versions() -> Json<VersionsResponse> {
    let versions = ["goose".to_string()];
    let default_version = "goose".to_string();

    Json(VersionsResponse {
        available_versions: versions.iter().map(|v| v.to_string()).collect(),
        default_version,
    })
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
                &tool.description,
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
        (status = 500, description = "Internal server error")
    )
)]
async fn update_agent_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProviderRequest>,
) -> Result<StatusCode, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    let config = Config::global();
    let model = payload.model.unwrap_or_else(|| {
        config
            .get_param("GOOSE_MODEL")
            .expect("Did not find a model on payload or in env to update provider with")
    });
    let model_config = ModelConfig::new(model);
    let new_provider = create(&payload.provider, model_config).unwrap();
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
    let agent = authenticate_and_get_agent(&headers, &state).await?;

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
    let agent = authenticate_and_get_agent(&headers, &state).await?;

    if let Some(response) = payload.response {
        agent.add_final_output_tool(response).await;
    }

    if let Some((provider_name, model_config)) =
        create_model_config_from_settings(payload.settings)?
    {
        let new_provider = create(&provider_name, model_config).map_err(|e| {
            Json(ErrorResponse {
                error: format!("Failed to create provider: {}", e),
            })
        })?;

        agent.update_provider(new_provider).await.map_err(|e| {
            Json(ErrorResponse {
                error: format!("Failed to update provider: {}", e),
            })
        })?;
    }
    let message = "Session config updated successfully".to_string();
    Ok(Json(message))
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
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_model_config_from_settings_none() {
        let result = create_model_config_from_settings(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_create_model_config_from_settings_model_without_provider() {
        let settings = Some(SessionSettings {
            goose_provider: None,
            goose_model: Some("gpt-4".to_string()),
            temperature: None,
        });

        let result = create_model_config_from_settings(settings);
        assert!(result.is_err());

        if let Err(Json(error)) = result {
            assert!(error
                .error
                .contains("Both goose_model and goose_provider must be specified together"));
        }
    }

    #[test]
    fn test_create_model_config_from_settings_both_specified() {
        let settings = Some(SessionSettings {
            goose_provider: Some("openai".to_string()),
            goose_model: Some("gpt-4".to_string()),
            temperature: Some(0.7),
        });

        let result = create_model_config_from_settings(settings);
        assert!(result.is_ok());

        if let Ok(Some((provider_name, model_config))) = result {
            assert_eq!(provider_name, "openai");
            assert_eq!(model_config.model_name, "gpt-4");
            assert_eq!(model_config.temperature, Some(0.7));
        }
    }

    #[test]
    fn test_create_model_config_from_settings_temperature_only() {
        struct EnvGuard {
            keys: Vec<&'static str>,
        }

        impl Drop for EnvGuard {
            fn drop(&mut self) {
                for key in &self.keys {
                    std::env::remove_var(key);
                }
            }
        }

        let _guard = EnvGuard {
            keys: vec!["GOOSE_PROVIDER", "GOOSE_MODEL"],
        };

        std::env::set_var("GOOSE_PROVIDER", "test_provider");
        std::env::set_var("GOOSE_MODEL", "test_model");

        let settings = Some(SessionSettings {
            goose_provider: None,
            goose_model: None,
            temperature: Some(0.8),
        });

        let result = create_model_config_from_settings(settings);

        assert!(result.is_ok());
        if let Ok(Some((provider_name, model_config))) = result {
            assert_eq!(provider_name, "test_provider");
            assert_eq!(model_config.model_name, "test_model");
            assert_eq!(model_config.temperature, Some(0.8));
        }
    }
}
