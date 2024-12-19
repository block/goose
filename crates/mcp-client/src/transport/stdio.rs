use super::{Error, Transport};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// A `StdioTransport` uses a child process’s stdin/stdout as a communication channel.
///
/// It starts the specified command with arguments and uses its stdin/stdout to send/receive
/// JSON-RPC messages line by line. This is useful for running MCP servers as subprocesses.
pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    stdout: Mutex<Option<BufReader<ChildStdout>>>,
}

impl StdioTransport {
    /// Create a new `StdioTransport` configured to run the given command with arguments.
    ///
    /// The transport will not start until `start()` is called.
    pub fn new<I, S>(command: S, args: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        Self {
            command: command.into(),
            args: args.into_iter().map(Into::into).collect(),
            child: Mutex::new(None),
            stdin: Mutex::new(None),
            stdout: Mutex::new(None),
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn start(&self) -> Result<(), Error> {
        if self.child.lock().await.is_some() {
            return Ok(()); // Already started
        }

        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn()?;

        let stdin = child.stdin.take().ok_or(Error::NotConnected)?;
        let stdout = child.stdout.take().ok_or(Error::NotConnected)?;

        *self.stdin.lock().await = Some(stdin);
        *self.stdout.lock().await = Some(BufReader::new(stdout));
        *self.child.lock().await = Some(child);

        Ok(())
    }

    async fn send(&self, msg: String) -> Result<(), Error> {
        let mut stdin = self.stdin.lock().await;
        let stdin = stdin.as_mut().ok_or(Error::NotConnected)?;
        // Write the message followed by a newline
        stdin.write_all(msg.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn receive(&self) -> Result<String, Error> {
        let mut stdout = self.stdout.lock().await;
        let stdout = stdout.as_mut().ok_or(Error::NotConnected)?;
        let mut line = String::new();
        let n = stdout.read_line(&mut line).await?;
        if n == 0 {
            // End of stream
            return Err(Error::NotConnected);
        }
        Ok(line)
    }

    async fn close(&self) -> Result<(), Error> {
        let mut child = self.child.lock().await;
        let mut stdin = self.stdin.lock().await;
        let mut stdout = self.stdout.lock().await;

        // Drop stdin to signal EOF
        *stdin = None;
        *stdout = None;

        if let Some(mut c) = child.take() {
            // Wait for child to exit
            let _status = c.wait().await?;
        }

        Ok(())
    }
}
