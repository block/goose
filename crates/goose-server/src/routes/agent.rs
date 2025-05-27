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
use goose::prompt_template::render_inline_once;
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
    recipe_config: Option<RecipeWithParams>,
}

#[derive(Deserialize)]
struct RecipeWithParams {
    config: goose::recipe::Recipe,
    parameters: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct GetToolsQuery {
    extension_name: Option<String>,
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
    verify_secret_key(&headers, &state)?;

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    // Process recipe parameters if provided
    if let Some(recipe_with_params) = payload.recipe_config {
        let mut recipe = recipe_with_params.config;
        
        if !recipe_with_params.parameters.is_empty() {
            apply_recipe_parameters(&mut recipe, &recipe_with_params.parameters)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
        }
        
        if let Some(instructions) = recipe.instructions.clone() {
            agent.extend_system_prompt(instructions).await;
        }
    }

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

fn apply_recipe_parameters(recipe: &mut goose::recipe::Recipe, params: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    // Serialize the recipe to JSON string
    let recipe_json = serde_json::to_string(recipe)?;
    
    // Render the entire JSON as a template
    let rendered_json = render_inline_once(&recipe_json, params)?;
    
    // Parse the rendered JSON back to a Recipe
    let rendered_recipe: goose::recipe::Recipe = serde_json::from_str(&rendered_json)?;
    
    // Update the original recipe with the rendered values
    *recipe = rendered_recipe;
    
    Ok(())
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/agent/versions", get(get_versions))
        .route("/agent/providers", get(list_providers))
        .route("/agent/prompt", post(extend_prompt))
        .route("/agent/tools", get(get_tools))
        .route("/agent/update_provider", post(update_agent_provider))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::recipe::Recipe;

    #[test]
    fn test_apply_recipe_parameters() {
        let mut recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "A test recipe".to_string(),
            instructions: Some("Hello {{ name }}, welcome to {{ location }}!".to_string()),
            prompt: Some("Your task is {{ task }}".to_string()),
            activities: Some(vec![
                "Learn about {{ topic }}".to_string(),
                "Practice {{ skill }}".to_string(),
            ]),
            context: Some(vec![
                "Context: {{ context_info }}".to_string(),
            ]),
            extensions: None,
            author: None,
            parameters: None,
        };

        let mut params = HashMap::new();
        params.insert("name".to_string(), "Alice".to_string());
        params.insert("location".to_string(), "Wonderland".to_string());
        params.insert("task".to_string(), "solve puzzles".to_string());
        params.insert("topic".to_string(), "logic".to_string());
        params.insert("skill".to_string(), "critical thinking".to_string());
        params.insert("context_info".to_string(), "fantasy world".to_string());

        let result = apply_recipe_parameters(&mut recipe, &params);
        assert!(result.is_ok());

        assert_eq!(recipe.instructions, Some("Hello Alice, welcome to Wonderland!".to_string()));
        assert_eq!(recipe.prompt, Some("Your task is solve puzzles".to_string()));
        assert_eq!(recipe.activities, Some(vec![
            "Learn about logic".to_string(),
            "Practice critical thinking".to_string(),
        ]));
        assert_eq!(recipe.context, Some(vec![
            "Context: fantasy world".to_string(),
        ]));
    }

    #[test]
    fn test_apply_recipe_parameters_empty_fields() {
        let mut recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "A test recipe".to_string(),
            instructions: None,
            prompt: None,
            activities: None,
            context: None,
            extensions: None,
            author: None,
            parameters: None,
        };

        let mut params = HashMap::new();
        params.insert("unused".to_string(), "value".to_string());

        let result = apply_recipe_parameters(&mut recipe, &params);
        assert!(result.is_ok());

        assert_eq!(recipe.instructions, None);
        assert_eq!(recipe.prompt, None);
        assert_eq!(recipe.activities, None);
        assert_eq!(recipe.context, None);
    }

    #[test]
    fn test_apply_recipe_parameters_handles_all_fields() {
        let mut recipe = Recipe {
            version: "{{ version }}".to_string(),
            title: "{{ title }}".to_string(),
            description: "{{ description }}".to_string(),
            instructions: Some("{{ instructions }}".to_string()),
            prompt: Some("{{ prompt }}".to_string()),
            activities: Some(vec!["{{ activity }}".to_string()]),
            context: Some(vec!["{{ context }}".to_string()]),
            extensions: None,
            author: None,
            parameters: None,
        };

        let mut params = HashMap::new();
        params.insert("version".to_string(), "2.0.0".to_string());
        params.insert("title".to_string(), "Rendered Title".to_string());
        params.insert("description".to_string(), "Rendered Description".to_string());
        params.insert("instructions".to_string(), "Rendered Instructions".to_string());
        params.insert("prompt".to_string(), "Rendered Prompt".to_string());
        params.insert("activity".to_string(), "Rendered Activity".to_string());
        params.insert("context".to_string(), "Rendered Context".to_string());

        let result = apply_recipe_parameters(&mut recipe, &params);
        assert!(result.is_ok());

        // Verify that ALL fields were rendered, including version, title, description
        assert_eq!(recipe.version, "2.0.0");
        assert_eq!(recipe.title, "Rendered Title");
        assert_eq!(recipe.description, "Rendered Description");
        assert_eq!(recipe.instructions, Some("Rendered Instructions".to_string()));
        assert_eq!(recipe.prompt, Some("Rendered Prompt".to_string()));
        assert_eq!(recipe.activities, Some(vec!["Rendered Activity".to_string()]));
        assert_eq!(recipe.context, Some(vec!["Rendered Context".to_string()]));
    }
}
