use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::base::{
    stream_from_single_message, MessageStream, Provider, ProviderDef, ProviderMetadata,
    ProviderUsage, Usage,
};
use super::errors::ProviderError;
use super::utils::filter_extensions_from_system_prompt;
use crate::config::base::CopilotCliCommand;
use crate::config::search_path::SearchPaths;
use crate::config::Config;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::ConfigKey;
use crate::subprocess::configure_subprocess;
use async_stream::try_stream;
use futures::future::BoxFuture;
use rmcp::model::Role;
use rmcp::model::Tool;
use tokio::io::AsyncReadExt;

const COPILOT_CLI_PROVIDER_NAME: &str = "copilot-cli";
pub const COPILOT_CLI_DEFAULT_MODEL: &str = "gpt-4.1";
pub const COPILOT_CLI_KNOWN_MODELS: &[&str] = &[
    "gpt-4.1",
    "gpt-5",
    "gpt-5-mini",
    "claude-sonnet-4",
    "gemini-2.5-pro",
    "o4-mini",
];

pub const COPILOT_CLI_DOC_URL: &str =
    "https://docs.github.com/en/copilot/github-copilot-in-the-cli";

#[derive(Debug, serde::Serialize)]
pub struct CopilotCliProvider {
    command: PathBuf,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl CopilotCliProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = Config::global();
        let command: String = config.get_copilot_cli_command().unwrap_or_default().into();
        let resolved_command = SearchPaths::builder().with_npm().resolve(&command)?;

        Ok(Self {
            command: resolved_command,
            model,
            name: COPILOT_CLI_PROVIDER_NAME.to_string(),
        })
    }

    fn build_prompt(&self, system: &str, messages: &[Message]) -> String {
        let mut full_prompt = String::new();

        let filtered_system = filter_extensions_from_system_prompt(system);
        if !filtered_system.is_empty() {
            full_prompt.push_str(&filtered_system);
            full_prompt.push_str("\n\n");
        }

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

    fn build_command(&self, prompt: &str, model_name: &str) -> Command {
        let mut cmd = Command::new(&self.command);
        configure_subprocess(&mut cmd);

        if let Ok(path) = SearchPaths::builder().with_npm().path() {
            cmd.env("PATH", path);
        }

        if COPILOT_CLI_KNOWN_MODELS.contains(&model_name) {
            cmd.arg("--model").arg(model_name);
        }

        if cfg!(windows) {
            let sanitized = prompt.replace("\r\n", "\\n").replace('\n', "\\n");
            cmd.arg("-p").arg(&sanitized);
        } else {
            cmd.arg("-p").arg(prompt);
        }
        cmd.arg("--yolo").arg("--silent");

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        cmd
    }

    fn spawn_command(
        &self,
        system: &str,
        messages: &[Message],
        model_name: &str,
    ) -> Result<
        (
            tokio::process::Child,
            BufReader<tokio::process::ChildStdout>,
        ),
        ProviderError,
    > {
        let prompt = self.build_prompt(system, messages);

        tracing::debug!(command = ?self.command, "Executing Copilot CLI command");

        let mut cmd = self.build_command(&prompt, model_name);

        let mut child = cmd.kill_on_drop(true).spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Copilot CLI command '{}': {e}. \
                Make sure the GitHub Copilot CLI is installed and available in the configured search paths.",
                self.command.display()
            ))
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdout".to_string()))?;

        Ok((child, BufReader::new(stdout)))
    }
}

impl ProviderDef for CopilotCliProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            COPILOT_CLI_PROVIDER_NAME,
            "Copilot CLI",
            "Execute models via GitHub Copilot CLI tool. Requires copilot CLI installed.",
            COPILOT_CLI_DEFAULT_MODEL,
            COPILOT_CLI_KNOWN_MODELS.to_vec(),
            COPILOT_CLI_DOC_URL,
            vec![ConfigKey::from_value_type::<CopilotCliCommand>(
                true, false, true,
            )],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for CopilotCliProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(COPILOT_CLI_KNOWN_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect())
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        _session_id: &str,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        if super::cli_common::is_session_description_request(system) {
            let (message, provider_usage) = super::cli_common::generate_simple_session_description(
                &model_config.model_name,
                messages,
            )?;
            return Ok(stream_from_single_message(message, provider_usage));
        }

        let (mut child, mut reader) =
            self.spawn_command(system, messages, &model_config.model_name)?;
        let model_name = model_config.model_name.clone();
        let message_id = uuid::Uuid::new_v4().to_string();

        let stderr = child.stderr.take();
        let stderr_drain = tokio::spawn(async move {
            let mut buf = String::new();
            if let Some(mut stderr) = stderr {
                let _ = AsyncReadExt::read_to_string(&mut stderr, &mut buf).await;
            }
            buf
        });

        Ok(Box::pin(try_stream! {
            let mut line = String::new();
            let mut lines = Vec::new();
            let stream_timestamp = chrono::Utc::now().timestamp();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        if !line.trim().is_empty() {
                            let content = line.trim_end_matches('\n').trim_end_matches('\r');
                            lines.push(content.to_string());
                            // Yield partial text as it arrives, preserving leading whitespace
                            let mut partial = Message::new(
                                Role::Assistant,
                                stream_timestamp,
                                vec![MessageContent::text(content)],
                            );
                            partial.id = Some(message_id.clone());
                            yield (Some(partial), None);
                        }
                    }
                    Err(e) => {
                        let _ = child.wait().await;
                        Err(ProviderError::RequestFailed(format!(
                            "Failed to read streaming output: {e}"
                        )))?;
                    }
                }
            }

            let stderr_text = stderr_drain.await.unwrap_or_default();
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
                Err(ProviderError::RequestFailed(format!(
                    "Copilot CLI command failed ({detail})"
                )))?;
            }

            if lines.is_empty() {
                Err(ProviderError::RequestFailed(
                    "Empty response from copilot command".to_string(),
                ))?;
            }

            let provider_usage = ProviderUsage::new(model_name, Usage::default());
            yield (None, Some(provider_usage));
        }))
    }
}
