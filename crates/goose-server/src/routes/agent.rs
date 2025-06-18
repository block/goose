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
use goose::{
    agents::{extension::ToolInfo, extension_manager::get_parameter_names},
    config::permission::PermissionLevel,
};
use mcp_core::{tool::ToolCall, Content};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
pub struct GetToolsQuery {
    extension_name: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct CallToolRequest {
    tool_name: String,
    arguments: Value,
    request_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CallToolResponse {
    request_id: String,
    success: bool,
    result: Option<Vec<Content>>,
    error: Option<String>,
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
    path = "/agent/call_tool",
    request_body = CallToolRequest,
    responses(
        (status = 200, description = "Tool called successfully", body = CallToolResponse),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn call_tool(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<CallToolRequest>,
) -> Result<Json<CallToolResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    // Create a ToolCall from the request
    let tool_call = ToolCall::new(payload.tool_name, payload.arguments);

    // Generate a request ID if not provided
    let request_id = payload.request_id.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("tool_call_{}", timestamp)
    });

    // Dispatch the tool call
    let (returned_id, result) = agent.dispatch_tool_call(tool_call, request_id).await;

    match result {
        Ok(tool_call_result) => {
            // Wait for the result future to complete
            match tool_call_result.result.await {
                Ok(contents) => Ok(Json(CallToolResponse {
                    request_id: returned_id,
                    success: true,
                    result: Some(contents),
                    error: None,
                })),
                Err(tool_error) => Ok(Json(CallToolResponse {
                    request_id: returned_id,
                    success: false,
                    result: None,
                    error: Some(tool_error.to_string()),
                })),
            }
        }
        Err(tool_error) => Ok(Json(CallToolResponse {
            request_id: returned_id,
            success: false,
            result: None,
            error: Some(tool_error.to_string()),
        })),
    }
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

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/agent/versions", get(get_versions))
        .route("/agent/providers", get(list_providers))
        .route("/agent/prompt", post(extend_prompt))
        .route("/agent/tools", get(get_tools))
        .route("/agent/call_tool", post(call_tool))
        .route("/agent/update_provider", post(update_agent_provider))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use goose::{
        agents::Agent,
        model::ModelConfig,
        providers::{
            base::{Provider, ProviderUsage, Usage},
            errors::ProviderError,
        },
    };
    use mcp_core::tool::Tool;
    use tower::ServiceExt;

    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> goose::providers::base::ProviderMetadata {
            goose::providers::base::ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[goose::message::Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(goose::message::Message, ProviderUsage), ProviderError> {
            Ok((
                goose::message::Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    #[tokio::test]
    async fn test_call_tool_endpoint() {
        let mock_model_config = ModelConfig::new("test-model".to_string());
        let mock_provider = Arc::new(MockProvider {
            model_config: mock_model_config,
        });
        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;
        let state = AppState::new(Arc::new(agent), "test-secret".to_string()).await;
        let scheduler_path = goose::scheduler::get_default_scheduler_storage_path()
            .expect("Failed to get default scheduler storage path");
        let scheduler = goose::scheduler_factory::SchedulerFactory::create_legacy(scheduler_path)
            .await
            .unwrap();
        state.set_scheduler(scheduler).await;

        let app = routes(state);

        // Test calling a platform tool (search_available_extensions)
        let request = Request::builder()
            .uri("/agent/call_tool")
            .method("POST")
            .header("content-type", "application/json")
            .header("x-secret-key", "test-secret")
            .body(Body::from(
                serde_json::to_string(&CallToolRequest {
                    tool_name: "platform__search_available_extensions".to_string(),
                    arguments: serde_json::json!({}),
                    request_id: Some("test-request-123".to_string()),
                })
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Parse the response
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let response_json: CallToolResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(response_json.request_id, "test-request-123");
        assert!(response_json.success);
        assert!(response_json.result.is_some());
        assert!(response_json.error.is_none());
    }
}
