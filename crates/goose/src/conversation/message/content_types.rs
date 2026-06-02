use super::{ProviderMetadata, ToolConfirmationRequest, ToolRequest, ToolResponse};
use crate::conversation::tool_result_serde;
use crate::mcp_utils::{extract_text_from_resource, ToolResult};
use rmcp::model::{
    AnnotateAble, CallToolRequestParams, CallToolResult, Content, ImageContent, JsonObject,
    RawContent, RawImageContent, RawTextContent, Role, TextContent,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "actionType", rename_all = "camelCase")]
pub enum ActionRequiredData {
    #[serde(rename_all = "camelCase")]
    ToolConfirmation {
        id: String,
        tool_name: String,
        arguments: JsonObject,
        prompt: Option<String>,
    },
    Elicitation {
        id: String,
        message: String,
        requested_schema: serde_json::Value,
    },
    ElicitationResponse {
        id: String,
        user_data: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionRequired {
    pub data: ActionRequiredData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ThinkingContent {
    pub thinking: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RedactedThinkingContent {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FrontendToolRequest {
    pub id: String,
    #[serde(with = "tool_result_serde")]
    #[schema(value_type = Object)]
    pub tool_call: ToolResult<CallToolRequestParams>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum SystemNotificationType {
    ThinkingMessage,
    InlineMessage,
    CreditsExhausted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SystemNotificationContent {
    pub notification_type: SystemNotificationType,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
/// Content passed inside a message, which can be both simple content and tool content
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolRequest(ToolRequest),
    ToolResponse(ToolResponse),
    ToolConfirmationRequest(ToolConfirmationRequest),
    ActionRequired(ActionRequired),
    FrontendToolRequest(FrontendToolRequest),
    Thinking(ThinkingContent),
    RedactedThinking(RedactedThinkingContent),
    SystemNotification(SystemNotificationContent),
}

impl fmt::Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageContent::Text(t) => write!(f, "{}", t.text),
            MessageContent::Image(i) => write!(f, "[Image: {}]", i.mime_type),
            MessageContent::ToolRequest(r) => {
                write!(f, "[ToolRequest: {}]", r.to_readable_string())
            }
            MessageContent::ToolResponse(r) => write!(
                f,
                "[ToolResponse: {}]",
                match &r.tool_result {
                    Ok(result) => format!("{} content item(s)", result.content.len()),
                    Err(e) => format!("Error: {e}"),
                }
            ),
            MessageContent::ToolConfirmationRequest(r) => {
                write!(f, "[ToolConfirmationRequest: {}]", r.tool_name)
            }
            MessageContent::ActionRequired(a) => match &a.data {
                ActionRequiredData::ToolConfirmation { tool_name, .. } => {
                    write!(f, "[ActionRequired: ToolConfirmation for {}]", tool_name)
                }
                ActionRequiredData::Elicitation { message, .. } => {
                    write!(f, "[ActionRequired: Elicitation - {}]", message)
                }
                ActionRequiredData::ElicitationResponse { id, .. } => {
                    write!(f, "[ActionRequired: ElicitationResponse for {}]", id)
                }
            },
            MessageContent::FrontendToolRequest(r) => match &r.tool_call {
                Ok(tool_call) => write!(f, "[FrontendToolRequest: {}]", tool_call.name),
                Err(e) => write!(f, "[FrontendToolRequest: Error: {}]", e),
            },
            MessageContent::Thinking(t) => write!(f, "[Thinking: {}]", t.thinking),
            MessageContent::RedactedThinking(_r) => write!(f, "[RedactedThinking]"),
            MessageContent::SystemNotification(r) => {
                write!(f, "[SystemNotification: {}]", r.msg)
            }
        }
    }
}

impl MessageContent {
    pub fn text<S: Into<String>>(text: S) -> Self {
        MessageContent::Text(
            RawTextContent {
                text: text.into(),
                meta: None,
            }
            .no_annotation(),
        )
    }

    pub fn filter_for_audience(&self, audience: Role) -> Option<MessageContent> {
        match self {
            MessageContent::Text(text) => {
                if text
                    .audience()
                    .map(|roles| roles.contains(&audience))
                    .unwrap_or(true)
                {
                    Some(self.clone())
                } else {
                    None
                }
            }
            MessageContent::Image(img) => {
                if img
                    .audience()
                    .map(|roles| roles.contains(&audience))
                    .unwrap_or(true)
                {
                    Some(self.clone())
                } else {
                    None
                }
            }
            MessageContent::ToolResponse(res) => {
                let Ok(result) = &res.tool_result else {
                    return Some(self.clone());
                };

                let filtered_content: Vec<Content> = result
                    .content
                    .iter()
                    .filter(|c| {
                        c.audience()
                            .map(|roles| roles.contains(&audience))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect();

                // Preserve ToolResponse even when content is empty - some providers
                // (like Google) need to handle empty tool responses specially
                let mut tool_result = result.clone();
                tool_result.content = filtered_content;
                Some(MessageContent::ToolResponse(ToolResponse {
                    id: res.id.clone(),
                    tool_result: Ok(tool_result),
                    metadata: res.metadata.clone(),
                }))
            }
            MessageContent::Thinking(_) | MessageContent::RedactedThinking(_) => {
                if audience == Role::Assistant {
                    Some(self.clone())
                } else {
                    None
                }
            }
            _ => Some(self.clone()),
        }
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        MessageContent::Image(
            RawImageContent {
                data: data.into(),
                mime_type: mime_type.into(),
                meta: None,
            }
            .no_annotation(),
        )
    }

    pub fn tool_request<S: Into<String>>(
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
    ) -> Self {
        MessageContent::ToolRequest(ToolRequest {
            id: id.into(),
            tool_call,
            metadata: None,
            tool_meta: None,
        })
    }

    pub fn tool_request_with_metadata<S: Into<String>>(
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
        metadata: Option<&ProviderMetadata>,
    ) -> Self {
        MessageContent::ToolRequest(ToolRequest {
            id: id.into(),
            tool_call,
            metadata: metadata.cloned(),
            tool_meta: None,
        })
    }

    pub fn tool_response<S: Into<String>>(id: S, tool_result: ToolResult<CallToolResult>) -> Self {
        MessageContent::ToolResponse(ToolResponse {
            id: id.into(),
            tool_result,
            metadata: None,
        })
    }

    pub fn tool_response_with_metadata<S: Into<String>>(
        id: S,
        tool_result: ToolResult<CallToolResult>,
        metadata: Option<&ProviderMetadata>,
    ) -> Self {
        MessageContent::ToolResponse(ToolResponse {
            id: id.into(),
            tool_result,
            metadata: metadata.cloned(),
        })
    }

    pub fn action_required<S: Into<String>>(
        id: S,
        tool_name: String,
        arguments: JsonObject,
        prompt: Option<String>,
    ) -> Self {
        MessageContent::ActionRequired(ActionRequired {
            data: ActionRequiredData::ToolConfirmation {
                id: id.into(),
                tool_name,
                arguments,
                prompt,
            },
        })
    }

    pub fn action_required_elicitation<S: Into<String>>(
        id: S,
        message: String,
        requested_schema: serde_json::Value,
    ) -> Self {
        MessageContent::ActionRequired(ActionRequired {
            data: ActionRequiredData::Elicitation {
                id: id.into(),
                message,
                requested_schema,
            },
        })
    }

    pub fn action_required_elicitation_response<S: Into<String>>(
        id: S,
        user_data: serde_json::Value,
    ) -> Self {
        MessageContent::ActionRequired(ActionRequired {
            data: ActionRequiredData::ElicitationResponse {
                id: id.into(),
                user_data,
            },
        })
    }

    pub fn thinking<S1: Into<String>, S2: Into<String>>(thinking: S1, signature: S2) -> Self {
        MessageContent::Thinking(ThinkingContent {
            thinking: thinking.into(),
            signature: signature.into(),
        })
    }

    pub fn redacted_thinking<S: Into<String>>(data: S) -> Self {
        MessageContent::RedactedThinking(RedactedThinkingContent { data: data.into() })
    }

    pub fn frontend_tool_request<S: Into<String>>(
        id: S,
        tool_call: ToolResult<CallToolRequestParams>,
    ) -> Self {
        MessageContent::FrontendToolRequest(FrontendToolRequest {
            id: id.into(),
            tool_call,
        })
    }

    pub fn system_notification<S: Into<String>>(
        notification_type: SystemNotificationType,
        msg: S,
    ) -> Self {
        MessageContent::SystemNotification(SystemNotificationContent {
            notification_type,
            msg: msg.into(),
            data: None,
        })
    }

    pub fn system_notification_with_data<S: Into<String>>(
        notification_type: SystemNotificationType,
        msg: S,
        data: serde_json::Value,
    ) -> Self {
        MessageContent::SystemNotification(SystemNotificationContent {
            notification_type,
            msg: msg.into(),
            data: Some(data),
        })
    }

    pub fn as_system_notification(&self) -> Option<&SystemNotificationContent> {
        if let MessageContent::SystemNotification(ref notification) = self {
            Some(notification)
        } else {
            None
        }
    }

    pub fn as_tool_request(&self) -> Option<&ToolRequest> {
        if let MessageContent::ToolRequest(ref tool_request) = self {
            Some(tool_request)
        } else {
            None
        }
    }

    pub fn as_tool_response(&self) -> Option<&ToolResponse> {
        if let MessageContent::ToolResponse(ref tool_response) = self {
            Some(tool_response)
        } else {
            None
        }
    }

    pub fn as_action_required(&self) -> Option<&ActionRequired> {
        if let MessageContent::ActionRequired(ref action_required) = self {
            Some(action_required)
        } else {
            None
        }
    }

    pub fn as_tool_response_text(&self) -> Option<String> {
        if let Some(tool_response) = self.as_tool_response() {
            if let Ok(result) = &tool_response.tool_result {
                let texts: Vec<String> = result
                    .content
                    .iter()
                    .filter_map(|content| content.as_text().map(|t| t.text.to_string()))
                    .collect();
                if !texts.is_empty() {
                    return Some(texts.join("\n"));
                }
            }
        }
        None
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(text) => Some(&text.text),
            _ => None,
        }
    }

    /// Get the thinking content if this is a ThinkingContent variant
    pub fn as_thinking(&self) -> Option<&ThinkingContent> {
        match self {
            MessageContent::Thinking(thinking) => Some(thinking),
            _ => None,
        }
    }

    /// Get the redacted thinking content if this is a RedactedThinkingContent variant
    pub fn as_redacted_thinking(&self) -> Option<&RedactedThinkingContent> {
        match self {
            MessageContent::RedactedThinking(redacted) => Some(redacted),
            _ => None,
        }
    }
}

impl From<Content> for MessageContent {
    fn from(content: Content) -> Self {
        match content.raw {
            RawContent::Text(text) => {
                MessageContent::Text(text.optional_annotate(content.annotations))
            }
            RawContent::Image(image) => {
                MessageContent::Image(image.optional_annotate(content.annotations))
            }
            RawContent::ResourceLink(_link) => MessageContent::text("[Resource link]"),
            RawContent::Resource(resource) => {
                MessageContent::text(extract_text_from_resource(&resource.resource))
            }
            RawContent::Audio(_) => {
                MessageContent::text("[Audio content: not supported]".to_string())
            }
        }
    }
}
