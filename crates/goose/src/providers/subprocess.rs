use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::emit_debug_trace;
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use mcp_core::content::TextContent;
use mcp_core::tool::Tool;
use mcp_core::Role;

pub const SUBPROCESS_DEFAULT_MODEL: &str = "default";
pub const SUBPROCESS_KNOWN_MODELS: &[&str] = &["default"];

pub const SUBPROCESS_DOC_URL: &str = "https://claude.ai/cli";

#[derive(Debug, serde::Serialize)]
pub struct SubprocessProvider {
    command: String,
    model: ModelConfig,
}

impl Default for SubprocessProvider {
    fn default() -> Self {
        let model = ModelConfig::new(SubprocessProvider::metadata().default_model);
        SubprocessProvider::from_env(model).expect("Failed to initialize Subprocess provider")
    }
}

impl SubprocessProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let command: String = config
            .get_param("SUBPROCESS_COMMAND")
            .unwrap_or_else(|_| "claude".to_string());

        Ok(Self { command, model })
    }

    /// Create a simplified system prompt without Extensions section
    fn create_simplified_system_prompt(&self) -> String {
        let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        let current_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        format!(
            "You are a general-purpose AI agent called Goose, created by Block, the parent company of Square, CashApp, and Tidal. Goose is being developed as an open-source software project.

The current date is {}.

You are working in the directory: {}

You have access to your own built-in tools for file operations, shell commands, and other tasks. Use them as needed to help the user accomplish their goals.

# Response Guidelines

- Use Markdown formatting for all responses.
- Follow best practices for Markdown, including:
  - Using headers for organization.
  - Bullet points for lists.
  - Links formatted correctly.
- For code examples, use fenced code blocks with language identifiers.
- Ensure clarity, conciseness, and proper formatting to enhance readability and usability.",
            current_date, current_dir
        )
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
                                .filter_map(|content| content.as_text())
                                .collect::<Vec<_>>()
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
        let json_array: Vec<Value> = serde_json::from_str(&full_response)
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse JSON response: {}", e)))?;

        for parsed in json_array {
            if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                match msg_type {
                    "assistant" => {
                        if let Some(message) = parsed.get("message") {
                            // Extract text content from this assistant message
                            if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                                for item in content {
                                    if let Some(content_type) = item.get("type").and_then(|t| t.as_str()) {
                                        if content_type == "text" {
                                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
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
            return Err(ProviderError::RequestFailed("No text content found in response".to_string()));
        }

        let message_content = vec![MessageContent::Text(TextContent {
            text: combined_text,
            annotations: None,
        })];

        let response_message = Message {
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

        // Create a simplified system prompt without Extensions section
        let simplified_system = self.create_simplified_system_prompt();

        if std::env::var("GOOSE_SUBPROCESS_DEBUG").is_ok() {
            println!("=== SUBPROCESS PROVIDER DEBUG ===");
            println!("Command: {}", self.command);
            println!("Original system prompt length: {} chars", system.len());
            println!("Simplified system prompt: {}", simplified_system);
            println!("Messages JSON: {}", serde_json::to_string_pretty(&messages_json).unwrap_or_else(|_| "Failed to serialize".to_string()));
            println!("================================");
        }

        let mut cmd = Command::new(&self.command);
        cmd.arg("-p")
            .arg(messages_json.to_string())
            .arg("--system-prompt")
            .arg(&simplified_system)  // Use simplified prompt instead of original
            .arg("--verbose")
            .arg("--output-format")
            .arg("json");

        // Let claude CLI use its own configured model

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to spawn command: {}", e)))?;

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
}

#[async_trait]
impl Provider for SubprocessProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "subprocess",
            "Subprocess",
            "Execute AI models via command-line tools (e.g., claude CLI)",
            SUBPROCESS_DEFAULT_MODEL,
            SUBPROCESS_KNOWN_MODELS.to_vec(),
            SUBPROCESS_DOC_URL,
            vec![ConfigKey::new(
                "SUBPROCESS_COMMAND",
                false,
                false,
                Some("claude"),
            )],
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
