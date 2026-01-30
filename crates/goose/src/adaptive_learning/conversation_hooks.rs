use crate::conversation::{Conversation, message::Message};
use crate::providers::base::{Provider, ProviderUsage};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Event emitted when a conversation is processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEvent {
    pub conversation_id: String,
    pub session_id: Option<String>,
    pub messages: Vec<Message>,
    pub provider_used: String,
    pub model_used: String,
    pub response_time_ms: Option<f32>,
    pub tokens_used: Option<usize>,
    pub error_occurred: bool,
    pub timestamp: DateTime<Utc>,
}

/// Hook that captures conversation data for adaptive learning
pub struct ConversationHook {
    event_sender: mpsc::UnboundedSender<ConversationEvent>,
    enabled: Arc<RwLock<bool>>,
}

impl ConversationHook {
    pub fn new(event_sender: mpsc::UnboundedSender<ConversationEvent>) -> Self {
        Self {
            event_sender,
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Enable or disable the conversation hook
    pub async fn set_enabled(&self, enabled: bool) {
        let mut enabled_guard = self.enabled.write().await;
        *enabled_guard = enabled;
        info!("Conversation hook {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if the hook is enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Process a completed conversation
    pub async fn process_conversation(
        &self,
        conversation_id: String,
        session_id: Option<String>,
        messages: Vec<Message>,
        provider_used: String,
        model_used: String,
        response_time_ms: Option<f32>,
        tokens_used: Option<usize>,
        error_occurred: bool,
    ) -> Result<()> {
        if !self.is_enabled().await {
            return Ok(());
        }

        let event = ConversationEvent {
            conversation_id: conversation_id.clone(),
            session_id: session_id.clone(),
            messages,
            provider_used: provider_used.clone(),
            model_used: model_used.clone(),
            response_time_ms,
            tokens_used,
            error_occurred,
            timestamp: Utc::now(),
        };

        debug!("Capturing conversation event: {} (provider: {}, model: {})", 
               conversation_id, provider_used, model_used);

        if let Err(e) = self.event_sender.send(event) {
            warn!("Failed to send conversation event: {}", e);
        }

        Ok(())
    }
}

/// Middleware that wraps providers to capture conversation data
pub struct ConversationMiddleware {
    inner_provider: Arc<dyn Provider>,
    conversation_hook: Arc<ConversationHook>,
    session_id: Option<String>,
}

impl ConversationMiddleware {
    pub fn new(
        inner_provider: Arc<dyn Provider>,
        conversation_hook: Arc<ConversationHook>,
        session_id: Option<String>,
    ) -> Self {
        Self {
            inner_provider,
            conversation_hook,
            session_id,
        }
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

#[async_trait]
impl Provider for ConversationMiddleware {
    fn metadata() -> crate::providers::base::ProviderMetadata {
        // This should never be called directly on the middleware
        panic!("ConversationMiddleware should not be used for metadata queries");
    }

    fn get_model_config(&self) -> crate::model::ModelConfig {
        self.inner_provider.get_model_config()
    }

    async fn complete_with_model(
        &self,
        model_config: &crate::model::ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[rmcp::model::Tool],
    ) -> Result<(Message, ProviderUsage), crate::providers::errors::ProviderError> {
        let start_time = std::time::Instant::now();
        let conversation_id = Uuid::new_v4().to_string();

        // Call the inner provider
        let result = self.inner_provider.complete_with_model(model_config, system, messages, tools).await;

        let response_time_ms = start_time.elapsed().as_millis() as f32;
        let error_occurred = result.is_err();

        // Prepare conversation data
        let mut conversation_messages = Vec::new();
        
        // Add system message if present
        if !system.is_empty() {
            conversation_messages.push(Message::assistant().with_text(system));
        }
        
        // Add input messages
        conversation_messages.extend_from_slice(messages);

        // Add response message if successful
        if let Ok((ref response_message, _)) = result {
            conversation_messages.push(response_message.clone());
        }

        // Calculate tokens used (rough estimate)
        let tokens_used = conversation_messages.iter()
            .map(|msg| msg.as_concat_text().len() / 4) // Rough token estimate
            .sum();

        // Capture the conversation
        if let Err(e) = self.conversation_hook.process_conversation(
            conversation_id,
            self.session_id.clone(),
            conversation_messages,
            "middleware".to_string(), // TODO: Get actual provider name
            model_config.model_name.clone(),
            Some(response_time_ms),
            Some(tokens_used),
            error_occurred,
        ).await {
            warn!("Failed to capture conversation: {}", e);
        }

        result
    }

    fn supports_streaming(&self) -> bool {
        self.inner_provider.supports_streaming()
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[rmcp::model::Tool],
    ) -> Result<crate::providers::base::MessageStream, crate::providers::errors::ProviderError> {
        let start_time = std::time::Instant::now();
        let conversation_id = Uuid::new_v4().to_string();
        let conversation_hook = self.conversation_hook.clone();
        let session_id = self.session_id.clone();
        let model_config = self.get_model_config();

        // Call the inner provider
        let result = self.inner_provider.stream(system, messages, tools).await;

        match result {
            Ok(stream) => {
                // Wrap the stream to capture the final conversation
                let wrapped_stream = self.wrap_stream(
                    stream,
                    conversation_id,
                    session_id,
                    system.to_string(),
                    messages.to_vec(),
                    model_config,
                    start_time,
                ).await;
                Ok(wrapped_stream)
            }
            Err(e) => {
                // Capture failed conversation
                let response_time_ms = start_time.elapsed().as_millis() as f32;
                
                let mut conversation_messages = Vec::new();
                if !system.is_empty() {
                    conversation_messages.push(Message::assistant().with_text(system));
                }
                conversation_messages.extend_from_slice(messages);

                let tokens_used = conversation_messages.iter()
                    .map(|msg| msg.as_concat_text().len() / 4)
                    .sum();

                if let Err(capture_err) = conversation_hook.process_conversation(
                    conversation_id,
                    session_id,
                    conversation_messages,
                    "middleware".to_string(),
                    model_config.model_name.clone(),
                    Some(response_time_ms),
                    Some(tokens_used),
                    true, // error occurred
                ).await {
                    warn!("Failed to capture failed conversation: {}", capture_err);
                }

                Err(e)
            }
        }
    }

    async fn generate_session_name(&self, messages: &Conversation) -> Result<String, crate::providers::errors::ProviderError> {
        self.inner_provider.generate_session_name(messages).await
    }
}

impl ConversationMiddleware {
    async fn wrap_stream(
        &self,
        stream: crate::providers::base::MessageStream,
        conversation_id: String,
        session_id: Option<String>,
        system: String,
        input_messages: Vec<Message>,
        model_config: crate::model::ModelConfig,
        start_time: std::time::Instant,
    ) -> crate::providers::base::MessageStream {
        use futures::StreamExt;
        use async_stream::try_stream;
        
        let conversation_hook = self.conversation_hook.clone();
        
        Box::pin(try_stream! {
            let mut accumulated_response = String::new();
            let mut final_message: Option<Message> = None;
            let mut final_usage: Option<ProviderUsage> = None;

            for await item in stream {
                let (message, usage) = item?;
                
                // Accumulate the response
                accumulated_response = message.as_concat_text();
                final_message = Some(message.clone());
                final_usage = usage.clone();
                
                yield (message, usage);
            }

            // After stream completes, capture the full conversation
            let response_time_ms = start_time.elapsed().as_millis() as f32;
            
            let mut conversation_messages = Vec::new();
            if !system.is_empty() {
                conversation_messages.push(Message::assistant().with_text(system));
            }
            conversation_messages.extend(input_messages);
            
            if let Some(response_msg) = final_message {
                conversation_messages.push(response_msg);
            }

            let tokens_used = if let Some(usage) = final_usage {
                usage.usage.total_tokens.or(
                    usage.usage.input_tokens.and_then(|input| 
                        usage.usage.output_tokens.map(|output| input + output)
                    )
                ).unwrap_or(0) as usize
            } else {
                conversation_messages.iter()
                    .map(|msg| msg.as_concat_text().len() / 4)
                    .sum()
            };

            if let Err(e) = conversation_hook.process_conversation(
                conversation_id,
                session_id,
                conversation_messages,
                "middleware".to_string(),
                model_config.model_name,
                Some(response_time_ms),
                Some(tokens_used),
                false, // no error in successful stream
            ).await {
                warn!("Failed to capture streamed conversation: {}", e);
            }
        })
    }
}

/// Factory for creating conversation middleware
pub struct ConversationMiddlewareFactory {
    conversation_hook: Arc<ConversationHook>,
}

impl ConversationMiddlewareFactory {
    pub fn new(conversation_hook: Arc<ConversationHook>) -> Self {
        Self { conversation_hook }
    }

    /// Wrap a provider with conversation middleware
    pub fn wrap_provider(
        &self,
        provider: Arc<dyn Provider>,
        session_id: Option<String>,
    ) -> ConversationMiddleware {
        ConversationMiddleware::new(provider, self.conversation_hook.clone(), session_id)
    }

    /// Create a conversation hook event receiver
    pub fn create_event_receiver(&self) -> mpsc::UnboundedReceiver<ConversationEvent> {
        let (sender, receiver) = mpsc::unbounded_channel();
        // Note: In practice, you'd want to manage this better to avoid creating multiple hooks
        receiver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelConfig;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_conversation_hook() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let hook = ConversationHook::new(sender);

        assert!(hook.is_enabled().await);

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
        ];

        hook.process_conversation(
            "test_conv".to_string(),
            Some("test_session".to_string()),
            messages.clone(),
            "test_provider".to_string(),
            "test_model".to_string(),
            Some(150.0),
            Some(20),
            false,
        ).await.unwrap();

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.conversation_id, "test_conv");
        assert_eq!(event.session_id, Some("test_session".to_string()));
        assert_eq!(event.messages.len(), 2);
        assert_eq!(event.provider_used, "test_provider");
        assert!(!event.error_occurred);
    }

    #[tokio::test]
    async fn test_conversation_hook_disable() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let hook = ConversationHook::new(sender);

        hook.set_enabled(false).await;
        assert!(!hook.is_enabled().await);

        hook.process_conversation(
            "test_conv".to_string(),
            None,
            vec![],
            "test_provider".to_string(),
            "test_model".to_string(),
            None,
            None,
            false,
        ).await.unwrap();

        // Should not receive any event when disabled
        assert!(receiver.try_recv().is_err());
    }
}
