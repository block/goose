use crate::transport::{ReadStream, Transport, WriteStream};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use mcp_core::protocol::*;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

pub struct StdioServerParams {
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
}

pub struct StdioTransport {
    pub params: StdioServerParams,
}

impl StdioTransport {
    fn get_default_environment() -> std::collections::HashMap<String, String> {
        let default_vars = if cfg!(windows) {
            vec!["APPDATA", "PATH", "TEMP", "USERNAME"] // Simplified list
        } else {
            vec!["HOME", "PATH", "SHELL", "USER"] // Simplified list
        };

        std::env::vars()
            .filter(|(key, value)| default_vars.contains(&key.as_str()) && !value.starts_with("()"))
            .collect()
    }

    async fn monitor_child(mut child: Child, tx_read: mpsc::Sender<Result<JsonRpcMessage>>) {
        match child.wait().await {
            Ok(status) => {
                let msg = if status.success() {
                    format!("Child process terminated normally with status: {}", status)
                } else {
                    format!("Child process terminated with error status: {}", status)
                };
                let _ = tx_read.send(Err(anyhow!(msg))).await;
            }
            Err(e) => {
                let _ = tx_read
                    .send(Err(anyhow!("Child process error: {}", e)))
                    .await;
            }
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn connect(&self) -> Result<(ReadStream, WriteStream)> {
        let mut child = Command::new(&self.params.command)
            .args(&self.params.args)
            .env_clear()
            .envs(
                self.params
                    .env
                    .clone()
                    .unwrap_or_else(Self::get_default_environment),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn child process")?;

        let stdin = child.stdin.take().context("Failed to get stdin handle")?;
        let stdout = child.stdout.take().context("Failed to get stdout handle")?;

        let (tx_read, rx_read) = mpsc::channel(100);
        let (tx_write, mut rx_write) = mpsc::channel(100);

        // Clone tx_read for the child monitor
        let tx_read_monitor = tx_read.clone();

        // Spawn child process monitor
        tokio::spawn(Self::monitor_child(child, tx_read_monitor));

        // Spawn stdout reader task
        let stdout_reader = BufReader::new(stdout);
        tokio::spawn(async move {
            let mut lines = stdout_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<JsonRpcMessage>(&line) {
                    Ok(msg) => {
                        if tx_read.send(Ok(msg)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx_read.send(Err(e.into())).await;
                    }
                }
            }
        });

        // Spawn stdin writer task
        let mut stdin = stdin;
        tokio::spawn(async move {
            while let Some(message) = rx_write.recv().await {
                let json = serde_json::to_string(&message).expect("Failed to serialize message");
                if stdin
                    .write_all(format!("{}\n", json).as_bytes())
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok((rx_read, tx_write))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_process_termination() {
        let transport = StdioTransport {
            params: StdioServerParams {
                command: "sleep".to_string(),
                args: vec!["0.3".to_string()],
                env: None,
            },
        };
        let (mut rx, _tx) = transport.connect().await.unwrap();

        // Try to receive a message - should get an error about process termination
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Some(Err(e))) => {
                assert!(
                    e.to_string().contains("Child process terminated normally"),
                    "Expected process termination error, got: {}",
                    e
                );
            }
            _ => panic!("Expected error, got a different message"),
        }
    }
}
