use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;

use crate::message::{Message, MessageContent, ToolRequest};
use crate::providers::base::{Provider, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::providers::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tool_json, OllamaInterpreter,
};
use crate::session;
use mcp_core::tool::Tool;

use super::super::agents::Agent;

impl Agent {
    /// Prepares tools and system prompt for a provider request
    pub(crate) async fn prepare_tools_and_prompt(
        &self,
    ) -> anyhow::Result<(Vec<Tool>, Vec<Tool>, String)> {
        // Get tools from extension manager
        let mut tools = self.list_tools(None).await;

        // Add frontend tools
        let frontend_tools = self.frontend_tools.lock().await;
        for frontend_tool in frontend_tools.values() {
            tools.push(frontend_tool.tool.clone());
        }

        // Prepare system prompt
        let extension_manager = self.extension_manager.lock().await;
        let extensions_info = extension_manager.get_extensions_info().await;

        // Get model name from provider
        let provider = self.provider().await?;
        let model_config = provider.get_model_config();
        let model_name = &model_config.model_name;

        let prompt_manager = self.prompt_manager.lock().await;
        let mut system_prompt = prompt_manager.build_system_prompt(
            extensions_info,
            self.frontend_instructions.lock().await.clone(),
            extension_manager.suggest_disable_extensions_prompt().await,
            Some(model_name),
        );

        // Handle toolshim if enabled
        let mut toolshim_tools = vec![];
        if model_config.toolshim {
            // If tool interpretation is enabled, modify the system prompt
            system_prompt = modify_system_prompt_for_tool_json(&system_prompt, &tools);
            // Make a copy of tools before emptying
            toolshim_tools = tools.clone();
            // Empty the tools vector for provider completion
            tools = vec![];
        }

        Ok((tools, toolshim_tools, system_prompt))
    }

    /// Categorize tools based on their annotations
    /// Returns:
    /// - read_only_tools: Tools with read-only annotations
    /// - non_read_tools: Tools without read-only annotations
    pub(crate) fn categorize_tools_by_annotation(
        tools: &[Tool],
    ) -> (HashSet<String>, HashSet<String>) {
        tools
            .iter()
            .fold((HashSet::new(), HashSet::new()), |mut acc, tool| {
                match &tool.annotations {
                    Some(annotations) if annotations.read_only_hint => {
                        acc.0.insert(tool.name.clone());
                    }
                    _ => {
                        acc.1.insert(tool.name.clone());
                    }
                }
                acc
            })
    }

    /// Generate a response from the LLM provider
    /// Handles toolshim transformations if needed
    pub(crate) async fn generate_response_from_provider(
        provider: Arc<dyn Provider>,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        toolshim_tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let config = provider.get_model_config();

        // Call the provider to get a response
        let (mut response, usage) = provider.complete(system_prompt, messages, tools).await?;

        // Store the model information in the global store
        crate::providers::base::set_current_model(&usage.model);

        // Post-process / structure the response only if tool interpretation is enabled
        if config.toolshim {
            let interpreter = OllamaInterpreter::new().map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to create OllamaInterpreter: {}", e))
            })?;

            response = augment_message_with_tool_calls(&interpreter, response, toolshim_tools)
                .await
                .map_err(|e| {
                    ProviderError::ExecutionError(format!("Failed to augment message: {}", e))
                })?;
        }

        Ok((response, usage))
    }

    /// Categorize tool requests from the response into different types
    /// Returns:
    /// - frontend_requests: Tool requests that should be handled by the frontend
    /// - other_requests: All other tool requests (including requests to enable extensions)
    /// - filtered_message: The original message with frontend tool requests removed
    pub(crate) async fn categorize_tool_requests(
        &self,
        response: &Message,
    ) -> (Vec<ToolRequest>, Vec<ToolRequest>, Message) {
        // First collect all tool requests
        let tool_requests: Vec<ToolRequest> = response
            .content
            .iter()
            .filter_map(|content| {
                if let MessageContent::ToolRequest(req) = content {
                    Some(req.clone())
                } else {
                    None
                }
            })
            .collect();

        // Create a filtered message with frontend tool requests removed
        let mut filtered_content = Vec::new();

        // Process each content item one by one
        for content in &response.content {
            let should_include = match content {
                MessageContent::ToolRequest(req) => {
                    if let Ok(tool_call) = &req.tool_call {
                        !self.is_frontend_tool(&tool_call.name).await
                    } else {
                        true
                    }
                }
                _ => true,
            };

            if should_include {
                filtered_content.push(content.clone());
            }
        }

        let filtered_message = Message {
            role: response.role.clone(),
            created: response.created,
            content: filtered_content,
        };

        // Categorize tool requests
        let mut frontend_requests = Vec::new();
        let mut other_requests = Vec::new();

        for request in tool_requests {
            if let Ok(tool_call) = &request.tool_call {
                if self.is_frontend_tool(&tool_call.name).await {
                    frontend_requests.push(request);
                } else {
                    other_requests.push(request);
                }
            } else {
                // If there's an error in the tool call, add it to other_requests
                other_requests.push(request);
            }
        }

        (frontend_requests, other_requests, filtered_message)
    }

    /// Update session metrics after a response
    pub(crate) async fn update_session_metrics(
        session_config: crate::agents::types::SessionConfig,
        usage: &crate::providers::base::ProviderUsage,
        messages_length: usize,
    ) -> Result<()> {
        let session_file = session::get_path(session_config.id);
        let mut metadata = session::read_metadata(&session_file)?;

        metadata.working_dir = session_config.working_dir.clone();
        metadata.total_tokens = usage.usage.total_tokens;
        metadata.input_tokens = usage.usage.input_tokens;
        metadata.output_tokens = usage.usage.output_tokens;
        // The message count is the number of messages in the session + 1 for the response
        // The message count does not include the tool response till next iteration
        metadata.message_count = messages_length + 1;

        // Keep running sum of tokens to track cost over the entire session
        let accumulate = |a: Option<i32>, b: Option<i32>| -> Option<i32> {
            match (a, b) {
                (Some(x), Some(y)) => Some(x + y),
                _ => a.or(b),
            }
        };
        metadata.accumulated_total_tokens =
            accumulate(metadata.accumulated_total_tokens, usage.usage.total_tokens);
        metadata.accumulated_input_tokens =
            accumulate(metadata.accumulated_input_tokens, usage.usage.input_tokens);
        metadata.accumulated_output_tokens = accumulate(
            metadata.accumulated_output_tokens,
            usage.usage.output_tokens,
        );
        session::update_metadata(&session_file, &metadata).await?;

        Ok(())
    }
}
