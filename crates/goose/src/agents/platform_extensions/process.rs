use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::tool_execution::ToolCallContext;
use crate::jobs::{Job, JobSource, JobState, NotifyPolicy};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ServerCapabilities, Tool,
};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "process";

#[allow(dead_code)]
struct ManagedProcess {
    child: Child,
    stdin: Option<tokio::process::ChildStdin>,
    output_lines: Arc<Mutex<Vec<String>>>,
    description: String,
}

pub struct ProcessClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
    next_id: Arc<Mutex<u32>>,
}

impl ProcessClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new(EXTENSION_NAME, "1.0.0").with_title("Process Manager"),
            )
            .with_instructions(
                "Manage background processes and scheduled reminders. Start processes, read output, write stdin, stop them. Schedule timed reminders that interrupt you with a message after a delay. List all active jobs.",
            );

        Ok(Self {
            info,
            context,
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    async fn handle_start(&self, arguments: Option<JsonObject>) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'command' argument")?;
        let wait_for = args
            .get("wait_for")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut id_lock = self.next_id.lock().await;
        let id = format!("proc_{}", *id_lock);
        *id_lock += 1;
        drop(id_lock);

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start process: {}", e))?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let output_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // Channel to signal when stdout closes (process exited)
        let (stdout_done_tx, stdout_done_rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn stdout reader with optional pattern matching
        if let Some(stdout) = stdout {
            let lines = Arc::clone(&output_lines);
            let pattern = wait_for.clone();
            let registry = self.context.job_registry.clone();
            let job_id = id.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines_stream = reader.lines();
                let mut pattern_fired = false;
                while let Ok(Some(line)) = lines_stream.next_line().await {
                    lines.lock().await.push(format!("[stdout] {}", line));
                    if !pattern_fired {
                        if let Some(ref pat) = pattern {
                            if line.contains(pat.as_str()) {
                                pattern_fired = true;
                                if let Some(ref reg) = registry {
                                    reg.lock().await.pattern_matched(
                                        &job_id,
                                        pat.clone(),
                                        line.clone(),
                                    );
                                }
                            }
                        }
                    }
                }
                // Stdout closed — process exited
                let _ = stdout_done_tx.send(());
            });
        }
        if let Some(stderr) = stderr {
            let lines = Arc::clone(&output_lines);
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines_stream = reader.lines();
                while let Ok(Some(line)) = lines_stream.next_line().await {
                    lines.lock().await.push(format!("[stderr] {}", line));
                }
            });
        }

        // Spawn completion watcher — waits for stdout to close then marks job complete/failed
        if let Some(ref registry) = self.context.job_registry {
            let registry = Arc::clone(registry);
            let job_id = id.clone();
            let processes = Arc::clone(&self.processes);
            tokio::spawn(async move {
                let _ = stdout_done_rx.await;
                // Small delay to let stderr drain and child to exit
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                // Try to get exit code from the child
                let code = {
                    let mut procs = processes.lock().await;
                    if let Some(proc) = procs.get_mut(&job_id) {
                        proc.child.try_wait().ok().flatten().and_then(|s| s.code())
                    } else {
                        None
                    }
                };
                let mut reg = registry.lock().await;
                if let Some(job) = reg.get(&job_id) {
                    if !job.state.is_terminal() {
                        if code.unwrap_or(0) == 0 {
                            reg.complete(&job_id, None);
                        } else {
                            reg.fail(&job_id, Some(format!("exit code: {}", code.unwrap_or(-1))));
                        }
                    }
                }
            });
        }

        let description = if command.len() > 60 {
            format!("{}...", &command.chars().take(60).collect::<String>())
        } else {
            command.to_string()
        };

        // Register in job registry BEFORE watcher can fire
        if let Some(ref registry) = self.context.job_registry {
            let job = Job {
                id: id.clone(),
                source: JobSource::Process,
                description: description.clone(),
                state: JobState::Working,
                batch_id: None,
                notify_policy: NotifyPolicy::OnCompletion,
                meta: crate::jobs::JobMeta::default(),
                created_at: std::time::Instant::now(),
                last_activity: std::time::Instant::now(),
                notifications: Vec::new(),
                result_summary: None,
            };
            registry.lock().await.register(job);
        }

        let managed = ManagedProcess {
            child,
            stdin,
            output_lines: output_lines.clone(),
            description: description.clone(),
        };

        // Store the output_lines reference for read_output
        // We'll read from the Arc in read_output
        self.processes.lock().await.insert(id.clone(), managed);

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Process started with handle: {}\nCommand: {}\n\nUse read_output(\"{}\") to check output, write_stdin(\"{}\", ...) to send input, stop_process(\"{}\") to terminate.",
            id, command, id, id, id
        ))]))
    }

    async fn handle_read_output(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let process_id = args
            .get("process_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'process_id' argument")?;

        let processes = self.processes.lock().await;
        let proc = processes
            .get(process_id)
            .ok_or_else(|| format!("Process '{}' not found", process_id))?;

        let is_running = proc.child.id().is_some();
        let lines = proc.output_lines.lock().await;
        let tail_count = 50;
        let output = if lines.len() > tail_count {
            format!(
                "({} lines total, showing last {})\n{}",
                lines.len(),
                tail_count,
                lines[lines.len() - tail_count..].join("\n")
            )
        } else {
            lines.join("\n")
        };

        let status = if is_running { "running" } else { "exited" };

        Ok(CallToolResult::success(vec![Content::text(format!(
            "# Process {} ({})\n\n{}\n\n---\n{} lines total",
            process_id,
            status,
            output,
            lines.len()
        ))]))
    }

    async fn handle_write_stdin(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let process_id = args
            .get("process_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'process_id' argument")?;
        let input = args
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'input' argument")?;

        let mut processes = self.processes.lock().await;
        let proc = processes
            .get_mut(process_id)
            .ok_or_else(|| format!("Process '{}' not found", process_id))?;

        let stdin = proc
            .stdin
            .as_mut()
            .ok_or("Process stdin not available (already closed)")?;

        stdin
            .write_all(input.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Failed to write newline: {}", e))?;
        stdin
            .flush()
            .await
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Wrote to {}: {}",
            process_id, input
        ))]))
    }

    async fn handle_stop(&self, arguments: Option<JsonObject>) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let process_id = args
            .get("process_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'process_id' argument")?;

        let mut processes = self.processes.lock().await;
        if let Some(mut proc) = processes.remove(process_id) {
            let _ = proc.child.kill().await;
            let status = proc.child.wait().await.ok();
            let exit_code = status.and_then(|s| s.code());

            if let Some(ref registry) = self.context.job_registry {
                registry.lock().await.cancel(process_id);
            }

            Ok(CallToolResult::success(vec![Content::text(format!(
                "Process {} stopped. Exit code: {:?}",
                process_id, exit_code
            ))]))
        } else {
            drop(processes);
            if let Some(ref registry) = self.context.job_registry {
                let mut reg = registry.lock().await;
                if reg.get(process_id).is_some() {
                    reg.cancel(process_id);
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "Job {} canceled.",
                        process_id
                    ))]));
                }
            }
            Err(format!("Job '{}' not found", process_id))
        }
    }

    async fn handle_schedule_reminder(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let delay_str = args
            .get("delay")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'delay' argument")?;
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'message' argument")?
            .to_string();

        let duration = parse_duration(delay_str)
            .ok_or_else(|| format!("Invalid delay '{}'. Use e.g. 30s, 5m, 2h, 1h30m", delay_str))?;

        let mut id_lock = self.next_id.lock().await;
        let id = format!("timer_{}", *id_lock);
        *id_lock += 1;
        drop(id_lock);

        let registry = self
            .context
            .job_registry
            .as_ref()
            .ok_or("Job registry not available")?;

        let deadline = std::time::Instant::now() + duration;
        let job = Job {
            id: id.clone(),
            source: JobSource::Timer,
            description: message.clone(),
            state: JobState::Working,
            batch_id: None,
            notify_policy: NotifyPolicy::OnCompletion,
            meta: crate::jobs::JobMeta {
                deadline: Some(deadline),
                ..Default::default()
            },
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            notifications: Vec::new(),
            result_summary: None,
        };
        registry.lock().await.register(job);

        let reg = Arc::clone(registry);
        let job_id = id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            reg.lock().await.complete(&job_id, Some(message));
        });

        let human_dur = format_duration(duration);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Reminder scheduled: {} (fires in {}). Job ID: {}",
            args.get("message").unwrap().as_str().unwrap(),
            human_dur,
            id
        ))]))
    }

    async fn handle_list_jobs(&self) -> Result<CallToolResult, String> {
        let registry = self
            .context
            .job_registry
            .as_ref()
            .ok_or("Job registry not available")?;

        let reg = registry.lock().await;
        let jobs = reg.running();
        if jobs.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No active jobs.",
            )]));
        }

        let now = std::time::Instant::now();
        let lines: Vec<String> = jobs
            .iter()
            .map(|j| {
                let state_str = match j.state {
                    JobState::Working => {
                        if let Some(deadline) = j.meta.deadline {
                            if deadline > now {
                                let remaining = deadline - now;
                                format!("Working ({} remaining)", format_duration(remaining))
                            } else {
                                "Working (firing...)".to_string()
                            }
                        } else {
                            "Working".to_string()
                        }
                    }
                    _ => format!("{:?}", j.state),
                };
                let source = match j.source {
                    JobSource::Timer => "timer",
                    JobSource::Process => "process",
                    JobSource::Subagent => "subagent",
                    _ => "job",
                };
                format!(
                    "• {} [{}] {}: \"{}\"",
                    j.id, source, state_str, j.description
                )
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }
}

fn parse_duration(s: &str) -> Option<std::time::Duration> {
    let s = s.trim().to_lowercase();
    let mut total_secs: u64 = 0;
    let mut num_buf = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            num_buf.push(c);
        } else {
            let n: u64 = num_buf.parse().ok()?;
            num_buf.clear();
            match c {
                's' => total_secs += n,
                'm' => total_secs += n * 60,
                'h' => total_secs += n * 3600,
                'd' => total_secs += n * 86400,
                _ => return None,
            }
        }
    }
    // Handle bare number (assume seconds)
    if !num_buf.is_empty() {
        let n: u64 = num_buf.parse().ok()?;
        total_secs += n;
    }
    if total_secs == 0 {
        return None;
    }
    Some(std::time::Duration::from_secs(total_secs))
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m > 0 {
            format!("{}h{}m", h, m)
        } else {
            format!("{}h", h)
        }
    } else if secs >= 60 {
        let m = secs / 60;
        let s = secs % 60;
        if s > 0 {
            format!("{}m{}s", m, s)
        } else {
            format!("{}m", m)
        }
    } else {
        format!("{}s", secs)
    }
}

#[async_trait]
impl McpClientTrait for ProcessClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        let tools = vec![
            Tool::new(
                "start_process",
                "Start a long-running process in the background. Returns a process handle for monitoring and interaction.".to_string(),
                serde_json::json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Shell command to run"
                        },
                        "wait_for": {
                            "type": "string",
                            "description": "Optional pattern to watch for in stdout. When matched, triggers a PatternMatched event (forced turn). The process keeps running."
                        }
                    }
                }).as_object().unwrap().clone(),
            ),
            Tool::new(
                "read_output",
                "Read recent output (stdout/stderr) from a running process.".to_string(),
                serde_json::json!({
                    "type": "object",
                    "required": ["process_id"],
                    "properties": {
                        "process_id": {
                            "type": "string",
                            "description": "Process handle from start_process"
                        }
                    }
                }).as_object().unwrap().clone(),
            ),
            Tool::new(
                "write_stdin",
                "Write input to a running process's stdin.".to_string(),
                serde_json::json!({
                    "type": "object",
                    "required": ["process_id", "input"],
                    "properties": {
                        "process_id": {
                            "type": "string",
                            "description": "Process handle from start_process"
                        },
                        "input": {
                            "type": "string",
                            "description": "Text to send to the process stdin"
                        }
                    }
                }).as_object().unwrap().clone(),
            ),
            Tool::new(
                "stop_process",
                "Stop/kill a running process and get its exit status.".to_string(),
                serde_json::json!({
                    "type": "object",
                    "required": ["process_id"],
                    "properties": {
                        "process_id": {
                            "type": "string",
                            "description": "Process handle from start_process"
                        }
                    }
                }).as_object().unwrap().clone(),
            ),
            Tool::new(
                "schedule_reminder",
                "Schedule a timed reminder. Creates a job that fires after a delay, interrupting you with the given message. Use for checking back on async work, polling status, or any deferred action.".to_string(),
                serde_json::json!({
                    "type": "object",
                    "required": ["delay", "message"],
                    "properties": {
                        "delay": {
                            "type": "string",
                            "description": "How long to wait before firing. Examples: 30s, 5m, 2h, 1h30m"
                        },
                        "message": {
                            "type": "string",
                            "description": "Reminder context injected when the timer fires"
                        }
                    }
                }).as_object().unwrap().clone(),
            ),
            Tool::new(
                "list_jobs",
                "List all active background jobs (processes, timers, subagents) with their current status and time remaining.".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }).as_object().unwrap().clone(),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        _ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let result = match name {
            "start_process" => self.handle_start(arguments).await,
            "read_output" => self.handle_read_output(arguments).await,
            "write_stdin" => self.handle_write_stdin(arguments).await,
            "stop_process" => self.handle_stop(arguments).await,
            "schedule_reminder" => self.handle_schedule_reminder(arguments).await,
            "list_jobs" => self.handle_list_jobs().await,
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match result {
            Ok(r) => Ok(r),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                e
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}
