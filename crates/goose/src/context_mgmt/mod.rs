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

/// Result of auto-compaction check
#[derive(Debug)]
pub struct AutoCompactResult {
    /// Whether compaction was performed
    pub compacted: bool,
    /// The messages after potential compaction
    pub messages: Conversation,
    /// Provider usage from summarization (if compaction occurred)
    /// This contains the actual token counts after compaction
    pub summarization_usage: Option<crate::providers::base::ProviderUsage>,
}

/// Result of checking if compaction is needed
#[derive(Debug)]
pub struct CompactionCheckResult {
    /// Whether compaction is needed
    pub needs_compaction: bool,
    /// Current token count
    pub current_tokens: usize,
    /// Context limit being used
    pub context_limit: usize,
    /// Current usage ratio (0.0 to 1.0)
    pub usage_ratio: f64,
    /// Remaining tokens before compaction threshold
    pub remaining_tokens: usize,
    /// Percentage until compaction threshold (0.0 to 100.0)
    pub percentage_until_compaction: f64,
}

#[derive(Serialize)]
struct SummarizeContext {
    messages: String,
}

/// Check if messages need compaction and compact them if necessary
///
/// This function combines checking and compaction. It first checks if compaction
/// is needed based on the threshold, and if so, performs the compaction by
/// summarizing messages and updating their visibility metadata.
///
/// # Arguments
/// * `agent` - The agent to use for context management
/// * `messages` - The current message history
/// * `force_compact` - If true, skip the threshold check and force compaction
/// * `preserve_last_user_message` - If true and last message is not a user message, copy the most recent user message to the end
/// * `threshold_override` - Optional threshold override (defaults to GOOSE_AUTO_COMPACT_THRESHOLD config)
/// * `session_metadata` - Optional session metadata containing actual token counts
///
/// # Returns
/// * A tuple containing:
///   - `bool`: Whether compaction was performed
///   - `Conversation`: The potentially compacted messages
///   - `Vec<usize>`: Indices of removed messages (empty if no compaction)
///   - `Option<ProviderUsage>`: Provider usage from summarization (if compaction occurred)
pub async fn check_and_compact_messages(
    agent: &Agent,
    messages_with_user_message: &[Message],
    force_compact: bool,
    preserve_last_user_message: bool,
    threshold_override: Option<f64>,
    session_metadata: Option<&crate::session::Session>,
) -> std::result::Result<(bool, Conversation, Vec<usize>, Option<ProviderUsage>), anyhow::Error> {
    if !force_compact {
        let check_result = check_compaction_needed(
            agent,
            messages_with_user_message,
            threshold_override,
            session_metadata,
        )
        .await?;

        // If no compaction is needed, return early
        if !check_result.needs_compaction {
            debug!(
                "No compaction needed (usage: {:.1}% <= {:.1}% threshold)",
                check_result.usage_ratio * 100.0,
                check_result.percentage_until_compaction
            );
            return Ok((
                false,
                Conversation::new_unvalidated(messages_with_user_message.to_vec()),
                Vec::new(),
                None,
            ));
        }

        info!(
            "Performing message compaction (usage: {:.1}%)",
            check_result.usage_ratio * 100.0
        );
    } else {
        info!("Forcing message compaction due to context limit exceeded");
    }

    // Perform the actual compaction
    // Check if the most recent message is a user message
    let (messages, preserved_user_message) =
        if let Some(last_message) = messages_with_user_message.last() {
            if matches!(last_message.role, rmcp::model::Role::User) {
                // Remove the last user message before compaction
                (
                    &messages_with_user_message[..messages_with_user_message.len() - 1],
                    Some(last_message.clone()),
                )
            } else if preserve_last_user_message {
                // Last message is not a user message, but we want to preserve the most recent user message
                // Find the most recent user message and copy it (don't remove from history)
                let most_recent_user_message = messages_with_user_message
                    .iter()
                    .rev()
                    .find(|msg| matches!(msg.role, rmcp::model::Role::User))
                    .cloned();
                (messages_with_user_message, most_recent_user_message)
            } else {
                (messages_with_user_message, None)
            }
        } else {
            (messages_with_user_message, None)
        };

    let provider = agent.provider().await?;
    let summary = do_compact(provider.clone(), messages).await?;

    let (summary_message, summarization_usage) = match summary {
        Some((summary_message, provider_usage)) => (summary_message, Some(provider_usage)),
        None => {
            // No summary was generated (empty input)
            tracing::warn!("Summarization failed. Returning empty messages.");
            return Ok((false, Conversation::empty(), vec![], None));
        }
    };

    // Create the final message list with updated visibility metadata:
    // 1. Original messages become user_visible but not agent_visible
    // 2. Summary message becomes agent_visible but not user_visible
    // 3. Assistant messages to continue the conversation remain both user_visible and agent_visible

    let mut final_messages = Vec::new();
    let mut final_token_counts = Vec::new();

    // Add all original messages with updated visibility (preserve user_visible, set agent_visible=false)
    for msg in messages.iter().cloned() {
        let updated_metadata = msg.metadata.with_agent_invisible();
        let updated_msg = msg.with_metadata(updated_metadata);
        final_messages.push(updated_msg);
        // Token count doesn't matter for agent_visible=false messages, but we'll use 0
        final_token_counts.push(0);
    }

    // Add the compaction marker (user_visible=true, agent_visible=false)
    let compaction_marker = Message::assistant()
        .with_conversation_compacted("Conversation compacted and summarized")
        .with_metadata(MessageMetadata::user_only());
    let compaction_marker_tokens: usize = 0; // Not counted since agent_visible=false
    final_messages.push(compaction_marker);
    final_token_counts.push(compaction_marker_tokens);

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
    if let Some(user_message) = preserved_user_message {
        final_messages.push(user_message);
    }

    Ok((
        true,
        Conversation::new_unvalidated(final_messages),
        final_token_counts,
        summarization_usage,
    ))
}

/// Check if messages need compaction without performing the compaction
///
/// This function analyzes the current token usage and returns detailed information
/// about whether compaction is needed and how close we are to the threshold.
/// It prioritizes actual token counts from session metadata when available,
/// falling back to estimated counts if needed.
///
/// # Arguments
/// * `agent` - The agent to use for context management
/// * `messages` - The current message history
/// * `threshold_override` - Optional threshold override (defaults to GOOSE_AUTO_COMPACT_THRESHOLD config)
/// * `session_metadata` - Optional session metadata containing actual token counts
///
/// # Returns
/// * `CompactionCheckResult` containing detailed information about compaction needs
async fn check_compaction_needed(
    agent: &Agent,
    messages: &[Message],
    threshold_override: Option<f64>,
    session_metadata: Option<&crate::session::Session>,
) -> Result<CompactionCheckResult> {
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

    let threshold_tokens = (context_limit as f64 * threshold) as usize;
    let remaining_tokens = threshold_tokens.saturating_sub(current_tokens);

    let percentage_until_compaction = if usage_ratio < threshold {
        (threshold - usage_ratio) * 100.0
    } else {
        0.0
    };

    let needs_compaction = if threshold <= 0.0 || threshold >= 1.0 {
        usage_ratio > DEFAULT_COMPACTION_THRESHOLD
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

    Ok(CompactionCheckResult {
        needs_compaction,
        current_tokens,
        context_limit,
        usage_ratio,
        remaining_tokens,
        percentage_until_compaction,
    })
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
                            if let Some(text_str) = content.as_text() {
                                Some(text_str.text.clone())
                            } else {
                                None
                            }
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
            MessageContent::ConversationCompacted(compact) => format!("compacted: {}", compact.msg),
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
