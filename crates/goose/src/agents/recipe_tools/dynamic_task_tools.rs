// =======================================
// Module: Dynamic Task Tools
// Handles creation of tasks dynamically without sub-recipes
// =======================================
use crate::agents::extension::ExtensionConfig;
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::agents::subagent_execution_tool::{lib::ExecutionMode, task_types::Task};
use crate::agents::tool_execution::ToolCallResult;
use crate::agents::types::RetryConfig;
use crate::recipe::{Recipe, Response, Settings};
use anyhow::{anyhow, Result};
use rmcp::model::{Content, ErrorCode, ErrorData, Tool, ToolAnnotations};
use rmcp::object;
use serde_json::{json, Value};
use std::borrow::Cow;

pub const DYNAMIC_TASK_TOOL_NAME_PREFIX: &str = "dynamic_task__create_task";

pub fn create_dynamic_task_tool() -> Tool {
    Tool::new(
        DYNAMIC_TASK_TOOL_NAME_PREFIX.to_string(),
        "Create tasks with instructions or prompt. For simple tasks, only include the instructions field. Extensions control: omit field = use all current extensions; empty array [] = no extensions; array with names = only those extensions. Specify extensions as shortnames (the prefixes for your tools). Specify return_last_only as true and have your subagent summarize its work in its last message to conserve your own context. Optional: title, description, extensions, settings, retry, response schema, context, activities. Arrays for multiple tasks.".to_string(),
        object!({
            "type": "object",
            "properties": {
                "task_parameters": {
                    "type": "array",
                    "description": "Array of tasks. Each needs 'instructions' OR 'prompt'.",
                    "items": {
                        "type": "object",
                        "properties": {
                            // Required (one of these)
                            "instructions": {
                                "type": "string",
                                "description": "Task instructions"
                            },
                            "prompt": {
                                "type": "string",
                                "description": "Initial prompt"
                            },
                            // Optional - auto-generated if not provided
                            "title": {"type": "string"},
                            "description": {"type": "string"},
                            "extensions": {"type": "array"},
                            "settings": {"type": "object"},
                            "parameters": {"type": "array"},
                            "response": {"type": "object"},
                            "retry": {"type": "object"},
                            "context": {"type": "array"},
                            "activities": {"type": "array"},
                            "return_last_only": {
                                "type": "boolean",
                                "description": "If true, return only the last message from the subagent (default: false, returns full conversation)"
                            }
                        },
                        "anyOf": [
                            {"required": ["instructions"]},
                            {"required": ["prompt"]}
                        ]
                    },
                    "minItems": 1
                },
                "execution_mode": {
                    "type": "string",
                    "enum": ["sequential", "parallel"],
                    "description": "How to execute multiple tasks (default: parallel for multiple tasks, sequential for single task)"
                }
            },
            "required": ["task_parameters"]
        })
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
    loaded_extensions: &[String],
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
                // This is a shortname - check if it's loaded
                if loaded_extensions.contains(&name_str.to_string()) {
                    converted_extensions.push(ExtensionConfig::Builtin {
                        name: name_str.to_string(),
                        display_name: None,
                        description: None,
                        timeout: None,
                        bundled: None,
                        available_tools: vec![],
                    });
                } else {
                    tracing::warn!(
                        "Extension '{}' specified but not loaded, skipping",
                        name_str
                    );
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
    if let Some(settings) = task_param.get("settings") {
        if let Ok(settings_obj) = serde_json::from_value::<Settings>(settings.clone()) {
            builder = builder.settings(settings_obj);
        }
    }

    if let Some(response) = task_param.get("response") {
        if let Ok(response_obj) = serde_json::from_value::<Response>(response.clone()) {
            builder = builder.response(response_obj);
        }
    }

    if let Some(retry) = task_param.get("retry") {
        if let Ok(retry_config) = serde_json::from_value::<RetryConfig>(retry.clone()) {
            builder = builder.retry(retry_config);
        }
    }

    if let Some(context) = task_param.get("context") {
        if let Ok(context_vec) = serde_json::from_value::<Vec<String>>(context.clone()) {
            builder = builder.context(context_vec);
        }
    }

    if let Some(activities) = task_param.get("activities") {
        if let Ok(activities_vec) = serde_json::from_value::<Vec<String>>(activities.clone()) {
            builder = builder.activities(activities_vec);
        }
    }

    if let Some(params) = task_param.get("parameters") {
        if let Ok(params_vec) = serde_json::from_value(params.clone()) {
            builder = builder.parameters(params_vec);
        }
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
        // Check if this is a legacy text_instruction task
        if task_param.get("text_instruction").is_some()
            && task_param.get("instructions").is_none()
            && task_param.get("prompt").is_none()
        {
            // Legacy path for backward compatibility
            let text_instruction = task_param
                .get("text_instruction")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let payload = json!({
                "text_instruction": text_instruction
            });

            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                task_type: "text_instruction".to_string(),
                payload,
            };
            tasks.push(task);
        } else {
            // New inline recipe path
            match task_params_to_inline_recipe(task_param, &loaded_extensions) {
                Ok(recipe) => {
                    let recipe_json = match serde_json::to_value(&recipe) {
                        Ok(json) => json,
                        Err(e) => {
                            return ToolCallResult::from(Err(ErrorData {
                                code: ErrorCode::INTERNAL_ERROR,
                                message: Cow::from(format!("Failed to serialize recipe: {}", e)),
                                data: None,
                            }));
                        }
                    };

                    // Extract return_last_only flag if present
                    let return_last_only = task_param
                        .get("return_last_only")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let task = Task {
                        id: uuid::Uuid::new_v4().to_string(),
                        task_type: "inline_recipe".to_string(),
                        payload: json!({
                            "recipe": recipe_json,
                            "return_last_only": return_last_only
                        }),
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
