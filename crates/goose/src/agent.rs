use anyhow::Result;
use async_stream;
use futures::stream::BoxStream;
use rust_decimal_macros::dec;
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::errors::{AgentError, AgentResult};
use crate::message::{Message, ToolRequest};
use crate::prompt_template::load_prompt_file;
use crate::providers::base::{Provider, ProviderUsage};
use crate::systems::System;
use crate::token_counter::TokenCounter;
use mcp_core::{Content, Resource, Tool, ToolCall};
use serde::Serialize;

// used to sort resources by priority within error margin
const PRIORITY_EPSILON: f32 = 0.001;

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
    provider_usage: Mutex<Vec<ProviderUsage>>,
    token_counter: TokenCounter,
}

#[allow(dead_code)]
impl Agent {
    /// Create a new Agent with the specified provider
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            systems: Vec::new(),
            provider,
            provider_usage: Mutex::new(Vec::new()),
            token_counter: TokenCounter::new(),
        }
    }

    /// Add a system to the agent
    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push(system);
    }

    /// Get the context limit from the provider's configuration
    fn get_context_limit(&self) -> usize {
        self.provider.get_model_config().context_limit()
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

    async fn get_systems_resources(
        &self,
    ) -> AgentResult<HashMap<String, HashMap<String, (Resource, String)>>> {
        let mut system_resource_content: HashMap<String, HashMap<String, (Resource, String)>> =
            HashMap::new();
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
        tools: &[Tool],
        messages: &[Message],
        pending: &Vec<Message>,
        target_limit: usize,
        token_counter: &TokenCounter,
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

        let resource_content = self.get_systems_resources().await?;

        // Flatten all resource content into a vector of strings
        let mut resources = Vec::new();
        for system_resources in resource_content.values() {
            for (_, content) in system_resources.values() {
                resources.push(content.clone());
            }
        }

        let approx_count = token_counter.count_everything(
            system_prompt,
            messages,
            tools,
            &resources,
            Some(&self.provider.get_model_config().model_name),
        );
        let mut status_content: Vec<String> = Vec::new();

        if approx_count > target_limit {
            println!("[WARNING] Token budget exceeded. Current count: {} \n Difference: {} tokens over buget. Removing context", approx_count, approx_count - target_limit);

            // Get token counts for each resource
            let mut system_token_counts = HashMap::new();

            // Iterate through each system and its resources
            for (system_name, resources) in &resource_content {
                let mut resource_counts = HashMap::new();
                for (uri, (_resource, content)) in resources {
                    let token_count = token_counter
                        .count_tokens(&content, Some(&self.provider.get_model_config().model_name))
                        as u32;
                    resource_counts.insert(uri.clone(), token_count);
                }
                system_token_counts.insert(system_name.clone(), resource_counts);
            }
            // Sort resources by priority and timestamp and trim to fit context limit
            let mut all_resources: Vec<(String, String, Resource, u32)> = Vec::new();
            for (system_name, resources) in &resource_content {
                for (uri, (resource, _)) in resources {
                    if let Some(token_count) = system_token_counts
                        .get(system_name)
                        .and_then(|counts| counts.get(uri))
                    {
                        all_resources.push((
                            system_name.clone(),
                            uri.clone(),
                            resource.clone(),
                            *token_count,
                        ));
                    }
                }
            }

            // Sort by priority (high to low) and timestamp (newest to oldest)
            // since priority is float, we need to sort by priority within error margin - PRIORITY_EPSILON
            all_resources.sort_by(|a, b| {
                // Compare priorities with epsilon
                // Compare priorities with Option handling - default to 0.0 if None
                let a_priority = a.2.priority().unwrap_or(0.0);
                let b_priority = b.2.priority().unwrap_or(0.0);
                if (b_priority - a_priority).abs() < PRIORITY_EPSILON {
                    // Priorities are "equal" within epsilon, use timestamp as tiebreaker
                    b.2.timestamp().cmp(&a.2.timestamp())
                } else {
                    // Priorities are different enough, use priority ordering
                    b.2.priority()
                        .partial_cmp(&a.2.priority())
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            });

            // Remove resources until we're under target limit
            let mut current_tokens = approx_count;

            while current_tokens > target_limit && !all_resources.is_empty() {
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
        } else {
            // Create status messages from all resources when no trimming needed
            for resources in resource_content.values() {
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
        let estimated_limit = self.provider.get_model_config().get_estimated_limit();

        // Update conversation history for the start of the reply
        messages = self
            .prepare_inference(
                &system_prompt,
                &tools,
                &messages,
                &Vec::new(),
                estimated_limit,
                &self.token_counter,
            )
            .await?;

        Ok(Box::pin(async_stream::try_stream! {
            loop {
                // Get completion from provider
                let (response, usage) = self.provider.complete(
                    &system_prompt,
                    &messages,
                    &tools,
                ).await?;
                self.provider_usage.lock().await.push(usage);

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
                messages = self.prepare_inference(&system_prompt, &tools, &messages, &pending, estimated_limit, &self.token_counter).await?;
            }
        }))
    }

    pub async fn usage(&self) -> Result<Vec<ProviderUsage>> {
        let provider_usage = self.provider_usage.lock().await.clone();

        let mut usage_map: HashMap<String, ProviderUsage> = HashMap::new();
        provider_usage.iter().for_each(|usage| {
            usage_map
                .entry(usage.model.clone())
                .and_modify(|e| {
                    e.usage.input_tokens = Some(
                        e.usage.input_tokens.unwrap_or(0) + usage.usage.input_tokens.unwrap_or(0),
                    );
                    e.usage.output_tokens = Some(
                        e.usage.output_tokens.unwrap_or(0) + usage.usage.output_tokens.unwrap_or(0),
                    );
                    e.usage.total_tokens = Some(
                        e.usage.total_tokens.unwrap_or(0) + usage.usage.total_tokens.unwrap_or(0),
                    );
                    if e.cost.is_none() || usage.cost.is_none() {
                        e.cost = None; // Pricing is not available for all models
                    } else {
                        e.cost = Some(e.cost.unwrap_or(dec!(0)) + usage.cost.unwrap_or(dec!(0)));
                    }
                })
                .or_insert_with(|| usage.clone());
        });
        Ok(usage_map.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use crate::providers::configs::ModelConfig;
    use crate::providers::mock::MockProvider;
    use async_trait::async_trait;
    use chrono::Utc;
    use futures::TryStreamExt;
    use mcp_core::resource::Resource;
    use mcp_core::Annotations;
    use serde_json::json;

    // Mock system for testing
    struct MockSystem {
        name: String,
        tools: Vec<Tool>,
        resources: Vec<Resource>,
        resource_content: HashMap<String, String>,
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
                resources: Vec::new(),
                resource_content: HashMap::new(),
            }
        }

        fn add_resource(&mut self, name: &str, content: &str, priority: f32) {
            let uri = format!("file://{}", name);
            let resource = Resource {
                name: name.to_string(),
                uri: uri.clone(),
                annotations: Some(Annotations::for_resource(priority, Utc::now())),
                description: Some("A mock resource".to_string()),
                mime_type: "text/plain".to_string(),
            };
            self.resources.push(resource);
            self.resource_content.insert(uri, content.to_string());
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
            Ok(self.resources.clone())
        }

        async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
            match tool_call.name.as_str() {
                "echo" => Ok(vec![Content::text(
                    tool_call.arguments["message"].as_str().unwrap_or(""),
                )]),
                _ => Err(AgentError::ToolNotFound(tool_call.name)),
            }
        }

        async fn read_resource(&self, uri: &str) -> AgentResult<String> {
            self.resource_content.get(uri).cloned().ok_or_else(|| {
                AgentError::InvalidParameters(format!("Resource {} could not be found", uri))
            })
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
    async fn test_usage_rollup() -> Result<()> {
        let response = Message::assistant().with_text("Hello!");
        let provider = MockProvider::new(vec![response.clone()]);
        let agent = Agent::new(Box::new(provider));

        let initial_message = Message::user().with_text("Hi");
        let initial_messages = vec![initial_message];

        let mut stream = agent.reply(&initial_messages).await?;
        while stream.try_next().await?.is_some() {}

        // Second message
        let mut stream = agent.reply(&initial_messages).await?;
        while stream.try_next().await?.is_some() {}

        let usage = agent.usage().await?;
        assert_eq!(usage.len(), 1); // 2 messages rolled up to one usage per model
        assert_eq!(usage[0].usage.input_tokens, Some(2));
        assert_eq!(usage[0].usage.output_tokens, Some(2));
        assert_eq!(usage[0].usage.total_tokens, Some(4));
        assert_eq!(usage[0].model, "mock");
        assert_eq!(usage[0].cost, Some(dec!(2)));
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

    #[tokio::test]
    async fn test_prepare_inference_trims_resources_when_budget_exceeded() -> Result<()> {
        // Create a mock provider
        let provider = MockProvider::new(vec![]);
        let mut agent = Agent::new(Box::new(provider));

        // Create a mock system with two resources
        let mut system = MockSystem::new("test");

        // Add two resources with different priorities
        let string_10toks = "hello ".repeat(10);
        system.add_resource("high_priority", &string_10toks, 0.8);
        system.add_resource("low_priority", &string_10toks, 0.1);

        agent.add_system(Box::new(system));

        // Set up test parameters
        // 18 tokens with system + user msg in chat format
        let system_prompt = "This is a system prompt";
        let messages = vec![Message::user().with_text("Hi there")];
        let tools = vec![];
        let pending = vec![];
        let token_counter = TokenCounter::new();
        // Approx count is 40, so target limit of 35 will force trimming
        let target_limit = 35;

        // Call prepare_inference
        let result = agent
            .prepare_inference(
                system_prompt,
                &tools,
                &messages,
                &pending,
                target_limit,
                &token_counter,
            )
            .await?;

        // Get the last message which should be the tool response containing status
        let status_message = result.last().unwrap();
        let status_content = status_message
            .content
            .first()
            .and_then(|content| content.as_tool_response_text())
            .unwrap_or_default();

        // Verify that only the high priority resource is included in the status
        assert!(status_content.contains("high_priority"));
        assert!(!status_content.contains("low_priority"));

        // Now test with a target limit that allows both resources (no trimming)
        let target_limit = 100;
        let token_counter = TokenCounter::new();

        // Call prepare_inference
        let result = agent
            .prepare_inference(
                system_prompt,
                &tools,
                &messages,
                &pending,
                target_limit,
                &token_counter,
            )
            .await?;

        // Get the last message which should be the tool response containing status
        let status_message = result.last().unwrap();
        let status_content = status_message
            .content
            .first()
            .and_then(|content| content.as_tool_response_text())
            .unwrap_or_default();

        // Verify that only the high priority resource is included in the status
        assert!(status_content.contains("high_priority"));
        assert!(status_content.contains("low_priority"));
        Ok(())
    }

    #[tokio::test]
    async fn test_context_trimming_with_custom_model_config() -> Result<()> {
        let provider = MockProvider::with_config(
            vec![],
            ModelConfig::new("test_model".to_string()).with_context_limit(Some(20)),
        );
        let mut agent = Agent::new(Box::new(provider));

        // Create a mock system with a resource that will exceed the context limit
        let mut system = MockSystem::new("test");

        // Add a resource that will exceed our tiny context limit
        let hello_1_tokens = "hello ".repeat(1); // 1 tokens
        let goodbye_10_tokens = "goodbye ".repeat(10); // 10 tokens
        system.add_resource("test_resource_removed", &goodbye_10_tokens, 0.1);
        system.add_resource("test_resource_expected", &hello_1_tokens, 0.5);

        agent.add_system(Box::new(system));

        // Set up test parameters
        // 18 tokens with system + user msg in chat format
        let system_prompt = "This is a system prompt";
        let messages = vec![Message::user().with_text("Hi there")];
        let tools = vec![];
        let pending = vec![];
        let token_counter = TokenCounter::new();
        // Use the context limit from the model config
        let target_limit = agent.get_context_limit();
        assert_eq!(target_limit, 20, "Context limit should be 20");

        // Call prepare_inference
        let result = agent
            .prepare_inference(
                system_prompt,
                &tools,
                &messages,
                &pending,
                target_limit,
                &token_counter,
            )
            .await?;

        // Get the last message which should be the tool response containing status
        let status_message = result.last().unwrap();
        let status_content = status_message
            .content
            .first()
            .and_then(|content| content.as_tool_response_text())
            .unwrap_or_default();

        // verify that "hello" is within the response, should be just under 20 tokens with "hello"
        assert!(status_content.contains("hello"));

        Ok(())
    }
}
