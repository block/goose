use lru::LruCache;
use rmcp::model::Tool;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tiktoken_rs::CoreBPE;
use tokio::sync::OnceCell;

use crate::conversation::message::{Message, MessageContent};
use crate::mcp_utils::extract_text_from_resource;

static TOKENIZER: OnceCell<Arc<CoreBPE>> = OnceCell::const_new();

/// Per-message framing the provider adds around each message's content.
const TOKENS_PER_MESSAGE: usize = 4;

const MAX_TOKEN_CACHE_SIZE: usize = 1_024;

// token use for various bits of a tool calls:
const FUNC_INIT: usize = 7;
const PROP_INIT: usize = 3;
const PROP_KEY: usize = 3;
const ENUM_INIT: isize = -3;
const ENUM_ITEM: usize = 3;
const FUNC_END: usize = 12;

pub struct TokenCounter {
    tokenizer: Arc<CoreBPE>,
    token_cache: Mutex<LruCache<TokenCacheKey, usize>>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct TokenCacheKey {
    len: usize,
    hash: [u8; 32],
}

impl TokenCacheKey {
    fn from_text(text: &str) -> Self {
        Self {
            len: text.len(),
            hash: *blake3::hash(text.as_bytes()).as_bytes(),
        }
    }
}

impl TokenCounter {
    pub async fn new() -> Result<Self, String> {
        let tokenizer = get_tokenizer().await?;
        let cache_capacity =
            NonZeroUsize::new(MAX_TOKEN_CACHE_SIZE).expect("token cache capacity must be non-zero");
        Ok(Self {
            tokenizer,
            token_cache: Mutex::new(LruCache::new(cache_capacity)),
        })
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        let cache_key = TokenCacheKey::from_text(text);
        if let Some(count) = self
            .token_cache
            .lock()
            .expect("token cache mutex poisoned")
            .get(&cache_key)
            .copied()
        {
            return count;
        }

        let tokens = self.tokenizer.encode_with_special_tokens(text);
        let count = tokens.len();

        self.token_cache
            .lock()
            .expect("token cache mutex poisoned")
            .put(cache_key, count);
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
                    .as_deref()
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
                                        func_token_count += ENUM_ITEM;
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

    pub fn count_message_tokens(&self, message: &Message) -> usize {
        let mut num_tokens = 0;

        if !message.metadata.agent_visible {
            return 0;
        }

        num_tokens += TOKENS_PER_MESSAGE;
        for content in &message.content {
            if let Some(text) = message_content_token_text(content) {
                num_tokens += self.count_tokens(&text);
            }
        }

        num_tokens
    }

    pub fn count_chat_tokens(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> usize {
        let mut num_tokens = 0;

        if !system_prompt.is_empty() {
            num_tokens += self.count_tokens(system_prompt) + TOKENS_PER_MESSAGE;
        }

        for message in messages {
            num_tokens += self.count_message_tokens(message);
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
        self.token_cache
            .lock()
            .expect("token cache mutex poisoned")
            .clear();
    }

    pub fn cache_size(&self) -> usize {
        self.token_cache
            .lock()
            .expect("token cache mutex poisoned")
            .len()
    }
}

pub(crate) fn message_content_token_text(content: &MessageContent) -> Option<String> {
    match content {
        MessageContent::Text(text) => Some(text.text.clone()),
        MessageContent::Image(image) => Some(format!(
            "[image: {}, {} base64 chars]",
            image.mime_type,
            image.data.len()
        )),
        MessageContent::ToolRequest(request) => match request.tool_call.as_ref() {
            Ok(call) => Some(format!(
                "{}:{}:{}",
                request.id,
                call.name,
                serde_json::to_string(&call.arguments)
                    .unwrap_or_else(|_| format!("{:?}", call.arguments))
            )),
            Err(error) => Some(format!("[tool call error: {}] {}", request.id, error)),
        },
        MessageContent::FrontendToolRequest(request) => match request.tool_call.as_ref() {
            Ok(call) => Some(format!(
                "{}:{}:{}",
                request.id,
                call.name,
                serde_json::to_string(&call.arguments)
                    .unwrap_or_else(|_| format!("{:?}", call.arguments))
            )),
            Err(error) => Some(format!(
                "[frontend tool call error: {}] {}",
                request.id, error
            )),
        },
        MessageContent::ToolResponse(response) => match response.tool_result.as_ref() {
            Ok(result) => {
                if result.content.is_empty() {
                    return Some("[tool result]".to_string());
                }

                let parts: Vec<String> = result
                    .content
                    .iter()
                    .map(|content| match content {
                        rmcp::model::ContentBlock::Text(text) => text.text.clone(),
                        rmcp::model::ContentBlock::Image(image) => format!(
                            "[image: {}, {} base64 chars]",
                            image.mime_type,
                            image.data.len()
                        ),
                        rmcp::model::ContentBlock::Resource(resource) => {
                            extract_text_from_resource(&resource.resource)
                        }
                        rmcp::model::ContentBlock::ResourceLink(_) => "[resource link]".to_string(),
                        rmcp::model::ContentBlock::Audio(_) => "[audio content]".to_string(),
                        _ => "[unsupported content]".to_string(),
                    })
                    .collect();
                Some(parts.join("\n"))
            }
            Err(error) => Some(format!("[tool result error: {}] {}", response.id, error)),
        },
        MessageContent::Thinking(thinking) => Some(thinking.thinking.clone()),
        MessageContent::RedactedThinking(redacted) => Some(format!(
            "[redacted thinking: {} base64 chars]",
            redacted.data.len()
        )),
        // UI-only blocks. Every format converter drops these before the
        // request goes out, so counting them would bill context that the
        // provider never sees.
        MessageContent::ToolConfirmationRequest(_)
        | MessageContent::ActionRequired(_)
        | MessageContent::SystemNotification(_) => None,
    }
}

async fn get_tokenizer() -> Result<Arc<CoreBPE>, String> {
    Ok(TOKENIZER
        .get_or_init(|| async {
            let bpe = tiktoken_rs::o200k_base().expect("Failed to initialize o200k_base tokenizer");
            Arc::new(bpe)
        })
        .await
        .clone())
}

pub async fn create_token_counter() -> Result<TokenCounter, String> {
    TokenCounter::new().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::SystemNotificationType;
    use rmcp::model::{ErrorCode, ErrorData};

    #[tokio::test]
    async fn test_token_caching() {
        let counter = create_token_counter().await.unwrap();

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
    async fn test_cache_management() {
        let counter = create_token_counter().await.unwrap();

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
            .map(|_| tokio::spawn(async { create_token_counter().await.unwrap() }))
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
        let counter = create_token_counter().await.unwrap();

        let mut cached_texts = Vec::new();
        for i in 0..=MAX_TOKEN_CACHE_SIZE {
            let text = format!("Test string number {}", i);
            counter.count_tokens(&text);
            cached_texts.push(text);
        }

        assert_eq!(counter.cache_size(), MAX_TOKEN_CACHE_SIZE);

        let recent_text = &cached_texts[cached_texts.len() - 1];
        let start_size = counter.cache_size();

        counter.count_tokens(recent_text);
        assert_eq!(counter.cache_size(), start_size);
    }

    #[tokio::test]
    async fn test_concurrent_cache_operations() {
        let counter = std::sync::Arc::new(create_token_counter().await.unwrap());

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

    #[tokio::test]
    async fn test_message_accounting_includes_non_text_provider_content() {
        let counter = create_token_counter().await.unwrap();
        let message = Message::user()
            .with_image("iVBORw0KGgo=", "image/png")
            .with_thinking("reasoning text", "signature")
            .with_tool_response(
                "call_1",
                Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "tool failed".to_string(),
                    None,
                )),
            );

        assert!(counter.count_message_tokens(&message) > 4);
        assert!(message_content_token_text(&message.content[0])
            .unwrap()
            .contains("image/png"));
        assert_eq!(
            message_content_token_text(&message.content[1]).unwrap(),
            "reasoning text"
        );
        assert!(message_content_token_text(&message.content[2])
            .unwrap()
            .contains("tool failed"));
    }

    #[tokio::test]
    async fn ui_only_blocks_are_not_counted_but_provider_visible_ones_are() {
        let counter = create_token_counter().await.unwrap();

        let ui_only = Message::assistant()
            .with_content(MessageContent::action_required(
                "req-1",
                "shell".to_string(),
                serde_json::Map::new(),
                Some("a prompt long enough to register as tokens".to_string()),
            ))
            .with_content(MessageContent::system_notification(
                SystemNotificationType::InlineMessage,
                "a notice the provider never receives",
            ));
        assert_eq!(
            counter.count_message_tokens(&ui_only),
            TOKENS_PER_MESSAGE,
            "blocks every format converter drops must not be billed"
        );

        let thinking = Message::assistant().with_thinking("internal reasoning", "sig");
        assert!(
            counter.count_message_tokens(&thinking) > TOKENS_PER_MESSAGE,
            "thinking is sent to the provider and must be counted"
        );
    }
}
