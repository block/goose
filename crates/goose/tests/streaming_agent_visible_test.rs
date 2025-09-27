use async_trait::async_trait;
use futures::stream;
use goose::conversation::message::{Message, MessageMetadata};
use goose::model::ModelConfig;
use goose::providers::base::{MessageStream, Provider, ProviderUsage, Usage};
use goose::providers::errors::ProviderError;
use rmcp::model::Tool;

/// Mock provider to test that stream() filters messages with agentVisible=false
struct MockStreamingProvider {
    model: ModelConfig,
    captured_messages: std::sync::Arc<std::sync::Mutex<Vec<Message>>>,
}

impl MockStreamingProvider {
    fn new() -> Self {
        Self {
            model: ModelConfig::new("test-model").unwrap(),
            captured_messages: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn get_captured_messages(&self) -> Vec<Message> {
        self.captured_messages.lock().unwrap().clone()
    }
}

#[async_trait]
impl Provider for MockStreamingProvider {
    fn metadata() -> goose::providers::base::ProviderMetadata {
        goose::providers::base::ProviderMetadata::empty()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // In real providers, format_messages would filter these
        // For testing, we simulate that filtering here
        let filtered: Vec<Message> = messages
            .iter()
            .filter(|m| m.is_agent_visible())
            .cloned()
            .collect();
        *self.captured_messages.lock().unwrap() = filtered;

        Ok((
            Message::assistant().with_text("Test response"),
            ProviderUsage::new("test-model".to_string(), Usage::default()),
        ))
    }

    async fn stream(
        &self,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // In real providers, format_messages would filter these
        // For testing, we simulate that filtering here
        let filtered: Vec<Message> = messages
            .iter()
            .filter(|m| m.is_agent_visible())
            .cloned()
            .collect();
        *self.captured_messages.lock().unwrap() = filtered;

        // Return a simple stream with one message
        let message = Message::assistant().with_text("Streamed response");
        let usage = ProviderUsage::new("test-model".to_string(), Usage::default());

        let stream = stream::once(async move { Ok((Some(message), Some(usage))) });

        Ok(Box::pin(stream))
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_streaming_filters_agent_invisible_messages() {
    let provider = MockStreamingProvider::new();

    // Create a mix of messages with different visibility settings
    let messages = vec![
        // Normal message - should be included
        Message::user().with_text("Hello, I need help"),
        // Agent-only message - should be included
        Message::assistant()
            .with_text("This is visible to agent only")
            .with_metadata(MessageMetadata::agent_only()),
        // User-only message - should be EXCLUDED
        Message::assistant()
            .with_text("This is visible to user only")
            .with_metadata(MessageMetadata::user_only()),
        // Invisible message - should be EXCLUDED
        Message::assistant()
            .with_text("This is invisible to both")
            .with_metadata(MessageMetadata::invisible()),
        // Another normal message - should be included
        Message::user().with_text("Please continue"),
    ];

    // Call the stream method
    let mut stream = provider
        .stream("Test system prompt", &messages, &[])
        .await
        .expect("Stream should succeed");

    // Consume the stream
    while let Some(result) = futures::StreamExt::next(&mut stream).await {
        let (_message, _usage) = result.expect("Stream item should be valid");
    }

    // Check what messages were actually passed to stream
    let captured = provider.get_captured_messages();

    // Should only have 3 messages (the ones with agent_visible=true)
    assert_eq!(captured.len(), 3, "Should only pass agent-visible messages");

    // Verify the correct messages were passed
    assert_eq!(captured[0].as_concat_text(), "Hello, I need help");
    assert_eq!(
        captured[1].as_concat_text(),
        "This is visible to agent only"
    );
    assert_eq!(captured[2].as_concat_text(), "Please continue");

    // Verify that user-only and invisible messages were filtered out
    for msg in &captured {
        assert!(
            msg.is_agent_visible(),
            "All messages passed to stream should be agent-visible"
        );
    }
}

#[tokio::test]
async fn test_complete_filters_agent_invisible_messages() {
    let provider = MockStreamingProvider::new();

    // Create a mix of messages with different visibility settings
    let messages = vec![
        Message::user().with_text("Hello"),
        Message::assistant()
            .with_text("Agent visible")
            .with_metadata(MessageMetadata::default()),
        Message::assistant()
            .with_text("User only - should be filtered")
            .with_metadata(MessageMetadata::user_only()),
        Message::user().with_text("Continue"),
    ];

    // Call the complete method
    let (_response, _usage) = provider
        .complete("Test system", &messages, &[])
        .await
        .expect("Complete should succeed");

    // Check what messages were actually passed to complete_with_model
    let captured = provider.get_captured_messages();

    // Should only have 3 messages (excluding the user-only one)
    assert_eq!(
        captured.len(),
        3,
        "Should filter out non-agent-visible messages"
    );
    assert_eq!(captured[0].as_concat_text(), "Hello");
    assert_eq!(captured[1].as_concat_text(), "Agent visible");
    assert_eq!(captured[2].as_concat_text(), "Continue");
}

#[tokio::test]
async fn test_all_messages_filtered_edge_case() {
    let provider = MockStreamingProvider::new();

    // Create messages that are all invisible to the agent
    let messages = vec![
        Message::user()
            .with_text("User only 1")
            .with_metadata(MessageMetadata::user_only()),
        Message::assistant()
            .with_text("User only 2")
            .with_metadata(MessageMetadata::user_only()),
        Message::user()
            .with_text("Invisible")
            .with_metadata(MessageMetadata::invisible()),
    ];

    // Call the stream method
    let mut stream = provider
        .stream("Test system", &messages, &[])
        .await
        .expect("Stream should succeed even with no agent-visible messages");

    // Consume the stream
    while let Some(result) = futures::StreamExt::next(&mut stream).await {
        let (_message, _usage) = result.expect("Stream item should be valid");
    }

    // Check that no messages were passed to stream
    let captured = provider.get_captured_messages();
    assert_eq!(captured.len(), 0, "All messages should be filtered out");
}
