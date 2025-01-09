use async_trait::async_trait;
use futures::stream::BoxStream;
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::Agent;
use crate::agents::capabilities::Capabilities;
use crate::agents::system::{SystemConfig, SystemResult};
use crate::message::{Message, MessageContent, ToolRequest};
use crate::providers::base::Provider;
use crate::providers::base::ProviderUsage;
use crate::register_agent;
use crate::token_counter::TokenCounter;
use mcp_core::{Content, Resource, Tool, ToolCall};
use serde_json::Value;
// used to sort resources by priority within error margin
const PRIORITY_EPSILON: f32 = 0.001;

/// Default implementation of an Agent
pub struct DefaultAgent {
    capabilities: Mutex<Capabilities>,
    token_counter: TokenCounter,
}

impl DefaultAgent {
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            capabilities: Mutex::new(Capabilities::new(provider)),
            token_counter: TokenCounter::new(),
        }
    }

    /// Setup the next inference by budgeting the context window
    async fn prepare_inference(
        &self,
        system_prompt: &str,
        tools: &[Tool],
        messages: &[Message],
        pending: &[Message],
        target_limit: usize,
        model_name: &str,
        resource_content: &HashMap<String, HashMap<String, (Resource, String)>>,
    ) -> SystemResult<Vec<Message>> {
        // Flatten all resource content into a vector of strings
        let mut resources = Vec::new();
        for system_resources in resource_content.values() {
            for (_, content) in system_resources.values() {
                resources.push(content.clone());
            }
        }

        let approx_count = self.token_counter.count_everything(
            system_prompt,
            messages,
            tools,
            &resources,
            Some(model_name),
        );
        let mut status_content: Vec<String> = Vec::new();

        if approx_count > target_limit {
            println!("[WARNING] Token budget exceeded. Current count: {} \n Difference: {} tokens over buget. Removing context", approx_count, approx_count - target_limit);

            // Get token counts for each resource
            let mut system_token_counts = HashMap::new();

            // Iterate through each system and its resources
            for (system_name, resources) in resource_content {
                let mut resource_counts = HashMap::new();
                for (uri, (_resource, content)) in resources {
                    let token_count =
                        self.token_counter.count_tokens(content, Some(model_name)) as u32;
                    resource_counts.insert(uri.clone(), token_count);
                }
                system_token_counts.insert(system_name.clone(), resource_counts);
            }

            // Sort resources by priority and timestamp and trim to fit context limit
            let mut all_resources: Vec<(String, String, Resource, u32)> = Vec::new();
            for (system_name, resources) in resource_content {
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
            all_resources.sort_by(|a, b| {
                let a_priority = a.2.priority().unwrap_or(0.0);
                let b_priority = b.2.priority().unwrap_or(0.0);
                if (b_priority - a_priority).abs() < PRIORITY_EPSILON {
                    b.2.timestamp().cmp(&a.2.timestamp())
                } else {
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

        // Create a new messages vector with our changes
        let mut new_messages = messages.to_vec();

        // Add pending messages
        for msg in pending {
            new_messages.push(msg.clone());
        }

        // Finally add the status messages, if we have any
        if !status_str.is_empty() {
            let message_use = Message::assistant()
                .with_tool_request("000", Ok(ToolCall::new("status", json!({}))));

            let message_result =
                Message::user().with_tool_response("000", Ok(vec![Content::text(status_str)]));

            new_messages.push(message_use);
            new_messages.push(message_result);
        }

        Ok(new_messages)
    }
}

#[async_trait]
impl Agent for DefaultAgent {
    async fn add_system(&mut self, system: SystemConfig) -> SystemResult<()> {
        let mut capabilities = self.capabilities.lock().await;
        capabilities.add_system(system).await
    }

    async fn remove_system(&mut self, name: &str) {
        let mut capabilities = self.capabilities.lock().await;
        capabilities
            .remove_system(name)
            .await
            .expect("Failed to remove system");
    }

    async fn list_systems(&self) -> Vec<String> {
        let capabilities = self.capabilities.lock().await;
        capabilities
            .list_systems()
            .await
            .expect("Failed to list systems")
    }

    async fn passthrough(&self, _system: &str, _request: Value) -> SystemResult<Value> {
        // TODO implement
        Ok(Value::Null)
    }

    #[instrument(skip(self, messages), fields(user_message))]
    async fn reply(
        &self,
        messages: &[Message],
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message>>> {
        let reply_span = tracing::Span::current();
        let mut capabilities = self.capabilities.lock().await;
        let tools = capabilities.get_prefixed_tools().await?;
        let system_prompt = capabilities.get_system_prompt().await;
        let estimated_limit = capabilities
            .provider()
            .get_model_config()
            .get_estimated_limit();

        // Set the user_message field in the span instead of creating a new event
        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        // Update conversation history for the start of the reply
        let resources = capabilities.get_resources().await?;
        let mut messages = self
            .prepare_inference(
                &system_prompt,
                &tools,
                messages,
                &Vec::new(),
                estimated_limit,
                &capabilities
                    .provider()
                    .get_model_config()
                    .model_name
                    .clone(),
                &resources,
            )
            .await?;

        Ok(Box::pin(async_stream::try_stream! {
            let _reply_guard = reply_span.enter();
            loop {
                // Get completion from provider
                let (response, usage) = capabilities.provider().complete(
                    &system_prompt,
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

                yield message_tool_response.clone();

                // Now we have to remove the previous status tooluse and toolresponse
                // before we add pending messages, then the status msgs back again
                if let Some(message) = messages.last() {
                    if let MessageContent::ToolResponse(result) = &message.content[0] {
                        if result.id == "000" {
                            messages.pop();
                            messages.pop();
                        }
                    }
                }


                let pending = vec![response, message_tool_response];
                messages = self.prepare_inference(&system_prompt, &tools, &messages, &pending, estimated_limit, &capabilities.provider().get_model_config().model_name, &capabilities.get_resources().await?).await?;
            }
        }))
    }

    async fn usage(&self) -> Vec<ProviderUsage> {
        let capabilities = self.capabilities.lock().await;
        capabilities.get_usage().await
    }
}

register_agent!("default", DefaultAgent);
