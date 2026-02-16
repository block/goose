use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use super::base::{Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage};
use super::cli_common::{error_from_event, extract_usage_tokens};
use super::errors::ProviderError;
use super::utils::{filter_extensions_from_system_prompt, RequestLog};
use crate::config::base::GeminiCliCommand;
use crate::config::search_path::SearchPaths;
use crate::config::Config;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::ConfigKey;
use crate::subprocess::configure_subprocess;
use futures::future::BoxFuture;
use rmcp::model::Role;
use rmcp::model::Tool;

const GEMINI_CLI_PROVIDER_NAME: &str = "gemini-cli";
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
    #[serde(skip)]
    cli_session_id: OnceLock<String>,
}

impl GeminiCliProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = Config::global();
        let command: String = config.get_gemini_cli_command().unwrap_or_default().into();
        let resolved_command = SearchPaths::builder().with_npm().resolve(&command)?;

        Ok(Self {
            command: resolved_command,
            model,
            name: GEMINI_CLI_PROVIDER_NAME.to_string(),
            cli_session_id: OnceLock::new(),
        })
    }

    fn session_id(&self) -> Option<&str> {
        self.cli_session_id.get().map(|s| s.as_str())
    }

    fn set_session_id(&self, sid: String) {
        let _ = self.cli_session_id.set(sid);
    }

    fn last_user_message_text(messages: &[Message]) -> String {
        messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.as_concat_text())
            .unwrap_or_default()
    }

    /// Build the prompt for the CLI invocation. When resuming a session the CLI
    /// maintains conversation context internally, so only the latest user
    /// message is needed. On the first turn (no session yet) the system prompt
    /// is prepended — there is typically only one user message at that point.
    fn build_prompt(&self, system: &str, messages: &[Message]) -> String {
        let user_text = Self::last_user_message_text(messages);

        if self.session_id().is_some() {
            user_text
        } else {
            let filtered_system = filter_extensions_from_system_prompt(system);
            if filtered_system.is_empty() {
                user_text
            } else {
                format!("{filtered_system}\n\n{user_text}")
            }
        }
    }

    fn build_command(&self, prompt: &str, model_name: &str) -> Command {
        let mut cmd = Command::new(&self.command);
        configure_subprocess(&mut cmd);

        if let Ok(path) = SearchPaths::builder().with_npm().path() {
            cmd.env("PATH", path);
        }

        cmd.arg("-m").arg(model_name);

        if let Some(sid) = self.session_id() {
            cmd.arg("-r").arg(sid);
        }

        cmd.arg("-p")
            .arg(prompt)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--yolo");

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        cmd
    }

    async fn execute_command(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
        model_name: &str,
    ) -> Result<Vec<Value>, ProviderError> {
        let prompt = self.build_prompt(system, messages);

        tracing::debug!(command = ?self.command, "Executing Gemini CLI command");

        let mut cmd = self.build_command(&prompt, model_name);

        let mut child = cmd.kill_on_drop(true).spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Gemini CLI command '{}': {e}. \
                Make sure the Gemini CLI is installed and available in the configured search paths.",
                self.command.display()
            ))
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdout".to_string()))?;

        // Drain stderr concurrently to avoid pipe deadlock
        let stderr_task = tokio::spawn(async move {
            let mut buf = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut buf).await;
            }
            (child, buf)
        });

        let mut reader = BufReader::new(stdout);
        let mut events = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<Value>(trimmed) {
                        Ok(parsed) => {
                            if parsed.get("type").and_then(|t| t.as_str()) == Some("init") {
                                if let Some(sid) = parsed.get("session_id").and_then(|s| s.as_str())
                                {
                                    self.set_session_id(sid.to_string());
                                }
                            }
                            events.push(parsed);
                        }
                        Err(_) => {
                            tracing::warn!(line = trimmed, "Non-JSON line in stream-json output");
                        }
                    }
                }
                Err(e) => {
                    return Err(ProviderError::RequestFailed(format!(
                        "Failed to read output: {e}"
                    )));
                }
            }
        }

        let (mut child, stderr_text) = stderr_task
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to read stderr: {e}")))?;

        let exit_status = child.wait().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to wait for command: {e}"))
        })?;

        if !exit_status.success() {
            let stderr_snippet = stderr_text.trim();
            let detail = if stderr_snippet.is_empty() {
                format!("exit code {:?}", exit_status.code())
            } else {
                format!("exit code {:?}: {stderr_snippet}", exit_status.code())
            };
            return Err(ProviderError::RequestFailed(format!(
                "Gemini CLI command failed ({detail})"
            )));
        }

        tracing::debug!(
            "Gemini CLI executed successfully, got {} events",
            events.len()
        );

        Ok(events)
    }

    fn parse_stream_json_response(events: &[Value]) -> Result<(Message, Usage), ProviderError> {
        let mut all_text_content = Vec::new();
        let mut usage = Usage::default();

        for parsed in events {
            match parsed.get("type").and_then(|t| t.as_str()) {
                Some("message") => {
                    if parsed.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                        if let Some(content) = parsed.get("content").and_then(|c| c.as_str()) {
                            if !content.is_empty() {
                                all_text_content.push(content.to_string());
                            }
                        }
                    }
                }
                Some("result") => {
                    if let Some(stats) = parsed.get("stats") {
                        usage = extract_usage_tokens(stats);
                    }
                }
                Some("error") => {
                    return Err(error_from_event("Gemini CLI", parsed));
                }
                _ => {}
            }
        }

        let combined_text = all_text_content.join("");
        if combined_text.is_empty() {
            return Err(ProviderError::RequestFailed(
                "No text content found in response".to_string(),
            ));
        }

        let message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            vec![MessageContent::text(combined_text)],
        );

        Ok((message, usage))
    }
}

impl ProviderDef for GeminiCliProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            GEMINI_CLI_PROVIDER_NAME,
            "Gemini CLI",
            "Execute Gemini models via gemini CLI tool",
            GEMINI_CLI_DEFAULT_MODEL,
            GEMINI_CLI_KNOWN_MODELS.to_vec(),
            GEMINI_CLI_DOC_URL,
            vec![ConfigKey::from_value_type::<GeminiCliCommand>(true, false)],
        )
        .with_unlisted_models()
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for GeminiCliProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(GEMINI_CLI_KNOWN_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect())
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        _session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        if super::cli_common::is_session_description_request(system) {
            return super::cli_common::generate_simple_session_description(
                &model_config.model_name,
                messages,
            );
        }

        let payload = json!({
            "command": self.command,
            "model": model_config.model_name,
            "system": system,
            "messages": messages.len()
        });

        let mut log = RequestLog::start(model_config, &payload).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to start request log: {e}"))
        })?;

        let events = self
            .execute_command(system, messages, tools, &model_config.model_name)
            .await?;
        let (message, usage) = Self::parse_stream_json_response(&events)?;

        let response = json!({
            "events": events.len(),
            "usage": usage
        });

        log.write(&response, Some(&usage)).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to write request log: {e}"))
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
    use serde_json::json;

    fn make_provider() -> GeminiCliProvider {
        GeminiCliProvider {
            command: PathBuf::from("gemini"),
            model: ModelConfig::new("gemini-2.5-pro").unwrap(),
            name: "gemini-cli".to_string(),
            cli_session_id: OnceLock::new(),
        }
    }

    #[test]
    fn test_parse_stream_json_response() {
        let events = vec![
            json!({"type":"init","session_id":"abc","model":"gemini-2.5-pro"}),
            json!({"type":"message","role":"user","content":"Hi"}),
            json!({"type":"message","role":"assistant","content":"Hello ","delta":true}),
            json!({"type":"message","role":"assistant","content":"there!","delta":true}),
            json!({"type":"result","status":"success","stats":{"input_tokens":20,"output_tokens":5,"total_tokens":25}}),
        ];
        let (message, usage) = GeminiCliProvider::parse_stream_json_response(&events).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.as_concat_text(), "Hello there!");
        assert_eq!(usage.input_tokens, Some(20));
        assert_eq!(usage.output_tokens, Some(5));

        let error_events = vec![
            json!({"type":"init","session_id":"abc"}),
            json!({"type":"error","error":"Rate limit exceeded"}),
        ];
        let err = GeminiCliProvider::parse_stream_json_response(&error_events).unwrap_err();
        assert!(err.to_string().contains("Rate limit exceeded"));

        let empty: Vec<Value> = vec![];
        assert!(GeminiCliProvider::parse_stream_json_response(&empty).is_err());
    }

    #[test]
    fn test_build_prompt_first_and_resume() {
        let provider = make_provider();
        let messages = vec![Message::new(
            Role::User,
            0,
            vec![MessageContent::text("Hello")],
        )];

        let prompt = provider.build_prompt("You are helpful.", &messages);
        assert!(prompt.contains("You are helpful."));
        assert!(prompt.contains("Hello"));

        provider.set_session_id("session-123".to_string());
        let messages = vec![
            Message::new(Role::User, 0, vec![MessageContent::text("Hello")]),
            Message::new(Role::Assistant, 0, vec![MessageContent::text("Hi!")]),
            Message::new(
                Role::User,
                0,
                vec![MessageContent::text("Follow up question")],
            ),
        ];
        let prompt = provider.build_prompt("You are helpful.", &messages);
        assert_eq!(prompt, "Follow up question");
    }
}
