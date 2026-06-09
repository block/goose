//! Client-side terminal support for goose acting as an ACP client.
//!
//! When goose bridges to an external ACP agent (e.g. Claude Code), the agent
//! runs shell commands by asking goose — the client — to execute them through
//! the `terminal/*` methods. goose owns the process, captures its output, and
//! reports exit status back. Embedding a terminal in a tool call lets the agent
//! release it while the captured output stays available, so this manager
//! retains output after release to resolve those embeds into text.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::{
    ContentBlock, CreateTerminalRequest, TerminalExitStatus, TerminalId, TerminalOutputResponse,
    TextContent, ToolCallContent,
};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::{watch, Notify};

use crate::subprocess::configure_subprocess;

/// Output cap applied when the agent does not request one, bounding memory for
/// commands that emit unbounded output.
const DEFAULT_OUTPUT_BYTE_LIMIT: usize = 1 << 20;

/// Number of released terminals whose output we keep for tool-call embeds.
const RETAINED_CAP: usize = 256;

const READ_CHUNK: usize = 8192;

#[derive(Clone)]
pub(crate) struct TerminalManager {
    inner: Arc<Inner>,
}

struct Inner {
    sessions: Mutex<HashMap<String, Arc<TerminalSession>>>,
    retained: Mutex<RetainedOutputs>,
    next_id: AtomicU64,
    work_dir: PathBuf,
}

struct TerminalSession {
    buffer: Arc<Mutex<TerminalBuffer>>,
    exit_rx: watch::Receiver<Option<TerminalExitStatus>>,
    kill: Arc<Notify>,
}

struct TerminalBuffer {
    data: Vec<u8>,
    truncated: bool,
    byte_limit: usize,
}

impl TerminalBuffer {
    fn append(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
        if self.data.len() > self.byte_limit {
            let overflow = self.data.len() - self.byte_limit;
            self.data.drain(0..overflow);
            while std::str::from_utf8(&self.data).is_err_and(|e| e.valid_up_to() == 0) {
                self.data.drain(..1);
            }
            self.truncated = true;
        }
    }

    fn output(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

#[derive(Default)]
struct RetainedOutputs {
    map: HashMap<String, String>,
    order: VecDeque<String>,
}

impl RetainedOutputs {
    fn insert(&mut self, id: String, output: String) {
        if self.map.insert(id.clone(), output).is_none() {
            self.order.push_back(id);
            while self.order.len() > RETAINED_CAP {
                if let Some(old) = self.order.pop_front() {
                    self.map.remove(&old);
                }
            }
        }
    }
}

impl TerminalManager {
    pub(crate) fn new(work_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Inner {
                sessions: Mutex::new(HashMap::new()),
                retained: Mutex::new(RetainedOutputs::default()),
                next_id: AtomicU64::new(0),
                work_dir,
            }),
        }
    }

    pub(crate) async fn create(
        &self,
        request: CreateTerminalRequest,
    ) -> std::io::Result<TerminalId> {
        let cwd = request
            .cwd
            .clone()
            .unwrap_or_else(|| self.inner.work_dir.clone());
        let byte_limit = request
            .output_byte_limit
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_OUTPUT_BYTE_LIMIT)
            .max(1);

        let mut cmd = Command::new(&request.command);
        cmd.args(&request.args)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for var in &request.env {
            cmd.env(&var.name, &var.value);
        }
        configure_subprocess(&mut cmd);

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let buffer = Arc::new(Mutex::new(TerminalBuffer {
            data: Vec::new(),
            truncated: false,
            byte_limit,
        }));
        let (exit_tx, exit_rx) = watch::channel(None);
        let kill = Arc::new(Notify::new());

        {
            let buffer = buffer.clone();
            let kill = kill.clone();
            tokio::spawn(async move {
                let pumps = [
                    stdout.map(|s| tokio::spawn(pump(s, buffer.clone()))),
                    stderr.map(|s| tokio::spawn(pump(s, buffer.clone()))),
                ];
                let status = tokio::select! {
                    s = child.wait() => s,
                    _ = kill.notified() => {
                        let _ = child.start_kill();
                        child.wait().await
                    }
                };
                for pump in pumps.into_iter().flatten() {
                    let _ = pump.await;
                }
                let _ = exit_tx.send(Some(to_exit_status(status)));
            });
        }

        let id = format!(
            "term-{}",
            self.inner.next_id.fetch_add(1, Ordering::Relaxed)
        );
        if let Ok(mut sessions) = self.inner.sessions.lock() {
            sessions.insert(
                id.clone(),
                Arc::new(TerminalSession {
                    buffer,
                    exit_rx,
                    kill,
                }),
            );
        }
        Ok(TerminalId::new(id))
    }

    pub(crate) fn output(&self, id: &str) -> Option<TerminalOutputResponse> {
        let session = self.inner.sessions.lock().ok()?.get(id).cloned()?;
        let (output, truncated) = {
            let buffer = session.buffer.lock().expect("terminal buffer poisoned");
            (buffer.output(), buffer.truncated)
        };
        let exit_status = session.exit_rx.borrow().clone();
        Some(TerminalOutputResponse::new(output, truncated).exit_status(exit_status))
    }

    pub(crate) async fn wait_for_exit(&self, id: &str) -> Option<TerminalExitStatus> {
        let mut exit_rx = self.inner.sessions.lock().ok()?.get(id)?.exit_rx.clone();
        loop {
            if let Some(status) = exit_rx.borrow().clone() {
                return Some(status);
            }
            if exit_rx.changed().await.is_err() {
                return exit_rx.borrow().clone();
            }
        }
    }

    pub(crate) fn kill(&self, id: &str) {
        if let Ok(sessions) = self.inner.sessions.lock() {
            if let Some(session) = sessions.get(id) {
                session.kill.notify_one();
            }
        }
    }

    pub(crate) fn release(&self, id: &str) {
        let session = self
            .inner
            .sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(id));
        if let Some(session) = session {
            session.kill.notify_one();
            let output = session
                .buffer
                .lock()
                .map(|buffer| buffer.output())
                .unwrap_or_default();
            if let Ok(mut retained) = self.inner.retained.lock() {
                retained.insert(id.to_string(), output);
            }
        }
    }

    /// Replace any embedded `terminal/*` references with the terminal's captured
    /// output so downstream consumers see real text instead of an opaque id.
    pub(crate) fn resolve_content(&self, content: Vec<ToolCallContent>) -> Vec<ToolCallContent> {
        content
            .into_iter()
            .map(|block| match block {
                ToolCallContent::Terminal(terminal) => {
                    let output = self
                        .resolve_output(terminal.terminal_id.0.as_ref())
                        .unwrap_or_default();
                    ToolCallContent::from(ContentBlock::Text(TextContent::new(output)))
                }
                other => other,
            })
            .collect()
    }

    fn resolve_output(&self, id: &str) -> Option<String> {
        if let Ok(sessions) = self.inner.sessions.lock() {
            if let Some(session) = sessions.get(id) {
                return Some(
                    session
                        .buffer
                        .lock()
                        .map(|buffer| buffer.output())
                        .unwrap_or_default(),
                );
            }
        }
        self.inner
            .retained
            .lock()
            .ok()
            .and_then(|retained| retained.map.get(id).cloned())
    }
}

async fn pump<R: AsyncRead + Unpin>(mut reader: R, buffer: Arc<Mutex<TerminalBuffer>>) {
    let mut chunk = [0u8; READ_CHUNK];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if let Ok(mut buffer) = buffer.lock() {
                    buffer.append(&chunk[..n]);
                }
            }
        }
    }
}

fn to_exit_status(status: std::io::Result<std::process::ExitStatus>) -> TerminalExitStatus {
    let Ok(status) = status else {
        return TerminalExitStatus::new();
    };
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&status).map(|s| s.to_string());
    #[cfg(not(unix))]
    let signal: Option<String> = None;
    TerminalExitStatus::new()
        .exit_code(status.code().map(|c| c as u32))
        .signal(signal)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use agent_client_protocol::schema::{SessionId, Terminal};

    fn request(command: &str, args: &[&str]) -> CreateTerminalRequest {
        CreateTerminalRequest::new(SessionId::new("s"), command)
            .args(args.iter().map(|a| a.to_string()).collect())
    }

    #[tokio::test]
    async fn captures_output_and_exit_status() {
        let manager = TerminalManager::new(PathBuf::from("."));
        let id = manager
            .create(request("sh", &["-c", "printf hello"]))
            .await
            .unwrap();
        let id = id.0.to_string();

        let status = manager.wait_for_exit(&id).await.unwrap();
        assert_eq!(status.exit_code, Some(0));

        let output = manager.output(&id).unwrap();
        assert_eq!(output.output, "hello");
        assert!(!output.truncated);
        assert_eq!(output.exit_status.unwrap().exit_code, Some(0));
    }

    #[tokio::test]
    async fn resolves_embedded_terminal_to_text() {
        let manager = TerminalManager::new(PathBuf::from("."));
        let id = manager
            .create(request("sh", &["-c", "printf done"]))
            .await
            .unwrap();
        manager.wait_for_exit(id.0.as_ref()).await;

        let resolved = manager.resolve_content(vec![ToolCallContent::Terminal(Terminal::new(id))]);

        match &resolved[..] {
            [ToolCallContent::Content(content)] => match &content.content {
                ContentBlock::Text(text) => assert_eq!(text.text, "done"),
                other => panic!("expected text content, got {other:?}"),
            },
            other => panic!("expected one resolved content block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn retains_output_after_release() {
        let manager = TerminalManager::new(PathBuf::from("."));
        let id = manager
            .create(request("sh", &["-c", "printf kept"]))
            .await
            .unwrap();
        let id = id.0.to_string();
        manager.wait_for_exit(&id).await;

        manager.release(&id);
        assert!(manager.output(&id).is_none());
        assert_eq!(manager.resolve_output(&id).as_deref(), Some("kept"));
    }

    #[tokio::test]
    async fn nonzero_exit_code_is_reported() {
        let manager = TerminalManager::new(PathBuf::from("."));
        let id = manager
            .create(request("sh", &["-c", "exit 3"]))
            .await
            .unwrap();
        let status = manager.wait_for_exit(id.0.as_ref()).await.unwrap();
        assert_eq!(status.exit_code, Some(3));
    }

    #[test]
    fn buffer_truncates_from_front() {
        let mut buffer = TerminalBuffer {
            data: Vec::new(),
            truncated: false,
            byte_limit: 4,
        };
        buffer.append(b"abcdef");
        assert_eq!(buffer.output(), "cdef");
        assert!(buffer.truncated);
    }

    #[test]
    fn buffer_truncates_to_utf8_boundary() {
        let mut buffer = TerminalBuffer {
            data: Vec::new(),
            truncated: false,
            byte_limit: 4,
        };
        buffer.append("ééé".as_bytes());
        assert_eq!(buffer.output(), "éé");
        assert!(buffer.truncated);
    }
}
