use crate::agents::capabilities::Capabilities;
use crate::message::{Message, MessageContent, ToolRequest};
use chrono::Utc;
use indoc::indoc;
use mcp_core::{tool::Tool, TextContent};
use serde_json::{json, Value};

/// Creates the tool definition for checking read-only permissions.
fn create_read_only_tool() -> Tool {
    Tool::new(
        "platform__tool_by_tool_permission".to_string(),
        indoc! {r#"
            List tool names which try to do read-only operations in the tool requests.

            This tool examines all available tool requests and returns a list of tool names that have read-only operations.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "properties": {
                "read_only_tools": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Optional list of tool names which has read-only operations."
                }
            },
            "required": []
        }),
    )
}

/// Builds the message to be sent to the LLM for detecting read-only operations.
fn create_check_messages(tool_requests: Vec<&ToolRequest>) -> Vec<Message> {
    let mut check_messages = vec![];
    check_messages.push(Message {
        role: mcp_core::Role::User,
        created: Utc::now().timestamp(),
        content: vec![MessageContent::Text(TextContent {
            text: format!(
                "Here are the tool requests: {:?}\n\nDetect and list the tools with read-only operations.",
                tool_requests,
            ),
            annotations: None,
        })],
    });
    check_messages
}

/// Processes the response to extract the list of tools with read-only operations.
fn extract_read_only_tools(response: &Message) -> Option<Vec<String>> {
    for content in &response.content {
        if let MessageContent::ToolRequest(tool_request) = content {
            if let Ok(tool_call) = &tool_request.tool_call {
                if tool_call.name == "platform__tool_by_tool_permission" {
                    if let Value::Object(arguments) = &tool_call.arguments {
                        if let Some(Value::Array(read_only_tools)) =
                            arguments.get("read_only_tools")
                        {
                            return Some(
                                read_only_tools
                                    .iter()
                                    .filter_map(|tool| tool.as_str().map(String::from))
                                    .collect(),
                            );
                        }
                    }
                }
            }
        }
    }
    None
}

/// Executes the read-only tools detection and returns the list of tools with read-only operations.
pub async fn detect_read_only_tools(
    capabilities: &Capabilities,
    tool_requests: Vec<&ToolRequest>,
) -> Vec<String> {
    let tool = create_read_only_tool();
    let check_messages = create_check_messages(tool_requests);

    let res = capabilities
        .provider()
        .complete(
            "You are a good analyst and can detect operations whether they have read-only operations.",
            &check_messages,
            &[tool.clone()],
        )
        .await;

    // Process the response and return an empty vector if the response is invalid
    if let Ok((message, _usage)) = res {
        extract_read_only_tools(&message).unwrap_or_default()
    } else {
        vec![]
    }
}
