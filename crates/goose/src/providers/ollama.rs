use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::toolshim::{augment_message_with_tool_calls, OllamaInterpreter};
use super::utils::{get_model, handle_response_openai_compat};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use indoc::formatdoc;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5";
// Ollama can run many models, we only provide the default
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[OLLAMA_DEFAULT_MODEL];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OllamaProvider::metadata().default_model);
        OllamaProvider::from_env(model).expect("Failed to initialize Ollama provider")
    }
}

impl OllamaProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get_param("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            model,
        })
    }

    /// Get the base URL for Ollama API calls
    fn get_base_url(&self) -> Result<Url, ProviderError> {
        // OLLAMA_HOST is sometimes just the 'host' or 'host:port' without a scheme
        let base = if self.host.starts_with("http://") || self.host.starts_with("https://") {
            self.host.clone()
        } else {
            format!("http://{}", self.host)
        };

        let mut base_url = Url::parse(&base)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Set the default port if missing
        let explicit_default_port = self.host.ends_with(":80") || self.host.ends_with(":443");
        if base_url.port().is_none() && !explicit_default_port {
            base_url.set_port(Some(OLLAMA_DEFAULT_PORT)).map_err(|_| {
                ProviderError::RequestFailed("Failed to set default port".to_string())
            })?;
        }

        Ok(base_url)
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = self.get_base_url()?;

        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self.client.post(url).json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }

    fn process_response(
        &self,
        payload: Value,
        response: Value,
        message: Message,
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Get usage information
        let usage = match get_usage(&response) {
            Ok(usage) => {
                tracing::info!(
                    "Got usage data: input_tokens={:?}, output_tokens={:?}, total_tokens={:?}",
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.total_tokens
                );
                usage
            }
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        tracing::info!("Using model: {}", model);
        super::utils::emit_debug_trace(self, &payload, &response, &usage);

        tracing::info!("Successfully completed request");
        Ok((message, ProviderUsage::new(model, usage)))
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "ollama",
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.iter().map(|&s| s.to_string()).collect(),
            OLLAMA_DOC_URL,
            vec![ConfigKey::new(
                "OLLAMA_HOST",
                true,
                false,
                Some(OLLAMA_HOST),
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
        // Transform the system message to replace developer instructions
        let modified_system = if let Some(dev_section) = system.split("## developer").nth(1) {
            if let (Some(start_idx), Some(end_idx)) = (
                dev_section.find("### Instructions"),
                dev_section.find("operating system:"),
            ) {
                let new_instructions = formatdoc! {r#"
        The Developer extension enables you to edit code files, execute shell commands, and capture screen/window content. These tools allow for various development and debugging workflows.
        Available Tools:
        1. Shell Execution (`developer__shell`)
        Executes commands in the shell and returns the combined output and error messages.
        Use cases:
        - Running scripts: `python script.py`
        - Installing dependencies: `pip install -r requirements.txt`
        - Checking system information: `uname -a`, `df -h`
        - Searching for files or text: **Use `rg` (ripgrep) instead of `find` or `ls -r`**
          - Find a file: `rg --files | rg example.py`
          - Search within files: `rg 'class Example'`
        Best Practices:
        - **Avoid commands with large output** (pipe them to a file if necessary).
        - **Run background processes** if they take a long time (e.g., `uvicorn main:app &`).
        - **git commands can be run on the shell, however if the git extension is installed, you should use the git tool instead.
        - **If the shell command is a rm, mv, or cp, you should verify with the user before running the command.
        2. Text Editor (`developer__text_editor`)
        Performs file-based operations such as viewing, writing, replacing text, and undoing edits.
        Commands:
        - view: Read the content of a file.
        - write: Create or overwrite a file. Caution: Overwrites the entire file!
        - str_replace: Replace a specific string in a file.
        - undo_edit: Revert the last edit.
        Example Usage:
        developer__text_editor(command="view", file_path="/absolute/path/to/file.py")
        developer__text_editor(command="write", file_path="/absolute/path/to/file.py", file_text="print('hello world')")
        developer__text_editor(command="str_replace", file_path="/absolute/path/to/file.py", old_str="hello world", new_str="goodbye world")
        developer__text_editor(command="undo_edit", file_path="/absolute/path/to/file.py")
        Protocol for Text Editor:
        For edit and replace commands, please verify what you are editing with the user before running the command.
        - User: "Please edit the file /absolute/path/to/file.py"
        - Assistant: "Ok sounds good, I'll be editing the file /absolute/path/to/file.py and creating modifications xyz to the file. Let me know whether you'd like to proceed."
        - User: "Yes, please proceed."
        - Assistant: "I've created the modifications xyz to the file /absolute/path/to/file.py"
        3. List Windows (`developer__list_windows`)
        Lists all visible windows with their titles.
        Use this to find window titles for screen capture.
        4. Screen Capture (`developer__screen_capture`)
        Takes a screenshot of a display or specific window.
        Options:
        - Capture display: `developer__screen_capture(display=0)`  # Main display
        - Capture window: `developer__screen_capture(window_title="Window Title")`
        To use tools, ask the user to execute the tools for you by requesting the tool use in the exact JSON format below. 

## Tool Call JSON Format
```json
{{
  "name": "tool_name",
  "arguments": {{
    "parameter1": "value1",
    "parameter2": "value2"
  }}
}}
```
        Info: at the start of the session, the user's directory is:
        "#};

                let before_dev = system.split("## developer").next().unwrap_or("");
                let after_marker = &dev_section[end_idx..];

                format!(
                    "{}## developer{}### Instructions\n{}{}",
                    before_dev,
                    &dev_section[..start_idx],
                    new_instructions,
                    after_marker
                )
            } else {
                system.to_string()
            }
        } else {
            system.to_string()
        };

        // Create initial messages with modified_system as the content of a user message
        // and add an assistant reply acknowledging it
        let mut initial_messages = vec![
            Message::user().with_text(&modified_system),
            Message::assistant()
                .with_text("I understand. I'm ready to help with any tasks or questions you have."),
        ];

        // Append the actual user messages
        initial_messages.extend_from_slice(messages);

        // Check if tool shim is enabled via environment variables
        let use_tool_shim = std::env::var("GOOSE_TOOLSHIM")
            .map(|val| val == "1" || val.to_lowercase() == "true")
            .unwrap_or(false);

        if use_tool_shim {
            tracing::info!("Using tool shim approach with Ollama");

            // Create request without tools
            tracing::info!("Creating request without tools for initial completion");
            let payload = create_request(
                &self.model,
                "", // No system prompt, using modified_system as user message content instead
                &initial_messages,
                &vec![], // No need to include tools since using tool shim
                &super::utils::ImageFormat::OpenAi,
            )?;

            let response = self.post(payload.clone()).await?;
            let mut message = response_to_message(response.clone())?;

            let base_url = match self.get_base_url() {
                Ok(url) => {
                    let url_str = url.to_string();
                    tracing::info!("Using interpreter base URL: {}", url_str);
                    url_str
                }
                Err(e) => {
                    tracing::error!("Failed to get base URL: {}", e);
                    return Err(e);
                }
            };

            let interpreter = OllamaInterpreter::new(base_url);
            message = match augment_message_with_tool_calls(&interpreter, message, tools).await {
                Ok(augmented) => {
                    tracing::info!("Successfully augmented message with tool calls");
                    augmented
                }
                Err(e) => {
                    tracing::error!("Failed to augment message with tool calls: {}", e);
                    return Err(e);
                }
            };

            // Process the response and return the result
            self.process_response(payload, response, message)
        } else {
            let payload = create_request(
                &self.model,
                "", // No system prompt, using modified_system as user message content instead
                &initial_messages,
                tools,
                &super::utils::ImageFormat::OpenAi,
            )?;

            let response = self.post(payload.clone()).await?;

            let message = response_to_message(response.clone())?;

            // Process the response and return the result
            self.process_response(payload, response, message)
        }
    }
}
