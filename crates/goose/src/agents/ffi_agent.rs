use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;

use crate::agents::extension::ExtensionConfig;
use crate::agents::types::SessionConfig;
use crate::message::Message;
use crate::permission::permission_confirmation::PrincipalType;
use crate::providers::base::Provider;
use mcp_core::{Content, ToolResult};

use super::agent::Agent;

/// FFI-friendly reply state that can be used across FFI boundaries
pub struct ReplyState {
    /// The current state of the reply process
    pub state: ReplyProcessState,
    /// The current message being processed
    pub current_message: Option<Message>,
    /// Any pending tool requests that need approval
    pub pending_tool_requests: Vec<PendingToolRequest>,
    /// The conversation history
    pub messages: Vec<Message>,
    /// Session configuration
    pub session: Option<SessionConfig>,
    /// Internal agent reference
    agent: Arc<Agent>,
}

#[derive(Debug, Clone)]
pub enum ReplyProcessState {
    /// Initial state, ready to start processing
    Ready,
    /// Waiting for provider response
    WaitingForProvider,
    /// Yielded a message, waiting for next action
    MessageYielded,
    /// Waiting for tool approval
    WaitingForToolApproval,
    /// Processing tool results
    ProcessingTools,
    /// Reply process completed
    Completed,
    /// Error occurred
    Error(String),
}

#[derive(Debug, Clone)]
pub struct PendingToolRequest {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
    pub requires_approval: bool,
}

impl ReplyState {
    pub fn new(agent: Arc<Agent>, messages: Vec<Message>, session: Option<SessionConfig>) -> Self {
        ReplyState {
            state: ReplyProcessState::Ready,
            current_message: None,
            pending_tool_requests: Vec::new(),
            messages,
            session,
            agent,
        }
    }

    /// Start the reply process and get the first message
    pub async fn start(&mut self) -> Result<()> {
        if !matches!(self.state, ReplyProcessState::Ready) {
            return Err(anyhow::anyhow!("Reply process already started"));
        }

        self.state = ReplyProcessState::WaitingForProvider;
        self.advance().await
    }

    /// Advance the reply process to the next state
    pub async fn advance(&mut self) -> Result<()> {
        // Create a new stream each time we need to advance
        let messages = self.messages.clone();
        let session = self.session.clone();
        let agent = Arc::clone(&self.agent);

        let mut stream = agent.reply(&messages, session).await?;

        match stream.next().await {
            Some(Ok(message)) => {
                self.process_message(message).await?;
            }
            Some(Err(e)) => {
                self.state = ReplyProcessState::Error(e.to_string());
            }
            None => {
                self.state = ReplyProcessState::Completed;
            }
        }

        Ok(())
    }

    async fn process_message(&mut self, message: Message) -> Result<()> {
        // Check if this message contains tool requests that need approval
        let tool_requests = self.extract_tool_requests(&message);

        if !tool_requests.is_empty() {
            self.pending_tool_requests = tool_requests;
            self.state = ReplyProcessState::WaitingForToolApproval;
        } else {
            self.current_message = Some(message);
            self.state = ReplyProcessState::MessageYielded;
        }

        Ok(())
    }

    fn extract_tool_requests(&self, message: &Message) -> Vec<PendingToolRequest> {
        // Extract tool requests from the message
        let mut requests = Vec::new();

        for content in &message.content {
            if let Some(tool_request) = content.as_tool_request() {
                if let Ok(tool_call) = &tool_request.tool_call {
                    requests.push(PendingToolRequest {
                        id: tool_request.id.clone(),
                        name: tool_call.name.clone(),
                        arguments: tool_call
                            .arguments
                            .as_object()
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default(),
                        requires_approval: self.agent.is_frontend_tool(&tool_call.name),
                    });
                }
            }
        }

        requests
    }

    /// Approve a pending tool request
    pub async fn approve_tool(&mut self, request_id: &str) -> Result<()> {
        if !matches!(self.state, ReplyProcessState::WaitingForToolApproval) {
            return Err(anyhow::anyhow!("Not waiting for tool approval"));
        }

        // Find and remove the request
        if let Some(index) = self
            .pending_tool_requests
            .iter()
            .position(|r| r.id == request_id)
        {
            let request = self.pending_tool_requests.remove(index);

            // Handle the approval
            self.agent
                .handle_confirmation(
                    request.id.clone(),
                    crate::permission::PermissionConfirmation {
                        principal_type: PrincipalType::Tool,
                        permission: crate::permission::Permission::AllowOnce,
                    },
                )
                .await;
        }

        // If no more pending requests, continue processing
        if self.pending_tool_requests.is_empty() {
            self.state = ReplyProcessState::ProcessingTools;
        }

        Ok(())
    }

    /// Deny a pending tool request
    pub async fn deny_tool(&mut self, request_id: &str) -> Result<()> {
        if !matches!(self.state, ReplyProcessState::WaitingForToolApproval) {
            return Err(anyhow::anyhow!("Not waiting for tool approval"));
        }

        // Find and remove the request
        if let Some(index) = self
            .pending_tool_requests
            .iter()
            .position(|r| r.id == request_id)
        {
            let request = self.pending_tool_requests.remove(index);

            // Handle the denial
            self.agent
                .handle_confirmation(
                    request.id.clone(),
                    crate::permission::PermissionConfirmation {
                        principal_type: PrincipalType::Tool,
                        permission: crate::permission::Permission::DenyOnce,
                    },
                )
                .await;
        }

        // If no more pending requests, continue processing
        if self.pending_tool_requests.is_empty() {
            self.state = ReplyProcessState::ProcessingTools;
        }

        Ok(())
    }

    /// Get the current state
    pub fn get_state(&self) -> &ReplyProcessState {
        &self.state
    }

    /// Get the current message if available
    pub fn get_current_message(&self) -> Option<&Message> {
        self.current_message.as_ref()
    }

    /// Get pending tool requests
    pub fn get_pending_tool_requests(&self) -> &[PendingToolRequest] {
        &self.pending_tool_requests
    }
}

/// FFI-friendly agent wrapper that provides synchronous-style methods
pub struct FFIAgent {
    agent: Arc<Agent>,
}

impl FFIAgent {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        FFIAgent {
            agent: Arc::new(Agent::new(provider)),
        }
    }

    /// Create a new reply state for processing a conversation
    pub fn create_reply_state(
        &self,
        messages: Vec<Message>,
        session: Option<SessionConfig>,
    ) -> ReplyState {
        ReplyState::new(Arc::clone(&self.agent), messages, session)
    }

    /// Add an extension to the agent
    pub async fn add_extension(&self, _extension: ExtensionConfig) -> Result<()> {
        // We need to clone the Arc to get a mutable reference
        // Since Agent doesn't have a mutable add_extension method,
        // we'll need to use a different approach
        // For now, we'll return an error indicating this needs to be implemented
        Err(anyhow::anyhow!(
            "Add extension not implemented for FFIAgent"
        ))
    }

    /// List available tools
    pub async fn list_tools(&self, extension_name: Option<String>) -> Vec<mcp_core::tool::Tool> {
        self.agent.list_tools(extension_name).await
    }

    /// Handle a tool result
    pub async fn handle_tool_result(&self, id: String, result: ToolResult<Vec<Content>>) {
        self.agent.handle_tool_result(id, result).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::tool::ToolCall;
    use serde_json::json;
    use crate::providers::base::{Provider, ProviderUsage, ProviderMetadata, Usage};
    use crate::model::ModelConfig;
    use crate::providers::errors::ProviderError;
    use async_trait::async_trait;
    
    // Simple mock provider for testing
    struct MockProvider;
    
    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata
        where
            Self: Sized {
            ProviderMetadata::empty()
        }
        
        fn get_model_config(&self) -> ModelConfig {
            ModelConfig::new("mock-model".to_string())
        }
        
        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[mcp_core::Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock-model".to_string(), Usage::default())
            ))
        }
    }

    #[tokio::test]
    async fn test_reply_state_basic_flow() {
        // Create a mock provider
        let provider = Arc::new(MockProvider);
        let agent = Arc::new(Agent::new(provider));
        
        // Create a simple message
        let messages = vec![Message::user().with_text("Hello")];
        
        // Create reply state
        let mut reply_state = ReplyState::new(agent, messages, None);
        
        // Test initial state
        assert!(matches!(reply_state.state, ReplyProcessState::Ready));
        
        // Start the reply process
        let result = reply_state.start().await;
        assert!(result.is_ok());
        
        // Check that state has changed
        assert!(!matches!(reply_state.state, ReplyProcessState::Ready));
    }

    #[tokio::test]
    async fn test_tool_request_extraction() {
        let provider = Arc::new(MockProvider);
        let agent = Arc::new(Agent::new(provider));
        
        // Create a message with a tool request
        let tool_call = ToolCall::new("test_tool", json!({"param": "value"}));
        let message = Message::assistant()
            .with_tool_request("tool123", Ok(tool_call));
        
        let reply_state = ReplyState::new(agent, vec![], None);
        let tool_requests = reply_state.extract_tool_requests(&message);
        
        assert_eq!(tool_requests.len(), 1);
        assert_eq!(tool_requests[0].id, "tool123");
        assert_eq!(tool_requests[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_tool_approval_flow() {
        let provider = Arc::new(MockProvider);
        let agent = Arc::new(Agent::new(provider));
        
        let mut reply_state = ReplyState::new(agent, vec![], None);
        
        // Add a pending tool request
        reply_state.pending_tool_requests.push(PendingToolRequest {
            id: "tool123".to_string(),
            name: "test_tool".to_string(),
            arguments: HashMap::new(),
            requires_approval: true,
        });
        
        // Set state to waiting for approval
        reply_state.state = ReplyProcessState::WaitingForToolApproval;
        
        // Approve the tool
        let result = reply_state.approve_tool("tool123").await;
        assert!(result.is_ok());
        
        // Check that pending requests are cleared
        assert!(reply_state.pending_tool_requests.is_empty());
        assert!(matches!(reply_state.state, ReplyProcessState::ProcessingTools));
    }
}
