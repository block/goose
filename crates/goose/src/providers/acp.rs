//! ACP Provider - connects to a remote ACP-compatible agent as if it were an LLM.
//!
//! This provider spawns an ACP agent as a subprocess and communicates via JSON-RPC
//! over stdio. Tool calls from the ACP agent are passed through to Goose's format.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolRequestParam, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use crate::conversation::message::{Message, MessageContent};
use crate::mcp_utils::ToolResult;
use crate::model::ModelConfig;

pub const ACP_DEFAULT_MODEL: &str = "acp-agent";
pub const ACP_DOC_URL: &str = "https://agentclientprotocol.com";

const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: i64,
    method: String,
    params: Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<i64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

struct AcpConnection {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: AtomicI64,
    session_id: Option<String>,
}

impl AcpConnection {
    fn spawn(command: &str, args: &[String]) -> Result<Self, ProviderError> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to spawn ACP agent '{}': {}",
                    command, e
                ))
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderError::RequestFailed("Failed to get stdout".to_string()))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: AtomicI64::new(1),
            session_id: None,
        })
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, ProviderError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let request_str = serde_json::to_string(&request).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to serialize request: {}", e))
        })?;

        writeln!(self.stdin, "{}", request_str).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to write to ACP agent: {}", e))
        })?;
        self.stdin
            .flush()
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to flush stdin: {}", e)))?;

        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to read from ACP agent: {}", e))
            })?;

            if line.trim().is_empty() {
                continue;
            }

            let response: JsonRpcResponse = serde_json::from_str(&line).map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to parse response: {} - line: {}",
                    e, line
                ))
            })?;

            if response.id.is_none() {
                continue;
            }

            if response.id == Some(id) {
                if let Some(error) = response.error {
                    return Err(ProviderError::RequestFailed(format!(
                        "ACP error {}: {}",
                        error.code, error.message
                    )));
                }
                return Ok(response.result.unwrap_or(Value::Null));
            }
        }
    }

    fn initialize(&mut self) -> Result<(), ProviderError> {
        let params = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "clientCapabilities": {},
            "clientInfo": {
                "name": "goose",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let _result = self.send_request("initialize", params)?;
        Ok(())
    }

    fn new_session(&mut self, cwd: &Path) -> Result<String, ProviderError> {
        let params = json!({
            "cwd": cwd.to_string_lossy(),
            "mcpServers": []
        });

        let result = self.send_request("session/new", params)?;
        let session_id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::RequestFailed("No sessionId in response".to_string()))?
            .to_string();

        self.session_id = Some(session_id.clone());
        Ok(session_id)
    }

    fn prompt(
        &mut self,
        session_id: &str,
        content: Vec<Value>,
    ) -> Result<PromptResult, ProviderError> {
        let params = json!({
            "sessionId": session_id,
            "prompt": content
        });

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: "session/prompt".to_string(),
            params,
        };

        let request_str = serde_json::to_string(&request).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to serialize request: {}", e))
        })?;

        writeln!(self.stdin, "{}", request_str).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to write to ACP agent: {}", e))
        })?;
        self.stdin
            .flush()
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to flush stdin: {}", e)))?;

        let mut result = PromptResult::default();
        let mut tool_calls: HashMap<String, PendingToolCall> = HashMap::new();

        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to read from ACP agent: {}", e))
            })?;

            if line.trim().is_empty() {
                continue;
            }

            let response: JsonRpcResponse = serde_json::from_str(&line).map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to parse response: {} - line: {}",
                    e, line
                ))
            })?;

            if response.id.is_none() {
                if let Some(params) =
                    response
                        .result
                        .as_ref()
                        .or(serde_json::from_str::<Value>(&line)
                            .ok()
                            .and_then(|v| v.get("params").cloned())
                            .as_ref())
                {
                    self.handle_session_update(params, &mut result, &mut tool_calls);
                }
                continue;
            }

            if response.id == Some(id) {
                if let Some(error) = response.error {
                    return Err(ProviderError::RequestFailed(format!(
                        "ACP prompt error {}: {}",
                        error.code, error.message
                    )));
                }

                for (tc_id, tc) in tool_calls {
                    result.tool_calls.push(ResultToolCall {
                        id: tc_id,
                        name: tc.name,
                        arguments: tc.arguments,
                    });
                }

                return Ok(result);
            }
        }
    }

    fn handle_session_update(
        &self,
        params: &Value,
        result: &mut PromptResult,
        tool_calls: &mut HashMap<String, PendingToolCall>,
    ) {
        if let Some(update) = params.get("update") {
            if let Some(chunk) = update.get("agentMessageChunk") {
                if let Some(content) = chunk.get("content") {
                    if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                        result.text.push_str(text);
                    }
                }
            }

            if let Some(chunk) = update.get("agentThoughtChunk") {
                if let Some(content) = chunk.get("content") {
                    if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                        result.thinking.push_str(text);
                    }
                }
            }

            if let Some(tool_call) = update.get("toolCall") {
                if let (Some(tc_id), Some(title)) = (
                    tool_call.get("toolCallId").and_then(|v| v.as_str()),
                    tool_call.get("title").and_then(|v| v.as_str()),
                ) {
                    tool_calls.insert(
                        tc_id.to_string(),
                        PendingToolCall {
                            name: title.to_string(),
                            arguments: tool_call.get("rawInput").cloned(),
                            status: "pending".to_string(),
                        },
                    );
                }
            }

            if let Some(tool_update) = update.get("toolCallUpdate") {
                if let Some(tc_id) = tool_update.get("toolCallId").and_then(|v| v.as_str()) {
                    if let Some(tc) = tool_calls.get_mut(tc_id) {
                        if let Some(status) = tool_update.get("status").and_then(|v| v.as_str()) {
                            tc.status = status.to_string();
                        }
                        if let Some(raw_input) = tool_update.get("rawInput") {
                            tc.arguments = Some(raw_input.clone());
                        }
                    }
                }
            }
        }
    }

    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for AcpConnection {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[derive(Debug, Default)]
struct PromptResult {
    text: String,
    thinking: String,
    tool_calls: Vec<ResultToolCall>,
}

#[derive(Debug)]
struct ResultToolCall {
    id: String,
    name: String,
    arguments: Option<Value>,
}

#[derive(Debug)]
struct PendingToolCall {
    name: String,
    arguments: Option<Value>,
    #[allow(dead_code)]
    status: String,
}

pub struct AcpProvider {
    model: ModelConfig,
    command: String,
    args: Vec<String>,
    connection: Arc<Mutex<Option<AcpConnection>>>,
}

impl AcpProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        let command_str: String = config.get_param("ACP_COMMAND").map_err(|_| {
            anyhow::anyhow!(
                "ACP_COMMAND is required - set to the ACP agent command (e.g., 'npx @zed-industries/claude-code-acp')"
            )
        })?;

        // Parse the command string - first part is the command, rest are args
        let parts: Vec<String> =
            shlex::split(&command_str).unwrap_or_else(|| vec![command_str.clone()]);
        let command = parts.first().cloned().unwrap_or(command_str);
        let args = parts.into_iter().skip(1).collect();

        Ok(Self {
            model,
            command,
            args,
            connection: Arc::new(Mutex::new(None)),
        })
    }

    fn ensure_connection(&self) -> Result<(), ProviderError> {
        let mut conn_guard = self
            .connection
            .lock()
            .map_err(|_| ProviderError::RequestFailed("Lock poisoned".to_string()))?;

        if let Some(ref mut conn) = *conn_guard {
            if conn.is_alive() {
                return Ok(());
            }
        }

        let mut conn = AcpConnection::spawn(&self.command, &self.args)?;
        conn.initialize()?;

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        conn.new_session(&cwd)?;

        *conn_guard = Some(conn);
        Ok(())
    }

    fn convert_messages_to_content(&self, system: &str, messages: &[Message]) -> Vec<Value> {
        let mut content = Vec::new();

        if !system.is_empty() {
            content.push(json!({
                "type": "text",
                "text": format!("[System Prompt]
            {}", system)
            }));
        }

        for msg in messages {
            for msg_content in &msg.content {
                match msg_content {
                    MessageContent::Text(text) => {
                        let prefix = match msg.role {
                            rmcp::model::Role::User => {
                                "[User]
"
                            }
                            rmcp::model::Role::Assistant => {
                                "[Assistant]
"
                            }
                        };
                        content.push(json!({
                            "type": "text",
                            "text": format!("{}{}", prefix, text.text)
                        }));
                    }
                    MessageContent::Image(img) => {
                        content.push(json!({
                            "type": "image",
                            "data": img.data,
                            "mimeType": img.mime_type
                        }));
                    }
                    MessageContent::ToolRequest(req) => {
                        if let Ok(tc) = &req.tool_call {
                            content.push(json!({
                                "type": "text",
                                "text": format!("[Tool Call: {}]
                            Arguments: {}",
                                    tc.name,
                                    serde_json::to_string_pretty(&tc.arguments).unwrap_or_default()
                                )
                            }));
                        }
                    }
                    MessageContent::ToolResponse(resp) => {
                        let result_text = match &resp.tool_result {
                            Ok(result) => result
                                .content
                                .iter()
                                .filter_map(|c| {
                                    if let rmcp::model::RawContent::Text(t) = &c.raw {
                                        Some(t.text.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(
                                    "
",
                                ),
                            Err(e) => format!("Error: {}", e),
                        };
                        content.push(json!({
                            "type": "text",
                            "text": format!("[Tool Result: {}]
                        {}", resp.id, result_text)
                        }));
                    }
                    _ => {}
                }
            }
        }

        content
    }

    #[allow(clippy::useless_conversion)]
    fn build_response(&self, result: PromptResult) -> Message {
        let mut msg = Message::assistant();

        if !result.thinking.is_empty() {
            msg = msg.with_thinking(&result.thinking, "");
        }

        if !result.text.is_empty() {
            msg = msg.with_text(&result.text);
        }

        for tc in result.tool_calls {
            let tool_call_param = CallToolRequestParam {
                name: tc.name.into(),
                arguments: tc.arguments.and_then(|v| v.as_object().cloned()),
            };
            msg = msg.with_tool_request(
                tc.id,
                Ok(tool_call_param) as ToolResult<CallToolRequestParam>,
            );
        }

        msg
    }
}

#[async_trait]
impl Provider for AcpProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "acp",
            "ACP Agent",
            "Connect to a remote ACP-compatible agent (e.g., Claude Code, Cursor Agent)",
            ACP_DEFAULT_MODEL,
            vec![ACP_DEFAULT_MODEL],
            ACP_DOC_URL,
            vec![ConfigKey::new("ACP_COMMAND", true, false, None)],
        )
    }

    fn get_name(&self) -> &str {
        "acp"
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        self.ensure_connection()?;

        let content = self.convert_messages_to_content(system, messages);

        let result = {
            let mut conn_guard = self
                .connection
                .lock()
                .map_err(|_| ProviderError::RequestFailed("Lock poisoned".to_string()))?;

            let conn = conn_guard
                .as_mut()
                .ok_or_else(|| ProviderError::RequestFailed("No connection".to_string()))?;

            let session_id = conn
                .session_id
                .clone()
                .ok_or_else(|| ProviderError::RequestFailed("No session".to_string()))?;

            conn.prompt(&session_id, content)?
        };

        let message = self.build_response(result);
        let usage = Usage::default();

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }
}
