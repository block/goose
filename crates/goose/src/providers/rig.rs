//! Adapters to use rig CompletionModels as goose Providers and vice-versa.
//!
//! This module provides bidirectional integration between goose's Provider trait
//! and rig's CompletionModel trait, allowing:
//! - Using any rig CompletionModel as a goose Provider via `RigProvider`
//! - Using any goose Provider as a rig CompletionModel via `GooseProvider`

use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::stream::{self, StreamExt};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionRequestBuilder,
    CompletionResponse, GetTokenUsage, ToolDefinition,
};
use rig::message::{
    AssistantContent, DocumentSourceKind, Image, ImageMediaType, Message as RigMessage, Reasoning,
    Text, ToolCall as RigToolCall, ToolFunction, ToolResult as RigToolResult, ToolResultContent,
    UserContent,
};
use rig::providers::anthropic::completion::CLAUDE_4_OPUS;
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse, StreamingResult};
use rig::OneOrMany;
use rmcp::model::{CallToolRequestParams, CallToolResult, Content, Role, Tool};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::conversation::message::{
    Message, MessageContent, ThinkingContent, ToolRequest, ToolResponse,
};
use crate::model::ModelConfig;
use crate::providers::base::{MessageStream, Provider, ProviderDef, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;
use rig::providers::anthropic;

pub fn claude_opus() -> anthropic::completion::CompletionModel {
    let config = crate::config::Config::global();
    let api_key: String = config.get_secret("ANTHROPIC_API_KEY").unwrap();

    let client = anthropic::Client::from_val(api_key);

    client.completion_model(CLAUDE_4_OPUS)
}

/// Wraps a rig `CompletionModel` to implement goose's `Provider` trait.
///
/// This allows any model from rig (OpenAI, Anthropic, etc.) to be used
/// as a goose provider.
pub struct RigProvider<M>
where
    M: CompletionModel + Send + Sync,
{
    model: M,
    model_config: ModelConfig,
    name: String,
}

impl<M> RigProvider<M>
where
    M: CompletionModel + Send + Sync,
{
    pub fn new(model: M, model_config: ModelConfig, name: impl Into<String>) -> Self {
        Self {
            model,
            model_config,
            name: name.into(),
        }
    }
}

/// Convert goose messages to rig messages
fn goose_messages_to_rig(messages: &[Message]) -> Vec<RigMessage> {
    let mut rig_messages = Vec::new();

    for msg in messages {
        match msg.role {
            Role::User => {
                let mut user_content = Vec::new();

                for content in &msg.content {
                    match content {
                        MessageContent::Text(text) => {
                            user_content.push(UserContent::Text(Text {
                                text: text.text.clone(),
                            }));
                        }
                        MessageContent::ToolResponse(tool_response) => {
                            let result_content = match &tool_response.tool_result {
                                Ok(call_result) => {
                                    let text = call_result
                                        .content
                                        .iter()
                                        .filter_map(|c: &Content| {
                                            c.as_text().map(|s| s.text.clone())
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    OneOrMany::one(ToolResultContent::text(text))
                                }
                                Err(error_data) => OneOrMany::one(ToolResultContent::text(
                                    format!("Error: {}", error_data.message),
                                )),
                            };
                            user_content.push(UserContent::ToolResult(RigToolResult {
                                id: tool_response.id.clone(),
                                call_id: None,
                                content: result_content,
                            }));
                        }
                        MessageContent::Image(image) => {
                            let media_type = mime_to_image_media_type(&image.mime_type);
                            let rig_image = Image {
                                data: DocumentSourceKind::Base64(image.data.clone()),
                                media_type: Some(media_type),
                                detail: None,
                                additional_params: None,
                            };
                            user_content.push(UserContent::Image(rig_image));
                        }
                        _ => {}
                    }
                }

                if !user_content.is_empty() {
                    rig_messages.push(RigMessage::User {
                        content: OneOrMany::many(user_content)
                            .expect("user_content should not be empty"),
                    });
                }
            }
            Role::Assistant => {
                let mut assistant_content = Vec::new();

                for content in &msg.content {
                    match content {
                        MessageContent::Text(text) => {
                            assistant_content.push(AssistantContent::Text(Text {
                                text: text.text.clone(),
                            }));
                        }
                        MessageContent::ToolRequest(tool_request) => {
                            if let Ok(tool_call) = &tool_request.tool_call {
                                assistant_content.push(AssistantContent::ToolCall(RigToolCall {
                                    id: tool_request.id.clone(),
                                    call_id: None,
                                    function: ToolFunction {
                                        name: tool_call.name.to_string(),
                                        arguments: tool_call
                                            .arguments
                                            .clone()
                                            .map(Value::Object)
                                            .unwrap_or(Value::Null),
                                    },
                                    signature: None,
                                    additional_params: None,
                                }));
                            }
                        }
                        MessageContent::Thinking(thinking) => {
                            let signature = if thinking.signature.is_empty() {
                                None
                            } else {
                                Some(thinking.signature.clone())
                            };
                            let reasoning =
                                Reasoning::new(&thinking.thinking).with_signature(signature);
                            assistant_content.push(AssistantContent::Reasoning(reasoning));
                        }
                        _ => {}
                    }
                }

                if !assistant_content.is_empty() {
                    rig_messages.push(RigMessage::Assistant {
                        id: None,
                        content: OneOrMany::many(assistant_content)
                            .expect("assistant_content should not be empty"),
                    });
                }
            }
        }
    }

    rig_messages
}

/// Convert a MIME type string to rig's ImageMediaType
fn mime_to_image_media_type(mime: &str) -> ImageMediaType {
    match mime.to_lowercase().as_str() {
        "image/png" => ImageMediaType::PNG,
        "image/gif" => ImageMediaType::GIF,
        "image/webp" => ImageMediaType::WEBP,
        _ => ImageMediaType::JPEG,
    }
}

/// Convert goose tools to rig tool definitions
fn goose_tools_to_rig(tools: &[Tool]) -> Vec<ToolDefinition> {
    tools
        .iter()
        .map(|tool| ToolDefinition {
            name: tool.name.to_string(),
            description: tool.description.as_deref().unwrap_or("").to_string(),
            parameters: serde_json::to_value(&*tool.input_schema).unwrap_or(Value::Null),
        })
        .collect()
}

/// Convert rig AssistantContent to goose MessageContent
fn rig_content_to_goose(content: &AssistantContent) -> Option<MessageContent> {
    match content {
        AssistantContent::Text(text) => Some(MessageContent::text(&text.text)),
        AssistantContent::ToolCall(tool_call) => {
            let arguments = match &tool_call.function.arguments {
                Value::Object(obj) => Some(obj.clone()),
                Value::Null => None,
                other => {
                    let mut map = serde_json::Map::new();
                    map.insert("value".to_string(), other.clone());
                    Some(map)
                }
            };
            Some(MessageContent::ToolRequest(ToolRequest {
                id: tool_call.id.clone(),
                tool_call: Ok(CallToolRequestParams {
                    meta: None,
                    name: Cow::Owned(tool_call.function.name.clone()),
                    arguments,
                    task: None,
                }),
                metadata: None,
                tool_meta: None,
            }))
        }
        AssistantContent::Reasoning(reasoning) => Some(MessageContent::Thinking(ThinkingContent {
            thinking: reasoning.reasoning.join("\n"),
            signature: reasoning.signature.clone().unwrap_or_default(),
        })),
        AssistantContent::Image(_) => None,
    }
}

/// Convert rig response to goose Message
fn rig_response_to_goose<T>(response: &CompletionResponse<T>) -> Message {
    let content: Vec<MessageContent> = response
        .choice
        .iter()
        .filter_map(rig_content_to_goose)
        .collect();

    let mut message = Message::assistant();
    for c in content {
        message = message.with_content(c);
    }
    message
}

/// Convert rig Usage to goose Usage
fn rig_usage_to_goose(usage: &rig::completion::Usage) -> Usage {
    Usage {
        input_tokens: Some(usage.input_tokens as i32),
        output_tokens: Some(usage.output_tokens as i32),
        total_tokens: Some(usage.total_tokens as i32),
    }
}

#[async_trait]
impl<M> Provider for RigProvider<M>
where
    M: CompletionModel + Send + Sync + 'static,
    M::Response: Unpin + Send,
{
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model_config.clone()
    }

    async fn complete_with_model(
        &self,
        _session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let rig_messages = goose_messages_to_rig(messages);
        let rig_tools = goose_tools_to_rig(tools);

        let chat_history = if rig_messages.is_empty() {
            OneOrMany::one(RigMessage::user(""))
        } else {
            OneOrMany::many(rig_messages).expect("rig_messages should not be empty")
        };

        let request = CompletionRequest {
            preamble: Some(system.to_string()),
            chat_history,
            documents: vec![],
            tools: rig_tools,
            temperature: model_config.temperature.map(|t| t as f64),
            max_tokens: model_config.max_tokens.map(|t| t as u64),
            tool_choice: None,
            additional_params: None,
        };

        let response = self
            .model
            .completion(request)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

        let message = rig_response_to_goose(&response);
        let usage = rig_usage_to_goose(&response.usage);
        let provider_usage = ProviderUsage::new(model_config.model_name.clone(), usage);

        Ok((message, provider_usage))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn stream(
        &self,
        _session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        use rig::streaming::StreamedAssistantContent;

        let rig_messages = goose_messages_to_rig(messages);
        let rig_tools = goose_tools_to_rig(tools);

        let chat_history = if rig_messages.is_empty() {
            OneOrMany::one(RigMessage::user(""))
        } else {
            OneOrMany::many(rig_messages).expect("rig_messages should not be empty")
        };

        let request = CompletionRequest {
            preamble: Some(system.to_string()),
            chat_history,
            documents: vec![],
            tools: rig_tools,
            temperature: self.model_config.temperature.map(|t| t as f64),
            max_tokens: self.model_config.max_tokens.map(|t| t as u64),
            tool_choice: None,
            additional_params: None,
        };

        let mut streaming_response = self
            .model
            .stream(request)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

        let model_name = self.model_config.model_name.clone();

        let stream = async_stream::try_stream! {
            let mut input_tokens: Option<i32> = None;
            let mut output_tokens: Option<i32> = None;

            while let Some(chunk) = streaming_response.next().await {
                match chunk {
                    Ok(choice) => {
                        match choice {
                            StreamedAssistantContent::Text(text) => {
                                let message = Message::assistant().with_text(&text.text);
                                yield (Some(message), None);
                            }
                            StreamedAssistantContent::ToolCall { tool_call, .. } => {
                                let arguments = match &tool_call.function.arguments {
                                    Value::Object(obj) => Some(obj.clone()),
                                    Value::Null => None,
                                    other => {
                                        let mut map = serde_json::Map::new();
                                        map.insert("value".to_string(), other.clone());
                                        Some(map)
                                    }
                                };
                                let message = Message::assistant().with_tool_request(
                                    tool_call.id.clone(),
                                    Ok(CallToolRequestParams {
                                        meta: None,
                                        name: Cow::Owned(tool_call.function.name.clone()),
                                        arguments,
                                        task: None,
                                    }),
                                );
                                yield (Some(message), None);
                            }
                            StreamedAssistantContent::ToolCallDelta { .. } => {
                                // Deltas are intermediate updates; we handle full tool calls above
                            }
                            StreamedAssistantContent::Reasoning(reasoning) => {
                                let text = reasoning.reasoning.join("\n");
                                let sig = reasoning.signature.unwrap_or_default();
                                let message = Message::assistant().with_thinking(&text, &sig);
                                yield (Some(message), None);
                            }
                            StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                                let message = Message::assistant().with_thinking(&reasoning, "");
                                yield (Some(message), None);
                            }
                            StreamedAssistantContent::Final(response) => {
                                if let Some(usage) = response.token_usage() {
                                    input_tokens = Some(usage.input_tokens as i32);
                                    output_tokens = Some(usage.output_tokens as i32);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        Err(ProviderError::ExecutionError(e.to_string()))?;
                    }
                }
            }

            let usage = Usage {
                input_tokens,
                output_tokens,
                total_tokens: match (input_tokens, output_tokens) {
                    (Some(i), Some(o)) => Some(i + o),
                    _ => None,
                },
            };
            let provider_usage = ProviderUsage::new(model_name, usage);
            yield (None, Some(provider_usage));
        };

        Ok(Box::pin(stream))
    }
}

/// Wraps a goose `Provider` to implement rig's `CompletionModel` trait.
///
/// This allows any goose provider to be used with rig's agent framework.
#[derive(Clone)]
pub struct GooseProvider {
    provider: Arc<dyn Provider>,
    system_prompt: Arc<Mutex<String>>,
}

impl GooseProvider {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self {
            provider,
            system_prompt: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn with_system_prompt(
        provider: Arc<dyn Provider>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            system_prompt: Arc::new(Mutex::new(system_prompt.into())),
        }
    }

    pub async fn set_system_prompt(&self, prompt: impl Into<String>) {
        let mut sp = self.system_prompt.lock().await;
        *sp = prompt.into();
    }
}

/// Convert rig messages to goose messages
fn rig_messages_to_goose(messages: &OneOrMany<RigMessage>) -> Vec<Message> {
    let mut goose_messages = Vec::new();

    for msg in messages.iter() {
        match msg {
            RigMessage::User { content } => {
                let mut message = Message::user();

                for item in content.iter() {
                    match item {
                        UserContent::Text(text) => {
                            message = message.with_text(&text.text);
                        }
                        UserContent::ToolResult(result) => {
                            let content_vec: Vec<Content> = result
                                .content
                                .iter()
                                .filter_map(|c| match c {
                                    ToolResultContent::Text(t) => Some(Content::text(&t.text)),
                                    _ => None,
                                })
                                .collect();
                            message
                                .content
                                .push(MessageContent::ToolResponse(ToolResponse {
                                    id: result.id.clone(),
                                    tool_result: Ok(CallToolResult {
                                        content: content_vec,
                                        structured_content: None,
                                        is_error: None,
                                        meta: None,
                                    }),
                                    metadata: None,
                                }));
                        }
                        UserContent::Image(_)
                        | UserContent::Audio(_)
                        | UserContent::Document(_)
                        | UserContent::Video(_) => {}
                    }
                }

                if !message.content.is_empty() {
                    goose_messages.push(message);
                }
            }
            RigMessage::Assistant { content, .. } => {
                let mut message = Message::assistant();

                for item in content.iter() {
                    match item {
                        AssistantContent::Text(text) => {
                            message = message.with_text(&text.text);
                        }
                        AssistantContent::ToolCall(tool_call) => {
                            let arguments = match &tool_call.function.arguments {
                                Value::Object(obj) => Some(obj.clone()),
                                Value::Null => None,
                                other => {
                                    let mut map = serde_json::Map::new();
                                    map.insert("value".to_string(), other.clone());
                                    Some(map)
                                }
                            };
                            message
                                .content
                                .push(MessageContent::ToolRequest(ToolRequest {
                                    id: tool_call.id.clone(),
                                    tool_call: Ok(CallToolRequestParams {
                                        meta: None,
                                        name: Cow::Owned(tool_call.function.name.clone()),
                                        arguments,
                                        task: None,
                                    }),
                                    metadata: None,
                                    tool_meta: None,
                                }));
                        }
                        AssistantContent::Reasoning(reasoning) => {
                            message
                                .content
                                .push(MessageContent::Thinking(ThinkingContent {
                                    thinking: reasoning.reasoning.join("\n"),
                                    signature: reasoning.signature.clone().unwrap_or_default(),
                                }));
                        }
                        AssistantContent::Image(_) => {}
                    }
                }

                if !message.content.is_empty() {
                    goose_messages.push(message);
                }
            }
        }
    }

    goose_messages
}

/// Convert rig tool definitions to goose tools
fn rig_tools_to_goose(tools: &[ToolDefinition]) -> Vec<Tool> {
    tools
        .iter()
        .map(|tool| {
            let input_schema = match &tool.parameters {
                Value::Object(obj) => obj.clone(),
                _ => serde_json::Map::new(),
            };
            Tool {
                name: tool.name.clone().into(),
                title: None,
                description: Some(tool.description.clone().into()),
                input_schema: Arc::new(input_schema),
                output_schema: None,
                annotations: None,
                icons: None,
                meta: None,
            }
        })
        .collect()
}

/// Convert goose message to rig AssistantContent
fn goose_message_to_rig_content(message: &Message) -> OneOrMany<AssistantContent> {
    let content: Vec<AssistantContent> = message
        .content
        .iter()
        .filter_map(|c| match c {
            MessageContent::Text(text) => Some(AssistantContent::Text(Text {
                text: text.text.clone(),
            })),
            MessageContent::ToolRequest(req) => {
                if let Ok(tool_call) = &req.tool_call {
                    Some(AssistantContent::ToolCall(RigToolCall {
                        id: req.id.clone(),
                        call_id: None,
                        function: ToolFunction {
                            name: tool_call.name.to_string(),
                            arguments: tool_call
                                .arguments
                                .clone()
                                .map(Value::Object)
                                .unwrap_or(Value::Null),
                        },
                        signature: None,
                        additional_params: None,
                    }))
                } else {
                    None
                }
            }
            MessageContent::Thinking(thinking) => {
                let signature = if thinking.signature.is_empty() {
                    None
                } else {
                    Some(thinking.signature.clone())
                };
                let reasoning = Reasoning::new(&thinking.thinking).with_signature(signature);
                Some(AssistantContent::Reasoning(reasoning))
            }
            _ => None,
        })
        .collect();

    if content.is_empty() {
        OneOrMany::one(AssistantContent::text(""))
    } else {
        OneOrMany::many(content).unwrap_or_else(|_| OneOrMany::one(AssistantContent::text("")))
    }
}

/// Response type for GooseProvider that implements GetTokenUsage
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GooseResponse {
    pub usage: Option<Usage>,
}

impl GetTokenUsage for GooseResponse {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        self.usage.map(|u| rig::completion::Usage {
            input_tokens: u.input_tokens.unwrap_or(0) as u64,
            output_tokens: u.output_tokens.unwrap_or(0) as u64,
            total_tokens: u.total_tokens.unwrap_or(0) as u64,
            cached_input_tokens: 0,
        })
    }
}

/// Dummy client type for GooseProvider
#[derive(Clone)]
pub struct GooseProviderClient;

impl CompletionModel for GooseProvider {
    type Response = GooseResponse;
    type StreamingResponse = GooseResponse;
    type Client = GooseProviderClient;

    fn make(_client: &Self::Client, _model: impl Into<String>) -> Self {
        panic!("GooseProvider cannot be created via make(). Use GooseProvider::new() instead.")
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let system_prompt = self.system_prompt.lock().await.clone();
        let system = request.preamble.as_deref().unwrap_or(&system_prompt);

        let messages = rig_messages_to_goose(&request.chat_history);
        let tools = rig_tools_to_goose(&request.tools);

        let (message, provider_usage) = self
            .provider
            .complete("", system, &messages, &tools)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        let choice = goose_message_to_rig_content(&message);
        let usage = provider_usage.usage;

        Ok(CompletionResponse {
            choice,
            usage: rig::completion::Usage {
                input_tokens: usage.input_tokens.unwrap_or(0) as u64,
                output_tokens: usage.output_tokens.unwrap_or(0) as u64,
                total_tokens: usage.total_tokens.unwrap_or(0) as u64,
                cached_input_tokens: 0,
            },
            raw_response: GooseResponse { usage: Some(usage) },
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        // Implement streaming by calling completion and yielding the result.
        // A proper implementation would use the provider's native streaming if available.
        let response = self.completion(request).await?;

        let text_content: String = response
            .choice
            .iter()
            .filter_map(|c| match c {
                AssistantContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let tool_calls: Vec<_> = response
            .choice
            .iter()
            .filter_map(|c| match c {
                AssistantContent::ToolCall(tc) => Some(tc.clone()),
                _ => None,
            })
            .collect();

        let final_response = response.raw_response.clone();

        let mut items: Vec<RawStreamingChoice<GooseResponse>> = Vec::new();

        if !text_content.is_empty() {
            items.push(RawStreamingChoice::Message(text_content));
        }

        for tool_call in tool_calls {
            items.push(RawStreamingChoice::ToolCall(
                rig::streaming::RawStreamingToolCall {
                    id: tool_call.id.clone(),
                    internal_call_id: nanoid::nanoid!(),
                    call_id: tool_call.call_id.clone(),
                    name: tool_call.function.name.clone(),
                    arguments: tool_call.function.arguments.clone(),
                    signature: tool_call.signature.clone(),
                    additional_params: tool_call.additional_params.clone(),
                },
            ));
        }

        items.push(RawStreamingChoice::FinalResponse(final_response));

        let stream: StreamingResult<GooseResponse> =
            Box::pin(stream::iter(items.into_iter().map(Ok)));

        Ok(StreamingCompletionResponse::stream(stream))
    }

    fn completion_request(&self, prompt: impl Into<RigMessage>) -> CompletionRequestBuilder<Self> {
        CompletionRequestBuilder::new(self.clone(), prompt)
    }
}

impl ProviderDef for RigProvider<anthropic::completion::CompletionModel> {
    type Provider = RigProvider<anthropic::completion::CompletionModel>;

    fn metadata() -> super::base::ProviderMetadata
    where
        Self: Sized,
    {
        super::base::ProviderMetadata {
            name: "anthropic-rig".to_owned(),
            display_name: "Anthropic (rig)".to_owned(),
            description: "".to_owned(),
            default_model: "claude-4-opus".to_owned(),
            known_models: vec![],
            model_doc_link: "".to_owned(),
            config_keys: vec![],
            allows_unlisted_models: true,
        }
    }

    fn from_env(_model: ModelConfig) -> BoxFuture<'static, anyhow::Result<Self::Provider>>
    where
        Self: Sized,
    {
        let model = claude_opus();
        Box::pin(async move {
            Ok(RigProvider::new(
                model,
                ModelConfig::new("claude opus (rig)").unwrap(),
                "claude opus (rig)",
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goose_messages_to_rig_user() {
        let messages = vec![Message::user().with_text("Hello, world!")];
        let rig_messages = goose_messages_to_rig(&messages);

        assert_eq!(rig_messages.len(), 1);
        match &rig_messages[0] {
            RigMessage::User { content } => {
                assert_eq!(content.iter().count(), 1);
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_goose_messages_to_rig_assistant() {
        let messages = vec![Message::assistant().with_text("Hello!")];
        let rig_messages = goose_messages_to_rig(&messages);

        assert_eq!(rig_messages.len(), 1);
        match &rig_messages[0] {
            RigMessage::Assistant { content, .. } => {
                assert_eq!(content.iter().count(), 1);
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_rig_messages_to_goose_user() {
        let messages = OneOrMany::one(RigMessage::user("Hello!"));
        let goose_messages = rig_messages_to_goose(&messages);

        assert_eq!(goose_messages.len(), 1);
        assert_eq!(goose_messages[0].role, Role::User);
    }

    #[test]
    fn test_rig_messages_to_goose_assistant() {
        let messages = OneOrMany::one(RigMessage::assistant("Hello!"));
        let goose_messages = rig_messages_to_goose(&messages);

        assert_eq!(goose_messages.len(), 1);
        assert_eq!(goose_messages[0].role, Role::Assistant);
    }

    #[test]
    fn test_tool_conversion_roundtrip() {
        let goose_tool = Tool {
            name: "test_tool".into(),
            title: None,
            description: Some("A test tool".into()),
            input_schema: Arc::new(serde_json::Map::new()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        };

        let rig_tools = goose_tools_to_rig(std::slice::from_ref(&goose_tool));
        assert_eq!(rig_tools.len(), 1);
        assert_eq!(rig_tools[0].name, "test_tool");
        assert_eq!(rig_tools[0].description, "A test tool");

        let back_to_goose = rig_tools_to_goose(&rig_tools);
        assert_eq!(back_to_goose.len(), 1);
        assert_eq!(back_to_goose[0].name, "test_tool");
    }
}
