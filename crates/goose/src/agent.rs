use anyhow::Result;
use async_stream;
use futures::stream::BoxStream;
use serde_json::json;
use std::collections::HashMap;

use crate::errors::{AgentError, AgentResult};
use crate::models::content::Content;
use crate::models::message::{Message, ToolRequest};
use crate::models::tool::{Tool, ToolCall};
use crate::prompt_template::load_prompt_file;
use crate::providers::base::Provider;
use crate::systems::{System, Resource};
use crate::token_counter::TokenCounter;
use serde::Serialize;

const CONTEXT_LIMIT: usize = 200_000; // TODO: update back to 200_000; // model's context limit
const ESTIMATE_FACTOR: f32 = 0.8;
const ESTIMATED_TOKEN_LIMIT: usize = (CONTEXT_LIMIT as f32 * ESTIMATE_FACTOR) as usize;

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

#[allow(dead_code)]
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
                    format!("{}__{}", system.name(), tool.name),
                    &tool.description,
                    tool.input_schema.clone(),
                ));
            }
        }
        tools
    }

    /// Find the appropriate system for a tool call based on the prefixed name
    fn get_system_for_tool(&self, prefixed_name: &str) -> Option<&dyn System> {
        let parts: Vec<&str> = prefixed_name.split("__").collect();
        if parts.len() != 2 {
            return None;
        }
        let system_name = parts[0];
        self.systems
            .iter()
            .find(|sys| sys.name() == system_name)
            .map(|v| &**v)
    }

    /// Dispatch a single tool call to the appropriate system
    async fn dispatch_tool_call(
        &self,
        tool_call: AgentResult<ToolCall>,
    ) -> AgentResult<Vec<Content>> {
        let call = tool_call?;
        let system = self
            .get_system_for_tool(&call.name)
            .ok_or_else(|| AgentError::ToolNotFound(call.name.clone()))?;

        let tool_name = call
            .name
            .split("__")
            .nth(1)
            .ok_or_else(|| AgentError::InvalidToolName(call.name.clone()))?;
        let system_tool_call = ToolCall::new(tool_name, call.arguments);

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

    async fn get_systems_resources(&self) -> AgentResult<HashMap<String, HashMap<String, (Resource,String)>>> {
        let mut system_resource_content: HashMap<String, HashMap<String, (Resource, String)>> = HashMap::new();
        for system in &self.systems {
            let system_status = system
                .status()
                .await
                .map_err(|e| AgentError::Internal(e.to_string()))?;

            let mut resource_content: HashMap<String, (Resource, String)> = HashMap::new();
            for resource in system_status {
                if let Ok(content) = system.read_resource(&resource.uri).await {
                    resource_content.insert(resource.uri.to_string(), (resource, content));
                }
            }
            system_resource_content.insert(system.name().to_string(), resource_content);
        }
        Ok(system_resource_content)
    }


    /// Setup the next inference by budgeting the context window as well as we can
    async fn prepare_inference(
            &self,
            system_prompt: &str,
            tools: &Vec<Tool>,
            messages: &Vec<Message>,
            pending: &Vec<Message>,
        ) -> AgentResult<Vec<Message>> {
        // Prepares the inference by managing context window and token budget.
        // This function:
        // 1. Retrieves and formats system resources and status
        // 2. Trims content if total tokens exceed the model's context limit
        // 3. Adds pending messages if any. Pending messages are messages that have been added
        //    to the conversation but not yet responded to.
        // 4. Adds two messages to the conversation:
        //    - A tool request message for status
        //    - A tool response message containing the (potentially trimmed) status
        //
        // Returns the updated message history with status information appended.
        //
        // Arguments:
        // * `system_prompt` - The system prompt to include
        // * `tools` - Available tools for the agent
        // * `messages` - Current conversation history
        //
        // Returns:
        // * `AgentResult<Vec<Message>>` - Updated message history with status appended

        let token_counter = TokenCounter::new();
        let resource_content = self.get_systems_resources().await?;

        // Flatten all resource content into a vector of strings
        let mut resources = Vec::new();
        for (_, system_resources) in &resource_content {
            for (_, (_, content)) in system_resources {
                resources.push(content.clone());
            }
        }

        let approx_count = token_counter.count_everything(
            &system_prompt,
            &messages,
            &tools,
            &resources,
            Some("gpt-4"),
        );

        let mut status_content: Vec<String> = Vec::new();

        if approx_count > ESTIMATED_TOKEN_LIMIT {
            println!("[WARNING]Token budget exceeded. Current count: {} \n Difference: {} tokens over buget. Removing context", approx_count, approx_count - ESTIMATED_TOKEN_LIMIT);

            // Get token counts for each resourcee
            let mut system_token_counts = HashMap::new();

            // Iterate through each system and its resources
            for (system_name, resources) in &resource_content {
                let mut resource_counts = HashMap::new();
                for (uri, (_resource, content)) in resources {
                    let token_count = token_counter.count_tokens(&content, Some("gpt-4")) as u32;
                    resource_counts.insert(uri.clone(), token_count);
                }
                system_token_counts.insert(system_name.clone(), resource_counts);
            }
            // Sort resources by priority and timestamp and trim to fit context limit
            let mut all_resources: Vec<(String, String, Resource, u32)> = Vec::new();
            for (system_name, resources) in &resource_content {
                for (uri, (resource, _)) in resources {
                    if let Some(token_count) = system_token_counts.get(system_name).and_then(|counts| counts.get(uri)) {
                        all_resources.push((
                            system_name.clone(),
                            uri.clone(),
                            resource.clone(),
                            *token_count
                        ));
                    }
                }
            }

            // Sort by priority (high to low) and timestamp (newest to oldest)
            all_resources.sort_by(|a, b| {
                let priority_cmp = b.2.priority.cmp(&a.2.priority);
                if priority_cmp == std::cmp::Ordering::Equal {
                    b.2.timestamp.cmp(&a.2.timestamp)
                } else {
                    priority_cmp
                }
            });

            // Remove resources until we're under target limit
            let mut current_tokens = approx_count;

            while current_tokens > ESTIMATED_TOKEN_LIMIT && !all_resources.is_empty() {
                if let Some((system_name, uri, _, token_count)) = all_resources.pop() {
                    if let Some(system_counts) = system_token_counts.get_mut(&system_name) {
                        system_counts.remove(&uri);
                        current_tokens -= token_count as usize;
                    }
                }
            }
            // Create status messages only from resources that remain after token trimming
            for (system_name, uri, _, _) in &all_resources {
                if let Some(system_resources) = resource_content.get(system_name) {
                    if let Some((resource, content)) = system_resources.get(uri) {
                        status_content.push(format!("{}\n```\n{}\n```\n", resource.name, content));
                    }
            }
        }
        }
        else {
            // Create status messages from all resources when no trimming needed
            for (_system_name, resources) in &resource_content {
                for (resource, content) in resources.values() {
                    status_content.push(format!("{}\n```\n{}\n```\n", resource.name, content));
                }
            }
        }

        // Join remaining status content and create status message
        let status_str = status_content.join("\n");
        let mut context = HashMap::new();
        let systems_status = vec![SystemStatus::new("system", status_str)];
        context.insert("systems", &systems_status);

        // Load and format the status template with only remaining resources
        let status = load_prompt_file("status.md", &context)
            .map_err(|e| AgentError::Internal(e.to_string()))?;


        // Create a new messages vector with our changes
        let mut new_messages = messages.to_vec();

        // Add pending messages
        for msg in pending {
            new_messages.push(msg.clone());
        }

        // Finally add the status messages
        let message_use =
            Message::assistant().with_tool_request("000", Ok(ToolCall::new("status", json!({}))));

        let message_result =
            Message::user().with_tool_response("000", Ok(vec![Content::text(status)]));


        new_messages.push(message_use);
        new_messages.push(message_result);

        Ok(new_messages)
    }

    /// Create a stream that yields each message as it's generated by the agent.
    /// This includes both the assistant's responses and any tool responses.
    pub async fn reply(&self, messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>> {
        let mut messages = messages.to_vec();
        let tools = self.get_prefixed_tools();
        let system_prompt = self.get_system_prompt()?;


        // Update conversation history for the start of the reply
        messages =self.prepare_inference(&system_prompt, &tools, &messages, &Vec::new()).await?;

        Ok(Box::pin(async_stream::try_stream! {
            loop {

                // Get completion from provider
                let (response, _) = self.provider.complete(
                    &system_prompt,
                    &messages,
                    &tools,
                ).await?;

                // The assistant's response is added in rewrite_messages_on_tool_response
                // Yield the assistant's response
                yield response.clone();

                // Not sure why this is needed, but this ensures that the above message is yielded
                // before the following potentially long-running commands start processing
                tokio::task::yield_now().await;

                // First collect any tool requests
                let tool_requests: Vec<&ToolRequest> = response.content
                    .iter()
                    .filter_map(|content| content.as_tool_request())
                    .collect();

                if tool_requests.is_empty() {
                    // No more tool calls, end the reply loop
                    break;
                }

                // Then dispatch each in parallel
                let futures: Vec<_> = tool_requests
                    .iter()
                    .map(|request| self.dispatch_tool_call(request.tool_call.clone()))
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

                yield message_tool_response.clone();

                // Now we have to remove the previous status tooluse and toolresponse
                // before we add pending messages, then the status msgs back again
                messages.pop();
                messages.pop();

                let pending = vec![response, message_tool_response];
                messages = self.prepare_inference(&system_prompt, &tools, &messages, &pending).await?;
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::MessageContent;
    use crate::providers::mock::MockProvider;
    use crate::systems::Resource;
    use async_trait::async_trait;
    use futures::TryStreamExt;
    use serde_json::json;
    use chrono::{Utc, Duration};

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

        async fn status(&self) -> anyhow::Result<Vec<Resource>> {
            Ok(Vec::new())
        }

        async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
            match tool_call.name.as_str() {
                "echo" => Ok(vec![Content::text(
                    tool_call.arguments["message"].as_str().unwrap_or(""),
                )]),
                _ => Err(AgentError::ToolNotFound(tool_call.name)),
            }
        }

        async fn read_resource(&self, _uri: &str) -> AgentResult<String> {
            Ok("".to_string())
        }
    }

    #[tokio::test]
    async fn test_simple_response() -> Result<()> {
        let response = Message::assistant().with_text("Hello!");
        let provider = MockProvider::new(vec![response.clone()]);
        let agent = Agent::new(Box::new(provider));

        let initial_message = Message::user().with_text("Hi");
        let initial_messages = vec![initial_message];

        let mut stream = agent.reply(&initial_messages).await?;
        let mut messages = Vec::new();
        while let Some(msg) = stream.try_next().await? {
            messages.push(msg);
        }

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], response);
        Ok(())
    }

    #[tokio::test]
    async fn test_tool_call() -> Result<()> {
        let mut agent = Agent::new(Box::new(MockProvider::new(vec![
            Message::assistant().with_tool_request(
                "1",
                Ok(ToolCall::new("test_echo", json!({"message": "test"}))),
            ),
            Message::assistant().with_text("Done!"),
        ])));

        agent.add_system(Box::new(MockSystem::new("test")));

        let initial_message = Message::user().with_text("Echo test");
        let initial_messages = vec![initial_message];

        let mut stream = agent.reply(&initial_messages).await?;
        let mut messages = Vec::new();
        while let Some(msg) = stream.try_next().await? {
            messages.push(msg);
        }

        // Should have three messages: tool request, response, and model text
        assert_eq!(messages.len(), 3);
        assert!(messages[0]
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_))));
        assert_eq!(messages[2].content[0], MessageContent::text("Done!"));
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_tool() -> Result<()> {
        let mut agent = Agent::new(Box::new(MockProvider::new(vec![
            Message::assistant()
                .with_tool_request("1", Ok(ToolCall::new("invalid_tool", json!({})))),
            Message::assistant().with_text("Error occurred"),
        ])));

        agent.add_system(Box::new(MockSystem::new("test")));

        let initial_message = Message::user().with_text("Invalid tool");
        let initial_messages = vec![initial_message];

        let mut stream = agent.reply(&initial_messages).await?;
        let mut messages = Vec::new();
        while let Some(msg) = stream.try_next().await? {
            messages.push(msg);
        }

        // Should have three messages: failed tool request, fail response, and model text
        assert_eq!(messages.len(), 3);
        assert!(messages[0]
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_))));
        assert_eq!(
            messages[2].content[0],
            MessageContent::text("Error occurred")
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_tool_calls() -> Result<()> {
        let mut agent = Agent::new(Box::new(MockProvider::new(vec![
            Message::assistant()
                .with_tool_request(
                    "1",
                    Ok(ToolCall::new("test_echo", json!({"message": "first"}))),
                )
                .with_tool_request(
                    "2",
                    Ok(ToolCall::new("test_echo", json!({"message": "second"}))),
                ),
            Message::assistant().with_text("All done!"),
        ])));

        agent.add_system(Box::new(MockSystem::new("test")));

        let initial_message = Message::user().with_text("Multiple calls");
        let initial_messages = vec![initial_message];

        let mut stream = agent.reply(&initial_messages).await?;
        let mut messages = Vec::new();
        while let Some(msg) = stream.try_next().await? {
            messages.push(msg);
        }

        // Should have three messages: tool requests, responses, and model text
        assert_eq!(messages.len(), 3);
        assert!(messages[0]
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_))));
        assert_eq!(messages[2].content[0], MessageContent::text("All done!"));
        Ok(())
    }
}
