use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{anyhow, Result};
use rmcp::model::{Content, ErrorCode, ErrorData, Tool};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::agents::extension::ExtensionConfig;
use crate::agents::subagent_handler::run_complete_subagent_task;
use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::tool_execution::ToolCallResult;
use crate::config::GooseMode;
use crate::providers;
use crate::recipe::build_recipe::build_recipe_from_template;
use crate::recipe::local_recipes::load_local_recipe_file;
use crate::recipe::{Recipe, SubRecipe};
use crate::session::SessionManager;

pub const SUBAGENT_TOOL_NAME: &str = "subagent";

const SUMMARY_INSTRUCTIONS: &str = r#"
Important: Your parent agent will only receive your final message as a summary of your work.
Make sure your last message provides a comprehensive summary of:
- What you were asked to do
- What actions you took  
- The results or outcomes
- Any important findings or recommendations

Be concise but complete.
"#;

#[derive(Debug, Deserialize)]
pub struct SubagentParams {
    pub instructions: Option<String>,
    pub subrecipe: Option<String>,
    pub parameters: Option<HashMap<String, Value>>,
    pub extensions: Option<Vec<String>>,
    pub settings: Option<SubagentSettings>,
    #[serde(default = "default_summary")]
    pub summary: bool,
}

fn default_summary() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SubagentSettings {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
}

pub fn create_subagent_tool(sub_recipes: &[SubRecipe]) -> Tool {
    let description = build_tool_description(sub_recipes);

    let schema = json!({
        "type": "object",
        "properties": {
            "instructions": {
                "type": "string",
                "description": "Instructions for the subagent. Required for ad-hoc tasks. For predefined tasks, adds additional context."
            },
            "subrecipe": {
                "type": "string",
                "description": "Name of a predefined subrecipe to run."
            },
            "parameters": {
                "type": "object",
                "additionalProperties": true,
                "description": "Parameters for the subrecipe. Only valid when 'subrecipe' is specified."
            },
            "extensions": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Extensions to enable. Omit to inherit all, empty array for none."
            },
            "settings": {
                "type": "object",
                "properties": {
                    "provider": {"type": "string", "description": "Override LLM provider"},
                    "model": {"type": "string", "description": "Override model"},
                    "temperature": {"type": "number", "description": "Override temperature"}
                },
                "description": "Override model/provider settings."
            },
            "summary": {
                "type": "boolean",
                "default": true,
                "description": "If true (default), return only the subagent's final summary."
            }
        }
    });

    Tool::new(
        SUBAGENT_TOOL_NAME,
        description,
        schema.as_object().unwrap().clone(),
    )
}

fn build_tool_description(sub_recipes: &[SubRecipe]) -> String {
    let mut desc = String::from(
        "Delegate a task to a subagent that runs independently with its own context.\n\n\
         Modes:\n\
         1. Ad-hoc: Provide `instructions` for a custom task\n\
         2. Predefined: Provide `subrecipe` name to run a predefined task\n\
         3. Augmented: Provide both `subrecipe` and `instructions` to add context\n\n\
         The subagent has access to the same tools as you by default. \
         Use `extensions` to limit which extensions the subagent can use.\n\n\
         For parallel execution, make multiple `subagent` tool calls in the same message.",
    );

    if !sub_recipes.is_empty() {
        desc.push_str("\n\nAvailable subrecipes:");
        for sr in sub_recipes {
            let params_info = get_subrecipe_params_description(sr);
            desc.push_str(&format!(
                "\n• {} - {}{}",
                sr.name,
                sr.description.as_deref().unwrap_or("No description"),
                if params_info.is_empty() {
                    String::new()
                } else {
                    format!(" (params: {})", params_info)
                }
            ));
        }
    }

    desc
}

fn get_subrecipe_params_description(sub_recipe: &SubRecipe) -> String {
    match load_local_recipe_file(&sub_recipe.path) {
        Ok(recipe_file) => match Recipe::from_content(&recipe_file.content) {
            Ok(recipe) => {
                if let Some(params) = recipe.parameters {
                    params
                        .iter()
                        .filter(|p| {
                            sub_recipe
                                .values
                                .as_ref()
                                .map(|v| !v.contains_key(&p.key))
                                .unwrap_or(true)
                        })
                        .map(|p| {
                            let req = match p.requirement {
                                crate::recipe::RecipeParameterRequirement::Required => "[required]",
                                _ => "[optional]",
                            };
                            format!("{} {}", p.key, req)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    String::new()
                }
            }
            Err(_) => String::new(),
        },
        Err(_) => String::new(),
    }
}

pub async fn handle_subagent_tool(
    params: Value,
    task_config: TaskConfig,
    sub_recipes: &HashMap<String, SubRecipe>,
    loaded_extensions: &[String],
    working_dir: &std::path::Path,
) -> ToolCallResult {
    let params: SubagentParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Invalid parameters: {}", e)),
                data: None,
            }));
        }
    };

    if params.instructions.is_none() && params.subrecipe.is_none() {
        return ToolCallResult::from(Err(ErrorData {
            code: ErrorCode::INVALID_PARAMS,
            message: Cow::from("Must provide 'instructions' or 'subrecipe' (or both)"),
            data: None,
        }));
    }

    if params.parameters.is_some() && params.subrecipe.is_none() {
        return ToolCallResult::from(Err(ErrorData {
            code: ErrorCode::INVALID_PARAMS,
            message: Cow::from("'parameters' can only be used with 'subrecipe'"),
            data: None,
        }));
    }

    let recipe = match build_recipe(&params, sub_recipes, loaded_extensions) {
        Ok(r) => r,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(e.to_string()),
                data: None,
            }));
        }
    };

    let session = match SessionManager::create_session(
        working_dir.to_path_buf(),
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

    let task_config = match apply_settings_overrides(task_config, &params).await {
        Ok(tc) => tc,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(e.to_string()),
                data: None,
            }));
        }
    };

    let result = run_complete_subagent_task(recipe, task_config, params.summary, session.id).await;

    match result {
        Ok(text) => ToolCallResult::from(Ok(vec![Content::text(text)])),
        Err(e) => ToolCallResult::from(Err(ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e.to_string()),
            data: None,
        })),
    }
}

fn build_recipe(
    params: &SubagentParams,
    sub_recipes: &HashMap<String, SubRecipe>,
    loaded_extensions: &[String],
) -> Result<Recipe> {
    let mut recipe = if let Some(subrecipe_name) = &params.subrecipe {
        build_subrecipe(subrecipe_name, params, sub_recipes)?
    } else {
        build_adhoc_recipe(params, loaded_extensions)?
    };

    if params.summary {
        let current = recipe.instructions.unwrap_or_default();
        recipe.instructions = Some(format!("{}\n{}", current, SUMMARY_INSTRUCTIONS));
    }

    Ok(recipe)
}

fn build_subrecipe(
    subrecipe_name: &str,
    params: &SubagentParams,
    sub_recipes: &HashMap<String, SubRecipe>,
) -> Result<Recipe> {
    let sub_recipe = sub_recipes.get(subrecipe_name).ok_or_else(|| {
        let available: Vec<_> = sub_recipes.keys().cloned().collect();
        anyhow!(
            "Unknown subrecipe '{}'. Available: {}",
            subrecipe_name,
            available.join(", ")
        )
    })?;

    let recipe_file = load_local_recipe_file(&sub_recipe.path)
        .map_err(|e| anyhow!("Failed to load subrecipe '{}': {}", subrecipe_name, e))?;

    let mut param_values: Vec<(String, String)> = Vec::new();

    if let Some(values) = &sub_recipe.values {
        for (k, v) in values {
            param_values.push((k.clone(), v.clone()));
        }
    }

    if let Some(provided_params) = &params.parameters {
        for (k, v) in provided_params {
            let value_str = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            param_values.push((k.clone(), value_str));
        }
    }

    let mut recipe = build_recipe_from_template(
        recipe_file.content,
        &recipe_file.parent_dir,
        param_values,
        None::<fn(&str, &str) -> Result<String, anyhow::Error>>,
    )
    .map_err(|e| anyhow!("Failed to build subrecipe: {}", e))?;

    // Merge prompt into instructions so the subagent gets the actual task.
    // The subagent handler uses `instructions` as the user message.
    let mut combined = String::new();

    if let Some(instructions) = &recipe.instructions {
        combined.push_str(instructions);
    }

    if let Some(prompt) = &recipe.prompt {
        if !combined.is_empty() {
            combined.push_str("\n\n");
        }
        combined.push_str(prompt);
    }

    if let Some(extra_instructions) = &params.instructions {
        if !combined.is_empty() {
            combined.push_str("\n\n");
        }
        combined.push_str("Additional context from parent agent:\n");
        combined.push_str(extra_instructions);
    }

    if !combined.is_empty() {
        recipe.instructions = Some(combined);
    }

    Ok(recipe)
}

fn build_adhoc_recipe(params: &SubagentParams, _loaded_extensions: &[String]) -> Result<Recipe> {
    let instructions = params
        .instructions
        .as_ref()
        .ok_or_else(|| anyhow!("Instructions required for ad-hoc task"))?;

    // Build recipe with defaults
    let mut builder = Recipe::builder()
        .version("1.0.0")
        .title("Subagent Task")
        .description("Ad-hoc subagent task")
        .instructions(instructions);

    // Handle extensions if provided
    if let Some(extensions) = &params.extensions {
        if let Some(ext_configs) = process_extensions(extensions) {
            builder = builder.extensions(ext_configs);
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

    Ok(recipe)
}

/// Process extension names into ExtensionConfig objects
fn process_extensions(extensions: &[String]) -> Option<Vec<ExtensionConfig>> {
    // If the array is empty, return an empty Vec (not None)
    // This is important: empty array means "no extensions"
    if extensions.is_empty() {
        return Some(Vec::new());
    }

    let mut converted_extensions = Vec::new();

    for name_str in extensions {
        if let Some(config) = crate::config::get_extension_by_name(name_str) {
            if crate::config::is_extension_enabled(&config.key()) {
                converted_extensions.push(config);
            } else {
                tracing::warn!("Extension '{}' is disabled, skipping", name_str);
            }
        } else {
            tracing::warn!("Extension '{}' not found in configuration", name_str);
        }
    }

    Some(converted_extensions)
}

async fn apply_settings_overrides(
    mut task_config: TaskConfig,
    params: &SubagentParams,
) -> Result<TaskConfig> {
    if let Some(settings) = &params.settings {
        if settings.provider.is_some() || settings.model.is_some() || settings.temperature.is_some()
        {
            let provider_name = settings
                .provider
                .clone()
                .unwrap_or_else(|| task_config.provider.get_name().to_string());

            let mut model_config = task_config.provider.get_model_config();

            if let Some(model) = &settings.model {
                model_config.model_name = model.clone();
            }

            if let Some(temp) = settings.temperature {
                model_config = model_config.with_temperature(Some(temp));
            }

            task_config.provider = providers::create(&provider_name, model_config)
                .await
                .map_err(|e| anyhow!("Failed to create provider '{}': {}", provider_name, e))?;
        }
    }

    if let Some(extension_names) = &params.extensions {
        if extension_names.is_empty() {
            task_config.extensions = Vec::new();
        } else {
            task_config
                .extensions
                .retain(|ext| extension_names.contains(&ext.name()));
        }
    }

    Ok(task_config)
}

pub fn should_enable_subagents(model_name: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        assert_eq!(SUBAGENT_TOOL_NAME, "subagent");
    }

    #[test]
    fn test_create_tool_without_subrecipes() {
        let tool = create_subagent_tool(&[]);
        assert_eq!(tool.name, "subagent");
        assert!(tool.description.as_ref().unwrap().contains("Ad-hoc"));
        assert!(!tool
            .description
            .as_ref()
            .unwrap()
            .contains("Available subrecipes"));
    }

    #[test]
    fn test_create_tool_with_subrecipes() {
        let sub_recipes = vec![SubRecipe {
            name: "test_recipe".to_string(),
            path: "test.yaml".to_string(),
            values: None,
            sequential_when_repeated: false,
            description: Some("A test recipe".to_string()),
        }];

        let tool = create_subagent_tool(&sub_recipes);
        assert!(tool
            .description
            .as_ref()
            .unwrap()
            .contains("Available subrecipes"));
        assert!(tool.description.as_ref().unwrap().contains("test_recipe"));
    }

    #[test]
    fn test_params_deserialization_minimal() {
        let params: SubagentParams = serde_json::from_value(json!({
            "instructions": "Do something"
        }))
        .unwrap();

        assert_eq!(params.instructions, Some("Do something".to_string()));
        assert!(params.subrecipe.is_none());
        assert!(params.summary);
    }

    #[test]
    fn test_params_deserialization_full() {
        let params: SubagentParams = serde_json::from_value(json!({
            "instructions": "Extra context",
            "subrecipe": "my_recipe",
            "parameters": {"key": "value"},
            "extensions": ["developer"],
            "settings": {"model": "gpt-4"},
            "summary": false
        }))
        .unwrap();

        assert_eq!(params.instructions, Some("Extra context".to_string()));
        assert_eq!(params.subrecipe, Some("my_recipe".to_string()));
        assert!(params.parameters.is_some());
        assert_eq!(params.extensions, Some(vec!["developer".to_string()]));
        assert!(!params.summary);
    }

    #[test]
    fn test_default_summary_is_true() {
        let params: SubagentParams = serde_json::from_value(json!({
            "instructions": "test"
        }))
        .unwrap();
        assert!(params.summary);
    }
}
