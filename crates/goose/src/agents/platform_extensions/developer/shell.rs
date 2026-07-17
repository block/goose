use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(windows))]
use std::sync::Arc;
#[cfg(not(windows))]
use std::sync::Mutex;
use std::time::Duration;

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(not(windows))]
use tokio::sync::OnceCell;
#[cfg(not(windows))]
use tokio::task::JoinHandle;
use tokio_stream::{wrappers::SplitStream, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::subprocess::configure_subprocess;

pub(crate) type EnvOverlay = HashMap<String, Option<String>>;

#[cfg(not(windows))]
const ENV_CAPTURE_PATH_VAR: &str = "__GOOSE_ENV_AFTER";

/// Check if the current process is running inside a Flatpak sandbox.
///
/// When inside Flatpak, shell commands must be wrapped with `flatpak-spawn --host`
/// to execute on the host system rather than inside the sandbox.
#[cfg(not(windows))]
pub(crate) fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

#[cfg(not(windows))]
const FLATPAK_HOST_ARGS: [&str; 2] = ["--host", "--watch-bus"];

#[cfg(not(windows))]
pub(crate) fn flatpak_spawn_command() -> tokio::process::Command {
    let mut command = tokio::process::Command::new("flatpak-spawn");
    command.args(FLATPAK_HOST_ARGS);
    command
}

#[cfg(not(windows))]
fn flatpak_spawn_process() -> std::process::Command {
    let mut command = std::process::Command::new("flatpak-spawn");
    command.args(FLATPAK_HOST_ARGS);
    command
}

#[cfg(not(windows))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnixShellFlavor {
    Posix,
    Nushell,
    Other,
}

#[cfg(not(windows))]
fn unix_shell_flavor(shell: &str) -> UnixShellFlavor {
    let name = std::path::Path::new(shell)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(shell)
        .to_ascii_lowercase();

    match name.as_str() {
        "bash" | "dash" | "ksh" | "mksh" | "sh" | "zsh" => UnixShellFlavor::Posix,
        "nu" | "nushell" => UnixShellFlavor::Nushell,
        _ => UnixShellFlavor::Other,
    }
}

#[cfg(not(windows))]
fn unix_login_shell_command_args(shell: &str) -> [&'static str; 4] {
    let probe = match unix_shell_flavor(shell) {
        UnixShellFlavor::Nushell => "print ($env.PATH | str join (char esep))",
        UnixShellFlavor::Posix | UnixShellFlavor::Other => "echo $PATH",
    };

    ["-l", "-i", "-c", probe]
}

#[cfg(not(windows))]
fn unix_shell_command_args(command_line: &str) -> [&str; 2] {
    ["-c", command_line]
}

/// Resolve the preferred Unix shell for command execution, respecting GOOSE_SHELL.
///
/// Auto-detected shells are returned as basenames (e.g. `"bash"`) so that
/// `Command::new` resolves them on `PATH` at spawn time — this also keeps
/// Flatpak happy, where absolute paths from inside the sandbox don't match
/// the host filesystem. `GOOSE_SHELL` is passed through as-is.
///
#[cfg(windows)]
fn windows_shell() -> String {
    std::env::var("GOOSE_SHELL").unwrap_or_else(|_| "cmd".to_string())
}

/// Short, human-readable name of a shell path (the file stem), used both to
/// pick the right argument style on Windows and to tell the LLM which
/// dialect to write in the tool description.
#[cfg(windows)]
fn shell_basename(shell: &str) -> String {
    Path::new(shell)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cmd")
        .to_lowercase()
}

#[cfg(not(windows))]
fn shell_basename(shell: &str) -> String {
    std::path::Path::new(shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(shell)
        .to_string()
}

/// Basename of the shell the `shell` tool will invoke, for use in the tool
/// description so the LLM knows which dialect to write.
#[cfg(windows)]
pub fn shell_display_name() -> String {
    shell_basename(&windows_shell())
}

#[cfg(not(windows))]
pub fn shell_display_name() -> String {
    shell_basename(&unix_shell())
}

/// The shell tool runs commands with `-c "..."`, and LLMs routinely emit
/// POSIX-style patterns such as heredocs (`cat <<EOF > file`), `$VAR`
/// expansion, and `2>&1` redirection. Non-POSIX shells (fish, csh, tcsh,
/// nu, ...) reject or mis-interpret these, so we don't auto-select based
/// on `$SHELL`: we check whether `bash` is on PATH and otherwise fall back
/// to `sh`. Users who really want their login shell can opt in via
/// `GOOSE_SHELL`.
#[cfg(not(windows))]
fn unix_shell() -> String {
    if let Ok(shell) = std::env::var("GOOSE_SHELL") {
        return shell;
    }
    if which::which("bash").is_ok() {
        "bash".to_string()
    } else {
        "sh".to_string()
    }
}

const OUTPUT_LIMIT_LINES: usize = 2000;
pub const OUTPUT_LIMIT_BYTES: usize = 50_000;
const OUTPUT_PREVIEW_LINES: usize = 50;

const OUTPUT_SLOTS: usize = 8;

/// Result of truncating command output.
struct TruncateResult {
    /// The (possibly truncated) text to display.
    text: String,
    /// When output was truncated, the path where the full output was saved
    /// and a human-readable reason for the truncation.
    truncation: Option<TruncationInfo>,
}

struct TruncationInfo {
    path: PathBuf,
    reason: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShellParams {
    pub command: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    /// Process exit code. 0 indicates success, non-zero indicates failure.
    /// Absent if the process was killed (e.g. timeout).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// True if the command was killed because it exceeded the timeout.
    #[serde(default)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub timed_out: bool,
    /// True if the command was killed because the tool call was cancelled.
    #[serde(default)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cancelled: bool,
    /// True if output collection was cut short after the shell exited.
    #[serde(default)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub output_truncated: bool,
    /// Error reported by output collection after process exit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_collection_error: Option<String>,
}

/// Resolve the user's full PATH by running a login shell.
///
/// When goosed is launched from a desktop app (e.g. Electron), it may inherit
/// a minimal PATH like `/usr/bin:/bin`. This function spawns a login shell to
/// source the user's profile and recover the full PATH.
#[cfg(not(windows))]
pub(crate) fn resolve_login_shell_path() -> Option<String> {
    use process_wrap::std::{CommandWrap, ProcessSession};

    let shell = unix_shell();
    let login_args = unix_login_shell_command_args(&shell);

    // Build the command, varying only the flatpak vs direct invocation.
    let mut cmd = if is_flatpak() {
        let mut c = flatpak_spawn_process();
        c.arg(&shell).args(login_args);
        CommandWrap::from(c)
    } else {
        let mut c = std::process::Command::new(&shell);
        c.args(login_args);
        CommandWrap::from(c)
    };

    cmd.command_mut()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    // Spawn in a new session so that bash's interactive job-control setup
    // (TIOCSPGRP) cannot steal the terminal foreground from goose, which
    // would cause goose to receive SIGTTIN and be suspended on startup.
    cmd.wrap(ProcessSession);

    let mut child = cmd.spawn().ok()?;

    let mut stdout = child.stdout().take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        use std::io::Read;
        if stdout.read_to_end(&mut buf).is_ok() {
            let _ = tx.send(buf);
        }
    });

    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(buf)
            if child
                .wait()
                .is_ok_and(|s: std::process::ExitStatus| s.success()) =>
        {
            // Take the last non-empty line — interactive shells may emit
            // extra output from profile scripts before our echo.
            String::from_utf8_lossy(&buf)
                .lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string())
                .filter(|path| !path.is_empty())
        }
        _ => {
            let _ = child.kill();
            None
        }
    }
}

/// Resolves the user's login-shell PATH in the background.
///
/// Spawned at `ShellTool` construction so the ~hundreds-of-ms cost of sourcing
/// the user's shell profile overlaps with the rest of agent setup and the
/// first LLM turn. The first `shell` invocation awaits the result; subsequent
/// invocations read from the cached cell.
#[cfg(not(windows))]
struct LoginPath {
    cell: OnceCell<Option<Arc<str>>>,
    handle: Mutex<Option<JoinHandle<Option<String>>>>,
}

#[cfg(not(windows))]
impl LoginPath {
    fn spawn() -> Self {
        let handle = tokio::task::spawn_blocking(resolve_login_shell_path);
        Self {
            cell: OnceCell::new(),
            handle: Mutex::new(Some(handle)),
        }
    }

    fn resolved(value: Option<String>) -> Self {
        let cell = OnceCell::new();
        let _ = cell.set(value.map(Arc::from));
        Self {
            cell,
            handle: Mutex::new(None),
        }
    }

    async fn get(&self) -> Option<Arc<str>> {
        self.cell
            .get_or_init(|| async {
                let handle = self
                    .handle
                    .lock()
                    .expect("login_path mutex poisoned")
                    .take();
                match handle {
                    Some(h) => h.await.ok().flatten().map(Arc::from),
                    None => None,
                }
            })
            .await
            .clone()
    }
}

pub struct ShellTool {
    output_dir: tempfile::TempDir,
    call_index: AtomicUsize,
    #[cfg(not(windows))]
    login_path: LoginPath,
}

pub(crate) struct ShellExecution {
    pub(crate) result: CallToolResult,
    pub(crate) env_overlay: Option<EnvOverlay>,
}

impl ShellTool {
    pub fn new(use_login_shell_path: bool) -> std::io::Result<Self> {
        Ok(Self {
            output_dir: tempfile::tempdir()?,
            call_index: AtomicUsize::new(0),
            #[cfg(not(windows))]
            login_path: if use_login_shell_path {
                LoginPath::spawn()
            } else {
                LoginPath::resolved(None)
            },
        })
    }

    #[cfg(test)]
    pub fn new_for_test() -> std::io::Result<Self> {
        Ok(Self {
            output_dir: tempfile::tempdir()?,
            call_index: AtomicUsize::new(0),
            #[cfg(not(windows))]
            login_path: LoginPath::resolved(None),
        })
    }

    #[cfg(test)]
    pub(crate) async fn shell(
        &self,
        params: ShellParams,
        working_dir: &std::path::Path,
        env_overlay: &EnvOverlay,
        cancellation_token: CancellationToken,
    ) -> ShellExecution {
        self.shell_with_cwd(params, Some(working_dir), env_overlay, cancellation_token)
            .await
    }

    pub(crate) async fn shell_with_cwd(
        &self,
        params: ShellParams,
        working_dir: Option<&std::path::Path>,
        env_overlay: &EnvOverlay,
        cancellation_token: CancellationToken,
    ) -> ShellExecution {
        if params.command.trim().is_empty() {
            return ShellExecution {
                result: Self::error_result("Command cannot be empty.", None),
                env_overlay: None,
            };
        }

        #[cfg(not(windows))]
        let login_path = self.login_path.get().await;
        #[cfg(not(windows))]
        let login_path_ref = login_path.as_deref();
        #[cfg(windows)]
        let login_path_ref: Option<&str> = None;

        #[cfg(windows)]
        let env_overlay = EnvOverlay::new();
        #[cfg(windows)]
        let env_overlay = &env_overlay;

        let execution = match run_command(
            &params.command,
            params.timeout_secs,
            working_dir,
            login_path_ref,
            env_overlay,
            self.output_dir.path(),
            cancellation_token,
        )
        .await
        {
            Ok(execution) => execution,
            Err(error) => {
                return ShellExecution {
                    result: Self::error_result(&error, None),
                    env_overlay: None,
                };
            }
        };

        let next_env_overlay = if let Some(env_after) = &execution.env_after {
            let base_env = base_environment(login_path_ref);
            Some(diff_environment(&base_env, env_after))
        } else {
            None
        };

        // Derive stdout, stderr, and interleaved display from the single tagged-line buffer
        let (raw_stdout, raw_stderr, interleaved) = split_lines(&execution.lines);

        let output_dir = self.output_dir.path();
        let slot = self.call_index.fetch_add(1, Ordering::Relaxed) % OUTPUT_SLOTS;
        let stdout_result = if raw_stdout.is_empty() {
            TruncateResult {
                text: String::new(),
                truncation: None,
            }
        } else {
            match truncate_output(&raw_stdout, &format!("stdout-{slot}"), output_dir) {
                Ok(r) => r,
                Err(error) => {
                    return ShellExecution {
                        result: Self::error_result(&error, None),
                        env_overlay: next_env_overlay,
                    };
                }
            }
        };
        let stderr_result = if raw_stderr.is_empty() {
            TruncateResult {
                text: String::new(),
                truncation: None,
            }
        } else {
            match truncate_output(&raw_stderr, &format!("stderr-{slot}"), output_dir) {
                Ok(r) => r,
                Err(error) => {
                    return ShellExecution {
                        result: Self::error_result(&error, None),
                        env_overlay: next_env_overlay,
                    };
                }
            }
        };

        let shell_output = ShellOutput {
            stdout: stdout_result.text,
            stderr: stderr_result.text,
            exit_code: execution.exit_code,
            timed_out: execution.timed_out,
            cancelled: execution.cancelled,
            output_truncated: execution.output_truncated,
            output_collection_error: execution.output_collection_error.clone(),
        };
        let structured_content = serde_json::to_value(&shell_output).ok();
        let render_result = match render_output(&interleaved, &format!("output-{slot}"), output_dir)
        {
            Ok(r) => r,
            Err(error) => {
                return ShellExecution {
                    result: Self::error_result(&error, None),
                    env_overlay: next_env_overlay,
                };
            }
        };
        let mut rendered = render_result.text;

        // Collect truncation notices from stdout, stderr, and interleaved output.
        // These are delivered as a separate Content block so the model sees them as
        // instructions rather than part of the command's data output.
        let truncation_notices: Vec<String> = [
            &stdout_result.truncation,
            &stderr_result.truncation,
            &render_result.truncation,
        ]
        .iter()
        .filter_map(|t| t.as_ref().map(truncation_notice))
        .collect();

        let is_error = if execution.cancelled {
            rendered.push_str("\n\nCommand cancelled");
            true
        } else if execution.timed_out {
            if let Some(timeout_secs) = params.timeout_secs {
                rendered.push_str(&format!(
                    "\n\nCommand timed out after {} seconds",
                    timeout_secs
                ));
            } else {
                rendered.push_str("\n\nCommand timed out");
            }
            true
        } else {
            execution.exit_code.unwrap_or(1) != 0
        };

        if execution.output_truncated {
            rendered.push_str(
                "\n\nOutput may be incomplete because stream draining timed out after process exit.",
            );
        }
        if let Some(error) = &execution.output_collection_error {
            rendered.push_str(&format!(
                "\n\nOutput collection error occurred; output may be incomplete: {error}"
            ));
        }

        let is_error = is_error || execution.output_collection_error.is_some();

        if is_error {
            if let Some(code) = execution.exit_code.filter(|c| *c != 0) {
                rendered.push_str(&format!("\n\nCommand exited with code {code}"));
            }
            let mut error_blocks = vec![Content::text(rendered).with_priority(0.0)];
            if !truncation_notices.is_empty() {
                error_blocks.push(Content::text(truncation_notices.join("\n")).with_priority(0.0));
            }
            let mut result = CallToolResult::error(error_blocks);
            result.structured_content = structured_content;
            return ShellExecution {
                result,
                env_overlay: next_env_overlay,
            };
        }

        let mut content_blocks = vec![Content::text(rendered).with_priority(0.0)];
        if !truncation_notices.is_empty() {
            content_blocks.push(Content::text(truncation_notices.join("\n")).with_priority(0.0));
        }
        let mut result = CallToolResult::success(content_blocks);
        result.structured_content = structured_content;
        ShellExecution {
            result,
            env_overlay: next_env_overlay,
        }
    }

    pub fn error_result(message: &str, exit_code: Option<i32>) -> CallToolResult {
        let shell_output = ShellOutput {
            stdout: String::new(),
            stderr: message.to_string(),
            exit_code,
            timed_out: false,
            cancelled: false,
            output_truncated: false,
            output_collection_error: None,
        };
        let mut result = CallToolResult::error(vec![Content::text(message).with_priority(0.0)]);
        result.structured_content = serde_json::to_value(&shell_output).ok();
        result
    }
}

struct ExecutionOutput {
    /// Lines in arrival order, tagged by source: (is_stderr, text)
    lines: Vec<(bool, String)>,
    exit_code: Option<i32>,
    timed_out: bool,
    cancelled: bool,
    env_after: Option<HashMap<String, String>>,
    output_truncated: bool,
    output_collection_error: Option<String>,
}

#[cfg(not(windows))]
struct EnvCapture {
    script: tempfile::NamedTempFile,
    after: tempfile::NamedTempFile,
}

#[cfg(not(windows))]
impl EnvCapture {
    fn new(command_line: &str, output_dir: &Path) -> Result<Self, String> {
        let mut script = tempfile::Builder::new()
            .prefix("shell-command-")
            .suffix(".sh")
            .tempfile_in(output_dir)
            .map_err(|error| format!("Failed to create shell wrapper: {error}"))?;
        script
            .write_all(command_line.as_bytes())
            .map_err(|error| format!("Failed to write shell wrapper: {error}"))?;
        script
            .write_all(
                format!(
                    "\n__GOOSE_STATUS=$?\nenv -0 > \"${ENV_CAPTURE_PATH_VAR}\"\nexit \"$__GOOSE_STATUS\"\n"
                )
                .as_bytes(),
            )
            .map_err(|error| format!("Failed to write shell wrapper: {error}"))?;
        script
            .flush()
            .map_err(|error| format!("Failed to write shell wrapper: {error}"))?;

        let after = tempfile::Builder::new()
            .prefix("shell-env-after-")
            .tempfile_in(output_dir)
            .map_err(|error| format!("Failed to create shell env capture: {error}"))?;

        Ok(Self { script, after })
    }

    fn script_path(&self) -> &Path {
        self.script.path()
    }

    fn after_path(&self) -> &Path {
        self.after.path()
    }

    fn read_after_env(&self) -> Option<HashMap<String, String>> {
        let bytes = std::fs::read(self.after_path()).ok()?;
        if bytes.is_empty() {
            return None;
        }
        Some(parse_env_block(&bytes))
    }
}

#[cfg(windows)]
struct EnvCapture;

#[cfg(windows)]
impl EnvCapture {
    fn read_after_env(&self) -> Option<HashMap<String, String>> {
        None
    }
}

fn base_environment(login_path: Option<&str>) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();
    if let Some(path) = login_path {
        env.insert("PATH".to_string(), path.to_string());
    }
    env
}

#[cfg(not(windows))]
fn env_updates(login_path: Option<&str>, env_overlay: &EnvOverlay) -> EnvOverlay {
    let mut updates = env_overlay.clone();
    if let Some(path) = login_path {
        updates
            .entry("PATH".to_string())
            .or_insert_with(|| Some(path.to_string()));
    }
    updates
}

#[cfg(not(windows))]
fn apply_command_env(
    command: &mut tokio::process::Command,
    login_path: Option<&str>,
    env_overlay: &EnvOverlay,
) {
    for (key, value) in env_updates(login_path, env_overlay) {
        match value {
            Some(value) => {
                command.env(key, value);
            }
            None => {
                command.env_remove(key);
            }
        }
    }
}

#[cfg(not(windows))]
fn apply_flatpak_env(
    command: &mut tokio::process::Command,
    login_path: Option<&str>,
    env_overlay: &EnvOverlay,
) {
    for (key, value) in env_updates(login_path, env_overlay) {
        match value {
            Some(value) => {
                command.arg(format!("--env={key}={value}"));
            }
            None => {
                command.arg(format!("--unset-env={key}"));
            }
        }
    }
}

fn diff_environment(
    base_env: &HashMap<String, String>,
    after_env: &HashMap<String, String>,
) -> EnvOverlay {
    let keys: HashSet<&String> = base_env.keys().chain(after_env.keys()).collect();
    keys.into_iter()
        .filter_map(|key| {
            let before = base_env.get(key);
            let after = after_env.get(key);
            if before == after {
                None
            } else {
                Some((key.clone(), after.cloned()))
            }
        })
        .collect()
}

#[cfg(not(windows))]
fn parse_env_block(bytes: &[u8]) -> HashMap<String, String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let split_at = entry.iter().position(|byte| *byte == b'=')?;
            let (key, value_with_equals) = entry.split_at(split_at);
            let value = &value_with_equals[1..];
            Some((
                String::from_utf8_lossy(key).into_owned(),
                String::from_utf8_lossy(value).into_owned(),
            ))
        })
        .filter(|(key, _)| key != ENV_CAPTURE_PATH_VAR)
        .collect()
}

async fn run_command(
    command_line: &str,
    timeout_secs: Option<u64>,
    working_dir: Option<&std::path::Path>,
    login_path: Option<&str>,
    env_overlay: &EnvOverlay,
    output_dir: &std::path::Path,
    cancellation_token: CancellationToken,
) -> Result<ExecutionOutput, String> {
    if cancellation_token.is_cancelled() {
        return Ok(ExecutionOutput {
            lines: Vec::new(),
            exit_code: None,
            timed_out: false,
            cancelled: true,
            env_after: None,
            output_truncated: false,
            output_collection_error: None,
        });
    }

    let (mut command, env_capture) = build_shell_command(
        command_line,
        working_dir,
        login_path,
        env_overlay,
        output_dir,
    )?;

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.stdin(Stdio::null());

    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to spawn shell command: {}", error))?;

    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let child_stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let output_task = tokio::spawn(collect_tagged_lines(child_stdout, child_stderr, tx));
    let abort_handle = output_task.abort_handle();

    let mut timed_out = false;
    let mut cancelled = false;
    let timeout = async {
        if let Some(timeout_secs) = timeout_secs.filter(|value| *value > 0) {
            tokio::time::sleep(Duration::from_secs(timeout_secs)).await;
        } else {
            std::future::pending::<()>().await
        }
    };

    let exit_code = tokio::select! {
        wait_result = child.wait() => {
            wait_result
                .map_err(|error| format!("Failed waiting on shell command: {}", error))?
                .code()
        }
        _ = timeout => {
            timed_out = true;
            None
        }
        _ = cancellation_token.cancelled() => {
            cancelled = true;
            None
        }
    };

    if timed_out || cancelled {
        kill_child_process(&mut child);
        let _ = child.wait().await;
    }

    const OUTPUT_DRAIN_TIMEOUT_MILLIS: u64 = 500;
    let mut output_collection_error = None;
    let output_truncated = match tokio::time::timeout(
        Duration::from_millis(OUTPUT_DRAIN_TIMEOUT_MILLIS),
        output_task,
    )
    .await
    {
        Ok(Ok(Ok(()))) => false,
        Ok(Ok(Err(e))) => {
            output_collection_error = Some(format!("Failed to collect shell output: {}", e));
            false
        }
        Ok(Err(e)) => {
            output_collection_error = Some(format!("Failed to collect shell output: {}", e));
            false
        }
        Err(_) => {
            tracing::debug!(
                    "output drain timed out after {OUTPUT_DRAIN_TIMEOUT_MILLIS}ms (backgrounded process?)"
                );
            abort_handle.abort();
            true
        }
    };

    rx.close();
    let mut lines = Vec::new();
    while let Some(item) = rx.recv().await {
        lines.push(item);
    }

    Ok(ExecutionOutput {
        lines,
        exit_code,
        timed_out,
        cancelled,
        env_after: env_capture.as_ref().and_then(EnvCapture::read_after_env),
        output_truncated,
        output_collection_error,
    })
}

fn kill_child_process(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let process_group = format!("-{pid}");
            if std::process::Command::new("kill")
                .args(["-KILL", &process_group])
                .status()
                .is_ok_and(|status| status.success())
            {
                return;
            }
        }
    }

    let _ = child.start_kill();
}

fn build_shell_command(
    command_line: &str,
    working_dir: Option<&std::path::Path>,
    login_path: Option<&str>,
    env_overlay: &EnvOverlay,
    output_dir: &std::path::Path,
) -> Result<(tokio::process::Command, Option<EnvCapture>), String> {
    #[cfg(windows)]
    let _ = (env_overlay, output_dir);

    #[cfg(windows)]
    let mut command = {
        let shell = windows_shell();
        let shell_stem = shell_basename(&shell);
        let mut command = tokio::process::Command::new(&shell);
        match shell_stem.as_str() {
            "pwsh" | "powershell" => {
                command.args(["-NoProfile", "-NonInteractive", "-Command", command_line]);
            }
            "cmd" => {
                command.arg("/C").raw_arg(command_line);
            }
            // POSIX-like shells (bash, zsh, etc.) on Windows (e.g. Cygwin/MSYS2)
            _ => {
                command.args(["-c", command_line]);
            }
        }
        if let Some(path) = working_dir {
            command.current_dir(path);
        }
        if let Some(path) = login_path {
            command.env("PATH", path);
        }
        command
    };
    #[cfg(windows)]
    let env_capture = None;

    #[cfg(not(windows))]
    let (mut command, env_capture) = {
        let shell = unix_shell();
        let flatpak = is_flatpak();
        // Under Flatpak the wrapper script lives in the sandbox-private temp dir,
        // which `flatpak-spawn --host` cannot read, so env capture is skipped there.
        let env_capture = if !flatpak && unix_shell_flavor(&shell) == UnixShellFlavor::Posix {
            Some(EnvCapture::new(command_line, output_dir)?)
        } else {
            None
        };

        if flatpak {
            let mut command = flatpak_spawn_command();
            if let Some(path) = working_dir {
                command.arg(format!("--directory={}", path.display()));
            }
            apply_flatpak_env(&mut command, login_path, env_overlay);
            command.arg(&shell);
            command.args(unix_shell_command_args(command_line));
            (command, env_capture)
        } else {
            let mut command = tokio::process::Command::new(shell);
            if let Some(env_capture) = &env_capture {
                command.arg(env_capture.script_path());
            } else {
                command.args(unix_shell_command_args(command_line));
            }
            if let Some(path) = working_dir {
                command.current_dir(path);
            }
            apply_command_env(&mut command, login_path, env_overlay);
            if let Some(env_capture) = &env_capture {
                command.env(ENV_CAPTURE_PATH_VAR, env_capture.after_path());
            }
            (command, env_capture)
        }
    };

    configure_subprocess(&mut command);
    Ok((command, env_capture))
}

/// Split tagged lines into (stdout, stderr, interleaved) strings.
fn split_lines(lines: &[(bool, String)]) -> (String, String, String) {
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut interleaved = String::new();
    let mut stdout_started = false;
    let mut stderr_started = false;
    for (i, (is_stderr, text)) in lines.iter().enumerate() {
        if i > 0 {
            interleaved.push('\n');
        }
        interleaved.push_str(text);
        let (target, started) = if *is_stderr {
            (&mut stderr, &mut stderr_started)
        } else {
            (&mut stdout, &mut stdout_started)
        };
        if *started {
            target.push('\n');
        }
        *started = true;
        target.push_str(text);
    }
    (stdout, stderr, interleaved)
}

/// Collect lines from stdout and stderr and send `(is_stderr, line)` tuples to `tx`.
async fn collect_tagged_lines(
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    tx: tokio::sync::mpsc::UnboundedSender<(bool, String)>,
) -> Result<(), std::io::Error> {
    let stdout_lines = SplitStream::new(BufReader::new(stdout).split(b'\n')).map(|l| (false, l));
    let stderr_lines = SplitStream::new(BufReader::new(stderr).split(b'\n')).map(|l| (true, l));
    let mut merged = stdout_lines.merge(stderr_lines);

    while let Some((is_stderr, line)) = merged.next().await {
        let line = line?;
        let _ = tx.send((is_stderr, String::from_utf8_lossy(&line).into_owned()));
    }
    Ok(())
}

/// Build a human-readable truncation notice with platform-appropriate commands.
fn truncation_notice(info: &TruncationInfo) -> String {
    let path = info.path.display();
    let commands = if cfg!(windows) {
        "PowerShell commands like `Get-Content -TotalCount 200`, `Select-String`, or \
         `Get-Content | Select-Object -Skip 100 -First 100`"
    } else {
        "shell commands like `head`, `tail`, or `sed -n '100,200p'`"
    };
    format!(
        "[{reason} Full output saved to {path}. \
         Read it with {commands} up to {limit} lines at a time.]",
        reason = info.reason,
        limit = OUTPUT_LIMIT_LINES,
    )
}

fn render_output(
    full_output: &str,
    label: &str,
    output_dir: &std::path::Path,
) -> Result<TruncateResult, String> {
    if full_output.is_empty() {
        return Ok(TruncateResult {
            text: "(no output)".to_string(),
            truncation: None,
        });
    }
    truncate_output(full_output, label, output_dir)
}

fn truncate_output(
    full_output: &str,
    label: &str,
    output_dir: &std::path::Path,
) -> Result<TruncateResult, String> {
    let lines: Vec<&str> = full_output.split('\n').collect();
    let total_lines = lines.len();
    let total_bytes = full_output.len();

    let exceeded_lines = total_lines > OUTPUT_LIMIT_LINES;
    let exceeded_bytes = total_bytes > OUTPUT_LIMIT_BYTES;

    if !exceeded_lines && !exceeded_bytes {
        return Ok(TruncateResult {
            text: full_output.to_string(),
            truncation: None,
        });
    }

    let output_path = save_full_output(full_output, label, output_dir)?;

    let preview_start = total_lines.saturating_sub(OUTPUT_PREVIEW_LINES);
    let preview = lines[preview_start..].join("\n");

    let reason = if exceeded_lines {
        format!("Output exceeded {OUTPUT_LIMIT_LINES} line limit ({total_lines} lines total).")
    } else {
        format!(
            "Output exceeded {} byte limit ({total_bytes} bytes total).",
            OUTPUT_LIMIT_BYTES
        )
    };

    Ok(TruncateResult {
        text: preview,
        truncation: Some(TruncationInfo {
            path: output_path,
            reason,
        }),
    })
}

fn save_full_output(
    output: &str,
    label: &str,
    output_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let path = output_dir.join(label);
    std::fs::write(&path, output).map_err(|e| format!("Failed to write output buffer: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::RawContent;

    fn extract_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(text) => &text.text,
            _ => panic!("expected text"),
        }
    }

    fn extract_shell_output(result: &CallToolResult) -> ShellOutput {
        let value = result
            .structured_content
            .clone()
            .expect("expected structured content");
        serde_json::from_value(value).expect("expected shell output structured content")
    }

    #[tokio::test]
    async fn shell_executes_command() {
        let tool = ShellTool::new_for_test().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let execution = tool
            .shell(
                ShellParams {
                    command: "echo hello".to_string(),
                    timeout_secs: None,
                },
                dir.path(),
                &EnvOverlay::new(),
                CancellationToken::new(),
            )
            .await;
        let result = execution.result;

        assert_eq!(result.is_error, Some(false));
        assert!(extract_text(&result).contains("hello"));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn shell_returns_error_for_non_zero_exit() {
        let tool = ShellTool::new_for_test().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let execution = tool
            .shell(
                ShellParams {
                    command: "echo fail && exit 7".to_string(),
                    timeout_secs: None,
                },
                dir.path(),
                &EnvOverlay::new(),
                CancellationToken::new(),
            )
            .await;
        let result = execution.result;

        assert_eq!(result.is_error, Some(true));
        assert!(extract_text(&result).contains("Command exited with code 7"));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn shell_uses_working_dir_for_relative_execution() {
        let dir = tempfile::tempdir().unwrap();
        let tool = ShellTool::new_for_test().unwrap();
        let execution = tool
            .shell(
                ShellParams {
                    command: "pwd".to_string(),
                    timeout_secs: None,
                },
                dir.path(),
                &EnvOverlay::new(),
                CancellationToken::new(),
            )
            .await;
        let result = execution.result;

        assert_eq!(result.is_error, Some(false));
        let observed = std::fs::canonicalize(extract_text(&result)).unwrap();
        let expected = std::fs::canonicalize(dir.path()).unwrap();
        assert_eq!(observed, expected);
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn shell_persists_exported_environment_between_calls() {
        let dir = tempfile::tempdir().unwrap();
        let tool = ShellTool::new_for_test().unwrap();
        let name = "GOOSE_TEST_ENV_TRACKING_VALUE";
        let mut env_overlay = EnvOverlay::new();

        let export = tool
            .shell(
                ShellParams {
                    command: format!("export {name}=persisted"),
                    timeout_secs: None,
                },
                dir.path(),
                &env_overlay,
                CancellationToken::new(),
            )
            .await;
        assert_eq!(export.result.is_error, Some(false));
        env_overlay = export.env_overlay.expect("expected env overlay update");

        let read = tool
            .shell(
                ShellParams {
                    command: format!("printf '%s' \"${{{name}}}\""),
                    timeout_secs: None,
                },
                dir.path(),
                &env_overlay,
                CancellationToken::new(),
            )
            .await;
        assert_eq!(read.result.is_error, Some(false));
        assert_eq!(extract_text(&read.result), "persisted");
        env_overlay = read.env_overlay.expect("expected env overlay update");

        let unset = tool
            .shell(
                ShellParams {
                    command: format!("unset {name}"),
                    timeout_secs: None,
                },
                dir.path(),
                &env_overlay,
                CancellationToken::new(),
            )
            .await;
        assert_eq!(unset.result.is_error, Some(false));
        env_overlay = unset.env_overlay.expect("expected env overlay update");

        let read_after_unset = tool
            .shell(
                ShellParams {
                    command: format!("printf '%s' \"${{{name}-unset}}\""),
                    timeout_secs: None,
                },
                dir.path(),
                &env_overlay,
                CancellationToken::new(),
            )
            .await;
        assert_eq!(read_after_unset.result.is_error, Some(false));
        assert_eq!(extract_text(&read_after_unset.result), "unset");
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_shell_flavor_detects_nushell_names() {
        assert_eq!(unix_shell_flavor("nu"), UnixShellFlavor::Nushell);
        assert_eq!(unix_shell_flavor("nushell"), UnixShellFlavor::Nushell);
        assert_eq!(
            unix_shell_flavor("/etc/profiles/per-user/can/bin/nu"),
            UnixShellFlavor::Nushell
        );
        assert_eq!(unix_shell_flavor("/bin/bash"), UnixShellFlavor::Posix);
        assert_eq!(unix_shell_flavor("fish"), UnixShellFlavor::Other);
        assert_eq!(unix_shell_flavor("/bin/csh"), UnixShellFlavor::Other);
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_login_shell_command_args_use_nushell_probe() {
        assert_eq!(
            unix_login_shell_command_args("nu"),
            ["-l", "-i", "-c", "print ($env.PATH | str join (char esep))"]
        );
        assert_eq!(
            unix_login_shell_command_args("/bin/bash"),
            ["-l", "-i", "-c", "echo $PATH"]
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_shell_command_args_wrap_commands_for_execution() {
        assert_eq!(unix_shell_command_args("ls -la"), ["-c", "ls -la"]);
    }

    #[test]
    fn render_output_returns_full_output_when_under_limit() {
        let dir = tempfile::tempdir().unwrap();
        let input = (0..100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let result = render_output(&input, "test", dir.path()).unwrap();
        assert_eq!(result.text, input);
        assert!(result.truncation.is_none());
    }

    #[test]
    fn render_output_shows_empty_message() {
        let dir = tempfile::tempdir().unwrap();
        let result = render_output("", "test", dir.path()).unwrap();
        assert_eq!(result.text, "(no output)");
        assert!(result.truncation.is_none());
    }

    #[test]
    fn render_output_truncates_when_lines_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let input = (0..2500)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let result = render_output(&input, "test_lines", dir.path()).unwrap();
        let preview = &result.text;

        assert_eq!(preview.lines().count(), OUTPUT_PREVIEW_LINES);
        assert!(preview.starts_with("line 2450"));
        assert!(preview.contains("line 2499"));

        let info = result
            .truncation
            .as_ref()
            .expect("expected truncation info");
        assert!(info.reason.contains("2000 line limit"));
        assert!(info.reason.contains("2500 lines total"));

        let notice = truncation_notice(info);
        assert!(notice.contains("Full output saved to"));
    }

    #[test]
    fn render_output_truncates_when_bytes_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let long_line = "x".repeat(1000);
        let input = (0..100)
            .map(|_| long_line.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(input.len() > OUTPUT_LIMIT_BYTES);
        assert!(input.lines().count() <= OUTPUT_LIMIT_LINES);

        let result = render_output(&input, "test_bytes", dir.path()).unwrap();
        let info = result
            .truncation
            .as_ref()
            .expect("expected truncation info");
        assert!(info.reason.contains("byte limit"));
        assert!(info.reason.contains("bytes total"));

        let notice = truncation_notice(info);
        assert!(notice.contains("Full output saved to"));
    }

    #[test]
    fn save_full_output_reuses_same_path() {
        let dir = tempfile::tempdir().unwrap();
        let path1 = save_full_output("first", "test_reuse", dir.path()).unwrap();
        let path2 = save_full_output("second", "test_reuse", dir.path()).unwrap();
        assert_eq!(path1, path2);
        // Note: we intentionally don't assert file content here because
        // parallel tests (render_output_truncates_*) share the same static
        // temp file and can overwrite the content between our write and read.
    }

    #[test]
    fn save_full_output_uses_separate_files_per_label() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = save_full_output("aaa", "label_a", dir.path()).unwrap();
        let path_b = save_full_output("bbb", "label_b", dir.path()).unwrap();
        assert_ne!(path_a, path_b);
        assert_eq!(std::fs::read_to_string(&path_a).unwrap(), "aaa");
        assert_eq!(std::fs::read_to_string(&path_b).unwrap(), "bbb");
    }

    #[test]
    fn call_index_cycles_through_slots() {
        let tool = ShellTool::new_for_test().unwrap();
        for _cycle in 0..3 {
            for expected in 0..OUTPUT_SLOTS {
                let slot = tool.call_index.fetch_add(1, Ordering::Relaxed) % OUTPUT_SLOTS;
                assert_eq!(slot, expected);
            }
        }
    }

    #[test]
    fn concurrent_calls_get_distinct_slots() {
        let tool = ShellTool::new_for_test().unwrap();
        let mut slots: Vec<usize> = (0..OUTPUT_SLOTS)
            .map(|_| tool.call_index.fetch_add(1, Ordering::Relaxed) % OUTPUT_SLOTS)
            .collect();
        slots.sort();
        let expected: Vec<usize> = (0..OUTPUT_SLOTS).collect();
        assert_eq!(slots, expected);
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn shell_cancellation_kills_child_process_tree() {
        let tool = ShellTool::new_for_test().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sentinel = dir.path().join("ticks");
        let command = format!(
            "( while true; do echo tick >> {sentinel:?}; sleep 0.1; done ) & wait",
            sentinel = sentinel.display()
        );

        let cancel_token = CancellationToken::new();
        let handle = {
            let cancel_token = cancel_token.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(400)).await;
                cancel_token.cancel();
            })
        };

        let execution = tool
            .shell(
                ShellParams {
                    command,
                    timeout_secs: None,
                },
                dir.path(),
                &EnvOverlay::new(),
                cancel_token,
            )
            .await;
        handle.await.unwrap();

        assert!(
            extract_shell_output(&execution.result).cancelled,
            "expected the execution to be reported as cancelled"
        );
        assert!(
            extract_text(&execution.result).contains("Command cancelled"),
            "expected cancellation notice in rendered output"
        );

        let ticks_after_cancel = std::fs::read_to_string(&sentinel).unwrap().lines().count();
        tokio::time::sleep(Duration::from_millis(500)).await;
        let ticks_later = std::fs::read_to_string(&sentinel).unwrap().lines().count();
        assert_eq!(
            ticks_after_cancel, ticks_later,
            "background child kept writing after cancellation, process group was not killed"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn shell_does_not_hang_on_backgrounded_process() {
        struct KillOnDrop(String);
        impl Drop for KillOnDrop {
            fn drop(&mut self) {
                let _ = std::process::Command::new("kill")
                    .args(["-9", &self.0])
                    .status();
            }
        }

        let tool = ShellTool::new_for_test().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let start = std::time::Instant::now();
        let execution = tool
            .shell(
                ShellParams {
                    command: "echo before && sleep 300 & echo bgpid:$! && echo after".to_string(),
                    timeout_secs: None,
                },
                dir.path(),
                &EnvOverlay::new(),
                CancellationToken::new(),
            )
            .await;
        let result = execution.result;

        assert!(
            start.elapsed().as_secs() < 10,
            "shell tool should return quickly, not wait for backgrounded sleep"
        );
        assert_eq!(result.is_error, Some(false));
        let text = extract_text(&result);
        let shell_output = extract_shell_output(&result);
        let background_pid = text
            .lines()
            .find_map(|line| line.strip_prefix("bgpid:"))
            .map(str::trim)
            .expect("expected bgpid in output");
        let _cleanup = KillOnDrop(background_pid.to_string());
        assert!(
            shell_output.output_truncated,
            "backgrounded process should set output_truncated"
        );
        assert!(
            shell_output.output_collection_error.is_none(),
            "timeout-based truncation should not set output collection error"
        );
        assert!(
            text.contains("before"),
            "should capture output before background cmd"
        );
        assert!(
            text.contains("after"),
            "should capture output after background cmd"
        );
    }
}
