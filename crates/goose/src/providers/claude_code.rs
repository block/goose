use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use rmcp::model::Role;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use super::base::{ConfigKey, Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{filter_extensions_from_system_prompt, RequestLog};
use crate::config::base::ClaudeCodeCommand;
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::subprocess::configure_subprocess;
use rmcp::model::Tool;

const CLAUDE_CODE_PROVIDER_NAME: &str = "claude-code";
pub const CLAUDE_CODE_DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
pub const CLAUDE_CODE_KNOWN_MODELS: &[&str] = &["sonnet", "opus"];
pub const CLAUDE_CODE_DOC_URL: &str = "https://code.claude.com/docs/en/setup";

#[derive(Debug)]
struct CliProcess {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    #[allow(dead_code)]
    stderr_handle: tokio::task::JoinHandle<String>,
}

impl Drop for CliProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Spawns the Claude Code CLI (`claude`) as a persistent child process using
/// `--input-format stream-json --output-format stream-json`. The CLI stays alive
/// across turns, maintaining conversation state internally. Messages are sent as
/// NDJSON on stdin with content arrays supporting text and image blocks. Responses
/// are NDJSON on stdout (`assistant` + `result` events per turn).
#[derive(Debug, serde::Serialize)]
pub struct ClaudeCodeProvider {
    command: PathBuf,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
    #[serde(skip)]
    cli_process: tokio::sync::OnceCell<tokio::sync::Mutex<CliProcess>>,
}

impl ClaudeCodeProvider {
    /// Build content blocks from the last user message only — the CLI maintains
    /// conversation context internally per session_id.
    fn last_user_content_blocks(&self, messages: &[Message]) -> Vec<Value> {
        let msgs = match messages.iter().rev().find(|m| m.role == Role::User) {
            Some(msg) => std::slice::from_ref(msg),
            None => messages,
        };
        let mut blocks: Vec<Value> = Vec::new();
        for message in msgs.iter().filter(|m| m.is_agent_visible()) {
            let prefix = match message.role {
                Role::User => "Human: ",
                Role::Assistant => "Assistant: ",
            };
            let mut text_parts = Vec::new();
            for content in &message.content {
                match content {
                    MessageContent::Text(t) => text_parts.push(t.text.clone()),
                    MessageContent::Image(img) => {
                        if !text_parts.is_empty() {
                            blocks.push(json!({"type":"text","text":format!("{}{}", prefix, text_parts.join("\n"))}));
                            text_parts.clear();
                        }
                        blocks.push(json!({"type":"image","source":{"type":"base64","media_type":img.mime_type,"data":img.data}}));
                    }
                    MessageContent::ToolRequest(req) => {
                        if let Ok(call) = &req.tool_call {
                            text_parts.push(format!("[tool_use: {} id={}]", call.name, req.id));
                        }
                    }
                    MessageContent::ToolResponse(resp) => {
                        if let Ok(result) = &resp.tool_result {
                            let text: String = result
                                .content
                                .iter()
                                .filter_map(|c| match &c.raw {
                                    rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                                    _ => None,
                                })
                                .collect::<Vec<&str>>()
                                .join("\n");
                            text_parts.push(format!("[tool_result id={}] {}", resp.id, text));
                        }
                    }
                    _ => {}
                }
            }
            if !text_parts.is_empty() {
                blocks.push(
                    json!({"type":"text","text":format!("{}{}", prefix, text_parts.join("\n"))}),
                );
            }
        }
        blocks
    }

    fn apply_permission_flags(cmd: &mut Command) -> Result<(), ProviderError> {
        let config = Config::global();
        let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

        match goose_mode {
            GooseMode::Auto => {
                cmd.arg("--dangerously-skip-permissions");
            }
            GooseMode::SmartApprove => {
                cmd.arg("--permission-mode").arg("acceptEdits");
            }
            GooseMode::Approve => {
                return Err(ProviderError::RequestFailed(
                    "\n\n\n### NOTE\n\n\n \
                    Claude Code CLI provider does not support Approve mode.\n \
                    Please use Auto (which will run anything it needs to) or \
                    SmartApprove (most things will run or Chat Mode)\n\n\n"
                        .to_string(),
                ));
            }
            GooseMode::Chat => {
                // Chat mode doesn't need permission flags
            }
        }
        Ok(())
    }

    /// Parse NDJSON stream-json response from Claude CLI
    fn parse_claude_response(
        &self,
        json_lines: &[String],
    ) -> Result<(Message, Usage), ProviderError> {
        let mut all_text_content = Vec::new();
        let mut usage = Usage::default();

        for line in json_lines {
            if let Ok(parsed) = serde_json::from_str::<Value>(line) {
                match parsed.get("type").and_then(|t| t.as_str()) {
                    Some("assistant") => {
                        if let Some(message) = parsed.get("message") {
                            // Extract text content from this assistant message
                            if let Some(content) = message.get("content").and_then(|c| c.as_array())
                            {
                                for item in content {
                                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                        if let Some(text) =
                                            item.get("text").and_then(|t| t.as_str())
                                        {
                                            all_text_content.push(text.to_string());
                                        }
                                    }
                                    // Skip tool_use - those are claude CLI's internal tools
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
                    Some("result") => {
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
                    Some("error") => {
                        let error_msg = parsed
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown error");
                        if error_msg.contains("context") && error_msg.contains("exceeded") {
                            return Err(ProviderError::ContextLengthExceeded(
                                error_msg.to_string(),
                            ));
                        }
                        return Err(ProviderError::RequestFailed(format!(
                            "Claude CLI error: {}",
                            error_msg
                        )));
                    }
                    Some("system") => {} // Ignore system init events
                    _ => {}              // Ignore other event types
                }
            }
        }

        // Combine all text content into a single message
        let combined_text = all_text_content.join("\n\n");
        if combined_text.contains("Prompt is too long") {
            return Err(ProviderError::ContextLengthExceeded(combined_text));
        }
        if combined_text.is_empty() {
            return Err(ProviderError::RequestFailed(
                "No text content found in response".to_string(),
            ));
        }

        let message_content = vec![MessageContent::text(combined_text)];

        let response_message = Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            message_content,
        );

        Ok((response_message, usage))
    }

    async fn execute_command(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
        session_id: &str,
    ) -> Result<Vec<String>, ProviderError> {
        let filtered_system = filter_extensions_from_system_prompt(system);

        if std::env::var("GOOSE_CLAUDE_CODE_DEBUG").is_ok() {
            println!("=== CLAUDE CODE PROVIDER DEBUG ===");
            println!("Command: {:?}", self.command);
            println!("Original system prompt length: {} chars", system.len());
            println!(
                "Filtered system prompt length: {} chars",
                filtered_system.len()
            );
            println!("Filtered system prompt: {}", filtered_system);
            println!("================================");
        }

        // Spawn lazily on first call (OnceCell ensures exactly once)
        let process_mutex = self
            .cli_process
            .get_or_try_init(|| async {
                let mut cmd = Command::new(&self.command);
                // NO -p flag — persistent mode
                configure_subprocess(&mut cmd);
                cmd.arg("--input-format")
                    .arg("stream-json")
                    .arg("--output-format")
                    .arg("stream-json")
                    .arg("--verbose")
                    // System prompt is set once at process start. The provider
                    // instance is not reused across sessions with different prompts.
                    .arg("--system-prompt")
                    .arg(&filtered_system);

                // Only pass model parameter if it's in the known models list
                if CLAUDE_CODE_KNOWN_MODELS.contains(&self.model.model_name.as_str()) {
                    cmd.arg("--model").arg(&self.model.model_name);
                }

                // Add permission mode based on GOOSE_MODE setting
                Self::apply_permission_flags(&mut cmd)?;

                cmd.stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let mut child = cmd.spawn().map_err(|e| {
                    ProviderError::RequestFailed(format!(
                        "Failed to spawn Claude CLI command '{:?}': {}.",
                        self.command, e
                    ))
                })?;

                let stdin = child.stdin.take().ok_or_else(|| {
                    ProviderError::RequestFailed("Failed to capture stdin".to_string())
                })?;
                let stdout = child.stdout.take().ok_or_else(|| {
                    ProviderError::RequestFailed("Failed to capture stdout".to_string())
                })?;

                // Drain stderr concurrently to prevent pipe buffer deadlock
                let stderr = child.stderr.take();
                let stderr_handle = tokio::spawn(async move {
                    let mut output = String::new();
                    if let Some(mut stderr) = stderr {
                        use tokio::io::AsyncReadExt;
                        let _ = stderr.read_to_string(&mut output).await;
                    }
                    output
                });

                Ok::<_, ProviderError>(tokio::sync::Mutex::new(CliProcess {
                    child,
                    stdin,
                    reader: BufReader::new(stdout),
                    stderr_handle,
                }))
            })
            .await?;

        let mut process = process_mutex.lock().await;

        let blocks = self.last_user_content_blocks(messages);

        // Write NDJSON line to stdin
        let ndjson_line = build_stream_json_input(&blocks, session_id);
        process
            .stdin
            .write_all(ndjson_line.as_bytes())
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to write to stdin: {}", e))
            })?;
        process.stdin.write_all(b"\n").await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to write newline to stdin: {}", e))
        })?;

        // Read lines until we see a "result" event
        let mut lines = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            match process.reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF means the process died
                    return Err(ProviderError::RequestFailed(
                        "Claude CLI process terminated unexpectedly".to_string(),
                    ));
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    lines.push(trimmed.to_string());

                    // Check if this is a result event (end of turn)
                    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                        match parsed.get("type").and_then(|t| t.as_str()) {
                            Some("result") => break,
                            Some("error") => break,
                            _ => {}
                        }
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

fn build_stream_json_input(content_blocks: &[Value], session_id: &str) -> String {
    let msg = json!({"type":"user","session_id":session_id,"message":{"role":"user","content":content_blocks}});
    serde_json::to_string(&msg).expect("serializing JSON content blocks cannot fail")
}

#[async_trait]
impl ProviderDef for ClaudeCodeProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CLAUDE_CODE_PROVIDER_NAME,
            "Claude Code CLI",
            "Requires claude CLI installed, no MCPs. Use Anthropic provider for full features.",
            CLAUDE_CODE_DEFAULT_MODEL,
            CLAUDE_CODE_KNOWN_MODELS.to_vec(),
            CLAUDE_CODE_DOC_URL,
            vec![ConfigKey::from_value_type::<ClaudeCodeCommand>(true, false)],
        )
        .with_unlisted_models()
    }

    fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();
            let command: String = config.get_claude_code_command().unwrap_or_default().into();
            let resolved_command = SearchPaths::builder().with_npm().resolve(command)?;

            Ok(Self {
                command: resolved_command,
                model,
                name: CLAUDE_CODE_PROVIDER_NAME.to_string(),
                cli_process: tokio::sync::OnceCell::new(),
            })
        })
    }
}

#[async_trait]
impl Provider for ClaudeCodeProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        // Return the model config with appropriate context limit for Claude models
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(CLAUDE_CODE_KNOWN_MODELS
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
        session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Check if this is a session description request (short system prompt asking for 4 words or less)
        if system.contains("four words or less") || system.contains("4 words or less") {
            return self.generate_simple_session_description(messages);
        }

        // session_id is None before a session is created (e.g. model listing).
        let sid = session_id.unwrap_or("default");
        let json_lines = self.execute_command(system, messages, tools, sid).await?;

        let (message, usage) = self.parse_claude_response(&json_lines)?;

        // Create a dummy payload for debug tracing
        let payload = json!({
            "command": self.command,
            "model": model_config.model_name,
            "system": system,
            "messages": messages.len()
        });
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = json!({
            "lines": json_lines.len(),
            "usage": usage
        });

        log.write(&response, Some(&usage))?;

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose_test_support::session::TEST_SESSION_ID;
    use serde_json::json;
    use test_case::test_case;

    /// (role, text, optional (image_data, mime_type))
    type MsgSpec<'a> = (&'a str, &'a str, Option<(&'a str, &'a str)>);

    fn build_messages(specs: &[MsgSpec]) -> Vec<Message> {
        specs
            .iter()
            .map(|(role, text, image)| {
                let role = if *role == "user" {
                    Role::User
                } else {
                    Role::Assistant
                };
                let mut msg = Message::new(role, 0, vec![]);
                if !text.is_empty() {
                    msg = Message::new(msg.role.clone(), 0, vec![MessageContent::text(*text)]);
                }
                if let Some((data, mime)) = image {
                    msg.content.push(MessageContent::image(*data, *mime));
                }
                msg
            })
            .collect()
    }

    #[test_case(
        build_messages(&[]),
        &[]
        ; "empty"
    )]
    #[test_case(
        build_messages(&[("user", "Hello", None)]),
        &[json!({"type":"text","text":"Human: Hello"})]
        ; "single_user"
    )]
    #[test_case(
        build_messages(&[("user", "Hello", None), ("assistant", "Hi there!", None)]),
        &[json!({"type":"text","text":"Human: Hello"})]
        ; "picks_last_user_ignores_assistant"
    )]
    #[test_case(
        build_messages(&[("user", "First", None), ("assistant", "Reply", None), ("user", "Second", None)]),
        &[json!({"type":"text","text":"Human: Second"})]
        ; "multi_turn_picks_last_user"
    )]
    #[test_case(
        build_messages(&[("user", "Describe this", Some(("base64data", "image/png")))]),
        &[json!({"type":"text","text":"Human: Describe this"}),
          json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":"base64data"}})]
        ; "user_with_image"
    )]
    #[test_case(
        build_messages(&[("user", "", Some(("iVBORw0KGgo", "image/png")))]),
        &[json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":"iVBORw0KGgo"}})]
        ; "image_only"
    )]
    #[test_case(
        vec![Message::new(Role::Assistant, 0, vec![
            MessageContent::tool_request("call_123", Ok(rmcp::model::CallToolRequestParams {
                name: "developer__shell".into(),
                arguments: Some(serde_json::from_value(json!({"cmd": "ls"})).unwrap()),
                meta: None, task: None,
            }))
        ])],
        &[json!({"type":"text","text":"Assistant: [tool_use: developer__shell id=call_123]"})]
        ; "tool_request_no_user_fallback"
    )]
    #[test_case(
        vec![Message::new(Role::User, 0, vec![
            MessageContent::tool_response("call_123", Ok(rmcp::model::CallToolResult {
                content: vec![rmcp::model::Content::text("file1.txt\nfile2.txt")],
                is_error: None, structured_content: None, meta: None,
            }))
        ])],
        &[json!({"type":"text","text":"Human: [tool_result id=call_123] file1.txt\nfile2.txt"})]
        ; "tool_response"
    )]
    fn test_last_user_content_blocks(messages: Vec<Message>, expected: &[Value]) {
        let provider = make_provider();
        let blocks = provider.last_user_content_blocks(&messages);
        assert_eq!(blocks, expected);
    }

    #[test_case(
        &[json!({"type":"text","text":"Hello"})],
        json!({"type":"user","session_id":TEST_SESSION_ID,"message":{"role":"user","content":[{"type":"text","text":"Hello"}]}})
        ; "text_block"
    )]
    #[test_case(
        &[json!({"type":"text","text":"Look"}), json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":"abc"}})],
        json!({"type":"user","session_id":TEST_SESSION_ID,"message":{"role":"user","content":[{"type":"text","text":"Look"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"abc"}}]}})
        ; "text_and_image_blocks"
    )]
    fn test_build_stream_json_input(blocks: &[Value], expected: Value) {
        let line = build_stream_json_input(blocks, TEST_SESSION_ID);
        let parsed: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test_case(
        &[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"The answer is 2."}],"usage":{"input_tokens":100,"output_tokens":20}}}"#,
            r#"{"type":"result","subtype":"success","result":"The answer is 2.","session_id":"abc"}"#,
        ],
        "The answer is 2.",
        Some(100), Some(20)
        ; "assistant_with_usage"
    )]
    #[test_case(
        &[
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"First"},{"type":"text","text":"Second"}]}}"#,
        ],
        "First\n\nSecond",
        None, None
        ; "multiple_text_blocks"
    )]
    #[test_case(
        &[
            r#"{"type":"system","subtype":"init","session_id":"abc"}"#,
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello"}]}}"#,
            r#"{"type":"result","subtype":"success","result":"Hello","session_id":"abc"}"#,
        ],
        "Hello",
        None, None
        ; "system_init_filtered"
    )]
    fn test_parse_claude_response_ok(
        lines: &[&str],
        expected_text: &str,
        expected_input: Option<i32>,
        expected_output: Option<i32>,
    ) {
        let provider = make_provider();
        let lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let (message, usage) = provider.parse_claude_response(&lines).unwrap();
        assert_eq!(message.role, Role::Assistant);
        if let MessageContent::Text(t) = &message.content[0] {
            assert_eq!(t.text, expected_text);
        } else {
            panic!("expected text content");
        }
        assert_eq!(usage.input_tokens, expected_input);
        assert_eq!(usage.output_tokens, expected_output);
    }

    #[test_case(
        &[],
        ProviderError::RequestFailed("No text content found in response".into())
        ; "empty_lines"
    )]
    #[test_case(
        &[r#"{"type":"error","error":"context window exceeded"}"#],
        ProviderError::ContextLengthExceeded("context window exceeded".into())
        ; "context_length"
    )]
    #[test_case(
        &[r#"{"type":"error","error":"Model not supported"}"#],
        ProviderError::RequestFailed("Claude CLI error: Model not supported".into())
        ; "generic_error"
    )]
    #[test_case(
        &[r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Prompt is too long"}]}}"#],
        ProviderError::ContextLengthExceeded("Prompt is too long".into())
        ; "prompt_too_long_exact"
    )]
    fn test_parse_claude_response_err(lines: &[&str], expected: ProviderError) {
        let provider = make_provider();
        let lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            provider.parse_claude_response(&lines).unwrap_err(),
            expected
        );
    }

    fn make_provider() -> ClaudeCodeProvider {
        ClaudeCodeProvider {
            command: PathBuf::from("claude"),
            model: ModelConfig::new("sonnet").unwrap(),
            name: "claude-code".to_string(),
            cli_process: tokio::sync::OnceCell::new(),
        }
    }
}
