use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{ChildStdin, ChildStdout};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

pub struct JsonRpcProtocol {
    next_id: AtomicU64,
}

impl JsonRpcProtocol {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
        }
    }

    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn send_request(
        &self,
        stdin: &mut ChildStdin,
        method: &str,
        params: Option<Value>,
    ) -> Result<u64> {
        let id = self.next_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        self.write_message(stdin, &request)?;
        Ok(id)
    }

    pub fn send_notification(
        &self,
        stdin: &mut ChildStdin,
        method: &str,
        params: Option<Value>,
    ) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        self.write_message(stdin, &notification)?;
        Ok(())
    }

    fn write_message<T: Serialize>(&self, stdin: &mut ChildStdin, message: &T) -> Result<()> {
        let content = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        stdin.write_all(header.as_bytes())?;
        stdin.write_all(content.as_bytes())?;
        stdin.flush()?;

        debug!("Sent LSP message: {}", content);
        Ok(())
    }

    pub fn read_message(reader: &mut BufReader<ChildStdout>) -> Result<Value> {
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;

            if bytes_read == 0 {
                return Err(anyhow!("LSP process closed connection"));
            }

            let line = line.trim();
            if line.is_empty() {
                break;
            }

            if let Some(length_str) = line.strip_prefix("Content-Length: ") {
                content_length = Some(length_str.parse()?);
            }
        }

        let content_length = content_length.ok_or_else(|| anyhow!("Missing Content-Length"))?;

        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content)?;

        let message: Value = serde_json::from_slice(&content)?;
        debug!("Received LSP message: {}", serde_json::to_string(&message)?);

        Ok(message)
    }

    pub fn parse_response(message: Value) -> Result<JsonRpcResponse> {
        serde_json::from_value(message).map_err(|e| anyhow!("Failed to parse response: {}", e))
    }

    pub fn parse_notification(message: Value) -> Result<JsonRpcNotification> {
        serde_json::from_value(message).map_err(|e| anyhow!("Failed to parse notification: {}", e))
    }
}

impl Default for JsonRpcProtocol {
    fn default() -> Self {
        Self::new()
    }
}
