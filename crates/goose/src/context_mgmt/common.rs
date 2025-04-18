use std::sync::Arc;

use mcp_core::Tool;

use crate::{message::Message, providers::base::Provider, token_counter::TokenCounter};

const ESTIMATE_FACTOR: f32 = 0.7;
const SYSTEM_PROMPT_TOKEN_OVERHEAD: usize = 3_000;
const TOOLS_TOKEN_OVERHEAD: usize = 5_000;

pub fn estimate_target_context_limit(provider: Arc<dyn Provider>) -> usize {
    let model_context_limit = provider.get_model_config().context_limit();

    // Our conservative estimate of the **target** context limit
    // Our token count is an estimate since model providers often don't provide the tokenizer (eg. Claude)
    let target_limit = (model_context_limit as f32 * ESTIMATE_FACTOR) as usize;

    // subtract out overhead for system prompt and tools
    target_limit - (SYSTEM_PROMPT_TOKEN_OVERHEAD + TOOLS_TOKEN_OVERHEAD)
}

pub fn get_messages_token_counts(token_counter: &TokenCounter, messages: &[Message]) -> Vec<usize> {
    // Calculate current token count of each message, use count_chat_tokens to ensure we
    // capture the full content of the message, include ToolRequests and ToolResponses
    messages
        .iter()
        .map(|msg| token_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]))
        .collect()
}

// These are not being used now but could be useful in the future

#[allow(dead_code)]
pub struct ChatTokenCounts {
    pub system: usize,
    pub tools: usize,
    pub messages: Vec<usize>,
}

#[allow(dead_code)]
pub fn get_token_counts(
    token_counter: &TokenCounter,
    messages: &mut [Message],
    system_prompt: &str,
    tools: &mut Vec<Tool>,
) -> ChatTokenCounts {
    // Take into account the system prompt (includes goosehints), and our tools input
    let system_prompt_token_count = token_counter.count_tokens(system_prompt);
    let tools_token_count = token_counter.count_tokens_for_tools(tools.as_slice());
    let messages_token_count = get_messages_token_counts(token_counter, messages);

    ChatTokenCounts {
        system: system_prompt_token_count,
        tools: tools_token_count,
        messages: messages_token_count,
    }
}
