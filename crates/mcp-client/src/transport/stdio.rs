use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

use async_trait::async_trait;
use mcp_core::protocol::JsonRpcMessage;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

// Import nix crate components instead of libc
#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::{getpgid, Pid};

use super::{serialize_and_send, Error, Transport, TransportHandle};

// Global to track process groups we've created
static PROCESS_GROUP: AtomicI32 = AtomicI32::new(-1);

/// A `StdioTransport` uses a child process's stdin/stdout as a communication channel.
///
/// It uses channels for message passing and handles responses asynchronously through a background task.
pub struct StdioActor {
    receiver: Option<mpsc::Receiver<String>>,
    sender: Option<mpsc::Sender<JsonRpcMessage>>,
    process: Child, // we store the process to keep it alive
    error_sender: mpsc::Sender<Error>,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
}

impl Drop for StdioActor {
    fn drop(&mut self) {
        // Get the process group ID before attempting cleanup
        #[cfg(unix)]
        if let Some(pid) = self.process.id() {
            if let Ok(pgid) = getpgid(Some(Pid::from_raw(pid as i32))) {
                // Send SIGTERM to the entire process group
                let _ = kill(Pid::from_raw(-pgid.as_raw()), Signal::SIGTERM);
                // Give processes a moment to cleanup
                std::thread::sleep(std::time::Duration::from_millis(100));
                // Force kill if still running
                let _ = kill(Pid::from_raw(-pgid.as_raw()), Signal::SIGKILL);
            }
        }
    }
}

impl StdioActor {
    pub async fn run(mut self) {
        use tokio::pin;

        let stdout = self.stdout.take().expect("stdout should be available");
        let stdin = self.stdin.take().expect("stdin should be available");
        let msg_inbox = self.receiver.take().expect("receiver should be available");
        let msg_outbox = self.sender.take().expect("sender should be available");

        let incoming = Self::handle_proc_output(stdout, msg_outbox);
        let outgoing = Self::handle_proc_input(stdin, msg_inbox);

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
            status = self.process.wait() => {
                tracing::debug!("Process exited with status: {:?}", status);
            }
        }

        // Then always try to read stderr before cleaning up
        let mut stderr_buffer = Vec::new();
        if let Some(mut stderr) = self.stderr.take() {
            if let Ok(bytes) = stderr.read_to_end(&mut stderr_buffer).await {
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
        }
    }

    async fn handle_proc_output(stdout: ChildStdout, sender: mpsc::Sender<JsonRpcMessage>) {
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
                        let _ = sender.send(message).await;
                    } else {
                        tracing::warn!(
                            message = ?line,
                            "Failed to parse incoming message"
                        );
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

    async fn handle_proc_input(mut stdin: ChildStdin, mut receiver: mpsc::Receiver<String>) {
        while let Some(message_str) = receiver.recv().await {
            tracing::debug!(message = ?message_str, "Sending outgoing message");

            if let Err(e) = stdin
                .write_all(format!("{}\n", message_str).as_bytes())
                .await
            {
                tracing::error!(error = ?e, "Error writing message to child process");
                break;
            }

            if let Err(e) = stdin.flush().await {
                tracing::error!(error = ?e, "Error flushing message to child process");
                break;
            }
        }
    }
}

#[derive(Clone)]
pub struct StdioTransportHandle {
    sender: mpsc::Sender<String>,                         // to process
    receiver: Arc<Mutex<mpsc::Receiver<JsonRpcMessage>>>, // from process
    error_receiver: Arc<Mutex<mpsc::Receiver<Error>>>,
}

#[async_trait::async_trait]
impl TransportHandle for StdioTransportHandle {
    async fn send(&self, message: JsonRpcMessage) -> Result<(), Error> {
        let result = serialize_and_send(&self.sender, message).await;
        // Check for any pending errors even if send is successful
        self.check_for_errors().await?;
        result
    }

    async fn receive(&self) -> Result<JsonRpcMessage, Error> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await.ok_or(Error::ChannelClosed)
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

        // Set process group and ensure signal handling on Unix systems
        #[cfg(unix)]
        command.process_group(0);

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

        // Store the process group ID for cleanup
        #[cfg(unix)]
        if let Some(pid) = process.id() {
            // Use nix instead of unsafe libc calls
            if let Ok(pgid) = getpgid(Some(Pid::from_raw(pid as i32))) {
                PROCESS_GROUP.store(pgid.as_raw(), Ordering::SeqCst);
            }
        }

        Ok((process, stdin, stdout, stderr))
    }
}

#[async_trait]
impl Transport for StdioTransport {
    type Handle = StdioTransportHandle;

    async fn start(&self) -> Result<Self::Handle, Error> {
        let (process, stdin, stdout, stderr) = self.spawn_process().await?;
        let (outbox_tx, outbox_rx) = mpsc::channel(32);
        let (inbox_tx, inbox_rx) = mpsc::channel(32);
        let (error_tx, error_rx) = mpsc::channel(1);

        let actor = StdioActor {
            receiver: Some(outbox_rx), // client to process
            sender: Some(inbox_tx),    // process to client
            process,
            error_sender: error_tx,
            stdin: Some(stdin),
            stdout: Some(stdout),
            stderr: Some(stderr),
        };

        tokio::spawn(actor.run());

        let handle = StdioTransportHandle {
            sender: outbox_tx,                        // client to process
            receiver: Arc::new(Mutex::new(inbox_rx)), // process to client
            error_receiver: Arc::new(Mutex::new(error_rx)),
        };
        Ok(handle)
    }

    async fn close(&self) -> Result<(), Error> {
        // Attempt to clean up the process group on close
        #[cfg(unix)]
        if let Some(pgid) = PROCESS_GROUP.load(Ordering::SeqCst).checked_abs() {
            // Use nix instead of unsafe libc calls
            // Try SIGTERM first
            let _ = kill(Pid::from_raw(-pgid), Signal::SIGTERM);
            // Give processes a moment to cleanup
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            // Force kill if still running
            let _ = kill(Pid::from_raw(-pgid), Signal::SIGKILL);
        }
        Ok(())
    }
}
