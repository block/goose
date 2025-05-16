use goose::message::{Message, MessageContent, ToolRequest, ToolResponse};
use mcp_core::content::Content as McpContent;
use mcp_core::resource::ResourceContents;
use mcp_core::role::Role;
use serde_json::Value;

const MAX_STRING_LENGTH_MD_EXPORT: usize = 4096; // Generous limit for export

fn value_to_simple_markdown_string(value: &Value, export_full_strings: bool) -> String {
    match value {
        Value::String(s) => {
            if !export_full_strings && s.len() > MAX_STRING_LENGTH_MD_EXPORT {
                format!("`[REDACTED: {} chars]`", s.len())
            } else {
                // Escape backticks and newlines for inline code.
                let escaped = s.replace('`', "\\`").replace("\n", "\\\\n");
                format!("`{}`", escaped)
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => format!("*{}*", b),
        Value::Null => "_null_".to_string(),
        _ => "`[Complex Value]`".to_string(),
    }
}

fn value_to_markdown(value: &Value, depth: usize, export_full_strings: bool) -> String {
    let mut md_string = String::new();
    let base_indent_str = "  ".repeat(depth); // Basic indentation for nesting

    match value {
        Value::Object(map) => {
            if map.is_empty() {
                md_string.push_str(&format!("{}*empty object*\n", base_indent_str));
            } else {
                for (key, val) in map {
                    md_string.push_str(&format!("{}*   **{}**: ", base_indent_str, key));
                    match val {
                        Value::String(s) => {
                            if s.contains('\n') || s.len() > 80 {
                                // Heuristic for block
                                md_string.push_str(&format!(
                                    "\n{}    ```\n{}{}\n{}    ```\n",
                                    base_indent_str,
                                    base_indent_str,
                                    s.trim(),
                                    base_indent_str
                                ));
                            } else {
                                md_string.push_str(&format!("`{}`\n", s.replace('`', "\\`")));
                            }
                        }
                        Value::Object(_) | Value::Array(_) => {
                            md_string.push_str("\n");
                            md_string.push_str(&value_to_markdown(
                                val,
                                depth + 2,
                                export_full_strings,
                            ));
                        }
                        _ => {
                            md_string.push_str(&format!(
                                "{}\n",
                                value_to_simple_markdown_string(val, export_full_strings)
                            ));
                        }
                    }
                }
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                md_string.push_str(&format!("{}*   *empty list*\n", base_indent_str));
            } else {
                for item in arr {
                    md_string.push_str(&format!("{}*   - ", base_indent_str));
                    match item {
                        Value::String(s) => {
                            if s.contains('\n') || s.len() > 80 {
                                // Heuristic for block
                                md_string.push_str(&format!(
                                    "\n{}      ```\n{}{}\n{}      ```\n",
                                    base_indent_str,
                                    base_indent_str,
                                    s.trim(),
                                    base_indent_str
                                ));
                            } else {
                                md_string.push_str(&format!("`{}`\n", s.replace('`', "\\`")));
                            }
                        }
                        Value::Object(_) | Value::Array(_) => {
                            md_string.push_str("\n");
                            md_string.push_str(&value_to_markdown(
                                item,
                                depth + 2,
                                export_full_strings,
                            ));
                        }
                        _ => {
                            md_string.push_str(&format!(
                                "{}\n",
                                value_to_simple_markdown_string(item, export_full_strings)
                            ));
                        }
                    }
                }
            }
        }
        _ => {
            md_string.push_str(&format!(
                "{}{}\n",
                base_indent_str,
                value_to_simple_markdown_string(value, export_full_strings)
            ));
        }
    }
    md_string
}

pub fn tool_request_to_markdown(req: &ToolRequest, export_all_content: bool) -> String {
    let mut md = String::new();
    match &req.tool_call {
        Ok(call) => {
            let parts: Vec<_> = call.name.rsplitn(2, "__").collect();
            let (namespace, tool_name_only) = if parts.len() == 2 {
                (parts[1], parts[0])
            } else {
                ("Tool", parts[0])
            };

            md.push_str(&format!(
                "#### Tool Call: `{}` (namespace: `{}`)\n",
                tool_name_only, namespace
            ));
            md.push_str("**Arguments:**\n");

            match call.name.as_str() {
                "developer__shell" => {
                    if let Some(Value::String(command)) = call.arguments.get("command") {
                        md.push_str(&format!(
                            "*   **command**:\n    ```sh\n    {}\n    ```\n",
                            command.trim()
                        ));
                    }
                    let other_args: serde_json::Map<String, Value> = call
                        .arguments
                        .as_object()
                        .map(|obj| {
                            obj.iter()
                                .filter(|(k, _)| k.as_str() != "command")
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if !other_args.is_empty() {
                        md.push_str(&value_to_markdown(
                            &Value::Object(other_args),
                            0,
                            export_all_content,
                        ));
                    }
                }
                "developer__text_editor" => {
                    if let Some(Value::String(path)) = call.arguments.get("path") {
                        md.push_str(&format!("*   **path**: `{}`\n", path));
                    }
                    if let Some(Value::String(code_edit)) = call.arguments.get("code_edit") {
                        md.push_str(&format!(
                            "*   **code_edit**:\n    ```\n{}\n    ```\n",
                            code_edit
                        ));
                    }

                    let other_args: serde_json::Map<String, Value> = call
                        .arguments
                        .as_object()
                        .map(|obj| {
                            obj.iter()
                                .filter(|(k, _)| k.as_str() != "path" && k.as_str() != "code_edit")
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if !other_args.is_empty() {
                        md.push_str(&value_to_markdown(
                            &Value::Object(other_args),
                            0,
                            export_all_content,
                        ));
                    }
                }
                _ => {
                    md.push_str(&value_to_markdown(&call.arguments, 0, export_all_content));
                }
            }
        }
        Err(e) => {
            md.push_str(&format!(
                "**Error in Tool Call:**\n```\n{}
```\n",
                e
            ));
        }
    }
    md
}

pub fn tool_response_to_markdown(resp: &ToolResponse, export_all_content: bool) -> String {
    let mut md = String::new();
    md.push_str("#### Tool Response:\n");

    match &resp.tool_result {
        Ok(contents) => {
            if contents.is_empty() {
                md.push_str("*No textual output from tool.*\n");
            }

            for content in contents {
                if !export_all_content {
                    if let Some(audience) = content.audience() {
                        if !audience.contains(&Role::Assistant) {
                            continue;
                        }
                    }
                }

                match content {
                    McpContent::Text(text_content) => {
                        let trimmed_text = text_content.text.trim();
                        if (trimmed_text.starts_with('{') && trimmed_text.ends_with('}'))
                            || (trimmed_text.starts_with('[') && trimmed_text.ends_with(']'))
                        {
                            md.push_str(&format!("```json\n{}\n```\n", trimmed_text));
                        } else if trimmed_text.starts_with('<')
                            && trimmed_text.ends_with('>')
                            && trimmed_text.contains("</")
                        {
                            md.push_str(&format!("```xml\n{}\n```\n", trimmed_text));
                        } else {
                            md.push_str(&text_content.text);
                            md.push_str("\n\n");
                        }
                    }
                    McpContent::Image(image_content) => {
                        if image_content.mime_type.starts_with("image/") {
                            // For actual images, provide a placeholder that indicates it's an image
                            md.push_str(&format!(
                                "**Image:** `(type: {}, data: first 30 chars of base64...)`\n\n",
                                image_content.mime_type
                            ));
                        } else {
                            // For non-image mime types, just indicate it's binary data
                            md.push_str(&format!(
                                "**Binary Content:** `(type: {}, length: {} bytes)`\n\n",
                                image_content.mime_type,
                                image_content.data.len()
                            ));
                        }
                    }
                    McpContent::Resource(resource) => {
                        match &resource.resource {
                            ResourceContents::TextResourceContents {
                                uri,
                                mime_type,
                                text,
                            } => {
                                // Extract file extension from the URI for syntax highlighting
                                let file_extension = uri.split('.').last().unwrap_or("");
                                let syntax_type = match file_extension {
                                    "rs" => "rust",
                                    "js" => "javascript",
                                    "ts" => "typescript",
                                    "py" => "python",
                                    "json" => "json",
                                    "yaml" | "yml" => "yaml",
                                    "md" => "markdown",
                                    "html" => "html",
                                    "css" => "css",
                                    "sh" => "bash",
                                    _ => mime_type
                                        .as_ref()
                                        .map(|mime| if mime == "text" { "" } else { mime })
                                        .unwrap_or(""),
                                };

                                md.push_str(&format!("**File:** `{}`\n", uri));
                                md.push_str(&format!(
                                    "```{}\n{}\n```\n\n",
                                    syntax_type,
                                    text.trim()
                                ));
                            }
                            ResourceContents::BlobResourceContents {
                                uri,
                                mime_type,
                                blob,
                            } => {
                                md.push_str(&format!(
                                    "**Binary File:** `{}` (type: {}, {} bytes)\n\n",
                                    uri,
                                    mime_type.as_ref().map(|s| s.as_str()).unwrap_or("unknown"),
                                    blob.len()
                                ));
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            md.push_str(&format!(
                "**Error in Tool Response:**\n```\n{}
```\n",
                e
            ));
        }
    }
    md
}

pub fn message_to_markdown(message: &Message, export_all_content: bool) -> String {
    let mut md = String::new();
    for content in &message.content {
        match content {
            MessageContent::Text(text) => {
                md.push_str(&text.text);
                md.push_str("\n\n");
            }
            MessageContent::ToolRequest(req) => {
                md.push_str(&tool_request_to_markdown(req, export_all_content));
                md.push_str("\n");
            }
            MessageContent::ToolResponse(resp) => {
                md.push_str(&tool_response_to_markdown(resp, export_all_content));
                md.push_str("\n");
            }
            MessageContent::Image(image) => {
                md.push_str(&format!(
                    "**Image:** `(type: {}, data placeholder: {}...)`\n\n",
                    image.mime_type,
                    image.data.chars().take(30).collect::<String>()
                ));
            }
            MessageContent::Thinking(thinking) => {
                md.push_str("**Thinking:**\n");
                md.push_str("> ");
                md.push_str(&thinking.thinking.replace("\n", "\n> "));
                md.push_str("\n\n");
            }
            MessageContent::RedactedThinking(_) => {
                md.push_str("**Thinking:**\n");
                md.push_str("> *Thinking was redacted*\n\n");
            }
            _ => {
                md.push_str(
                    "`WARNING: Message content type could not be rendered to Markdown`\n\n",
                );
            }
        }
    }
    md.trim_end_matches("\n").to_string()
}
