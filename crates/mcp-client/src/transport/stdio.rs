use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use mcp_core::protocol::{JsonRpcMessage, JsonRpcNotification};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

use super::{send_message, Error, PendingRequests, Transport, TransportHandle, TransportMessage};
use crate::transport::logging::{LoggingManager, handle_notification};

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
    logging_manager: Arc<LoggingManager>,
}

impl StdioActor {
    pub async fn run(mut self) {
        use tokio::pin;

        let incoming = Self::handle_incoming_messages(
            self.stdout, 
            self.pending_requests.clone(),
            self.logging_manager.clone(),
        );
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

    async fn handle_incoming_messages(
        stdout: ChildStdout, 
        pending_requests: Arc<PendingRequests>,
        logging_manager: Arc<LoggingManager>,
    ) {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    tracing::error!("Child process ended (EOF on stdout)");
                    break;
                } // EOF
                Ok(_) => {
                    eprintln!("Got line from stdout: {}", line);
                    if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&line) {
                        tracing::debug!(
                            message = ?message,
                            "Received incoming message"
                        );

                    match &message {
                            JsonRpcMessage::Response(response) => {
                                tracing::debug!("Got response message with id: {:?}", response.id);
                                if let Some(id) = &response.id {
                                    pending_requests.respond(&id.to_string(), Ok(message)).await;
                                }
                            }
                            JsonRpcMessage::Notification(n) => {
                                tracing::debug!("Got notification message with method: {}", n.method);
                                let notification: JsonRpcNotification = n.clone();
                                if n.method == "notifications/message" {
                                    println!("Processing log notification with params: {:?}", n.params);
                                }
                                if let Err(e) = handle_notification(notification, &logging_manager).await {
                                    tracing::error!("Error handling notification: {:?}", e);
                                }
                            }
                            _ => {
                                tracing::debug!("Got other message type: {:?}", message);
                            }
                        }
                    } else {
                        tracing::debug!("Failed to parse line as JsonRpcMessage: {}", line);
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
    logging_manager: Arc<LoggingManager>,
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

    /// Register a handler for log messages
    pub async fn on_log<F>(&self, handler: F)
    where
        F: Fn(super::logging::LogMessage) + Send + Sync + 'static,
    {
        self.logging_manager.add_handler(handler).await;
    }

    /// Enable logging to a file
    pub async fn enable_file_logging(&self, log_path: impl Into<PathBuf>) -> Result<(), Error> {
        let file_logger = super::logging::file_logger::FileLogger::new(log_path.into())
            .map_err(|e| Error::Io(e))?;
        
        let file_logger = Arc::new(file_logger);
        let file_logger_clone = file_logger.clone();

        self.on_log(move |msg| {
            let file_logger = file_logger_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = file_logger.log(&msg).await {
                    tracing::error!("Failed to write to log file: {}", e);
                }
            });
        })
        .await;

        Ok(())
    }

    /// Set the desired log level
    pub async fn set_log_level(&self, level: super::logging::LogLevel) -> Result<(), Error> {
        let request = super::logging::create_set_level_request(level);
        self.send(request).await?;
        Ok(())
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
        let logging_manager = Arc::new(LoggingManager::new());

        let actor = StdioActor {
            receiver: message_rx,
            pending_requests: Arc::new(PendingRequests::new()),
            _process: process,
            error_sender: error_tx,
            stdin,
            stdout,
            stderr,
            logging_manager: logging_manager.clone(),
        };

        tokio::spawn(actor.run());

        let handle = StdioTransportHandle {
            sender: message_tx,
            error_receiver: Arc::new(Mutex::new(error_rx)),
            logging_manager,
        };
        Ok(handle)
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_logging_flow() {
        let transport = StdioTransport::new("echo", vec![], HashMap::new());
        let handle = transport.start().await.unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Register log handler
        handle
            .on_log(move |msg| {
                assert_eq!(msg.level, crate::transport::logging::LogLevel::Info);
                assert_eq!(msg.message, "test message");
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        // Set log level
        handle.set_log_level(crate::transport::logging::LogLevel::Info).await.unwrap();

        // Create a test notification
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/message".to_string(),
            params: Some(serde_json::json!({
                "level": "info",
                "data": "test message"
            })),
        };

        // Send the notification through the transport
        handle
            .send(JsonRpcMessage::Notification(notification))
            .await
            .unwrap();

        // Give some time for the handler to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the handler was called
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}