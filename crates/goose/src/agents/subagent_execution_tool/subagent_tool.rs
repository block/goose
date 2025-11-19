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
use crate::recipe::{Recipe, Settings};
use crate::session::SessionManager;
use tokio_util::sync::CancellationToken;

pub const SUBAGENT_TOOL_NAME: &str = "subagent";

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SubagentParams {
    /// The natural language instructions for the subagent to execute.
    pub instructions: String,

    /// Optional specialized persona/type for the subagent.
    /// Available types: "default", "investigator", "critic".
    /// If omitted, uses "default".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<SubagentType>,

    /// Optional name for the subagent task (for logging/identification).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

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
        Defaults to 'return_last_only=true' for concise results."
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
    // We use an empty context because our system prompts currently don't require variable injection
    // at this stage (or variables are handled later by the agent).
    // If the template needs variables, we'd pass them here.
    let context: HashMap<String, String> = HashMap::new();

    match prompt_template::render_global_file(template_path, &context) {
        Ok(prompt) => Some(prompt),
        Err(e) => {
            // If it's the default that failed, we really can't do much but return None
            // and let the agent rely on its internal default.
            // If a specific type failed (e.g. missing file), we might want to fallback to default.
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

pub async fn run_subagent_tool(
    params: Value,
    tasks_manager: &TasksManager,
    provider: crate::agents::types::SharedProvider,
    session_id: &str,
    working_dir: &std::path::Path,
    extension_configs: Vec<crate::agents::extension::ExtensionConfig>,
    cancellation_token: Option<CancellationToken>,
) -> ToolCallResult {
    let params: SubagentParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Invalid parameters: {}", e)),
                data: None,
            }))
        }
    };

    let subagent_type = params.subagent_type.unwrap_or(SubagentType::Default);
    let system_prompt = load_system_prompt(&subagent_type);

    // Build the recipe
    let mut recipe_builder = Recipe::builder()
        .version("1.0.0")
        .title(params.name.unwrap_or_else(|| "Subagent Task".to_string()))
        .description("Ephemeral subagent task")
        .instructions(params.instructions);

    if let Some(prompt) = system_prompt {
        let settings = Settings {
            system_prompt: Some(prompt),
            goose_provider: None,
            goose_model: None,
            temperature: None,
        };
        recipe_builder = recipe_builder.settings(settings);
    }

    let recipe = match recipe_builder.build() {
        Ok(r) => r,
        Err(e) => {
            return ToolCallResult::from(Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(format!("Failed to build recipe: {}", e)),
                data: None,
            }))
        }
    };

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
            return_last_only: params.return_last_only,
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
