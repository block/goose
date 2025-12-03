use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{filter_extensions_from_system_prompt, RequestLog};
use crate::config::base::GeminiCliCommand;
use crate::config::search_path::SearchPaths;
use crate::config::Config;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::ConfigKey;
use crate::subprocess::configure_command_no_window;
use rmcp::model::Role;
use rmcp::model::Tool;

pub const GEMINI_CLI_DEFAULT_MODEL: &str = "gemini-2.5-pro";
pub const GEMINI_CLI_KNOWN_MODELS: &[&str] = &[
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
];

pub const GEMINI_CLI_DOC_URL: &str = "https://ai.google.dev/gemini-api/docs";

#[derive(Debug, serde::Serialize)]
pub struct GeminiCliProvider {
    command: PathBuf,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl GeminiCliProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = Config::global();
        let command: OsString = config.get_gemini_cli_command().unwrap_or_default().into();
        let resolved_command = SearchPaths::builder().with_npm().resolve(command)?;

        Ok(Self {
            command: resolved_command,
            model,
            name: Self::metadata().name,
        })
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
            println!("Command: {:?}", self.command);
            println!("Full prompt: {}", full_prompt);
            println!("================================");
        }

        let mut cmd = Command::new(&self.command);
        configure_command_no_window(&mut cmd);

        if let Ok(path) = SearchPaths::builder().with_npm().path() {
            cmd.env("PATH", path);
        }

        // Only pass model parameter if it's in the known models list
        if GEMINI_CLI_KNOWN_MODELS.contains(&self.model.model_name.as_str()) {
            cmd.arg("-m").arg(&self.model.model_name);
        }

        if cfg!(windows) {
            let sanitized_prompt = full_prompt.replace("\r\n", "\\n").replace('\n', "\\n");

            cmd.arg("-p").arg(&sanitized_prompt).arg("--yolo");
        } else {
            cmd.arg("-p").arg(&full_prompt).arg("--yolo");
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Gemini CLI command '{:?}': {}. \
                Make sure the Gemini CLI is installed and available in the configured search paths.",
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
            vec![ConfigKey::from_value_type::<GeminiCliCommand>(true, false)],
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
