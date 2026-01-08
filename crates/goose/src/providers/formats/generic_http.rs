//! Template engine and formatters for Generic HTTP LLM Provider
//!
//! This module provides:
//! - Template variable substitution
//! - Message history formatting
//! - Tool injection prompt generation
//! - Response parsing with JSONPath
//! - Tool call extraction from responses

use crate::config::generic_provider_config::{
    RoleMappings, ToolInjectionConfig, ToolInjectionFormat,
};
use crate::conversation::message::{Message, MessageContent};
use crate::providers::base::Usage;
use anyhow::{anyhow, Result};
use regex::Regex;
use rmcp::model::{Role, Tool};
use serde_json::Value;
use std::collections::HashMap;

/// Parsed tool call from LLM response
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Substitute variables in a template string
///
/// Supported formats:
/// - `${VAR}` - simple substitution
/// - `${VAR:number}` - parse as number
/// - `${VAR:bool}` - parse as boolean (true/false)
/// - `${VAR:bool_string}` - boolean as string ("True"/"False")
/// - `${VAR:json}` - parse as JSON
pub fn substitute_template(template: &str, variables: &HashMap<String, String>) -> Result<String> {
    let re = Regex::new(r"\$\{([^}:]+)(?::([^}]+))?\}")?;

    let mut result = template.to_string();
    let mut replacements = Vec::new();

    for cap in re.captures_iter(template) {
        let full_match = cap.get(0).unwrap().as_str();
        let var_name = cap.get(1).unwrap().as_str();
        let type_hint = cap.get(2).map(|m| m.as_str());

        let value = variables
            .get(var_name)
            .ok_or_else(|| anyhow!("Variable not found: {}", var_name))?;

        let replacement = match type_hint {
            Some("bool_string") => {
                let bool_val = value.parse::<bool>().unwrap_or(false);
                if bool_val { "True" } else { "False" }.to_string()
            }
            _ => value.clone(),
        };

        replacements.push((full_match.to_string(), replacement));
    }

    for (pattern, replacement) in replacements {
        result = result.replace(&pattern, &replacement);
    }

    Ok(result)
}

/// Substitute variables in a JSON template
///
/// Recursively processes all string values in the JSON structure
pub fn substitute_json_template(
    template: &Value,
    variables: &HashMap<String, String>,
) -> Result<Value> {
    match template {
        Value::String(s) => {
            let substituted = substitute_template(s, variables)?;

            // Check for type hints and convert accordingly
            let re = Regex::new(r"\$\{([^}:]+):([^}]+)\}")?;
            if let Some(cap) = re.captures(s) {
                let var_name = cap.get(1).unwrap().as_str();
                let type_hint = cap.get(2).unwrap().as_str();

                if let Some(value) = variables.get(var_name) {
                    return match type_hint {
                        "number" => {
                            if let Ok(n) = value.parse::<i64>() {
                                Ok(Value::Number(n.into()))
                            } else if let Ok(n) = value.parse::<f64>() {
                                Ok(serde_json::Number::from_f64(n)
                                    .map(Value::Number)
                                    .unwrap_or(Value::String(substituted)))
                            } else {
                                Ok(Value::String(substituted))
                            }
                        }
                        "bool" => {
                            let bool_val = value.parse::<bool>().unwrap_or(false);
                            Ok(Value::Bool(bool_val))
                        }
                        "json" => serde_json::from_str(value)
                            .map_err(|e| anyhow!("Failed to parse JSON: {}", e)),
                        _ => Ok(Value::String(substituted)),
                    };
                }
            }

            Ok(Value::String(substituted))
        }
        Value::Array(arr) => {
            let new_arr: Result<Vec<Value>> = arr
                .iter()
                .map(|v| substitute_json_template(v, variables))
                .collect();
            Ok(Value::Array(new_arr?))
        }
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                let new_key = substitute_template(k, variables)?;
                let new_val = substitute_json_template(v, variables)?;
                new_obj.insert(new_key, new_val);
            }
            Ok(Value::Object(new_obj))
        }
        _ => Ok(template.clone()),
    }
}

/// Extract a value from JSON using a simple JSONPath-like syntax
///
/// Supported syntax:
/// - `$.field` - access object field
/// - `$.field.nested` - nested field access
/// - `$.array[0]` - array index access
/// - `$.array[0].field` - combined access
pub fn extract_by_path(json: &Value, path: &str) -> Option<Value> {
    let path = path.strip_prefix("$.").unwrap_or(path);
    if path.is_empty() {
        return Some(json.clone());
    }

    let mut current = json;

    for part in split_path(path) {
        current = match &part {
            PathPart::Field(name) => current.get(name)?,
            PathPart::Index(idx) => current.get(*idx)?,
        };
    }

    Some(current.clone())
}

enum PathPart {
    Field(String),
    Index(usize),
}

fn split_path(path: &str) -> Vec<PathPart> {
    let mut parts = Vec::new();
    let re = Regex::new(r"([^\.\[\]]+)|\[(\d+)\]").unwrap();

    for cap in re.captures_iter(path) {
        if let Some(field) = cap.get(1) {
            parts.push(PathPart::Field(field.as_str().to_string()));
        } else if let Some(idx) = cap.get(2) {
            if let Ok(i) = idx.as_str().parse::<usize>() {
                parts.push(PathPart::Index(i));
            }
        }
    }

    parts
}

/// Extract full message content including tool requests and responses
/// This is different from as_concat_text() which only extracts Text content
fn extract_message_content(message: &Message) -> String {
    let mut parts: Vec<String> = Vec::new();

    for content in &message.content {
        match content {
            MessageContent::Text(text) => {
                parts.push(text.text.clone());
            }
            MessageContent::ToolRequest(request) => {
                // Format tool request as the LLM originally output it
                if let Ok(call) = &request.tool_call {
                    let tool_json = serde_json::json!({
                        "name": call.name,
                        "arguments": call.arguments
                    });
                    parts.push(format!(
                        "```tool_call\n{}\n```",
                        serde_json::to_string_pretty(&tool_json).unwrap_or_default()
                    ));
                }
            }
            MessageContent::ToolResponse(response) => {
                // Format tool response so LLM knows the result
                match &response.tool_result {
                    Ok(result) => {
                        let result_text: Vec<String> = result
                            .content
                            .iter()
                            .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                            .collect();
                        if !result_text.is_empty() {
                            parts.push(format!(
                                "[Tool Result for {}]:\n{}",
                                response.id,
                                result_text.join("\n")
                            ));
                        } else {
                            parts.push(format!("[Tool {} completed successfully]", response.id));
                        }
                    }
                    Err(err) => {
                        parts.push(format!("[Tool Error for {}]: {}", response.id, err.message));
                    }
                }
            }
            // Skip other content types (Image, Thinking, etc.)
            _ => {}
        }
    }

    parts.join("\n")
}

/// Format message history into a single string
pub fn format_messages(
    system: &str,
    messages: &[Message],
    prompt_format: Option<&str>,
    message_format: Option<&str>,
    role_mappings: Option<&RoleMappings>,
) -> String {
    let default_mappings = RoleMappings::default();
    let mappings = role_mappings.unwrap_or(&default_mappings);

    let msg_fmt = message_format.unwrap_or("${ROLE}: ${CONTENT}");

    // Format each message
    let history: Vec<String> = messages
        .iter()
        .take(messages.len().saturating_sub(1))
        .filter_map(|msg| {
            let role = match msg.role {
                Role::User => &mappings.user,
                Role::Assistant => &mappings.assistant,
            };
            let content = extract_message_content(msg);

            // Skip empty messages
            if content.trim().is_empty() {
                return None;
            }

            Some(
                msg_fmt
                    .replace("${ROLE}", role)
                    .replace("${CONTENT}", &content),
            )
        })
        .collect();

    let history_str = history.join("\n\n");

    // Get current user query (last message)
    let user_query = messages
        .last()
        .map(extract_message_content)
        .unwrap_or_default();

    // Apply prompt format
    let default_format = "${SYSTEM}\n\n${HISTORY}\n\n${ROLE}: ${USER_QUERY}";
    let fmt = prompt_format.unwrap_or(default_format);

    fmt.replace("${SYSTEM}", system)
        .replace("${HISTORY}", &history_str)
        .replace("${USER_QUERY}", &user_query)
        .replace("${ROLE}", &mappings.user)
}

/// Generate tool injection prompt to append to system prompt
pub fn generate_tool_injection(tools: &[Tool], config: &ToolInjectionConfig) -> String {
    if !config.enabled || tools.is_empty() {
        return String::new();
    }

    // Use custom template if provided
    if let Some(template) = &config.system_template {
        let tools_json = serde_json::to_string_pretty(tools).unwrap_or_default();
        return template.replace("${TOOLS}", &tools_json);
    }

    // Default template based on format
    let mut injection = String::new();

    injection.push_str("\n\n## Available Tools\n\n");
    injection.push_str("You have access to the following tools. ");

    match config.format {
        ToolInjectionFormat::MarkdownCodeblock => {
            injection.push_str(&format!(
                "To use a tool, respond with a JSON block in this EXACT format:\n\n```{}\n",
                config.block_name
            ));
            injection.push_str(r#"{"name": "tool_name", "arguments": {"param1": "value1"}}"#);
            injection.push_str("\n```\n\n");
        }
        ToolInjectionFormat::Xml => {
            injection.push_str("To use a tool, respond with XML in this EXACT format:\n\n");
            injection.push_str(&format!("<{}>\n", config.block_name));
            injection.push_str("<name>tool_name</name>\n");
            injection.push_str(r#"<arguments>{"param1": "value1"}</arguments>"#);
            injection.push_str(&format!("\n</{}>\n\n", config.block_name));
        }
        ToolInjectionFormat::Json => {
            injection
                .push_str("To use a tool, respond with a JSON object in this EXACT format:\n\n");
            injection.push_str(&format!(
                r#"{{"{block_name}": {{"name": "tool_name", "arguments": {{"param1": "value1"}}}}}}"#,
                block_name = config.block_name
            ));
            injection.push_str("\n\n");
        }
    }

    injection.push_str("### Available tools:\n\n");

    for tool in tools {
        injection.push_str(&format!("#### {}\n", tool.name));
        if let Some(desc) = &tool.description {
            injection.push_str(&format!("{}\n", desc));
        }
        injection.push_str(&format!(
            "Parameters: {}\n\n",
            serde_json::to_string(&tool.input_schema).unwrap_or_default()
        ));
    }

    injection.push_str("### Rules:\n");
    injection.push_str("- When you need to use a tool, output ONLY the tool call block\n");
    injection.push_str("- Do not explain that you're using a tool, just use it\n");
    injection.push_str("- After receiving tool results, continue your response normally\n");
    injection.push_str("- You can call multiple tools by outputting multiple tool call blocks\n");

    injection
}

/// Parse tool calls from LLM response based on injection format
pub fn parse_tool_calls(
    content: &str,
    config: &ToolInjectionConfig,
) -> (String, Vec<ParsedToolCall>) {
    if !config.enabled {
        return (content.to_string(), Vec::new());
    }

    match config.format {
        ToolInjectionFormat::MarkdownCodeblock => {
            parse_markdown_tool_calls(content, &config.block_name)
        }
        ToolInjectionFormat::Xml => parse_xml_tool_calls(content, &config.block_name),
        ToolInjectionFormat::Json => parse_json_tool_calls(content, &config.block_name),
    }
}

fn parse_markdown_tool_calls(content: &str, block_name: &str) -> (String, Vec<ParsedToolCall>) {
    let pattern = format!(r"```{}\s*\n?([\s\S]*?)```", regex::escape(block_name));
    let re = Regex::new(&pattern).unwrap();

    let mut tool_calls = Vec::new();
    let mut remaining = content.to_string();

    for cap in re.captures_iter(content) {
        if let Some(json_str) = cap.get(1) {
            if let Ok(parsed) = serde_json::from_str::<Value>(json_str.as_str().trim()) {
                if let (Some(name), Some(args)) = (
                    parsed.get("name").and_then(|v| v.as_str()),
                    parsed.get("arguments"),
                ) {
                    tool_calls.push(ParsedToolCall {
                        id: format!("call_{}", uuid::Uuid::new_v4()),
                        name: name.to_string(),
                        arguments: args.clone(),
                    });
                }
            }
        }
    }

    remaining = re.replace_all(&remaining, "").to_string();
    (remaining.trim().to_string(), tool_calls)
}

fn parse_xml_tool_calls(content: &str, block_name: &str) -> (String, Vec<ParsedToolCall>) {
    let pattern = format!(
        r"<{}>\s*<name>([^<]+)</name>\s*<arguments>([\s\S]*?)</arguments>\s*</{}>",
        regex::escape(block_name),
        regex::escape(block_name)
    );
    let re = Regex::new(&pattern).unwrap();

    let mut tool_calls = Vec::new();
    let mut remaining = content.to_string();

    for cap in re.captures_iter(content) {
        if let (Some(name), Some(args_str)) = (cap.get(1), cap.get(2)) {
            if let Ok(args) = serde_json::from_str::<Value>(args_str.as_str().trim()) {
                tool_calls.push(ParsedToolCall {
                    id: format!("call_{}", uuid::Uuid::new_v4()),
                    name: name.as_str().trim().to_string(),
                    arguments: args,
                });
            }
        }
    }

    let full_pattern = format!(
        r"<{}>\s*<name>[^<]+</name>\s*<arguments>[\s\S]*?</arguments>\s*</{}>",
        regex::escape(block_name),
        regex::escape(block_name)
    );
    let full_re = Regex::new(&full_pattern).unwrap();
    remaining = full_re.replace_all(&remaining, "").to_string();

    (remaining.trim().to_string(), tool_calls)
}

fn parse_json_tool_calls(content: &str, block_name: &str) -> (String, Vec<ParsedToolCall>) {
    // Try to find JSON objects with the block_name key
    let mut tool_calls = Vec::new();
    let mut remaining = content.to_string();

    // Simple approach: try to parse each line as JSON
    let json_pattern = Regex::new(r"\{[^{}]*\{[^{}]*\}[^{}]*\}").unwrap();

    for cap in json_pattern.find_iter(content) {
        if let Ok(parsed) = serde_json::from_str::<Value>(cap.as_str()) {
            if let Some(tool_call) = parsed.get(block_name) {
                if let (Some(name), Some(args)) = (
                    tool_call.get("name").and_then(|v| v.as_str()),
                    tool_call.get("arguments"),
                ) {
                    tool_calls.push(ParsedToolCall {
                        id: format!("call_{}", uuid::Uuid::new_v4()),
                        name: name.to_string(),
                        arguments: args.clone(),
                    });
                    remaining = remaining.replace(cap.as_str(), "");
                }
            }
        }
    }

    (remaining.trim().to_string(), tool_calls)
}

/// Extract usage information from response
pub fn extract_usage(
    response: &Value,
    input_path: Option<&str>,
    output_path: Option<&str>,
    total_path: Option<&str>,
) -> Usage {
    let input_tokens = input_path
        .and_then(|p| extract_by_path(response, p))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let output_tokens = output_path
        .and_then(|p| extract_by_path(response, p))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let total_tokens = total_path
        .and_then(|p| extract_by_path(response, p))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    Usage::new(input_tokens, output_tokens, total_tokens)
}

/// Build variables map for template substitution
#[allow(clippy::too_many_arguments)]
pub fn build_variables(
    system: &str,
    messages: &[Message],
    model: &str,
    is_stream: bool,
    config_values: &HashMap<String, String>,
    prompt_format: Option<&str>,
    message_format: Option<&str>,
    role_mappings: Option<&RoleMappings>,
) -> HashMap<String, String> {
    let mut vars = config_values.clone();

    // Add system variables
    vars.insert("SYSTEM".to_string(), system.to_string());
    vars.insert("MODEL".to_string(), model.to_string());
    vars.insert("IS_STREAM".to_string(), is_stream.to_string());

    // Format history
    let history: Vec<String> = messages
        .iter()
        .take(messages.len().saturating_sub(1))
        .map(|msg| {
            let role = match msg.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
            };
            format!("{}: {}", role, msg.as_concat_text())
        })
        .collect();
    vars.insert("HISTORY".to_string(), history.join("\n\n"));

    // Get current query
    let user_query = messages
        .last()
        .map(|m| m.as_concat_text())
        .unwrap_or_default();
    vars.insert("USER_QUERY".to_string(), user_query);

    // Build full prompt
    let prompt = format_messages(
        system,
        messages,
        prompt_format,
        message_format,
        role_mappings,
    );
    vars.insert("PROMPT".to_string(), prompt);

    vars
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_substitute_simple() {
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "test".to_string());
        vars.insert("VALUE".to_string(), "123".to_string());

        let result = substitute_template("Hello ${NAME}, value is ${VALUE}", &vars).unwrap();
        assert_eq!(result, "Hello test, value is 123");
    }

    #[test]
    fn test_substitute_bool_string() {
        let mut vars = HashMap::new();
        vars.insert("FLAG".to_string(), "true".to_string());

        let result = substitute_template("${FLAG:bool_string}", &vars).unwrap();
        assert_eq!(result, "True");
    }

    #[test]
    fn test_substitute_json_number() {
        let mut vars = HashMap::new();
        vars.insert("ID".to_string(), "123".to_string());

        let template = json!({"id": "${ID:number}"});
        let result = substitute_json_template(&template, &vars).unwrap();
        assert_eq!(result, json!({"id": 123}));
    }

    #[test]
    fn test_extract_by_path() {
        let json = json!({
            "response": {
                "content": "Hello",
                "items": [{"id": 1}, {"id": 2}]
            }
        });

        assert_eq!(
            extract_by_path(&json, "$.response.content"),
            Some(json!("Hello"))
        );
        assert_eq!(
            extract_by_path(&json, "$.response.items[0].id"),
            Some(json!(1))
        );
    }

    #[test]
    fn test_parse_markdown_tool_calls() {
        let content = r#"I'll read the file for you.

```tool_call
{"name": "read_file", "arguments": {"path": "/tmp/test.txt"}}
```

Done!"#;

        let config = ToolInjectionConfig {
            enabled: true,
            format: ToolInjectionFormat::MarkdownCodeblock,
            block_name: "tool_call".to_string(),
            system_template: None,
        };

        let (remaining, calls) = parse_tool_calls(content, &config);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert!(remaining.contains("I'll read the file"));
        assert!(remaining.contains("Done!"));
    }

    #[test]
    fn test_parse_xml_tool_calls() {
        let content = r#"<tool_call>
<name>read_file</name>
<arguments>{"path": "/tmp/test.txt"}</arguments>
</tool_call>"#;

        let config = ToolInjectionConfig {
            enabled: true,
            format: ToolInjectionFormat::Xml,
            block_name: "tool_call".to_string(),
            system_template: None,
        };

        let (_, calls) = parse_tool_calls(content, &config);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
    }
}
