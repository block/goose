use std::sync::Arc;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use mcp_core::protocol::JsonRpcMessage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

use super::{Error, Transport, TransportMessage};

/// A `StdioTransport` uses a child process's stdin/stdout as a communication channel.
///
/// It uses channels for message passing and handles responses asynchronously through a background task.
pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    process: Arc<Mutex<Option<Child>>>,
    reader_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pending_requests: Arc<
        Mutex<std::collections::HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>>,
    >,
}

impl StdioTransport {
    /// Create a new `StdioTransport` configured to run the given command with arguments.
    pub fn new<S: Into<String>>(command: S, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            process: Arc::new(Mutex::new(None)),
            reader_handle: Arc::new(Mutex::new(None)),
            pending_requests: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    async fn spawn_process(&self) -> Result<(ChildStdin, ChildStdout), Error> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or(Error::Other("Failed to get stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(Error::Other("Failed to get stdout".into()))?;

        *self.process.lock().await = Some(child);

        Ok((stdin, stdout))
    }

    async fn handle_message(
        message: JsonRpcMessage,
        pending_requests: Arc<
            Mutex<
                std::collections::HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>,
            >,
        >,
    ) {
        if let JsonRpcMessage::Response(response) = &message {
            if let Some(id) = &response.id {
                if let Some(tx) = pending_requests.lock().await.remove(&id.to_string()) {
                    let _ = tx.send(Ok(message));
                }
            }
        }
    }

    async fn process_messages(
        mut message_rx: mpsc::Receiver<TransportMessage>,
        mut stdin: ChildStdin,
        stdout: ChildStdout,
        pending_requests: Arc<
            Mutex<
                std::collections::HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>,
            >,
        >,
    ) {
        // Set up async reader for stdout
        let mut reader = BufReader::new(stdout);

        // Spawn stdout reader task
        let pending_clone = pending_requests.clone();
        let reader_handle = tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&line) {
                            Self::handle_message(message, pending_clone.clone()).await;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading line: {}", e);
                        break;
                    }
                }
            }
        });

        // Process incoming messages
        while let Some(transport_msg) = message_rx.recv().await {
            let message_str = match serde_json::to_string(&transport_msg.message) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(tx) = transport_msg.response_tx {
                        let _ = tx.send(Err(Error::Serialization(e)));
                    }
                    continue;
                }
            };

            // Store response channel if this is a request
            if let Some(response_tx) = transport_msg.response_tx {
                if let JsonRpcMessage::Request(request) = &transport_msg.message {
                    if let Some(id) = &request.id {
                        pending_requests
                            .lock()
                            .await
                            .insert(id.to_string(), response_tx);
                    }
                }
            }

            // Write message to stdin
            if let Err(_) = stdin
                .write_all(format!("{}\n", message_str).as_bytes())
                .await
            {
                // Break with a specific error indicating write failure
                let mut pending = pending_requests.lock().await;
                for (_, tx) in pending.drain() {
                    let _ = tx.send(Err(Error::SendFailed));
                }
                break;
            }
            if let Err(_) = stdin.flush().await {
                // Break with a specific error indicating connection issues
                let mut pending = pending_requests.lock().await;
                for (_, tx) in pending.drain() {
                    let _ = tx.send(Err(Error::ConnectionClosed));
                }
                break;
            }
        }

        // Clean up
        reader_handle.abort();
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn start(&self) -> Result<mpsc::Sender<TransportMessage>, Error> {
        let (stdin, stdout) = self.spawn_process().await?;

        let (message_tx, message_rx) = mpsc::channel(32);

        let pending_requests = self.pending_requests.clone();
        let handle = tokio::spawn(Self::process_messages(
            message_rx,
            stdin,
            stdout,
            pending_requests,
        ));

        *self.reader_handle.lock().await = Some(handle);

        Ok(message_tx)
    }

    async fn close(&self) -> Result<(), Error> {
        // Kill the process
        if let Some(mut process) = self.process.lock().await.take() {
            let _ = process.kill().await;
        }

        // Abort the reader task
        if let Some(handle) = self.reader_handle.lock().await.take() {
            handle.abort();
            let _ = handle.await;
        }

        // Clear any pending requests
        self.pending_requests.lock().await.clear();

        Ok(())
    }
}

// No Drop implementation needed - we'll handle cleanup in the TransportService
