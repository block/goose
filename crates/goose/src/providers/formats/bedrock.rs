use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use aws_sdk_bedrockruntime::types as bedrock;
use aws_smithy_types::{Document, Number};
use base64::Engine;
use chrono::Utc;
use mcp_core::{ToolCall, ToolResult};
use rmcp::model::{Content, ErrorCode, ErrorData, RawContent, ResourceContents, Role, Tool};
use serde_json::Value;

use super::super::base::Usage;
use crate::conversation::message::{Message, MessageContent};
use std::collections::HashSet;

/// Sanitize a conversation for Bedrock requirements:
/// - Remove assistant tool requests that don't have a matching ToolResponse later in the visible history
/// - Drop frontend tool requests (not for LLM)
pub fn sanitize_messages_for_bedrock(messages: &[Message]) -> Vec<Message> {
    let mut seen_tool_results: HashSet<String> = HashSet::new();
    let mut out_rev: Vec<Message> = Vec::with_capacity(messages.len());

    for msg in messages.iter().rev() {
        let mut new_msg = msg.clone();
        let mut new_content: Vec<MessageContent> = Vec::with_capacity(msg.content.len());

        for c in &msg.content {
            match c {
                MessageContent::ToolResponse(res) => {
                    seen_tool_results.insert(res.id.clone());
                    new_content.push(c.clone());
                }
                MessageContent::ToolRequest(req) => {
                    // Keep only if we will include a matching ToolResponse later in the list
                    if seen_tool_results.contains(req.id.as_str()) {
                        new_content.push(c.clone());
                    } else {
                        // drop orphaned tool request
                    }
                }
                MessageContent::FrontendToolRequest(_) => {
                    // Always drop frontend tool requests for provider input
                }
                _ => new_content.push(c.clone()),
            }
        }

        new_msg.content = new_content;
        out_rev.push(new_msg);
    }

    out_rev.reverse();
    out_rev
}

pub fn to_bedrock_message(message: &Message) -> Result<Option<bedrock::Message>> {
    // Build content blocks, skipping unsupported/empty ones
    let mut blocks: Vec<bedrock::ContentBlock> = Vec::new();
    for c in &message.content {
        if let Some(block) = to_bedrock_message_content(c)? {
            blocks.push(block);
        }
    }

    // If nothing remains after filtering, skip this message entirely
    if blocks.is_empty() {
        return Ok(None);
    }

    Ok(Some(
        bedrock::Message::builder()
            .role(to_bedrock_role(&message.role))
            .set_content(Some(blocks))
            .build()
            .map_err(|err| anyhow!("Failed to construct Bedrock message: {}", err))?,
    ))
}

pub fn to_bedrock_message_content(
    content: &MessageContent,
) -> Result<Option<bedrock::ContentBlock>> {
    Ok(match content {
        MessageContent::Text(text) => {
            let t = text.text.trim();
            if t.is_empty() {
                None
            } else {
                Some(bedrock::ContentBlock::Text(t.to_string()))
            }
        }
        MessageContent::ToolConfirmationRequest(_) => None, // skip
        MessageContent::Image(image) => Some(bedrock::ContentBlock::Image(to_bedrock_image(
            &image.data,
            &image.mime_type,
        )?)),
        MessageContent::Thinking(_) => None,         // skip
        MessageContent::RedactedThinking(_) => None, // skip
        MessageContent::ContextLengthExceeded(_) => {
            bail!("ContextLengthExceeded should not get passed to the provider")
        }
        MessageContent::SummarizationRequested(_) => {
            bail!("SummarizationRequested should not get passed to the provider")
        }
        MessageContent::ToolRequest(tool_req) => {
            // Only send valid tool requests with a name and arguments
            if let Ok(call) = tool_req.tool_call.as_ref() {
                let tool_use = bedrock::ToolUseBlock::builder()
                    .tool_use_id(tool_req.id.to_string())
                    .name(call.name.to_string())
                    .input(to_bedrock_json(&call.arguments))
                    .build()?;
                Some(bedrock::ContentBlock::ToolUse(tool_use))
            } else {
                None
            }
        }
        MessageContent::FrontendToolRequest(tool_req) => {
            // Only send valid tool requests with a name and arguments
            if let Ok(call) = tool_req.tool_call.as_ref() {
                let tool_use = bedrock::ToolUseBlock::builder()
                    .tool_use_id(tool_req.id.to_string())
                    .name(call.name.to_string())
                    .input(to_bedrock_json(&call.arguments))
                    .build()?;
                Some(bedrock::ContentBlock::ToolUse(tool_use))
            } else {
                None
            }
        }
        MessageContent::ToolResponse(tool_res) => {
            let content = match &tool_res.tool_result {
                Ok(content) => Some(
                    content
                        .iter()
                        // Filter out content items that have User in their audience
                        .filter(|c| {
                            c.audience()
                                .is_none_or(|audience| !audience.contains(&Role::User))
                        })
                        .map(|c| to_bedrock_tool_result_content_block(&tool_res.id, c.clone()))
                        .collect::<Result<_>>()?,
                ),
                Err(_) => None,
            };
            Some(bedrock::ContentBlock::ToolResult(
                bedrock::ToolResultBlock::builder()
                    .tool_use_id(tool_res.id.to_string())
                    .status(if content.is_some() {
                        bedrock::ToolResultStatus::Success
                    } else {
                        bedrock::ToolResultStatus::Error
                    })
                    .set_content(content)
                    .build()?,
            ))
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
        RawContent::Audio(..) => bail!("Audio is not not supported by Bedrock provider"),
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
                &Value::Object(tool.input_schema.as_ref().clone()),
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
            Ok(ToolCall::new(
                tool_use.name.to_string(),
                from_bedrock_json(&tool_use.input)?,
            )),
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
    Usage {
        input_tokens: Some(usage.input_tokens),
        output_tokens: Some(usage.output_tokens),
        total_tokens: Some(usage.total_tokens),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rmcp::model::{
        AnnotateAble, Content as McpContent, RawContent as McpRawContent, RawEmbeddedResource,
        RawImageContent, ResourceContents, Role as McpRole,
    };
    use serde_json::json;

    // Base64 encoded 1x1 PNG image for testing
    const TEST_IMAGE_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

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
                data: TEST_IMAGE_BASE64.to_string(),
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
            data: TEST_IMAGE_BASE64.to_string(),
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
            data: TEST_IMAGE_BASE64.to_string(),
            mime_type: "image/png".to_string(),
            meta: None,
        }
        .no_annotation();

        let message_content = MessageContent::Image(image);
        let result = to_bedrock_message_content(&message_content)?;

        // Verify we get an Image content block
        assert!(matches!(result, Some(bedrock::ContentBlock::Image(_))));

        Ok(())
    }

    #[test]
    fn test_to_bedrock_tool_result_content_block_image() -> Result<()> {
        let content = Content::image(TEST_IMAGE_BASE64.to_string(), "image/png".to_string());
        let result = to_bedrock_tool_result_content_block("test_id", content)?;

        // Verify the wrapper correctly converts Content::Image to ToolResultContentBlock::Image
        assert!(matches!(result, bedrock::ToolResultContentBlock::Image(_)));

        Ok(())
    }

    #[test]
    fn test_skip_empty_text_block() -> Result<()> {
        let msg = Message::assistant().with_text("");
        let mapped = to_bedrock_message(&msg)?;
        assert!(
            mapped.is_none(),
            "empty text-only message should be skipped"
        );
        Ok(())
    }

    #[test]
    fn test_skip_thinking_and_confirmation_blocks() -> Result<()> {
        let msg = Message::assistant()
            .with_thinking("internal chain of thought", "sig")
            .with_tool_confirmation_request(
                "id1",
                "tool".to_string(),
                serde_json::json!({"a":1}),
                None,
            );
        let mapped = to_bedrock_message(&msg)?;
        assert!(
            mapped.is_none(),
            "unsupported content should result in skipping message"
        );
        Ok(())
    }

    #[test]
    fn test_sanitize_drops_orphan_tool_request() -> Result<()> {
        let tool_call = ToolCall::new("echo".to_string(), json!({"text": "hi"}));

        // Assistant issued a tool request that never received a response
        let msg = Message::assistant().with_tool_request("orphan", Ok(tool_call));

        let sanitized = sanitize_messages_for_bedrock(&[msg.clone()]);
        assert_eq!(sanitized.len(), 1);
        // The request should be removed from content, leaving the message empty
        assert!(sanitized[0].content.is_empty());
        Ok(())
    }

    #[test]
    fn test_sanitize_keeps_tool_request_with_matching_response() -> Result<()> {
        let id = "req-1";
        let call = ToolCall::new("do_work".to_string(), json!({"x": 1}));
        let req = Message::assistant().with_tool_request(id, Ok(call));
        let res = Message::user()
            .with_tool_response(id, Ok(vec![McpContent::text("ok")]))
            .with_id("later");

        // When a matching ToolResponse exists later in history, keep the request
        let sanitized = sanitize_messages_for_bedrock(&[req.clone(), res.clone()]);
        assert_eq!(sanitized.len(), 2);
        assert!(sanitized[0]
            .content
            .iter()
            .any(|c| c.as_tool_request().is_some()));
        assert!(sanitized[1]
            .content
            .iter()
            .any(|c| c.as_tool_response().is_some()));
        Ok(())
    }

    #[test]
    fn test_sanitize_drops_frontend_tool_request() -> Result<()> {
        let call = ToolCall::new("ui_only".to_string(), json!({}));
        let msg = Message::assistant().with_frontend_tool_request("f-1", Ok(call));

        let sanitized = sanitize_messages_for_bedrock(&[msg]);
        assert_eq!(sanitized.len(), 1);
        assert!(
            sanitized[0].content.is_empty(),
            "Frontend tool requests should be stripped"
        );
        Ok(())
    }

    #[test]
    fn test_from_bedrock_content_block_tool_use_and_result() -> Result<()> {
        // ToolUse -> ToolRequest
        let tool_use = bedrock::ToolUseBlock::builder()
            .tool_use_id("id-1")
            .name("sum")
            .input(to_bedrock_json(&json!({"a": 1, "b": 2})))
            .build()?;
        let req_block = bedrock::ContentBlock::ToolUse(tool_use);
        let req = from_bedrock_content_block(&req_block)?;
        match req {
            MessageContent::ToolRequest(r) => {
                let call = r.tool_call.unwrap();
                assert_eq!(call.name, "sum");
                assert_eq!(call.arguments, json!({"a": 1, "b": 2}));
            }
            _ => panic!("Expected ToolRequest"),
        }

        // ToolResult with empty content -> error
        let tool_res_empty = bedrock::ToolResultBlock::builder()
            .tool_use_id("id-1")
            .status(bedrock::ToolResultStatus::Success)
            .set_content(Some(vec![]))
            .build()?;
        let res_block = bedrock::ContentBlock::ToolResult(tool_res_empty);
        let res = from_bedrock_content_block(&res_block)?;
        match res {
            MessageContent::ToolResponse(r) => {
                assert!(r.tool_result.is_err(), "Empty content should map to error");
            }
            _ => panic!("Expected ToolResponse"),
        }

        // ToolResult with text content -> Ok(Vec<Content::Text>)
        let tool_res_ok = bedrock::ToolResultBlock::builder()
            .tool_use_id("id-1")
            .status(bedrock::ToolResultStatus::Success)
            .set_content(Some(vec![bedrock::ToolResultContentBlock::Text(
                "hello".into(),
            )]))
            .build()?;
        let res_ok_block = bedrock::ContentBlock::ToolResult(tool_res_ok);
        let res_ok = from_bedrock_content_block(&res_ok_block)?;
        match res_ok {
            MessageContent::ToolResponse(r) => {
                let contents = r.tool_result.unwrap();
                assert_eq!(contents.len(), 1);
                assert_eq!(contents[0].as_text().unwrap().text, "hello");
            }
            _ => panic!("Expected ToolResponse with Ok result"),
        }

        Ok(())
    }

    #[test]
    fn test_to_bedrock_tool_result_content_block_document_and_fallback() -> Result<()> {
        // Supported extension -> Document
        let text_res = ResourceContents::TextResourceContents {
            uri: "file:///docs/readme.md".to_string(),
            mime_type: Some("text/markdown".to_string()),
            text: "# Title".to_string(),
            meta: None,
        };
        let content_doc: McpContent = McpRawContent::Resource(RawEmbeddedResource {
            resource: text_res.clone(),
            meta: None,
        })
        .no_annotation();
        let block = to_bedrock_tool_result_content_block("tid", content_doc)?;
        assert!(matches!(
            block,
            bedrock::ToolResultContentBlock::Document(_)
        ));

        // Unsupported extension -> fallback to Text
        let json_res = ResourceContents::TextResourceContents {
            uri: "file:///data/data.json".to_string(),
            mime_type: Some("application/json".to_string()),
            text: "{\"x\":1}".to_string(),
            meta: None,
        };
        let content_text: McpContent = McpRawContent::Resource(RawEmbeddedResource {
            resource: json_res,
            meta: None,
        })
        .no_annotation();
        let block2 = to_bedrock_tool_result_content_block("tid", content_text)?;
        assert!(matches!(block2, bedrock::ToolResultContentBlock::Text(_)));

        Ok(())
    }

    #[test]
    fn test_bedrock_message_roundtrip_parts() -> Result<()> {
        // Build a Bedrock message with text and tool use
        let mut blocks: Vec<bedrock::ContentBlock> = Vec::new();
        blocks.push(bedrock::ContentBlock::Text("hi".to_string()));
        blocks.push(bedrock::ContentBlock::ToolUse(
            bedrock::ToolUseBlock::builder()
                .tool_use_id("x")
                .name("echo")
                .input(to_bedrock_json(&json!({"t":"hi"})))
                .build()?,
        ));

        let bedrock_msg = bedrock::Message::builder()
            .role(bedrock::ConversationRole::Assistant)
            .set_content(Some(blocks))
            .build()?;

        let converted = from_bedrock_message(&bedrock_msg)?;
        assert_eq!(converted.role, McpRole::Assistant);
        assert_eq!(converted.content.len(), 2);
        assert!(matches!(converted.content[0], MessageContent::Text(_)));
        assert!(matches!(
            converted.content[1],
            MessageContent::ToolRequest(_)
        ));
        Ok(())
    }

    #[test]
    fn test_json_roundtrip() -> Result<()> {
        let value = json!({
            "a": [1, 2, {"b": true, "c": 3.14}],
            "d": null,
            "e": {"x": "y"}
        });

        let doc = to_bedrock_json(&value);
        let back = from_bedrock_json(&doc)?;
        assert_eq!(back, value);
        Ok(())
    }
}
