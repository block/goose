pub mod structured;

use crate::context_mgmt::structured::{DifficultyLevel, ForwardDifficulty, StructuredSummary};
use crate::conversation::message::{ActionRequiredData, MessageMetadata};
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::{merge_consecutive_messages, Conversation};
use crate::prompt_template::render_template;
use crate::providers::base::Provider;
#[cfg(test)]
use crate::providers::base::{stream_from_single_message, MessageStream};
use crate::{config::Config, token_counter::create_token_counter};
use anyhow::Result;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use goose_providers::thinking::ThinkingEffort;
use indoc::indoc;
use rmcp::model::Role;
use serde::Serialize;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;
use tracing::log::warn;

pub const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;

const TOOLCALL_SUMMARIZATION_BATCH_SIZE: usize = 10;

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

pub struct CompactionResult {
    pub conversation: Conversation,
    /// Billable usage of the summarization call, counting the raw model
    /// output even when it is rewritten to the rendered structured summary.
    pub usage: ProviderUsage,
    /// Estimated tokens of the agent-visible context retained after
    /// compaction. Smaller than the billable output when the raw response was
    /// rewritten to the rendered structured summary.
    pub retained_context_tokens: i32,
    /// The compaction model's estimate of how hard the remaining work is.
    /// `None` when the model omitted it, produced garbage, or the response
    /// was unstructured - the summary itself is unaffected either way.
    pub forward_difficulty: Option<ForwardDifficulty>,
}

/// Advisory nudge pairing the compaction model's difficulty estimate with the
/// thinking-effort setting that matches it. Delivered to clients as a signal;
/// never applied automatically - the user stays in control.
#[derive(Debug, Clone)]
pub struct EffortRecommendation {
    pub difficulty: DifficultyLevel,
    pub reason: String,
    pub recommended_effort: ThinkingEffort,
    /// The provider-effective effort the session runs at, which the
    /// recommendation is a change from - not the raw configured value.
    pub current_effort: ThinkingEffort,
}

const OBSERVED_THINKING_WINDOW: usize = 8;
const OBSERVED_THINKING_MIN_TURNS: usize = 3;
/// A downgrade needs the recent turns to be nearly thinking-free; a genuinely
/// hard stretch at high effort runs well above this on every provider.
const OBSERVED_THINKING_IDLE_MAX_UTILIZATION: f64 = 0.05;

/// Thinking activity over the most recent assistant turns. Within the window
/// only turns with a provider-reported reasoning token count on per-message
/// usage (including an explicit 0) are sampled as idle evidence. When a
/// provider (or proxy) strips the breakdown, reasoning is invisible: visible
/// thinking content still adds its estimated size so it can block a
/// downgrade, but an estimate never vouches for idleness - Claude models
/// surface a summary that is a fraction of what `output_tokens` charges.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObservedThinking {
    pub sampled_turns: usize,
    pub thinking_tokens: i64,
    pub output_tokens: i64,
}

impl ObservedThinking {
    pub fn utilization(&self) -> f64 {
        self.thinking_tokens as f64 / self.output_tokens.max(1) as f64
    }

    fn shows_idle_reasoning(&self) -> bool {
        self.sampled_turns >= OBSERVED_THINKING_MIN_TURNS
            && self.utilization() <= OBSERVED_THINKING_IDLE_MAX_UTILIZATION
    }
}

fn message_thinking_estimate(message: &Message) -> i64 {
    message
        .content
        .iter()
        .map(|content| match content {
            MessageContent::Thinking(thinking) => (thinking.thinking.len() / 4) as i64,
            // The payload is encrypted so the true count is unknowable; the
            // blob length over-estimates it, which can only block a downgrade.
            MessageContent::RedactedThinking(redacted) => (redacted.data.len() / 4) as i64,
            _ => 0,
        })
        .sum()
}

pub fn observed_thinking(
    conversation: &Conversation,
    provider_name: &str,
    model_name: &str,
) -> ObservedThinking {
    let mut observed = ObservedThinking::default();
    let mut examined = 0;
    for message in conversation
        .messages()
        .iter()
        .rev()
        .filter(|m| m.role == Role::Assistant)
    {
        // Telemetry only vouches for the model that produced it: the scan
        // stops at the first turn attributed to a different provider or model.
        if message
            .metadata
            .inference
            .as_ref()
            .is_some_and(|inference| {
                inference.provider != provider_name || inference.requested_model != model_name
            })
        {
            break;
        }
        // A turn persisted without usage (e.g. cancelled mid-stream) still
        // surfaces its thinking content as blocking evidence.
        let Some(usage) = &message.metadata.usage else {
            observed.thinking_tokens += message_thinking_estimate(message);
            continue;
        };
        // Every usage-bearing turn consumes the window, even without an
        // output count (Gemini omits candidatesTokenCount on thinking-only
        // turns), so stale telemetry from before a provider switch can never
        // vouch for newer turns whose reasoning is invisible. Evidence-free
        // turns are consumed but not sampled.
        examined += 1;
        match usage.thinking_tokens {
            Some(reported) => {
                observed.sampled_turns += 1;
                observed.output_tokens += usage.output_tokens.unwrap_or(0) as i64;
                observed.thinking_tokens += reported as i64;
            }
            None => observed.thinking_tokens += message_thinking_estimate(message),
        }
        if examined == OBSERVED_THINKING_WINDOW {
            break;
        }
    }
    observed
}

fn one_step_down(effort: ThinkingEffort) -> Option<ThinkingEffort> {
    match effort {
        ThinkingEffort::Max => Some(ThinkingEffort::High),
        ThinkingEffort::High => Some(ThinkingEffort::Medium),
        ThinkingEffort::Medium => Some(ThinkingEffort::Low),
        ThinkingEffort::Low | ThinkingEffort::Off => None,
    }
}

/// A recommendation is made only when the provider's request formatting
/// actually maps a `thinking_effort` for the session's model
/// ([`Provider::maps_thinking_effort`]) and the estimate calls for a
/// different effort than the provider actually runs with
/// ([`Provider::effective_thinking_effort`]). An explicit `Off` (the user
/// opted out) is second-guessed only by a `High` estimate; sessions pinning
/// an explicit thinking budget are never nudged (the budget overrides any
/// effort); `Low` raise recommendations are never made. Providers may
/// coalesce adjacent effort levels, so a nudge can still be a behavioral
/// no-op on some models - acceptable for an advisory hint.
///
/// Downgrades are held to a stricter bar than raises because the difficulty
/// estimator's errors are under-ratings: a `Low` estimate alone nudges
/// nothing. Lowering requires the estimate AND provider-reported thinking
/// telemetry agreeing the recent turns were nearly reasoning-free, steps down
/// a single level (never to `Off`), and never second-guesses an effort the
/// user pinned for the session (a session value that differs from the global
/// default; a matching value is just the inherited default).
pub fn build_effort_recommendation(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    difficulty: &ForwardDifficulty,
    current_effort: Option<ThinkingEffort>,
    observed: Option<&ObservedThinking>,
) -> Option<EffortRecommendation> {
    // Capability is the provider's call (overrides handle e.g. Bedrock's
    // `openai.` prefix); only an explicit reasoning opt-out short-circuits it.
    if model_config.reasoning == Some(false) || !provider.maps_thinking_effort(model_config) {
        return None;
    }
    if current_effort == Some(ThinkingEffort::Off) && difficulty.level != DifficultyLevel::High {
        return None;
    }
    // Explicit reasoning request params pin the request's reasoning behavior
    // directly (each formatter lets them override any effort-derived value),
    // so an effort nudge would not change the request.
    const REASONING_PIN_PARAMS: [&str; 4] = [
        "budget_tokens",
        "thinking_budget",
        "reasoning",
        "reasoning_effort",
    ];
    if REASONING_PIN_PARAMS.iter().any(|param| {
        model_config
            .request_param::<serde_json::Value>(param)
            .is_some()
    }) {
        return None;
    }
    let recommended = match difficulty.level {
        DifficultyLevel::Low => ThinkingEffort::Low,
        DifficultyLevel::Medium => ThinkingEffort::Medium,
        DifficultyLevel::High => ThinkingEffort::High,
    };
    let effective_current = provider.effective_thinking_effort(model_config, current_effort);
    if recommended > effective_current {
        // Only reachable when the model defaults to thinking-off (Gemini 3,
        // most Claude models): enabling thinking because the remaining work
        // looks easy is noise.
        if recommended == ThinkingEffort::Low {
            return None;
        }
        return Some(EffortRecommendation {
            difficulty: difficulty.level,
            reason: difficulty.reason.clone(),
            recommended_effort: recommended,
            current_effort: effective_current,
        });
    }

    if difficulty.level != DifficultyLevel::Low {
        return None;
    }
    // `ModelConfig::new` bakes the global default effort into every session's
    // request params, so a session value equal to the global setting is an
    // inherited default, not a per-session choice. Only a differing value
    // marks an effort the user pinned for this session.
    let session_effort = model_config.thinking_effort();
    if session_effort.is_some() && session_effort != Config::global().get_goose_thinking_effort() {
        return None;
    }
    if !observed.is_some_and(ObservedThinking::shows_idle_reasoning) {
        return None;
    }
    let target = one_step_down(effective_current)?;
    // A downgrade promises savings, so it must actually lower what the
    // provider runs with (Gemini 3 coalesces Low back up to Medium).
    if provider.effective_thinking_effort(model_config, Some(target)) >= effective_current {
        return None;
    }
    Some(EffortRecommendation {
        difficulty: difficulty.level,
        reason: difficulty.reason.clone(),
        recommended_effort: target,
        current_effort: effective_current,
    })
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
pub async fn compact_messages(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
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

    // Find and preserve the most recent user message for non-manual compacts
    let (preserved_user_message, is_most_recent) = if !manual_compact {
        let found_msg = messages.iter().enumerate().rev().find_map(|(idx, msg)| {
            if !msg.is_agent_visible() || !matches!(msg.role, rmcp::model::Role::User) {
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
            let is_last = idx == messages.len() - 1;
            (Some(msg), is_last)
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    let messages_to_compact = messages.as_slice();

    let (summary_message, summarization_usage, forward_difficulty) =
        do_compact(provider, model_config, session_id, messages_to_compact).await?;

    // Create the final message list with updated visibility metadata:
    // 1. Original messages become user_visible but not agent_visible
    // 2. Summary message becomes agent_visible but not user_visible
    // 3. Assistant messages to continue the conversation are also agent_visible but not user_visible
    let mut final_messages = Vec::new();

    for msg in messages_to_compact {
        let updated_metadata = msg.metadata.clone().with_agent_invisible();
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
        final_messages.push(user_msg);
    }

    let conversation = Conversation::new_unvalidated(final_messages);
    let retained_context_tokens = count_retained_context_tokens(&conversation)
        .await
        .or(summarization_usage.usage.output_tokens)
        .unwrap_or(0);

    Ok(CompactionResult {
        conversation,
        usage: summarization_usage,
        retained_context_tokens,
        forward_difficulty,
    })
}

/// Estimate the tokens of the agent-visible conversation retained after
/// compaction, counted the same way as the fallback estimation in
/// `check_if_compaction_needed`.
async fn count_retained_context_tokens(conversation: &Conversation) -> Option<i32> {
    match create_token_counter().await {
        Ok(counter) => {
            let total: usize = conversation
                .messages()
                .iter()
                .filter(|m| m.is_agent_visible())
                .map(|msg| counter.count_chat_tokens("", std::slice::from_ref(msg), &[]))
                .sum();
            Some(total as i32)
        }
        Err(e) => {
            warn!(
                "Failed to count retained context tokens, using billable output tokens: {}",
                e
            );
            None
        }
    }
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

    let model_config = session
        .model_config
        .clone()
        .unwrap_or_else(|| ModelConfig::new("unknown"));
    let context_limit = provider
        .get_context_limit(&model_config)
        .await
        .unwrap_or_else(|_| model_config.context_limit());

    let (current_tokens, _token_source) = match session.usage.total_tokens {
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

async fn do_compact(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    messages: &[Message],
) -> Result<(Message, ProviderUsage, Option<ForwardDifficulty>), anyhow::Error> {
    let agent_visible_messages =
        Conversation::new_unvalidated(messages.iter().cloned()).agent_visible_messages();

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

        let system_prompt = render_template("compaction.md", &context)?;

        let user_message = Message::user()
            .with_text("Please summarize the conversation history provided in the system prompt.");
        let summarization_request = vec![user_message];

        match crate::model_config::complete_fast(
            provider,
            model_config,
            session_id,
            &system_prompt,
            &summarization_request,
            &[],
        )
        .await
        {
            Ok((mut response, mut provider_usage)) => {
                response.role = Role::User;

                // Usage must reflect the raw model output (billable tokens),
                // so estimate before the response is rewritten to the smaller
                // rendered summary.
                crate::providers::usage_estimator::ensure_usage_tokens(
                    &mut provider_usage,
                    &system_prompt,
                    &summarization_request,
                    &response,
                    &[],
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to ensure usage tokens: {}", e))?;

                let forward_difficulty = apply_structured_summary(&mut response);

                return Ok((response, provider_usage, forward_difficulty));
            }
            Err(e) => {
                if matches!(e, ProviderError::ContextLengthExceeded(_)) {
                    if attempt < removal_percentages.len() - 1 {
                        continue;
                    } else {
                        return Err(anyhow::anyhow!(
                            "Failed to compact: context limit exceeded even after removing all tool responses"
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

/// When the model didn't follow the structured output format (schema-ignoring
/// models, user-customized prompts), the raw response text is kept unchanged
/// as the summary. The difficulty estimate is returned even when rendering
/// falls back to raw output: it parsed fine, and the raw text is still a
/// complete summary. On these raw-fallback paths any difficulty text the
/// model emitted remains part of the agent-visible summary, like every other
/// structured field; only the rendered template excludes it.
fn apply_structured_summary(response: &mut Message) -> Option<ForwardDifficulty> {
    let summary = StructuredSummary::parse(&response.as_concat_text())?;
    let forward_difficulty = summary.forward_difficulty.clone();
    match summary.render() {
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
    forward_difficulty
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
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    conversation: &Conversation,
    tool_id: &str,
) -> Result<Message> {
    let matching_messages = agent_visible_tool_pair(conversation, tool_id)?;

    let formatted = matching_messages
        .iter()
        .map(format_message_for_compacting)
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

    let (mut response, _) = crate::model_config::complete_fast(
        provider,
        model_config,
        session_id,
        system_prompt,
        &summarization_request,
        &[],
    )
    .await?;

    response.role = Role::User;
    response.created = matching_messages.last().unwrap().created;
    response.metadata = MessageMetadata::agent_only();

    Ok(response.with_generated_id())
}

pub fn maybe_summarize_tool_pairs(
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
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
            match summarize_tool_call(
                provider.as_ref(),
                &model_config,
                &session_id,
                &conversation,
                &tool_id,
            )
            .await
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
    use async_trait::async_trait;
    use goose_providers::conversation::token_usage::Usage;
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
                    request_params: None,
                    reasoning: None,
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
        fn get_name(&self) -> &str {
            "mock"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
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

        async fn get_context_limit(
            &self,
            _model_config: &ModelConfig,
        ) -> Result<usize, ProviderError> {
            Ok(self.config.context_limit())
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
        let model_config = provider.config.clone();
        let compaction = compact_messages(
            &provider,
            &model_config,
            "test-session-id",
            &conversation,
            false,
        )
        .await
        .unwrap();

        let agent_conversation = compaction.conversation.agent_visible_messages();

        let _ = Conversation::new(agent_conversation)
            .expect("compaction should produce a valid conversation");
    }

    #[tokio::test]
    async fn test_structured_summary_is_rendered() {
        let structured_response = r#"<analysis>User asked to fix a bug; I patched parser.rs.</analysis>
```json
{
  "user_intent": ["Fix the parser bug"],
  "files": [{"path": "src/parser.rs", "summary": "Fixed off-by-one"}],
  "pending_tasks": ["Add a regression test"],
  "current_work": "Writing the regression test",
  "forward_difficulty": {"reason": "Only a routine regression test remains", "level": "low"}
}
```"#;
        let provider =
            MockProvider::new(Message::assistant().with_text(structured_response), 100_000);
        let conversation = Conversation::new_unvalidated(vec![
            Message::user().with_text("fix the parser bug"),
            Message::assistant().with_text("Looking into it"),
        ]);

        let model_config = provider.config.clone();
        let compaction = compact_messages(
            &provider,
            &model_config,
            "test-session-id",
            &conversation,
            true,
        )
        .await
        .unwrap();

        let summary_text = compaction.conversation.agent_visible_messages()[0].as_concat_text();
        assert!(summary_text.contains("# Conversation Summary"));
        assert!(summary_text.contains("## User Intent"));
        assert!(summary_text.contains("- Fix the parser bug"));
        assert!(summary_text.contains("### src/parser.rs"));
        assert!(
            !summary_text.contains("```json"),
            "raw JSON should be replaced"
        );
        assert!(
            !summary_text.contains("<analysis>"),
            "analysis scratchpad should be dropped"
        );
        assert!(compaction.retained_context_tokens > 0);
        assert!(
            compaction.usage.usage.output_tokens.is_some(),
            "billable output tokens must survive the rewrite"
        );
        let difficulty = compaction
            .forward_difficulty
            .expect("difficulty estimate should surface on the compaction result");
        assert_eq!(difficulty.level, DifficultyLevel::Low);
        assert!(
            !summary_text.contains("Only a routine regression test remains"),
            "difficulty must not leak into the agent-visible summary"
        );
    }

    #[tokio::test]
    async fn retained_context_counts_preserved_user_message() {
        async fn retained(final_user_text: &str) -> i32 {
            let provider =
                MockProvider::new(Message::assistant().with_text("<mock summary>"), 100_000);
            let conversation = Conversation::new_unvalidated(vec![
                Message::user().with_text("start"),
                Message::assistant().with_text("ok"),
                Message::user().with_text(final_user_text),
            ]);
            let model_config = provider.config.clone();
            compact_messages(
                &provider,
                &model_config,
                "test-session-id",
                &conversation,
                false,
            )
            .await
            .unwrap()
            .retained_context_tokens
        }

        let short = retained("continue").await;
        let long = retained(&"long preserved user message ".repeat(200)).await;
        assert!(
            long > short,
            "the preserved user message must be part of the retained context ({short} vs {long})"
        );
    }

    #[tokio::test]
    async fn preserved_user_message_keeps_audience_projection_after_compaction() {
        use rmcp::model::{RawTextContent, Role};

        let annotated_text = |text: &str, audience| {
            MessageContent::Text(
                RawTextContent {
                    text: text.to_string(),
                    meta: None,
                }
                .no_annotation()
                .with_audience(audience),
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
        let provider = MockProvider::new(Message::assistant().with_text("summary"), 1000);

        let compacted = compact_messages(
            &provider,
            &provider.config,
            "test-session-id",
            &conversation,
            false,
        )
        .await
        .unwrap()
        .conversation;

        let preserved_copies = compacted
            .messages()
            .iter()
            .filter(|message| message.as_concat_text().contains("visible current request"))
            .collect::<Vec<_>>();
        assert_eq!(preserved_copies.len(), 2);
        let archived = preserved_copies
            .iter()
            .find(|message| message.is_user_visible())
            .unwrap();
        assert!(!archived.is_agent_visible());
        assert!(archived.as_concat_text().contains("user-only secret"));
        let replay = preserved_copies
            .iter()
            .find(|message| message.is_agent_visible())
            .unwrap();
        assert!(!replay.is_user_visible());
        assert!(replay.as_concat_text().contains("assistant-only preprompt"));
        assert!(!replay.as_concat_text().contains("user-only secret"));

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
    async fn tool_pair_summary_projects_nested_audiences_before_provider_input() {
        let provider = MockProvider::new(Message::assistant().with_text("summary"), 1000);
        let conversation = Conversation::new_unvalidated([
            Message::assistant()
                .with_tool_request("tool_0", Ok(CallToolRequestParams::new("read_file"))),
            Message::user().with_tool_response(
                "tool_0",
                Ok(rmcp::model::CallToolResult::success(vec![
                    RawContent::text("visible result").no_annotation(),
                    RawContent::text("user-only secret")
                        .no_annotation()
                        .with_audience(vec![Role::User]),
                ])),
            ),
        ]);

        let projected = agent_visible_tool_pair(&conversation, "tool_0").unwrap();
        let formatted = projected
            .iter()
            .map(format_message_for_compacting)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(formatted.contains("visible result"));
        assert!(!formatted.contains("user-only secret"));

        let user_only_conversation = Conversation::new_unvalidated([
            Message::assistant()
                .with_tool_request("tool_1", Ok(CallToolRequestParams::new("read_file"))),
            Message::user().with_tool_response(
                "tool_1",
                Ok(rmcp::model::CallToolResult::success(vec![
                    RawContent::text("user-only secret")
                        .no_annotation()
                        .with_audience(vec![Role::User]),
                ])),
            ),
        ]);
        let user_only_formatted = agent_visible_tool_pair(&user_only_conversation, "tool_1")
            .unwrap()
            .iter()
            .map(format_message_for_compacting)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!user_only_formatted.contains("user-only secret"));

        summarize_tool_call(
            &provider,
            &provider.config,
            "test-session-id",
            &conversation,
            "tool_0",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn tool_pair_summary_rejects_agent_hidden_response() {
        let provider = MockProvider::new(Message::assistant().with_text("summary"), 1000);
        let conversation = Conversation::new_unvalidated([
            Message::assistant()
                .with_tool_request("tool_0", Ok(CallToolRequestParams::new("read_file"))),
            Message::user()
                .with_tool_response(
                    "tool_0",
                    Ok(rmcp::model::CallToolResult::success(vec![
                        RawContent::text("user-only secret").no_annotation(),
                    ])),
                )
                .with_metadata(MessageMetadata::user_only()),
        ]);

        let error = summarize_tool_call(
            &provider,
            &provider.config,
            "test-session-id",
            &conversation,
            "tool_0",
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("No agent-visible tool pair"));
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
        let model_config = provider.config.clone();
        let result = compact_messages(
            &provider,
            &model_config,
            "test-session-id",
            &conversation,
            false,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should succeed with progressive removal: {:?}",
            result.err()
        );
    }

    fn estimate(level: DifficultyLevel) -> ForwardDifficulty {
        ForwardDifficulty {
            level,
            reason: "because".to_string(),
        }
    }

    fn nudge(
        model: &ModelConfig,
        level: DifficultyLevel,
        current: Option<ThinkingEffort>,
    ) -> Option<EffortRecommendation> {
        nudge_observing(model, level, current, None)
    }

    fn nudge_observing(
        model: &ModelConfig,
        level: DifficultyLevel,
        current: Option<ThinkingEffort>,
        observed: Option<&ObservedThinking>,
    ) -> Option<EffortRecommendation> {
        let provider = MockProvider::new(Message::assistant(), 100_000);
        build_effort_recommendation(&provider, model, &estimate(level), current, observed)
    }

    fn idle_reasoning() -> ObservedThinking {
        ObservedThinking {
            sampled_turns: 5,
            thinking_tokens: 20,
            output_tokens: 2000,
        }
    }

    #[test]
    fn effort_recommendation_requires_a_model_that_maps_thinking_effort() {
        let non_reasoning = ModelConfig::new("plain-model");
        assert!(
            nudge(&non_reasoning, DifficultyLevel::High, None).is_none(),
            "must never suggest a setting the current model can't use"
        );

        let mut unmapped = ModelConfig::new("deepseek-reasoner");
        unmapped.reasoning = Some(true);
        assert!(
            nudge(&unmapped, DifficultyLevel::High, None).is_none(),
            "the OpenAI-compatible formatter drops thinking_effort for this model, \
             so applying the nudge would change nothing"
        );

        let mut opted_out = ModelConfig::new("gpt-5");
        opted_out.reasoning = Some(false);
        assert!(
            nudge(&opted_out, DifficultyLevel::High, None).is_none(),
            "an explicit reasoning opt-out beats the capability check"
        );
    }

    #[test]
    fn effort_recommendation_uses_model_family_defaults_when_effort_is_unset() {
        let claude = ModelConfig::new("claude-sonnet-4-5-20250929");
        let recommendation = nudge(&claude, DifficultyLevel::High, None)
            .expect("a default Anthropic session runs with thinking disabled, so high is a raise");
        assert_eq!(recommendation.recommended_effort, ThinkingEffort::High);
        assert_eq!(recommendation.current_effort, ThinkingEffort::Off);

        let always_on = ModelConfig::new("claude-fable-5");
        assert!(
            nudge(&always_on, DifficultyLevel::High, None).is_none(),
            "an always-on adaptive Anthropic session already thinks at high"
        );
        assert!(
            nudge(&always_on, DifficultyLevel::High, Some(ThinkingEffort::Off)).is_none(),
            "always-on adaptive models ignore Off and still think at high"
        );

        let budget_pinned = ModelConfig::new("claude-sonnet-4-5-20250929")
            .with_merged_request_params(std::collections::HashMap::from([(
                "budget_tokens".to_string(),
                serde_json::json!(32000),
            )]));
        assert!(
            nudge(&budget_pinned, DifficultyLevel::High, None).is_none(),
            "an explicit thinking budget overrides effort, so a nudge would be a no-op"
        );

        let reasoning_pinned = ModelConfig::new("gpt-5").with_merged_request_params(
            std::collections::HashMap::from([(
                "reasoning".to_string(),
                serde_json::json!({"max_tokens": 2000}),
            )]),
        );
        assert!(
            nudge(&reasoning_pinned, DifficultyLevel::High, None).is_none(),
            "an explicit reasoning request param overrides effort, so a nudge would be a no-op"
        );

        let gemini = ModelConfig::new("gemini-3-flash");
        let recommendation = nudge(&gemini, DifficultyLevel::Medium, None)
            .expect("default Gemini 3 runs with thinking off, so medium is a raise");
        assert_eq!(recommendation.recommended_effort, ThinkingEffort::Medium);
        assert_eq!(recommendation.current_effort, ThinkingEffort::Off);
        assert!(
            nudge(&gemini, DifficultyLevel::Low, None).is_none(),
            "enabling thinking for easy remaining work is noise"
        );
        assert!(
            nudge(&gemini, DifficultyLevel::Medium, Some(ThinkingEffort::Low)).is_none(),
            "Gemini 3 maps low and medium to the same thinking level, so this changes nothing"
        );

        let pinned_high = ModelConfig::new("gpt-5-pro");
        assert!(
            nudge(
                &pinned_high,
                DifficultyLevel::High,
                Some(ThinkingEffort::Off)
            )
            .is_none(),
            "gpt-5-pro only supports high; Off can't be expressed, so it already runs at high"
        );
    }

    #[test]
    fn effort_recommendation_only_raises_effort() {
        let model = ModelConfig::new("gpt-5");
        assert!(model.is_reasoning_model());
        let build = |level, current| nudge(&model, level, current);

        // Unknown current effort resolves to the OpenAI API default, medium.
        let nudge = build(DifficultyLevel::High, None).expect("high beats the medium default");
        assert_eq!(nudge.recommended_effort, ThinkingEffort::High);
        assert_eq!(nudge.current_effort, ThinkingEffort::Medium);
        assert_eq!(nudge.difficulty, DifficultyLevel::High);
        assert_eq!(nudge.reason, "because");
        assert!(build(DifficultyLevel::Medium, None).is_none());
        assert!(build(DifficultyLevel::Low, None).is_none());

        let nudge = build(DifficultyLevel::Medium, Some(ThinkingEffort::Low))
            .expect("medium beats an explicit low");
        assert_eq!(nudge.recommended_effort, ThinkingEffort::Medium);
        assert_eq!(nudge.current_effort, ThinkingEffort::Low);

        // Matching or higher current effort: nothing to raise, and without
        // thinking telemetry there is never a downgrade either.
        assert!(build(DifficultyLevel::High, Some(ThinkingEffort::High)).is_none());
        assert!(build(DifficultyLevel::High, Some(ThinkingEffort::Max)).is_none());
        assert!(build(DifficultyLevel::Low, Some(ThinkingEffort::Medium)).is_none());

        // An explicit opt-out of thinking is second-guessed only by `high`.
        assert!(build(DifficultyLevel::Low, Some(ThinkingEffort::Off)).is_none());
        assert!(build(DifficultyLevel::Medium, Some(ThinkingEffort::Off)).is_none());
        let nudge = build(DifficultyLevel::High, Some(ThinkingEffort::Off))
            .expect("high overrides an explicit off");
        assert_eq!(nudge.recommended_effort, ThinkingEffort::High);
    }

    #[test]
    fn effort_recommendation_lowers_one_step_with_low_estimate_and_idle_telemetry() {
        let model = ModelConfig::new("gpt-5");
        let idle = idle_reasoning();

        let nudge = nudge_observing(
            &model,
            DifficultyLevel::Low,
            Some(ThinkingEffort::High),
            Some(&idle),
        )
        .expect("low estimate plus idle telemetry lowers a global-default high");
        assert_eq!(nudge.recommended_effort, ThinkingEffort::Medium);
        assert_eq!(nudge.current_effort, ThinkingEffort::High);

        let nudge = nudge_observing(
            &model,
            DifficultyLevel::Low,
            Some(ThinkingEffort::Max),
            Some(&idle),
        )
        .expect("a downgrade steps down a single level from the provider-effective value");
        // gpt-5 has no max tier, so the provider-effective effort is high and
        // the single step lands on medium.
        assert_eq!(nudge.current_effort, ThinkingEffort::High);
        assert_eq!(nudge.recommended_effort, ThinkingEffort::Medium);

        let nudge = nudge_observing(&model, DifficultyLevel::Low, None, Some(&idle))
            .expect("the OpenAI default medium can step down to low");
        assert_eq!(nudge.recommended_effort, ThinkingEffort::Low);
        assert_eq!(nudge.current_effort, ThinkingEffort::Medium);
    }

    #[test]
    fn effort_recommendation_downgrade_gates() {
        let model = ModelConfig::new("gpt-5");
        let idle = idle_reasoning();
        let lower = |level, current, observed: Option<&ObservedThinking>| {
            nudge_observing(&model, level, current, observed)
        };

        assert!(
            lower(
                DifficultyLevel::Medium,
                Some(ThinkingEffort::High),
                Some(&idle)
            )
            .is_none(),
            "only a low estimate may lower; medium-vs-high stays silent"
        );
        assert!(
            lower(DifficultyLevel::Low, Some(ThinkingEffort::High), None).is_none(),
            "no telemetry, no downgrade"
        );
        assert!(
            lower(
                DifficultyLevel::Low,
                Some(ThinkingEffort::High),
                Some(&ObservedThinking {
                    sampled_turns: OBSERVED_THINKING_MIN_TURNS - 1,
                    ..idle_reasoning()
                }),
            )
            .is_none(),
            "too few sampled turns"
        );
        assert!(
            lower(
                DifficultyLevel::Low,
                Some(ThinkingEffort::High),
                Some(&ObservedThinking {
                    thinking_tokens: 600,
                    ..idle_reasoning()
                }),
            )
            .is_none(),
            "recent turns actually reasoned; the estimate alone is not trusted downward"
        );
        assert!(
            lower(DifficultyLevel::Low, Some(ThinkingEffort::Low), Some(&idle)).is_none(),
            "low has nowhere to go; off is never suggested"
        );
        assert!(
            lower(DifficultyLevel::Low, Some(ThinkingEffort::Off), Some(&idle)).is_none(),
            "an explicit off is never adjusted downward"
        );

        let gemini = ModelConfig::new("gemini-3-flash");
        assert!(
            nudge_observing(
                &gemini,
                DifficultyLevel::Low,
                Some(ThinkingEffort::Medium),
                Some(&idle),
            )
            .is_none(),
            "Gemini 3 coalesces low back up to medium, so the downgrade would save nothing"
        );
        let gemini_max = nudge_observing(
            &gemini,
            DifficultyLevel::Low,
            Some(ThinkingEffort::Max),
            Some(&idle),
        )
        .expect("max coalesces to high on Gemini 3; the real step down is medium");
        assert_eq!(gemini_max.recommended_effort, ThinkingEffort::Medium);
        assert_eq!(gemini_max.current_effort, ThinkingEffort::High);

        let session_high = model.clone().with_thinking_effort(ThinkingEffort::High);
        {
            let _env = env_lock::lock_env([("GOOSE_THINKING_EFFORT", Some("low"))]);
            assert!(
                nudge_observing(
                    &session_high,
                    DifficultyLevel::Low,
                    Some(ThinkingEffort::High),
                    Some(&idle),
                )
                .is_none(),
                "a session effort differing from the global default is a user pin; never second-guessed downward"
            );
        }
        {
            let _env = env_lock::lock_env([("GOOSE_THINKING_EFFORT", Some("high"))]);
            let recommendation = nudge_observing(
                &session_high,
                DifficultyLevel::Low,
                Some(ThinkingEffort::High),
                Some(&idle),
            )
            .expect("a session effort equal to the global default is inherited, not pinned");
            assert_eq!(recommendation.recommended_effort, ThinkingEffort::Medium);
        }
    }

    #[test]
    fn observed_thinking_samples_recent_assistant_usage() {
        use crate::conversation::message::MessageUsage;

        let with_usage = |thinking: Option<i32>, output: i32| {
            let mut message = Message::assistant().with_text("done");
            message.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(output),
                thinking_tokens: thinking,
                ..Default::default()
            }));
            message
        };

        let mut messages = vec![Message::user().with_text("go")];
        // An early turn that reasoned hard, outside the window once enough
        // newer turns exist.
        messages.push(with_usage(Some(5000), 6000));
        for _ in 0..OBSERVED_THINKING_WINDOW {
            messages.push(with_usage(Some(10), 500));
        }
        // Assistant message without usage metadata: skipped, not sampled.
        messages.push(Message::assistant().with_text("note"));
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(observed.sampled_turns, OBSERVED_THINKING_WINDOW);
        assert_eq!(
            observed.thinking_tokens,
            10 * OBSERVED_THINKING_WINDOW as i64
        );
        assert_eq!(
            observed.output_tokens,
            500 * OBSERVED_THINKING_WINDOW as i64
        );
        assert!(observed.shows_idle_reasoning());
    }

    #[test]
    fn observed_thinking_estimates_from_thinking_content_when_counts_are_missing() {
        use crate::conversation::message::MessageUsage;

        let mut with_content_thinking = Message::assistant()
            .with_thinking("x".repeat(4000), "sig")
            .with_text("done");
        with_content_thinking.metadata.usage = Some(Box::new(MessageUsage {
            output_tokens: Some(500),
            thinking_tokens: None,
            ..Default::default()
        }));

        let mut reported = Message::assistant().with_text("ok");
        reported.metadata.usage = Some(Box::new(MessageUsage {
            output_tokens: Some(100),
            thinking_tokens: Some(0),
            ..Default::default()
        }));

        let conversation = Conversation::new_unvalidated(vec![
            Message::user().with_text("go"),
            reported,
            with_content_thinking,
        ]);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(
            observed.sampled_turns, 1,
            "an estimated turn adds thinking but is not idle evidence"
        );
        assert_eq!(observed.thinking_tokens, 1000);
        assert!(!observed.shows_idle_reasoning());
    }

    #[test]
    fn observed_thinking_counts_cancelled_turns_without_usage_as_blocking_evidence() {
        use crate::conversation::message::MessageUsage;

        let mut messages = vec![Message::user().with_text("go")];
        for _ in 0..OBSERVED_THINKING_MIN_TURNS {
            let mut idle = Message::assistant().with_text("done");
            idle.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(500),
                thinking_tokens: Some(0),
                ..Default::default()
            }));
            messages.push(idle);
        }
        messages.push(Message::assistant().with_thinking("x".repeat(4000), "sig"));
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(observed.sampled_turns, OBSERVED_THINKING_MIN_TURNS);
        assert!(
            !observed.shows_idle_reasoning(),
            "a reasoning-heavy turn cancelled before its usage trailer must still block"
        );
    }

    #[test]
    fn observed_thinking_never_reads_summarized_thinking_as_idle() {
        use crate::conversation::message::MessageUsage;

        // Bedrock reports no thinking breakdown; Claude surfaces a short
        // summary while output_tokens charges the full hidden reasoning.
        let mut messages = vec![Message::user().with_text("go")];
        for _ in 0..OBSERVED_THINKING_WINDOW {
            let mut summarized = Message::assistant()
                .with_thinking("x".repeat(1200), "sig")
                .with_text("done");
            summarized.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(12_500),
                thinking_tokens: None,
                ..Default::default()
            }));
            messages.push(summarized);
        }
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(observed.sampled_turns, 0);
        assert!(
            !observed.shows_idle_reasoning(),
            "a tiny summary over a large output must not pass the idle gate"
        );
    }

    #[test]
    fn observed_thinking_without_any_reported_counts_is_not_evidence() {
        use crate::conversation::message::MessageUsage;

        let mut messages = vec![Message::user().with_text("go")];
        for _ in 0..4 {
            let mut message = Message::assistant().with_text("done");
            message.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(500),
                thinking_tokens: None,
                ..Default::default()
            }));
            messages.push(message);
        }
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(observed.sampled_turns, 0);
        assert!(!observed.shows_idle_reasoning());
    }

    #[test]
    fn observed_thinking_counts_thinking_only_turns_without_an_output_count() {
        use crate::conversation::message::MessageUsage;

        // Gemini omits candidatesTokenCount on thinking-only turns.
        let mut thinking_only = Message::assistant().with_text("...");
        thinking_only.metadata.usage = Some(Box::new(MessageUsage {
            output_tokens: None,
            thinking_tokens: Some(5000),
            ..Default::default()
        }));

        let mut messages = vec![Message::user().with_text("go")];
        for _ in 0..OBSERVED_THINKING_MIN_TURNS {
            let mut idle = Message::assistant().with_text("done");
            idle.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(500),
                thinking_tokens: Some(0),
                ..Default::default()
            }));
            messages.push(idle);
        }
        messages.push(thinking_only);
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(observed.sampled_turns, OBSERVED_THINKING_MIN_TURNS + 1);
        assert_eq!(observed.thinking_tokens, 5000);
        assert!(
            !observed.shows_idle_reasoning(),
            "a heavy thinking-only turn must block a downgrade even without an output count"
        );
    }

    #[test]
    fn observed_thinking_estimates_redacted_thinking_from_the_blob_length() {
        use crate::conversation::message::MessageUsage;

        let mut messages = vec![Message::user().with_text("go")];
        for _ in 0..OBSERVED_THINKING_MIN_TURNS {
            let mut redacted = Message::assistant()
                .with_redacted_thinking("x".repeat(4000))
                .with_text("done");
            redacted.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(10_000),
                thinking_tokens: None,
                ..Default::default()
            }));
            messages.push(redacted);
        }
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(
            observed.thinking_tokens,
            1000 * OBSERVED_THINKING_MIN_TURNS as i64
        );
        assert!(
            !observed.shows_idle_reasoning(),
            "hidden reasoning of unknowable size must not read as idle"
        );
    }

    #[test]
    fn observed_thinking_ignores_stale_telemetry_from_before_a_provider_switch() {
        use crate::conversation::message::MessageUsage;

        let with_usage = |thinking: Option<i32>| {
            let mut message = Message::assistant().with_text("done");
            message.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(500),
                thinking_tokens: thinking,
                ..Default::default()
            }));
            message
        };

        // Idle turns reported by the previous provider, then a window's worth
        // of turns from a provider that strips the breakdown.
        let mut messages = vec![Message::user().with_text("go")];
        for _ in 0..OBSERVED_THINKING_MIN_TURNS {
            messages.push(with_usage(Some(0)));
        }
        for _ in 0..OBSERVED_THINKING_WINDOW {
            messages.push(with_usage(None));
        }
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "test-provider", "test-model");
        assert_eq!(
            observed.sampled_turns, 0,
            "evidence-free turns consume the window; stale reported turns behind them are never reached"
        );
        assert!(!observed.shows_idle_reasoning());
    }

    #[test]
    fn observed_thinking_stops_at_turns_attributed_to_a_different_model() {
        use crate::conversation::message::{InferenceMetadata, MessageUsage};

        let mut messages = vec![Message::user().with_text("go")];
        for _ in 0..OBSERVED_THINKING_MIN_TURNS {
            let mut idle =
                Message::assistant()
                    .with_text("done")
                    .with_inference(InferenceMetadata {
                        provider: "openai".to_string(),
                        requested_model: "gpt-5".to_string(),
                        resolved_model: None,
                    });
            idle.metadata.usage = Some(Box::new(MessageUsage {
                output_tokens: Some(500),
                thinking_tokens: Some(0),
                ..Default::default()
            }));
            messages.push(idle);
        }
        let conversation = Conversation::new_unvalidated(messages);

        let observed = observed_thinking(&conversation, "anthropic", "claude-sonnet-5");
        assert_eq!(
            observed.sampled_turns, 0,
            "another model's idle turns must not authorize downgrading this one"
        );

        let same_model = observed_thinking(&conversation, "openai", "gpt-5");
        assert_eq!(same_model.sampled_turns, OBSERVED_THINKING_MIN_TURNS);
        assert!(same_model.shows_idle_reasoning());
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
