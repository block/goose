use super::base::{ProviderUsage, Usage};
use super::errors::ProviderError;
use crate::conversation::message::{Message, MessageContent};
use rmcp::model::Role;

pub fn is_session_description_request(system: &str) -> bool {
    system.contains("four words or less") || system.contains("4 words or less")
}

pub fn generate_simple_session_description(
    model_name: &str,
    messages: &[Message],
) -> Result<(Message, ProviderUsage), ProviderError> {
    let description = messages
        .iter()
        .find(|m| m.role == Role::User)
        .and_then(|m| {
            m.content.iter().find_map(|c| match c {
                MessageContent::Text(text_content) => Some(&text_content.text),
                _ => None,
            })
        })
        .map(|text| {
            text.split_whitespace()
                .take(4)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "Simple task".to_string());

    tracing::debug!(
        description = %description,
        "Generated simple session description, skipped subprocess"
    );

    let message = Message::new(
        Role::Assistant,
        chrono::Utc::now().timestamp(),
        vec![MessageContent::text(description)],
    );

    Ok((
        message,
        ProviderUsage::new(model_name.to_string(), Usage::default()),
    ))
}
