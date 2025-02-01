use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use aws_sdk_bedrockruntime::{types as bedrock, Client};
use aws_smithy_types::{Document, Number};
use chrono::Utc;
use mcp_core::{Content, Role, Tool, ToolCall, ToolError, ToolResult};
use serde_json::Value;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use crate::config::Config;
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;

pub const BEDROCK_DOC_LINK: &str =
    "https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html";

pub const BEDROCK_DEFAULT_MODEL: &str = "anthropic.claude-3-5-sonnet-20240620-v1:0";
pub const BEDROCK_KNOWN_MODELS: &[&str] = &[
    "anthropic.claude-3-5-sonnet-20240620-v1:0",
    "anthropic.claude-3-5-sonnet-20241022-v2:0",
];

#[derive(Debug, serde::Serialize)]
pub struct BedrockProvider {
    #[serde(skip)]
    client: Client,
    model: ModelConfig,
}

impl BedrockProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = Config::global();
        let sdk_config = tokio::task::block_in_place(|| {
            let mut aws_config = aws_config::from_env();

            if let Ok(region) = config.get::<String>("AWS_REGION") {
                aws_config = aws_config.region(aws_config::Region::new(region));
            }

            tokio::runtime::Handle::current().block_on(aws_config.load())
        });
        let client = Client::new(&sdk_config);

        Ok(Self { client, model })
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "bedrock",
            "Amazon Bedrock",
            "Run models through Amazon Bedrock",
            BEDROCK_DEFAULT_MODEL,
            BEDROCK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect(),
            BEDROCK_DOC_LINK,
            vec![ConfigKey::new("AWS_REGION", false, false, None)],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let model_name = &self.model.model_name;

        let response = self
            .client
            .converse()
            .tool_config(to_bedrock_tool_config(tools)?)
            .model_id(model_name.to_string())
            .system(bedrock::SystemContentBlock::Text(system.to_string()))
            .set_messages(Some(
                messages
                    .iter()
                    .map(to_bedrock_message)
                    .collect::<Result<_>>()?,
            ))
            .send()
            .await
            .map_err(|err| anyhow!("Failed to call Bedrock: {}", err))?;

        let message = match response.output {
            Some(bedrock::ConverseOutput::Message(message)) => message,
            _ => {
                return Err(ProviderError::RequestFailed(
                    "No output from Bedrock".to_string(),
                ))
            }
        };

        let usage = response
            .usage
            .as_ref()
            .map(from_bedrock_usage)
            .unwrap_or_default();

        let message = from_bedrock_message(&message)?;
        let provider_usage = ProviderUsage::new(model_name.to_string(), usage);

        Ok((message, provider_usage))
    }
}

fn to_bedrock_message(message: &Message) -> Result<bedrock::Message> {
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

fn to_bedrock_message_content(content: &MessageContent) -> Result<bedrock::ContentBlock> {
    Ok(match content {
        MessageContent::Text(text) => bedrock::ContentBlock::Text(text.text.to_string()),
        MessageContent::Image(_) => {
            bail!("Image content is not supported by Bedrock provider yet")
        }
        MessageContent::ToolRequest(tool_req) => {
            let tool_use_id = tool_req.id.to_string();
            let tool_use = if let Ok(call) = tool_req.tool_call.as_ref() {
                bedrock::ToolUseBlock::builder()
                    .tool_use_id(tool_use_id)
                    .name(call.name.to_string())
                    .input(to_bedrock_json(&call.arguments))
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
                Ok(content) => Some(
                    content
                        .iter()
                        .map(to_bedrock_tool_result_content_block)
                        .collect::<Result<_>>()?,
                ),
                Err(_) => None,
            };
            bedrock::ContentBlock::ToolResult(
                bedrock::ToolResultBlock::builder()
                    .tool_use_id(tool_res.id.to_string())
                    .status(if content.is_some() {
                        bedrock::ToolResultStatus::Success
                    } else {
                        bedrock::ToolResultStatus::Error
                    })
                    .set_content(content)
                    .build()?,
            )
        }
    })
}

fn to_bedrock_tool_result_content_block(
    content: &Content,
) -> Result<bedrock::ToolResultContentBlock> {
    Ok(match content {
        Content::Text(text) => bedrock::ToolResultContentBlock::Text(text.text.to_string()),
        Content::Image(_) => bail!("Image content is not supported by Bedrock provider yet"),
        Content::Resource(_) => bail!("Resource content is not supported by Bedrock provider yet"),
    })
}

fn to_bedrock_role(role: &Role) -> bedrock::ConversationRole {
    match role {
        Role::User => bedrock::ConversationRole::User,
        Role::Assistant => bedrock::ConversationRole::Assistant,
    }
}

fn to_bedrock_tool_config(tools: &[Tool]) -> Result<bedrock::ToolConfiguration> {
    Ok(bedrock::ToolConfiguration::builder()
        .set_tools(Some(
            tools.iter().map(to_bedrock_tool).collect::<Result<_>>()?,
        ))
        .build()?)
}

fn to_bedrock_tool(tool: &Tool) -> Result<bedrock::Tool> {
    Ok(bedrock::Tool::ToolSpec(
        bedrock::ToolSpecification::builder()
            .name(tool.name.to_string())
            .description(tool.description.to_string())
            .input_schema(bedrock::ToolInputSchema::Json(to_bedrock_json(
                &tool.input_schema,
            )))
            .build()?,
    ))
}

fn to_bedrock_json(value: &Value) -> Document {
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

fn from_bedrock_message(message: &bedrock::Message) -> Result<Message> {
    let role = from_bedrock_role(message.role())?;
    let content = message
        .content()
        .iter()
        .map(from_bedrock_content_block)
        .collect::<Result<Vec<_>>>()?;
    let created = Utc::now().timestamp();

    Ok(Message {
        role,
        content,
        created,
    })
}

fn from_bedrock_content_block(block: &bedrock::ContentBlock) -> Result<MessageContent> {
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
                Err(ToolError::ExecutionError(
                    "Empty content for tool use from Bedrock".to_string(),
                ))
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

fn from_bedrock_tool_result_content_block(
    content: &bedrock::ToolResultContentBlock,
) -> ToolResult<Content> {
    Ok(match content {
        bedrock::ToolResultContentBlock::Text(text) => Content::text(text.to_string()),
        _ => {
            return Err(ToolError::ExecutionError(
                "Unsupported tool result from Bedrock".to_string(),
            ))
        }
    })
}

fn from_bedrock_role(role: &bedrock::ConversationRole) -> Result<Role> {
    Ok(match role {
        bedrock::ConversationRole::User => Role::User,
        bedrock::ConversationRole::Assistant => Role::Assistant,
        _ => bail!("Unknown role from Bedrock"),
    })
}

fn from_bedrock_usage(usage: &bedrock::TokenUsage) -> Usage {
    Usage {
        input_tokens: Some(usage.input_tokens),
        output_tokens: Some(usage.output_tokens),
        total_tokens: Some(usage.total_tokens),
    }
}

fn from_bedrock_json(document: &Document) -> Result<Value> {
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
