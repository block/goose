use std::sync::Arc;

use rmcp::model::Tool;

use crate::conversation::message::Message;
use crate::{
    providers::base::Provider,
    token_counter::{AsyncTokenCounter, TokenCounter},
};

const ESTIMATE_FACTOR: f32 = 0.7;
pub const SYSTEM_PROMPT_TOKEN_OVERHEAD: usize = 3_000;
pub const TOOLS_TOKEN_OVERHEAD: usize = 5_000;

pub fn estimate_target_context_limit(provider: Arc<dyn Provider>) -> usize {
    let model_context_limit = provider.get_model_config().context_limit();

    // Our conservative estimate of the **target** context limit
    // Our token count is an estimate since model providers often don't provide the tokenizer (eg. Claude)
    let target_limit = (model_context_limit as f32 * ESTIMATE_FACTOR) as usize;

    // subtract out overhead for system prompt and tools, but ensure we don't go negative
    let overhead = SYSTEM_PROMPT_TOKEN_OVERHEAD + TOOLS_TOKEN_OVERHEAD;
    if target_limit > overhead {
        target_limit - overhead
    } else {
        // If overhead is larger than target limit, return a minimal usable limit
        std::cmp::max(target_limit / 2, 1000)
    }
}

pub fn get_messages_token_counts(token_counter: &TokenCounter, messages: &[Message]) -> Vec<usize> {
    // Calculate current token count of each message, use count_chat_tokens to ensure we
    // capture the full content of the message, include ToolRequests and ToolResponses
    messages
        .iter()
        .map(|msg| token_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]))
        .collect()
}

/// Async version of get_messages_token_counts for better performance
pub fn get_messages_token_counts_async(
    token_counter: &AsyncTokenCounter,
    messages: &[Message],
) -> Vec<usize> {
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

/// Async version of get_token_counts for better performance
#[allow(dead_code)]
pub fn get_token_counts_async(
    token_counter: &AsyncTokenCounter,
    messages: &mut [Message],
    system_prompt: &str,
    tools: &mut Vec<Tool>,
) -> ChatTokenCounts {
    // Take into account the system prompt (includes goosehints), and our tools input
    let system_prompt_token_count = token_counter.count_tokens(system_prompt);
    let tools_token_count = token_counter.count_tokens_for_tools(tools.as_slice());
    let messages_token_count = get_messages_token_counts_async(token_counter, messages);

    ChatTokenCounts {
        system: system_prompt_token_count,
        tools: tools_token_count,
        messages: messages_token_count,
    }
}
