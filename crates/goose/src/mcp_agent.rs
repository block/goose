use anyhow::Result;
use async_stream;
use futures::stream::BoxStream;
use futures::TryFutureExt;
use rust_decimal_macros::dec;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::errors::{AgentError, AgentResult};
use crate::message::{Message, ToolRequest};
use crate::prompt_template::load_prompt_file;
use crate::providers::base::{Provider, ProviderUsage};
use crate::token_counter::TokenCounter;
use mcp_client::client::McpClient;
use mcp_core::resource::ResourceContents;
use mcp_core::{Content, Resource, Tool, ToolCall};

// used to sort resources by priority within error margin
const PRIORITY_EPSILON: f32 = 0.001;

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

pub struct McpAgent {
    clients: HashMap<String, Arc<Mutex<Box<dyn McpClient + Send>>>>,
    provider: Box<dyn Provider>,
    provider_usage: Mutex<Vec<ProviderUsage>>,
}

impl McpAgent {
    // Create new McpAgent with specified provider
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            clients: HashMap::new(),
            provider,
            provider_usage: Mutex::new(Vec::new()),
        }
    }

    /// Get the context limit from the provider's configuration
    fn get_context_limit(&self) -> usize {
        self.provider.get_model_config().context_limit()
    }

    // Add a named McpClient to the agent
    pub fn add_mcp_client(&mut self, name: String, mcp_client: Box<dyn McpClient + Send>) {
        //TODO: initialize the client here and verify we have connectivity before
        //inserting into the map, probably return an error too
        self.clients.insert(name, Arc::new(Mutex::new(mcp_client)));
    }

    // Get all tools from all servers, and prefix with the configured name
    async fn get_prefixed_tools(&mut self) -> Vec<Tool> {
        let results =
            futures::future::join_all(self.clients.iter_mut().map(|(name, client)| async move {
                let name = name.clone();
                let mut client_guard = client.lock().await;
                match client_guard.list_tools().await {
                    Ok(tools) => (name, Ok(tools)),
                    Err(e) => (name, Err(e)),
                }
            }))
            .await;

        //TODO: do something with _errors
        let (tools, _errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|(_, result)| result.is_ok());

        for e in _errors {
            println!("{}: {:#?}", e.0, e.1);
        }

        tools
            .into_iter()
            .flat_map(|(name, result)| {
                result.unwrap().tools.into_iter().map(move |t| {
                    Tool::new(
                        format!("{}__{}", name, t.name),
                        &t.description,
                        t.input_schema,
                    )
                })
            })
            .collect()
    }

    /// Find and return a reference to the appropriate client for a tool call
    fn get_client_for_tool(
        &self,
        prefixed_name: &str,
    ) -> Option<Arc<Mutex<Box<dyn McpClient + Send>>>> {
        prefixed_name
            .split_once("__")
            .and_then(|(client_name, _)| self.clients.get(client_name))
            .map(Arc::clone)
    }

    async fn dispatch_tool_call(
        &self,
        tool_call: AgentResult<ToolCall>,
    ) -> AgentResult<Vec<Content>> {
        let call = tool_call?;

        let client = self
            .get_client_for_tool(&call.name)
            .ok_or_else(|| AgentError::ToolNotFound(call.name.clone()))?;

        let tool_name = call
            .name
            .split("__")
            .nth(1)
            .ok_or_else(|| AgentError::InvalidToolName(call.name.clone()))?;

        // wrap in AgentError for now
        let mut client_guard = client.lock().await;
        client_guard
            .call_tool(tool_name, call.arguments)
            .map_ok_or_else(
                |err| AgentResult::Err(AgentError::ExecutionError(err.to_string())),
                |result| AgentResult::Ok(result.content),
            )
            .await
    }

    async fn get_server_prompts(&self) -> AgentResult<String> {
        // TODO: implement when McpClient::list/get_prompts exist
        // use each McpClient to get some set of prompts that function as instructions
        Err(AgentError::Internal("not yet implemented".to_string()))
    }

    async fn get_server_resources(
        &mut self,
    ) -> AgentResult<HashMap<String, HashMap<String, (Resource, String)>>> {
        let mut server_resource_content: HashMap<String, HashMap<String, (Resource, String)>> =
            HashMap::new();
        //TODO: handle the errors from the McpClient
        for (name, client) in self.clients.iter_mut() {
            let mut client_guard = client.lock().await;
            let resources = client_guard
                .list_resources()
                .await
                .map_err(|e| AgentError::Internal(e.to_string()))?;

            let mut resource_content: HashMap<String, (Resource, String)> = HashMap::new();

            for resource in resources.resources {
                if let Ok(contents) = client_guard.read_resource(&resource.uri).await {
                    for content in contents.contents {
                        let (uri, content_str) = match &content {
                            ResourceContents::TextResourceContents { uri, text, .. } => (uri, text),
                            ResourceContents::BlobResourceContents { uri, blob, .. } => (uri, blob),
                        };

                        //TODO: must we clone here?
                        resource_content
                            .insert(uri.clone(), (resource.clone(), content_str.to_string()));
                    }
                }
            }
            server_resource_content.insert(name.to_string(), resource_content);
        }
        Ok(server_resource_content)
    }

    /// Setup the next inference by budgeting the context window as well as we can
    async fn prepare_inference(
        &mut self,
        system_prompt: &str,
        tools: &[Tool],
        messages: &[Message],
        pending: &Vec<Message>,
        target_limit: usize,
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
        let resource_content = self.get_server_resources().await?;

        // Flatten all resource content into a vector of strings
        let mut resources = Vec::new();
        for server_resources in resource_content.values() {
            for (_, content) in server_resources.values() {
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

            // Get token counts for each resourcee
            let mut server_token_counts = HashMap::new();

            // Iterate through each system and its resources
            for (server_name, resources) in &resource_content {
                let mut resource_counts = HashMap::new();
                for (uri, (_resource, content)) in resources {
                    let token_count = token_counter
                        .count_tokens(&content, Some(&self.provider.get_model_config().model_name))
                        as u32;
                    resource_counts.insert(uri.clone(), token_count);
                }
                server_token_counts.insert(server_name.clone(), resource_counts);
            }
            // Sort resources by priority and timestamp and trim to fit context limit
            let mut all_resources: Vec<(String, String, Resource, u32)> = Vec::new();
            for (server_name, resources) in &resource_content {
                for (uri, (resource, _)) in resources {
                    if let Some(token_count) = server_token_counts
                        .get(server_name)
                        .and_then(|counts| counts.get(uri))
                    {
                        all_resources.push((
                            server_name.clone(),
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
                if let Some((server_name, uri, _, token_count)) = all_resources.pop() {
                    if let Some(server_counts) = server_token_counts.get_mut(&server_name) {
                        server_counts.remove(&uri);
                        current_tokens -= token_count as usize;
                    }
                }
            }
            // Create status messages only from resources that remain after token trimming
            for (server_name, uri, _, _) in &all_resources {
                if let Some(server_resources) = resource_content.get(server_name) {
                    if let Some((resource, content)) = server_resources.get(uri) {
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
    pub async fn reply(&mut self, messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>> {
        let mut messages = messages.to_vec();
        let tools = self.get_prefixed_tools().await;
        let server_prompt = self.get_server_prompts().await?;
        let estimated_limit = self.provider.get_model_config().get_estimated_limit();

        // Update conversation history for the start of the reply
        messages = self
            .prepare_inference(
                &server_prompt,
                &tools,
                &messages,
                &Vec::new(),
                estimated_limit,
            )
            .await?;

        Ok(Box::pin(async_stream::try_stream! {
            loop {
                // Get completion from provider
                let (response, usage) = self.provider.complete(
                    &server_prompt,
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
                messages = self.prepare_inference(&server_prompt, &tools, &messages, &pending, estimated_limit).await?;
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
    use crate::message::Message;
    use crate::providers::mock::MockProvider;
    use async_trait::async_trait;
    use mcp_client::client::{ClientCapabilities, ClientInfo, Error, McpClientImpl};
    use mcp_core::protocol::Implementation;
    use mcp_core::protocol::{
        InitializeResult, ListResourcesResult, ListToolsResult, ReadResourceResult,
        ServerCapabilities,
    };
    use mcp_core::resource::ResourceContents;
    use serde_json::json;

    // Mock MCP Client for testing
    struct MockMcpClient {
        tools: Vec<Tool>,
        responses: HashMap<String, Vec<Content>>,
    }

    impl MockMcpClient {
        fn new() -> Self {
            Self {
                tools: vec![Tool::new(
                    "test_tool",
                    "A test tool",
                    json!({"type": "object", "properties": {"message": {"type": "string"}}, "required": ["message"]}),
                )],
                responses: HashMap::new(),
            }
        }

        fn with_tool_response(mut self, tool_name: &str, response: Vec<Content>) -> Self {
            self.responses.insert(tool_name.to_string(), response);
            self
        }
    }

    #[async_trait]
    impl McpClient for MockMcpClient {
        async fn initialize(
            &self,
            _info: ClientInfo,
            _capabilities: ClientCapabilities,
        ) -> Result<InitializeResult, Error> {
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    prompts: None,
                    resources: None,
                    tools: None,
                },
                protocol_version: "2.0".to_string(),
                server_info: Implementation {
                    name: "mock".to_string(),
                    version: "2.0".to_string(),
                },
            })
        }

        async fn list_tools(&self) -> Result<ListToolsResult, Error> {
            Ok(ListToolsResult {
                tools: self.tools.clone(),
            })
        }

        async fn list_resources(&self) -> Result<ListResourcesResult, Error> {
            Ok(ListResourcesResult { resources: vec![] })
        }

        async fn read_resource(&self, _uri: &str) -> Result<ReadResourceResult, Error> {
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: "".to_string(),
                    mime_type: None,
                    text: "".to_string(),
                }],
            })
        }

        async fn call_tool(
            &self,
            name: &str,
            _arguments: serde_json::Value,
        ) -> Result<mcp_core::protocol::CallToolResult, Error> {
            Ok(mcp_core::protocol::CallToolResult {
                content: self.responses.get(name).cloned().unwrap_or_default(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_new_mcp_agent() {
        let provider = MockProvider::new(vec![Message::assistant().with_text("test")]);
        let agent = McpAgent::new(Box::new(provider));

        assert!(agent.clients.is_empty());
        assert!(agent.provider_usage.try_lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mcp_agent_add_mcp_client() {
        let provider = MockProvider::new(vec![Message::assistant().with_text("test")]);
        let mut agent = McpAgent::new(Box::new(provider));
        let client = MockMcpClient::new();

        agent.add_mcp_client("test".to_string(), Box::new(client));

        assert_eq!(agent.clients.len(), 1);
        assert!(agent.clients.contains_key("test"));
    }

    #[tokio::test]
    async fn test_mcp_agent_get_prefixed_tools() {
        let provider = MockProvider::new(vec![Message::assistant().with_text("test")]);
        let mut agent = McpAgent::new(Box::new(provider));
        let client = MockMcpClient::new();

        agent.add_mcp_client("test".to_string(), Box::new(client));

        let tools = agent.get_prefixed_tools().await;

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test__test_tool");
        assert_eq!(tools[0].description, "A test tool");
    }

    #[tokio::test]
    async fn test_mcp_agent_get_client_for_tool() {
        let provider = MockProvider::new(vec![Message::assistant().with_text("test")]);
        let mut agent = McpAgent::new(Box::new(provider));
        let client = MockMcpClient::new();

        agent.add_mcp_client("test".to_string(), Box::new(client));

        // Valid tool name
        let client = agent.get_client_for_tool("test__tool_name");
        assert!(client.is_some());

        // Invalid tool name format
        let client = agent.get_client_for_tool("invalid_format");
        assert!(client.is_none());

        // Unknown client
        let client = agent.get_client_for_tool("unknown__tool_name");
        assert!(client.is_none());
    }

    #[tokio::test]
    async fn test_mcp_agent_dispatch_tool_call() {
        let provider = MockProvider::new(vec![Message::assistant().with_text("test")]);
        let mut agent = McpAgent::new(Box::new(provider));

        let response_content = vec![Content::text("test response")];
        let client = MockMcpClient::new().with_tool_response("test_tool", response_content.clone());

        agent.add_mcp_client("test".to_string(), Box::new(client));

        // Test successful tool call
        let tool_call = Ok(ToolCall::new("test__test_tool", json!({"message": "test"})));
        let result = agent.dispatch_tool_call(tool_call).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), response_content);

        // Test invalid tool name format
        let tool_call = Ok(ToolCall::new("invalid_format", json!({})));
        let result = agent.dispatch_tool_call(tool_call).await;
        assert!(matches!(result, Err(AgentError::ToolNotFound(_))));

        // Test unknown client
        let tool_call = Ok(ToolCall::new("unknown__test_tool", json!({})));
        let result = agent.dispatch_tool_call(tool_call).await;
        assert!(matches!(result, Err(AgentError::ToolNotFound(_))));
    }

    use mcp_client::service::{ServiceError, TransportService};
    use mcp_client::transport::SseTransport;
    use std::time::Duration;
    use tower::timeout::TimeoutLayer;
    use tower::{ServiceBuilder, ServiceExt};
    #[tokio::test]
    async fn test_mcp_agent_local_sse() {
        let provider = MockProvider::new(vec![Message::assistant().with_text("test")]);
        let mut agent = McpAgent::new(Box::new(provider));

        let response_content = vec![Content::text("test response")];
        let client = MockMcpClient::new().with_tool_response("test_tool", response_content.clone());

        let transport = SseTransport::new("http://localhost:8000/sse");

        let service = ServiceBuilder::new().service(TransportService::new(transport));

        let client_sse = Box::new(McpClientImpl::new(service));
        let info = ClientInfo {
            name: format!("example-client-{}", 1),
            version: "1.0.0".to_string(),
        };
        let capabilities = ClientCapabilities::default();
        let initialize_result = client_sse.initialize(info, capabilities).await.unwrap();

        println!("{:#?}", initialize_result);

        // Sleep for 100ms to allow the server to start - surprisingly this is required!
        tokio::time::sleep(Duration::from_millis(100)).await;

        agent.add_mcp_client("test".to_string(), Box::new(client));
        agent.add_mcp_client("sse".to_string(), client_sse);

        // Test successful tool call
        let tool_call = Ok(ToolCall::new("test__test_tool", json!({"message": "test"})));
        let result = agent.dispatch_tool_call(tool_call).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), response_content);

        let tools = agent.get_prefixed_tools().await;
        assert_eq!(tools.len(), 5);
        for t in tools {
            println!("{}", t.name)
        }
    }
}
