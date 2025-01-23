use async_trait::async_trait;
use futures::stream::BoxStream;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::Agent;
use crate::agents::capabilities::{Capabilities, ResourceItem};
use crate::agents::extension::{ExtensionConfig, ExtensionError, ExtensionResult};
use crate::message::{Message, ToolRequest};
use crate::providers::base::Provider;
use crate::providers::base::ProviderUsage;
use crate::register_agent;
use crate::token_counter::TokenCounter;
use mcp_core::{Role, Tool};
use serde_json::Value;

/// Agent impl. that truncates oldest messages when payload over LLM ctx-limit
pub struct TruncateAgent {
    capabilities: Mutex<Capabilities>,
    token_counter: TokenCounter,
}

impl TruncateAgent {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        Self {
            capabilities: Mutex::new(Capabilities::new(provider)),
            token_counter,
        }
    }

    async fn enforce_ctx_limit(
        &self,
        extension_prompt: &str,
        tools: &[Tool],
        messages: &[Message],
        target_limit: usize,
        resource_items: &mut [ResourceItem],
    ) -> ExtensionResult<Vec<Message>> {
        // Flatten all resource content into a vector of strings
        let resources: Vec<String> = resource_items
            .iter()
            .map(|item| item.content.clone())
            .collect();

        let approx_count =
            self.token_counter
                .count_everything(extension_prompt, messages, tools, &resources);

        let mut new_messages = messages.to_vec();
        if approx_count > target_limit {
            new_messages = self.drop_messages(messages, approx_count, target_limit);
            if new_messages.is_empty() {
                return Err(ExtensionError::ContextLimit);
            }
        }

        Ok(new_messages)
    }

    fn text_content_size(&self, message: Option<&Message>) -> usize {
        if let Some(msg) = message {
            let mut approx_count = 0;
            for content in msg.content.iter() {
                if let Some(content_text) = content.as_text() {
                    approx_count += self.token_counter.count_tokens(content_text);
                }
            }
            return approx_count;
        }

        0
    }

    fn drop_messages(
        &self,
        messages: &[Message],
        approx_count: usize,
        target_limit: usize,
    ) -> Vec<Message> {
        debug!(
            "[WARNING] Conversation history has size: {} exceeding the token budget of {}. \
            Dropping oldest messages.",
            approx_count,
            approx_count - target_limit
        );

        let user_msg_size = self.text_content_size(messages.last());
        if messages.last().unwrap().role == Role::User && user_msg_size > target_limit {
            debug!(
                "[WARNING] User message {} exceeds token budget {}.",
                user_msg_size,
                user_msg_size - target_limit
            );
            return Vec::new();
        }

        let mut truncated_conv: VecDeque<Message> = VecDeque::from(messages.to_vec());
        let mut current_tokens = approx_count;

        while current_tokens > target_limit && truncated_conv.len() > 1 {
            let user_msg = truncated_conv.pop_front().unwrap();
            let user_msg_size = self.text_content_size(Some(&user_msg));
            let assistant_msg = truncated_conv.pop_front().unwrap();
            let assistant_msg_size = self.text_content_size(Some(&assistant_msg));

            current_tokens = current_tokens.saturating_sub(user_msg_size + assistant_msg_size);
        }

        Vec::from(truncated_conv)
    }
}

#[async_trait]
impl Agent for TruncateAgent {
    #[instrument(skip(self, messages), fields(user_message))]
    async fn reply(
        &self,
        messages: &[Message],
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message>>> {
        let reply_span = tracing::Span::current();
        let mut capabilities = self.capabilities.lock().await;
        let tools = capabilities.get_prefixed_tools().await?;
        let extension_prompt = capabilities.get_extension_prompt().await;
        let estimated_limit = capabilities
            .provider()
            .get_model_config()
            .get_estimated_limit();

        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        let mut messages = self
            .enforce_ctx_limit(
                &extension_prompt,
                &tools,
                messages,
                estimated_limit,
                &mut capabilities.get_resources().await?,
            )
            .await?;

        Ok(Box::pin(async_stream::try_stream! {
            let _reply_guard = reply_span.enter();

            loop {
                messages = self
                    .enforce_ctx_limit(
                        &extension_prompt,
                        &tools,
                        &messages,
                        estimated_limit,
                        &mut capabilities.get_resources().await?,
                    )
                    .await?;

                // Get completion from provider
                let (response, usage) = capabilities.provider().complete(
                    &extension_prompt,
                    &messages,
                    &tools,
                ).await?;
                capabilities.record_usage(usage).await;

                // Yield the assistant's response
                yield response.clone();

                tokio::task::yield_now().await;

                // First collect any tool requests
                let tool_requests: Vec<&ToolRequest> = response.content
                    .iter()
                    .filter_map(|content| content.as_tool_request())
                    .collect();

                if tool_requests.is_empty() {
                    break;
                }

                // Then dispatch each in parallel
                let futures: Vec<_> = tool_requests
                    .iter()
                    .filter_map(|request| request.tool_call.clone().ok())
                    .map(|tool_call| capabilities.dispatch_tool_call(tool_call))
                    .collect();

                // Process all the futures in parallel but wait until all are finished
                let outputs = futures::future::join_all(futures).await;

                // Create a message with the responses
                let mut message_tool_response = Message::user();
                // Now combine these into MessageContent::ToolResponse using the original ID
                for (request, output) in tool_requests.iter().zip(outputs.into_iter()) {
                    message_tool_response = message_tool_response.with_tool_response(
                        request.id.clone(),
                        output,
                    );
                }

                let tool_resp_size = self.text_content_size(
                    Some(&message_tool_response),
                );
                if tool_resp_size > estimated_limit {
                    // don't push assistant response or tool_response into history
                    // last message is `user message => tool call`, remove it from history too
                    messages.pop();
                    continue;
                }

                yield message_tool_response.clone();
                messages.push(response);
                messages.push(message_tool_response);
            }
        }))
    }

    async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()> {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.add_extension(extension).await
    }

    async fn remove_extension(&mut self, name: &str) {
        let mut capabilities = self.capabilities.lock().await;
        capabilities
            .remove_extension(name)
            .await
            .expect("Failed to remove extension");
    }

    async fn list_extensions(&self) -> Vec<String> {
        let capabilities = self.capabilities.lock().await;
        capabilities
            .list_extensions()
            .await
            .expect("Failed to list extensions")
    }

    async fn passthrough(&self, _extension: &str, _request: Value) -> ExtensionResult<Value> {
        // TODO implement
        Ok(Value::Null)
    }

    async fn usage(&self) -> Vec<ProviderUsage> {
        let capabilities = self.capabilities.lock().await;
        capabilities.get_usage().await
    }
}

register_agent!("truncate", TruncateAgent);

#[cfg(test)]
mod tests {
    use crate::agents::truncate::TruncateAgent;
    use crate::message::Message;
    use crate::model::ModelConfig;
    use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
    use mcp_core::{Content, Tool};
    use std::iter;

    // Mock Provider implementation for testing
    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _extension: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(Message, ProviderUsage)> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    const SMALL_MESSAGE: &str = "This is a test, this is just a test, this is only a test.\n";

    async fn call_enforce_ctx_limit(conversation: &[Message]) -> anyhow::Result<Vec<Message>> {
        let mock_model_config =
            ModelConfig::new("test-model".to_string()).with_context_limit(200_000.into());
        let provider = Box::new(MockProvider {
            model_config: mock_model_config,
        });
        let agent = TruncateAgent::new(provider);

        let mut capabilities = agent.capabilities.lock().await;
        let tools = capabilities.get_prefixed_tools().await?;
        let extension_prompt = capabilities.get_extension_prompt().await;
        let estimated_limit = capabilities
            .provider()
            .get_model_config()
            .get_estimated_limit();

        let messages = agent
            .enforce_ctx_limit(
                &extension_prompt,
                &tools,
                conversation,
                estimated_limit,
                &mut capabilities.get_resources().await?,
            )
            .await?;

        Ok(messages)
    }

    fn create_basic_valid_conversation(
        interactions_count: usize,
        is_tool_use: bool,
    ) -> Vec<Message> {
        let mut conversation = Vec::<Message>::new();

        if is_tool_use {
            (0..interactions_count).for_each(|i| {
                let tool_output = format!("{:?}{}", SMALL_MESSAGE, i);
                conversation.push(
                    Message::user()
                        .with_tool_response("id:0", Ok(vec![Content::text(tool_output)])),
                );
                conversation.push(Message::assistant().with_text(format!(
                    "{:?}{}",
                    SMALL_MESSAGE,
                    i + 1
                )));
            });
        } else {
            (0..interactions_count).for_each(|i| {
                conversation.push(Message::user().with_text(format!("{:?}{}", SMALL_MESSAGE, i)));
                conversation.push(Message::assistant().with_text(format!(
                    "{:?}{}",
                    SMALL_MESSAGE,
                    i + 1
                )));
            });
        }

        conversation
    }
    #[tokio::test]
    async fn test_simple_conversation_no_truncation() -> anyhow::Result<()> {
        let conversation = create_basic_valid_conversation(1, false);
        let messages = call_enforce_ctx_limit(&conversation).await?;
        assert_eq!(messages.len(), conversation.len());
        Ok(())
    }
    #[tokio::test]
    async fn test_truncation_when_conversation_history_too_big() -> anyhow::Result<()> {
        let conversation = create_basic_valid_conversation(5000, false);
        let messages = call_enforce_ctx_limit(&conversation).await?;
        assert!(conversation.len() > messages.len());
        assert!(!messages.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_truncation_when_single_user_message_too_big() -> anyhow::Result<()> {
        let oversized_message: String = iter::repeat(SMALL_MESSAGE)
            .take(10000)
            .collect::<Vec<&str>>()
            .join("");
        let mut conversation = create_basic_valid_conversation(3, false);
        conversation.push(Message::user().with_text(oversized_message));

        let messages = call_enforce_ctx_limit(&conversation).await;

        assert!(messages.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_truncation_when_tool_response_set_too_big() -> anyhow::Result<()> {
        let conversation = create_basic_valid_conversation(5000, true);
        let messages = call_enforce_ctx_limit(&conversation).await?;
        assert!(conversation.len() > messages.len());
        assert!(!messages.is_empty());
        Ok(())
    }
}
