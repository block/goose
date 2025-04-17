use crate::message::Message;
use crate::token_counter::TokenCounter;

use crate::context_mgmt::summarize::summarize_messages;
use crate::context_mgmt::truncate::{truncate_messages, OldestFirstTruncation};
use crate::context_mgmt::{estimate_target_context_limit, get_messages_token_counts};

use super::super::agents::Agent;

impl Agent {
    /// Public API to truncate oldest messages so that the conversation's token count is within the allowed context limit.
    pub fn truncate_context(
        &self,
        mut messages: Vec<Message>,
    ) -> Result<Vec<Message>, anyhow::Error> {
        let provider = self.provider.clone();
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        let target_context_limit = estimate_target_context_limit(provider);
        let mut token_counts = get_messages_token_counts(&token_counter, &messages);

        truncate_messages(
            &mut messages,
            &mut token_counts,
            target_context_limit,
            &OldestFirstTruncation,
        )?;

        Ok(messages)
    }

    /// Public API to summarize the conversation so that its token count is within the allowed context limit.
    pub async fn summarize_context(
        &self,
        mut messages: Vec<Message>,
    ) -> Result<Vec<Message>, anyhow::Error> {
        let provider = self.provider.clone();
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        let target_context_limit = estimate_target_context_limit(provider.clone());

        summarize_messages(
            provider,
            &mut messages,
            &token_counter,
            target_context_limit,
        )
        .await?;

        Ok(messages)
    }
}
