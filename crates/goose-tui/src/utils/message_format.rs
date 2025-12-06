use goose::conversation::message::{Message, MessageContent};

pub fn message_to_plain_text(message: &Message) -> String {
    let mut lines = Vec::new();

    for content in &message.content {
        match content {
            MessageContent::Text(t) => {
                lines.push(t.text.clone());
            }
            MessageContent::ToolRequest(req) => {
                lines.push("Tool Request:".to_string());
                if let Ok(call) = &req.tool_call {
                    lines.push(format!("Name: {}", call.name));
                    if let Some(args) = &call.arguments {
                        if let Ok(json_str) = serde_json::to_string_pretty(args) {
                            lines.push(json_str);
                        }
                    }
                }
            }
            MessageContent::ToolResponse(resp) => {
                lines.push("Tool Output:".to_string());
                if let Ok(contents) = &resp.tool_result {
                    for content in contents {
                        if let Some(audience) = content.audience() {
                            if !audience.contains(&rmcp::model::Role::User) {
                                continue;
                            }
                        }
                        if let rmcp::model::Content {
                            raw: rmcp::model::RawContent::Text(text_content),
                            ..
                        } = content
                        {
                            let text = &text_content.text;
                            let display_string =
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| text.to_string())
                                } else {
                                    text.to_string()
                                };
                            lines.push(display_string);
                        }
                    }
                }
            }
            MessageContent::ToolConfirmationRequest(req) => {
                lines.push("Tool Confirmation Request:".to_string());
                lines.push(format!("Tool: {}", req.tool_name));
                if let Some(warning) = &req.prompt {
                    lines.push(format!("Warning: {warning}"));
                }
                lines.push("Arguments:".to_string());
                if let Ok(json_str) = serde_json::to_string_pretty(&req.arguments) {
                    lines.push(json_str);
                }
            }
            _ => {}
        }
        lines.push(String::new());
    }
    lines.join("\n")
}
