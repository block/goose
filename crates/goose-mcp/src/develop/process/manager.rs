//! Process manager: spawns, tracks, and manages shell processes.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

use crate::develop::process::buffer::OutputBuffer;
use crate::develop::process::shell::{EnvState, ShellConfig};
use crate::develop::process::types::*;

/// Threshold for promoting a command to the process manager.
const EPHEMERAL_TIMEOUT: Duration = Duration::from_secs(2);

/// Line count threshold for promotion.
const EPHEMERAL_LINE_LIMIT: usize = 100;

/// Lines to show at head of preview.
const PREVIEW_HEAD_LINES: usize = 20;

/// Lines to show at tail of preview.
const PREVIEW_TAIL_LINES: usize = 30;

/// Marker prefix used in wrapped commands (followed by UUID).
const ENV_MARKER_PREFIX: &str = "__GOOSE_ENV_";

/// Strip environment markers from raw output, returning just the command output.
/// The marker format is __GOOSE_ENV_<uuid>__ so we find the first occurrence.
fn strip_env_markers(raw: &str) -> String {
    // The format is: <command output>MARKER<cwd>MARKER<env>MARKER<exit_code>
    // We want just the command output (everything before the first marker)
    if let Some(pos) = raw.find(ENV_MARKER_PREFIX) {
        raw.get(..pos).unwrap_or(raw).trim_end().to_string()
    } else {
        raw.trim_end().to_string()
    }
}

/// A managed process with its output buffer and metadata.
struct ManagedProcess {
    id: ProcessId,
    command: String,
    status: Arc<Mutex<ProcessStatus>>,
    buffer: Arc<Mutex<OutputBuffer>>,
    stdin: Option<Arc<Mutex<std::process::ChildStdin>>>,
    started_at: Instant,
    child: Option<Arc<Mutex<Child>>>,
    /// Whether the buffer contains raw output that needs marker stripping.
    needs_parsing: Arc<Mutex<bool>>,
}

/// The process manager implementation.
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
    next_id: AtomicU32,
    shell_config: ShellConfig,
    env_state: Arc<Mutex<EnvState>>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU32::new(1),
            shell_config: ShellConfig::detect(),
            env_state: Arc::new(Mutex::new(EnvState::new())),
        }
    }

    /// Get warning if using fallback shell.
    pub fn shell_warning(&self) -> Option<String> {
        self.shell_config.fallback_warning()
    }

    /// Get current working directory.
    pub fn cwd(&self) -> std::path::PathBuf {
        self.env_state.lock().unwrap().cwd.clone()
    }

    fn next_process_id(&self) -> ProcessId {
        let n = self.next_id.fetch_add(1, Ordering::SeqCst);
        ProcessId::new(n)
    }

    /// Spawn a command and either return immediately or promote to manager.
    pub fn spawn(&self, command: &str) -> Result<SpawnResult> {
        let env_state = self.env_state.lock().unwrap();

        // Build the wrapped command with env setup and capture
        let env_setup = env_state.setup_commands();
        let (wrapped, delimiter) = EnvState::wrap_command(command);
        let full_command = format!("{}{}", env_setup, wrapped);

        let cwd = env_state.cwd.clone();
        drop(env_state); // Release lock before spawning

        // Spawn the process
        let mut child = Command::new(&self.shell_config.shell_path)
            .arg("-c")
            .arg(&full_command)
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stderr"))?;
        let stdin = child.stdin.take();

        let buffer = Arc::new(Mutex::new(OutputBuffer::new()));
        let status = Arc::new(Mutex::new(ProcessStatus::Running));

        // Spawn reader threads
        let buffer_clone = Arc::clone(&buffer);
        let stdout_handle = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                buffer_clone.lock().unwrap().append(&format!("{}\n", line));
            }
        });

        let buffer_clone = Arc::clone(&buffer);
        let stderr_handle = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                buffer_clone.lock().unwrap().append(&format!("{}\n", line));
            }
        });

        // Wait for ephemeral timeout
        let start = Instant::now();
        loop {
            // Check if process has exited
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    // Process finished - wait for readers to complete
                    let _ = stdout_handle.join();
                    let _ = stderr_handle.join();

                    let exit_code = exit_status.code().unwrap_or(1);
                    *status.lock().unwrap() = ProcessStatus::Exited(exit_code);

                    // Parse the wrapped output to extract actual output and env changes
                    let buf = buffer.lock().unwrap();
                    let raw_output = buf.full_output();
                    drop(buf);

                    let mut env_state = self.env_state.lock().unwrap();
                    let (actual_output, new_cwd, changed, unset, parsed_exit) =
                        env_state.parse_wrapped_output(&raw_output, &delimiter)?;
                    env_state.apply_changes(new_cwd, changed, unset);
                    drop(env_state);

                    // Check line count of ACTUAL output (not raw with env dump)
                    let actual_line_count = actual_output.lines().count();

                    // Check if should promote due to large output
                    if actual_line_count > EPHEMERAL_LINE_LIMIT {
                        let id = self.next_process_id();
                        let mut buf = OutputBuffer::new();
                        buf.append(&actual_output);
                        let (preview, omitted) =
                            buf.preview(PREVIEW_HEAD_LINES, PREVIEW_TAIL_LINES);

                        // Store in manager (already parsed, no markers)
                        let proc = ManagedProcess {
                            id: id.clone(),
                            command: command.to_string(),
                            status: Arc::new(Mutex::new(ProcessStatus::Exited(parsed_exit))),
                            buffer: Arc::new(Mutex::new(buf)),
                            stdin: None,
                            started_at: start,
                            child: None,
                            needs_parsing: Arc::new(Mutex::new(false)),
                        };
                        self.processes.lock().unwrap().insert(id.0.clone(), proc);

                        return Ok(SpawnResult::Promoted {
                            id,
                            output_preview: preview,
                            lines_omitted: omitted,
                        });
                    }

                    return Ok(SpawnResult::Completed {
                        output: actual_output,
                        exit_code: parsed_exit,
                    });
                }
                Ok(None) => {
                    // Still running
                    if start.elapsed() >= EPHEMERAL_TIMEOUT {
                        // Promote to manager
                        let id = self.next_process_id();

                        let buf = buffer.lock().unwrap();
                        let (preview, omitted) =
                            buf.preview(PREVIEW_HEAD_LINES, PREVIEW_TAIL_LINES);
                        drop(buf);

                        let stdin_arc = stdin.map(|s| Arc::new(Mutex::new(s)));
                        let needs_parsing = Arc::new(Mutex::new(true));

                        let proc = ManagedProcess {
                            id: id.clone(),
                            command: command.to_string(),
                            status: Arc::clone(&status),
                            buffer: Arc::clone(&buffer),
                            stdin: stdin_arc,
                            started_at: start,
                            child: Some(Arc::new(Mutex::new(child))),
                            needs_parsing: Arc::clone(&needs_parsing),
                        };
                        self.processes.lock().unwrap().insert(id.0.clone(), proc);

                        // Spawn background thread to monitor completion
                        let status_clone = Arc::clone(&status);
                        let child_arc = self
                            .processes
                            .lock()
                            .unwrap()
                            .get(&id.0)
                            .and_then(|p| p.child.clone());

                        if let Some(child_arc) = child_arc {
                            thread::spawn(move || {
                                // Wait for readers to finish (they'll finish when process exits)
                                let _ = stdout_handle.join();
                                let _ = stderr_handle.join();

                                // Get exit status
                                if let Ok(mut child) = child_arc.lock() {
                                    if let Ok(exit_status) = child.wait() {
                                        let code = exit_status.code().unwrap_or(1);
                                        *status_clone.lock().unwrap() = ProcessStatus::Exited(code);
                                    }
                                }
                            });
                        }

                        return Ok(SpawnResult::Promoted {
                            id,
                            output_preview: preview,
                            lines_omitted: omitted,
                        });
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(anyhow!("Failed to check process status: {}", e));
                }
            }
        }
    }

    /// List all tracked processes.
    pub fn list(&self) -> Vec<ProcessInfo> {
        let procs = self.processes.lock().unwrap();
        procs
            .values()
            .map(|p| ProcessInfo {
                id: p.id.clone(),
                command: p.command.clone(),
                status: p.status.lock().unwrap().clone(),
                started_at: p.started_at,
            })
            .collect()
    }

    /// Get output from a process.
    pub fn output(&self, id: &str, query: OutputQuery) -> Result<String> {
        let procs = self.processes.lock().unwrap();
        let proc = procs
            .get(id)
            .ok_or_else(|| anyhow!("Process not found: {}", id))?;

        let needs_parsing = *proc.needs_parsing.lock().unwrap();
        let buf = proc.buffer.lock().unwrap();

        if needs_parsing {
            // Strip markers from raw output before querying
            let raw = buf.full_output();
            let clean = strip_env_markers(&raw);
            drop(buf);

            // Create temp buffer with clean output for querying
            let mut clean_buf = OutputBuffer::new();
            clean_buf.append(&clean);
            Ok(clean_buf.query(&query))
        } else {
            Ok(buf.query(&query))
        }
    }

    /// Get status of a process.
    pub fn status(&self, id: &str) -> Result<ProcessStatus> {
        let procs = self.processes.lock().unwrap();
        let proc = procs
            .get(id)
            .ok_or_else(|| anyhow!("Process not found: {}", id))?;
        let status = proc.status.lock().unwrap().clone();
        Ok(status)
    }

    /// Wait for a process to complete.
    pub fn await_completion(&self, id: &str, timeout: Duration) -> Result<AwaitResult> {
        let start = Instant::now();

        loop {
            let status = self.status(id)?;

            match status {
                ProcessStatus::Exited(code) => {
                    let output = self.output(id, OutputQuery::default())?;
                    return Ok(AwaitResult::Completed {
                        output,
                        exit_code: code,
                    });
                }
                ProcessStatus::Killed => {
                    let output = self.output(id, OutputQuery::default())?;
                    return Ok(AwaitResult::Completed {
                        output,
                        exit_code: -1,
                    });
                }
                ProcessStatus::Running => {
                    if start.elapsed() >= timeout {
                        let output = self.output(id, OutputQuery::default())?;
                        return Ok(AwaitResult::TimedOut {
                            output_so_far: output,
                        });
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

    /// Kill a process.
    pub fn kill(&self, id: &str) -> Result<KillResult> {
        let procs = self.processes.lock().unwrap();
        let proc = procs
            .get(id)
            .ok_or_else(|| anyhow!("Process not found: {}", id))?;

        let current_status = proc.status.lock().unwrap().clone();
        match current_status {
            ProcessStatus::Exited(code) => Ok(KillResult::AlreadyExited(code)),
            ProcessStatus::Killed => Ok(KillResult::AlreadyKilled),
            ProcessStatus::Running => {
                if let Some(ref child_arc) = proc.child {
                    if let Ok(mut child) = child_arc.lock() {
                        let _ = child.kill();
                    }
                }
                *proc.status.lock().unwrap() = ProcessStatus::Killed;
                Ok(KillResult::Killed)
            }
        }
    }

    /// Send input to a process's stdin.
    pub fn send(&self, id: &str, text: &str) -> Result<()> {
        let procs = self.processes.lock().unwrap();
        let proc = procs
            .get(id)
            .ok_or_else(|| anyhow!("Process not found: {}", id))?;

        let stdin = proc
            .stdin
            .as_ref()
            .ok_or_else(|| anyhow!("Process has no stdin"))?;
        let mut stdin = stdin.lock().unwrap();
        stdin.write_all(text.as_bytes())?;
        stdin.flush()?;
        Ok(())
    }

    /// Kill all managed processes (for cleanup on shutdown).
    pub fn kill_all(&self) {
        let procs = self.processes.lock().unwrap();
        for proc in procs.values() {
            if let Some(ref child_arc) = proc.child {
                if let Ok(mut child) = child_arc.lock() {
                    let _ = child.kill();
                }
            }
        }
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.kill_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_command() {
        let manager = ProcessManager::new();
        let result = manager.spawn("echo hello").unwrap();

        match result {
            SpawnResult::Completed { output, exit_code } => {
                assert!(output.contains("hello"), "Output was: {}", output);
                assert_eq!(exit_code, 0);
            }
            SpawnResult::Promoted {
                id,
                output_preview,
                lines_omitted,
            } => {
                panic!("Expected ephemeral completion, got Promoted {{ id: {}, lines_omitted: {}, preview: {} }}", id, lines_omitted, output_preview);
            }
        }
    }

    #[test]
    fn test_cwd_persistence() {
        let manager = ProcessManager::new();

        // Change directory
        let _ = manager.spawn("cd /tmp").unwrap();

        // Verify cwd changed
        let cwd = manager.cwd();
        assert_eq!(cwd.to_string_lossy(), "/tmp");

        // Next command should run in /tmp
        let result = manager.spawn("pwd").unwrap();
        match result {
            SpawnResult::Completed { output, .. } => {
                assert!(output.contains("/tmp") || output.contains("/private/tmp"));
            }
            _ => panic!("Expected completed"),
        }
    }

    #[test]
    fn test_env_persistence() {
        let manager = ProcessManager::new();

        // Set env var
        let _ = manager.spawn("export TEST_VAR=hello123").unwrap();

        // Read it back
        let result = manager.spawn("echo $TEST_VAR").unwrap();
        match result {
            SpawnResult::Completed { output, .. } => {
                assert!(output.contains("hello123"));
            }
            _ => panic!("Expected completed"),
        }
    }

    #[test]
    fn test_process_list() {
        let manager = ProcessManager::new();

        // Start a long-running process
        let result = manager.spawn("sleep 10").unwrap();

        match result {
            SpawnResult::Promoted { id, .. } => {
                let list = manager.list();
                assert!(!list.is_empty());
                assert!(list.iter().any(|p| p.id == id));

                // Clean up
                manager.kill(&id.0).unwrap();
            }
            SpawnResult::Completed { .. } => {
                // If it completed within 2s, that's fine too
            }
        }
    }

    #[test]
    fn test_long_running_output_no_markers() {
        let manager = ProcessManager::new();

        // Start a process that outputs something then sleeps
        let result = manager.spawn("echo 'test output'; sleep 10").unwrap();

        match result {
            SpawnResult::Promoted { id, .. } => {
                // Wait a bit for output to be captured
                std::thread::sleep(Duration::from_millis(100));

                // Get output - should NOT contain markers
                let output = manager.output(&id.0, OutputQuery::default()).unwrap();
                assert!(
                    !output.contains(ENV_MARKER_PREFIX),
                    "Output should not contain markers: {}",
                    output
                );
                assert!(
                    output.contains("test output"),
                    "Output should contain command output: {}",
                    output
                );

                // Clean up
                manager.kill(&id.0).unwrap();
            }
            SpawnResult::Completed { output, .. } => {
                // If it completed, output should still be clean
                assert!(
                    !output.contains(ENV_MARKER_PREFIX),
                    "Output should not contain markers"
                );
            }
        }
    }

    #[test]
    fn test_strip_env_markers() {
        // Test with UUID-style marker
        let raw = "hello world\n__GOOSE_ENV_abc123def456__\n/tmp\n__GOOSE_ENV_abc123def456__\nPATH=/bin\n__GOOSE_ENV_abc123def456__\n0";
        let clean = strip_env_markers(raw);
        assert_eq!(clean, "hello world");
        assert!(!clean.contains("__GOOSE_ENV_"));
        assert!(!clean.contains("/tmp"));
        assert!(!clean.contains("PATH="));
    }
}
