---
title: Context Revision and Token Management
sidebar_position: 4
---

# Context Revision and Token Management

Goose implements sophisticated context revision mechanisms to manage token usage and maintain conversation coherence. This document explains the token management system, truncation strategies, and summarization approaches used in Goose.

## Token Management System

The token management system is responsible for:

1. Tracking token usage for each message
2. Estimating token counts for different content types
3. Enforcing context limits based on model configuration

### Token Counting

Goose uses a `TokenCounter` to estimate token counts for different content types:

```rust
pub struct TokenCounter {
    tokenizer: Option<Tokenizer>,
}

impl TokenCounter {
    pub fn count_tokens(&self, text: &str) -> usize;
    pub fn count_chat_tokens(&self, system: &str, messages: &[Message], tools: &[Tool]) -> usize;
    pub fn count_tokens_for_tools(&self, tools: &[Tool]) -> usize;
}
```

The token counter can use different tokenizers based on the model configuration, or fall back to a simple heuristic when a tokenizer is not available.

## Context Revision Strategies

Goose implements two main strategies for context revision:

1. **Truncation**: Removing messages when context limits are reached
2. **Summarization**: Using the LLM to summarize conversation history

### Truncation Strategy

The truncation system:

1. Calculates token counts for all messages
2. Identifies messages to remove based on the selected strategy
3. Ensures tool request-response pairs are kept together
4. Maintains conversation coherence by preserving critical messages

The primary truncation strategy is `OldestFirstTruncation`, which removes the oldest messages first:

```rust
pub struct OldestFirstTruncation;

impl TruncationStrategy for OldestFirstTruncation {
    fn determine_indices_to_remove(
        &self,
        messages: &[Message],
        token_counts: &[usize],
        context_limit: usize,
    ) -> Result<HashSet<usize>>;
}
```

The truncation process ensures that:

- Tool request-response pairs are kept together
- The last message is always a user message with text content
- The conversation remains coherent

### Summarization Strategy

The `SummarizeAgent` uses the LLM to condense conversation history when context limits are reached:

```rust
pub struct SummarizeAgent {
    capabilities: Mutex<Capabilities>,
    token_counter: TokenCounter,
    confirmation_tx: mpsc::Sender<(String, bool)>,
    confirmation_rx: Mutex<mpsc::Receiver<(String, bool)>>,
}
```

The summarization process:

1. Identifies when context limits are approaching
2. Asks the LLM to summarize parts of the conversation
3. Replaces the original messages with the summary
4. Falls back to truncation if summarization fails

## Context Window Management

The context window is managed dynamically based on:

1. The model's context limit
2. The current token usage
3. The selected context revision strategy

When context limits are approached, Goose:

1. Calculates the current token usage
2. Applies the appropriate revision strategy
3. Verifies that the revised context fits within limits
4. Retries with more aggressive revision if needed

## Token Usage Tracking

Goose tracks token usage for each provider interaction:

```rust
pub struct Usage {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

pub struct ProviderUsage {
    pub model: String,
    pub usage: Usage,
}
```

This information is used for:

1. Monitoring resource usage
2. Optimizing context management
3. Reporting to users

## Adaptive Context Management

Goose implements adaptive context management through:

1. Estimating token usage with a conservative factor
2. Decaying the estimate factor on repeated truncation attempts
3. Balancing context preservation with token limits

```rust
const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;
```

## Best Practices

1. **Prioritize Recent Messages**: Recent messages are generally more relevant
2. **Preserve Tool Interactions**: Keep tool request-response pairs together
3. **Maintain Conversation Coherence**: Ensure the conversation remains understandable
4. **Balance Token Usage**: Optimize token usage across system prompt, messages, and tools
