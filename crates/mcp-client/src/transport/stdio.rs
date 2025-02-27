use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use mcp_core::protocol::JsonRpcMessage;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

use super::{send_message, Error, PendingRequests, Transport, TransportHandle, TransportMessage};

/// A `StdioTransport` uses a child process's stdin/stdout as a communication channel.
///
/// It uses channels for message passing and handles responses asynchronously through a background task.
pub struct StdioActor {
    receiver: mpsc::Receiver<TransportMessage>,
    pending_requests: Arc<PendingRequests>,
    _process: Child, // we store the process to keep it alive
    error_sender: mpsc::Sender<Error>,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl StdioActor {
    pub async fn run(mut self) {
        use tokio::pin;

        let incoming = Self::handle_incoming_messages(self.stdout, self.pending_requests.clone());
        let outgoing = Self::handle_outgoing_messages(
            self.receiver,
            self.stdin,
            self.pending_requests.clone(),
        );

        // take ownership of futures for tokio::select
        pin!(incoming);
        pin!(outgoing);

        // Use select! to wait for either I/O completion or process exit
        tokio::select! {
            result = &mut incoming => {
                tracing::debug!("Stdin handler completed: {:?}", result);
            }
            result = &mut outgoing => {
                tracing::debug!("Stdout handler completed: {:?}", result);
            }
            // capture the status so we don't need to wait for a timeout
            status = self._process.wait() => {
                tracing::debug!("Process exited with status: {:?}", status);
            }
        }

        // Then always try to read stderr before cleaning up
        let mut stderr_buffer = Vec::new();
        if let Ok(bytes) = self.stderr.read_to_end(&mut stderr_buffer).await {
            let err_msg = if bytes > 0 {
                String::from_utf8_lossy(&stderr_buffer).to_string()
            } else {
                "Process ended unexpectedly".to_string()
            };

            tracing::info!("Process stderr: {}", err_msg);
            let _ = self
                .error_sender
                .send(Error::StdioProcessError(err_msg))
                .await;
        }

        // Clean up regardless of which path we took
        self.pending_requests.clear().await;
    }

    async fn handle_incoming_messages(stdout: ChildStdout, pending_requests: Arc<PendingRequests>) {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    tracing::error!("Child process ended (EOF on stdout)");
                    break;
                } // EOF
                Ok(_) => {
                    if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&line) {
                        tracing::debug!(
                            message = ?message,
                            "Received incoming message"
                        );

                        match &message {
                            JsonRpcMessage::Response(response) => {
                                if let Some(id) = &response.id {
                                    pending_requests.respond(&id.to_string(), Ok(message)).await;
                                }
                            }
                            JsonRpcMessage::Error(error) => {
                                if let Some(id) = &error.id {
                                    pending_requests.respond(&id.to_string(), Ok(message)).await;
                                }
                            }
                            _ => {} // TODO: Handle other variants (Request, etc.)
                        }
                    }
                    line.clear();
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Error reading line");
                    break;
                }
            }
        }
    }

    async fn handle_outgoing_messages(
        mut receiver: mpsc::Receiver<TransportMessage>,
        mut stdin: ChildStdin,
        pending_requests: Arc<PendingRequests>,
    ) {
        while let Some(mut transport_msg) = receiver.recv().await {
            let message_str = match serde_json::to_string(&transport_msg.message) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(tx) = transport_msg.response_tx.take() {
                        let _ = tx.send(Err(Error::Serialization(e)));
                    }
                    continue;
                }
            };

            tracing::debug!(message = ?transport_msg.message, "Sending outgoing message");

            if let Some(response_tx) = transport_msg.response_tx.take() {
                if let JsonRpcMessage::Request(request) = &transport_msg.message {
                    if let Some(id) = &request.id {
                        pending_requests.insert(id.to_string(), response_tx).await;
                    }
                }
            }

            if let Err(e) = stdin
                .write_all(format!("{}\n", message_str).as_bytes())
                .await
            {
                tracing::error!(error = ?e, "Error writing message to child process");
                pending_requests.clear().await;
                break;
            }

            if let Err(e) = stdin.flush().await {
                tracing::error!(error = ?e, "Error flushing message to child process");
                pending_requests.clear().await;
                break;
            }
        }
    }
}

#[derive(Clone)]
pub struct StdioTransportHandle {
    sender: mpsc::Sender<TransportMessage>,
    error_receiver: Arc<Mutex<mpsc::Receiver<Error>>>,
}

#[async_trait::async_trait]
impl TransportHandle for StdioTransportHandle {
    async fn send(&self, message: JsonRpcMessage) -> Result<JsonRpcMessage, Error> {
        let result = send_message(&self.sender, message).await;
        // Check for any pending errors even if send is successful
        self.check_for_errors().await?;
        result
    }
}

impl StdioTransportHandle {
    /// Check if there are any process errors
    pub async fn check_for_errors(&self) -> Result<(), Error> {
        match self.error_receiver.lock().await.try_recv() {
            Ok(error) => {
                tracing::debug!("Found error: {:?}", error);
                Err(error)
            }
            Err(_) => Ok(()),
        }
    }
}

pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

impl StdioTransport {
    pub fn new<S: Into<String>>(
        command: S,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> Self {
        Self {
            command: command.into(),
            args,
            env,
        }
    }

    async fn spawn_process(&self) -> Result<(Child, ChildStdin, ChildStdout, ChildStderr), Error> {
        let mut command = Command::new(&self.command);
        command
            .envs(&self.env)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        // Set process group only on Unix systems
        #[cfg(unix)]
        command.process_group(0); // don't inherit signal handling from parent process

        // Hide console window on Windows
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW flag

        let mut process = command
            .spawn()
            .map_err(|e| Error::StdioProcessError(e.to_string()))?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| Error::StdioProcessError("Failed to get stdin".into()))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| Error::StdioProcessError("Failed to get stdout".into()))?;

        let stderr = process
            .stderr
            .take()
            .ok_or_else(|| Error::StdioProcessError("Failed to get stderr".into()))?;

        Ok((process, stdin, stdout, stderr))
    }
}

#[async_trait]
impl Transport for StdioTransport {
    type Handle = StdioTransportHandle;

    async fn start(&self) -> Result<Self::Handle, Error> {
        let (process, stdin, stdout, stderr) = self.spawn_process().await?;
        let (message_tx, message_rx) = mpsc::channel(32);
        let (error_tx, error_rx) = mpsc::channel(1);

        let actor = StdioActor {
            receiver: message_rx,
            pending_requests: Arc::new(PendingRequests::new()),
            _process: process,
            error_sender: error_tx,
            stdin,
            stdout,
            stderr,
        };

        tokio::spawn(actor.run());

        let handle = StdioTransportHandle {
            sender: message_tx,
            error_receiver: Arc::new(Mutex::new(error_rx)),
        };
        Ok(handle)
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}
