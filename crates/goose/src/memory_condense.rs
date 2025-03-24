use crate::agents::Capabilities;
use crate::message::Message;
use crate::token_counter::TokenCounter;
use anyhow::{anyhow, Result};
use tracing::debug;

const SYSTEM_PROMPT: &str = "You are good at summarizing.";

fn create_summarize_request(messages: &[Message]) -> Vec<Message> {
    vec![
        Message::user().with_text(format!("Please use a few concise sentences to summarize this chat, while keeping the important information.\n\n```\n{:?}```", messages)),
    ]
}
async fn single_request(
    capabilities: &Capabilities,
    messages: &[Message],
) -> Result<Message, anyhow::Error> {
    Ok(capabilities
        .provider()
        .complete(SYSTEM_PROMPT, messages, &[])
        .await?
        .0)
}
async fn memory_condense(
    capabilities: &Capabilities,
    token_counter: &TokenCounter,
    messages: &mut Vec<Message>,
    token_counts: &mut Vec<usize>,
    context_limit: usize,
) -> Result<(), anyhow::Error> {
    let system_prompt_tokens = token_counter.count_tokens(SYSTEM_PROMPT);

    // Since the process will run multiple times, we should avoid expensive operations like random access.
    let mut message_stack = messages.iter().cloned().rev().collect::<Vec<_>>();
    let mut count_stack = token_counts.iter().copied().rev().collect::<Vec<_>>();

    // Tracks the number of remaining tokens in the stack
    let mut total_tokens = count_stack.iter().sum::<usize>();

    // Tracks the change of total_tokens in the previous loop.
    // If diff <= 0, then the model cannot summarize any further. We set it to 1 before the process
    // to ensure that the process starts.
    let mut diff = 1;

    while total_tokens > context_limit && diff > 0 {
        let mut batch = Vec::new();
        let mut current_tokens = 0;

        // Extracts the beginning messages (which appears in the front of the message stack) to
        // summarize.
        while total_tokens > current_tokens + context_limit
            && current_tokens + system_prompt_tokens <= context_limit
        {
            batch.push(message_stack.pop().unwrap());
            current_tokens += count_stack.pop().unwrap();
        }

        // It could happen that the extracted messages are always the previous summary when the
        // context limit is very small. We should force it to consume more messages.
        if !batch.is_empty()
            && !message_stack.is_empty()
            && current_tokens + system_prompt_tokens <= context_limit
        {
            batch.push(message_stack.pop().unwrap());
            current_tokens += count_stack.pop().unwrap();
        }

        diff = -(current_tokens as isize);
        let request = create_summarize_request(&batch);
        let response_text = single_request(capabilities, &request)
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

        // Update the counter
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

pub async fn condense_messages(
    capabilities: &Capabilities,
    token_counter: &TokenCounter,
    messages: &mut Vec<Message>,
    token_counts: &mut Vec<usize>,
    context_limit: usize,
) -> Result<(), anyhow::Error> {
    let total_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens before memory condensation: {}", total_tokens);

    // The compressor should determine whether we need to compress the messages or not. This
    // function just checks if the limit is satisfied.
    memory_condense(
        capabilities,
        token_counter,
        messages,
        token_counts,
        context_limit,
    )
    .await?;

    let total_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens after memory condensation: {}", total_tokens);

    // Compressor should handle this case.
    assert!(total_tokens <= context_limit, "Illegal compression result from the compressor: the number of tokens is greater than the limit.");

    debug!(
        "Memory condensation complete. Total tokens: {}",
        total_tokens
    );
    Ok(())
}
