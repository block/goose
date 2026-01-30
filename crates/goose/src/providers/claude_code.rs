use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures;
use rmcp::model::Role;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{filter_extensions_from_system_prompt, RequestLog};
use crate::config::base::ClaudeCodeCommand;
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::subprocess::configure_command_no_window;
use rmcp::model::Tool;

pub const CLAUDE_CODE_DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
pub const CLAUDE_CODE_KNOWN_MODELS: &[&str] = &["sonnet", "opus"];
pub const CLAUDE_CODE_DOC_URL: &str = "https://code.claude.com/docs/en/setup";

#[derive(Debug, serde::Serialize)]
pub struct ClaudeCodeProvider {
    command: PathBuf,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl ClaudeCodeProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let command: OsString = config.get_claude_code_command().unwrap_or_default().into();
        let resolved_command = SearchPaths::builder().with_npm().resolve(command)?;

        Ok(Self {
            command: resolved_command,
            model,
            name: Self::metadata().name,
        })
    }

    /// Convert goose messages to the format expected by claude CLI
    fn messages_to_claude_format(&self, _system: &str, messages: &[Message]) -> Result<Value> {
        let mut claude_messages = Vec::new();

        for message in messages.iter().filter(|m| m.is_agent_visible()) {
            let role = match message.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };

            let mut content_parts = Vec::new();
            for content in &message.content {
                match content {
                    MessageContent::Text(text_content) => {
                        content_parts.push(json!({
                            "type": "text",
                            "text": text_content.text
                        }));
                    }
                    MessageContent::ToolRequest(tool_request) => {
                        if let Ok(tool_call) = &tool_request.tool_call {
                            content_parts.push(json!({
                                "type": "tool_use",
                                "id": tool_request.id,
                                "name": tool_call.name,
                                "input": tool_call.arguments
                            }));
                        }
                    }
                    MessageContent::ToolResponse(tool_response) => {
                        if let Ok(result) = &tool_response.tool_result {
                            // Convert tool result contents to text
                            let content_text = result
                                .content
                                .iter()
                                .filter_map(|content| match &content.raw {
                                    rmcp::model::RawContent::Text(text_content) => {
                                        Some(text_content.text.as_str())
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<&str>>()
                                .join("\n");

                            content_parts.push(json!({
                                "type": "tool_result",
                                "tool_use_id": tool_response.id,
                                "content": content_text
                            }));
                        }
                    }
                    _ => {
                        // Skip other content types for now
                    }
                }
            }

            claude_messages.push(json!({
                "role": role,
                "content": content_parts
            }));
        }

        Ok(json!(claude_messages))
    }

    /// Parse the JSON response from claude CLI
    fn apply_permission_flags(cmd: &mut Command) -> Result<(), ProviderError> {
        let config = Config::global();
        let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

        match goose_mode {
            GooseMode::Auto => {
                cmd.arg("--dangerously-skip-permissions");
            }
            GooseMode::SmartApprove => {
                cmd.arg("--permission-mode").arg("acceptEdits");
            }
            GooseMode::Approve => {
                return Err(ProviderError::RequestFailed(
                    "\n\n\n### NOTE\n\n\n \
                    Claude Code CLI provider does not support Approve mode.\n \
                    Please use Auto (which will run anything it needs to) or \
                    SmartApprove (most things will run or Chat Mode)\n\n\n"
                        .to_string(),
                ));
            }
            GooseMode::Chat => {
                // Chat mode doesn't need permission flags
            }
        }
        Ok(())
    }

    fn parse_claude_response(
        &self,
        json_lines: &[String],
    ) -> Result<(Message, Usage), ProviderError> {
        let mut all_text_content = Vec::new();
        let mut usage = Usage::default();

        // Join all lines and parse as a single JSON array
        let full_response = json_lines.join("");
        let json_array: Vec<Value> = serde_json::from_str(&full_response).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse JSON response: {}", e))
        })?;

        for parsed in json_array {
            if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                match msg_type {
                    "assistant" => {
                        if let Some(message) = parsed.get("message") {
                            // Extract text content from this assistant message
                            if let Some(content) = message.get("content").and_then(|c| c.as_array())
                            {
                                for item in content {
                                    if let Some(content_type) =
                                        item.get("type").and_then(|t| t.as_str())
                                    {
                                        if content_type == "text" {
                                            if let Some(text) =
                                                item.get("text").and_then(|t| t.as_str())
                                            {
                                                all_text_content.push(text.to_string());
                                            }
                                        }
                                        // Skip tool_use - those are claude CLI's internal tools
                                    }
                                }
                            }

                            // Extract usage information
                            if let Some(usage_info) = message.get("usage") {
                                usage.input_tokens = usage_info
                                    .get("input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .map(|v| v as i32);
                                usage.output_tokens = usage_info
                                    .get("output_tokens")
                                    .and_then(|v| v.as_i64())
                                    .map(|v| v as i32);

                                // Calculate total if not provided
                                if usage.total_tokens.is_none() {
                                    if let (Some(input), Some(output)) =
                                        (usage.input_tokens, usage.output_tokens)
                                    {
                                        usage.total_tokens = Some(input + output);
                                    }
                                }
                            }
                        }
                    }
                    "result" => {
                        // Extract additional usage info from result if available
                        if let Some(result_usage) = parsed.get("usage") {
                            if usage.input_tokens.is_none() {
                                usage.input_tokens = result_usage
                                    .get("input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .map(|v| v as i32);
                            }
                            if usage.output_tokens.is_none() {
                                usage.output_tokens = result_usage
                                    .get("output_tokens")
                                    .and_then(|v| v.as_i64())
                                    .map(|v| v as i32);
                            }
                        }
                    }
                    _ => {} // Ignore other message types
                }
            }
        }

        // Combine all text content into a single message
        let combined_text = all_text_content.join("\n\n");
        if combined_text.is_empty() {
            return Err(ProviderError::RequestFailed(
                "No text content found in response".to_string(),
            ));
        }

        let message_content = vec![MessageContent::text(combined_text)];

        let response_message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            message_content,
        );

        Ok((response_message, usage))
    }

    fn parse_streaming_event(line: &str) -> Option<(Option<String>, Option<Usage>)> {
        let parsed: Value = serde_json::from_str(line).ok()?;
        let event_type = parsed.get("type").and_then(|t| t.as_str())?;

        match event_type {
            "stream_event" => {
                let event = parsed.get("event")?;
                let inner_type = event.get("type").and_then(|t| t.as_str())?;

                match inner_type {
                    "content_block_delta" => {
                        let delta = event.get("delta")?;
                        if delta.get("type").and_then(|t| t.as_str()) == Some("text_delta") {
                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                return Some((Some(text.to_string()), None));
                            }
                        }
                        None
                    }
                    "message_delta" => {
                        if let Some(usage_info) = event.get("usage") {
                            let output_tokens = usage_info
                                .get("output_tokens")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32);
                            return Some((None, Some(Usage::new(None, output_tokens, None))));
                        }
                        None
                    }
                    "message_start" => {
                        if let Some(message) = event.get("message") {
                            if let Some(usage_info) = message.get("usage") {
                                let input_tokens = usage_info
                                    .get("input_tokens")
                                    .and_then(|v| v.as_i64())
                                    .map(|v| v as i32);
                                return Some((None, Some(Usage::new(input_tokens, None, None))));
                            }
                        }
                        None
                    }
                    _ => None,
                }
            }
            "result" => {
                if let Some(usage_info) = parsed.get("usage") {
                    let input_tokens = usage_info
                        .get("input_tokens")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32);
                    let output_tokens = usage_info
                        .get("output_tokens")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32);
                    return Some((None, Some(Usage::new(input_tokens, output_tokens, None))));
                }
                None
            }
            _ => None,
        }
    }

    fn build_command(
        &self,
        messages_json: &Value,
        filtered_system: &str,
        streaming: bool,
    ) -> Result<Command, ProviderError> {
        let mut cmd = Command::new(&self.command);
        configure_command_no_window(&mut cmd);
        cmd.arg("-p")
            .arg(messages_json.to_string())
            .arg("--system-prompt")
            .arg(filtered_system);

        if CLAUDE_CODE_KNOWN_MODELS.contains(&self.model.model_name.as_str()) {
            cmd.arg("--model").arg(&self.model.model_name);
        }

        if streaming {
            cmd.arg("--verbose")
                .arg("--output-format")
                .arg("stream-json")
                .arg("--include-partial-messages");
        } else {
            cmd.arg("--verbose").arg("--output-format").arg("json");
        }

        Self::apply_permission_flags(&mut cmd)?;

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        Ok(cmd)
    }

    async fn execute_command(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<Vec<String>, ProviderError> {
        let messages_json = self
            .messages_to_claude_format(system, messages)
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to format messages: {}", e))
            })?;

        let filtered_system = filter_extensions_from_system_prompt(system);

        if std::env::var("GOOSE_CLAUDE_CODE_DEBUG").is_ok() {
            println!("=== CLAUDE CODE PROVIDER DEBUG ===");
            println!("Command: {:?}", self.command);
            println!("Original system prompt length: {} chars", system.len());
            println!(
                "Filtered system prompt length: {} chars",
                filtered_system.len()
            );
            println!("Filtered system prompt: {}", filtered_system);
            println!(
                "Messages JSON: {}",
                serde_json::to_string_pretty(&messages_json)
                    .unwrap_or_else(|_| "Failed to serialize".to_string())
            );
            println!("================================");
        }

        let mut cmd = self.build_command(&messages_json, &filtered_system, false)?;

        let mut child = cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Claude CLI command '{:?}': {}.",
                self.command, e
            ))
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdout".to_string()))?;

        let mut reader = BufReader::new(stdout);
        let mut lines = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        lines.push(trimmed.to_string());
                    }
                }
                Err(e) => {
                    return Err(ProviderError::RequestFailed(format!(
                        "Failed to read output: {}",
                        e
                    )));
                }
            }
        }

        let exit_status = child.wait().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to wait for command: {}", e))
        })?;

        if !exit_status.success() {
            return Err(ProviderError::RequestFailed(format!(
                "Command failed with exit code: {:?}",
                exit_status.code()
            )));
        }

        tracing::debug!("Command executed successfully, got {} lines", lines.len());
        for (i, line) in lines.iter().enumerate() {
            tracing::debug!("Line {}: {}", i, line);
        }

        Ok(lines)
    }

    /// Generate a simple session description without calling subprocess
    fn generate_simple_session_description(
        &self,
        messages: &[Message],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Extract the first user message text
        let description = messages
            .iter()
            .find(|m| m.role == Role::User)
            .and_then(|m| {
                m.content.iter().find_map(|c| match c {
                    MessageContent::Text(text_content) => Some(&text_content.text),
                    _ => None,
                })
            })
            .map(|text| {
                // Take first few words, limit to 4 words
                text.split_whitespace()
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| "Simple task".to_string());

        if std::env::var("GOOSE_CLAUDE_CODE_DEBUG").is_ok() {
            println!("=== CLAUDE CODE PROVIDER DEBUG ===");
            println!("Generated simple session description: {}", description);
            println!("Skipped subprocess call for session description");
            println!("================================");
        }

        let message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            vec![MessageContent::text(description.clone())],
        );

        let usage = Usage::default();

        Ok((
            message,
            ProviderUsage::new(self.model.model_name.clone(), usage),
        ))
    }
}

#[async_trait]
impl Provider for ClaudeCodeProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "claude-code",
            "Claude Code CLI",
            "Requires claude CLI installed, no MCPs. Use Anthropic provider for full features.",
            CLAUDE_CODE_DEFAULT_MODEL,
            CLAUDE_CODE_KNOWN_MODELS.to_vec(),
            CLAUDE_CODE_DOC_URL,
            vec![ConfigKey::from_value_type::<ClaudeCodeCommand>(true, false)],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        // Return the model config with appropriate context limit for Claude models
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        _session_id: Option<&str>, // create_session == YYYYMMDD_N, but --session-id requires a UUID
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Check if this is a session description request (short system prompt asking for 4 words or less)
        if system.contains("four words or less") || system.contains("4 words or less") {
            return self.generate_simple_session_description(messages);
        }

        let json_lines = self.execute_command(system, messages, tools).await?;

        let (message, usage) = self.parse_claude_response(&json_lines)?;

        // Create a dummy payload for debug tracing
        let payload = json!({
            "command": self.command,
            "model": model_config.model_name,
            "system": system,
            "messages": messages.len()
        });
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = json!({
            "lines": json_lines.len(),
            "usage": usage
        });

        log.write(&response, Some(&usage))?;

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn stream(
        &self,
        _session_id: &str,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        if system.contains("four words or less") || system.contains("4 words or less") {
            let (message, usage) = self.generate_simple_session_description(messages)?;
            return Ok(Box::pin(futures::stream::once(async move {
                Ok((Some(message), Some(usage)))
            })));
        }

        let messages_json = self
            .messages_to_claude_format(system, messages)
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to format messages: {}", e))
            })?;

        let filtered_system = filter_extensions_from_system_prompt(system);

        let mut cmd = self.build_command(&messages_json, &filtered_system, true)?;

        let mut child = cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Claude CLI command '{:?}': {}.",
                self.command, e
            ))
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdout".to_string()))?;

        let stderr = child.stderr.take();

        let model_name = self.model.model_name.clone();

        let message_id = uuid::Uuid::new_v4().to_string();

        Ok(Box::pin(try_stream! {
            // Drain stderr in background to prevent the child process from blocking
            let stderr_task = stderr.map(|stderr| {
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stderr);
                    let mut content = String::new();
                    let _ = tokio::io::AsyncReadExt::read_to_string(&mut reader, &mut content).await;
                    content
                })
            });

            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut accumulated_usage = Usage::default();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            if let Some((text_opt, usage_opt)) = Self::parse_streaming_event(trimmed) {
                                if let Some(text) = text_opt {
                                    let mut partial_message = Message::new(
                                        Role::Assistant,
                                        chrono::Utc::now().timestamp(),
                                        vec![MessageContent::text(text)],
                                    );
                                    partial_message.id = Some(message_id.clone());
                                    yield (Some(partial_message), None);
                                }
                                if let Some(usage) = usage_opt {
                                    accumulated_usage.input_tokens = usage.input_tokens.or(accumulated_usage.input_tokens);
                                    accumulated_usage.output_tokens = usage.output_tokens.or(accumulated_usage.output_tokens);
                                    accumulated_usage.total_tokens = accumulated_usage.input_tokens
                                        .zip(accumulated_usage.output_tokens)
                                        .map(|(i, o)| i + o)
                                        .or(accumulated_usage.input_tokens)
                                        .or(accumulated_usage.output_tokens);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        Err(ProviderError::RequestFailed(format!(
                            "Failed to read streaming output: {e}"
                        )))?;
                    }
                }
            }

            let stderr_content = if let Some(task) = stderr_task {
                task.await.ok().unwrap_or_default()
            } else {
                String::new()
            };

            let exit_status = child.wait().await.map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to wait for command: {e}"))
            })?;

            if !exit_status.success() {
                let stderr_msg = if stderr_content.is_empty() {
                    String::new()
                } else {
                    format!(" Stderr: {}", stderr_content.trim())
                };
                Err(ProviderError::RequestFailed(format!(
                    "Command failed with exit code: {:?}.{stderr_msg}",
                    exit_status.code()
                )))?;
            }

            let provider_usage = ProviderUsage::new(model_name, accumulated_usage);
            yield (None, Some(provider_usage));
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_streaming_event() {
        let event = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}}"#;
        assert!(
            matches!(ClaudeCodeProvider::parse_streaming_event(event), Some((Some(text), None)) if text == "Hello")
        );

        let event = r#"{"type":"stream_event","event":{"type":"message_start","message":{"usage":{"input_tokens":100}}}}"#;
        assert!(
            matches!(ClaudeCodeProvider::parse_streaming_event(event), Some((None, Some(usage))) if usage.input_tokens == Some(100))
        );

        let event = r#"{"type":"stream_event","event":{"type":"message_delta","usage":{"output_tokens":50}}}"#;
        assert!(
            matches!(ClaudeCodeProvider::parse_streaming_event(event), Some((None, Some(usage))) if usage.output_tokens == Some(50))
        );

        let event = r#"{"type":"result","usage":{"input_tokens":100,"output_tokens":50}}"#;
        assert!(
            matches!(ClaudeCodeProvider::parse_streaming_event(event), Some((None, Some(usage))) if usage.input_tokens == Some(100) && usage.output_tokens == Some(50))
        );

        assert!(ClaudeCodeProvider::parse_streaming_event("not json").is_none());
        assert!(ClaudeCodeProvider::parse_streaming_event(r#"{"type":"unknown"}"#).is_none());
        assert!(ClaudeCodeProvider::parse_streaming_event(r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta"}}}"#).is_none());
    }
}
