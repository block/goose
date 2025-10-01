use ahash::AHasher;
use dashmap::DashMap;
use rmcp::model::Tool;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tiktoken_rs::CoreBPE;
use tokio::sync::OnceCell;

use crate::conversation::message::Message;

static TOKENIZER: OnceCell<Arc<CoreBPE>> = OnceCell::const_new();

const MAX_TOKEN_CACHE_SIZE: usize = 10_000;

// token use for various bits of a tool calls:
const FUNC_INIT: usize = 7;
const PROP_INIT: usize = 3;
const PROP_KEY: usize = 3;
const ENUM_INIT: isize = -3;
const ENUM_ITEM: usize = 3;
const FUNC_END: usize = 12;

pub struct AsyncTokenCounter {
    tokenizer: Arc<CoreBPE>,
    token_cache: Arc<DashMap<u64, usize>>,
}

pub struct TokenCounter {
    tokenizer: Arc<CoreBPE>,
}

impl AsyncTokenCounter {
    pub async fn new() -> Result<Self, String> {
        let tokenizer = get_tokenizer().await?;
        Ok(Self {
            tokenizer,
            token_cache: Arc::new(DashMap::new()),
        })
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        let mut hasher = AHasher::default();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(count) = self.token_cache.get(&hash) {
            return *count;
        }

        let tokens = self.tokenizer.encode_with_special_tokens(text);
        let count = tokens.len();

        if self.token_cache.len() >= MAX_TOKEN_CACHE_SIZE {
            if let Some(entry) = self.token_cache.iter().next() {
                let old_hash = *entry.key();
                self.token_cache.remove(&old_hash);
            }
        }

        self.token_cache.insert(hash, count);
        count
    }

    pub fn count_tokens_for_tools(&self, tools: &[Tool]) -> usize {
        let mut func_token_count = 0;
        if !tools.is_empty() {
            for tool in tools {
                func_token_count += FUNC_INIT;
                let name = &tool.name;
                let description = &tool
                    .description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default()
                    .trim_end_matches('.');

                let line = format!("{}:{}", name, description);
                func_token_count += self.count_tokens(&line);

                if let Some(serde_json::Value::Object(properties)) =
                    tool.input_schema.get("properties")
                {
                    if !properties.is_empty() {
                        func_token_count += PROP_INIT;
                        for (key, value) in properties {
                            func_token_count += PROP_KEY;
                            let p_name = key;
                            let p_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            let p_desc = value
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .trim_end_matches('.');

                            let line = format!("{}:{}:{}", p_name, p_type, p_desc);
                            func_token_count += self.count_tokens(&line);

                            if let Some(enum_values) = value.get("enum").and_then(|v| v.as_array())
                            {
                                func_token_count =
                                    func_token_count.saturating_add_signed(ENUM_INIT);
                                for item in enum_values {
                                    if let Some(item_str) = item.as_str() {
                                        func_token_count =
                                            func_token_count.saturating_add_signed(ENUM_INIT);
                                        func_token_count += self.count_tokens(item_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            func_token_count += FUNC_END;
        }

        func_token_count
    }

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
                    if let Ok(tool_call) = tool_request.tool_call.as_ref() {
                        let text = format!(
                            "{}:{}:{:?}",
                            tool_request.id, tool_call.name, tool_call.arguments
                        );
                        num_tokens += self.count_tokens(&text);
                    }
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
    pub fn new() -> Self {
        /// Count tokens for a piece of text using our single tokenizer.
        let tokenizer = get_tokenizer_blocking().expect("Failed to initialize tokenizer");
        Self { tokenizer }
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        tokens.len()
    }

    pub fn count_tokens_for_tools(&self, tools: &[Tool]) -> usize {
        let mut func_token_count = 0;
        if !tools.is_empty() {
            for tool in tools {
                func_token_count += FUNC_INIT;
                let name = &tool.name;
                let description = &tool
                    .description
                    .as_ref()
                    .map(|d| d.as_ref())
                    .unwrap_or_default()
                    .trim_end_matches('.');
                let line = format!("{}:{}", name, description);
                func_token_count += self.count_tokens(&line);

                if let Some(serde_json::Value::Object(properties)) =
                    tool.input_schema.get("properties")
                {
                    if !properties.is_empty() {
                        func_token_count += PROP_INIT;
                        for (key, value) in properties {
                            func_token_count += PROP_KEY;
                            let p_name = key;
                            let p_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            let p_desc = value
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .trim_end_matches('.');
                            let line = format!("{}:{}:{}", p_name, p_type, p_desc);
                            func_token_count += self.count_tokens(&line);
                            if let Some(enum_values) = value.get("enum").and_then(|v| v.as_array())
                            {
                                func_token_count =
                                    func_token_count.saturating_add_signed(ENUM_INIT);
                                for item in enum_values {
                                    if let Some(item_str) = item.as_str() {
                                        func_token_count =
                                            func_token_count.saturating_add_signed(ENUM_INIT);
                                        func_token_count += self.count_tokens(item_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            func_token_count += FUNC_END;
        }

        func_token_count
    }

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
                    if let Ok(tool_call) = tool_request.tool_call.as_ref() {
                        let text = format!(
                            "{}:{}:{:?}",
                            tool_request.id, tool_call.name, tool_call.arguments
                        );
                        num_tokens += self.count_tokens(&text);
                    }
                } else if let Some(tool_response_text) = content.as_tool_response_text() {
                    num_tokens += self.count_tokens(&tool_response_text);
                } else {
                    continue;
                }
            }
        }

        if !tools.is_empty() {
            num_tokens += self.count_tokens_for_tools(tools);
        }

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

async fn get_tokenizer() -> Result<Arc<CoreBPE>, String> {
    let tokenizer = TOKENIZER
        .get_or_init(|| async {
            tiktoken_rs::o200k_base().map(Arc::new).unwrap_or_else(|e| {
                eprintln!("Failed to initialize o200k_base tokenizer: {}", e);
                Arc::new(tiktoken_rs::cl100k_base().expect("Fallback tokenizer failed"))
            })
        })
        .await;
    Ok(tokenizer.clone())
}

fn get_tokenizer_blocking() -> Result<Arc<CoreBPE>, String> {
    if let Some(tokenizer) = TOKENIZER.get() {
        return Ok(tokenizer.clone());
    }

    match tiktoken_rs::o200k_base() {
        Ok(bpe) => {
            let tokenizer = Arc::new(bpe);
            let _ = TOKENIZER.set(tokenizer.clone());
            Ok(tokenizer)
        }
        Err(e) => Err(format!("Failed to initialize o200k_base tokenizer: {}", e)),
    }
}

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
    async fn test_async_token_caching() {
        let counter = create_async_token_counter().await.unwrap();

        let text = "This is a test for caching functionality";

        let count1 = counter.count_tokens(text);
        assert_eq!(counter.cache_size(), 1);

        let count2 = counter.count_tokens(text);
        assert_eq!(count1, count2);
        assert_eq!(counter.cache_size(), 1);

        let count3 = counter.count_tokens("Different text");
        assert_eq!(counter.cache_size(), 2);
        assert_ne!(count1, count3);
    }

    #[tokio::test]
    async fn test_async_cache_management() {
        let counter = create_async_token_counter().await.unwrap();

        counter.count_tokens("First text");
        counter.count_tokens("Second text");
        counter.count_tokens("Third text");

        assert_eq!(counter.cache_size(), 3);

        counter.clear_cache();
        assert_eq!(counter.cache_size(), 0);

        let count = counter.count_tokens("First text");
        assert!(count > 0);
        assert_eq!(counter.cache_size(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_token_counter_creation() {
        let handles: Vec<_> = (0..10)
            .map(|_| tokio::spawn(async { create_async_token_counter().await.unwrap() }))
            .collect();

        let counters: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let text = "Test concurrent creation";
        let expected_count = counters[0].count_tokens(text);

        for counter in &counters {
            assert_eq!(counter.count_tokens(text), expected_count);
        }
    }

    #[tokio::test]
    async fn test_cache_eviction_behavior() {
        let counter = create_async_token_counter().await.unwrap();

        let mut cached_texts = Vec::new();
        for i in 0..50 {
            let text = format!("Test string number {}", i);
            counter.count_tokens(&text);
            cached_texts.push(text);
        }

        assert!(counter.cache_size() <= MAX_TOKEN_CACHE_SIZE);

        let recent_text = &cached_texts[cached_texts.len() - 1];
        let start_size = counter.cache_size();

        counter.count_tokens(recent_text);
        assert_eq!(counter.cache_size(), start_size);
    }

    #[tokio::test]
    async fn test_concurrent_cache_operations() {
        let counter = std::sync::Arc::new(create_async_token_counter().await.unwrap());

        let handles: Vec<_> = (0..20)
            .map(|i| {
                let counter_clone = counter.clone();
                tokio::spawn(async move {
                    let text = format!("Concurrent test {}", i % 5);
                    counter_clone.count_tokens(&text)
                })
            })
            .collect();

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        for result in results {
            assert!(result > 0);
        }

        assert!(counter.cache_size() > 0);
        assert!(counter.cache_size() <= MAX_TOKEN_CACHE_SIZE);
    }

    #[test]
    fn test_tokenizer_consistency() {
        let sync_counter = TokenCounter::new();
        let text = "This is a test for tokenizer consistency";
        let sync_count = sync_counter.count_tokens(text);

        assert!(sync_count > 0, "Sync tokenizer should produce tokens");

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
