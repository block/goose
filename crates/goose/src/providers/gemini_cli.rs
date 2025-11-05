use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{filter_extensions_from_system_prompt, RequestLog};
use crate::conversation::message::{Message, MessageContent};

use crate::model::ModelConfig;
use rmcp::model::Role;
use rmcp::model::Tool;

pub const GEMINI_CLI_DEFAULT_MODEL: &str = "gemini-2.5-pro";
pub const GEMINI_CLI_KNOWN_MODELS: &[&str] = &["gemini-2.5-pro"];

pub const GEMINI_CLI_DOC_URL: &str = "https://ai.google.dev/gemini-api/docs";

#[derive(Debug, serde::Serialize)]
pub struct GeminiCliProvider {
    command: String,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl GeminiCliProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let command: String = config
            .get_param("GEMINI_CLI_COMMAND")
            .unwrap_or_else(|_| "gemini".to_string());

        let resolved_command = if !command.contains('/') {
            Self::find_gemini_executable(&command).unwrap_or(command)
        } else {
            command
        };

        Ok(Self {
            command: resolved_command,
            model,
            name: Self::metadata().name,
        })
    }

    /// Search for gemini executable in common installation locations
    fn find_gemini_executable(command_name: &str) -> Option<String> {
        let home = std::env::var("HOME").ok()?;

        // Common locations where gemini might be installed
        let search_paths = vec![
            format!("{}/.gemini/local/{}", home, command_name),
            format!("{}/.local/bin/{}", home, command_name),
            format!("{}/bin/{}", home, command_name),
            format!("/usr/local/bin/{}", command_name),
            format!("/usr/bin/{}", command_name),
            format!("/opt/gemini/{}", command_name),
            format!("/opt/google/{}", command_name),
        ];

        for path in search_paths {
            let path_buf = PathBuf::from(&path);
            if path_buf.exists() && path_buf.is_file() {
                // Check if it's executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&path_buf) {
                        let permissions = metadata.permissions();
                        if permissions.mode() & 0o111 != 0 {
                            tracing::info!("Found gemini executable at: {}", path);
                            return Some(path);
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    // On non-Unix systems, just check if file exists
                    tracing::info!("Found gemini executable at: {}", path);
                    return Some(path);
                }
            }
        }

        // If not found in common locations, check if it's in PATH
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(':') {
                let full_path = format!("{}/{}", dir, command_name);
                let path_buf = PathBuf::from(&full_path);
                if path_buf.exists() && path_buf.is_file() {
                    tracing::info!("Found gemini executable in PATH at: {}", full_path);
                    return Some(full_path);
                }
            }
        }

        tracing::warn!("Could not find gemini executable in common locations");
        None
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

        let filtered_system = filter_extensions_from_system_prompt(system);
        full_prompt.push_str(&filtered_system);
        full_prompt.push_str("\n\n");

        // Add conversation history
        for message in messages.iter().filter(|m| m.is_agent_visible()) {
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

        // Only pass model parameter if it's in the known models list
        if GEMINI_CLI_KNOWN_MODELS.contains(&self.model.model_name.as_str()) {
            cmd.arg("-m").arg(&self.model.model_name);
        }

        cmd.arg("-p").arg(&full_prompt).arg("--yolo");

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Gemini CLI command '{}': {}. \
                Make sure the Gemini CLI is installed and in your PATH.",
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
                    if !trimmed.is_empty() && !trimmed.starts_with("Loaded cached credentials") {
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

        let message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            vec![MessageContent::text(response_text)],
        );

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

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        // Return the model config with appropriate context limit for Gemini models
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, _model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Check if this is a session description request (short system prompt asking for 4 words or less)
        if system.contains("four words or less") || system.contains("4 words or less") {
            return self.generate_simple_session_description(messages);
        }

        // Create a dummy payload for debug tracing
        let payload = json!({
            "command": self.command,
            "model": self.model.model_name,
            "system": system,
            "messages": messages.len()
        });

        let mut log = RequestLog::start(&self.model, &payload).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to start request log: {}", e))
        })?;

        let lines = self.execute_command(system, messages, tools).await?;

        let (message, usage) = self.parse_response(&lines)?;

        let response = json!({
            "lines": lines.len(),
            "usage": usage
        });

        log.write(&response, Some(&usage)).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to write request log: {}", e))
        })?;

        Ok((
            message,
            ProviderUsage::new(self.model.model_name.clone(), usage),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gemini_cli_invalid_model_no_fallback() {
        // Test that an invalid model is kept as-is (no fallback)
        let invalid_model = ModelConfig::new_or_fail("invalid-model");
        let provider = GeminiCliProvider::from_env(invalid_model).await.unwrap();
        let config = provider.get_model_config();

        assert_eq!(config.model_name, "invalid-model");
    }

    #[tokio::test]
    async fn test_gemini_cli_valid_model() {
        // Test that a valid model is preserved
        let valid_model = ModelConfig::new_or_fail(GEMINI_CLI_DEFAULT_MODEL);
        let provider = GeminiCliProvider::from_env(valid_model).await.unwrap();
        let config = provider.get_model_config();

        assert_eq!(config.model_name, GEMINI_CLI_DEFAULT_MODEL);
    }
}
