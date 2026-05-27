use crate::conversation::message::{ActionRequiredData, MessageMetadata};
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::{merge_consecutive_messages, Conversation};
use crate::prompt_template::render_template;
#[cfg(test)]
use crate::providers::base::{stream_from_single_message, MessageStream};
use crate::providers::base::{Provider, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::{config::Config, token_counter::create_token_counter};
use anyhow::Result;
use indoc::indoc;
use rmcp::model::Role;
use serde::Serialize;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;
use tracing::log::warn;

pub const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;

const MAX_AUTO_COMPACTION_THRESHOLD: f64 = 0.9;
const MAX_COMPACTION_HISTORY_TOKENS: usize = 20_000;
const MIN_COMPACTION_HISTORY_TOKENS: usize = 1_000;
const TOOLCALL_SUMMARIZATION_BATCH_SIZE: usize = 10;

fn effective_auto_compaction_threshold(threshold: f64) -> Option<f64> {
    if threshold <= 0.0 || threshold >= 1.0 {
        None
    } else {
        Some(threshold.min(MAX_AUTO_COMPACTION_THRESHOLD))
    }
}

fn tool_pair_summarization_enabled() -> bool {
    Config::global()
        .get_param::<bool>("GOOSE_TOOL_PAIR_SUMMARIZATION")
        .unwrap_or(true)
}

const CONVERSATION_CONTINUATION_TEXT: &str =
    "Your context was compacted. The previous message contains a summary of the conversation so far.
Do not mention that you read a summary or that conversation summarization occurred.
Just continue the conversation naturally based on the summarized context.";

const TOOL_LOOP_CONTINUATION_TEXT: &str =
    "Your context was compacted. The previous message contains a summary of the conversation so far.
Do not mention that you read a summary or that conversation summarization occurred.
Continue calling tools as necessary to complete the task.";

const MANUAL_COMPACT_CONTINUATION_TEXT: &str =
    "Your context was compacted at the user's request. The previous message contains a summary of the conversation so far.
Do not mention that you read a summary or that conversation summarization occurred.
Just continue the conversation naturally based on the summarized context.";

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
/// * `session_id` - The session to use for summarization
/// * `conversation` - The current conversation history
/// * `manual_compact` - If true, this is a manual compaction (don't preserve user message)
///
/// # Returns
/// * A tuple containing:
///   - `Conversation`: The compacted messages
///   - `ProviderUsage`: Provider usage from summarization
pub async fn compact_messages(
    provider: &dyn Provider,
    session_id: &str,
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

    let (summary_message, summarization_usage) =
        do_compact(provider, session_id, messages_to_compact).await?;

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
            msg.metadata.clone().with_agent_invisible()
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
    if provider.manages_own_context() {
        return Ok(false);
    }

    let messages = conversation.messages();
    let config = Config::global();
    let threshold = threshold_override.unwrap_or_else(|| {
        config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD)
    });

    let context_limit = provider.get_model_config().context_limit();

    let (current_tokens, _token_source) = match session.total_tokens {
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

    let needs_compaction = effective_auto_compaction_threshold(threshold)
        .is_some_and(|effective_threshold| usage_ratio > effective_threshold);
    Ok(needs_compaction)
}

fn filter_tool_responses(messages: &[Message], remove_percent: u32) -> Vec<&Message> {
    fn has_tool_response(msg: &Message) -> bool {
        msg.content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolResponse(_)))
    }

    if remove_percent == 0 {
        return messages.iter().collect();
    }

    let tool_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, msg)| has_tool_response(msg))
        .map(|(i, _)| i)
        .collect();

    if tool_indices.is_empty() {
        return messages.iter().collect();
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
        .map(|(_, msg)| msg)
        .collect()
}

fn compaction_history_token_budgets(context_limit: usize) -> Vec<usize> {
    let initial_budget = MAX_COMPACTION_HISTORY_TOKENS.min(context_limit.saturating_mul(7) / 10);
    if initial_budget == 0 {
        return vec![1];
    }

    let mut budgets = vec![initial_budget];
    let mut next_budget = initial_budget / 2;

    while next_budget >= MIN_COMPACTION_HISTORY_TOKENS {
        budgets.push(next_budget);
        next_budget /= 2;
    }

    if initial_budget > MIN_COMPACTION_HISTORY_TOKENS {
        budgets.push(MIN_COMPACTION_HISTORY_TOKENS);
    }

    budgets.dedup();
    budgets
}

fn truncate_to_token_budget(
    text: &str,
    token_budget: usize,
    token_counter: &crate::token_counter::TokenCounter,
) -> String {
    if token_budget == 0 {
        return String::new();
    }

    let notice = "\n[truncated to fit compaction context]";
    let content_budget = token_budget.saturating_sub(token_counter.count_tokens(notice));
    if content_budget == 0 {
        return String::new();
    }

    let mut low = 0;
    let mut high = text.len();
    let mut best = 0;

    while low <= high {
        let mut mid = low + (high - low) / 2;
        while mid > 0 && !text.is_char_boundary(mid) {
            mid -= 1;
        }

        let candidate = text.get(..mid).unwrap_or_default();
        if token_counter.count_tokens(candidate) <= content_budget {
            best = mid;
            low = mid.saturating_add(1);
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }

    if best == 0 {
        String::new()
    } else {
        format!("{}{}", text.get(..best).unwrap_or_default(), notice)
    }
}

fn build_bounded_compaction_text(
    messages: &[&Message],
    max_tokens: usize,
    token_counter: &crate::token_counter::TokenCounter,
) -> String {
    let mut remaining_tokens = max_tokens;
    let mut selected = Vec::new();

    for msg in messages.iter().rev() {
        let formatted = format_message_for_compacting(msg);
        let token_count = token_counter.count_tokens(&formatted);

        if token_count <= remaining_tokens {
            selected.push(formatted);
            remaining_tokens -= token_count;
            continue;
        }

        if remaining_tokens > 0 {
            let truncated = truncate_to_token_budget(&formatted, remaining_tokens, token_counter);
            if !truncated.is_empty() {
                selected.push(truncated);
            }
        }
        break;
    }

    selected.reverse();
    selected.join("\n")
}

async fn do_compact(
    provider: &dyn Provider,
    session_id: &str,
    messages: &[Message],
) -> Result<(Message, ProviderUsage), anyhow::Error> {
    let agent_visible_messages: Vec<Message> = messages
        .iter()
        .filter(|msg| msg.is_agent_visible())
        .map(|msg| msg.agent_visible_content())
        .collect();

    // Try progressively removing tool responses and shrinking non-tool history so the
    // summarization request has headroom even when the reported context limit is optimistic.
    let removal_percentages = [0, 10, 20, 50, 100];
    let history_token_budgets =
        compaction_history_token_budgets(provider.get_model_config().context_limit());
    let token_counter = create_token_counter()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;

    for &remove_percent in removal_percentages.iter() {
        let filtered_messages = filter_tool_responses(&agent_visible_messages, remove_percent);

        for &history_token_budget in history_token_budgets.iter() {
            let messages_text = build_bounded_compaction_text(
                &filtered_messages,
                history_token_budget,
                &token_counter,
            );

            let context = SummarizeContext {
                messages: messages_text,
            };

            let system_prompt = render_template("compaction.md", &context)?;

            let user_message = Message::user().with_text(
                "Please summarize the conversation history provided in the system prompt.",
            );
            let summarization_request = vec![user_message];

            match provider
                .complete_fast(session_id, &system_prompt, &summarization_request, &[])
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
                        continue;
                    }
                    return Err(e.into());
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Failed to compact: context limit exceeded after trimming compaction input"
    ))
}

pub fn format_message_for_compacting(msg: &Message) -> String {
    let content_parts: Vec<String> = msg
        .content
        .iter()
        .filter_map(|content| match content {
            MessageContent::Text(text) => Some(text.text.clone()),
            MessageContent::Image(img) => Some(format!("[image: {}]", img.mime_type)),
            MessageContent::ToolRequest(req) => {
                if let Ok(call) = &req.tool_call {
                    Some(format!(
                        "tool_request({}): {}",
                        call.name,
                        serde_json::to_string(&call.arguments)
                            .unwrap_or_else(|_| "<<invalid json>>".to_string())
                    ))
                } else {
                    Some("tool_request: [error]".to_string())
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
                        Some(format!("tool_response: {}", text_items.join("\n")))
                    } else {
                        Some("tool_response: [non-text content]".to_string())
                    }
                } else {
                    Some("tool_response: [error]".to_string())
                }
            }
            MessageContent::ToolConfirmationRequest(req) => {
                Some(format!("tool_confirmation_request: {}", req.tool_name))
            }
            MessageContent::ActionRequired(action) => match &action.data {
                ActionRequiredData::ToolConfirmation { tool_name, .. } => {
                    Some(format!("action_required(tool_confirmation): {}", tool_name))
                }
                ActionRequiredData::Elicitation { message, .. } => {
                    Some(format!("action_required(elicitation): {}", message))
                }
                ActionRequiredData::ElicitationResponse { id, .. } => {
                    Some(format!("action_required(elicitation_response): {}", id))
                }
            },
            MessageContent::FrontendToolRequest(req) => {
                if let Ok(call) = &req.tool_call {
                    Some(format!("frontend_tool_request: {}", call.name))
                } else {
                    Some("frontend_tool_request: [error]".to_string())
                }
            }
            MessageContent::Thinking(_) => None,
            MessageContent::RedactedThinking(_) => None,
            MessageContent::SystemNotification(notification) => {
                Some(format!("system_notification: {}", notification.msg))
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

pub fn compute_tool_call_cutoff(context_limit: usize, compaction_threshold: f64) -> usize {
    let threshold = if compaction_threshold > 0.0 && compaction_threshold <= 1.0 {
        compaction_threshold
    } else {
        DEFAULT_COMPACTION_THRESHOLD
    };
    let effective_limit = (context_limit as f64 * threshold) as usize;
    (3 * effective_limit / 20_000).clamp(10, 500)
}

pub fn tool_ids_to_summarize(
    conversation: &Conversation,
    cutoff: usize,
    protect_last_n: usize,
) -> Vec<String> {
    let messages = conversation.messages();

    let mut tool_call_ids: Vec<String> = Vec::new();

    for msg in messages.iter() {
        if !msg.is_agent_visible() {
            continue;
        }

        for content in &msg.content {
            if let MessageContent::ToolRequest(req) = content {
                tool_call_ids.push(req.id.clone());
            }
        }
    }

    // Never summarize the last N tool calls (current turn)
    let eligible = tool_call_ids.len().saturating_sub(protect_last_n);
    if eligible <= cutoff + TOOLCALL_SUMMARIZATION_BATCH_SIZE {
        return Vec::new();
    }

    tool_call_ids
        .into_iter()
        .take(TOOLCALL_SUMMARIZATION_BATCH_SIZE)
        .collect()
}

pub async fn summarize_tool_call(
    provider: &dyn Provider,
    session_id: &str,
    conversation: &Conversation,
    tool_id: &str,
) -> Result<Message> {
    let messages = conversation.messages();

    let matching_messages: Vec<&Message> = messages
        .iter()
        .filter(|m| {
            m.content.iter().any(|c| match c {
                MessageContent::ToolRequest(req) => req.id == tool_id,
                MessageContent::ToolResponse(resp) => resp.id == tool_id,
                _ => false,
            })
        })
        .collect();

    if matching_messages.is_empty() {
        return Err(anyhow::anyhow!(
            "No messages found for tool id: {}",
            tool_id
        ));
    }

    let formatted = matching_messages
        .iter()
        .map(|msg| format_message_for_compacting(msg))
        .collect::<Vec<_>>()
        .join("\n");

    let user_message = Message::user().with_text(formatted);
    let summarization_request = vec![user_message];

    let system_prompt = indoc! {r#"
                Your task is to summarize a tool call & response pair to save tokens.

                Reply with a single message that describes what happened. Typically a tool call
                asks for something using a bunch of parameters and then the result is also some
                structured output. So the tool might ask to look up something on github and the
                reply might be a json document. So you could reply with something like:

                "A call to github was made to get the project status"

                if that is what it was.
            "#};

    let (mut response, _) = provider
        .complete_fast(session_id, system_prompt, &summarization_request, &[])
        .await?;

    response.role = Role::User;
    response.created = matching_messages.last().unwrap().created;
    response.metadata = MessageMetadata::agent_only();

    Ok(response.with_generated_id())
}

pub fn maybe_summarize_tool_pairs(
    provider: Arc<dyn Provider>,
    session_id: String,
    conversation: Conversation,
    cutoff: usize,
    protect_last_n: usize,
) -> Option<JoinHandle<Vec<(Message, String)>>> {
    if !tool_pair_summarization_enabled() || provider.manages_own_context() {
        return None;
    }

    let tool_ids = tool_ids_to_summarize(&conversation, cutoff, protect_last_n);
    if tool_ids.is_empty() {
        return None;
    }

    Some(tokio::spawn(async move {
        let mut results = Vec::new();
        for tool_id in tool_ids {
            match summarize_tool_call(provider.as_ref(), &session_id, &conversation, &tool_id).await
            {
                Ok(summary) => results.push((summary, tool_id)),
                Err(e) => {
                    warn!("Failed to summarize tool pair: {}", e);
                }
            }
        }
        results
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::ModelConfig,
        providers::{base::Usage, errors::ProviderError},
    };
    use async_trait::async_trait;
    use rmcp::model::{AnnotateAble, CallToolRequestParams, RawContent, Tool};

    fn create_tool_pair(
        call_id: &str,
        response_id: &str,
        tool_name: &str,
        response_text: &str,
    ) -> Vec<Message> {
        vec![
            Message::assistant()
                .with_tool_request(
                    call_id,
                    Ok(CallToolRequestParams::new(tool_name.to_string())),
                )
                .with_id(call_id),
            Message::user()
                .with_tool_response(
                    call_id,
                    Ok(rmcp::model::CallToolResult::success(vec![
                        RawContent::text(response_text).no_annotation(),
                    ])),
                )
                .with_id(response_id),
        ]
    }

    struct MockProvider {
        message: Message,
        config: ModelConfig,
        max_tool_responses: Option<usize>,
        max_system_tokens: Option<usize>,
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
                    fast_model_config: None,
                    request_params: None,
                    reasoning: None,
                },
                max_tool_responses: None,
                max_system_tokens: None,
            }
        }

        fn with_max_tool_responses(mut self, max: usize) -> Self {
            self.max_tool_responses = Some(max);
            self
        }

        fn with_max_system_tokens(mut self, max: usize) -> Self {
            self.max_system_tokens = Some(max);
            self
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn get_name(&self) -> &str {
            "mock"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _session_id: &str,
            system: &str,
            messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
            if let Some(max) = self.max_system_tokens {
                let token_counter = crate::token_counter::create_token_counter()
                    .await
                    .map_err(ProviderError::UsageError)?;
                let system_tokens = token_counter.count_tokens(system);

                if system_tokens > max {
                    return Err(ProviderError::ContextLengthExceeded(format!(
                        "System prompt too large: {} > {}",
                        system_tokens, max
                    )));
                }
            }

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

            let message = self.message.clone();
            let usage = ProviderUsage::new("mock-model".to_string(), Usage::default());
            Ok(stream_from_single_message(message, usage))
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
            Message::assistant()
                .with_tool_request("tool_0", Ok(CallToolRequestParams::new("read_file"))),
            Message::user().with_tool_response(
                "tool_0",
                Ok(rmcp::model::CallToolResult::success(vec![
                    RawContent::text("hello, world").no_annotation(),
                ])),
            ),
        ];

        let conversation = Conversation::new_unvalidated(basic_conversation);
        let (compacted_conversation, _usage) =
            compact_messages(&provider, "test-session-id", &conversation, false)
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
                Ok(CallToolRequestParams::new("read_file")),
            ));
            messages.push(Message::user().with_tool_response(
                format!("tool_{}", i),
                Ok(rmcp::model::CallToolResult::success(vec![
                    RawContent::text(format!("response{}", i)).no_annotation(),
                ])),
            ));
        }

        let conversation = Conversation::new_unvalidated(messages);
        let result = compact_messages(&provider, "test-session-id", &conversation, false).await;

        assert!(
            result.is_ok(),
            "Should succeed with progressive removal: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_compaction_bounds_non_tool_history_on_context_exceeded() {
        let response_message = Message::assistant().with_text("<mock summary>");
        let provider = MockProvider::new(response_message, 20_000).with_max_system_tokens(3_000);

        let mut messages = Vec::new();
        for i in 0..20 {
            messages.push(Message::user().with_text(format!(
                "large non-tool user message {} {}",
                i,
                "word ".repeat(1000)
            )));
            messages.push(Message::assistant().with_text(format!(
                "large non-tool assistant message {} {}",
                i,
                "reply ".repeat(1000)
            )));
        }

        let conversation = Conversation::new_unvalidated(messages);
        let result = compact_messages(&provider, "test-session-id", &conversation, false).await;

        assert!(
            result.is_ok(),
            "Should shrink non-tool history until compaction fits: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_bounded_compaction_text_prefers_recent_messages() {
        let token_counter = crate::token_counter::create_token_counter()
            .await
            .expect("token counter should be available");
        let messages = [
            Message::user().with_text(format!("oldest {}", "old ".repeat(1000))),
            Message::assistant().with_text(format!("middle {}", "middle ".repeat(1000))),
            Message::user().with_text(format!("newest {}", "new ".repeat(1000))),
        ];
        let message_refs = messages.iter().collect::<Vec<_>>();

        let bounded_text = build_bounded_compaction_text(&message_refs, 300, &token_counter);

        assert!(
            bounded_text.contains("newest"),
            "Most recent content should be retained"
        );
        assert!(
            !bounded_text.contains("oldest"),
            "Oldest content should be dropped first"
        );
    }

    #[test]
    fn test_auto_compaction_threshold_keeps_headroom() {
        assert_eq!(effective_auto_compaction_threshold(0.8), Some(0.8));
        assert_eq!(effective_auto_compaction_threshold(0.95), Some(0.9));
        assert_eq!(effective_auto_compaction_threshold(0.0), None);
        assert_eq!(effective_auto_compaction_threshold(1.0), None);
    }

    #[test]
    fn test_compute_tool_call_cutoff_scales_with_context() {
        // Default threshold (0.8)
        assert_eq!(compute_tool_call_cutoff(128_000, 0.8), 15); // 102K effective
        assert_eq!(compute_tool_call_cutoff(200_000, 0.8), 24); // 160K effective
        assert_eq!(compute_tool_call_cutoff(1_000_000, 0.8), 120); // 800K effective
                                                                   // Clamp at minimum
        assert_eq!(compute_tool_call_cutoff(50_000, 0.8), 10);
        assert_eq!(compute_tool_call_cutoff(10_000, 0.8), 10);
        // Clamp at maximum (500)
        assert_eq!(compute_tool_call_cutoff(10_000_000, 0.8), 500);
        // Lower compaction threshold means earlier summarization
        assert_eq!(compute_tool_call_cutoff(200_000, 0.3), 10); // 60K effective
        assert_eq!(compute_tool_call_cutoff(1_000_000, 0.5), 75); // 500K effective
                                                                  // Invalid threshold falls back to default 0.8
        assert_eq!(compute_tool_call_cutoff(200_000, 0.0), 24); // falls back to 0.8
        assert_eq!(compute_tool_call_cutoff(200_000, -1.0), 24); // falls back to 0.8
    }

    #[test]
    fn test_tool_ids_to_summarize_triggers_at_cutoff_plus_batch() {
        // cutoff=5, so we need >5+10=15 to trigger. 15 exactly should NOT trigger.
        let mut messages = vec![Message::user().with_text("hello")];
        for i in 0..15 {
            messages.extend(create_tool_pair(
                &format!("call{}", i),
                &format!("resp{}", i),
                "read_file",
                "content",
            ));
        }
        let conversation = Conversation::new_unvalidated(messages);
        let result = tool_ids_to_summarize(&conversation, 5, 0);
        assert!(result.is_empty(), "Exactly cutoff+batch should not trigger");

        // 16 tool calls: now exceeds cutoff+10, should return a batch of 10
        let mut messages = vec![Message::user().with_text("hello")];
        for i in 0..16 {
            messages.extend(create_tool_pair(
                &format!("call{}", i),
                &format!("resp{}", i),
                "read_file",
                "content",
            ));
        }
        let conversation = Conversation::new_unvalidated(messages);
        let result = tool_ids_to_summarize(&conversation, 5, 0);
        assert_eq!(result.len(), TOOLCALL_SUMMARIZATION_BATCH_SIZE);
        assert_eq!(result[0], "call0");
        assert_eq!(result[9], "call9");
    }

    #[test]
    fn test_tool_ids_to_summarize_protects_current_turn() {
        // 20 tool pairs, cutoff=2 → 20 > 12, would normally trigger
        let mut messages = vec![Message::user().with_text("hello")];
        for i in 0..20 {
            messages.extend(create_tool_pair(
                &format!("call{}", i),
                &format!("resp{}", i),
                "read_file",
                "content",
            ));
        }
        let conversation = Conversation::new_unvalidated(messages);

        // No protection: 20 eligible, 20 > 12 → batch of 10
        let result = tool_ids_to_summarize(&conversation, 2, 0);
        assert_eq!(result.len(), TOOLCALL_SUMMARIZATION_BATCH_SIZE);

        // Protect last 8: 12 eligible, 12 <= 12 → nothing
        let result = tool_ids_to_summarize(&conversation, 2, 8);
        assert!(
            result.is_empty(),
            "Should not summarize when protected count leaves eligible <= cutoff + batch"
        );

        // Protect last 7: 13 eligible, 13 > 12 → batch of 10
        let result = tool_ids_to_summarize(&conversation, 2, 7);
        assert_eq!(result.len(), TOOLCALL_SUMMARIZATION_BATCH_SIZE);
        assert_eq!(result[0], "call0");
    }
}
