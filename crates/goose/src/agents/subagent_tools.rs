use crate::agents::extension::ExtensionConfig;
use crate::config::GooseMode;
use crate::recipe::{Recipe, RecipeBuilder};
use anyhow::{anyhow, Result};
use rmcp::model::{Tool, ToolAnnotations};
use serde_json::{json, Value};
use std::sync::Arc;

pub const SUBAGENT_TOOL_NAME: &str = "subagent";

pub fn create_subagent_tool() -> Tool {
    let schema = json!({
        "type": "object",
        "properties": {
            "instructions": {
                "type": "string",
                "description": "The detailed goal and instructions for the subagent. This will be the first message in its new conversation."
            },
            "title": { "type": "string" },
            "description": { "type": "string" },
            "extensions": {
                "type": "array",
                "items": { "type": "string" },
                "description": "List of extension names to enable for the subagent. If omitted, uses parent's extensions. If empty array, no extensions."
            },
            "settings": { "type": "object" },
            "return_last_only": { "type": "boolean", "description": "If true, returns only the last message from the subagent. Default is true (returns only the last message)." }
        },
        "required": ["instructions"]
    });

    Tool::new(
        SUBAGENT_TOOL_NAME,
        "Create a subagent to handle complex, isolated tasks. It starts without prior conversation context. Provide self-contained instructions for its operation.",
        Arc::new(schema.as_object().expect("Schema must be an object").clone())
    ).annotate(ToolAnnotations {
        title: Some("Start Subagent".to_string()),
        read_only_hint: Some(false),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(true),
    })
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

fn process_extensions(
    extensions: &Value,
    _loaded_extensions: &[String],
) -> Option<Vec<ExtensionConfig>> {
    if let Ok(ext_configs) = serde_json::from_value::<Vec<ExtensionConfig>>(extensions.clone()) {
        return Some(ext_configs);
    }

    if let Some(arr) = extensions.as_array() {
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

        return Some(converted_extensions);
    }
    None
}

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
    let instructions = task_param.get("instructions").and_then(|v| v.as_str());
    let prompt = task_param.get("prompt").and_then(|v| v.as_str());

    if instructions.is_none() && prompt.is_none() {
        return Err(anyhow!("Either 'instructions' or 'prompt' is required"));
    }

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

    if let Some(inst) = instructions {
        builder = builder.instructions(inst);
    }
    if let Some(p) = prompt {
        builder = builder.prompt(p);
    }

    if let Some(extensions) = task_param.get("extensions") {
        if let Some(ext_configs) = process_extensions(extensions, loaded_extensions) {
            builder = builder.extensions(ext_configs);
        }
    }

    builder = apply_if_ok(builder, task_param.get("settings"), RecipeBuilder::settings);
    builder = apply_if_ok(builder, task_param.get("response"), RecipeBuilder::response);
    builder = apply_if_ok(builder, task_param.get("retry"), RecipeBuilder::retry);
    builder = apply_if_ok(
        builder,
        task_param.get("parameters"),
        RecipeBuilder::parameters,
    );

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
