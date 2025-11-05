use anyhow::Result;
use std::sync::Arc;

use async_stream::try_stream;
use futures::stream::StreamExt;
use serde_json::{json, Value};
use tracing::debug;

use super::super::agents::Agent;
use crate::conversation::message::{Message, MessageContent, ToolRequest};
use crate::conversation::Conversation;
use crate::providers::base::{stream_from_single_message, MessageStream, Provider, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::providers::toolshim::{
    augment_message_with_tool_calls, convert_tool_messages_to_text,
    modify_system_prompt_for_tool_json, OllamaInterpreter,
};

use crate::agents::recipe_tools::dynamic_task_tools::should_enabled_subagents;
use crate::session::SessionManager;
use rmcp::model::Tool;

fn coerce_value(s: &str, schema: &Value) -> Value {
    let type_str = schema.get("type");

    match type_str {
        Some(Value::String(t)) => match t.as_str() {
            "number" | "integer" => try_coerce_number(s),
            "boolean" => try_coerce_boolean(s),
            _ => Value::String(s.to_string()),
        },
        Some(Value::Array(types)) => {
            // Try each type in order
            for t in types {
                if let Value::String(type_name) = t {
                    match type_name.as_str() {
                        "number" | "integer" if s.parse::<f64>().is_ok() => {
                            return try_coerce_number(s)
                        }
                        "boolean" if matches!(s.to_lowercase().as_str(), "true" | "false") => {
                            return try_coerce_boolean(s)
                        }
                        _ => continue,
                    }
                }
            }
            Value::String(s.to_string())
        }
        _ => Value::String(s.to_string()),
    }
}

fn try_coerce_number(s: &str) -> Value {
    if let Ok(n) = s.parse::<f64>() {
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            json!(n as i64)
        } else {
            json!(n)
        }
    } else {
        Value::String(s.to_string())
    }
}

fn try_coerce_boolean(s: &str) -> Value {
    match s.to_lowercase().as_str() {
        "true" => json!(true),
        "false" => json!(false),
        _ => Value::String(s.to_string()),
    }
}

fn coerce_tool_arguments(
    arguments: Option<serde_json::Map<String, Value>>,
    tool_schema: &Value,
) -> Option<serde_json::Map<String, Value>> {
    let args = arguments?;

    let properties = tool_schema.get("properties").and_then(|p| p.as_object())?;

    let mut coerced = serde_json::Map::new();

    for (key, value) in args.iter() {
        let coerced_value =
            if let (Value::String(s), Some(prop_schema)) = (value, properties.get(key)) {
                coerce_value(s, prop_schema)
            } else {
                value.clone()
            };
        coerced.insert(key.clone(), coerced_value);
    }

    Some(coerced)
}

async fn toolshim_postprocess(
    response: Message,
    toolshim_tools: &[Tool],
) -> Result<Message, ProviderError> {
    let interpreter = OllamaInterpreter::new().map_err(|e| {
        ProviderError::ExecutionError(format!("Failed to create OllamaInterpreter: {}", e))
    })?;

    augment_message_with_tool_calls(&interpreter, response, toolshim_tools)
        .await
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to augment message: {}", e)))
}

impl Agent {
    pub async fn prepare_tools_and_prompt(&self) -> Result<(Vec<Tool>, Vec<Tool>, String)> {
        // Get router enabled status
        let router_enabled = self.tool_route_manager.is_router_enabled().await;

        // Get tools from extension manager
        let mut tools = self.list_tools_for_router().await;

        // If router is disabled and no tools were returned, fall back to regular tools
        if !router_enabled && tools.is_empty() {
            tools = self.list_tools(None).await;
            let provider = self.provider().await?;
            let model_name = provider.get_model_config().model_name;

            if !should_enabled_subagents(&model_name) {
                tools.retain(|tool| {
                    tool.name != crate::agents::subagent_execution_tool::subagent_execute_task_tool::SUBAGENT_EXECUTE_TASK_TOOL_NAME
                        && tool.name != crate::agents::recipe_tools::dynamic_task_tools::DYNAMIC_TASK_TOOL_NAME_PREFIX
                });
            }
        }

        // Add frontend tools
        let frontend_tools = self.frontend_tools.lock().await;
        for frontend_tool in frontend_tools.values() {
            tools.push(frontend_tool.tool.clone());
        }

        if !router_enabled {
            // Stable tool ordering is important for multi session prompt caching.
            tools.sort_by(|a, b| a.name.cmp(&b.name));
        }

        // Prepare system prompt
        let extensions_info = self.extension_manager.get_extensions_info().await;
        let (extension_count, tool_count) =
            self.extension_manager.get_extension_and_tool_counts().await;

        // Get model name from provider
        let provider = self.provider().await?;
        let model_config = provider.get_model_config();
        let model_name = &model_config.model_name;

        let prompt_manager = self.prompt_manager.lock().await;
        let mut system_prompt = prompt_manager
            .builder(model_name)
            .with_extensions(extensions_info.into_iter())
            .with_frontend_instructions(self.frontend_instructions.lock().await.clone())
            .with_extension_and_tool_counts(extension_count, tool_count)
            .with_router_enabled(router_enabled)
            .build();

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

    /// Stream a response from the LLM provider.
    /// Handles toolshim transformations if needed
    pub(crate) async fn stream_response_from_provider(
        provider: Arc<dyn Provider>,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        toolshim_tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let config = provider.get_model_config();

        // Convert tool messages to text if toolshim is enabled
        let messages_for_provider = if config.toolshim {
            convert_tool_messages_to_text(messages)
        } else {
            Conversation::new_unvalidated(messages.to_vec())
        };

        // Clone owned data to move into the async stream
        let system_prompt = system_prompt.to_owned();
        let tools = tools.to_owned();
        let toolshim_tools = toolshim_tools.to_owned();
        let provider = provider.clone();

        // Capture errors during stream creation and return them as part of the stream
        // so they can be handled by the existing error handling logic in the agent
        let stream_result = if provider.supports_streaming() {
            debug!("WAITING_LLM_STREAM_START");
            let result = provider
                .stream(
                    system_prompt.as_str(),
                    messages_for_provider.messages(),
                    &tools,
                )
                .await;
            debug!("WAITING_LLM_STREAM_END");
            result
        } else {
            debug!("WAITING_LLM_START");
            let complete_result = provider
                .complete(
                    system_prompt.as_str(),
                    messages_for_provider.messages(),
                    &tools,
                )
                .await;
            debug!("WAITING_LLM_END");

            match complete_result {
                Ok((message, usage)) => Ok(stream_from_single_message(message, usage)),
                Err(e) => Err(e),
            }
        };

        // If there was an error creating the stream, return a stream that yields that error
        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                // Return a stream that immediately yields the error
                // This allows the error to be caught by existing error handling in agent.rs
                return Ok(Box::pin(try_stream! {
                    yield Err(e)?;
                }));
            }
        };

        Ok(Box::pin(try_stream! {
            while let Some(Ok((mut message, usage))) = stream.next().await {
                // Store the model information in the global store
                if let Some(usage) = usage.as_ref() {
                    crate::providers::base::set_current_model(&usage.model);
                }

                // Post-process / structure the response only if tool interpretation is enabled
                if message.is_some() && config.toolshim {
                    message = Some(toolshim_postprocess(message.unwrap(), &toolshim_tools).await?);
                }

                yield (message, usage);
            }
        }))
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
        let tools = self.list_tools(None).await;

        // First collect all tool requests with coercion applied
        let tool_requests: Vec<ToolRequest> = response
            .content
            .iter()
            .filter_map(|content| {
                if let MessageContent::ToolRequest(req) = content {
                    let mut coerced_req = req.clone();

                    if let Ok(ref mut tool_call) = coerced_req.tool_call {
                        if let Some(tool) = tools.iter().find(|t| t.name == tool_call.name) {
                            let schema_value = Value::Object(tool.input_schema.as_ref().clone());
                            tool_call.arguments =
                                coerce_tool_arguments(tool_call.arguments.clone(), &schema_value);
                        }
                    }

                    Some(coerced_req)
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

        let mut filtered_message =
            Message::new(response.role.clone(), response.created, filtered_content);

        // Preserve the ID if it exists
        if let Some(id) = response.id.clone() {
            filtered_message = filtered_message.with_id(id);
        }

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

    pub(crate) async fn update_session_metrics(
        session_config: &crate::agents::types::SessionConfig,
        usage: &ProviderUsage,
        is_compaction_usage: bool,
    ) -> Result<()> {
        let session_id = session_config.id.as_str();
        let session = SessionManager::get_session(session_id, false).await?;

        let accumulate = |a: Option<i32>, b: Option<i32>| -> Option<i32> {
            match (a, b) {
                (Some(x), Some(y)) => Some(x + y),
                _ => a.or(b),
            }
        };

        let accumulated_total =
            accumulate(session.accumulated_total_tokens, usage.usage.total_tokens);
        let accumulated_input =
            accumulate(session.accumulated_input_tokens, usage.usage.input_tokens);
        let accumulated_output =
            accumulate(session.accumulated_output_tokens, usage.usage.output_tokens);

        let (current_total, current_input, current_output) = if is_compaction_usage {
            // After compaction: summary output becomes new input context
            let new_input = usage.usage.output_tokens;
            (new_input, new_input, None)
        } else {
            (
                usage.usage.total_tokens,
                usage.usage.input_tokens,
                usage.usage.output_tokens,
            )
        };

        SessionManager::update_session(session_id)
            .schedule_id(session_config.schedule_id.clone())
            .total_tokens(current_total)
            .input_tokens(current_input)
            .output_tokens(current_output)
            .accumulated_total_tokens(accumulated_total)
            .accumulated_input_tokens(accumulated_input)
            .accumulated_output_tokens(accumulated_output)
            .apply()
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::model::ModelConfig;
    use crate::providers::base::{Provider, ProviderUsage, Usage};
    use crate::providers::errors::ProviderError;
    use async_trait::async_trait;
    use rmcp::object;

    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> crate::providers::base::ProviderMetadata {
            crate::providers::base::ProviderMetadata::empty()
        }

        fn get_name(&self) -> &str {
            "mock"
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete_with_model(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("ok"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    #[tokio::test]
    async fn prepare_tools_sorts_when_router_disabled_and_includes_frontend_and_list_tools(
    ) -> anyhow::Result<()> {
        let agent = crate::agents::Agent::new();

        let model_config = ModelConfig::new("test-model").unwrap();
        let provider = std::sync::Arc::new(MockProvider { model_config });
        agent.update_provider(provider).await?;

        // Disable the router to trigger sorting
        agent.disable_router_for_recipe().await;

        // Add unsorted frontend tools
        let frontend_tools = vec![
            Tool::new(
                "frontend__z_tool".to_string(),
                "Z tool".to_string(),
                object!({ "type": "object", "properties": { } }),
            ),
            Tool::new(
                "frontend__a_tool".to_string(),
                "A tool".to_string(),
                object!({ "type": "object", "properties": { } }),
            ),
        ];

        agent
            .add_extension(crate::agents::extension::ExtensionConfig::Frontend {
                name: "frontend".to_string(),
                description: "desc".to_string(),
                tools: frontend_tools,
                instructions: None,
                bundled: None,
                available_tools: vec![],
            })
            .await
            .unwrap();

        let (tools, _toolshim_tools, _system_prompt) = agent.prepare_tools_and_prompt().await?;

        // Ensure both platform and frontend tools are present
        let names: Vec<String> = tools.iter().map(|t| t.name.clone().into_owned()).collect();
        assert!(names.iter().any(|n| n.starts_with("platform__")));
        assert!(names.iter().any(|n| n == "frontend__a_tool"));
        assert!(names.iter().any(|n| n == "frontend__z_tool"));

        // Verify the names are sorted ascending
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);

        Ok(())
    }
}
