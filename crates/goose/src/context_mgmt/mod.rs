use crate::conversation::message::{ActionRequiredData, MessageMetadata};
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::{merge_consecutive_messages, Conversation};
use crate::prompt_template::render_global_file;
use crate::providers::base::{Provider, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::{config::Config, token_counter::create_token_counter};
use anyhow::Result;
use rmcp::model::Role;
use serde::Serialize;
use tracing::{debug, info};

pub const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;

const CONVERSATION_CONTINUATION_TEXT: &str =
    "The previous message contains a summary that was prepared because a context limit was reached.
Do not mention that you read a summary or that conversation summarization occurred.
Just continue the conversation naturally based on the summarized context";

const TOOL_LOOP_CONTINUATION_TEXT: &str =
    "The previous message contains a summary that was prepared because a context limit was reached.
Do not mention that you read a summary or that conversation summarization occurred.
Continue calling tools as necessary to complete the task.";

const MANUAL_COMPACT_CONTINUATION_TEXT: &str =
    "The previous message contains a summary that was prepared at the user's request.
Do not mention that you read a summary or that conversation summarization occurred.
Just continue the conversation naturally based on the summarized context";

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
/// * `provider` - The provider to use for summarization
/// * `conversation` - The current conversation history
/// * `manual_compact` - If true, this is a manual compaction (don't preserve user message)
///
/// # Returns
/// * A tuple containing:
///   - `Conversation`: The compacted messages
///   - `ProviderUsage`: Provider usage from summarization
pub async fn compact_messages(
    provider: &dyn Provider,
    conversation: &Conversation,
    manual_compact: bool,
) -> Result<(Conversation, ProviderUsage)> {
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

    // Find and preserve the most recent user message for non-manual compacts
    let (preserved_user_message, is_most_recent) = if !manual_compact {
        let found_msg = messages.iter().enumerate().rev().find(|(_, msg)| {
            msg.is_agent_visible()
                && matches!(msg.role, rmcp::model::Role::User)
                && has_text_only(msg)
        });

        if let Some((idx, msg)) = found_msg {
            let is_last = idx == messages.len() - 1;
            (Some(msg.clone()), is_last)
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    let messages_to_compact = messages.as_slice();

    let (summary_message, summarization_usage) = do_compact(provider, messages_to_compact).await?;

    // Create the final message list with updated visibility metadata:
    // 1. Original messages become user_visible but not agent_visible
    // 2. Summary message becomes agent_visible but not user_visible
    // 3. Assistant messages to continue the conversation are also agent_visible but not user_visible
    let mut final_messages = Vec::new();

    for (idx, msg) in messages_to_compact.iter().enumerate() {
        let updated_metadata = if is_most_recent
            && idx == messages_to_compact.len() - 1
            && preserved_user_message.is_some()
        {
            // This is the most recent message and we're preserving it by adding a fresh copy
            MessageMetadata::invisible()
        } else {
            msg.metadata.with_agent_invisible()
        };
        let updated_msg = msg.clone().with_metadata(updated_metadata);
        final_messages.push(updated_msg);
    }

    let summary_msg = summary_message.with_metadata(MessageMetadata::agent_only());

    let mut continuation_messages = vec![summary_msg];

    let continuation_text = if manual_compact {
        MANUAL_COMPACT_CONTINUATION_TEXT
    } else if is_most_recent {
        CONVERSATION_CONTINUATION_TEXT
    } else {
        TOOL_LOOP_CONTINUATION_TEXT
    };

    let continuation_msg = Message::assistant()
        .with_text(continuation_text)
        .with_metadata(MessageMetadata::agent_only());
    continuation_messages.push(continuation_msg);

    let (merged_continuation, _issues) = merge_consecutive_messages(continuation_messages);
    final_messages.extend(merged_continuation);

    if let Some(user_msg) = preserved_user_message {
        if let Some(text) = extract_text(&user_msg) {
            final_messages.push(Message::user().with_text(&text));
        }
    }

    Ok((
        Conversation::new_unvalidated(final_messages),
        summarization_usage,
    ))
}

/// Check if messages exceed the auto-compaction threshold
pub async fn check_if_compaction_needed(
    provider: &dyn Provider,
    conversation: &Conversation,
    threshold_override: Option<f64>,
    session: &crate::session::Session,
) -> Result<bool> {
    let messages = conversation.messages();
    let config = Config::global();
    let threshold = threshold_override.unwrap_or_else(|| {
        config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD)
    });

    let context_limit = provider.get_model_config().context_limit();

    let (current_tokens, token_source) = match session.total_tokens {
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

fn filter_tool_responses<'a>(messages: &[&'a Message], remove_percent: u32) -> Vec<&'a Message> {
    fn has_tool_response(msg: &Message) -> bool {
        msg.content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolResponse(_)))
    }

    if remove_percent == 0 {
        return messages.to_vec();
    }

    let tool_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, msg)| has_tool_response(msg))
        .map(|(i, _)| i)
        .collect();

    if tool_indices.is_empty() {
        return messages.to_vec();
    }

    let num_to_remove = ((tool_indices.len() * remove_percent as usize) / 100).max(1);

    let middle = tool_indices.len() / 2;
    let mut indices_to_remove = Vec::new();

    // Middle out
    for i in 0..num_to_remove {
        if i % 2 == 0 {
            let offset = i / 2;
            if middle > offset {
                indices_to_remove.push(tool_indices[middle - offset - 1]);
            }
        } else {
            let offset = i / 2;
            if middle + offset < tool_indices.len() {
                indices_to_remove.push(tool_indices[middle + offset]);
            }
        }
    }

    messages
        .iter()
        .enumerate()
        .filter(|(i, _)| !indices_to_remove.contains(i))
        .map(|(_, msg)| *msg)
        .collect()
}

async fn do_compact(
    provider: &dyn Provider,
    messages: &[Message],
) -> Result<(Message, ProviderUsage), anyhow::Error> {
    let agent_visible_messages: Vec<&Message> = messages
        .iter()
        .filter(|msg| msg.is_agent_visible())
        .collect();

    // Try progressively removing more tool response messages from the middle to reduce context length
    let removal_percentages = [0, 10, 20, 50, 100];

    for (attempt, &remove_percent) in removal_percentages.iter().enumerate() {
        let filtered_messages = filter_tool_responses(&agent_visible_messages, remove_percent);

        let messages_text = filtered_messages
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

        match provider
            .complete_fast(&system_prompt, &summarization_request, &[])
            .await
        {
            Ok((mut response, mut provider_usage)) => {
                response.role = Role::User;

                provider_usage
                    .ensure_tokens(&system_prompt, &summarization_request, &response, &[])
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to ensure usage tokens: {}", e))?;

                return Ok((response, provider_usage));
            }
            Err(e) => {
                if matches!(e, ProviderError::ContextLengthExceeded(_)) {
                    if attempt < removal_percentages.len() - 1 {
                        continue;
                    } else {
                        return Err(anyhow::anyhow!(
                            "Failed to compact messages: context length still exceeded after {} attempts with maximum removal",
                            removal_percentages.len()
                        ));
                    }
                }
                return Err(e.into());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Unexpected: exhausted all attempts without returning"
    ))
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
                if let Ok(result) = &res.tool_result {
                    let text_items: Vec<String> = result
                        .content
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
            MessageContent::ActionRequired(action) => match &action.data {
                ActionRequiredData::ToolConfirmation { tool_name, .. } => {
                    format!("action_required(tool_confirmation): {}", tool_name)
                }
                ActionRequiredData::Elicitation { message, .. } => {
                    format!("action_required(elicitation): {}", message)
                }
                ActionRequiredData::ElicitationResponse { id, .. } => {
                    format!("action_required(elicitation_response): {}", id)
                }
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::ModelConfig,
        providers::{
            base::{ProviderMetadata, Usage},
            errors::ProviderError,
        },
    };
    use async_trait::async_trait;
    use rmcp::model::{AnnotateAble, CallToolRequestParam, RawContent, Tool};

    struct MockProvider {
        message: Message,
        config: ModelConfig,
        max_tool_responses: Option<usize>,
    }

    impl MockProvider {
        fn new(message: Message, context_limit: usize) -> Self {
            Self {
                message,
                config: ModelConfig {
                    model_name: "test".to_string(),
                    context_limit: Some(context_limit),
                    temperature: None,
                    max_tokens: None,
                    toolshim: false,
                    toolshim_model: None,
                    fast_model: None,
                },
                max_tool_responses: None,
            }
        }

        fn with_max_tool_responses(mut self, max: usize) -> Self {
            self.max_tool_responses = Some(max);
            self
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::new("mock", "", "", "", vec![""], "", vec![])
        }

        fn get_name(&self) -> &str {
            "mock"
        }

        async fn complete_with_model(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            // If max_tool_responses is set, fail if we have too many
            if let Some(max) = self.max_tool_responses {
                let tool_response_count = messages
                    .iter()
                    .filter(|m| {
                        m.content
                            .iter()
                            .any(|c| matches!(c, MessageContent::ToolResponse(_)))
                    })
                    .count();

                if tool_response_count > max {
                    return Err(ProviderError::ContextLengthExceeded(format!(
                        "Too many tool responses: {} > {}",
                        tool_response_count, max
                    )));
                }
            }

            Ok((
                self.message.clone(),
                ProviderUsage::new("mock-model".to_string(), Usage::default()),
            ))
        }

        fn get_model_config(&self) -> ModelConfig {
            self.config.clone()
        }
    }

    #[tokio::test]
    async fn test_keeps_tool_request() {
        let response_message = Message::assistant().with_text("<mock summary>");
        let provider = MockProvider::new(response_message, 1);
        let basic_conversation = vec![
            Message::user().with_text("read hello.txt"),
            Message::assistant().with_tool_request(
                "tool_0",
                Ok(CallToolRequestParam {
                    name: "read_file".into(),
                    arguments: None,
                }),
            ),
            Message::user().with_tool_response(
                "tool_0",
                Ok(rmcp::model::CallToolResult {
                    content: vec![RawContent::text("hello, world").no_annotation()],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            ),
        ];

        let conversation = Conversation::new_unvalidated(basic_conversation);
        let (compacted_conversation, _usage) = compact_messages(&provider, &conversation, false)
            .await
            .unwrap();

        let agent_conversation = compacted_conversation.agent_visible_messages();

        let _ = Conversation::new(agent_conversation)
            .expect("compaction should produce a valid conversation");
    }

    #[tokio::test]
    async fn test_progressive_removal_on_context_exceeded() {
        let response_message = Message::assistant().with_text("<mock summary>");
        // Set max to 2 tool responses - will trigger progressive removal
        let provider = MockProvider::new(response_message, 1000).with_max_tool_responses(2);

        // Create a conversation with many tool responses
        let mut messages = vec![Message::user().with_text("start")];
        for i in 0..10 {
            messages.push(Message::assistant().with_tool_request(
                format!("tool_{}", i),
                Ok(CallToolRequestParam {
                    name: "read_file".into(),
                    arguments: None,
                }),
            ));
            messages.push(Message::user().with_tool_response(
                format!("tool_{}", i),
                Ok(rmcp::model::CallToolResult {
                    content: vec![RawContent::text(format!("response{}", i)).no_annotation()],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            ));
        }

        let conversation = Conversation::new_unvalidated(messages);
        let result = compact_messages(&provider, &conversation, false).await;

        // Should succeed after progressive removal
        assert!(
            result.is_ok(),
            "Should succeed with progressive removal: {:?}",
            result.err()
        );
    }
}
