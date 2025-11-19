use anyhow::{anyhow, Result};
use rmcp::model::{ErrorCode, ErrorData, Tool, ToolAnnotations};
use rmcp::schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::agents::subagent_execution_tool::lib::run_tasks;
use crate::agents::subagent_execution_tool::task_types::{ExecutionMode, Task, TaskPayload};
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::tool_execution::ToolCallResult;
use crate::prompt_template;
use crate::recipe::{Recipe, RecipeBuilder, Settings};
use crate::session::SessionManager;
use tokio_util::sync::CancellationToken;

pub const SUBAGENT_TOOL_NAME: &str = "subagent";

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SubagentParams {
    /// The natural language instructions for the subagent to execute.
    pub instructions: Option<String>,

    /// The prompt to start the subagent session with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// Optional specialized persona/type for the subagent.
    /// Available types: "default", "investigator", "critic".
    /// If omitted, uses "default".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<SubagentType>,

    /// A short title for the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// A longer description of the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional version of the recipe file format. Defaults to "1.0.0".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// A list of extensions to enable for the subagent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<Value>>,

    /// Settings for the subagent recipe (e.g., provider, model, temperature, system_prompt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Value>,

    /// The activity pills that show up when loading the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<String>>,

    /// Any additional author information for the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Value>,

    /// Any additional parameters for the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Value>>,

    /// Response configuration including JSON schema for the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,

    /// Sub-recipes for the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_recipes: Option<Vec<Value>>,

    /// Retry configuration for the subagent recipe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<Value>,

    /// If true, returns only the final result (default: true).
    /// Set to false if you need the full conversation history for debugging.
    #[serde(default = "default_return_last_only")]
    pub return_last_only: bool,
}

fn default_return_last_only() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubagentType {
    Default,
    Investigator,
    Critic,
    // Add more types here as we create more personas
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
        Accepts standard recipe parameters like 'extensions', 'settings', 'retry', etc."
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

/// Load the system prompt for the given subagent type.
/// If the template fails to load, we fallback to a generic default prompt.
fn load_system_prompt(subagent_type: &SubagentType) -> Option<String> {
    let template_path = subagent_type.template_path();
    let context: HashMap<String, String> = HashMap::new();

    match prompt_template::render_global_file(template_path, &context) {
        Ok(prompt) => Some(prompt),
        Err(e) => {
            if !matches!(subagent_type, SubagentType::Default) {
                tracing::warn!(
                    "Failed to load subagent persona '{}': {}. Falling back to default.",
                    template_path,
                    e
                );
                return load_system_prompt(&SubagentType::Default);
            }
            tracing::error!("Failed to load default subagent persona: {}", e);
            None
        }
    }
}

fn apply_if_ok<T, F>(builder: RecipeBuilder, val: Option<&Value>, func: F) -> RecipeBuilder
where
    T: for<'de> Deserialize<'de>,
    F: FnOnce(RecipeBuilder, T) -> RecipeBuilder,
{
    if let Some(v) = val {
        if let Ok(decoded) = serde_json::from_value::<T>(v.clone()) {
            return func(builder, decoded);
        } else {
            // Log warning?
            tracing::warn!("Failed to deserialize optional field: {:?}", v);
        }
    }
    builder
}

pub fn task_params_to_inline_recipe(
    task_param: &Value,
    // loaded_extensions: &[String], // Removed for simplicity as we process extensions directly
) -> Result<Recipe> {
    // Extract and validate core fields
    let instructions = task_param.get("instructions").and_then(|v| v.as_str());
    let prompt = task_param.get("prompt").and_then(|v| v.as_str());

    if instructions.is_none() && prompt.is_none() {
        return Err(anyhow!("Either 'instructions' or 'prompt' is required"));
    }

    // Build recipe with auto-generated defaults
    let mut builder = Recipe::builder()
        .version("1.0.0")
        .title(
            task_param
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Dynamic Task"),
        )
        .description(
            task_param
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Inline recipe task"),
        );

    // Set instructions/prompt
    if let Some(inst) = instructions {
        builder = builder.instructions(inst);
    }
    if let Some(p) = prompt {
        builder = builder.prompt(p);
    }

    // Handle extensions
    if let Some(extensions) = task_param.get("extensions") {
        if let Ok(ext_configs) = serde_json::from_value::<
            Vec<crate::agents::extension::ExtensionConfig>,
        >(extensions.clone())
        {
            builder = builder.extensions(ext_configs);
        } else {
            tracing::warn!("Failed to parse extensions: {:?}", extensions);
        }
    }

    // Handle other optional fields
    builder = apply_if_ok(builder, task_param.get("settings"), RecipeBuilder::settings);
    builder = apply_if_ok(builder, task_param.get("response"), RecipeBuilder::response);
    builder = apply_if_ok(builder, task_param.get("retry"), RecipeBuilder::retry);
    builder = apply_if_ok(
        builder,
        task_param.get("activities"),
        RecipeBuilder::activities,
    );
    builder = apply_if_ok(
        builder,
        task_param.get("author"), // Add author
        RecipeBuilder::author,
    );
    builder = apply_if_ok(
        builder,
        task_param.get("parameters"),
        RecipeBuilder::parameters,
    );
    builder = apply_if_ok(
        builder,
        task_param.get("sub_recipes"), // Add sub_recipes
        RecipeBuilder::sub_recipes,
    );

    if let Some(version) = task_param.get("version").and_then(|v| v.as_str()) {
        builder = builder.version(version);
    }

    // Build and validate
    let recipe = builder
        .build()
        .map_err(|e| anyhow!("Failed to build recipe: {}", e))?;

    // Security validation
    if recipe.check_for_security_warnings() {
        return Err(anyhow!("Recipe contains potentially harmful content"));
    }

    // Validate retry config if present
    if let Some(ref retry) = recipe.retry {
        retry
            .validate()
            .map_err(|e| anyhow!("Invalid retry config: {}", e))?;
    }

    Ok(recipe)
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
    // Use SubagentParams for validation of known fields and defaults,
    // but reuse the raw `params` Value for recipe construction to avoid JsonSchema bounds.
    let parsed_params: SubagentParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Invalid parameters: {}", e)),
                data: None,
            }))
        }
    };

    let subagent_type = parsed_params.subagent_type.unwrap_or(SubagentType::Default);
    let system_prompt = load_system_prompt(&subagent_type);

    // Construct recipe using the helper function
    let mut recipe = match task_params_to_inline_recipe(&params) {
        Ok(r) => r,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Failed to create recipe: {}", e)),
                data: None,
            }))
        }
    };

    // Apply system prompt from persona if settings/system_prompt is NOT explicitly provided in the params
    // Check if params.settings.system_prompt exists
    let has_explicit_system_prompt = params
        .get("settings")
        .and_then(|s| s.get("system_prompt"))
        .is_some();

    if !has_explicit_system_prompt {
        if let Some(prompt) = system_prompt {
            let mut settings = recipe.settings.unwrap_or(Settings {
                goose_provider: None,
                goose_model: None,
                temperature: None,
                system_prompt: None,
            });
            settings.system_prompt = Some(prompt);
            recipe.settings = Some(settings);
        }
    }

    // Create Session
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

    // Create Task
    let task = Task {
        id: session.id.clone(),
        payload: TaskPayload {
            recipe,
            return_last_only: parsed_params.return_last_only,
            sequential_when_repeated: false,
            parameter_values: None,
        },
    };

    // Save Task
    tasks_manager.save_tasks(vec![task]).await;

    // Prepare TaskConfig
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
    // Execute
    run_tasks(
        vec![session.id],
        ExecutionMode::Sequential,
        task_config,
        tasks_manager,
        cancellation_token,
    )
    .await
}
