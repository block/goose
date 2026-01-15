use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{filter_extensions_from_system_prompt, RequestLog};
use crate::config::base::{
    CodexCommand, CodexEnableSkills, CodexReasoningEffort, CodexSkipGitCheck,
};
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::subprocess::configure_command_no_window;
use rmcp::model::Role;
use rmcp::model::Tool;

pub const CODEX_DEFAULT_MODEL: &str = "gpt-5.2-codex";
pub const CODEX_KNOWN_MODELS: &[&str] = &[
    "gpt-5.2-codex",
    "gpt-5.2",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini",
];
pub const CODEX_DOC_URL: &str = "https://developers.openai.com/codex/cli";

/// Valid reasoning effort levels for Codex
pub const CODEX_REASONING_LEVELS: &[&str] = &["low", "medium", "high"];

#[derive(Debug, serde::Serialize)]
pub struct CodexProvider {
    command: PathBuf,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
    /// Reasoning effort level (low, medium, high)
    reasoning_effort: String,
    /// Whether to enable skills
    enable_skills: bool,
    /// Whether to skip git repo check
    skip_git_check: bool,
}

impl CodexProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = Config::global();
        let command: OsString = config.get_codex_command().unwrap_or_default().into();
        let resolved_command = SearchPaths::builder().with_npm().resolve(command)?;

        // Get reasoning effort from config, default to "high"
        let reasoning_effort = config
            .get_codex_reasoning_effort()
            .map(String::from)
            .unwrap_or_else(|_| "high".to_string());

        // Validate reasoning effort
        let reasoning_effort = if CODEX_REASONING_LEVELS.contains(&reasoning_effort.as_str()) {
            reasoning_effort
        } else {
            tracing::warn!(
                "Invalid CODEX_REASONING_EFFORT '{}', using 'high'",
                reasoning_effort
            );
            "high".to_string()
        };

        // Get enable_skills from config, default to true
        let enable_skills = config
            .get_codex_enable_skills()
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);

        // Get skip_git_check from config, default to false
        let skip_git_check = config
            .get_codex_skip_git_check()
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Self {
            command: resolved_command,
            model,
            name: Self::metadata().name,
            reasoning_effort,
            enable_skills,
            skip_git_check,
        })
    }

    /// Convert goose messages to a simple text prompt format
    /// Similar to Gemini CLI, we use Human:/Assistant: prefixes
    fn messages_to_prompt(&self, system: &str, messages: &[Message]) -> String {
        let mut full_prompt = String::new();

        let filtered_system = filter_extensions_from_system_prompt(system);
        if !filtered_system.is_empty() {
            full_prompt.push_str(&filtered_system);
            full_prompt.push_str("\n\n");
        }

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
        full_prompt
    }

    /// Apply permission flags based on GOOSE_MODE setting
    fn apply_permission_flags(cmd: &mut Command) -> Result<(), ProviderError> {
        let config = Config::global();
        let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

        match goose_mode {
            GooseMode::Auto => {
                // --yolo is shorthand for --dangerously-bypass-approvals-and-sandbox
                cmd.arg("--yolo");
            }
            GooseMode::SmartApprove => {
                // --full-auto applies workspace-write sandbox and approvals only on failure
                cmd.arg("--full-auto");
            }
            GooseMode::Approve => {
                // Default codex behavior - interactive approvals
                // No special flags needed
            }
            GooseMode::Chat => {
                // Read-only sandbox mode
                cmd.arg("--sandbox").arg("read-only");
            }
        }
        Ok(())
    }

    /// Execute codex CLI command
    async fn execute_command(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<Vec<String>, ProviderError> {
        let prompt = self.messages_to_prompt(system, messages);

        if std::env::var("GOOSE_CODEX_DEBUG").is_ok() {
            println!("=== CODEX PROVIDER DEBUG ===");
            println!("Command: {:?}", self.command);
            println!("Model: {}", self.model.model_name);
            println!("Reasoning effort: {}", self.reasoning_effort);
            println!("Enable skills: {}", self.enable_skills);
            println!("Skip git check: {}", self.skip_git_check);
            println!("Prompt length: {} chars", prompt.len());
            println!("Prompt: {}", prompt);
            println!("============================");
        }

        let mut cmd = Command::new(&self.command);
        configure_command_no_window(&mut cmd);

        // Use 'exec' subcommand for non-interactive mode
        cmd.arg("exec");

        // Only pass model parameter if it's in the known models list
        // This allows users to set GOOSE_PROVIDER=codex without needing to specify a model
        if CODEX_KNOWN_MODELS.contains(&self.model.model_name.as_str()) {
            cmd.arg("-m").arg(&self.model.model_name);
        }

        // Reasoning effort configuration
        cmd.arg("-c").arg(format!(
            "model_reasoning_effort=\"{}\"",
            self.reasoning_effort
        ));

        // Enable skills if configured
        if self.enable_skills {
            cmd.arg("--enable").arg("skills");
        }

        // JSON output format for structured parsing
        cmd.arg("--json");

        // Apply permission mode based on GOOSE_MODE
        Self::apply_permission_flags(&mut cmd)?;

        // Skip git repo check if configured
        if self.skip_git_check {
            cmd.arg("--skip-git-repo-check");
        }

        // Pass the prompt via stdin using '-' argument
        cmd.arg("-");

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Codex CLI command '{:?}': {}. \
                Make sure the Codex CLI is installed (npm i -g @openai/codex) \
                and available in the configured search paths.",
                self.command, e
            ))
        })?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(prompt.as_bytes()).await.map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to write to stdin: {}", e))
            })?;
            // Close stdin to signal end of input
            drop(stdin);
        }

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
                "Codex command failed with exit code: {:?}",
                exit_status.code()
            )));
        }

        tracing::debug!("Codex CLI executed successfully, got {} lines", lines.len());

        Ok(lines)
    }

    /// Extract text content from an item.completed event (agent_message only, skip reasoning)
    fn extract_text_from_item(item: &serde_json::Value) -> Option<String> {
        let item_type = item.get("type").and_then(|t| t.as_str());
        if item_type == Some("agent_message") {
            item.get("text")
                .and_then(|t| t.as_str())
                .filter(|text| !text.trim().is_empty())
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Extract usage information from a JSON object
    fn extract_usage(usage_info: &serde_json::Value, usage: &mut Usage) {
        if usage.input_tokens.is_none() {
            usage.input_tokens = usage_info
                .get("input_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
        }
        if usage.output_tokens.is_none() {
            usage.output_tokens = usage_info
                .get("output_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
        }
    }

    /// Extract error message from an error event
    fn extract_error(parsed: &serde_json::Value) -> Option<String> {
        parsed
            .get("message")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                parsed
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string())
            })
    }

    /// Extract text from legacy message formats
    fn extract_legacy_text(parsed: &serde_json::Value) -> Vec<String> {
        let mut texts = Vec::new();
        if let Some(content) = parsed.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    texts.push(text.to_string());
                }
            }
        }
        if let Some(text) = parsed.get("text").and_then(|t| t.as_str()) {
            texts.push(text.to_string());
        }
        if let Some(text) = parsed.get("result").and_then(|r| r.as_str()) {
            texts.push(text.to_string());
        }
        texts
    }

    /// Build fallback text from non-JSON lines
    fn build_fallback_text(lines: &[String]) -> Option<String> {
        let response_text: String = lines
            .iter()
            .filter(|line| {
                !line.starts_with('{')
                    || serde_json::from_str::<serde_json::Value>(line)
                        .map(|v| v.get("type").is_none())
                        .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if response_text.trim().is_empty() {
            None
        } else {
            Some(response_text)
        }
    }

    /// Parse newline-delimited JSON response from Codex CLI
    fn parse_response(&self, lines: &[String]) -> Result<(Message, Usage), ProviderError> {
        let mut all_text_content = Vec::new();
        let mut usage = Usage::default();
        let mut error_message: Option<String> = None;

        for line in lines {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(event_type) = parsed.get("type").and_then(|t| t.as_str()) {
                    match event_type {
                        "item.completed" => {
                            if let Some(item) = parsed.get("item") {
                                if let Some(text) = Self::extract_text_from_item(item) {
                                    all_text_content.push(text);
                                }
                            }
                        }
                        "turn.completed" | "result" | "done" => {
                            if let Some(usage_info) = parsed.get("usage") {
                                Self::extract_usage(usage_info, &mut usage);
                            }
                            all_text_content.extend(Self::extract_legacy_text(&parsed));
                        }
                        "error" | "turn.failed" => {
                            error_message = Self::extract_error(&parsed);
                        }
                        "message" | "assistant" => {
                            all_text_content.extend(Self::extract_legacy_text(&parsed));
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(err) = error_message {
            if all_text_content.is_empty() {
                return Err(ProviderError::RequestFailed(format!(
                    "Codex CLI error: {}",
                    err
                )));
            }
        }

        if all_text_content.is_empty() {
            if let Some(fallback) = Self::build_fallback_text(lines) {
                all_text_content.push(fallback);
            }
        }

        if let (Some(input), Some(output)) = (usage.input_tokens, usage.output_tokens) {
            usage.total_tokens = Some(input + output);
        }

        let combined_text = all_text_content.join("\n\n");
        if combined_text.is_empty() {
            return Err(ProviderError::RequestFailed(
                "Empty response from Codex CLI".to_string(),
            ));
        }

        let message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            vec![MessageContent::text(combined_text)],
        );

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

        if std::env::var("GOOSE_CODEX_DEBUG").is_ok() {
            println!("=== CODEX PROVIDER DEBUG ===");
            println!("Generated simple session description: {}", description);
            println!("Skipped subprocess call for session description");
            println!("============================");
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
impl Provider for CodexProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "codex",
            "OpenAI Codex CLI",
            "Execute OpenAI models via Codex CLI tool. Requires codex CLI installed.",
            CODEX_DEFAULT_MODEL,
            CODEX_KNOWN_MODELS.to_vec(),
            CODEX_DOC_URL,
            vec![
                ConfigKey::from_value_type::<CodexCommand>(true, false),
                ConfigKey::from_value_type::<CodexReasoningEffort>(false, false),
                ConfigKey::from_value_type::<CodexEnableSkills>(false, false),
                ConfigKey::from_value_type::<CodexSkipGitCheck>(false, false),
            ],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Check if this is a session description request
        if system.contains("four words or less") || system.contains("4 words or less") {
            return self.generate_simple_session_description(messages);
        }

        let lines = self.execute_command(system, messages, tools).await?;

        let (message, usage) = self.parse_response(&lines)?;

        // Create a payload for debug tracing
        let payload = json!({
            "command": self.command,
            "model": model_config.model_name,
            "reasoning_effort": self.reasoning_effort,
            "enable_skills": self.enable_skills,
            "system_length": system.len(),
            "messages_count": messages.len()
        });

        let mut log = RequestLog::start(model_config, &payload).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to start request log: {}", e))
        })?;

        let response = json!({
            "lines": lines.len(),
            "usage": usage
        });

        log.write(&response, Some(&usage)).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to write request log: {}", e))
        })?;

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_metadata() {
        let metadata = CodexProvider::metadata();
        assert_eq!(metadata.name, "codex");
        assert_eq!(metadata.default_model, CODEX_DEFAULT_MODEL);
        assert!(!metadata.known_models.is_empty());
        // Check that the default model is in the known models
        assert!(metadata
            .known_models
            .iter()
            .any(|m| m.name == CODEX_DEFAULT_MODEL));
    }

    #[test]
    fn test_messages_to_prompt_empty() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let prompt = provider.messages_to_prompt("", &[]);
        assert_eq!(prompt, "Assistant: ");
    }

    #[test]
    fn test_messages_to_prompt_with_system() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let prompt = provider.messages_to_prompt("You are a helpful assistant.", &[]);
        assert!(prompt.starts_with("You are a helpful assistant."));
        assert!(prompt.ends_with("Assistant: "));
    }

    #[test]
    fn test_messages_to_prompt_with_messages() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let messages = vec![
            Message::new(
                Role::User,
                chrono::Utc::now().timestamp(),
                vec![MessageContent::text("Hello")],
            ),
            Message::new(
                Role::Assistant,
                chrono::Utc::now().timestamp(),
                vec![MessageContent::text("Hi there!")],
            ),
        ];

        let prompt = provider.messages_to_prompt("", &messages);
        assert!(prompt.contains("Human: Hello"));
        assert!(prompt.contains("Assistant: Hi there!"));
    }

    #[test]
    fn test_parse_response_plain_text() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let lines = vec!["Hello, world!".to_string()];
        let result = provider.parse_response(&lines);
        assert!(result.is_ok());

        let (message, _usage) = result.unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert!(message.content.len() == 1);
    }

    #[test]
    fn test_parse_response_json_events() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        // Test with actual Codex CLI output format
        let lines = vec![
            r#"{"type":"thread.started","thread_id":"test-123"}"#.to_string(),
            r#"{"type":"turn.started"}"#.to_string(),
            r#"{"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"Thinking..."}}"#.to_string(),
            r#"{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"Hello there!"}}"#.to_string(),
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50,"cached_input_tokens":30}}"#.to_string(),
        ];
        let result = provider.parse_response(&lines);
        assert!(result.is_ok());

        let (message, usage) = result.unwrap();
        // Should only contain agent_message text, not reasoning
        if let MessageContent::Text(text) = &message.content[0] {
            assert!(text.text.contains("Hello there!"));
            assert!(!text.text.contains("Thinking"));
        }
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(usage.total_tokens, Some(150));
    }

    #[test]
    fn test_parse_response_empty() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let lines: Vec<String> = vec![];
        let result = provider.parse_response(&lines);
        assert!(result.is_err());
    }

    #[test]
    fn test_reasoning_level_validation() {
        assert!(CODEX_REASONING_LEVELS.contains(&"low"));
        assert!(CODEX_REASONING_LEVELS.contains(&"medium"));
        assert!(CODEX_REASONING_LEVELS.contains(&"high"));
        assert!(!CODEX_REASONING_LEVELS.contains(&"invalid"));
    }

    #[test]
    fn test_known_models() {
        assert!(CODEX_KNOWN_MODELS.contains(&"gpt-5.2-codex"));
        assert!(CODEX_KNOWN_MODELS.contains(&"gpt-5.2"));
        assert!(CODEX_KNOWN_MODELS.contains(&"gpt-5.1-codex-max"));
        assert!(CODEX_KNOWN_MODELS.contains(&"gpt-5.1-codex-mini"));
    }

    #[test]
    fn test_parse_response_item_completed() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let lines = vec![
            r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Hello from codex"}}"#.to_string(),
        ];
        let result = provider.parse_response(&lines);
        assert!(result.is_ok());

        let (message, _usage) = result.unwrap();
        if let MessageContent::Text(text) = &message.content[0] {
            assert!(text.text.contains("Hello from codex"));
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_parse_response_turn_completed_usage() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let lines = vec![
            r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Response"}}"#.to_string(),
            r#"{"type":"turn.completed","usage":{"input_tokens":5000,"output_tokens":100,"cached_input_tokens":3000}}"#.to_string(),
        ];
        let result = provider.parse_response(&lines);
        assert!(result.is_ok());

        let (_message, usage) = result.unwrap();
        assert_eq!(usage.input_tokens, Some(5000));
        assert_eq!(usage.output_tokens, Some(100));
        assert_eq!(usage.total_tokens, Some(5100));
    }

    #[test]
    fn test_parse_response_error_event() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let lines = vec![
            r#"{"type":"thread.started","thread_id":"test"}"#.to_string(),
            r#"{"type":"error","message":"Model not supported"}"#.to_string(),
        ];
        let result = provider.parse_response(&lines);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Model not supported"));
    }

    #[test]
    fn test_parse_response_skips_reasoning() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let lines = vec![
            r#"{"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"Let me think about this..."}}"#.to_string(),
            r#"{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"The answer is 42"}}"#.to_string(),
        ];
        let result = provider.parse_response(&lines);
        assert!(result.is_ok());

        let (message, _usage) = result.unwrap();
        if let MessageContent::Text(text) = &message.content[0] {
            assert!(text.text.contains("The answer is 42"));
            assert!(!text.text.contains("Let me think"));
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_session_description_generation() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let messages = vec![Message::new(
            Role::User,
            chrono::Utc::now().timestamp(),
            vec![MessageContent::text(
                "This is a very long message that should be truncated to four words",
            )],
        )];

        let result = provider.generate_simple_session_description(&messages);
        assert!(result.is_ok());

        let (message, _usage) = result.unwrap();
        if let MessageContent::Text(text) = &message.content[0] {
            // Should be truncated to 4 words
            let word_count = text.text.split_whitespace().count();
            assert!(word_count <= 4);
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_session_description_empty_messages() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let messages: Vec<Message> = vec![];

        let result = provider.generate_simple_session_description(&messages);
        assert!(result.is_ok());

        let (message, _usage) = result.unwrap();
        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Simple task");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_config_keys() {
        let metadata = CodexProvider::metadata();
        assert_eq!(metadata.config_keys.len(), 4);

        // First key should be CODEX_COMMAND (required)
        assert_eq!(metadata.config_keys[0].name, "CODEX_COMMAND");
        assert!(metadata.config_keys[0].required);
        assert!(!metadata.config_keys[0].secret);

        // Second key should be CODEX_REASONING_EFFORT (optional)
        assert_eq!(metadata.config_keys[1].name, "CODEX_REASONING_EFFORT");
        assert!(!metadata.config_keys[1].required);

        // Third key should be CODEX_ENABLE_SKILLS (optional)
        assert_eq!(metadata.config_keys[2].name, "CODEX_ENABLE_SKILLS");
        assert!(!metadata.config_keys[2].required);

        // Fourth key should be CODEX_SKIP_GIT_CHECK (optional)
        assert_eq!(metadata.config_keys[3].name, "CODEX_SKIP_GIT_CHECK");
        assert!(!metadata.config_keys[3].required);
    }

    #[test]
    fn test_messages_to_prompt_filters_non_text() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        // Create messages with both text and non-text content
        let messages = vec![Message::new(
            Role::User,
            chrono::Utc::now().timestamp(),
            vec![
                MessageContent::text("Hello"),
                // Tool requests would be filtered out as they're not text
            ],
        )];

        let prompt = provider.messages_to_prompt("System prompt", &messages);
        assert!(prompt.contains("System prompt"));
        assert!(prompt.contains("Human: Hello"));
    }

    #[test]
    fn test_parse_response_multiple_agent_messages() {
        let provider = CodexProvider {
            command: PathBuf::from("codex"),
            model: ModelConfig::new("gpt-5.2-codex").unwrap(),
            name: "codex".to_string(),
            reasoning_effort: "high".to_string(),
            enable_skills: true,
            skip_git_check: false,
        };

        let lines = vec![
            r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"First part"}}"#.to_string(),
            r#"{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"Second part"}}"#.to_string(),
        ];
        let result = provider.parse_response(&lines);
        assert!(result.is_ok());

        let (message, _usage) = result.unwrap();
        if let MessageContent::Text(text) = &message.content[0] {
            assert!(text.text.contains("First part"));
            assert!(text.text.contains("Second part"));
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_doc_url() {
        assert_eq!(CODEX_DOC_URL, "https://developers.openai.com/codex/cli");
    }

    #[test]
    fn test_default_model() {
        assert_eq!(CODEX_DEFAULT_MODEL, "gpt-5.2-codex");
    }
}
