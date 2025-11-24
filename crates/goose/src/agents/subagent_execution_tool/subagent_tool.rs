use anyhow::{anyhow, Result};
use rmcp::model::{ErrorCode, ErrorData, Tool, ToolAnnotations};
use rmcp::schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::agents::extension::ExtensionConfig;
use crate::agents::subagent_execution_tool::lib::run_tasks;
use crate::agents::subagent_execution_tool::task_types::{ExecutionMode, Task, TaskPayload};
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::tool_execution::ToolCallResult;
use crate::prompt_template;
use crate::recipe::{Recipe, RecipeBuilder, RecipeParameter, Response, Settings, SubRecipe};
use crate::session::SessionManager;
use tokio_util::sync::CancellationToken;

pub const SUBAGENT_TOOL_NAME: &str = "subagent";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubagentParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<SubagentType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::recipe::recipe_extension_adapter::deserialize_recipe_extensions"
    )]
    pub extensions: Option<Vec<ExtensionConfig>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<RecipeParameter>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_recipes: Option<Vec<SubRecipe>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<Value>,

    #[serde(default = "default_return_last_only")]
    pub return_last_only: bool,
}

fn default_return_last_only() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubagentType {
    Default,
    Investigator,
    Critic,
}

impl SubagentType {
    fn template_path(&self) -> &'static str {
        match self {
            SubagentType::Default => "subagents/default.md",
            SubagentType::Investigator => "subagents/investigator.md",
            SubagentType::Critic => "subagents/critic.md",
        }
    }
}

pub fn create_subagent_tool() -> Tool {
    let schema = schema_for!(SubagentParams);
    let schema_value =
        serde_json::to_value(schema).expect("Failed to serialize SubagentParams schema");
    let input_schema = schema_value
        .as_object()
        .expect("Schema should be an object")
        .clone();

    Tool::new(
        SUBAGENT_TOOL_NAME.to_string(),
        "Delegate a task to an autonomous subagent. Returns the result immediately. 
        Use this tool to offload complex work (research, coding, critique) to a specialized agent. 
        Defaults to 'return_last_only=true' for concise results.
        Accepts standard recipe parameters like 'extensions', 'settings', 'retry', etc.
        Can also be used to execute an existing task by providing 'task_id' (e.g. from a sub-recipe)."
            .to_string(),
        input_schema,
    )
    .annotate(ToolAnnotations {
        title: Some("Run Subagent".to_string()),
        read_only_hint: Some(false),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(true),
    })
}

fn load_system_prompt(
    subagent_type: &SubagentType,
    max_turns: Option<u32>,
    tool_count: usize,
    available_tools: Vec<String>,
    task_instructions: &str,
) -> Option<String> {
    let template_path = subagent_type.template_path();

    let mut context = HashMap::new();
    context.insert(
        "max_turns".to_string(),
        max_turns.unwrap_or(100).to_string(),
    );
    context.insert("tool_count".to_string(), tool_count.to_string());
    context.insert("available_tools".to_string(), available_tools.join(", "));
    context.insert(
        "task_instructions".to_string(),
        task_instructions.to_string(),
    );
    context.insert("subagent_id".to_string(), uuid::Uuid::new_v4().to_string());

    match prompt_template::render_global_file(template_path, &context) {
        Ok(prompt) => {
            tracing::info!("Successfully loaded subagent template: {}", template_path);
            Some(prompt)
        }
        Err(e) => {
            tracing::error!(
                "Failed to load subagent template '{}': {}",
                template_path,
                e
            );
            if !matches!(subagent_type, SubagentType::Default) {
                tracing::warn!(
                    "Failed to load subagent persona '{}': {}. Falling back to default.",
                    template_path,
                    e
                );
                return load_system_prompt(
                    &SubagentType::Default,
                    max_turns,
                    tool_count,
                    available_tools,
                    task_instructions,
                );
            }
            tracing::error!("Failed to load default subagent persona: {}", e);
            None
        }
    }
}

fn apply_if_ok<T, F>(
    builder: crate::recipe::RecipeBuilder,
    val: Option<T>,
    func: F,
) -> crate::recipe::RecipeBuilder
where
    F: FnOnce(crate::recipe::RecipeBuilder, T) -> crate::recipe::RecipeBuilder,
{
    if let Some(v) = val {
        func(builder, v)
    } else {
        builder
    }
}

// Helper for retry which is still Value
fn apply_value_if_ok<T, F>(
    builder: crate::recipe::RecipeBuilder,
    val: Option<&Value>,
    func: F,
) -> crate::recipe::RecipeBuilder
where
    T: for<'de> Deserialize<'de>,
    F: FnOnce(crate::recipe::RecipeBuilder, T) -> crate::recipe::RecipeBuilder,
{
    if let Some(v) = val {
        if let Ok(decoded) = serde_json::from_value::<T>(v.clone()) {
            return func(builder, decoded);
        } else {
            tracing::warn!("Failed to deserialize optional field: {:?}", v);
        }
    }
    builder
}

// Helper for author which is still Value
fn apply_author_if_ok(
    builder: crate::recipe::RecipeBuilder,
    val: Option<Value>,
) -> crate::recipe::RecipeBuilder {
    if let Some(v) = val {
        if let Ok(decoded) = serde_json::from_value::<crate::recipe::Author>(v.clone()) {
            return builder.author(decoded);
        } else {
            tracing::warn!("Failed to deserialize optional field: {:?}", v);
        }
    }
    builder
}

impl TryFrom<SubagentParams> for Recipe {
    type Error = anyhow::Error;

    fn try_from(params: SubagentParams) -> Result<Self, Self::Error> {
        if params.task_id.is_none() && params.instructions.is_none() && params.prompt.is_none() {
            return Err(anyhow!(
                "Either 'instructions' or 'prompt' is required unless 'task_id' is provided"
            ));
        }

        let mut builder = Recipe::builder()
            .version(params.version.unwrap_or_else(|| "1.0.0".to_string()))
            .title(params.title.unwrap_or_else(|| "Dynamic Task".to_string()))
            .description(
                params
                    .description
                    .unwrap_or_else(|| "Inline recipe task".to_string()),
            );

        // Allow building partial recipe if we are just executing a task ID
        if params.task_id.is_some() && params.instructions.is_none() && params.prompt.is_none() {
            builder = builder.instructions("Executing existing task".to_string());
        }

        if let Some(inst) = params.instructions {
            builder = builder.instructions(inst);
        }
        if let Some(p) = params.prompt {
            builder = builder.prompt(p);
        }

        builder = apply_if_ok(builder, params.extensions, RecipeBuilder::extensions);
        builder = apply_if_ok(builder, params.settings, RecipeBuilder::settings);
        builder = apply_if_ok(builder, params.activities, RecipeBuilder::activities);
        builder = apply_author_if_ok(builder, params.author);
        builder = apply_if_ok(builder, params.parameters, RecipeBuilder::parameters);
        builder = apply_if_ok(builder, params.response, RecipeBuilder::response);
        builder = apply_if_ok(builder, params.sub_recipes, RecipeBuilder::sub_recipes);

        // retry is still Value in SubagentParams
        builder = apply_value_if_ok(builder, params.retry.as_ref(), RecipeBuilder::retry);

        let recipe = builder
            .build()
            .map_err(|e| anyhow!("Failed to build recipe: {}", e))?;

        if recipe.check_for_security_warnings() {
            return Err(anyhow!("Recipe contains potentially harmful content"));
        }

        if let Some(ref retry) = recipe.retry {
            retry
                .validate()
                .map_err(|e| anyhow!("Invalid retry config: {}", e))?;
        }

        Ok(recipe)
    }
}

pub async fn run_subagent_tool(
    params: Value,
    tasks_manager: &TasksManager,
    provider: crate::agents::types::SharedProvider,
    session_id: &str,
    working_dir: &std::path::Path,
    extension_configs: Vec<crate::agents::extension::ExtensionConfig>,
    cancellation_token: Option<CancellationToken>,
) -> ToolCallResult {
    // 1. Strict Deserialization
    let parsed_params: SubagentParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Invalid parameters: {}", e)),
                data: None,
            }))
        }
    };

    let return_last_only = parsed_params.return_last_only;

    // 2. Type-Safe Conversion
    let mut recipe = match Recipe::try_from(parsed_params.clone()) {
        Ok(r) => r,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Failed to create recipe: {}", e)),
                data: None,
            }))
        }
    };

    // 3. Persona Application
    // If settings.system_prompt is not explicitly set, use the persona's prompt
    let has_explicit_system_prompt = recipe
        .settings
        .as_ref()
        .and_then(|s| s.system_prompt.as_ref())
        .is_some();

    if !has_explicit_system_prompt {
        let subagent_type = parsed_params
            .subagent_type
            .as_ref()
            .unwrap_or(&SubagentType::Default);

        tracing::info!("Applying subagent type: {:?}", subagent_type);

        let task_instructions = recipe
            .instructions
            .as_deref()
            .or(recipe.prompt.as_deref())
            .unwrap_or("Execute the assigned task");

        let tool_count = extension_configs.len();
        let available_tools: Vec<String> = extension_configs
            .iter()
            .map(|ext| ext.name().to_string())
            .collect();

        let max_turns: Option<u32> = None;
        // TODO: Extract max_turns from settings if available

        if let Some(system_prompt) = load_system_prompt(
            subagent_type,
            max_turns,
            tool_count,
            available_tools,
            task_instructions,
        ) {
            tracing::info!(
                "Applied custom system prompt for subagent type: {:?}",
                subagent_type
            );
            let preview_len = system_prompt
                .char_indices()
                .nth(200)
                .map(|(i, _)| i)
                .unwrap_or(system_prompt.len());
            tracing::debug!("System prompt preview: {}", &system_prompt[..preview_len]);

            let mut settings = recipe.settings.unwrap_or(Settings {
                goose_provider: None,
                goose_model: None,
                temperature: None,
                system_prompt: None,
            });
            settings.system_prompt = Some(system_prompt);
            recipe.settings = Some(settings);
        } else {
            tracing::error!(
                "Failed to load system prompt for subagent type: {:?}",
                subagent_type
            );
        }
    } else {
        tracing::info!("Recipe already has explicit system prompt, not applying subagent type");
    }

    // 4. Execution
    let session = match SessionManager::create_session(
        working_dir.to_path_buf(),
        "Subagent Task".to_string(),
        crate::session::session_manager::SessionType::SubAgent,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(format!("Failed to create session: {}", e)),
                data: None,
            }))
        }
    };

    let task = Task {
        id: session.id.clone(),
        payload: TaskPayload {
            recipe,
            return_last_only,
            sequential_when_repeated: false,
            parameter_values: None,
        },
    };

    tasks_manager.save_tasks(vec![task]).await;

    let provider_guard = provider.lock().await;
    let provider_instance = match provider_guard.as_ref() {
        Some(p) => p.clone(),
        None => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from("Provider not available"),
                data: None,
            }))
        }
    };
    drop(provider_guard);

    let task_config = TaskConfig::new(
        provider_instance,
        session_id,
        working_dir,
        extension_configs,
    );
    run_tasks(
        vec![session.id],
        ExecutionMode::Sequential,
        task_config,
        tasks_manager,
        cancellation_token,
    )
    .await
}
