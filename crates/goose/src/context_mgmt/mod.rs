use crate::conversation::message::MessageMetadata;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::prompt_template::render_global_file;
use crate::providers::base::{Provider, ProviderUsage};
use crate::{agents::Agent, config::Config, token_counter::create_token_counter};
use anyhow::Result;
use rmcp::model::Role;
use serde::Serialize;
use std::sync::Arc;
use tracing::{debug, info};

pub const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;

#[derive(Serialize)]
struct SummarizeContext {
    messages: String,
}

/// Compact messages by summarizing them
///
/// This function performs the actual compaction by summarizing messages and updating
/// their visibility metadata. It does not check thresholds - use `check_if_compaction_needed`
/// first to determine if compaction is necessary.
///
/// # Arguments
/// * `agent` - The agent to use for context management
/// * `conversation` - The current conversation history
/// * `preserve_last_user_message` - If true and last message is not a user message, copy the most recent user message to the end
///
/// # Returns
/// * A tuple containing:
///   - `Conversation`: The compacted messages
///   - `Vec<usize>`: Token counts for each message
///   - `Option<ProviderUsage>`: Provider usage from summarization
pub async fn compact_messages(
    agent: &Agent,
    conversation: &Conversation,
    preserve_last_user_message: bool,
) -> Result<(Conversation, Vec<usize>, Option<ProviderUsage>)> {
    info!("Performing message compaction");

    let messages = conversation.messages();

    let has_text_only = |msg: &Message| {
        let has_text = msg
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::Text(_)));
        let has_tool_content = msg.content.iter().any(|c| {
            matches!(
                c,
                MessageContent::ToolRequest(_) | MessageContent::ToolResponse(_)
            )
        });
        has_text && !has_tool_content
    };

    // Helper function to extract text content from a message
    let extract_text = |msg: &Message| -> Option<String> {
        let text_parts: Vec<String> = msg
            .content
            .iter()
            .filter_map(|c| {
                if let MessageContent::Text(text) = c {
                    Some(text.text.clone())
                } else {
                    None
                }
            })
            .collect();

        if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join("\n"))
        }
    };

    // Check if the most recent message is a user message with text content only
    let (messages_to_compact, preserved_user_text) = if let Some(last_message) = messages.last() {
        if matches!(last_message.role, rmcp::model::Role::User) && has_text_only(last_message) {
            // Remove the last user message before compaction and preserve its text
            (&messages[..messages.len() - 1], extract_text(last_message))
        } else if preserve_last_user_message {
            // Last message is not a user message with text only, but we want to preserve the most recent user message with text only
            // Find the most recent user message with text content only and extract its text
            let preserved_text = messages
                .iter()
                .rev()
                .find(|msg| matches!(msg.role, rmcp::model::Role::User) && has_text_only(msg))
                .and_then(extract_text);
            (messages.as_slice(), preserved_text)
        } else {
            (messages.as_slice(), None)
        }
    } else {
        (messages.as_slice(), None)
    };

    let provider = agent.provider().await?;
    let summary = do_compact(provider.clone(), messages_to_compact).await?;

    let (summary_message, summarization_usage) = match summary {
        Some((summary_message, provider_usage)) => (summary_message, Some(provider_usage)),
        None => {
            // No summary was generated (empty input)
            tracing::warn!("Summarization failed. Returning empty messages.");
            return Ok((Conversation::empty(), vec![], None));
        }
    };

    // Create the final message list with updated visibility metadata:
    // 1. Original messages become user_visible but not agent_visible
    // 2. Summary message becomes agent_visible but not user_visible
    // 3. Assistant messages to continue the conversation remain both user_visible and agent_visible

    let mut final_messages = Vec::new();
    let mut final_token_counts = Vec::new();

    // Add all original messages with updated visibility (preserve user_visible, set agent_visible=false)
    for msg in messages_to_compact.iter().cloned() {
        let updated_metadata = msg.metadata.with_agent_invisible();
        let updated_msg = msg.with_metadata(updated_metadata);
        final_messages.push(updated_msg);
        // Token count doesn't matter for agent_visible=false messages, but we'll use 0
        final_token_counts.push(0);
    }

    // Add the summary message (agent_visible=true, user_visible=false)
    let summary_msg = summary_message.with_metadata(MessageMetadata::agent_only());
    // For token counting purposes, we use the output tokens (the actual summary content)
    // since that's what will be in the context going forward
    let summary_tokens = summarization_usage
        .as_ref()
        .and_then(|usage| usage.usage.output_tokens)
        .unwrap_or(0) as usize;
    final_messages.push(summary_msg);
    final_token_counts.push(summary_tokens);

    // Add an assistant message to continue the conversation (agent_visible=true, user_visible=false)
    let assistant_message = Message::assistant()
        .with_text(
            "The previous message contains a summary that was prepared because a context limit was reached.
Do not mention that you read a summary or that conversation summarization occurred
Just continue the conversation naturally based on the summarized context"
        )
        .with_metadata(MessageMetadata::agent_only());
    let assistant_message_tokens: usize = 0; // Not counted since it's for agent context only
    final_messages.push(assistant_message);
    final_token_counts.push(assistant_message_tokens);

    // Add back the preserved user message if it exists
    if let Some(user_text) = preserved_user_text {
        final_messages.push(Message::user().with_text(&user_text));
    }

    Ok((
        Conversation::new_unvalidated(final_messages),
        final_token_counts,
        summarization_usage,
    ))
}

/// Check if messages exceed the auto-compaction threshold
pub async fn check_if_compaction_needed(
    agent: &Agent,
    conversation: &Conversation,
    threshold_override: Option<f64>,
    session_metadata: Option<&crate::session::Session>,
) -> Result<bool> {
    let messages = conversation.messages();
    let config = Config::global();
    // TODO(Douwe): check the default here; it seems to reset to 0.3 sometimes
    let threshold = threshold_override.unwrap_or_else(|| {
        config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD)
    });

    let provider = agent.provider().await?;
    let context_limit = provider.get_model_config().context_limit();

    let (current_tokens, token_source) = match session_metadata.and_then(|m| m.total_tokens) {
        Some(tokens) => (tokens as usize, "session metadata"),
        None => {
            let token_counter = create_token_counter()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;

            let token_counts: Vec<_> = messages
                .iter()
                .filter(|m| m.is_agent_visible())
                .map(|msg| token_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]))
                .collect();

            (token_counts.iter().sum(), "estimated")
        }
    };

    let usage_ratio = current_tokens as f64 / context_limit as f64;

    let needs_compaction = if threshold <= 0.0 || threshold >= 1.0 {
        false // Auto-compact is disabled.
    } else {
        usage_ratio > threshold
    };

    debug!(
        "Compaction check: {} / {} tokens ({:.1}%), threshold: {:.1}%, needs compaction: {}, source: {}",
        current_tokens,
        context_limit,
        usage_ratio * 100.0,
        threshold * 100.0,
        needs_compaction,
        token_source
    );

    Ok(needs_compaction)
}

async fn do_compact(
    provider: Arc<dyn Provider>,
    messages: &[Message],
) -> Result<Option<(Message, ProviderUsage)>, anyhow::Error> {
    let agent_visible_messages: Vec<&Message> = messages
        .iter()
        .filter(|msg| msg.is_agent_visible())
        .collect();

    let messages_text = agent_visible_messages
        .iter()
        .map(|&msg| format_message_for_compacting(msg))
        .collect::<Vec<_>>()
        .join("\n");

    let context = SummarizeContext {
        messages: messages_text,
    };

    let system_prompt = render_global_file("summarize_oneshot.md", &context)?;

    let user_message = Message::user()
        .with_text("Please summarize the conversation history provided in the system prompt.");
    let summarization_request = vec![user_message];

    let (mut response, mut provider_usage) = provider
        .complete_fast(&system_prompt, &summarization_request, &[])
        .await?;

    response.role = Role::User;

    provider_usage
        .ensure_tokens(&system_prompt, &summarization_request, &response, &[])
        .await
        .map_err(|e| anyhow::anyhow!("Failed to ensure usage tokens: {}", e))?;

    Ok(Some((response, provider_usage)))
}

fn format_message_for_compacting(msg: &Message) -> String {
    let content_parts: Vec<String> = msg
        .content
        .iter()
        .map(|content| match content {
            MessageContent::Text(text) => text.text.clone(),
            MessageContent::Image(img) => format!("[image: {}]", img.mime_type),
            MessageContent::ToolRequest(req) => {
                if let Ok(call) = &req.tool_call {
                    format!(
                        "tool_request({}): {}",
                        call.name,
                        serde_json::to_string_pretty(&call.arguments)
                            .unwrap_or_else(|_| "<<invalid json>>".to_string())
                    )
                } else {
                    "tool_request: [error]".to_string()
                }
            }
            MessageContent::ToolResponse(res) => {
                if let Ok(contents) = &res.tool_result {
                    let text_items: Vec<String> = contents
                        .iter()
                        .filter_map(|content| {
                            content.as_text().map(|text_str| text_str.text.clone())
                        })
                        .collect();

                    if !text_items.is_empty() {
                        format!("tool_response: {}", text_items.join("\n"))
                    } else {
                        "tool_response: [non-text content]".to_string()
                    }
                } else {
                    "tool_response: [error]".to_string()
                }
            }
            MessageContent::ToolConfirmationRequest(req) => {
                format!("tool_confirmation_request: {}", req.tool_name)
            }
            MessageContent::FrontendToolRequest(req) => {
                if let Ok(call) = &req.tool_call {
                    format!("frontend_tool_request: {}", call.name)
                } else {
                    "frontend_tool_request: [error]".to_string()
                }
            }
            MessageContent::Thinking(thinking) => format!("thinking: {}", thinking.thinking),
            MessageContent::RedactedThinking(_) => "redacted_thinking".to_string(),
            MessageContent::SystemNotification(notification) => {
                format!("system_notification: {}", notification.msg)
            }
        })
        .collect();

    let role_str = match msg.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };

    if content_parts.is_empty() {
        format!("[{}]: <empty message>", role_str)
    } else {
        format!("[{}]: {}", role_str, content_parts.join("\n"))
    }
}
