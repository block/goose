use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use fm_rs::{GenerationOptions, Session, SystemLanguageModel};
use futures::future::BoxFuture;
use rmcp::model::{CallToolRequestParams, Role, Tool};
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::base::{
    MessageStream, ModelInfo, Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage,
};
use super::errors::ProviderError;
use super::local_inference::inference_emulated_tools::{
    build_emulator_tool_description, load_tiny_model_prompt, EmulatorAction,
    StreamingEmulatorParser,
};
use crate::config::ExtensionConfig;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::model::ModelConfig;

const PROVIDER_NAME: &str = "apple_fm";
const DEFAULT_MODEL: &str = "apple-intelligence";
const SHELL_TOOL: &str = "developer__shell";
const CODE_EXECUTION_TOOL: &str = "code_execution__execute";

/// Event sent from the blocking session thread to the async stream consumer.
enum SessionEvent {
    TextChunk(String),
    Done(ProviderUsage),
    Error(ProviderError),
}

/// Holds the channels for communicating with a persistent session thread.
struct SessionHandle {
    prompt_tx: mpsc::Sender<String>,
    event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SessionEvent>>>,
}

/// Extract the text of the latest user message (ignoring tool responses).
fn extract_latest_user_text(messages: &[Message]) -> String {
    for msg in messages.iter().rev() {
        if msg.role == Role::User {
            let text = msg.as_concat_text();
            if !text.is_empty() {
                return text;
            }
        }
    }
    String::new()
}

/// Check if the latest interaction is a tool result (i.e. we need to feed the
/// output back to the model rather than sending a new user prompt).
fn is_tool_result_continuation(messages: &[Message]) -> bool {
    for msg in messages.iter().rev() {
        for content in msg.content.iter().rev() {
            if content.as_tool_response().is_some() {
                return true;
            }
            if content.as_text().is_some() {
                return false;
            }
        }
    }
    false
}

/// Maximum characters to return to the fm_rs session from a tool result.
const MAX_TOOL_RESULT_CHARS: usize = 2000;

/// Truncate a tool result to fit within the on-device model's context budget.
fn truncate_tool_result(text: &str) -> String {
    if text.len() <= MAX_TOOL_RESULT_CHARS {
        return text.to_string();
    }
    let truncated: String = text.chars().take(MAX_TOOL_RESULT_CHARS).collect();
    format!(
        "{}\n\n[output truncated — {} total chars]",
        truncated,
        text.len()
    )
}

/// Extract the tool result text from the latest tool response in messages.
fn extract_tool_result_text(messages: &[Message]) -> String {
    for msg in messages.iter().rev() {
        for content in msg.content.iter().rev() {
            if let Some(text) = content.as_tool_response_text() {
                return text;
            }
        }
    }
    String::new()
}

/// Build the system prompt with tool descriptions baked in.
fn build_system_prompt(tools: &[Tool]) -> String {
    let code_mode_enabled = tools.iter().any(|t| t.name == CODE_EXECUTION_TOOL);
    let base = load_tiny_model_prompt(code_mode_enabled);
    let tool_desc = build_emulator_tool_description(tools, code_mode_enabled);
    format!("{base}{tool_desc}")
}

pub struct AppleFMProvider {
    model: ModelConfig,
    fm_model: Arc<SystemLanguageModel>,
    sessions: Arc<Mutex<HashMap<String, SessionHandle>>>,
}

impl ProviderDef for AppleFMProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            PROVIDER_NAME,
            "Apple Intelligence",
            "On-device inference via Apple's Foundation Models framework (macOS 26+)",
            DEFAULT_MODEL,
            vec![ModelInfo::new(DEFAULT_MODEL, fm_rs::DEFAULT_CONTEXT_TOKENS)],
            "https://developer.apple.com/documentation/FoundationModels",
            vec![],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self>> {
        Box::pin(async move {
            let fm_model = SystemLanguageModel::new()
                .map_err(|e| anyhow::anyhow!("Failed to create SystemLanguageModel: {}", e))?;
            if !fm_model.is_available() {
                anyhow::bail!(
                    "Apple Intelligence is not available on this device. \
                     Requires macOS 26+ with Apple Intelligence enabled."
                );
            }
            Ok(Self {
                model,
                fm_model: Arc::new(fm_model),
                sessions: Arc::new(Mutex::new(HashMap::new())),
            })
        })
    }
}

#[async_trait]
impl Provider for AppleFMProvider {
    fn get_name(&self) -> &str {
        PROVIDER_NAME
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec![DEFAULT_MODEL.to_string()])
    }

    async fn stream(
        &self,
        _model_config: &ModelConfig,
        session_id: &str,
        _system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let is_tool_result = is_tool_result_continuation(messages);

        // Build the prompt to send to the session
        let prompt = if is_tool_result {
            let result_text = extract_tool_result_text(messages);
            let truncated = truncate_tool_result(&result_text);
            tracing::debug!(
                original_len = result_text.len(),
                truncated_len = truncated.len(),
                "apple_fm: sending tool result as prompt"
            );
            format!("Output:\n{truncated}")
        } else {
            extract_latest_user_text(messages)
        };

        let event_rx;
        let prompt_tx;

        {
            let mut sessions = self.sessions.lock().unwrap();

            if let Some(handle) = sessions.get(session_id) {
                event_rx = Arc::clone(&handle.event_rx);
                prompt_tx = handle.prompt_tx.clone();
            } else {
                let (ptx, mut prompt_rx) = mpsc::channel::<String>(1);
                let (event_tx, erx) = mpsc::channel::<SessionEvent>(64);

                let fm_model = Arc::clone(&self.fm_model);
                let system = build_system_prompt(tools);
                let temperature = self.model.temperature;

                tracing::info!(
                    system_len = system.len(),
                    num_tools = tools.len(),
                    "apple_fm: creating session with emulated tool calling"
                );

                tokio::task::spawn_blocking(move || {
                    let session = Session::with_instructions(&fm_model, &system);

                    let session = match session {
                        Ok(s) => s,
                        Err(e) => {
                            let _ = event_tx.blocking_send(SessionEvent::Error(
                                ProviderError::ExecutionError(format!(
                                    "Failed to create FM session: {}",
                                    e
                                )),
                            ));
                            return;
                        }
                    };

                    let mut options_builder = GenerationOptions::builder();
                    if let Some(temp) = temperature {
                        options_builder = options_builder.temperature(temp as f64);
                    }
                    let options = options_builder.build();

                    while let Some(prompt) = prompt_rx.blocking_recv() {
                        tracing::debug!(
                            prompt_len = prompt.len(),
                            "apple_fm: sending prompt to session"
                        );

                        let tx = event_tx.clone();
                        let mut cumulative = String::new();
                        let result =
                            session.stream_response(&prompt, &options, move |chunk: &str| {
                                if chunk.len() > cumulative.len() {
                                    let delta = chunk
                                        .chars()
                                        .skip(cumulative.chars().count())
                                        .collect::<String>();
                                    if !delta.is_empty() {
                                        let _ = tx.blocking_send(SessionEvent::TextChunk(delta));
                                    }
                                    cumulative = chunk.to_string();
                                }
                            });

                        match result {
                            Ok(()) => {
                                let total_tokens = session
                                    .transcript_json()
                                    .ok()
                                    .and_then(|t| fm_rs::transcript_to_text(&t).ok())
                                    .and_then(|text| fm_model.token_usage_for(&text).ok())
                                    .map(|u| u.token_count as i32);

                                let usage = ProviderUsage::new(
                                    DEFAULT_MODEL.to_string(),
                                    Usage::new(total_tokens, None, total_tokens),
                                );
                                let _ = event_tx.blocking_send(SessionEvent::Done(usage));
                            }
                            Err(e) => {
                                match session.transcript_json() {
                                    Ok(transcript) => tracing::error!(
                                        error = %e,
                                        transcript = %transcript,
                                        "apple_fm: generation failed"
                                    ),
                                    Err(t_err) => tracing::error!(
                                        error = %e,
                                        transcript_error = %t_err,
                                        "apple_fm: generation failed (could not retrieve transcript)"
                                    ),
                                }
                                let _ = event_tx.blocking_send(SessionEvent::Error(
                                    ProviderError::ExecutionError(format!(
                                        "Generation failed: {}",
                                        e
                                    )),
                                ));
                            }
                        }
                    }
                });

                let shared_rx = Arc::new(tokio::sync::Mutex::new(erx));

                sessions.insert(
                    session_id.to_string(),
                    SessionHandle {
                        prompt_tx: ptx.clone(),
                        event_rx: Arc::clone(&shared_rx),
                    },
                );

                event_rx = shared_rx;
                prompt_tx = ptx;
            }
        }

        prompt_tx
            .send(prompt)
            .await
            .map_err(|_| ProviderError::ExecutionError("Session generation task died".into()))?;

        let message_id = Uuid::new_v4().to_string();
        let code_mode_enabled = tools.iter().any(|t| t.name == CODE_EXECUTION_TOOL);
        Ok(Box::pin(build_event_stream(
            event_rx,
            message_id,
            code_mode_enabled,
        )))
    }

    async fn generate_session_name(
        &self,
        _session_id: &str,
        messages: &Conversation,
    ) -> Result<String, ProviderError> {
        let context = self.get_initial_user_messages(messages);
        let prompt = format!(
            "Generate a short title (4 words or less) for a conversation that starts with:\n{}",
            context.join("\n")
        );

        let fm_model = Arc::clone(&self.fm_model);
        let result = tokio::task::spawn_blocking(move || {
            let session = Session::with_instructions(
                &fm_model,
                "You generate short titles. Respond with only the title, nothing else.",
            )
            .map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to create naming session: {}", e))
            })?;

            let options = GenerationOptions::builder().build();
            session
                .respond(&prompt, &options)
                .map(|r| r.into_content())
                .map_err(|e| ProviderError::ExecutionError(format!("Session naming failed: {}", e)))
        })
        .await
        .map_err(|e| ProviderError::ExecutionError(format!("Task join error: {}", e)))??;

        Ok(crate::utils::safe_truncate(result.trim(), 100))
    }
}

/// Build the async stream that consumes session events, parses emulated tool
/// calls from the text, and yields messages.
fn build_event_stream(
    event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SessionEvent>>>,
    message_id: String,
    code_mode_enabled: bool,
) -> impl futures::Stream<Item = Result<(Option<Message>, Option<ProviderUsage>), ProviderError>> {
    async_stream::try_stream! {
        let mut rx = event_rx.lock().await;
        let mut parser = StreamingEmulatorParser::new(code_mode_enabled);
        let mut tool_call_emitted = false;

        loop {
            match rx.recv().await {
                Some(SessionEvent::TextChunk(text)) => {
                    if text.trim() == "null" {
                        continue;
                    }
                    for action in parser.process_chunk(&text) {
                        if let Some((msg, is_tool)) = action_to_message(&action, &message_id) {
                            yield (Some(msg), None);
                            if is_tool {
                                tool_call_emitted = true;
                            }
                        }
                    }
                    if tool_call_emitted {
                        // Drain remaining events until Done so the session is ready
                        // for the next prompt, but don't yield them.
                        drain_until_done(&mut rx).await;
                        break;
                    }
                }
                Some(SessionEvent::Done(usage)) => {
                    for action in parser.flush() {
                        if let Some((msg, _)) = action_to_message(&action, &message_id) {
                            yield (Some(msg), None);
                        }
                    }
                    yield (None, Some(usage));
                    break;
                }
                Some(SessionEvent::Error(e)) => {
                    Err(e)?;
                }
                None => {
                    for action in parser.flush() {
                        if let Some((msg, _)) = action_to_message(&action, &message_id) {
                            yield (Some(msg), None);
                        }
                    }
                    break;
                }
            }
        }
    }
}

/// Drain session events until Done (or channel close) so the session thread
/// finishes its generation turn cleanly.
async fn drain_until_done(rx: &mut mpsc::Receiver<SessionEvent>) {
    loop {
        match rx.recv().await {
            Some(SessionEvent::Done(_)) | None => break,
            Some(SessionEvent::Error(_)) => break,
            Some(SessionEvent::TextChunk(_)) => continue,
        }
    }
}

/// Convert an emulator action into a message. Returns the message and whether
/// it's a tool call.
fn action_to_message(action: &EmulatorAction, message_id: &str) -> Option<(Message, bool)> {
    match action {
        EmulatorAction::Text(text) => {
            let mut msg = Message::assistant().with_text(text);
            msg.id = Some(message_id.to_string());
            Some((msg, false))
        }
        EmulatorAction::ShellCommand(command) => {
            let mut args = serde_json::Map::new();
            args.insert("command".to_string(), json!(command));
            let call = CallToolRequestParams {
                meta: None,
                task: None,
                name: Cow::Borrowed(SHELL_TOOL),
                arguments: Some(args),
            };
            let mut msg = Message::assistant();
            msg.id = Some(message_id.to_string());
            msg.content.push(MessageContent::tool_request(
                Uuid::new_v4().to_string(),
                Ok(call),
            ));
            Some((msg, true))
        }
        EmulatorAction::ExecuteCode(code) => {
            let wrapped = if code.contains("async function run()") {
                code.clone()
            } else {
                format!("async function run() {{\n{code}\n}}")
            };
            let mut args = serde_json::Map::new();
            args.insert("code".to_string(), json!(wrapped));
            let call = CallToolRequestParams {
                meta: None,
                task: None,
                name: Cow::Borrowed(CODE_EXECUTION_TOOL),
                arguments: Some(args),
            };
            let mut msg = Message::assistant();
            msg.id = Some(message_id.to_string());
            msg.content.push(MessageContent::tool_request(
                Uuid::new_v4().to_string(),
                Ok(call),
            ));
            Some((msg, true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let meta = AppleFMProvider::metadata();
        assert_eq!(meta.name, "apple_fm");
        assert_eq!(meta.default_model, "apple-intelligence");
        assert!(meta.config_keys.is_empty());
        assert_eq!(meta.known_models.len(), 1);
        assert_eq!(
            meta.known_models[0].context_limit,
            fm_rs::DEFAULT_CONTEXT_TOKENS
        );
    }

    #[test]
    fn test_extract_latest_user_text() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi"),
            Message::user().with_text("How are you?"),
        ];
        assert_eq!(extract_latest_user_text(&messages), "How are you?");
    }

    #[test]
    fn test_extract_latest_user_text_empty() {
        let messages = vec![Message::assistant().with_text("Hi")];
        assert_eq!(extract_latest_user_text(&messages), "");
    }

    #[test]
    fn test_is_tool_result_continuation() {
        let messages = vec![Message::user().with_text("hello")];
        assert!(!is_tool_result_continuation(&messages));

        let mut msg = Message::user();
        msg.content.push(MessageContent::tool_response(
            "id1".to_string(),
            Ok(rmcp::model::CallToolResult {
                content: vec![rmcp::model::Content::text("file1.rs")],
                structured_content: None,
                is_error: Some(false),
                meta: None,
            }),
        ));
        assert!(is_tool_result_continuation(&[msg]));
    }

    #[test]
    fn test_extract_tool_result_text() {
        let mut msg = Message::user();
        msg.content.push(MessageContent::tool_response(
            "id1".to_string(),
            Ok(rmcp::model::CallToolResult {
                content: vec![rmcp::model::Content::text("file1.rs\nfile2.rs")],
                structured_content: None,
                is_error: Some(false),
                meta: None,
            }),
        ));
        let text = extract_tool_result_text(&[msg]);
        assert!(text.contains("file1.rs"));
    }

    #[test]
    fn test_truncate_tool_result_short() {
        let short = "hello world";
        assert_eq!(truncate_tool_result(short), short);
    }

    #[test]
    fn test_truncate_tool_result_long() {
        let long = "x".repeat(3000);
        let result = truncate_tool_result(&long);
        assert!(result.len() < long.len());
        assert!(result.contains("[output truncated"));
    }

    #[test]
    fn test_action_to_message_text() {
        let action = EmulatorAction::Text("hello".to_string());
        let (msg, is_tool) = action_to_message(&action, "id1").unwrap();
        assert!(!is_tool);
        assert_eq!(msg.as_concat_text(), "hello");
    }

    #[test]
    fn test_action_to_message_shell() {
        let action = EmulatorAction::ShellCommand("ls -la".to_string());
        let (msg, is_tool) = action_to_message(&action, "id1").unwrap();
        assert!(is_tool);
        assert!(msg.content.iter().any(|c| c.as_tool_request().is_some()));
    }

    #[test]
    fn test_build_system_prompt_contains_tool_info() {
        let prompt = build_system_prompt(&[]);
        assert!(prompt.contains("goose"));
        assert!(prompt.contains("$ ls"));
    }
}
