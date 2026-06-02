//! # ToolShim Module
//!
//! The ToolShim module provides a reusable component for interpreting and augmenting LLM outputs with tool calls,
//! regardless of whether the underlying model natively supports tool/function calling.
//!
//! ## Overview
//!
//! ToolShim addresses the challenge of working with models that don't natively support tools by:
//!
//! 1. Taking the text output from any LLM
//! 2. Sending it to a separate "interpreter" model (which can be the same or different model)
//! 3. Using a model to extract tool call intentions into the appropriate format
//! 4. Converting the outputs of the interpreter model into proper tool call structs
//! 5. Augmenting the original message with the extracted tool calls
//!
//! ## Key Components
//!
//! ### ToolInterpreter Trait
//!
//! The core of ToolShim is the `ToolInterpreter` trait, which defines the interface for any model that can interpret text and extract tool calls.
//!
//! ### Implementations
//!
//! The module provides an implementation for Ollama:
//!
//! - `OllamaInterpreter`: Uses Ollama's structured output API to interpret tool calls
//!
//! ### Helper Functions
//!
//! - `augment_message_with_tool_calls`: A utility function that takes any message, extracts text content, sends it to an interpreter, and adds any detected tool calls back to the message.
//!

mod interpreter;
mod parsing;
mod sanitize;
pub use interpreter::*;
pub use parsing::*;
pub use sanitize::*;

use super::errors::ProviderError;
#[cfg(feature = "local-inference")]
use super::local_inference::LOCAL_LLM_MODEL_CONFIG_KEY;
use super::ollama::OLLAMA_DEFAULT_PORT;
use super::ollama::OLLAMA_HOST;
use super::utils;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::model::ModelConfig;
use crate::providers::base::DEFAULT_PROVIDER_TIMEOUT_SECS;
use crate::providers::formats::openai::create_request;
use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use rmcp::model::{object, CallToolRequestParams, RawContent, Tool};
use serde_json::{json, Value};
use std::ops::Deref;
use std::time::Duration;
use uuid::Uuid;

/// Default model to use for tool interpretation
pub const DEFAULT_INTERPRETER_MODEL_OLLAMA: &str = "mistral-nemo";
pub const TOOLSHIM_BACKEND_ENV_VAR: &str = "GOOSE_TOOLSHIM_BACKEND";
pub const TOOLSHIM_LOCAL_MODEL_ENV_VAR: &str = "GOOSE_TOOLSHIM_MODEL";
#[cfg(not(feature = "local-inference"))]
const LOCAL_LLM_MODEL_CONFIG_KEY: &str = "LOCAL_LLM_MODEL";

pub(super) const TOOL_CALLS_SECTION_BEGIN: &str = "<|tool_calls_section_begin|>";
pub(super) const TOOL_CALLS_SECTION_END: &str = "<|tool_calls_section_end|>";
pub(super) const TOOL_CALL_BEGIN: &str = "<|tool_call_begin|>";
pub(super) const TOOL_CALL_ARGUMENT_BEGIN: &str = "<|tool_call_argument_begin|>";
pub(super) const TOOL_CALL_ARGUMENT_END: &str = "<|tool_call_argument_end|>";
pub(super) const TOOL_CALL_END: &str = "<|tool_call_end|>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolshimBackend {
    Ollama,
    Local,
}

fn parse_toolshim_backend(value: &str) -> Result<ToolshimBackend, ProviderError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "ollama" => Ok(ToolshimBackend::Ollama),
        "local" | "llama.cpp" | "llama_cpp" => Ok(ToolshimBackend::Local),
        other => Err(ProviderError::RequestFailed(format!(
            "Invalid {} value '{}'. Expected one of: ollama, local, llama.cpp",
            TOOLSHIM_BACKEND_ENV_VAR, other
        ))),
    }
}

fn get_toolshim_backend() -> Result<ToolshimBackend, ProviderError> {
    match std::env::var(TOOLSHIM_BACKEND_ENV_VAR) {
        Ok(value) => parse_toolshim_backend(&value),
        Err(_) => Ok(ToolshimBackend::Ollama),
    }
}

fn resolve_local_interpreter_model() -> Result<String, ProviderError> {
    let env_model = std::env::var(TOOLSHIM_LOCAL_MODEL_ENV_VAR)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let config_model = crate::config::Config::global()
        .get_param::<String>(LOCAL_LLM_MODEL_CONFIG_KEY)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    resolve_local_interpreter_model_from_sources(env_model, config_model)
}

fn resolve_local_interpreter_model_from_sources(
    env_model: Option<String>,
    config_model: Option<String>,
) -> Result<String, ProviderError> {
    env_model.or(config_model).ok_or_else(|| {
        ProviderError::RequestFailed(format!(
            "Local toolshim backend requires {} or {} to be set",
            TOOLSHIM_LOCAL_MODEL_ENV_VAR, LOCAL_LLM_MODEL_CONFIG_KEY
        ))
    })
}

pub fn resolve_tool_name(raw_tool_name: &str, tools: &[Tool]) -> Option<String> {
    let trimmed = raw_tool_name.trim();
    let without_index = trimmed.split(':').next().unwrap_or(trimmed).trim();
    let without_functions_prefix = without_index
        .strip_prefix("functions.")
        .unwrap_or(without_index)
        .trim();
    let short_name = without_functions_prefix
        .rsplit('.')
        .next()
        .unwrap_or(without_functions_prefix)
        .trim();

    // Also try replacing dots with double-underscores (goose tool name convention)
    let with_dunder = without_functions_prefix.replace('.', "__");

    let mut candidates = vec![
        trimmed.to_string(),
        without_index.to_string(),
        without_functions_prefix.to_string(),
        with_dunder,
        short_name.to_string(),
    ];
    candidates.dedup();

    for candidate in &candidates {
        if tools.iter().any(|tool| tool.name == *candidate) {
            return Some(candidate.clone());
        }
    }

    for candidate in &candidates {
        let mut matches: Vec<String> = tools
            .iter()
            .filter(|tool| tool.name.ends_with(&format!("__{}", candidate)))
            .map(|tool| tool.name.to_string())
            .collect();
        matches.sort();
        matches.dedup();

        if matches.len() == 1 {
            return Some(matches[0].clone());
        }
    }

    None
}

pub fn normalized_tool_alias(raw_tool_name: &str) -> String {
    let trimmed = raw_tool_name.trim();
    let without_index = trimmed.split(':').next().unwrap_or(trimmed).trim();
    let without_functions_prefix = without_index
        .strip_prefix("functions.")
        .unwrap_or(without_index)
        .trim();

    without_functions_prefix
        .rsplit('.')
        .next()
        .unwrap_or(without_functions_prefix)
        .trim()
        .to_ascii_lowercase()
}

/// Creates a string containing formatted tool information
pub fn format_tool_info(tools: &[Tool]) -> String {
    let mut tool_info = String::new();
    for tool in tools {
        tool_info.push_str(&format!(
            "Tool Name: {}\nSchema: {}\nDescription: {:?}\n\n",
            tool.name,
            serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default(),
            tool.description
        ));
    }
    tool_info
}

/// Convert messages containing ToolRequest/ToolResponse to text messages for toolshim mode
/// This is necessary because some providers (like Bedrock) validate that tool_use/tool_result
/// blocks can only exist when tools are defined, but in toolshim mode we pass empty tools
pub fn convert_tool_messages_to_text(messages: &[Message]) -> Conversation {
    let converted_messages: Vec<Message> = messages
        .iter()
        .map(|message| {
            let mut new_content = Vec::new();
            let mut has_tool_content = false;

            for content in &message.content {
                match content {
                    MessageContent::ToolRequest(req) => {
                        has_tool_content = true;
                        // Convert tool request to text format
                        let text = if let Ok(tool_call) = &req.tool_call {
                            format!(
                                "Using tool: {}\n{{\n  \"name\": \"{}\",\n  \"arguments\": {}\n}}",
                                tool_call.name,
                                tool_call.name,
                                serde_json::to_string_pretty(&tool_call.arguments)
                                    .unwrap_or_default()
                            )
                        } else {
                            "Tool request failed".to_string()
                        };
                        new_content.push(MessageContent::text(text));
                    }
                    MessageContent::ToolResponse(res) => {
                        has_tool_content = true;
                        // Convert tool response to text format
                        let text = match &res.tool_result {
                            Ok(result) => {
                                let text_contents: Vec<String> = result
                                    .content
                                    .iter()
                                    .filter_map(|c| match c.deref() {
                                        RawContent::Text(t) => Some(t.text.clone()),
                                        _ => None,
                                    })
                                    .collect();
                                format!("Tool result:\n{}", text_contents.join("\n"))
                            }
                            Err(e) => format!("Tool error: {}", e),
                        };
                        new_content.push(MessageContent::text(text));
                    }
                    _ => {
                        // Keep other content types as-is
                        new_content.push(content.clone());
                    }
                }
            }

            if has_tool_content {
                Message::new(message.role.clone(), message.created, new_content)
            } else {
                message.clone()
            }
        })
        .collect();

    Conversation::new_unvalidated(converted_messages)
}

/// Modifies the system prompt to include tool usage instructions when tool interpretation is enabled
pub fn modify_system_prompt_for_tool_json(system_prompt: &str, tools: &[Tool]) -> String {
    let tool_info = format_tool_info(tools);

    format!(
        "{}\n\n{}\n\nBreak down your task into smaller steps and do one step and tool call at a time. Do not try to use multiple tools at once. If you want to use a tool, tell the user what tool to use by specifying the tool in this JSON format\n{{\n  \"name\": \"tool_name\",\n  \"arguments\": {{\n    \"parameter1\": \"value1\",\n    \"parameter2\": \"value2\"\n }}\n}}. After you get the tool result back, consider the result and then proceed to do the next step and tool call if required.",
        system_prompt, tool_info
    )
}

/// Helper function to augment a message with tool calls if any are detected
pub async fn augment_message_with_tool_calls<T: ToolInterpreter>(
    interpreter: &T,
    message: Message,
    tools: &[Tool],
) -> Result<Message, ProviderError> {
    // If there are no tools or the message is empty, return the original message
    if tools.is_empty() {
        return Ok(message);
    }

    // Extract and combine all text content blocks from the message.
    let content = message
        .content
        .iter()
        .filter_map(|content| {
            if let MessageContent::Text(text) = content {
                Some(text.text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if content.trim().is_empty() {
        return Ok(message);
    }

    let has_existing_tool_request = message
        .content
        .iter()
        .any(|content| matches!(content, MessageContent::ToolRequest(_)));

    let direct_tool_calls = parse_tokenized_tool_calls(&content, tools);
    if !direct_tool_calls.is_empty() {
        let cleaned = sanitize_message_after_tokenized_parse(message);
        return Ok(append_tool_calls_to_message(cleaned, direct_tool_calls));
    }

    let inline_json_tool_calls = parse_inline_json_tool_calls(&content, tools);
    if !inline_json_tool_calls.is_empty() {
        let cleaned = sanitize_message_after_json_tool_parse(message);
        return Ok(append_tool_calls_to_message(
            cleaned,
            inline_json_tool_calls,
        ));
    }

    if has_existing_tool_request {
        return Ok(sanitize_residual_markers(message));
    }

    // Use the interpreter to convert the content to tool calls
    let tool_calls = interpreter.interpret_to_tool_calls(&content, tools).await?;

    // If no tool calls were detected, sanitize any residual markers
    if tool_calls.is_empty() {
        return Ok(sanitize_residual_markers(message));
    }

    Ok(sanitize_residual_markers(append_tool_calls_to_message(
        message, tool_calls,
    )))
}

pub async fn augment_message_with_selected_tool_interpreter(
    message: Message,
    tools: &[Tool],
) -> Result<Message, ProviderError> {
    match get_toolshim_backend()? {
        ToolshimBackend::Ollama => {
            let interpreter = OllamaInterpreter::new()?;
            augment_message_with_tool_calls(&interpreter, message, tools).await
        }
        ToolshimBackend::Local => {
            let interpreter = LocalInterpreter::new()?;
            augment_message_with_tool_calls(&interpreter, message, tools).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingInterpreter;

    #[async_trait::async_trait]
    impl ToolInterpreter for FailingInterpreter {
        async fn interpret_to_tool_calls(
            &self,
            _content: &str,
            _tools: &[Tool],
        ) -> Result<Vec<CallToolRequestParams>, ProviderError> {
            Err(ProviderError::RequestFailed(
                "interpreter should not be called".to_string(),
            ))
        }
    }

    #[test]
    fn parses_toolshim_backend_values() {
        assert_eq!(
            parse_toolshim_backend("ollama").unwrap(),
            ToolshimBackend::Ollama
        );
        assert_eq!(
            parse_toolshim_backend("local").unwrap(),
            ToolshimBackend::Local
        );
        assert_eq!(
            parse_toolshim_backend("llama.cpp").unwrap(),
            ToolshimBackend::Local
        );
        assert!(parse_toolshim_backend("something-else").is_err());
    }

    #[test]
    fn resolves_local_interpreter_model_prefers_env() {
        let model = resolve_local_interpreter_model_from_sources(
            Some("env-model".to_string()),
            Some("config-model".to_string()),
        )
        .unwrap();
        assert_eq!(model, "env-model");
    }

    #[test]
    fn resolves_local_interpreter_model_uses_config_fallback() {
        let model =
            resolve_local_interpreter_model_from_sources(None, Some("config-model".to_string()))
                .unwrap();
        assert_eq!(model, "config-model");
    }

    #[test]
    fn resolves_local_interpreter_model_requires_source() {
        assert!(resolve_local_interpreter_model_from_sources(None, None).is_err());
    }

    #[test]
    fn parses_tokenized_tool_calls() {
        let tools = vec![Tool::new(
            "shell".to_string(),
            "Shell command execution".to_string(),
            serde_json::Map::new(),
        )];

        let content = "<|tool_calls_section_begin|> <|tool_call_begin|> functions.shell:0 <|tool_call_argument_begin|> {\"command\":\"cat Cargo.toml\"} <|tool_call_end|> <|tool_calls_section_end|>";
        let calls = parse_tokenized_tool_calls(content, &tools);

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(
            calls[0]
                .arguments
                .as_ref()
                .and_then(|a| a.get("command"))
                .and_then(|v| v.as_str()),
            Some("cat Cargo.toml")
        );
    }

    #[test]
    fn parses_execute_marker_and_converts_to_shell_call() {
        let tools = vec![Tool::new(
            "shell".to_string(),
            "Shell command execution".to_string(),
            serde_json::Map::new(),
        )];

        let content = "<|tool_calls_section_begin|> <|tool_call_begin|> functions.execute:0 <|tool_call_argument_begin|> {\"code\":\"async function run() { const result = await Developer.shell({ command: \\\"cat Cargo.toml\\\" }); return result; }\"} <|tool_call_end|> <|tool_calls_section_end|>";

        let calls = parse_tokenized_tool_calls(content, &tools);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(
            calls[0]
                .arguments
                .as_ref()
                .and_then(|a| a.get("command"))
                .and_then(|v| v.as_str()),
            Some("cat Cargo.toml")
        );
    }

    #[test]
    fn parses_inline_json_tool_directive() {
        let tools = vec![Tool::new(
            "shell".to_string(),
            "Shell command execution".to_string(),
            serde_json::Map::new(),
        )];

        let content = "Using tool: shell\n{\n  \"name\": \"shell\",\n  \"arguments\": {\n    \"command\": \"type Cargo.toml\"\n  }\n}";
        let calls = parse_inline_json_tool_calls(content, &tools);

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(
            calls[0]
                .arguments
                .as_ref()
                .and_then(|a| a.get("command"))
                .and_then(|v| v.as_str()),
            Some("type Cargo.toml")
        );
    }

    #[test]
    fn parses_tokenized_tool_call_with_windows_path_arguments() {
        let tools = vec![Tool::new(
            "tree".to_string(),
            "Directory tree".to_string(),
            serde_json::Map::new(),
        )];

        let content = "<|tool_calls_section_begin|> <|tool_call_begin|> functions.tree:0 <|tool_call_argument_begin|> {\"path\": \"C:\\Users\\eugen\\programmazione\\goose-fork\", \"depth\": 1} <|tool_call_end|> <|tool_calls_section_end|>";
        let calls = parse_tokenized_tool_calls(content, &tools);

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "tree");
        assert_eq!(
            calls[0]
                .arguments
                .as_ref()
                .and_then(|a| a.get("path"))
                .and_then(|v| v.as_str()),
            Some("C:\\Users\\eugen\\programmazione\\goose-fork")
        );
    }

    #[tokio::test]
    async fn augment_uses_direct_tokenized_parser_before_interpreter() {
        let tools = vec![Tool::new(
            "shell".to_string(),
            "Shell command execution".to_string(),
            serde_json::Map::new(),
        )];

        let message = Message::assistant().with_text(
            "<|tool_calls_section_begin|> <|tool_call_begin|> functions.shell:0 <|tool_call_argument_begin|> {\"command\":\"cat Cargo.toml\"} <|tool_call_end|> <|tool_calls_section_end|>",
        );

        let augmented = augment_message_with_tool_calls(&FailingInterpreter, message, &tools)
            .await
            .unwrap();

        assert!(augmented
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_))));
        assert!(!augmented.as_concat_text().contains("<|tool_call_begin|>"));
    }

    #[tokio::test]
    async fn augment_parses_inline_json_even_with_existing_tool_request() {
        let tools = vec![
            Tool::new(
                "analyze".to_string(),
                "Analyze files".to_string(),
                serde_json::Map::new(),
            ),
            Tool::new(
                "shell".to_string(),
                "Shell command execution".to_string(),
                serde_json::Map::new(),
            ),
        ];

        let message = Message::assistant()
            .with_tool_request("existing", Ok(CallToolRequestParams::new("analyze")))
            .with_text(
                "Using tool: shell\n{\n  \"name\": \"shell\",\n  \"arguments\": {\n    \"command\": \"type Cargo.toml\"\n  }\n}",
            );

        let augmented = augment_message_with_tool_calls(&FailingInterpreter, message, &tools)
            .await
            .unwrap();

        let tool_request_count = augmented
            .content
            .iter()
            .filter(|c| matches!(c, MessageContent::ToolRequest(_)))
            .count();
        assert_eq!(tool_request_count, 2);
    }

    #[tokio::test]
    async fn augment_parses_tokenized_tool_call_from_later_text_chunk() {
        let tools = vec![Tool::new(
            "shell".to_string(),
            "Shell command execution".to_string(),
            serde_json::Map::new(),
        )];

        let message = Message::assistant()
            .with_text("I will inspect the file now.")
            .with_text(
                "<|tool_calls_section_begin|> <|tool_call_begin|> functions.shell:0 <|tool_call_argument_begin|> {\"command\":\"type Cargo.toml\"} <|tool_call_end|> <|tool_calls_section_end|>",
            );

        let augmented = augment_message_with_tool_calls(&FailingInterpreter, message, &tools)
            .await
            .unwrap();

        assert!(augmented
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_))));
    }

    // ── Regression tests: malformed marker leakage ──────────────────────

    /// Malformed tokenized markers (incomplete/garbled) must be stripped
    /// from the final text even when parsing yields zero tool calls.
    #[tokio::test]
    async fn malformed_tokenized_markers_stripped_from_text_output() {
        let tools = vec![Tool::new(
            "shell".to_string(),
            "Shell command execution".to_string(),
            serde_json::Map::new(),
        )];

        // Marker sequence is incomplete — no TOOL_CALL_ARGUMENT_BEGIN,
        // so parse_tokenized_tool_calls returns empty.
        let message = Message::assistant().with_text(
            "Here is the result.\n<|tool_calls_section_begin|> <|tool_call_begin|> functions.shell:0 GARBAGE <|tool_call_end|> <|tool_calls_section_end|>",
        );

        // Use an interpreter that returns empty (simulates no-match fallback)
        struct EmptyInterpreter;
        #[async_trait::async_trait]
        impl ToolInterpreter for EmptyInterpreter {
            async fn interpret_to_tool_calls(
                &self,
                _content: &str,
                _tools: &[Tool],
            ) -> Result<Vec<CallToolRequestParams>, ProviderError> {
                Ok(vec![])
            }
        }

        let result = augment_message_with_tool_calls(&EmptyInterpreter, message, &tools)
            .await
            .unwrap();

        let text = result.as_concat_text();
        assert!(
            !has_tool_markers(&text),
            "Residual tokenized markers leaked into output: {text}"
        );
    }

    /// Malformed JSON-style tool directives ("Using tool: …" without valid
    /// JSON) must be stripped from the final text.
    #[tokio::test]
    async fn malformed_json_directive_stripped_from_text_output() {
        let tools = vec![Tool::new(
            "shell".to_string(),
            "Shell command execution".to_string(),
            serde_json::Map::new(),
        )];

        // "Using tool:" present but no valid JSON follows
        let message = Message::assistant().with_text(
            "I will run the command.\nUsing tool: shell\n{invalid json that won't parse}",
        );

        struct EmptyInterpreter;
        #[async_trait::async_trait]
        impl ToolInterpreter for EmptyInterpreter {
            async fn interpret_to_tool_calls(
                &self,
                _content: &str,
                _tools: &[Tool],
            ) -> Result<Vec<CallToolRequestParams>, ProviderError> {
                Ok(vec![])
            }
        }

        let result = augment_message_with_tool_calls(&EmptyInterpreter, message, &tools)
            .await
            .unwrap();

        let text = result.as_concat_text();
        assert!(
            !has_tool_markers(&text),
            "Residual JSON tool directive leaked into output: {text}"
        );
    }

    #[test]
    fn has_tool_markers_detects_tokenized_markers() {
        assert!(has_tool_markers("hello <|tool_calls_section_begin|> world"));
        assert!(has_tool_markers("text <|tool_call_begin|> more"));
        assert!(!has_tool_markers("clean assistant text with no markers"));
    }

    #[test]
    fn has_tool_markers_detects_json_directive() {
        assert!(has_tool_markers("Using tool: shell\n{...}"));
        assert!(has_tool_markers("blah \"name\" blah \"arguments\" blah"));
        assert!(!has_tool_markers("just normal text mentioning a name"));
    }

    #[test]
    fn parses_tokenized_tool_call_without_argument_marker() {
        let tools = vec![Tool::new(
            "Nadirclawusage__usageSummary".to_string(),
            "Usage summary".to_string(),
            serde_json::Map::new(),
        )];

        // Model emits tool call without <|tool_call_argument_begin|>
        let content = "<|tool_calls_section_begin|> <|tool_call_begin|> functions.Nadirclawusage.usageSummary:1  {\"period\": \"24h\"} <|tool_call_end|> <|tool_calls_section_end|>";
        let calls = parse_tokenized_tool_calls(content, &tools);

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "Nadirclawusage__usageSummary");
        assert_eq!(
            calls[0]
                .arguments
                .as_ref()
                .and_then(|a| a.get("period"))
                .and_then(|v| v.as_str()),
            Some("24h")
        );
    }
}
