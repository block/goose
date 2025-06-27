use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::emit_debug_trace;
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use mcp_core::content::TextContent;
use mcp_core::tool::Tool;
use mcp_core::Role;

pub const GEMINI_CLI_DEFAULT_MODEL: &str = "default";
pub const GEMINI_CLI_KNOWN_MODELS: &[&str] = &["default"];

pub const GEMINI_CLI_DOC_URL: &str = "https://ai.google.dev/gemini-api/docs";

#[derive(Debug, serde::Serialize)]
pub struct GeminiCliProvider {
    command: String,
    model: ModelConfig,
}

impl Default for GeminiCliProvider {
    fn default() -> Self {
        let model = ModelConfig::new(GeminiCliProvider::metadata().default_model);
        GeminiCliProvider::from_env(model).expect("Failed to initialize Gemini CLI provider")
    }
}

impl GeminiCliProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let command = "gemini".to_string(); // Fixed command, no configuration needed

        Ok(Self { command, model })
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

    /// Execute gemini CLI command with simple text prompt
    async fn execute_command(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<Vec<String>, ProviderError> {
        // Create a simple prompt combining system + conversation
        let mut full_prompt = String::new();

        // Add system prompt
        let filtered_system = self.filter_extensions_from_system_prompt(system);
        full_prompt.push_str(&filtered_system);
        full_prompt.push_str("\n\n");

        // Add conversation history
        for message in messages {
            let role_prefix = match message.role {
                Role::User => "Human: ",
                Role::Assistant => "Assistant: ",
            };
            full_prompt.push_str(role_prefix);

            for content in &message.content {
                if let MessageContent::Text(text_content) = content {
                    full_prompt.push_str(&text_content.text);
                    full_prompt.push('\n');
                }
            }
            full_prompt.push('\n');
        }

        full_prompt.push_str("Assistant: ");

        if std::env::var("GOOSE_GEMINI_CLI_DEBUG").is_ok() {
            println!("=== GEMINI CLI PROVIDER DEBUG ===");
            println!("Command: {}", self.command);
            println!("Full prompt: {}", full_prompt);
            println!("================================");
        }

        let mut cmd = Command::new(&self.command);
        cmd.arg("-p").arg(&full_prompt).arg("--yolo");

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

        tracing::debug!(
            "Gemini CLI executed successfully, got {} lines",
            lines.len()
        );

        Ok(lines)
    }

    /// Parse simple text response
    fn parse_response(&self, lines: &[String]) -> Result<(Message, Usage), ProviderError> {
        // Join all lines into a single response
        let response_text = lines.join("\n");

        if response_text.trim().is_empty() {
            return Err(ProviderError::RequestFailed(
                "Empty response from gemini command".to_string(),
            ));
        }

        let message = Message {
            role: Role::Assistant,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: response_text,
                annotations: None,
            })],
        };

        let usage = Usage::default(); // No usage info available for gemini CLI

        Ok((message, usage))
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

        if std::env::var("GOOSE_GEMINI_CLI_DEBUG").is_ok() {
            println!("=== GEMINI CLI PROVIDER DEBUG ===");
            println!("Generated simple session description: {}", description);
            println!("Skipped subprocess call for session description");
            println!("================================");
        }

        let message = Message {
            role: Role::Assistant,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: description.clone(),
                annotations: None,
            })],
        };

        let usage = Usage::default();

        Ok((
            message,
            ProviderUsage::new(self.model.model_name.clone(), usage),
        ))
    }
}

#[async_trait]
impl Provider for GeminiCliProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "gemini-cli",
            "Gemini CLI",
            "Execute Gemini models via gemini CLI tool",
            GEMINI_CLI_DEFAULT_MODEL,
            GEMINI_CLI_KNOWN_MODELS.to_vec(),
            GEMINI_CLI_DOC_URL,
            vec![], // No configuration needed
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        // Return a custom config with 1M token limit for Gemini CLI
        ModelConfig::new("gemini-1.5-pro".to_string()).with_context_limit(Some(1_000_000))
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

        let lines = self.execute_command(system, messages, tools).await?;

        let (message, usage) = self.parse_response(&lines)?;

        // Create a dummy payload for debug tracing
        let payload = json!({
            "command": self.command,
            "model": self.model.model_name,
            "system": system,
            "messages": messages.len()
        });

        let response = json!({
            "lines": lines.len(),
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
    use super::*;

    #[test]
    fn test_gemini_cli_model_config() {
        let provider = GeminiCliProvider::default();
        let config = provider.get_model_config();

        assert_eq!(config.model_name, "gemini-1.5-pro");
        assert_eq!(config.context_limit(), 1_000_000);
    }
}
