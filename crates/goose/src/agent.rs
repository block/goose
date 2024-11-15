use anyhow::Result;
use async_stream;
use futures::stream::BoxStream;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::errors::{AgentError, AgentResult};
use crate::prompt_template::load_prompt_file;
use crate::providers::base::Provider;
use crate::providers::types::content::{Content, ToolResponse};
use crate::providers::types::message::{Message, Role};
use crate::systems::System;
use crate::tool::{Tool, ToolCall};
use futures::stream;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
struct SystemInfo {
    name: String,
    description: String,
    instructions: String,
}

impl SystemInfo {
    fn new(name: &str, description: &str, instructions: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            instructions: instructions.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct SystemStatus {
    name: String,
    status: String,
}

impl SystemStatus {
    fn new(name: &str, status: String) -> Self {
        Self {
            name: name.to_string(),
            status,
        }
    }
}

/// Agent integrates a foundational LLM with the systems it needs to pilot
pub struct Agent {
    systems: Vec<Box<dyn System>>,
    provider: Box<dyn Provider>,
}

impl Agent {
    /// Create a new Agent with the specified provider
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            systems: Vec::new(),
            provider,
        }
    }

    /// Add a system to the agent
    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push(system);
    }

    /// Get all tools from all systems with proper system prefixing
    fn get_prefixed_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();
        for system in &self.systems {
            for tool in system.tools() {
                tools.push(Tool::new(
                    &format!("{}__{}", system.name(), tool.name),
                    &tool.description,
                    tool.parameters.clone(),
                ));
            }
        }
        tools
    }

    /// Find the appropriate system for a tool call based on the prefixed name
    fn get_system_for_tool(&self, prefixed_name: &str) -> Option<&Box<dyn System>> {
        let parts: Vec<&str> = prefixed_name.split("__").collect();
        if parts.len() != 2 {
            return None;
        }
        let system_name = parts[0];
        self.systems.iter().find(|sys| sys.name() == system_name)
    }

    /// Dispatch a single tool call to the appropriate system
    async fn dispatch_tool_call(&self, tool_call: AgentResult<ToolCall>) -> AgentResult<Value> {
        let call = tool_call?;
        let system = self
            .get_system_for_tool(&call.name)
            .ok_or_else(|| AgentError::ToolNotFound(call.name.clone()))?;

        let tool_name = call
            .name
            .split("__")
            .nth(1)
            .ok_or_else(|| AgentError::InvalidToolName(call.name.clone()))?;
        let system_tool_call = ToolCall::new(tool_name, call.parameters);

        system.call(system_tool_call).await
    }

    fn get_system_prompt(&self) -> AgentResult<String> {
        let mut context = HashMap::new();
        let systems_info: Vec<SystemInfo> = self
            .systems
            .iter()
            .map(|system| {
                SystemInfo::new(system.name(), system.description(), system.instructions())
            })
            .collect();

        context.insert("systems", systems_info);
        load_prompt_file("system.md", &context).map_err(|e| AgentError::Internal(e.to_string()))
    }

    /// Fetches the current status of all systems and formats it as a status message
    async fn get_system_status(&self) -> AgentResult<String> {
        // Get status of all systems
        let status = if !self.systems.is_empty() {
            let mut context = HashMap::new();
            let mut systems_status: Vec<SystemStatus> = Vec::new();
            for system in &self.systems {
                let system_status = system
                    .status()
                    .await
                    .map_err(|e| AgentError::Internal(e.to_string()))?;

                // Format the status into a readable string
                let status_str = serde_json::to_string(&system_status).unwrap_or_default();

                systems_status.push(SystemStatus::new(system.name(), status_str));
            }
            context.insert("systems", systems_status);
            load_prompt_file("status.md", &context)
                .map_err(|e| AgentError::Internal(e.to_string()))?
        } else {
            "No systems loaded".to_string()
        };

        Ok(status)
    }

    // Initialize a new reply round, which may call multiple tools
    // NOTE this is a simple no-op in this implementation
    // NOTE this is a potential home for summarization, checkpointing, planning
    async fn rewrite_messages_on_reply(
        &self,
        messages: &mut Vec<Message>,
        status: String,
    ) -> AgentResult<()> {
        // Create tool use message for status check
        let message_use = Message::new(
            Role::Assistant,
            vec![Content::tool_request_success("000", "status", json!({}))],
        )
        .map_err(|e| AgentError::Internal(e.to_string()))?;

        // Create tool result message with status
        let message_result = Message::new(
            Role::User,
            vec![Content::tool_response_success(
                "000",
                serde_json::json!(status),
            )],
        )
        .map_err(|e| AgentError::Internal(e.to_string()))?;

        messages.push(message_use);
        messages.push(message_result);
        Ok(())
    }

    // Rewrite the exchange as needed after each tool call
    // NOTE in this implementation, we do system by system updates other agents might use a very different approach
    // we from a message list that always looks like:
    // [kickoff, tool_use_0, tool_result_0, ..., tool_use_n, tool_result_n, status_use, status_result]
    // where status contains system detail that we always want to include for the agent
    async fn rewrite_messages_on_tool_response(
        &self,
        messages: &mut Vec<Message>,
        pending: Vec<Message>,
    ) -> AgentResult<()> {
        // Remove the last two messages (status and tool response)
        messages.pop();
        messages.pop();

        // Append the pending messages
        for message in pending {
            messages.push(message);
        }

        Ok(())
    }

    /// Create a stream that yields each message as it's generated by the agent.
    /// This includes both the assistant's responses and any tool responses.
    pub fn reply(&self, messages: &[Message]) -> BoxStream<'_, Result<Message>> {
        let mut messages = messages.to_vec();
        let tools = self.get_prefixed_tools();
        let system_prompt = match self.get_system_prompt() {
            Ok(prompt) => prompt,
            Err(e) => return Box::pin(stream::once(async move { Err(e.into()) })),
        };

        Box::pin(async_stream::try_stream! {
            loop {
                // Append the status tool request and response messages
                let status = self.get_system_status().await.map_err(|e| AgentError::Internal(e.to_string()))?;
                self.rewrite_messages_on_reply(&mut messages, status).await?;

                // Get completion from provider
                let (response, _) = self.provider.complete(
                    &system_prompt,
                    &messages,
                    &tools,
                ).await?;

                // The assistant's response is added in rewrite_messages_on_tool_response
                // Yield the assistant's response
                yield response.clone();

                // If there are tool calls, handle them in phases
                if response.has_tool_request() {
                    // Phase 1: Collect and process all tool requests
                    let mut tool_requests = Vec::new();
                    for tool_request in response.tool_request() {
                        tool_requests.push(tool_request);
                    }

                    // Phase 2: Dispatch tool calls and collect responses
                    let mut content = Vec::new();
                    for tool_request in tool_requests {
                        // Add a small delay before processing each tool call to ensure proper sequencing
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        
                        let output = self.dispatch_tool_call(tool_request.call).await;
                        content.push(Content::ToolResponse(ToolResponse {
                            request_id: tool_request.id, 
                            output
                        }));
                    }

                    // Phase 3: Create and yield the tool response message
                    let tool_response = Message::new(Role::User, content)
                        .expect("Failed to create tool response message");
                    yield tool_response.clone();

                    // Update conversation history
                    self.rewrite_messages_on_tool_response(&mut messages, vec![response.clone(), tool_response.clone()]).await?;
                } else {
                    // No more tool calls, end the conversation
                    break;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::mock::MockProvider;
    use crate::providers::types::content::Content;
    use async_trait::async_trait;
    use futures::StreamExt;
    use serde_json::json;
    use std::collections::HashMap;

    // Mock system for testing
    struct MockSystem {
        name: String,
        tools: Vec<Tool>,
    }

    impl MockSystem {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                tools: vec![Tool::new(
                    "echo",
                    "Echoes back the input",
                    json!({"type": "object", "properties": {"message": {"type": "string"}}, "required": ["message"]}),
                )],
            }
        }
    }

    #[async_trait]
    impl System for MockSystem {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A mock system for testing"
        }

        fn instructions(&self) -> &str {
            "Mock system instructions"
        }

        fn tools(&self) -> &[Tool] {
            &self.tools
        }

        async fn status(&self) -> anyhow::Result<HashMap<String, Value>> {
            Ok(HashMap::new())
        }

        async fn call(&self, tool_call: ToolCall) -> AgentResult<Value> {
            match tool_call.name.as_str() {
                "echo" => Ok(tool_call.parameters),
                _ => Err(AgentError::ToolNotFound(tool_call.name)),
            }
        }
    }

    #[tokio::test]
    async fn test_simple_response() -> Result<()> {
        let response = Message::new(Role::Assistant, vec![Content::text("Hello!")])?;
        let provider = MockProvider::new(vec![response.clone()]);
        let agent = Agent::new(Box::new(provider));

        let initial_message = Message::new(Role::User, vec![Content::text("Hi")])?;
        let initial_messages = vec![initial_message];
        let messages: Vec<Message> = agent
            .reply(&initial_messages)
            .collect::<Vec<Result<Message>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<Message>>>()?;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], response);
        Ok(())
    }

    #[tokio::test]
    async fn test_tool_call() -> Result<()> {
        let mut agent = Agent::new(Box::new(MockProvider::new(vec![
            Message::new(
                Role::Assistant,
                vec![Content::tool_request(
                    "test__echo",
                    json!({"message": "test"}),
                )],
            )?,
            Message::new(Role::Assistant, vec![Content::text("Done!")])?,
        ])));

        agent.add_system(Box::new(MockSystem::new("test")));

        let initial_message = Message::new(Role::User, vec![Content::text("Echo test")])?;
        let initial_messages = vec![initial_message];
        let messages: Vec<Message> = agent
            .reply(&initial_messages)
            .collect::<Vec<Result<Message>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<Message>>>()?;

        // Should have three messages: tool request, response, and model text
        assert_eq!(messages.len(), 3);
        assert!(messages[0].has_tool_request());
        assert_eq!(messages[2].content[0], Content::text("Done!"));
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_tool() -> Result<()> {
        let mut agent = Agent::new(Box::new(MockProvider::new(vec![
            Message::new(
                Role::Assistant,
                vec![Content::tool_request("invalid.tool", json!({}))],
            )?,
            Message::new(Role::Assistant, vec![Content::text("Error occurred")])?,
        ])));

        agent.add_system(Box::new(MockSystem::new("test")));

        let initial_message = Message::new(Role::User, vec![Content::text("Invalid tool")])?;
        let initial_messages = vec![initial_message];
        let messages: Vec<Message> = agent
            .reply(&initial_messages)
            .collect::<Vec<Result<Message>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<Message>>>()?;

        // Should have three messages: failed tool request, fail response, and model text
        assert_eq!(messages.len(), 3);
        assert!(messages[0].has_tool_request());
        assert_eq!(messages[2].content[0], Content::text("Error occurred"));
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_tool_calls() -> Result<()> {
        let mut agent = Agent::new(Box::new(MockProvider::new(vec![
            Message::new(
                Role::Assistant,
                vec![
                    Content::tool_request("test__echo", json!({"message": "first"})),
                    Content::tool_request("test__echo", json!({"message": "second"})),
                ],
            )?,
            Message::new(Role::Assistant, vec![Content::text("All done!")])?,
        ])));

        agent.add_system(Box::new(MockSystem::new("test")));

        let initial_message = Message::new(Role::User, vec![Content::text("Multiple calls")])?;
        let initial_messages = vec![initial_message];
        let messages: Vec<Message> = agent
            .reply(&initial_messages)
            .collect::<Vec<Result<Message>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<Message>>>()?;

        // Should have three messages: tool requests, response, and model text
        assert_eq!(messages.len(), 3);
        assert!(messages[0].has_tool_request());
        assert_eq!(messages[2].content[0], Content::text("All done!"));
        Ok(())
    }

    #[tokio::test]
    async fn test_conversation_flow() -> Result<()> {
        let mut agent = Agent::new(Box::new(MockProvider::new(vec![
            // First interaction: tool request
            Message::new(
                Role::Assistant,
                vec![Content::tool_request(
                    "test__echo",
                    json!({"message": "first"}),
                )],
            )?,
            // Second interaction: final response
            Message::new(Role::Assistant, vec![Content::text("Done!")])?,
        ])));

        agent.add_system(Box::new(MockSystem::new("test")));

        let initial_message = Message::new(Role::User, vec![Content::text("Test conversation")])?;
        let initial_messages = vec![initial_message];
        let messages: Vec<Message> = agent
            .reply(&initial_messages)
            .collect::<Vec<Result<Message>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<Message>>>()?;

        assert_eq!(messages.len(), 3);

        // First message should be the tool request
        assert!(messages[0].has_tool_request());
        let tool_requests: Vec<_> = messages[0].tool_request().into_iter().collect();
        assert_eq!(tool_requests[0].call.as_ref().unwrap().name, "test__echo");

        // Second message should be the tool response
        assert_eq!(
            messages[1].content[0],
            Content::tool_response_success(
                tool_requests[0].id.clone(),
                json!({"message": "first"})
            )
        );

        // Third message should be the final response
        assert_eq!(messages[2].content[0], Content::text("Done!"));

        Ok(())
    }
}
