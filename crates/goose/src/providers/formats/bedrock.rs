use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use crate::mcp_utils::ToolResult;
use anyhow::{anyhow, bail, Result};
use aws_sdk_bedrockruntime::types as bedrock;
use aws_sdk_bedrockruntime::types::ConverseStreamOutput;
use aws_smithy_types::{Document, Number};
use base64::Engine;
use chrono::Utc;
use rmcp::model::{
    object, CallToolRequestParams, Content, ErrorCode, ErrorData, RawContent, ResourceContents,
    Role, Tool,
};
use serde_json::Value;

use super::super::base::{MessageStream, ProviderUsage, Usage};
use super::super::errors::ProviderError;
use crate::conversation::message::{Message, MessageContent};

pub fn to_bedrock_message(message: &Message) -> Result<bedrock::Message> {
    bedrock::Message::builder()
        .role(to_bedrock_role(&message.role))
        .set_content(Some(
            message
                .content
                .iter()
                .map(to_bedrock_message_content)
                .collect::<Result<_>>()?,
        ))
        .build()
        .map_err(|err| anyhow!("Failed to construct Bedrock message: {}", err))
}

pub fn to_bedrock_message_content(content: &MessageContent) -> Result<bedrock::ContentBlock> {
    Ok(match content {
        MessageContent::Text(text) => bedrock::ContentBlock::Text(text.text.to_string()),
        MessageContent::ToolConfirmationRequest(_tool_confirmation_request) => {
            bedrock::ContentBlock::Text("".to_string())
        }
        MessageContent::ActionRequired(_action_required) => {
            bedrock::ContentBlock::Text("".to_string())
        }
        MessageContent::Image(image) => {
            bedrock::ContentBlock::Image(to_bedrock_image(&image.data, &image.mime_type)?)
        }
        MessageContent::Thinking(_) => {
            // Thinking blocks are not supported in Bedrock - skip
            bedrock::ContentBlock::Text("".to_string())
        }
        MessageContent::RedactedThinking(_) => {
            // Redacted thinking blocks are not supported in Bedrock - skip
            bedrock::ContentBlock::Text("".to_string())
        }
        MessageContent::SystemNotification(_) => {
            bail!("SystemNotification should not get passed to the provider")
        }
        MessageContent::ToolRequest(tool_req) => {
            let tool_use_id = tool_req.id.to_string();
            let tool_use = if let Ok(call) = tool_req.tool_call.as_ref() {
                bedrock::ToolUseBlock::builder()
                    .tool_use_id(tool_use_id)
                    .name(call.name.to_string())
                    .input(to_bedrock_json(&Value::from(call.arguments.clone())))
                    .build()
            } else {
                bedrock::ToolUseBlock::builder()
                    .tool_use_id(tool_use_id)
                    .build()
            }?;
            bedrock::ContentBlock::ToolUse(tool_use)
        }
        MessageContent::FrontendToolRequest(tool_req) => {
            let tool_use_id = tool_req.id.to_string();
            let tool_use = if let Ok(call) = tool_req.tool_call.as_ref() {
                bedrock::ToolUseBlock::builder()
                    .tool_use_id(tool_use_id)
                    .name(call.name.to_string())
                    .input(to_bedrock_json(&Value::from(call.arguments.clone())))
                    .build()
            } else {
                bedrock::ToolUseBlock::builder()
                    .tool_use_id(tool_use_id)
                    .build()
            }?;
            bedrock::ContentBlock::ToolUse(tool_use)
        }
        MessageContent::ToolResponse(tool_res) => {
            let content = match &tool_res.tool_result {
                Ok(result) => Some(
                    result
                        .content
                        .iter()
                        .map(|c| to_bedrock_tool_result_content_block(&tool_res.id, c.clone()))
                        .collect::<Result<_>>()?,
                ),
                Err(error) => {
                    // For errors, create a text content block with the error message
                    Some(vec![bedrock::ToolResultContentBlock::Text(format!(
                        "The tool call returned the following error:\n{}",
                        error
                    ))])
                }
            };
            bedrock::ContentBlock::ToolResult(
                bedrock::ToolResultBlock::builder()
                    .tool_use_id(tool_res.id.to_string())
                    .status(if tool_res.tool_result.is_ok() {
                        bedrock::ToolResultStatus::Success
                    } else {
                        bedrock::ToolResultStatus::Error
                    })
                    .set_content(content)
                    .build()?,
            )
        }
        MessageContent::Reasoning(_reasoning) => {
            // Reasoning content is for OpenAI-compatible APIs (e.g., DeepSeek)
            // Bedrock doesn't use this format, so skip
            bedrock::ContentBlock::Text("".to_string())
        }
    })
}

/// Convert MCP Content to Bedrock ToolResultContentBlock
///
/// Supports text, images, and document resources. Images are supported
/// by Bedrock for Anthropic Claude 3 models.
pub fn to_bedrock_tool_result_content_block(
    tool_use_id: &str,
    content: Content,
) -> Result<bedrock::ToolResultContentBlock> {
    Ok(match content.raw {
        RawContent::Text(text) => bedrock::ToolResultContentBlock::Text(text.text),
        RawContent::Image(image) => {
            bedrock::ToolResultContentBlock::Image(to_bedrock_image(&image.data, &image.mime_type)?)
        }
        RawContent::ResourceLink(_link) => {
            bedrock::ToolResultContentBlock::Text("[Resource link]".to_string())
        }
        RawContent::Resource(resource) => match &resource.resource {
            ResourceContents::TextResourceContents { text, .. } => {
                match to_bedrock_document(tool_use_id, &resource.resource)? {
                    Some(doc) => bedrock::ToolResultContentBlock::Document(doc),
                    None => bedrock::ToolResultContentBlock::Text(text.to_string()),
                }
            }
            ResourceContents::BlobResourceContents { .. } => {
                bail!("Blob resource content is not supported by Bedrock provider yet")
            }
        },
        RawContent::Audio(..) => bail!("Audio is not supported by Bedrock provider"),
    })
}

pub fn to_bedrock_role(role: &Role) -> bedrock::ConversationRole {
    match role {
        Role::User => bedrock::ConversationRole::User,
        Role::Assistant => bedrock::ConversationRole::Assistant,
    }
}

pub fn to_bedrock_image(data: &String, mime_type: &String) -> Result<bedrock::ImageBlock> {
    // Extract format from MIME type
    let format = match mime_type.as_str() {
        "image/png" => bedrock::ImageFormat::Png,
        "image/jpeg" | "image/jpg" => bedrock::ImageFormat::Jpeg,
        "image/gif" => bedrock::ImageFormat::Gif,
        "image/webp" => bedrock::ImageFormat::Webp,
        _ => bail!(
            "Unsupported image format: {}. Bedrock supports png, jpeg, gif, webp",
            mime_type
        ),
    };

    // Create image source with base64 data
    let source = bedrock::ImageSource::Bytes(aws_smithy_types::Blob::new(
        base64::prelude::BASE64_STANDARD
            .decode(data)
            .map_err(|e| anyhow!("Failed to decode base64 image data: {}", e))?,
    ));

    // Build the image block
    Ok(bedrock::ImageBlock::builder()
        .format(format)
        .source(source)
        .build()?)
}

pub fn to_bedrock_tool_config(tools: &[Tool]) -> Result<bedrock::ToolConfiguration> {
    Ok(bedrock::ToolConfiguration::builder()
        .set_tools(Some(
            tools.iter().map(to_bedrock_tool).collect::<Result<_>>()?,
        ))
        .build()?)
}

pub fn to_bedrock_tool(tool: &Tool) -> Result<bedrock::Tool> {
    let mut input_schema = tool.input_schema.as_ref().clone();

    // If the schema doesn't have a "type" field, add it
    // This is required by Bedrock
    if !input_schema.contains_key("type") {
        input_schema.insert("type".to_string(), Value::String("object".to_string()));
    }

    Ok(bedrock::Tool::ToolSpec(
        bedrock::ToolSpecification::builder()
            .name(tool.name.to_string())
            .description(
                tool.description
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
            )
            .input_schema(bedrock::ToolInputSchema::Json(to_bedrock_json(
                &Value::Object(input_schema),
            )))
            .build()?,
    ))
}

pub fn to_bedrock_json(value: &Value) -> Document {
    match value {
        Value::Null => Document::Null,
        Value::Bool(bool) => Document::Bool(*bool),
        Value::Number(num) => {
            if let Some(n) = num.as_u64() {
                Document::Number(Number::PosInt(n))
            } else if let Some(n) = num.as_i64() {
                Document::Number(Number::NegInt(n))
            } else if let Some(n) = num.as_f64() {
                Document::Number(Number::Float(n))
            } else {
                unreachable!()
            }
        }
        Value::String(str) => Document::String(str.to_string()),
        Value::Array(arr) => Document::Array(arr.iter().map(to_bedrock_json).collect()),
        Value::Object(obj) => Document::Object(HashMap::from_iter(
            obj.into_iter()
                .map(|(key, val)| (key.to_string(), to_bedrock_json(val))),
        )),
    }
}

fn to_bedrock_document(
    tool_use_id: &str,
    content: &ResourceContents,
) -> Result<Option<bedrock::DocumentBlock>> {
    let (uri, text) = match content {
        ResourceContents::TextResourceContents { uri, text, .. } => (uri, text),
        ResourceContents::BlobResourceContents { .. } => {
            bail!("Blob resource content is not supported by Bedrock provider yet")
        }
    };

    let filename = Path::new(uri)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(uri);

    // Return None if the file type is not supported
    let (name, format) = match filename.split_once('.') {
        Some((name, "txt")) => (name, bedrock::DocumentFormat::Txt),
        Some((name, "csv")) => (name, bedrock::DocumentFormat::Csv),
        Some((name, "md")) => (name, bedrock::DocumentFormat::Md),
        Some((name, "html")) => (name, bedrock::DocumentFormat::Html),
        _ => return Ok(None), // Not a supported document type
    };

    // Since we can't use the full path (due to character limit and also Bedrock does not accept `/` etc.),
    // and Bedrock wants document names to be unique, we're adding `tool_use_id` as a prefix to make
    // document names unique.
    let name = format!("{tool_use_id}-{name}");

    Ok(Some(
        bedrock::DocumentBlock::builder()
            .format(format)
            .name(name)
            .source(bedrock::DocumentSource::Bytes(text.as_bytes().into()))
            .build()
            .map_err(|err| anyhow!("Failed to construct Bedrock document: {}", err))?,
    ))
}

pub fn from_bedrock_message(message: &bedrock::Message) -> Result<Message> {
    let role = from_bedrock_role(message.role())?;
    let content = message
        .content()
        .iter()
        .map(from_bedrock_content_block)
        .collect::<Result<Vec<_>>>()?;
    let created = Utc::now().timestamp();

    Ok(Message::new(role, created, content))
}

pub fn from_bedrock_content_block(block: &bedrock::ContentBlock) -> Result<MessageContent> {
    Ok(match block {
        bedrock::ContentBlock::Text(text) => MessageContent::text(text),
        bedrock::ContentBlock::ToolUse(tool_use) => MessageContent::tool_request(
            tool_use.tool_use_id.to_string(),
            Ok(CallToolRequestParams {
                meta: None,
                task: None,
                name: tool_use.name.clone().into(),
                arguments: Some(object(from_bedrock_json(&tool_use.input.clone())?)),
            }),
        ),
        bedrock::ContentBlock::ToolResult(tool_res) => MessageContent::tool_response(
            tool_res.tool_use_id.to_string(),
            if tool_res.content.is_empty() {
                Err(ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: Cow::from("Empty content for tool use from Bedrock".to_string()),
                    data: None,
                })
            } else {
                tool_res
                    .content
                    .iter()
                    .map(from_bedrock_tool_result_content_block)
                    .collect::<ToolResult<Vec<_>>>()
                    .map(|content| rmcp::model::CallToolResult {
                        content,
                        structured_content: None,
                        is_error: Some(false),
                        meta: None,
                    })
            },
        ),
        _ => bail!("Unsupported content block type from Bedrock"),
    })
}

pub fn from_bedrock_tool_result_content_block(
    content: &bedrock::ToolResultContentBlock,
) -> ToolResult<Content> {
    Ok(match content {
        bedrock::ToolResultContentBlock::Text(text) => Content::text(text.to_string()),
        _ => {
            return Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from("Unsupported tool result from Bedrock".to_string()),
                data: None,
            })
        }
    })
}

pub fn from_bedrock_role(role: &bedrock::ConversationRole) -> Result<Role> {
    Ok(match role {
        bedrock::ConversationRole::User => Role::User,
        bedrock::ConversationRole::Assistant => Role::Assistant,
        _ => bail!("Unknown role from Bedrock"),
    })
}

pub fn from_bedrock_usage(usage: &bedrock::TokenUsage) -> Usage {
    Usage::new(
        Some(usage.input_tokens),
        Some(usage.output_tokens),
        Some(usage.total_tokens),
    )
}

pub fn from_bedrock_json(document: &Document) -> Result<Value> {
    Ok(match document {
        Document::Null => Value::Null,
        Document::Bool(bool) => Value::Bool(*bool),
        Document::Number(num) => match num {
            Number::PosInt(i) => Value::Number((*i).into()),
            Number::NegInt(i) => Value::Number((*i).into()),
            Number::Float(f) => Value::Number(
                serde_json::Number::from_f64(*f).ok_or(anyhow!("Expected a valid float"))?,
            ),
        },
        Document::String(str) => Value::String(str.clone()),
        Document::Array(arr) => {
            Value::Array(arr.iter().map(from_bedrock_json).collect::<Result<_>>()?)
        }
        Document::Object(obj) => Value::Object(
            obj.iter()
                .map(|(key, val)| Ok((key.clone(), from_bedrock_json(val)?)))
                .collect::<Result<_>>()?,
        ),
    })
}

/// State for accumulating tool call data across streaming deltas
#[derive(Debug, Default)]
struct ToolCallState {
    tool_use_id: String,
    name: String,
    input_json: String,
}

/// Process Bedrock streaming events into a stream of Messages
///
/// This function takes the event receiver from converse_stream and yields
/// Messages as content is received. Text content is yielded incrementally,
/// while tool calls are accumulated and yielded when complete.
pub fn stream_bedrock_response(
    mut event_receiver: aws_sdk_bedrockruntime::primitives::event_stream::EventReceiver<
        ConverseStreamOutput,
        aws_sdk_bedrockruntime::types::error::ConverseStreamOutputError,
    >,
    message_id: String,
    model_name: String,
) -> MessageStream {
    Box::pin(async_stream::stream! {
        let created = Utc::now().timestamp();

        // Track accumulated tool calls (indexed by content block index)
        let mut tool_calls: HashMap<i32, ToolCallState> = HashMap::new();
        let mut current_block_index: i32 = -1;

        // Track usage
        let mut usage: Option<Usage> = None;

        loop {
            let event = match event_receiver.recv().await {
                Ok(Some(event)) => event,
                Ok(None) => break, // Stream ended
                Err(e) => {
                    yield Err(ProviderError::RequestFailed(format!("Bedrock stream error: {:?}", e)));
                    break;
                }
            };

            match event {
                ConverseStreamOutput::MessageStart(_) => {
                    // Message started, role is always assistant
                }
                ConverseStreamOutput::ContentBlockStart(start) => {
                    current_block_index = start.content_block_index;

                    // Check if this is a tool use block
                    if let Some(bedrock::ContentBlockStart::ToolUse(tool_start)) = start.start {
                        tool_calls.insert(
                            current_block_index,
                            ToolCallState {
                                tool_use_id: tool_start.tool_use_id,
                                name: tool_start.name,
                                input_json: String::new(),
                            },
                        );
                    }
                }
                ConverseStreamOutput::ContentBlockDelta(delta) => {
                    current_block_index = delta.content_block_index;

                    if let Some(delta_content) = delta.delta {
                        match delta_content {
                            bedrock::ContentBlockDelta::Text(text) => {
                                // Yield text content immediately
                                let mut message = Message::assistant().with_text(&text);
                                message.id = Some(message_id.clone());
                                message.created = created;
                                yield Ok((Some(message), None));
                            }
                            bedrock::ContentBlockDelta::ToolUse(tool_delta) => {
                                // Accumulate tool input JSON
                                if let Some(tool_state) = tool_calls.get_mut(&current_block_index) {
                                    tool_state.input_json.push_str(&tool_delta.input);
                                }
                            }
                            bedrock::ContentBlockDelta::ReasoningContent(reasoning) => {
                                // Handle reasoning/thinking content (extended thinking)
                                // ReasoningContentBlockDelta is an enum with Text, Signature, RedactedContent variants
                                if let Ok(text) = reasoning.as_text() {
                                    let mut message = Message::assistant().with_thinking(
                                        text,
                                        "", // Signature comes as a separate delta
                                    );
                                    message.id = Some(message_id.clone());
                                    message.created = created;
                                    yield Ok((Some(message), None));
                                }
                                // Note: Signature and RedactedContent deltas are handled separately by the model
                            }
                            _ => {
                                // Other delta types (citations, images, etc.) - ignore for now
                            }
                        }
                    }
                }
                ConverseStreamOutput::ContentBlockStop(_) => {
                    // Content block finished - if it was a tool call, yield it now
                    if let Some(tool_state) = tool_calls.remove(&current_block_index) {
                        // Parse the accumulated JSON input
                        let arguments: Value = if tool_state.input_json.is_empty() {
                            Value::Object(serde_json::Map::new())
                        } else {
                            serde_json::from_str(&tool_state.input_json)
                                .unwrap_or(Value::Object(serde_json::Map::new()))
                        };

                        let mut message = Message::assistant().with_tool_request(
                            &tool_state.tool_use_id,
                            Ok(CallToolRequestParams {
                                meta: None,
                                task: None,
                                name: tool_state.name.into(),
                                arguments: Some(object(arguments)),
                            }),
                        );
                        message.id = Some(message_id.clone());
                        message.created = created;
                        yield Ok((Some(message), None));
                    }
                }
                ConverseStreamOutput::MessageStop(_) => {
                    // Message complete - any remaining tool calls should have been yielded
                }
                ConverseStreamOutput::Metadata(metadata) => {
                    // Extract usage information
                    if let Some(token_usage) = metadata.usage {
                        usage = Some(from_bedrock_usage(&token_usage));
                    }
                }
                _ => {
                    // Unknown event type - ignore
                }
            }
        }

        // Yield final usage if we have it
        if let Some(usage) = usage {
            yield Ok((None, Some(ProviderUsage::new(model_name, usage))));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use goose_test_support::TEST_IMAGE_B64;
    use rmcp::model::{AnnotateAble, RawImageContent};

    #[test]
    fn test_to_bedrock_image_supported_formats() -> Result<()> {
        let supported_formats = [
            "image/png",
            "image/jpeg",
            "image/jpg",
            "image/gif",
            "image/webp",
        ];

        for mime_type in supported_formats {
            let image = RawImageContent {
                data: TEST_IMAGE_B64.to_string(),
                mime_type: mime_type.to_string(),
                meta: None,
            }
            .no_annotation();

            let result = to_bedrock_image(&image.data, &image.mime_type);
            assert!(result.is_ok(), "Failed to convert {} format", mime_type);
        }

        Ok(())
    }

    #[test]
    fn test_to_bedrock_image_unsupported_format() {
        let image = RawImageContent {
            data: TEST_IMAGE_B64.to_string(),
            mime_type: "image/bmp".to_string(),
            meta: None,
        }
        .no_annotation();

        let result = to_bedrock_image(&image.data, &image.mime_type);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported image format: image/bmp"));
        assert!(error_msg.contains("Bedrock supports png, jpeg, gif, webp"));
    }

    #[test]
    fn test_to_bedrock_image_invalid_base64() {
        let image = RawImageContent {
            data: "invalid_base64_data!!!".to_string(),
            mime_type: "image/png".to_string(),
            meta: None,
        }
        .no_annotation();

        let result = to_bedrock_image(&image.data, &image.mime_type);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to decode base64 image data"));
    }

    #[test]
    fn test_to_bedrock_message_content_image() -> Result<()> {
        let image = RawImageContent {
            data: TEST_IMAGE_B64.to_string(),
            mime_type: "image/png".to_string(),
            meta: None,
        }
        .no_annotation();

        let message_content = MessageContent::Image(image);
        let result = to_bedrock_message_content(&message_content)?;

        // Verify we get an Image content block
        assert!(matches!(result, bedrock::ContentBlock::Image(_)));

        Ok(())
    }

    #[test]
    fn test_to_bedrock_tool_result_content_block_image() -> Result<()> {
        let content = Content::image(TEST_IMAGE_B64.to_string(), "image/png".to_string());
        let result = to_bedrock_tool_result_content_block("test_id", content)?;

        // Verify the wrapper correctly converts Content::Image to ToolResultContentBlock::Image
        assert!(matches!(result, bedrock::ToolResultContentBlock::Image(_)));

        Ok(())
    }
}
