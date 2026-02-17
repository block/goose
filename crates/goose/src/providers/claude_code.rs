use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::future::BoxFuture;
use rmcp::model::{Role, Tool};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use super::base::{
    stream_from_single_message, ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata,
    ProviderUsage, Usage,
};
use super::errors::ProviderError;
use super::utils::filter_extensions_from_system_prompt;
use crate::config::base::ClaudeCodeCommand;
use crate::config::paths::Paths;
use crate::config::search_path::SearchPaths;
use crate::config::{Config, ExtensionConfig, GooseMode};
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::subprocess::configure_subprocess;

use super::cli_common::{error_from_event, extract_usage_tokens};

const CLAUDE_CODE_PROVIDER_NAME: &str = "claude-code";
pub const CLAUDE_CODE_DEFAULT_MODEL: &str = "default";
pub const CLAUDE_CODE_DOC_URL: &str = "https://code.claude.com/docs/en/setup";

struct CliProcess {
    child: tokio::process::Child,
    stdin: Box<dyn tokio::io::AsyncWrite + Unpin + Send>,
    reader: BufReader<Box<dyn tokio::io::AsyncRead + Unpin + Send>>,
    #[allow(dead_code)]
    stderr_handle: tokio::task::JoinHandle<String>,
    current_model: String,
    log_model_update: bool,
    next_request_id: u64,
    needs_drain: bool,
}

impl std::fmt::Debug for CliProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliProcess")
            .field("current_model", &self.current_model)
            .field("next_request_id", &self.next_request_id)
            .finish_non_exhaustive()
    }
}

impl CliProcess {
    fn next_request_id(&mut self) -> String {
        let id = self.next_request_id;
        self.next_request_id += 1;
        format!("req_{id}")
    }

    /// Send a `set_model` control request and wait for the response before returning.
    /// Skips the request if the model is already active.
    async fn send_set_model(&mut self, model: &str) -> Result<(), ProviderError> {
        if model == self.current_model {
            return Ok(());
        }

        let request_id = self.next_request_id();
        let req = json!({
            "type": "control_request",
            "request_id": request_id,
            "request": {"subtype": "set_model", "model": model}
        });
        let mut req_str = serde_json::to_string(&req).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to serialize set_model request: {e}"))
        })?;
        req_str.push('\n');
        self.stdin
            .write_all(req_str.as_bytes())
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to write set_model request: {e}"))
            })?;

        // Read lines until we get the control_response for our request.
        let mut line = String::new();
        loop {
            line.clear();
            match self.reader.read_line(&mut line).await {
                Ok(0) => {
                    return Err(ProviderError::RequestFailed(
                        "CLI process terminated while waiting for set_model response".to_string(),
                    ));
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                        if parsed.get("type").and_then(|t| t.as_str()) == Some("control_response") {
                            // Skip responses that don't match our request_id
                            if parsed
                                .pointer("/response/request_id")
                                .and_then(|id| id.as_str())
                                != Some(request_id.as_str())
                            {
                                continue;
                            }
                            let success =
                                parsed.pointer("/response/subtype").and_then(|s| s.as_str())
                                    == Some("success");
                            if success {
                                self.current_model = model.to_string();
                                self.log_model_update = true;
                                return Ok(());
                            } else {
                                let err = parsed
                                    .pointer("/response/error")
                                    .and_then(|e| e.as_str())
                                    .unwrap_or("unknown");
                                return Err(ProviderError::RequestFailed(format!(
                                    "set_model failed: {err}"
                                )));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(ProviderError::RequestFailed(format!(
                        "Failed to read set_model response: {e}"
                    )));
                }
            }
        }
    }

    async fn drain_pending_response(&mut self) {
        if !self.needs_drain {
            return;
        }
        tracing::debug!("Draining cancelled response from CLI process");

        let drain = async {
            let mut line = String::new();
            loop {
                line.clear();
                match self.reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                            match parsed.get("type").and_then(|t| t.as_str()) {
                                Some("result") | Some("error") => break,
                                _ => continue,
                            }
                        } else {
                            tracing::trace!(line = trimmed, "Non-JSON line during drain");
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        if tokio::time::timeout(DRAIN_TIMEOUT, drain).await.is_err() {
            // CLI is still producing the old response. Leave needs_drain
            // true so the next call retries — by then the old response
            // likely completed and drain will succeed quickly.
            tracing::warn!(
                "Drain did not complete in {DRAIN_TIMEOUT:?}; \
                 will retry on next request"
            );
            return;
        }

        self.needs_drain = false;
        tracing::debug!("Drain complete, protocol re-synced");
    }
}

impl Drop for CliProcess {
    fn drop(&mut self) {
        self.stderr_handle.abort();
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
    /// Temp file holding MCP config JSON (auto-deleted on drop).
    #[serde(skip)]
    mcp_config_file: Option<NamedTempFile>,
    #[serde(skip)]
    cli_process: tokio::sync::OnceCell<Arc<tokio::sync::Mutex<CliProcess>>>,
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

    fn build_stream_json_command(&self) -> Command {
        let mut cmd = Command::new(&self.command);
        configure_subprocess(&mut cmd);
        // Allow goose to run inside a Claude Code session.
        cmd.env_remove("CLAUDECODE");
        cmd.arg("--input-format")
            .arg("stream-json")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
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
            GooseMode::Chat => {}
        }
        Ok(())
    }

    fn spawn_process(&self, filtered_system: &str) -> Result<CliProcess, ProviderError> {
        let mut cmd = self.build_stream_json_command();

        if let Some(f) = &self.mcp_config_file {
            cmd.arg("--mcp-config").arg(f.path());
            cmd.arg("--strict-mcp-config");
        }

        cmd.arg("--include-partial-messages")
            .arg("--system-prompt")
            .arg(filtered_system)
            .arg("--model")
            .arg(&self.model.model_name);

        Self::apply_permission_flags(&mut cmd)?;

        let mut child = cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to spawn Claude CLI command '{:?}': {}.",
                self.command, e
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdout".to_string()))?;

        let stderr = child.stderr.take();
        let stderr_handle = tokio::spawn(async move {
            let mut output = String::new();
            if let Some(mut stderr) = stderr {
                use tokio::io::AsyncReadExt;
                let _ = stderr.read_to_string(&mut output).await;
            }
            output
        });

        Ok(CliProcess {
            child,
            stdin: Box::new(stdin),
            reader: BufReader::new(Box::new(stdout)),
            stderr_handle,
            current_model: self.model.model_name.clone(),
            log_model_update: false,
            next_request_id: 0,
            needs_drain: false,
        })
    }

    async fn get_or_init_process(
        &self,
        filtered_system: &str,
    ) -> Result<&Arc<tokio::sync::Mutex<CliProcess>>, ProviderError> {
        self.cli_process
            .get_or_try_init(|| async {
                Ok(Arc::new(tokio::sync::Mutex::new(
                    self.spawn_process(filtered_system)?,
                )))
            })
            .await
    }
}

/// Extract model aliases from the CLI's initialize control_response.
fn parse_models_from_lines(lines: &[String]) -> Vec<String> {
    for line in lines {
        if let Ok(parsed) = serde_json::from_str::<Value>(line) {
            if parsed.get("type").and_then(|t| t.as_str()) != Some("control_response") {
                continue;
            }
            let success =
                parsed.pointer("/response/subtype").and_then(|s| s.as_str()) == Some("success");
            if !success {
                continue;
            }
            if let Some(models) = parsed
                .pointer("/response/response/models")
                .and_then(|m| m.as_array())
            {
                return models
                    .iter()
                    .filter_map(|m| m.get("value").and_then(|v| v.as_str()).map(String::from))
                    .collect();
            }
        }
    }
    Vec::new()
}

fn build_stream_json_input(content_blocks: &[Value], session_id: &str) -> String {
    let msg = json!({"type":"user","session_id":session_id,"message":{"role":"user","content":content_blocks}});
    serde_json::to_string(&msg).expect("serializing JSON content blocks cannot fail")
}

fn claude_mcp_config_json(extensions: &[ExtensionConfig]) -> Option<String> {
    let mut mcp_servers = serde_json::Map::new();

    for extension in extensions {
        match extension {
            ExtensionConfig::StreamableHttp { uri, headers, .. } => {
                let key = extension.key();
                let mut config = serde_json::Map::new();
                config.insert("type".to_string(), json!("http"));
                config.insert("url".to_string(), json!(uri));
                if !headers.is_empty() {
                    config.insert("headers".to_string(), json!(headers));
                }
                mcp_servers.insert(key, Value::Object(config));
            }
            ExtensionConfig::Stdio {
                cmd, args, envs, ..
            } => {
                let key = extension.key();
                let mut config = serde_json::Map::new();
                config.insert("type".to_string(), json!("stdio"));
                config.insert("command".to_string(), json!(cmd));
                if !args.is_empty() {
                    config.insert("args".to_string(), json!(args));
                }
                let env_map = envs.get_env();
                if !env_map.is_empty() {
                    config.insert("env".to_string(), json!(env_map));
                }
                mcp_servers.insert(key, Value::Object(config));
            }
            ExtensionConfig::Sse { name, .. } => {
                tracing::debug!(name, "skipping SSE extension, migrate to streamable_http");
            }
            _ => {}
        }
    }

    if mcp_servers.is_empty() {
        return None;
    }

    serde_json::to_string(&json!({ "mcpServers": mcp_servers })).ok()
}

/// Write the MCP config JSON to a temp file with restricted permissions
/// so secrets (headers, env vars) are not leaked via process argv.
fn write_mcp_config_file(state_dir: &Path, json: &str) -> Result<NamedTempFile, anyhow::Error> {
    let dir = state_dir.join("claude-code");
    std::fs::create_dir_all(&dir)?;
    let prefix = format!("mcp-config-{}_", chrono::Utc::now().format("%Y%m%d"));
    let mut tmp = tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".json")
        .tempfile_in(&dir)?;
    tmp.write_all(json.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(tmp)
}

impl ProviderDef for ClaudeCodeProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CLAUDE_CODE_PROVIDER_NAME,
            "Claude Code CLI",
            "Requires claude CLI installed, no MCPs. Use Anthropic provider for full features.",
            CLAUDE_CODE_DEFAULT_MODEL,
            // Only a few agentic choices; fetched dynamically via fetch_supported_models.
            vec![],
            CLAUDE_CODE_DOC_URL,
            vec![ConfigKey::from_value_type::<ClaudeCodeCommand>(true, false)],
        )
        // The model list only returns aliases the `claude` CLI uses, such as "default"
        // and "haiku". There is no listing that includes full names like
        // "claude-sonnet-4-5-20250929". However, they are permitted.
        .with_unlisted_models()
    }

    fn from_env(
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();
            let command: String = config.get_claude_code_command().unwrap_or_default().into();
            let resolved_command = SearchPaths::builder().with_npm().resolve(command)?;

            let mut resolved = Vec::with_capacity(extensions.len());
            for ext in extensions {
                resolved.push(ext.resolve(config).await?);
            }

            let mcp_config_file = claude_mcp_config_json(&resolved)
                .map(|json| write_mcp_config_file(&Paths::state_dir(), &json))
                .transpose()?;

            Ok(Self {
                command: resolved_command,
                model,
                name: CLAUDE_CODE_PROVIDER_NAME.to_string(),
                mcp_config_file,
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
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        // Uses a separate short-lived process because --system-prompt is a CLI-only
        // flag with no NDJSON equivalent. The persistent process needs it at spawn,
        // but it's unavailable during model listing.
        // See: https://code.claude.com/docs/en/cli-reference#system-prompt-flags
        let mut cmd = self.build_stream_json_command();
        let mut child = cmd.spawn().map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to spawn CLI for model listing: {e}"))
        })?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to capture stdout".to_string()))?;

        let request = json!({
            "type": "control_request",
            "request_id": "model_list",
            "request": {"subtype": "initialize"}
        });
        let mut request_str = serde_json::to_string(&request).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to serialize initialize request: {e}"))
        })?;
        request_str.push('\n');
        stdin.write_all(request_str.as_bytes()).await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to write initialize request: {e}"))
        })?;

        let mut reader = BufReader::new(stdout);
        let mut lines = Vec::new();
        let mut line = String::new();

        // Read until we see a control_response or hit EOF
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    lines.push(trimmed.to_string());
                    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                        if parsed.get("type").and_then(|t| t.as_str()) == Some("control_response") {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }

        let _ = child.kill().await;
        Ok(parse_models_from_lines(&lines))
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        if super::cli_common::is_session_description_request(system) {
            let (message, usage) = super::cli_common::generate_simple_session_description(
                &model_config.model_name,
                messages,
            )?;
            return Ok(stream_from_single_message(message, usage));
        }

        let filtered_system = filter_extensions_from_system_prompt(system);
        let process_arc = Arc::clone(self.get_or_init_process(&filtered_system).await?);

        // Prepare the payload outside the lock — these don't need the process.
        let blocks = self.last_user_content_blocks(messages);
        let ndjson_line = build_stream_json_input(&blocks, session_id);
        let model_name = model_config.model_name.clone();
        let message_id = uuid::Uuid::new_v4().to_string();

        Ok(Box::pin(try_stream! {
            // Single lock acquisition covers write-to-stdin and read-from-stdout,
            // eliminating the race window between the two.
            let mut process = process_arc.lock_owned().await;
            process.drain_pending_response().await;
            process.send_set_model(&model_name).await?;

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

            process.needs_drain = true;
            let mut line = String::new();
            let mut accumulated_usage = Usage::default();
            let mut stream_error: Option<ProviderError> = None;
            let stream_timestamp = chrono::Utc::now().timestamp();

            loop {
                line.clear();
                match process.reader.read_line(&mut line).await {
                    Ok(0) => {
                        process.needs_drain = false;
                        stream_error = Some(ProviderError::RequestFailed(
                            "Claude CLI process terminated unexpectedly".to_string(),
                        ));
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                            match parsed.get("type").and_then(|t| t.as_str()) {
                                Some("stream_event") => {
                                    if let Some(event) = parsed.get("event") {
                                        match event.get("type").and_then(|t| t.as_str()) {
                                            Some("content_block_delta") => {
                                                if let Some(text) = event
                                                    .get("delta")
                                                    .filter(|d| {
                                                        d.get("type").and_then(|t| t.as_str())
                                                            == Some("text_delta")
                                                    })
                                                    .and_then(|d| d.get("text"))
                                                    .and_then(|t| t.as_str())
                                                {
                                                    let mut partial_message = Message::new(
                                                        Role::Assistant,
                                                        stream_timestamp,
                                                        vec![MessageContent::text(text)],
                                                    );
                                                    partial_message.id =
                                                        Some(message_id.clone());
                                                    yield (Some(partial_message), None);
                                                }
                                            }
                                            Some("message_start") => {
                                                if let Some(usage_info) = event
                                                    .get("message")
                                                    .and_then(|m| m.get("usage"))
                                                {
                                                    let new = extract_usage_tokens(usage_info);
                                                    if let Some(i) = new.input_tokens {
                                                        accumulated_usage.input_tokens = Some(i);
                                                    }
                                                }
                                            }
                                            Some("message_delta") => {
                                                if let Some(usage_info) = event.get("usage") {
                                                    let new = extract_usage_tokens(usage_info);
                                                    if let Some(o) = new.output_tokens {
                                                        accumulated_usage.output_tokens = Some(o);
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Some("result") => {
                                    process.needs_drain = false;
                                    if let Some(usage_info) = parsed.get("usage") {
                                        let new = extract_usage_tokens(usage_info);
                                        accumulated_usage = Usage::new(
                                            new.input_tokens.or(accumulated_usage.input_tokens),
                                            new.output_tokens.or(accumulated_usage.output_tokens),
                                            None,
                                        );
                                    }
                                    break;
                                }
                                Some("error") => {
                                    process.needs_drain = false;
                                    stream_error = Some(error_from_event("Claude CLI", &parsed));
                                    break;
                                }
                                Some("system") if process.log_model_update => {
                                    if let Some(resolved) = parsed.get("model").and_then(|m| m.as_str()) {
                                        tracing::debug!(
                                            from = %process.current_model,
                                            to = %resolved,
                                            "set_model resolved"
                                        );
                                    }
                                    process.log_model_update = false;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        process.needs_drain = false;
                        stream_error = Some(ProviderError::RequestFailed(format!(
                            "Failed to read streaming output: {e}"
                        )));
                        break;
                    }
                }
            }

            if let Some(err) = stream_error {
                Err(err)?;
            }

            let provider_usage = ProviderUsage::new(model_name, accumulated_usage);
            yield (None, Some(provider_usage));
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::extension::Envs;
    use chrono::Utc;
    use goose_test_support::session::TEST_SESSION_ID;
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;
    use test_case::test_case;

    #[test_case(
        json!({"input_tokens": 100, "output_tokens": 50}),
        Some(100), Some(50)
        ; "both_tokens"
    )]
    #[test_case(json!({"input_tokens": 100}), Some(100), None ; "input_only")]
    #[test_case(json!({}), None, None ; "empty_usage")]
    fn test_extract_usage_tokens(
        usage_json: Value,
        expected_input: Option<i32>,
        expected_output: Option<i32>,
    ) {
        let usage = extract_usage_tokens(&usage_json);
        assert_eq!(usage.input_tokens, expected_input);
        assert_eq!(usage.output_tokens, expected_output);
    }

    #[test_case(
        r#"{"type":"error","error":"context window exceeded"}"#,
        true
        ; "context_exceeded"
    )]
    #[test_case(
        r#"{"type":"error","error":"Model not supported"}"#,
        false
        ; "generic_error_from_event"
    )]
    #[test_case(r#"{"type":"error"}"#, false ; "missing_error_field")]
    fn test_error_from_event(line: &str, is_context_exceeded: bool) {
        let parsed: Value = serde_json::from_str(line).unwrap();
        let err = error_from_event("Claude CLI", &parsed);
        if is_context_exceeded {
            assert!(matches!(err, ProviderError::ContextLengthExceeded(_)));
        } else {
            assert!(matches!(err, ProviderError::RequestFailed(_)));
        }
    }

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
            r#"{"type":"control_response","response":{"subtype":"success","request_id":"model_list","response":{"models":[{"value":"default","displayName":"Default (recommended)","description":"Opus 4.6 · Most capable for complex work"},{"value":"sonnet","displayName":"Sonnet","description":"Sonnet 4.5 · Best for everyday tasks"},{"value":"haiku","displayName":"Haiku","description":"Haiku 4.5 · Fastest for quick answers"}]}}}"#,
        ],
        &["default", "sonnet", "haiku"]
        ; "success"
    )]
    #[test_case(
        &[
            r#"{"type":"control_response","response":{"subtype":"success","request_id":"model_list","response":{"models":[{"value":"default","displayName":"Default","description":"..."},{"value":null,"displayName":"Bad","description":"..."}]}}}"#,
        ],
        &["default"]
        ; "filters_null_values"
    )]
    #[test_case(
        &[r#"{"type":"system","subtype":"init","session_id":"abc"}"#],
        &[]
        ; "no_control_response"
    )]
    #[test_case(
        &[r#"{"type":"control_response","response":{"subtype":"error","request_id":"req_1","error":"fail"}}"#],
        &[]
        ; "error_response"
    )]
    fn test_parse_models_from_lines(lines: &[&str], expected: &[&str]) {
        let lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let result = parse_models_from_lines(&lines);
        let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        assert_eq!(result, expected);
    }

    #[test_case(
        vec![],
        None
        ; "empty_extensions_returns_none"
    )]
    #[test_case(
        vec![ExtensionConfig::Sse {
            name: "legacy".into(),
            description: String::new(),
            uri: Some("http://localhost/sse".into()),
        }],
        None
        ; "sse_only_returns_none"
    )]
    #[test_case(
        vec![ExtensionConfig::Stdio {
            name: "lookup".into(),
            description: String::new(),
            cmd: "node".into(),
            args: vec!["server.js".into()],
            envs: Envs::new([("API_KEY".into(), "secret".into())].into()),
            env_keys: vec![],
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        }],
        Some(json!({ "mcpServers": {
            "lookup": {
                "type": "stdio",
                "command": "node",
                "args": ["server.js"],
                "env": { "API_KEY": "secret" }
            }
        }}))
        ; "stdio_converts_to_mcp_config_json"
    )]
    #[test_case(
        vec![ExtensionConfig::StreamableHttp {
            name: "lookup".into(),
            description: String::new(),
            uri: "http://localhost/mcp".into(),
            envs: Envs::default(),
            env_keys: vec![],
            headers: HashMap::from([("Authorization".into(), "Bearer token".into())]),
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        }],
        Some(json!({ "mcpServers": {
            "lookup": {
                "type": "http",
                "url": "http://localhost/mcp",
                "headers": { "Authorization": "Bearer token" }
            }
        }}))
        ; "streamable_http_converts_to_mcp_config_json"
    )]
    #[test_case(
        vec![ExtensionConfig::StreamableHttp {
            name: "mcp_kiwi_com".into(),
            description: String::new(),
            uri: "https://mcp.kiwi.com".into(),
            envs: Envs::default(),
            env_keys: vec![],
            headers: HashMap::new(),
            timeout: None,
            bundled: None,
            available_tools: vec![],
        }],
        Some(json!({ "mcpServers": {
            "mcp_kiwi_com": {
                "type": "http",
                "url": "https://mcp.kiwi.com"
            }
        }}))
        ; "resolved_name_used_as_key"
    )]
    fn test_claude_mcp_config_json(extensions: Vec<ExtensionConfig>, expected: Option<Value>) {
        let result = claude_mcp_config_json(&extensions)
            .map(|json| serde_json::from_str::<Value>(&json).unwrap());
        assert_eq!(result, expected);
    }

    #[test]
    fn test_write_mcp_config_file() {
        let state_dir = tempdir().unwrap();
        let json = r#"{"mcpServers":{}}"#;

        let tmp = write_mcp_config_file(state_dir.path(), json).unwrap();

        assert_eq!(fs::read_to_string(tmp.path()).unwrap(), json);

        let norm_path = tmp.path().to_string_lossy().replace('\\', "/");
        let expected_prefix = format!("claude-code/mcp-config-{}_", Utc::now().format("%Y%m%d"));
        assert!(norm_path.contains(&expected_prefix));
        assert!(norm_path.ends_with(".json"));
    }

    #[test]
    fn test_write_mcp_config_file_invalid_state_dir() {
        assert!(write_mcp_config_file(Path::new("/dev/null"), "{}").is_err());
    }

    fn make_provider() -> ClaudeCodeProvider {
        ClaudeCodeProvider {
            command: PathBuf::from("claude"),
            model: ModelConfig::new(CLAUDE_CODE_DEFAULT_MODEL)
                .unwrap()
                .with_canonical_limits(CLAUDE_CODE_PROVIDER_NAME),
            name: "claude-code".to_string(),
            mcp_config_file: None,
            cli_process: tokio::sync::OnceCell::new(),
        }
    }

    fn make_test_process(canned_stdout: &str) -> (CliProcess, tokio::io::DuplexStream) {
        let child = tokio::process::Command::new("true")
            .spawn()
            .expect("failed to spawn `true`");
        let (stdin_writer, stdin_reader) = tokio::io::duplex(1024);
        let process = CliProcess {
            child,
            stdin: Box::new(stdin_writer),
            reader: BufReader::new(Box::new(std::io::Cursor::new(
                canned_stdout.as_bytes().to_vec(),
            ))),
            stderr_handle: tokio::spawn(async { String::new() }),
            current_model: String::new(),
            log_model_update: false,
            next_request_id: 0,
            needs_drain: false,
        };
        (process, stdin_reader)
    }

    #[test_case(
        &[r#"{"type":"control_response","response":{"subtype":"success","request_id":"req_0"}}"#],
        Some("default"), "sonnet",
        Ok(()),
        "{\"type\":\"control_request\",\"request_id\":\"req_0\",\"request\":{\"subtype\":\"set_model\",\"model\":\"sonnet\"}}\n"
        ; "default_to_sonnet"
    )]
    #[test_case(
        &[r#"{"type":"control_response","response":{"subtype":"success","request_id":"req_0"}}"#],
        Some("sonnet"), "default",
        Ok(()),
        "{\"type\":\"control_request\",\"request_id\":\"req_0\",\"request\":{\"subtype\":\"set_model\",\"model\":\"default\"}}\n"
        ; "sonnet_to_default"
    )]
    #[test_case(
        &[r#"{"type":"control_response","response":{"subtype":"error","request_id":"req_0","error":"bad model"}}"#],
        None, "bad",
        Err(ProviderError::RequestFailed("set_model failed: bad model".into())),
        "{\"type\":\"control_request\",\"request_id\":\"req_0\",\"request\":{\"subtype\":\"set_model\",\"model\":\"bad\"}}\n"
        ; "failure"
    )]
    #[test_case(
        &[],
        Some("sonnet"), "sonnet",
        Ok(()), ""
        ; "skip_when_same_model"
    )]
    #[test_case(
        &[],
        None, "sonnet",
        Err(ProviderError::RequestFailed("CLI process terminated while waiting for set_model response".into())),
        "{\"type\":\"control_request\",\"request_id\":\"req_0\",\"request\":{\"subtype\":\"set_model\",\"model\":\"sonnet\"}}\n"
        ; "eof"
    )]
    #[tokio::test]
    async fn test_send_set_model(
        lines: &[&str],
        initial_model: Option<&str>,
        target_model: &str,
        expected: Result<(), ProviderError>,
        expected_stdin: &str,
    ) {
        use tokio::io::AsyncReadExt;

        let stdout = lines.join("\n");
        let (mut process, mut stdin_reader) = make_test_process(&stdout);
        if let Some(m) = initial_model {
            process.current_model = m.to_string();
        }

        let result = process.send_set_model(target_model).await;
        process.stdin = Box::new(tokio::io::sink());
        let mut stdin_bytes = Vec::new();
        stdin_reader.read_to_end(&mut stdin_bytes).await.unwrap();

        assert_eq!(result, expected);
        if expected.is_ok() {
            assert_eq!(process.current_model, target_model);
        }
        assert_eq!(String::from_utf8(stdin_bytes).unwrap(), expected_stdin);
    }
}
