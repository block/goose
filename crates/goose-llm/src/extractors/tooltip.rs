use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::Provider;
use crate::providers::databricks::DatabricksProvider;
use crate::providers::errors::ProviderError;
use crate::types::core::{Content, Role};
use anyhow::Result;
use indoc::indoc;
use serde_json::{json, Value};

const TOOLTIP_EXAMPLES: &[&str] = &[
    "analyzing KPIs",
    "detecting anomalies",
    "building artifacts in Buildkite",
    "categorizing issues",
    "checking dependencies",
    "collecting feedback",
    "deploying changes in AWS",
    "drafting report in Google Docs",
    "extracting action items",
    "generating insights",
    "logging issues",
    "monitoring tickets in Zendesk",
    "notifying design team",
    "running integration tests",
    "scanning threads in Figma",
    "sending reminders in Gmail",
    "sending surveys",
    "sharing with stakeholders",
    "summarizing findings",
    "transcribing meeting",
    "tracking resolution",
    "updating status in Linear",
];

fn build_system_prompt() -> String {
    let examples = TOOLTIP_EXAMPLES
        .iter()
        .map(|e| format!("- {}", e))
        .collect::<Vec<_>>()
        .join("\n");

    indoc! {r#"
    You are an assistant that summarizes the recent conversation into a tooltip.
    Given the last two messages, reply with only a short tooltip (up to 4 words)
    describing what is happening now.

    Examples:
    "#}
    .to_string()
        + &examples
}

/// Generates a tooltip summarizing the last two messages in the session,
/// including any tool calls or results.
pub async fn generate_tooltip(messages: &[Message]) -> Result<String, ProviderError> {
    // Need at least two messages to summarize
    if messages.len() < 2 {
        return Err(ProviderError::ExecutionError(
            "Need at least two messages to generate a tooltip".to_string(),
        ));
    }

    // Helper to render a single message's content
    fn render_message(m: &Message) -> String {
        let mut parts = Vec::new();
        for content in m.content.iter() {
            match content {
                MessageContent::Text(text_block) => {
                    let txt = text_block.text.trim();
                    if !txt.is_empty() {
                        parts.push(txt.to_string());
                    }
                }
                MessageContent::ToolRequest(req) => {
                    if let Ok(tool_call) = &req.tool_call {
                        parts.push(format!(
                            "called tool '{}' with args {}",
                            tool_call.name, tool_call.arguments
                        ));
                    } else if let Err(e) = &req.tool_call {
                        parts.push(format!("tool request error: {}", e));
                    }
                }
                MessageContent::ToolResponse(resp) => match &resp.tool_result {
                    Ok(contents) => {
                        let results: Vec<String> = contents
                            .iter()
                            .map(|c| match c {
                                Content::Text(t) => t.text.clone(),
                                Content::Image(_) => "[image]".to_string(),
                            })
                            .collect();
                        parts.push(format!("tool responded with: {}", results.join(" ")));
                    }
                    Err(e) => {
                        parts.push(format!("tool error: {}", e));
                    }
                },
                _ => {} // ignore other variants
            }
        }

        let role = match m.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
        };

        format!("{}: {}", role, parts.join("; "))
    }

    // Take the last two messages (in correct chronological order)
    let rendered: Vec<String> = messages
        .iter()
        .rev()
        .take(2)
        .map(render_message)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let system_prompt = build_system_prompt();

    let user_msg_text = format!(
        "Here are the last two messages:\n{}\n\nTooltip:",
        rendered.join("\n")
    );

    // Instantiate the provider
    let model_cfg = ModelConfig::new("goose-gpt-4-1".to_string()).with_temperature(Some(0.0));
    let provider = DatabricksProvider::from_env(model_cfg)?;

    // Schema wrapping our tooltip string
    let schema = json!({
        "type": "object",
        "properties": {
            "tooltip": { "type": "string" }
        },
        "required": ["tooltip"],
        "additionalProperties": false
    });

    // Call extract
    let user_msg = Message::user().with_text(&user_msg_text);
    let resp = provider
        .extract(&system_prompt, &[user_msg], &schema)
        .await?;

    // Pull out the tooltip field
    let obj = resp
        .data
        .as_object()
        .ok_or_else(|| ProviderError::ResponseParseError("Expected JSON object".into()))?;

    let tooltip = obj
        .get("tooltip")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProviderError::ResponseParseError("Missing or non-string `tooltip` field".into())
        })?
        .to_string();

    Ok(tooltip)
}
