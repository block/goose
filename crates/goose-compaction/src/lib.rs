pub mod structured;

use crate::structured::StructuredSummary;
use anyhow::Result;
use async_trait::async_trait;
use goose_provider_types::conversation::message::{
    ActionRequiredData, Message, MessageContent, MessageMetadata,
};
use goose_provider_types::conversation::token_usage::ProviderUsage;
use goose_provider_types::conversation::{merge_consecutive_messages, Conversation};
use goose_provider_types::errors::ProviderError;
use indoc::indoc;
use minijinja::{Environment, Error as MiniJinjaError, Value as MJValue};
use rmcp::model::{Role, Tool};
use serde::Serialize;
use tracing::{info, warn};

pub const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;
pub const TOOLCALL_SUMMARIZATION_BATCH_SIZE: usize = 10;

pub const COMPACTION_PROMPT_TEMPLATE: &str = include_str!("prompts/compaction.md");
pub const COMPACTION_SUMMARY_TEMPLATE: &str = include_str!("prompts/compaction_summary.md");

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

/// The model I/O used for compaction, injected by the caller so model
/// selection, session attribution, and token counting stay in the
/// application layer.
#[async_trait]
pub trait CompactionModel: Send + Sync {
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError>;

    async fn count_tokens(&self, system: &str, messages: &[Message]) -> Result<usize>;
}

#[derive(Debug, Clone, Default)]
pub struct CompactionSettings {
    pub compaction_prompt_override: Option<String>,
    pub summary_template_override: Option<String>,
}

impl CompactionSettings {
    fn compaction_prompt(&self) -> &str {
        self.compaction_prompt_override
            .as_deref()
            .unwrap_or(COMPACTION_PROMPT_TEMPLATE)
    }

    fn summary_template(&self) -> &str {
        self.summary_template_override
            .as_deref()
            .unwrap_or(COMPACTION_SUMMARY_TEMPLATE)
    }
}

fn code_fence(code: String) -> String {
    let longest_run = code
        .chars()
        .fold((0usize, 0usize), |(max, run), c| {
            if c == '`' {
                (max.max(run + 1), run + 1)
            } else {
                (max, 0)
            }
        })
        .0;
    let fence = "`".repeat((longest_run + 1).max(3));
    format!("{fence}\n{}\n{fence}", code.trim_end_matches('\n'))
}

pub(crate) fn render_template_str<T: Serialize>(
    template_str: &str,
    context: &T,
) -> Result<String, MiniJinjaError> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    env.add_filter("code_fence", code_fence);
    env.add_template("template", template_str)?;
    let tmpl = env.get_template("template")?;
    let rendered = tmpl.render(MJValue::from_serialize(context))?;
    Ok(rendered.trim().to_string())
}

#[derive(Serialize)]
struct SummarizeContext {
    messages: String,
}

pub struct CompactionResult {
    pub conversation: Conversation,
    /// Billable usage of the summarization call, counting the raw model
    /// output even when it is rewritten to the rendered structured summary.
    pub usage: ProviderUsage,
    /// Estimated tokens of the agent-visible context retained after
    /// compaction.
    pub retained_context_tokens: i32,
}

pub async fn compact_messages(
    model: &dyn CompactionModel,
    settings: &CompactionSettings,
    conversation: &Conversation,
    manual_compact: bool,
) -> Result<CompactionResult> {
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

    // Turn-context events are agent-appended, never the message to preserve.
    let (preserved_user_message, preserved_idx, is_most_recent) = if !manual_compact {
        let found_msg = messages.iter().enumerate().rev().find_map(|(idx, msg)| {
            if !msg.is_agent_visible() || msg.is_turn_context() || !matches!(msg.role, Role::User) {
                return None;
            }

            let projected = msg.agent_visible_content();
            if !has_text_only(&projected) {
                return None;
            }

            let preserved = projected
                .content
                .into_iter()
                .filter(|content| matches!(content, MessageContent::Text(_)))
                .fold(
                    Message::user().with_metadata(MessageMetadata::agent_only()),
                    Message::with_content,
                );
            Some((idx, preserved))
        });

        if let Some((idx, msg)) = found_msg {
            let is_last = messages[idx + 1..].iter().all(Message::is_turn_context);
            (Some(msg), Some(idx), is_last)
        } else {
            (None, None, false)
        }
    } else {
        (None, None, false)
    };

    let (summary_message, summarization_usage) =
        do_compact(model, settings, messages.as_slice()).await?;

    let mut final_messages = Vec::new();
    for msg in messages.iter() {
        let updated_metadata = msg.metadata.clone().with_agent_invisible();
        final_messages.push(msg.clone().with_metadata(updated_metadata));
    }

    let summary_msg = summary_message.with_metadata(MessageMetadata::agent_only());

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
    let continuation_created = continuation_msg.created;

    let (merged_continuation, _issues) =
        merge_consecutive_messages(vec![summary_msg, continuation_msg]);
    final_messages.extend(merged_continuation);

    if let Some(mut user_msg) = preserved_user_message {
        user_msg.created = continuation_created;
        final_messages.push(user_msg);
    }

    // Carry the turn's own context event (it follows the preserved prompt) so
    // a mid-turn retry keeps it; anything earlier belongs to a previous turn.
    if let Some(carry_from) = preserved_idx.map(|idx| idx + 1) {
        if let Some(turn_context) = messages[carry_from..]
            .iter()
            .rev()
            .find(|msg| msg.is_turn_context() && msg.is_agent_visible())
        {
            let mut carried = turn_context.clone();
            carried.id = None;
            // Storage reloads order by created_timestamp; the copy must keep
            // its appended position, not resurface at the original event's time.
            if let Some(latest) = final_messages.iter().map(|msg| msg.created).max() {
                carried.created = carried.created.max(latest);
            }
            final_messages.push(carried);
        }
    }

    let conversation = Conversation::new_unvalidated(final_messages);
    let retained_context_tokens = match count_context_tokens(model, &conversation).await {
        Ok(tokens) => tokens,
        Err(error) => {
            warn!("Failed to count retained context tokens, using billable output tokens: {error}");
            summarization_usage.usage.output_tokens.unwrap_or(0)
        }
    };

    Ok(CompactionResult {
        conversation,
        usage: summarization_usage,
        retained_context_tokens,
    })
}

/// Estimate the tokens of the agent-visible conversation, counted the same way
/// as the fallback estimation in [`check_if_compaction_needed`].
pub async fn count_context_tokens(
    model: &dyn CompactionModel,
    conversation: &Conversation,
) -> Result<i32> {
    let mut total = 0usize;
    for message in conversation
        .messages()
        .iter()
        .filter(|message| message.is_agent_visible())
    {
        total += model
            .count_tokens("", std::slice::from_ref(message))
            .await?;
    }
    Ok(total.try_into()?)
}

/// Values of `threshold` outside (0, 1) disable auto-compaction. When
/// `known_total_tokens` is absent the agent-visible conversation is estimated
/// via [`CompactionModel::count_tokens`].
pub async fn check_if_compaction_needed(
    model: &dyn CompactionModel,
    conversation: &Conversation,
    context_limit: usize,
    known_total_tokens: Option<usize>,
    threshold: f64,
) -> Result<bool> {
    if threshold <= 0.0 || threshold >= 1.0 {
        return Ok(false);
    }

    let current_tokens = match known_total_tokens {
        Some(tokens) => tokens,
        None => count_context_tokens(model, conversation).await? as usize,
    };

    Ok(current_tokens as f64 / context_limit as f64 > threshold)
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

    for i in 0..num_to_remove {
        let offset = i / 2;
        if i % 2 == 0 {
            if middle > offset {
                indices_to_remove.push(tool_indices[middle - offset - 1]);
            }
        } else if middle + offset < tool_indices.len() {
            indices_to_remove.push(tool_indices[middle + offset]);
        }
    }

    messages
        .iter()
        .enumerate()
        .filter(|(i, _)| !indices_to_remove.contains(i))
        .map(|(_, msg)| msg)
        .collect()
}

async fn ensure_usage_tokens(
    model: &dyn CompactionModel,
    provider_usage: &mut ProviderUsage,
    system: &str,
    request: &[Message],
    response: &Message,
) -> Result<()> {
    if provider_usage.usage.input_tokens.is_none() {
        provider_usage.usage.input_tokens = Some(model.count_tokens(system, request).await? as i32);
    }
    if provider_usage.usage.output_tokens.is_none() {
        provider_usage.usage.output_tokens = Some(
            model
                .count_tokens("", std::slice::from_ref(response))
                .await? as i32,
        );
    }
    if let (Some(input), Some(output)) = (
        provider_usage.usage.input_tokens,
        provider_usage.usage.output_tokens,
    ) {
        provider_usage.usage.total_tokens = Some(input + output);
    }
    Ok(())
}

async fn do_compact(
    model: &dyn CompactionModel,
    settings: &CompactionSettings,
    messages: &[Message],
) -> Result<(Message, ProviderUsage)> {
    // Keep stale per-turn state out of the summary.
    let agent_visible_messages = Conversation::new_unvalidated(
        messages
            .iter()
            .filter(|msg| !msg.is_turn_context())
            .cloned(),
    )
    .agent_visible_messages();

    let removal_percentages = [0, 10, 20, 50, 100];

    for (attempt, &remove_percent) in removal_percentages.iter().enumerate() {
        let filtered_messages = filter_tool_responses(&agent_visible_messages, remove_percent);

        let messages_text = filtered_messages
            .iter()
            .map(|&msg| format_message_for_compacting(msg))
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = render_template_str(
            settings.compaction_prompt(),
            &SummarizeContext {
                messages: messages_text,
            },
        )?;

        let user_message = Message::user()
            .with_text("Please summarize the conversation history provided in the system prompt.");
        let summarization_request = vec![user_message];

        match model
            .complete(&system_prompt, &summarization_request, &[])
            .await
        {
            Ok((mut response, mut provider_usage)) => {
                response.role = Role::User;

                // Usage must reflect the raw model output (billable tokens),
                // so estimate before the response is rewritten to the smaller
                // rendered summary.
                ensure_usage_tokens(
                    model,
                    &mut provider_usage,
                    &system_prompt,
                    &summarization_request,
                    &response,
                )
                .await?;

                apply_structured_summary(&mut response, settings);

                return Ok((response, provider_usage));
            }
            Err(e) => {
                if matches!(e, ProviderError::ContextLengthExceeded(_)) {
                    if attempt < removal_percentages.len() - 1 {
                        continue;
                    }
                    return Err(anyhow::anyhow!(
                        "Failed to compact: context limit exceeded even after removing all tool responses"
                    ));
                }
                return Err(e.into());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Unexpected: exhausted all attempts without returning"
    ))
}

/// When the model didn't follow the structured output format, the raw
/// response text is kept unchanged as the summary.
fn apply_structured_summary(response: &mut Message, settings: &CompactionSettings) {
    let Some(summary) = StructuredSummary::parse(&response.as_concat_text()) else {
        return;
    };
    match summary.render(settings.summary_template()) {
        Ok(rendered) if !rendered.trim().is_empty() => {
            response.content = vec![MessageContent::text(rendered)];
        }
        Ok(_) => warn!(
            "Structured compaction summary rendered empty (broken template override?), keeping raw output"
        ),
        Err(e) => warn!(
            "Failed to render structured compaction summary, keeping raw output: {}",
            e
        ),
    }
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
                ActionRequiredData::ToolConfirmationResponse { id, .. } => Some(format!(
                    "action_required(tool_confirmation_response): {}",
                    id
                )),
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
            MessageContent::Error(error) => Some(format!("error: {}", error.message)),
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
    let mut tool_call_ids: Vec<String> = Vec::new();

    for msg in conversation.messages().iter() {
        if !msg.is_agent_visible() {
            continue;
        }
        for content in &msg.content {
            if let MessageContent::ToolRequest(req) = content {
                tool_call_ids.push(req.id.clone());
            }
        }
    }

    let eligible = tool_call_ids.len().saturating_sub(protect_last_n);
    if eligible <= cutoff + TOOLCALL_SUMMARIZATION_BATCH_SIZE {
        return Vec::new();
    }

    tool_call_ids
        .into_iter()
        .take(TOOLCALL_SUMMARIZATION_BATCH_SIZE)
        .collect()
}

fn agent_visible_tool_pair(conversation: &Conversation, tool_id: &str) -> Result<Vec<Message>> {
    let matching_messages = conversation
        .messages()
        .iter()
        .filter(|m| {
            m.content.iter().any(|c| match c {
                MessageContent::ToolRequest(req) => req.id == tool_id,
                MessageContent::ToolResponse(resp) => resp.id == tool_id,
                _ => false,
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let matching_messages =
        Conversation::new_unvalidated(matching_messages).agent_visible_messages();

    let has_request = matching_messages.iter().any(|message| {
        message.content.iter().any(
            |content| matches!(content, MessageContent::ToolRequest(request) if request.id == tool_id),
        )
    });
    let has_response = matching_messages.iter().any(|message| {
        message.content.iter().any(
            |content| matches!(content, MessageContent::ToolResponse(response) if response.id == tool_id),
        )
    });
    if !has_request || !has_response {
        return Err(anyhow::anyhow!(
            "No agent-visible tool pair found for tool id: {}",
            tool_id
        ));
    }
    Ok(matching_messages)
}

pub async fn summarize_tool_call(
    model: &dyn CompactionModel,
    conversation: &Conversation,
    tool_id: &str,
) -> Result<Message> {
    let matching_messages = agent_visible_tool_pair(conversation, tool_id)?;

    let formatted = matching_messages
        .iter()
        .map(format_message_for_compacting)
        .collect::<Vec<_>>()
        .join("\n");

    let summarization_request = vec![Message::user().with_text(formatted)];

    let system_prompt = indoc! {r#"
                Your task is to summarize a tool call & response pair to save tokens.

                Reply with a single message that describes what happened. Typically a tool call
                asks for something using a bunch of parameters and then the result is also some
                structured output. So the tool might ask to look up something on github and the
                reply might be a json document. So you could reply with something like:

                "A call to github was made to get the project status"

                if that is what it was.
            "#};

    let (mut response, _) = model
        .complete(system_prompt, &summarization_request, &[])
        .await?;

    response.role = Role::User;
    response.created = matching_messages.last().unwrap().created;
    response.metadata = MessageMetadata::agent_only();

    Ok(response.with_generated_id())
}

pub async fn summarize_tool_calls(
    model: &dyn CompactionModel,
    conversation: &Conversation,
    tool_ids: Vec<String>,
) -> Vec<(Message, String)> {
    let mut results = Vec::new();
    for tool_id in tool_ids {
        match summarize_tool_call(model, conversation, &tool_id).await {
            Ok(summary) => results.push((summary, tool_id)),
            Err(e) => warn!("Failed to summarize tool pair: {}", e),
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{Annotations, CallToolRequestParams, ContentBlock, TextContent};

    struct MockModel {
        response: Message,
        max_tool_responses_in_system: Option<usize>,
        captured_system: std::sync::Mutex<Option<String>>,
    }

    impl MockModel {
        fn new(response: Message) -> Self {
            Self {
                response,
                max_tool_responses_in_system: None,
                captured_system: std::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl CompactionModel for MockModel {
        async fn complete(
            &self,
            system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            *self.captured_system.lock().unwrap() = Some(system.to_string());
            if let Some(max) = self.max_tool_responses_in_system {
                let count = system.matches("tool_response:").count();
                if count > max {
                    return Err(ProviderError::ContextLengthExceeded(format!(
                        "Too many tool responses: {} > {}",
                        count, max
                    )));
                }
            }
            Ok((
                self.response.clone(),
                ProviderUsage::new("mock-model".to_string(), Default::default()),
            ))
        }

        async fn count_tokens(&self, system: &str, messages: &[Message]) -> Result<usize> {
            Ok(system.len() / 4
                + messages
                    .iter()
                    .map(|m| m.as_concat_text().len() / 4 + 4)
                    .sum::<usize>())
        }
    }

    #[tokio::test]
    async fn structured_summary_is_rendered_and_usage_survives_rewrite() {
        let structured_response = r#"<analysis>User asked to fix a bug; I patched parser.rs.</analysis>
```json
{
  "user_intent": ["Fix the parser bug"],
  "files": [{"path": "src/parser.rs", "summary": "Fixed off-by-one"}],
  "pending_tasks": ["Add a regression test"],
  "current_work": "Writing the regression test"
}
```"#;
        let model = MockModel::new(Message::assistant().with_text(structured_response));
        let conversation = Conversation::new_unvalidated(vec![
            Message::user().with_text("fix the parser bug"),
            Message::assistant().with_text("Looking into it"),
        ]);

        let compaction =
            compact_messages(&model, &CompactionSettings::default(), &conversation, true)
                .await
                .unwrap();

        let summary_text = compaction.conversation.agent_visible_messages()[0].as_concat_text();
        assert!(summary_text.contains("# Conversation Summary"));
        assert!(summary_text.contains("## User Intent"));
        assert!(summary_text.contains("- Fix the parser bug"));
        assert!(summary_text.contains("### src/parser.rs"));
        assert!(!summary_text.contains("```json"));
        assert!(!summary_text.contains("<analysis>"));
        assert!(compaction.retained_context_tokens > 0);
        assert!(
            compaction.usage.usage.output_tokens.is_some(),
            "billable output tokens must survive the rewrite"
        );
    }

    #[tokio::test]
    async fn progressive_removal_recovers_from_context_exceeded() {
        let mut model = MockModel::new(Message::assistant().with_text("<mock summary>"));
        model.max_tool_responses_in_system = Some(2);

        let mut messages = vec![Message::user().with_text("start")];
        for i in 0..10 {
            messages.push(Message::assistant().with_tool_request(
                format!("tool_{}", i),
                Ok(CallToolRequestParams::new("read_file")),
            ));
            messages.push(Message::user().with_tool_response(
                format!("tool_{}", i),
                Ok(rmcp::model::CallToolResult::success(vec![
                    ContentBlock::text(format!("response{}", i)),
                ])),
            ));
        }

        let result = compact_messages(
            &model,
            &CompactionSettings::default(),
            &Conversation::new_unvalidated(messages),
            false,
        )
        .await;

        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[tokio::test]
    async fn preserved_user_message_keeps_audience_projection_after_compaction() {
        let annotated_text = |text: &str, audience| {
            MessageContent::Text(
                TextContent::new(text)
                    .with_annotations(Annotations::default().with_audience(audience)),
            )
        };
        let current_request = Message::user()
            .with_text("visible current request")
            .with_content(annotated_text("user-only secret", vec![Role::User]))
            .with_content(annotated_text(
                "assistant-only preprompt",
                vec![Role::Assistant],
            ));
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("earlier request"),
            Message::assistant().with_text("earlier response"),
            current_request,
        ]);
        let model = MockModel::new(Message::assistant().with_text("summary"));

        let compacted =
            compact_messages(&model, &CompactionSettings::default(), &conversation, false)
                .await
                .unwrap()
                .conversation;

        let agent_text = compacted
            .agent_visible_messages()
            .iter()
            .map(Message::as_concat_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(agent_text.contains("visible current request"));
        assert!(agent_text.contains("assistant-only preprompt"));
        assert!(!agent_text.contains("user-only secret"));

        let user_text = compacted
            .user_visible_messages()
            .iter()
            .map(Message::as_concat_text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(user_text.contains("user-only secret"));
        assert!(!user_text.contains("assistant-only preprompt"));
    }

    #[tokio::test]
    async fn preserved_user_message_skips_turn_context_events() {
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("earlier request"),
            Message::assistant().with_text("earlier response"),
            Message::user().with_text("the real current request"),
            Message::user()
                .with_text("<turn-context>frozen block</turn-context>")
                .with_metadata(MessageMetadata::agent_only().with_turn_context()),
        ]);
        let model = MockModel::new(Message::assistant().with_text("summary"));

        let compacted =
            compact_messages(&model, &CompactionSettings::default(), &conversation, false)
                .await
                .unwrap()
                .conversation;

        let preserved: Vec<_> = compacted
            .messages()
            .iter()
            .filter(|message| message.is_agent_visible() && message.role == Role::User)
            .collect();
        let turn_context_events: Vec<_> = preserved
            .iter()
            .filter(|message| message.is_turn_context())
            .collect();
        let [carried_block] = turn_context_events.as_slice() else {
            panic!("expected exactly one carried turn-context event");
        };
        assert!(
            carried_block.as_concat_text().contains("frozen block"),
            "the turn's context event must be carried forward for the mid-turn retry"
        );
        let last = compacted.messages().last().unwrap();
        assert!(
            last.is_turn_context() && last.is_agent_visible(),
            "the carried event must trail the preserved user message"
        );
        let user_prompt = preserved[preserved.len() - 2];
        assert!(
            user_prompt
                .as_concat_text()
                .contains("the real current request"),
            "the user's prompt must survive compaction verbatim"
        );
        assert!(!user_prompt.is_turn_context());

        let continuation = compacted
            .messages()
            .iter()
            .find(|message| message.role == Role::Assistant && message.is_agent_visible())
            .unwrap()
            .as_concat_text();
        assert!(
            continuation.contains(CONVERSATION_CONTINUATION_TEXT),
            "a trailing turn-context event must not demote the compaction to a tool-loop continuation"
        );
    }

    #[tokio::test]
    async fn stale_turn_context_from_an_earlier_turn_is_not_carried() {
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("earlier request"),
            Message::user()
                .with_text("<turn-context>stale block</turn-context>")
                .with_metadata(MessageMetadata::agent_only().with_turn_context()),
            Message::assistant().with_text("earlier response"),
            Message::user().with_text("the new prompt"),
        ]);
        let model = MockModel::new(Message::assistant().with_text("summary"));

        let compacted =
            compact_messages(&model, &CompactionSettings::default(), &conversation, false)
                .await
                .unwrap()
                .conversation;

        assert!(
            !compacted
                .messages()
                .iter()
                .any(|message| message.is_agent_visible() && message.is_turn_context()),
            "pre-turn compaction must not resurrect a previous turn's context event"
        );
        let last = compacted.messages().last().unwrap();
        assert_eq!(last.as_concat_text(), "the new prompt");
    }

    #[tokio::test]
    async fn carried_turn_context_is_appended_after_the_newest_retained_message() {
        let model = MockModel::new(Message::assistant().with_text("summary"));
        let mut prompt = Message::user().with_text("the real current request");
        prompt.created -= 3600;
        let mut block = Message::user()
            .with_text("<turn-context>frozen block</turn-context>")
            .with_metadata(MessageMetadata::agent_only().with_turn_context());
        block.created -= 3600;
        let conversation = Conversation::new_unvalidated([prompt, block]);

        let compacted =
            compact_messages(&model, &CompactionSettings::default(), &conversation, false)
                .await
                .unwrap()
                .conversation;

        let messages = compacted.messages();
        let carried = messages.last().unwrap();
        assert!(carried.is_turn_context());
        assert!(carried.id.is_none());
        let newest_other = messages[..messages.len() - 1]
            .iter()
            .map(|message| message.created)
            .max()
            .unwrap();
        assert!(
            carried.created >= newest_other,
            "storage reloads order by created_timestamp, so the copy must not resurface at the original event's time"
        );
    }

    #[tokio::test]
    async fn summarizer_input_excludes_turn_context_events() {
        let model = MockModel::new(Message::assistant().with_text("summary"));
        let turn_context = |text: &str| {
            Message::user()
                .with_text(text)
                .with_metadata(MessageMetadata::agent_only().with_turn_context())
        };
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("please refactor the parser"),
            turn_context("<turn-context>cwd /old/dir</turn-context>"),
            Message::assistant().with_text("working on it"),
            turn_context("<turn-context>cwd /new/dir</turn-context>"),
        ]);

        let compacted =
            compact_messages(&model, &CompactionSettings::default(), &conversation, false)
                .await
                .unwrap()
                .conversation;

        let system = model.captured_system.lock().unwrap().clone().unwrap();
        assert!(system.contains("please refactor the parser"));
        assert!(
            !system.contains("/old/dir") && !system.contains("/new/dir"),
            "turn-context events must not reach the summarizer as dialogue"
        );

        let carried = compacted.messages().last().unwrap();
        assert!(
            carried.is_turn_context() && carried.as_concat_text().contains("/new/dir"),
            "the newest turn-context event must still be carried forward"
        );
    }

    #[tokio::test]
    async fn tool_pair_summary_projects_audiences_and_rejects_hidden_pairs() {
        let model = MockModel::new(Message::assistant().with_text("summary"));
        let conversation =
            Conversation::new_unvalidated([
                Message::assistant()
                    .with_tool_request("tool_0", Ok(CallToolRequestParams::new("read_file"))),
                Message::user().with_tool_response(
                    "tool_0",
                    Ok(rmcp::model::CallToolResult::success(vec![
                        ContentBlock::text("visible result"),
                        ContentBlock::Text(TextContent::new("user-only secret").with_annotations(
                            Annotations::default().with_audience(vec![Role::User]),
                        )),
                    ])),
                ),
            ]);

        let formatted = agent_visible_tool_pair(&conversation, "tool_0")
            .unwrap()
            .iter()
            .map(format_message_for_compacting)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(formatted.contains("visible result"));
        assert!(!formatted.contains("user-only secret"));

        summarize_tool_call(&model, &conversation, "tool_0")
            .await
            .unwrap();

        let hidden = Conversation::new_unvalidated([
            Message::assistant()
                .with_tool_request("tool_1", Ok(CallToolRequestParams::new("read_file"))),
            Message::user()
                .with_tool_response(
                    "tool_1",
                    Ok(rmcp::model::CallToolResult::success(vec![
                        ContentBlock::text("secret"),
                    ])),
                )
                .with_metadata(MessageMetadata::user_only()),
        ]);
        let error = summarize_tool_call(&model, &hidden, "tool_1")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("No agent-visible tool pair"));
    }

    #[test]
    fn tool_ids_to_summarize_respects_cutoff_batch_and_protection() {
        let build = |n: usize| {
            let mut messages = vec![Message::user().with_text("hello")];
            for i in 0..n {
                messages.push(Message::assistant().with_tool_request(
                    format!("call{}", i),
                    Ok(CallToolRequestParams::new("read_file")),
                ));
                messages.push(Message::user().with_tool_response(
                    format!("call{}", i),
                    Ok(rmcp::model::CallToolResult::success(vec![
                        ContentBlock::text("content"),
                    ])),
                ));
            }
            Conversation::new_unvalidated(messages)
        };

        assert!(tool_ids_to_summarize(&build(15), 5, 0).is_empty());

        let result = tool_ids_to_summarize(&build(16), 5, 0);
        assert_eq!(result.len(), TOOLCALL_SUMMARIZATION_BATCH_SIZE);
        assert_eq!(result[0], "call0");
        assert_eq!(result[9], "call9");

        assert!(tool_ids_to_summarize(&build(20), 2, 8).is_empty());
        assert_eq!(
            tool_ids_to_summarize(&build(20), 2, 7).len(),
            TOOLCALL_SUMMARIZATION_BATCH_SIZE
        );
    }

    #[test]
    fn compute_tool_call_cutoff_scales_with_context() {
        assert_eq!(compute_tool_call_cutoff(128_000, 0.8), 15);
        assert_eq!(compute_tool_call_cutoff(1_000_000, 0.8), 120);
        assert_eq!(compute_tool_call_cutoff(10_000, 0.8), 10);
        assert_eq!(compute_tool_call_cutoff(10_000_000, 0.8), 500);
        assert_eq!(compute_tool_call_cutoff(200_000, 0.0), 24);
    }
}
