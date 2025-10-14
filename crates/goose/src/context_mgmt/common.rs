use crate::conversation::message::Message;
use crate::token_counter::AsyncTokenCounter;

pub const SYSTEM_PROMPT_TOKEN_OVERHEAD: usize = 3_000;
pub const TOOLS_TOKEN_OVERHEAD: usize = 5_000;

/// Async version of get_messages_token_counts for better performance
pub fn get_messages_token_counts_async(
    token_counter: &AsyncTokenCounter,
    messages: &[Message],
) -> Vec<usize> {
    messages
        .iter()
        .filter(|m| m.is_agent_visible())
        .map(|msg| token_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]))
        .collect()
}
