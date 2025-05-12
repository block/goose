use crate::model::ModelConfig;
use crate::providers::create;
use crate::providers::errors::ProviderError;
use crate::types::core::Role;
use crate::{message::Message, types::json_value_ffi::JsonValueFfi};
use anyhow::Result;
use indoc::indoc;
use serde_json::{json, Value};

const SESSION_NAME_EXAMPLES: &[&str] = &[
    "Research Synthesis",
    "Sentiment Analysis",
    "Performance Report",
    "Feedback Collector",
    "Accessibility Check",
    "Design Reminder",
    "Project Reminder",
    "Launch Checklist",
    "Metrics Monitor",
    "Incident Response",
    "Deploy Cabinet App",
    "Design Reminder Alert",
    "Generate Monthly Expense Report",
    "Automate Incident Response Workflow",
    "Analyze Brand Sentiment Trends",
    "Monitor Device Health Issues",
    "Collect UI Feedback Summary",
    "Schedule Project Deadline Reminders",
];

fn build_system_prompt() -> String {
    let examples = SESSION_NAME_EXAMPLES
        .iter()
        .map(|e| format!("- {}", e))
        .collect::<Vec<_>>()
        .join("\n");

    indoc! {r#"
    You are an assistant that crafts a concise session title.
    Given the first couple user messages in the conversation so far, 
    reply with only a short name (up to 4 words) that best describes 
    this session’s goal.

    Examples:
    "#}
    .to_string()
        + &examples
}

/// Generates a short (≤4 words) session name
#[uniffi::export(async_runtime = "tokio")]
pub async fn generate_session_name(
    provider_name: &str,
    provider_config: JsonValueFfi,
    messages: &[Message],
) -> Result<String, ProviderError> {
    // Use OpenAI models specifically for this task
    let model_name = if provider_name == "databricks" {
        "goose-gpt-4-1"
    } else {
        "gpt-4.1"
    };
    let model_cfg = ModelConfig::new(model_name.to_string()).with_temperature(Some(0.0));
    let provider = create(provider_name, provider_config.into(), model_cfg)?;

    // Collect up to the first 3 user messages (truncated to 300 chars each)
    let context: Vec<String> = messages
        .iter()
        .filter(|m| m.role == Role::User)
        .take(3)
        .map(|m| {
            let text = m.content.concat_text_str();
            if text.len() > 300 {
                text.chars().take(300).collect()
            } else {
                text
            }
        })
        .collect();

    if context.is_empty() {
        return Err(ProviderError::ExecutionError(
            "No user messages found to generate a session name.".to_string(),
        ));
    }

    let system_prompt = build_system_prompt();
    let user_msg_text = format!("Here are the user messages:\n{}", context.join("\n"));

    // Use `extract` with a simple string schema
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"],
        "additionalProperties": false
    });
    let user_msg = Message::user().with_text(&user_msg_text);
    let resp = provider
        .extract(&system_prompt, &[user_msg], &schema)
        .await?;

    let obj = resp
        .data
        .as_object()
        .ok_or_else(|| ProviderError::ResponseParseError("Expected object".into()))?;

    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::ResponseParseError("Missing or non-string name".into()))?
        .to_string();

    Ok(name)
}
