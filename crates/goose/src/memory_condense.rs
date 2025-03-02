use crate::agents::Capabilities;
use crate::compress::Compressor;
use crate::message::Message;
use crate::token_counter::TokenCounter;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

pub struct MemoryCondense;

#[async_trait]
impl Compressor for MemoryCondense {
    async fn compress(
        &self,
        capabilities: &Capabilities,
        token_counter: &TokenCounter,
        messages: &mut Vec<Message>,
        token_counts: &mut Vec<usize>,
        context_limit: usize,
    ) -> Result<(), anyhow::Error> {
        self.condense(
            capabilities,
            token_counter,
            messages,
            token_counts,
            context_limit,
        )
        .await
    }
}

impl MemoryCondense {
    const SYSTEM_PROMPT: &str = "You are good at summarizing.";
    fn create_summarize_request(&self, messages: &[Message]) -> Vec<Message> {
        vec![
            Message::user().with_text(format!("Please use a few concise sentences to summarize this chat, while keeping the important information.\n\n```\n{:?}```", messages)),
        ]
    }
    async fn single_request(
        &self,
        capabilities: &Capabilities,
        messages: &[Message],
    ) -> Result<Message, anyhow::Error> {
        Ok(capabilities
            .provider()
            .complete(Self::SYSTEM_PROMPT, messages, &[])
            .await?
            .0)
    }
    async fn condense(
        &self,
        capabilities: &Capabilities,
        token_counter: &TokenCounter,
        messages: &mut Vec<Message>,
        token_counts: &mut Vec<usize>,
        context_limit: usize,
    ) -> Result<(), anyhow::Error> {
        let system_prompt_tokens = token_counter.count_tokens(Self::SYSTEM_PROMPT);

        // Since the process will run multiple times, we should avoid expensive operations like random access.
        let mut message_stack = messages.iter().cloned().rev().collect::<Vec<_>>();
        let mut count_stack = token_counts.iter().copied().rev().collect::<Vec<_>>();

        let mut total_tokens = count_stack.iter().sum::<usize>();

        let mut diff = 1;
        while total_tokens > context_limit && diff > 0 {
            let mut batch = Vec::new();
            let mut current_tokens = 0;
            while total_tokens > current_tokens + context_limit
                && current_tokens + system_prompt_tokens <= context_limit
            {
                batch.push(message_stack.pop().unwrap());
                current_tokens += count_stack.pop().unwrap();
            }
            if !batch.is_empty() && current_tokens + system_prompt_tokens <= context_limit {
                batch.push(message_stack.pop().unwrap());
                current_tokens += count_stack.pop().unwrap();
            }

            diff = -(current_tokens as isize);
            let request = self.create_summarize_request(&batch);
            let response_text = self
                .single_request(capabilities, &request)
                .await?
                .as_concat_text();

            // Ensure the conversation starts with a User message
            let curr_messages = vec![
                // shoule be in reversed order
                Message::assistant().with_text(&response_text),
                Message::user().with_text("Hello! How are we progressing?"),
            ];
            let curr_tokens = token_counter.count_chat_tokens("", &curr_messages, &[]);
            diff += curr_tokens as isize;
            count_stack.push(curr_tokens);
            message_stack.extend(curr_messages);
            total_tokens = total_tokens.checked_add_signed(diff).unwrap();
        }

        if total_tokens <= context_limit {
            *messages = message_stack.into_iter().rev().collect();
            *token_counts = count_stack.into_iter().rev().collect();
            Ok(())
        } else {
            Err(anyhow!("Cannot compress the messages anymore"))
        }
    }
}
