// =======================================
// Module: Dynamic Task Tools
// Handles creation of tasks dynamically without sub-recipes
// =======================================
use crate::agents::extension::ExtensionConfig;
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::agents::subagent_execution_tool::{
    lib::ExecutionMode,
    task_types::{Task, TaskPayload},
};
use crate::agents::tool_execution::ToolCallResult;
use crate::config::GooseMode;
use crate::recipe::{Recipe, RecipeBuilder};
use crate::session::SessionManager;
use anyhow::{anyhow, Result};
use rmcp::model::{Content, ErrorCode, ErrorData, Tool, ToolAnnotations};
use rmcp::schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;

pub const DYNAMIC_TASK_TOOL_NAME_PREFIX: &str = "dynamic_task__create_task";

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDynamicTaskParams {
    /// Array of tasks. Each task must have either 'instructions' OR 'prompt' field (at least one is required).
    #[schemars(length(min = 1))]
    pub task_parameters: Vec<TaskParameter>,

    /// How to execute multiple tasks (default: parallel for multiple tasks, sequential for single task)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub execution_mode: Option<ExecutionModeParam>,
}

/// Execution mode for tasks
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionModeParam {
    Sequential,
    Parallel,
}

impl From<ExecutionModeParam> for ExecutionMode {
    fn from(mode: ExecutionModeParam) -> Self {
        match mode {
            ExecutionModeParam::Sequential => ExecutionMode::Sequential,
            ExecutionModeParam::Parallel => ExecutionMode::Parallel,
        }
    }
}

type JsonObject = serde_json::Map<String, Value>;

/// Parameters for a single task
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TaskParameter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<JsonObject>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<JsonObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<JsonObject>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<JsonObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<JsonObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<String>>,

    /// If true, return only the last message from the subagent (default: false, returns full conversation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_last_only: Option<bool>,
}

pub fn should_enabled_subagents(model_name: &str) -> bool {
    let config = crate::config::Config::global();
    let is_autonomous = config.get_goose_mode().unwrap_or(GooseMode::Auto) == GooseMode::Auto;
    if !is_autonomous {
        return false;
    }
    if model_name.starts_with("gemini") {
        return false;
    }
    true
}

pub fn create_dynamic_task_tool() -> Tool {
    let schema = schema_for!(CreateDynamicTaskParams);
    let schema_value =
        serde_json::to_value(schema).expect("Failed to serialize CreateDynamicTaskParams schema");

    let input_schema = schema_value
        .as_object()
        .expect("Schema should be an object")
        .clone();

    Tool::new(
        DYNAMIC_TASK_TOOL_NAME_PREFIX.to_string(),
        "Create tasks with instructions or prompt. For simple tasks, only include the instructions field. Extensions control: omit field = use all current extensions; empty array [] = no extensions; array with names = only those extensions. Specify extensions as shortnames (the prefixes for your tools). Specify return_last_only as true and have your subagent summarize its work in its last message to conserve your own context. Optional: title, description, extensions, settings, retry, response schema, activities. Arrays for multiple tasks.".to_string(),
        input_schema,
    ).annotate(ToolAnnotations {
        title: Some("Create Dynamic Tasks".to_string()),
        read_only_hint: Some(false),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(true),
    })
}

fn process_extensions(
    extensions: &Value,
    _loaded_extensions: &[String],
) -> Option<Vec<ExtensionConfig>> {
    // First try to deserialize as ExtensionConfig array
    if let Ok(ext_configs) = serde_json::from_value::<Vec<ExtensionConfig>>(extensions.clone()) {
        return Some(ext_configs);
    }

    // Try to handle mixed array of strings and objects
    if let Some(arr) = extensions.as_array() {
        // If the array is empty, return an empty Vec (not None)
        // This is important: empty array means "no extensions"
        if arr.is_empty() {
            return Some(Vec::new());
        }

        let mut converted_extensions = Vec::new();

        for ext in arr {
            if let Some(name_str) = ext.as_str() {
                if let Some(config) = crate::config::get_extension_by_name(name_str) {
                    if crate::config::is_extension_enabled(&config.key()) {
                        converted_extensions.push(config);
                    } else {
                        tracing::warn!("Extension '{}' is disabled, skipping", name_str);
                    }
                } else {
                    tracing::warn!("Extension '{}' not found in configuration", name_str);
                }
            } else if let Ok(ext_config) = serde_json::from_value::<ExtensionConfig>(ext.clone()) {
                converted_extensions.push(ext_config);
            }
        }

        // Return the converted extensions even if empty
        // (empty means user explicitly wants no extensions)
        return Some(converted_extensions);
    }
    None
}

// Helper function to apply recipe builder methods if value can be deserialized
fn apply_if_ok<T: serde::de::DeserializeOwned>(
    builder: RecipeBuilder,
    value: Option<&Value>,
    f: impl FnOnce(RecipeBuilder, T) -> RecipeBuilder,
) -> RecipeBuilder {
    match value.and_then(|v| serde_json::from_value(v.clone()).ok()) {
        Some(parsed) => f(builder, parsed),
        None => builder,
    }
}

pub fn task_params_to_inline_recipe(
    task_param: &Value,
    loaded_extensions: &[String],
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
        if let Some(ext_configs) = process_extensions(extensions, loaded_extensions) {
            builder = builder.extensions(ext_configs);
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
        task_param.get("parameters"),
        RecipeBuilder::parameters,
    );

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

fn extract_task_parameters(params: &Value) -> Vec<Value> {
    params
        .get("task_parameters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn create_task_execution_payload(tasks: Vec<Task>, execution_mode: ExecutionMode) -> Value {
    let task_ids: Vec<String> = tasks.iter().map(|task| task.id.clone()).collect();
    json!({
        "task_ids": task_ids,
        "execution_mode": execution_mode
    })
}

pub async fn create_dynamic_task(
    params: Value,
    tasks_manager: &TasksManager,
    loaded_extensions: Vec<String>,
    parent_working_dir: &std::path::Path,
) -> ToolCallResult {
    let task_params_array = extract_task_parameters(&params);

    if task_params_array.is_empty() {
        return ToolCallResult::from(Err(ErrorData {
            code: ErrorCode::INVALID_PARAMS,
            message: Cow::from("task_parameters array cannot be empty"),
            data: None,
        }));
    }

    // Convert each parameter set to inline recipe and create tasks
    let mut tasks = Vec::new();
    for task_param in &task_params_array {
        // All tasks must use the new inline recipe path
        match task_params_to_inline_recipe(task_param, &loaded_extensions) {
            Ok(recipe) => {
                // Extract return_last_only flag if present
                let return_last_only = task_param
                    .get("return_last_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Create a session for this task - use its ID as the task ID
                let session = match SessionManager::create_session(
                    parent_working_dir.to_path_buf(),
                    "Subagent task".to_string(),
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
                        }));
                    }
                };

                let task = Task {
                    id: session.id,
                    payload: TaskPayload {
                        recipe,
                        return_last_only,
                        sequential_when_repeated: false,
                        parameter_values: None,
                    },
                };
                tasks.push(task);
            }
            Err(e) => {
                return ToolCallResult::from(Err(ErrorData {
                    code: ErrorCode::INVALID_PARAMS,
                    message: Cow::from(format!("Invalid task parameters: {}", e)),
                    data: None,
                }));
            }
        }
    }

    let execution_mode = params
        .get("execution_mode")
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "sequential" => ExecutionMode::Sequential,
            "parallel" => ExecutionMode::Parallel,
            _ => ExecutionMode::Parallel,
        })
        .unwrap_or_else(|| {
            if tasks.len() > 1 {
                ExecutionMode::Parallel
            } else {
                ExecutionMode::Sequential
            }
        });

    let task_execution_payload = create_task_execution_payload(tasks.clone(), execution_mode);

    let tasks_json = match serde_json::to_string(&task_execution_payload) {
        Ok(json) => json,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(format!("Failed to serialize task list: {}", e)),
                data: None,
            }))
        }
    };

    tasks_manager.save_tasks(tasks).await;
    ToolCallResult::from(Ok(vec![Content::text(tasks_json)]))
}
