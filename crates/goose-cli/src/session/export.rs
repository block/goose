use goose::conversation::message::{Message, MessageContent, ToolRequest, ToolResponse};
use goose::utils::safe_truncate;
use rmcp::model::{RawContent, ResourceContents, Role};
use serde_json::Value;

const MAX_STRING_LENGTH_MD_EXPORT: usize = 4096; // Generous limit for export
const REDACTED_PREFIX_LENGTH: usize = 100; // Show first 100 chars before trimming

fn value_to_simple_markdown_string(value: &Value, export_full_strings: bool) -> String {
    match value {
        Value::String(s) => {
            if !export_full_strings && s.chars().count() > MAX_STRING_LENGTH_MD_EXPORT {
                let prefix = safe_truncate(s, REDACTED_PREFIX_LENGTH);
                let trimmed_chars = s.chars().count() - prefix.chars().count();
                format!("`{}[ ... trimmed : {} chars ... ]`", prefix, trimmed_chars)
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
                            if s.contains('\n') || s.chars().count() > 80 {
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
                        _ => {
                            // Use recursive call for all values including complex objects/arrays
                            md_string.push('\n');
                            md_string.push_str(&value_to_markdown(
                                val,
                                depth + 2,
                                export_full_strings,
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
                            if s.contains('\n') || s.chars().count() > 80 {
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
                        _ => {
                            // Use recursive call for all values including complex objects/arrays
                            md_string.push('\n');
                            md_string.push_str(&value_to_markdown(
                                item,
                                depth + 2,
                                export_full_strings,
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

                match &content.raw {
                    RawContent::Text(text_content) => {
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
                    RawContent::Image(image_content) => {
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
                    RawContent::Resource(resource) => {
                        match &resource.resource {
                            ResourceContents::TextResourceContents {
                                uri,
                                mime_type,
                                text,
                            } => {
                                // Extract file extension from the URI for syntax highlighting
                                let file_extension = uri.split('.').next_back().unwrap_or("");
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
                    RawContent::Audio(_) => {
                        md.push_str("[audio content not displayed in Markdown export]\n\n")
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
                md.push('\n');
            }
            MessageContent::ToolResponse(resp) => {
                md.push_str(&tool_response_to_markdown(resp, export_all_content));
                md.push('\n');
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

#[cfg(test)]
mod tests {
    use super::*;
    use goose::conversation::message::{Message, ToolRequest, ToolResponse};
    use mcp_core::tool::ToolCall;
    use rmcp::model::{Content, RawTextContent, TextContent};
    use serde_json::json;

    #[test]
    fn test_value_to_simple_markdown_string_normal() {
        let value = json!("hello world");
        let result = value_to_simple_markdown_string(&value, true);
        assert_eq!(result, "`hello world`");
    }

    #[test]
    fn test_value_to_simple_markdown_string_with_backticks() {
        let value = json!("hello `world`");
        let result = value_to_simple_markdown_string(&value, true);
        assert_eq!(result, "`hello \\`world\\``");
    }

    #[test]
    fn test_value_to_simple_markdown_string_long_string_full_export() {
        let long_string = "a".repeat(5000);
        let value = json!(long_string);
        let result = value_to_simple_markdown_string(&value, true);
        // When export_full_strings is true, should return full string
        assert!(result.starts_with("`"));
        assert!(result.ends_with("`"));
        assert!(result.contains(&"a".repeat(5000)));
    }

    #[test]
    fn test_value_to_simple_markdown_string_long_string_trimmed() {
        let long_string = "a".repeat(5000);
        let value = json!(long_string);
        let result = value_to_simple_markdown_string(&value, false);
        // When export_full_strings is false, should trim long strings
        assert!(result.starts_with("`"));
        assert!(result.contains("[ ... trimmed : "));
        assert!(result.contains("4900 chars ... ]`"));
        assert!(result.contains(&"a".repeat(97))); // Should contain the prefix (100 - 3 for "...")
    }

    #[test]
    fn test_value_to_simple_markdown_string_numbers_and_bools() {
        assert_eq!(value_to_simple_markdown_string(&json!(42), true), "42");
        assert_eq!(
            value_to_simple_markdown_string(&json!(true), true),
            "*true*"
        );
        assert_eq!(
            value_to_simple_markdown_string(&json!(false), true),
            "*false*"
        );
        assert_eq!(
            value_to_simple_markdown_string(&json!(null), true),
            "_null_"
        );
    }

    #[test]
    fn test_value_to_markdown_empty_object() {
        let value = json!({});
        let result = value_to_markdown(&value, 0, true);
        assert!(result.contains("*empty object*"));
    }

    #[test]
    fn test_value_to_markdown_empty_array() {
        let value = json!([]);
        let result = value_to_markdown(&value, 0, true);
        assert!(result.contains("*empty list*"));
    }

    #[test]
    fn test_value_to_markdown_simple_object() {
        let value = json!({
            "name": "test",
            "count": 42,
            "active": true
        });
        let result = value_to_markdown(&value, 0, true);
        assert!(result.contains("**name**"));
        assert!(result.contains("`test`"));
        assert!(result.contains("**count**"));
        assert!(result.contains("42"));
        assert!(result.contains("**active**"));
        assert!(result.contains("*true*"));
    }

    #[test]
    fn test_value_to_markdown_nested_object() {
        let value = json!({
            "user": {
                "name": "Alice",
                "age": 30
            }
        });
        let result = value_to_markdown(&value, 0, true);
        assert!(result.contains("**user**"));
        assert!(result.contains("**name**"));
        assert!(result.contains("`Alice`"));
        assert!(result.contains("**age**"));
        assert!(result.contains("30"));
    }

    #[test]
    fn test_value_to_markdown_array_with_items() {
        let value = json!(["item1", "item2", 42]);
        let result = value_to_markdown(&value, 0, true);
        assert!(result.contains("- `item1`"));
        assert!(result.contains("- `item2`"));
        // Numbers are handled by recursive call, so they get formatted differently
        assert!(result.contains("42"));
    }

    #[test]
    fn test_tool_request_to_markdown_shell() {
        let tool_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "ls -la",
                "working_dir": "/home/user"
            }),
        };
        let tool_request = ToolRequest {
            id: "test-id".to_string(),
            tool_call: Ok(tool_call),
        };

        let result = tool_request_to_markdown(&tool_request, true);
        assert!(result.contains("#### Tool Call: `shell`"));
        assert!(result.contains("namespace: `developer`"));
        assert!(result.contains("**command**:"));
        assert!(result.contains("```sh"));
        assert!(result.contains("ls -la"));
        assert!(result.contains("**working_dir**"));
    }

    #[test]
    fn test_tool_request_to_markdown_text_editor() {
        let tool_call = ToolCall {
            name: "developer__text_editor".to_string(),
            arguments: json!({
                "path": "/path/to/file.txt",
                "code_edit": "print('Hello World')"
            }),
        };
        let tool_request = ToolRequest {
            id: "test-id".to_string(),
            tool_call: Ok(tool_call),
        };

        let result = tool_request_to_markdown(&tool_request, true);
        assert!(result.contains("#### Tool Call: `text_editor`"));
        assert!(result.contains("**path**: `/path/to/file.txt`"));
        assert!(result.contains("**code_edit**:"));
        assert!(result.contains("print('Hello World')"));
    }

    #[test]
    fn test_tool_response_to_markdown_text() {
        let text_content = TextContent {
            raw: RawTextContent {
                text: "Command executed successfully".to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "test-id".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let result = tool_response_to_markdown(&tool_response, true);
        assert!(result.contains("#### Tool Response:"));
        assert!(result.contains("Command executed successfully"));
    }

    #[test]
    fn test_tool_response_to_markdown_json() {
        let json_text = r#"{"status": "success", "data": "test"}"#;
        let text_content = TextContent {
            raw: RawTextContent {
                text: json_text.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "test-id".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let result = tool_response_to_markdown(&tool_response, true);
        assert!(result.contains("#### Tool Response:"));
        assert!(result.contains("```json"));
        assert!(result.contains(json_text));
    }

    #[test]
    fn test_message_to_markdown_text() {
        let message = Message::user().with_text("Hello, this is a test message");

        let result = message_to_markdown(&message, true);
        assert_eq!(result, "Hello, this is a test message");
    }

    #[test]
    fn test_message_to_markdown_with_tool_request() {
        let tool_call = ToolCall {
            name: "test_tool".to_string(),
            arguments: json!({"param": "value"}),
        };

        let message = Message::assistant().with_tool_request("test-id", Ok(tool_call));

        let result = message_to_markdown(&message, true);
        assert!(result.contains("#### Tool Call: `test_tool`"));
        assert!(result.contains("**param**"));
    }

    #[test]
    fn test_message_to_markdown_thinking() {
        let message = Message::assistant()
            .with_thinking("I need to analyze this problem...", "test-signature");

        let result = message_to_markdown(&message, true);
        assert!(result.contains("**Thinking:**"));
        assert!(result.contains("> I need to analyze this problem..."));
    }

    #[test]
    fn test_message_to_markdown_redacted_thinking() {
        let message = Message::assistant().with_redacted_thinking("redacted-data");

        let result = message_to_markdown(&message, true);
        assert!(result.contains("**Thinking:**"));
        assert!(result.contains("> *Thinking was redacted*"));
    }

    #[test]
    fn test_recursive_value_to_markdown() {
        // Test that complex nested structures are properly handled with recursion
        let value = json!({
            "level1": {
                "level2": {
                    "data": "nested value"
                },
                "array": [
                    {"item": "first"},
                    {"item": "second"}
                ]
            }
        });

        let result = value_to_markdown(&value, 0, true);
        assert!(result.contains("**level1**"));
        assert!(result.contains("**level2**"));
        assert!(result.contains("**data**"));
        assert!(result.contains("`nested value`"));
        assert!(result.contains("**array**"));
        assert!(result.contains("**item**"));
        assert!(result.contains("`first`"));
        assert!(result.contains("`second`"));
    }

    #[test]
    fn test_shell_tool_with_code_output() {
        let tool_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "cat main.py"
            }),
        };
        let tool_request = ToolRequest {
            id: "shell-cat".to_string(),
            tool_call: Ok(tool_call),
        };

        let python_code = r#"#!/usr/bin/env python3
def hello_world():
    print("Hello, World!")
    
if __name__ == "__main__":
    hello_world()"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: python_code.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "shell-cat".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let request_result = tool_request_to_markdown(&tool_request, true);
        let response_result = tool_response_to_markdown(&tool_response, true);

        // Check request formatting
        assert!(request_result.contains("#### Tool Call: `shell`"));
        assert!(request_result.contains("```sh"));
        assert!(request_result.contains("cat main.py"));

        // Check response formatting - text content is output as plain text
        assert!(response_result.contains("#### Tool Response:"));
        assert!(response_result.contains("def hello_world():"));
        assert!(response_result.contains("print(\"Hello, World!\")"));
    }

    #[test]
    fn test_shell_tool_with_git_commands() {
        let git_status_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "git status --porcelain"
            }),
        };
        let tool_request = ToolRequest {
            id: "git-status".to_string(),
            tool_call: Ok(git_status_call),
        };

        let git_output = " M src/main.rs\n?? temp.txt\n A new_feature.rs";
        let text_content = TextContent {
            raw: RawTextContent {
                text: git_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "git-status".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let request_result = tool_request_to_markdown(&tool_request, true);
        let response_result = tool_response_to_markdown(&tool_response, true);

        // Check request formatting
        assert!(request_result.contains("git status --porcelain"));
        assert!(request_result.contains("```sh"));

        // Check response formatting - git output as plain text
        assert!(response_result.contains("M src/main.rs"));
        assert!(response_result.contains("?? temp.txt"));
    }

    #[test]
    fn test_shell_tool_with_build_output() {
        let cargo_build_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "cargo build"
            }),
        };
        let _tool_request = ToolRequest {
            id: "cargo-build".to_string(),
            tool_call: Ok(cargo_build_call),
        };

        let build_output = r#"   Compiling goose-cli v0.1.0 (/Users/user/goose)
warning: unused variable `x`
 --> src/main.rs:10:9
   |
10 |     let x = 5;
   |         ^ help: if this is intentional, prefix it with an underscore: `_x`
   |
   = note: `#[warn(unused_variables)]` on by default

    Finished dev [unoptimized + debuginfo] target(s) in 2.45s"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: build_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "cargo-build".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let response_result = tool_response_to_markdown(&tool_response, true);

        // Should format as plain text since it's build output, not code
        assert!(response_result.contains("Compiling goose-cli"));
        assert!(response_result.contains("warning: unused variable"));
        assert!(response_result.contains("Finished dev"));
    }

    #[test]
    fn test_shell_tool_with_json_api_response() {
        let curl_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "curl -s https://api.github.com/repos/microsoft/vscode/releases/latest"
            }),
        };
        let _tool_request = ToolRequest {
            id: "curl-api".to_string(),
            tool_call: Ok(curl_call),
        };

        let api_response = r#"{
  "url": "https://api.github.com/repos/microsoft/vscode/releases/90543298",
  "tag_name": "1.85.0",
  "name": "1.85.0",
  "published_at": "2023-12-07T16:54:32Z",
  "assets": [
    {
      "name": "VSCode-darwin-universal.zip",
      "download_count": 123456
    }
  ]
}"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: api_response.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "curl-api".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let response_result = tool_response_to_markdown(&tool_response, true);

        // Should detect and format as JSON
        assert!(response_result.contains("```json"));
        assert!(response_result.contains("\"tag_name\": \"1.85.0\""));
        assert!(response_result.contains("\"download_count\": 123456"));
    }

    #[test]
    fn test_text_editor_tool_with_code_creation() {
        let editor_call = ToolCall {
            name: "developer__text_editor".to_string(),
            arguments: json!({
                "command": "write",
                "path": "/tmp/fibonacci.js",
                "file_text": "function fibonacci(n) {\n  if (n <= 1) return n;\n  return fibonacci(n - 1) + fibonacci(n - 2);\n}\n\nconsole.log(fibonacci(10));"
            }),
        };
        let tool_request = ToolRequest {
            id: "editor-write".to_string(),
            tool_call: Ok(editor_call),
        };

        let text_content = TextContent {
            raw: RawTextContent {
                text: "File created successfully".to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "editor-write".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let request_result = tool_request_to_markdown(&tool_request, true);
        let response_result = tool_response_to_markdown(&tool_response, true);

        // Check request formatting - should format code in file_text properly
        assert!(request_result.contains("#### Tool Call: `text_editor`"));
        assert!(request_result.contains("**path**: `/tmp/fibonacci.js`"));
        assert!(request_result.contains("**file_text**:"));
        assert!(request_result.contains("function fibonacci(n)"));
        assert!(request_result.contains("return fibonacci(n - 1)"));

        // Check response formatting
        assert!(response_result.contains("File created successfully"));
    }

    #[test]
    fn test_text_editor_tool_view_code() {
        let editor_call = ToolCall {
            name: "developer__text_editor".to_string(),
            arguments: json!({
                "command": "view",
                "path": "/src/utils.py"
            }),
        };
        let _tool_request = ToolRequest {
            id: "editor-view".to_string(),
            tool_call: Ok(editor_call),
        };

        let python_code = r#"import os
import json
from typing import Dict, List, Optional

def load_config(config_path: str) -> Dict:
    """Load configuration from JSON file."""
    if not os.path.exists(config_path):
        raise FileNotFoundError(f"Config file not found: {config_path}")
    
    with open(config_path, 'r') as f:
        return json.load(f)

def process_data(data: List[Dict]) -> List[Dict]:
    """Process a list of data dictionaries."""
    return [item for item in data if item.get('active', False)]"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: python_code.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "editor-view".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let response_result = tool_response_to_markdown(&tool_response, true);

        // Text content is output as plain text
        assert!(response_result.contains("import os"));
        assert!(response_result.contains("def load_config"));
        assert!(response_result.contains("typing import Dict"));
    }

    #[test]
    fn test_shell_tool_with_error_output() {
        let error_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "python nonexistent_script.py"
            }),
        };
        let _tool_request = ToolRequest {
            id: "shell-error".to_string(),
            tool_call: Ok(error_call),
        };

        let error_output = r#"python: can't open file 'nonexistent_script.py': [Errno 2] No such file or directory
Command failed with exit code 2"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: error_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "shell-error".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let response_result = tool_response_to_markdown(&tool_response, true);

        // Error output should be formatted as plain text
        assert!(response_result.contains("can't open file"));
        assert!(response_result.contains("Command failed with exit code 2"));
    }

    #[test]
    fn test_shell_tool_complex_script_execution() {
        let script_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "python -c \"import sys; print(f'Python {sys.version}'); [print(f'{i}^2 = {i**2}') for i in range(1, 6)]\""
            }),
        };
        let tool_request = ToolRequest {
            id: "script-exec".to_string(),
            tool_call: Ok(script_call),
        };

        let script_output = r#"Python 3.11.5 (main, Aug 24 2023, 15:18:16) [Clang 14.0.3 ]
1^2 = 1
2^2 = 4
3^2 = 9
4^2 = 16
5^2 = 25"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: script_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "script-exec".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let request_result = tool_request_to_markdown(&tool_request, true);
        let response_result = tool_response_to_markdown(&tool_response, true);

        // Check request formatting for complex command
        assert!(request_result.contains("```sh"));
        assert!(request_result.contains("python -c"));
        assert!(request_result.contains("sys.version"));

        // Check response formatting
        assert!(response_result.contains("Python 3.11.5"));
        assert!(response_result.contains("1^2 = 1"));
        assert!(response_result.contains("5^2 = 25"));
    }

    #[test]
    fn test_shell_tool_with_multi_command() {
        let multi_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "cd /tmp && ls -la | head -5 && pwd"
            }),
        };
        let _tool_request = ToolRequest {
            id: "multi-cmd".to_string(),
            tool_call: Ok(multi_call),
        };

        let multi_output = r#"total 24
drwxrwxrwt  15 root  wheel   480 Dec  7 10:30 .
drwxr-xr-x   6 root  wheel   192 Nov 15 09:15 ..
-rw-r--r--   1 user  staff   256 Dec  7 09:45 config.json
drwx------   3 user  staff    96 Dec  6 16:20 com.apple.launchd.abc
/tmp"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: multi_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "multi-cmd".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let request_result = tool_request_to_markdown(&_tool_request, true);
        let response_result = tool_response_to_markdown(&tool_response, true);

        // Check request formatting for chained commands
        assert!(request_result.contains("cd /tmp && ls -la | head -5 && pwd"));

        // Check response formatting
        assert!(response_result.contains("drwxrwxrwt"));
        assert!(response_result.contains("config.json"));
        assert!(response_result.contains("/tmp"));
    }

    #[test]
    fn test_developer_tool_grep_code_search() {
        let grep_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "rg 'async fn' --type rust -n"
            }),
        };
        let tool_request = ToolRequest {
            id: "grep-search".to_string(),
            tool_call: Ok(grep_call),
        };

        let grep_output = r#"src/main.rs:15:async fn process_request(req: Request) -> Result<Response> {
src/handler.rs:8:async fn handle_connection(stream: TcpStream) {
src/database.rs:23:async fn query_users(pool: &Pool) -> Result<Vec<User>> {
src/middleware.rs:12:async fn auth_middleware(req: Request, next: Next) -> Result<Response> {"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: grep_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "grep-search".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let request_result = tool_request_to_markdown(&tool_request, true);
        let response_result = tool_response_to_markdown(&tool_response, true);

        // Check request formatting
        assert!(request_result.contains("rg 'async fn' --type rust -n"));

        // Check response formatting - should be formatted as search results
        assert!(response_result.contains("src/main.rs:15:"));
        assert!(response_result.contains("async fn process_request"));
        assert!(response_result.contains("src/database.rs:23:"));
    }

    #[test]
    fn test_shell_tool_json_detection_works() {
        // This test shows that JSON detection in tool responses DOES work
        let tool_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "echo '{\"test\": \"json\"}'"
            }),
        };
        let _tool_request = ToolRequest {
            id: "json-test".to_string(),
            tool_call: Ok(tool_call),
        };

        let json_output = r#"{"status": "success", "data": {"count": 42}}"#;
        let text_content = TextContent {
            raw: RawTextContent {
                text: json_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "json-test".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let response_result = tool_response_to_markdown(&tool_response, true);

        // JSON should be auto-detected and formatted
        assert!(response_result.contains("```json"));
        assert!(response_result.contains("\"status\": \"success\""));
        assert!(response_result.contains("\"count\": 42"));
    }

    #[test]
    fn test_shell_tool_with_package_management() {
        let npm_call = ToolCall {
            name: "developer__shell".to_string(),
            arguments: json!({
                "command": "npm install express typescript @types/node --save-dev"
            }),
        };
        let tool_request = ToolRequest {
            id: "npm-install".to_string(),
            tool_call: Ok(npm_call),
        };

        let npm_output = r#"added 57 packages, and audited 58 packages in 3s

8 packages are looking for funding
  run `npm fund` for details

found 0 vulnerabilities"#;

        let text_content = TextContent {
            raw: RawTextContent {
                text: npm_output.to_string(),
            },
            annotations: None,
        };
        let tool_response = ToolResponse {
            id: "npm-install".to_string(),
            tool_result: Ok(vec![Content::text(text_content.raw.text)]),
        };

        let request_result = tool_request_to_markdown(&tool_request, true);
        let response_result = tool_response_to_markdown(&tool_response, true);

        // Check request formatting
        assert!(request_result.contains("npm install express typescript"));
        assert!(request_result.contains("--save-dev"));

        // Check response formatting
        assert!(response_result.contains("added 57 packages"));
        assert!(response_result.contains("found 0 vulnerabilities"));
    }
}
