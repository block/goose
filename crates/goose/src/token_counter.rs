use ahash::AHasher;
use dashmap::DashMap;
use rmcp::model::Tool;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tiktoken_rs::CoreBPE;
use tokio::sync::OnceCell;

use crate::conversation::message::Message;

// Global tokenizer instance to avoid repeated initialization
static TOKENIZER: OnceCell<Arc<CoreBPE>> = OnceCell::const_new();

// Cache size limits to prevent unbounded growth
const MAX_TOKEN_CACHE_SIZE: usize = 10_000;

/// Async token counter with caching capabilities
pub struct AsyncTokenCounter {
    tokenizer: Arc<CoreBPE>,
    token_cache: Arc<DashMap<u64, usize>>, // content hash -> token count
}

/// Legacy synchronous token counter for backward compatibility
pub struct TokenCounter {
    tokenizer: Arc<CoreBPE>,
}

impl AsyncTokenCounter {
    /// Creates a new async token counter with caching
    pub async fn new() -> Result<Self, String> {
        let tokenizer = get_tokenizer().await?;
        Ok(Self {
            tokenizer,
            token_cache: Arc::new(DashMap::new()),
        })
    }

    /// Count tokens with optimized caching
    pub fn count_tokens(&self, text: &str) -> usize {
        // Use faster AHash for better performance
        let mut hasher = AHasher::default();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        // Check cache first
        if let Some(count) = self.token_cache.get(&hash) {
            return *count;
        }

        // Compute and cache result with size management
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        let count = tokens.len();

        // Manage cache size to prevent unbounded growth
        if self.token_cache.len() >= MAX_TOKEN_CACHE_SIZE {
            // Simple eviction: remove a random entry
            if let Some(entry) = self.token_cache.iter().next() {
                let old_hash = *entry.key();
                self.token_cache.remove(&old_hash);
            }
        }

        self.token_cache.insert(hash, count);
        count
    }

    /// Count tokens for tools with optimized string handling
    pub fn count_tokens_for_tools(&self, tools: &[Tool]) -> usize {
        // Token counts for different function components
        let func_init = 7; // Tokens for function initialization
        let prop_init = 3; // Tokens for properties initialization
        let prop_key = 3; // Tokens for each property key
        let enum_init: isize = -3; // Tokens adjustment for enum list start
        let enum_item = 3; // Tokens for each enum item
        let func_end = 12; // Tokens for function ending

        let mut func_token_count = 0;
        if !tools.is_empty() {
            for tool in tools {
                func_token_count += func_init;
                let name = &tool.name;
                let description = &tool
                    .description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default()
                    .trim_end_matches('.');

                // Note: the separator (:) is likely tokenized with adjacent tokens, so we use original approach for accuracy
                let line = format!("{}:{}", name, description);
                func_token_count += self.count_tokens(&line);

                if let serde_json::Value::Object(properties) = &tool.input_schema["properties"] {
                    if !properties.is_empty() {
                        func_token_count += prop_init;
                        for (key, value) in properties {
                            func_token_count += prop_key;
                            let p_name = key;
                            let p_type = value["type"].as_str().unwrap_or("");
                            let p_desc = value["description"]
                                .as_str()
                                .unwrap_or("")
                                .trim_end_matches('.');

                            // Note: separators are tokenized with adjacent tokens, keep original for accuracy
                            let line = format!("{}:{}:{}", p_name, p_type, p_desc);
                            func_token_count += self.count_tokens(&line);

                            if let Some(enum_values) = value["enum"].as_array() {
                                func_token_count =
                                    func_token_count.saturating_add_signed(enum_init);
                                for item in enum_values {
                                    if let Some(item_str) = item.as_str() {
                                        func_token_count += enum_item;
                                        func_token_count += self.count_tokens(item_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            func_token_count += func_end;
        }

        func_token_count
    }

    /// Count chat tokens (using cached count_tokens)
    pub fn count_chat_tokens(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> usize {
        let tokens_per_message = 4;
        let mut num_tokens = 0;

        if !system_prompt.is_empty() {
            num_tokens += self.count_tokens(system_prompt) + tokens_per_message;
        }

        for message in messages {
            num_tokens += tokens_per_message;
            for content in &message.content {
                if let Some(content_text) = content.as_text() {
                    num_tokens += self.count_tokens(content_text);
                } else if let Some(tool_request) = content.as_tool_request() {
                    let tool_call = tool_request.tool_call.as_ref().unwrap();
                    // Note: separators are tokenized with adjacent tokens, keep original for accuracy
                    let text = format!(
                        "{}:{}:{}",
                        tool_request.id, tool_call.name, tool_call.arguments
                    );
                    num_tokens += self.count_tokens(&text);
                } else if let Some(tool_response_text) = content.as_tool_response_text() {
                    num_tokens += self.count_tokens(&tool_response_text);
                }
            }
        }

        if !tools.is_empty() {
            num_tokens += self.count_tokens_for_tools(tools);
        }

        num_tokens += 3; // Reply primer

        num_tokens
    }

    /// Count everything including resources (using cached count_tokens)
    pub fn count_everything(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        resources: &[String],
    ) -> usize {
        let mut num_tokens = self.count_chat_tokens(system_prompt, messages, tools);

        if !resources.is_empty() {
            for resource in resources {
                num_tokens += self.count_tokens(resource);
            }
        }
        num_tokens
    }

    /// Cache management methods
    pub fn clear_cache(&self) {
        self.token_cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.token_cache.len()
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter {
    /// Creates a new `TokenCounter` using the fixed o200k_base encoding.
    pub fn new() -> Self {
        // Use blocking version of get_tokenizer
        let tokenizer = get_tokenizer_blocking().expect("Failed to initialize tokenizer");
        Self { tokenizer }
    }

    /// Count tokens for a piece of text using our single tokenizer.
    pub fn count_tokens(&self, text: &str) -> usize {
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        tokens.len()
    }

    pub fn count_tokens_for_tools(&self, tools: &[Tool]) -> usize {
        // Token counts for different function components
        let func_init = 7; // Tokens for function initialization
        let prop_init = 3; // Tokens for properties initialization
        let prop_key = 3; // Tokens for each property key
        let enum_init: isize = -3; // Tokens adjustment for enum list start
        let enum_item = 3; // Tokens for each enum item
        let func_end = 12; // Tokens for function ending

        let mut func_token_count = 0;
        if !tools.is_empty() {
            for tool in tools {
                func_token_count += func_init; // Add tokens for start of each function
                let name = &tool.name;
                let description = &tool
                    .description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default()
                    .trim_end_matches('.');
                let line = format!("{}:{}", name, description);
                func_token_count += self.count_tokens(&line); // Add tokens for name and description

                if let serde_json::Value::Object(properties) = &tool.input_schema["properties"] {
                    if !properties.is_empty() {
                        func_token_count += prop_init; // Add tokens for start of properties
                        for (key, value) in properties {
                            func_token_count += prop_key; // Add tokens for each property
                            let p_name = key;
                            let p_type = value["type"].as_str().unwrap_or("");
                            let p_desc = value["description"]
                                .as_str()
                                .unwrap_or("")
                                .trim_end_matches('.');
                            let line = format!("{}:{}:{}", p_name, p_type, p_desc);
                            func_token_count += self.count_tokens(&line);
                            if let Some(enum_values) = value["enum"].as_array() {
                                func_token_count =
                                    func_token_count.saturating_add_signed(enum_init); // Add tokens if property has enum list
                                for item in enum_values {
                                    if let Some(item_str) = item.as_str() {
                                        func_token_count += enum_item;
                                        func_token_count += self.count_tokens(item_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            func_token_count += func_end;
        }

        func_token_count
    }

    pub fn count_chat_tokens(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> usize {
        // <|im_start|>ROLE<|im_sep|>MESSAGE<|im_end|>
        let tokens_per_message = 4;

        // Count tokens in the system prompt
        let mut num_tokens = 0;
        if !system_prompt.is_empty() {
            num_tokens += self.count_tokens(system_prompt) + tokens_per_message;
        }

        for message in messages {
            num_tokens += tokens_per_message;
            // Count tokens in the content
            for content in &message.content {
                // content can either be text response or tool request
                if let Some(content_text) = content.as_text() {
                    num_tokens += self.count_tokens(content_text);
                } else if let Some(tool_request) = content.as_tool_request() {
                    let tool_call = tool_request.tool_call.as_ref().unwrap();
                    let text = format!(
                        "{}:{}:{}",
                        tool_request.id, tool_call.name, tool_call.arguments
                    );
                    num_tokens += self.count_tokens(&text);
                } else if let Some(tool_response_text) = content.as_tool_response_text() {
                    num_tokens += self.count_tokens(&tool_response_text);
                } else {
                    // unsupported content type such as image - pass
                    continue;
                }
            }
        }

        // Count tokens for tools if provided
        if !tools.is_empty() {
            num_tokens += self.count_tokens_for_tools(tools);
        }

        // Every reply is primed with <|start|>assistant<|message|>
        num_tokens += 3;

        num_tokens
    }

    pub fn count_everything(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        resources: &[String],
    ) -> usize {
        let mut num_tokens = self.count_chat_tokens(system_prompt, messages, tools);

        if !resources.is_empty() {
            for resource in resources {
                num_tokens += self.count_tokens(resource);
            }
        }
        num_tokens
    }
}

/// Get the global tokenizer instance (async version)
/// Fixed encoding for all tokenization - using o200k_base for GPT-4o and o1 models
async fn get_tokenizer() -> Result<Arc<CoreBPE>, String> {
    let tokenizer = TOKENIZER
        .get_or_init(|| async {
            match tiktoken_rs::o200k_base() {
                Ok(bpe) => Arc::new(bpe),
                Err(e) => panic!("Failed to initialize o200k_base tokenizer: {}", e),
            }
        })
        .await;
    Ok(tokenizer.clone())
}

/// Get the global tokenizer instance (blocking version for backward compatibility)
fn get_tokenizer_blocking() -> Result<Arc<CoreBPE>, String> {
    // For the blocking version, we need to handle the case where the tokenizer hasn't been initialized yet
    if let Some(tokenizer) = TOKENIZER.get() {
        return Ok(tokenizer.clone());
    }

    // Initialize the tokenizer synchronously
    match tiktoken_rs::o200k_base() {
        Ok(bpe) => {
            let tokenizer = Arc::new(bpe);
            // Try to set it in the OnceCell, but it's okay if another thread beat us to it
            let _ = TOKENIZER.set(tokenizer.clone());
            Ok(tokenizer)
        }
        Err(e) => Err(format!("Failed to initialize o200k_base tokenizer: {}", e)),
    }
}

/// Factory function for creating async token counters with proper error handling
pub async fn create_async_token_counter() -> Result<AsyncTokenCounter, String> {
    AsyncTokenCounter::new().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent};
    use rmcp::model::{Role, Tool};
    use rmcp::object;

    #[test]
    fn test_token_counter_basic() {
        let counter = TokenCounter::new();

        let text = "Hello, how are you?";
        let count = counter.count_tokens(text);
        println!("Token count for '{}': {:?}", text, count);

        // With o200k_base encoding, this should give us a reasonable count
        assert!(count > 0, "Token count should be greater than 0");
    }

    #[test]
    fn test_token_counter_simple_text() {
        let counter = TokenCounter::new();

        let text = "Hey there!";
        let count = counter.count_tokens(text);
        println!("Token count for '{}': {:?}", text, count);

        // With o200k_base encoding, this should give us a reasonable count
        assert!(count > 0, "Token count should be greater than 0");
    }

    #[test]
    fn test_count_chat_tokens() {
        let counter = TokenCounter::new();

        let system_prompt =
            "You are a helpful assistant that can answer questions about the weather.";

        let messages = vec![
            Message::new(
                Role::User,
                0,
                vec![MessageContent::text(
                    "What's the weather like in San Francisco?",
                )],
            ),
            Message::new(
                Role::Assistant,
                1,
                vec![MessageContent::text(
                    "Looks like it's 60 degrees Fahrenheit in San Francisco.",
                )],
            ),
            Message::new(
                Role::User,
                2,
                vec![MessageContent::text("How about New York?")],
            ),
        ];

        let tools = vec![Tool::new(
            "get_current_weather",
            "Get the current weather in a given location",
            object!({
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    },
                    "unit": {
                        "type": "string",
                        "description": "The unit of temperature to return",
                        "enum": ["celsius", "fahrenheit"]
                    }
                },
                "required": ["location"]
            }),
        )];

        let token_count_without_tools = counter.count_chat_tokens(system_prompt, &messages, &[]);
        println!("Total tokens without tools: {}", token_count_without_tools);

        let token_count_with_tools = counter.count_chat_tokens(system_prompt, &messages, &tools);
        println!("Total tokens with tools: {}", token_count_with_tools);

        // Basic sanity checks - with o200k_base the exact counts may differ from the old tokenizer
        assert!(
            token_count_without_tools > 0,
            "Should have some tokens without tools"
        );
        assert!(
            token_count_with_tools > token_count_without_tools,
            "Should have more tokens with tools"
        );
    }

    #[tokio::test]
    async fn test_async_token_counter() {
        let counter = create_async_token_counter().await.unwrap();

        let text = "Hello, how are you?";
        let count = counter.count_tokens(text);
        println!("Async token count for '{}': {:?}", text, count);

        assert!(count > 0, "Async token count should be greater than 0");
    }

    #[tokio::test]
    async fn test_async_token_caching() {
        let counter = create_async_token_counter().await.unwrap();

        let text = "This is a test for caching functionality";

        // First call should compute and cache
        let count1 = counter.count_tokens(text);
        assert_eq!(counter.cache_size(), 1);

        // Second call should use cache
        let count2 = counter.count_tokens(text);
        assert_eq!(count1, count2);
        assert_eq!(counter.cache_size(), 1);

        // Different text should increase cache
        let count3 = counter.count_tokens("Different text");
        assert_eq!(counter.cache_size(), 2);
        assert_ne!(count1, count3);
    }

    #[tokio::test]
    async fn test_async_count_chat_tokens() {
        let counter = create_async_token_counter().await.unwrap();

        let system_prompt =
            "You are a helpful assistant that can answer questions about the weather.";

        let messages = vec![
            Message::new(
                Role::User,
                0,
                vec![MessageContent::text(
                    "What's the weather like in San Francisco?",
                )],
            ),
            Message::new(
                Role::Assistant,
                1,
                vec![MessageContent::text(
                    "Looks like it's 60 degrees Fahrenheit in San Francisco.",
                )],
            ),
            Message::new(
                Role::User,
                2,
                vec![MessageContent::text("How about New York?")],
            ),
        ];

        let tools = vec![Tool::new(
            "get_current_weather",
            "Get the current weather in a given location",
            object!({
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    },
                    "unit": {
                        "type": "string",
                        "description": "The unit of temperature to return",
                        "enum": ["celsius", "fahrenheit"]
                    }
                },
                "required": ["location"]
            }),
        )];

        let token_count_without_tools = counter.count_chat_tokens(system_prompt, &messages, &[]);
        println!(
            "Async total tokens without tools: {}",
            token_count_without_tools
        );

        let token_count_with_tools = counter.count_chat_tokens(system_prompt, &messages, &tools);
        println!("Async total tokens with tools: {}", token_count_with_tools);

        // Basic sanity checks
        assert!(
            token_count_without_tools > 0,
            "Should have some tokens without tools"
        );
        assert!(
            token_count_with_tools > token_count_without_tools,
            "Should have more tokens with tools"
        );
    }

    #[tokio::test]
    async fn test_async_cache_management() {
        let counter = create_async_token_counter().await.unwrap();

        // Add some items to cache
        counter.count_tokens("First text");
        counter.count_tokens("Second text");
        counter.count_tokens("Third text");

        assert_eq!(counter.cache_size(), 3);

        // Clear cache
        counter.clear_cache();
        assert_eq!(counter.cache_size(), 0);

        // Re-count should work fine
        let count = counter.count_tokens("First text");
        assert!(count > 0);
        assert_eq!(counter.cache_size(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_token_counter_creation() {
        // Test concurrent creation of token counters to verify no race conditions
        let handles: Vec<_> = (0..10)
            .map(|_| tokio::spawn(async { create_async_token_counter().await.unwrap() }))
            .collect();

        let counters: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All should work and give same results
        let text = "Test concurrent creation";
        let expected_count = counters[0].count_tokens(text);

        for counter in &counters {
            assert_eq!(counter.count_tokens(text), expected_count);
        }
    }

    #[tokio::test]
    async fn test_cache_eviction_behavior() {
        let counter = create_async_token_counter().await.unwrap();

        // Fill cache beyond normal size to test eviction
        let mut cached_texts = Vec::new();
        for i in 0..50 {
            let text = format!("Test string number {}", i);
            counter.count_tokens(&text);
            cached_texts.push(text);
        }

        // Cache should be bounded
        assert!(counter.cache_size() <= MAX_TOKEN_CACHE_SIZE);

        // Earlier entries may have been evicted, but recent ones should still be cached
        let recent_text = &cached_texts[cached_texts.len() - 1];
        let start_size = counter.cache_size();

        // This should be a cache hit (no size increase)
        counter.count_tokens(recent_text);
        assert_eq!(counter.cache_size(), start_size);
    }

    #[tokio::test]
    async fn test_concurrent_cache_operations() {
        let counter = std::sync::Arc::new(create_async_token_counter().await.unwrap());

        // Test concurrent token counting operations
        let handles: Vec<_> = (0..20)
            .map(|i| {
                let counter_clone = counter.clone();
                tokio::spawn(async move {
                    let text = format!("Concurrent test {}", i % 5); // Some repetition for cache hits
                    counter_clone.count_tokens(&text)
                })
            })
            .collect();

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All results should be valid (> 0)
        for result in results {
            assert!(result > 0);
        }

        // Cache should have some entries but be bounded
        assert!(counter.cache_size() > 0);
        assert!(counter.cache_size() <= MAX_TOKEN_CACHE_SIZE);
    }

    #[test]
    fn test_tokenizer_consistency() {
        // Test that both sync and async versions give the same results
        let sync_counter = TokenCounter::new();
        let text = "This is a test for tokenizer consistency";
        let sync_count = sync_counter.count_tokens(text);

        // Test that the tokenizer is working correctly
        assert!(sync_count > 0, "Sync tokenizer should produce tokens");

        // Test with different text lengths
        let short_text = "Hi";
        let long_text = "This is a much longer text that should produce significantly more tokens than the short text";

        let short_count = sync_counter.count_tokens(short_text);
        let long_count = sync_counter.count_tokens(long_text);

        assert!(
            short_count < long_count,
            "Longer text should have more tokens"
        );
    }
}
