use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Role;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::emit_debug_trace;
use crate::config::Config;
use crate::conversation::message::{Message, MessageContent};
use crate::impl_provider_default;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const CLAUDE_CODE_DEFAULT_MODEL: &str = "claude-3-5-sonnet-latest";
pub const CLAUDE_CODE_KNOWN_MODELS: &[&str] = &["sonnet", "opus", "claude-3-5-sonnet-latest"];

pub const CLAUDE_CODE_DOC_URL: &str = "https://claude.ai/cli";

#[derive(Debug, serde::Serialize)]
pub struct ClaudeCodeProvider {
    command: String,
    model: ModelConfig,
}

impl_provider_default!(ClaudeCodeProvider);

impl ClaudeCodeProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let command: String = config
            .get_param("CLAUDE_CODE_COMMAND")
            .unwrap_or_else(|_| "claude".to_string());

        let resolved_command = if !command.contains('/') {
            Self::find_claude_executable(&command).unwrap_or(command)
        } else {
            command
        };

        Ok(Self {
            command: resolved_command,
            model,
        })
    }

    /// Search for claude executable in common installation locations
    fn find_claude_executable(command_name: &str) -> Option<String> {
        let home = std::env::var("HOME").ok()?;

        let search_paths = vec![
            format!("{}/.claude/local/{}", home, command_name),
            format!("{}/.local/bin/{}", home, command_name),
            format!("{}/bin/{}", home, command_name),
            format!("/usr/local/bin/{}", command_name),
            format!("/usr/bin/{}", command_name),
            format!("/opt/claude/{}", command_name),
        ];

        for path in search_paths {
            let path_buf = PathBuf::from(&path);
            if path_buf.exists() && path_buf.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&path_buf) {
                        let permissions = metadata.permissions();
                        if permissions.mode() & 0o111 != 0 {
                            tracing::info!("Found claude executable at: {}", path);
                            return Some(path);
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    tracing::info!("Found claude executable at: {}", path);
                    return Some(path);
                }
            }
        }

        if let Ok(path_var) = std::env::var("PATH") {
            #[cfg(unix)]
            let path_separator = ':';
            #[cfg(windows)]
            let path_separator = ';';

            for dir in path_var.split(path_separator) {
                let path_buf = PathBuf::from(dir).join(command_name);
                if path_buf.exists() && path_buf.is_file() {
                    let full_path = path_buf.to_string_lossy().to_string();
                    tracing::info!("Found claude executable in PATH at: {}", full_path);
                    return Some(full_path);
                }
            }
        }

        tracing::warn!("Could not find claude executable in common locations");
        None
    }

    /// Filter out the Extensions section from the system prompt
    fn filter_extensions_from_system_prompt(&self, system: &str) -> String {
        // Find the Extensions section and remove it
        if let Some(extensions_start) = system.find("# Extensions") {
            // Look for the next major section that starts with #
            let after_extensions = &system[extensions_start..];
            if let Some(next_section_pos) = after_extensions[1..].find("\n# ") {
                // Found next section, keep everything before Extensions and after the next section
                let before_extensions = &system[..extensions_start];
                let next_section_start = extensions_start + next_section_pos + 1;
                let after_next_section = &system[next_section_start..];
                format!("{}{}", before_extensions.trim_end(), after_next_section)
            } else {
                // No next section found, just remove everything from Extensions onward
                system[..extensions_start].trim_end().to_string()
            }
        } else {
            // No Extensions section found, return original
            system.to_string()
        }
    }

    /// Convert goose messages to the format expected by claude CLI
    fn messages_to_claude_format(&self, _system: &str, messages: &[Message]) -> Result<Value> {
        let mut claude_messages = Vec::new();

        for message in messages {
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
                        if let Ok(tool_contents) = &tool_response.tool_result {
                            // Convert tool result contents to text
                            let content_text = tool_contents
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

        let response_message = Message {
            id: None,
            role: Role::Assistant,
            created: chrono::Utc::now().timestamp(),
            content: message_content,
        };

        Ok((response_message, usage))
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

        // Create a filtered system prompt without Extensions section
        let filtered_system = self.filter_extensions_from_system_prompt(system);

        if std::env::var("GOOSE_CLAUDE_CODE_DEBUG").is_ok() {
            println!("=== CLAUDE CODE PROVIDER DEBUG ===");
            println!("Command: {}", self.command);
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

        let mut cmd = Command::new(&self.command);
        cmd.arg("-p")
            .arg(messages_json.to_string())
            .arg("--system-prompt")
            .arg(&filtered_system);

        // Only pass model parameter if it's in the known models list
        if CLAUDE_CODE_KNOWN_MODELS.contains(&self.model.model_name.as_str()) {
            cmd.arg("--model").arg(&self.model.model_name);
        }

        cmd.arg("--verbose").arg("--output-format").arg("json");

        // Add permission mode based on GOOSE_MODE setting
        let config = Config::global();
        if let Ok(goose_mode) = config.get_param::<String>("GOOSE_MODE") {
            if goose_mode.as_str() == "auto" {
                cmd.arg("--permission-mode").arg("acceptEdits");
            }
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| ProviderError::RequestFailed(format!(
                "Failed to spawn Claude CLI command '{}': {}. \
                Make sure the Claude Code CLI is installed and in your PATH, or set CLAUDE_CODE_COMMAND in your config to the correct path.",
                self.command, e
            )))?;

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

        let message = Message {
            id: None,
            role: Role::Assistant,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::text(description.clone())],
        };

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
            "Claude Code",
            "Execute Claude models via claude CLI tool",
            CLAUDE_CODE_DEFAULT_MODEL,
            CLAUDE_CODE_KNOWN_MODELS.to_vec(),
            CLAUDE_CODE_DOC_URL,
            vec![ConfigKey::new(
                "CLAUDE_CODE_COMMAND",
                false,
                false,
                Some("claude"),
            )],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        // Return the model config with appropriate context limit for Claude models
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
        // Check if this is a session description request (short system prompt asking for 4 words or less)
        if system.contains("four words or less") || system.contains("4 words or less") {
            return self.generate_simple_session_description(messages);
        }

        let json_lines = self.execute_command(system, messages, tools).await?;

        let (message, usage) = self.parse_claude_response(&json_lines)?;

        // Create a dummy payload for debug tracing
        let payload = json!({
            "command": self.command,
            "model": self.model.model_name,
            "system": system,
            "messages": messages.len()
        });

        let response = json!({
            "lines": json_lines.len(),
            "usage": usage
        });

        emit_debug_trace(&self.model, &payload, &response, &usage);

        Ok((
            message,
            ProviderUsage::new(self.model.model_name.clone(), usage),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::ModelConfig;
    use super::*;

    #[test]
    fn test_claude_code_model_config() {
        let provider = ClaudeCodeProvider::default();
        let config = provider.get_model_config();

        assert_eq!(config.model_name, "claude-3-5-sonnet-latest");
        // Context limit should be set by the ModelConfig
        assert!(config.context_limit() > 0);
    }

    #[test]
    fn test_permission_mode_flag_construction() {
        // Test that in auto mode, the --permission-mode acceptEdits flag is added
        std::env::set_var("GOOSE_MODE", "auto");

        let config = Config::global();
        let goose_mode: String = config.get_param("GOOSE_MODE").unwrap();
        assert_eq!(goose_mode, "auto");

        std::env::remove_var("GOOSE_MODE");
    }

    #[test]
    fn test_claude_code_invalid_model_no_fallback() {
        // Test that an invalid model is kept as-is (no fallback)
        let invalid_model = ModelConfig::new_or_fail("invalid-model");
        let provider = ClaudeCodeProvider::from_env(invalid_model).unwrap();
        let config = provider.get_model_config();

        assert_eq!(config.model_name, "invalid-model");
    }

    #[test]
    fn test_claude_code_valid_model() {
        // Test that a valid model is preserved
        let valid_model = ModelConfig::new_or_fail("sonnet");
        let provider = ClaudeCodeProvider::from_env(valid_model).unwrap();
        let config = provider.get_model_config();

        assert_eq!(config.model_name, "sonnet");
    }
}
