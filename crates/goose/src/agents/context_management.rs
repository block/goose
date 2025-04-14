use std::sync::Arc;

use anyhow::Result;
use tracing::{error, warn};

use crate::agents::agent::Agent;
use crate::message::Message;
use crate::providers::base::Provider;
use crate::token_counter::TokenCounter;
use crate::truncate;
use mcp_core::tool::Tool;

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

/// Truncates the messages to fit within the model's context window
/// Ensures the last message is a user message and removes tool call-response pairs
pub async fn truncate_messages(
    provider: &Arc<dyn Provider>,
    token_counter: &TokenCounter,
    messages: &mut Vec<Message>,
    estimate_factor: f32,
    system_prompt: &str,
    tools: &mut Vec<Tool>,
) -> anyhow::Result<()> {
    // Model's actual context limit
    let context_limit = provider.get_model_config().context_limit();

    // Our conservative estimate of the **target** context limit
    // Our token count is an estimate since model providers often don't provide the tokenizer (eg. Claude)
    let context_limit = (context_limit as f32 * estimate_factor) as usize;

    // Take into account the system prompt, and our tools input and subtract that from the
    // remaining context limit
    let system_prompt_token_count = token_counter.count_tokens(system_prompt);
    let tools_token_count = token_counter.count_tokens_for_tools(tools.as_slice());

    // Check if system prompt + tools exceed our context limit
    let remaining_tokens = context_limit
        .checked_sub(system_prompt_token_count)
        .and_then(|remaining| remaining.checked_sub(tools_token_count))
        .ok_or_else(|| anyhow::anyhow!("System prompt and tools exceed estimated context limit"))?;

    let context_limit = remaining_tokens;

    // Calculate current token count of each message, use count_chat_tokens to ensure we
    // capture the full content of the message, include ToolRequests and ToolResponses
    let mut token_counts: Vec<usize> = messages
        .iter()
        .map(|msg| token_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]))
        .collect();

    crate::truncate::truncate_messages(
        messages,
        &mut token_counts,
        context_limit,
        &truncate::OldestFirstTruncation,
    )
}

/// Handle truncation when context length is exceeded
pub async fn handle_truncation_error(
    provider: &Arc<dyn Provider>,
    token_counter: &TokenCounter,
    messages: &mut Vec<Message>,
    truncation_attempt: &mut usize,
    system_prompt: &str,
    tools: &mut Vec<Tool>,
) -> Result<bool> {
    if *truncation_attempt >= MAX_TRUNCATION_ATTEMPTS {
        return Ok(false);
    }

    *truncation_attempt += 1;
    warn!(
        "Context length exceeded. Truncation Attempt: {}/{}.",
        truncation_attempt, MAX_TRUNCATION_ATTEMPTS
    );

    // Decay the estimate factor as we make more truncation attempts
    let estimate_factor: f32 = ESTIMATE_FACTOR_DECAY.powi(*truncation_attempt as i32);

    // Call our standalone truncate_messages function
    if let Err(err) = truncate_messages(
        provider,
        token_counter,
        messages,
        estimate_factor,
        system_prompt,
        tools,
    )
    .await
    {
        error!("Failed to truncate messages: {}", err);
        return Ok(false);
    }

    Ok(true)
}