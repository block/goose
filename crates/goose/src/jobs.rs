//! Unified Job Registry for tracking all async work in goose.
//!
//! Every background operation (subagent, process, MCP resource subscription,
//! long-running tool call) is represented as a Job with consistent lifecycle,
//! notification, and tracking semantics.
//!
//! ## State Machine
//!
//! ```text
//! Working → Complete
//! Working → Failed
//! Working → Canceled
//! Working → InputRequired → Working (after input provided)
//! Working → (event: PatternMatched) → still Working
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};

/// Unique identifier for a job.
pub type JobId = String;

/// Unique identifier for a batch of related jobs.
pub type BatchId = String;

/// The source/type of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobSource {
    /// A subagent spawned via delegate(async: true)
    Subagent,
    /// A long-running process (shell command, build, etc.)
    Process,
    /// An MCP resource subscription being watched
    McpResource,
    /// A long-running MCP tool call
    McpTool,
    /// An orchestrator-managed swarm agent
    SwarmAgent,
    /// A scheduled timer/reminder
    Timer,
}

/// Current state of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Actively running / in progress.
    Working,
    /// The job needs input from the user or parent agent to continue.
    InputRequired,
    /// Successfully completed.
    Completed,
    /// Errored or crashed.
    Failed,
    /// Explicitly canceled by user or agent.
    Canceled,
}

// Compat alias
impl JobState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
}

/// Which state transitions should trigger an interrupt (forced turn).
#[derive(Debug, Clone)]
pub struct InterruptPolicy {
    pub on_complete: bool,
    pub on_failed: bool,
    pub on_input_required: bool,
    pub on_pattern_matched: bool,
}

impl Default for InterruptPolicy {
    fn default() -> Self {
        Self {
            on_complete: true,
            on_failed: true,
            on_input_required: true,
            on_pattern_matched: true,
        }
    }
}

/// Metadata hints for job polling/retention behavior.
#[derive(Debug, Clone)]
pub struct JobMeta {
    /// Don't poll more often than this.
    pub min_poll_interval_ms: u64,
    /// After reaching a terminal state, data is retained for this long.
    pub ttl_after_completion_s: u64,
    /// Which transitions trigger a forced turn.
    pub interrupt_on: InterruptPolicy,
    /// For timer jobs: when the timer fires.
    pub deadline: Option<Instant>,
}

impl Default for JobMeta {
    fn default() -> Self {
        Self {
            min_poll_interval_ms: 1000,
            ttl_after_completion_s: 300,
            interrupt_on: InterruptPolicy::default(),
            deadline: None,
        }
    }
}

/// Policy for when a job's completion should trigger an assistant turn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NotifyPolicy {
    /// Only surface in MOIM. Never force a turn on its own.
    Informational,
    /// Force assistant turn when THIS specific job completes/fails.
    OnCompletion,
    /// Force assistant turn only when ALL jobs in the same batch are done.
    #[default]
    OnBatchCompletion,
}

/// A timestamped notification from a job.
#[derive(Debug, Clone)]
pub struct JobNotification {
    pub timestamp: Instant,
    pub message: String,
}

/// A single tracked job.
#[derive(Debug)]
pub struct Job {
    pub id: JobId,
    pub source: JobSource,
    pub description: String,
    pub state: JobState,
    pub batch_id: Option<BatchId>,
    pub notify_policy: NotifyPolicy,
    pub meta: JobMeta,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub notifications: Vec<JobNotification>,
    pub result_summary: Option<String>,
}

/// Event emitted when an actionable condition is met.
#[derive(Debug, Clone)]
pub enum JobEvent {
    /// A batch of jobs has fully completed.
    BatchCompleted {
        batch_id: BatchId,
        job_summaries: Vec<JobSummary>,
    },
    /// A single job completed with OnCompletion policy.
    JobCompleted { job: JobSummary },
    /// A job needs input to continue.
    JobNeedsInput { job_id: JobId, question: String },
    /// A pattern was matched in the job's output stream (job still working).
    PatternMatched {
        job_id: JobId,
        pattern: String,
        context: String,
    },
}

/// Summary of a completed job (for event payloads).
#[derive(Debug, Clone)]
pub struct JobSummary {
    pub id: JobId,
    pub source: JobSource,
    pub description: String,
    pub state: JobState,
    pub duration: Duration,
}

/// The Job Registry — tracks all async work for a session.
pub struct JobRegistry {
    jobs: HashMap<JobId, Job>,
    event_tx: mpsc::UnboundedSender<JobEvent>,
}

impl JobRegistry {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<JobEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        (
            Self {
                jobs: HashMap::new(),
                event_tx,
            },
            event_rx,
        )
    }

    pub fn register(&mut self, job: Job) {
        self.jobs.insert(job.id.clone(), job);
    }

    pub fn get(&self, id: &str) -> Option<&Job> {
        self.jobs.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Job> {
        self.jobs.get_mut(id)
    }

    /// Mark a job as completed.
    pub fn complete(&mut self, id: &str, summary: Option<String>) {
        self.transition(id, JobState::Completed, summary);
    }

    /// Mark a job as failed.
    pub fn fail(&mut self, id: &str, summary: Option<String>) {
        self.transition(id, JobState::Failed, summary);
    }

    /// Mark a job as canceled.
    pub fn cancel(&mut self, id: &str) {
        self.transition(id, JobState::Canceled, Some("canceled".into()));
    }

    /// Transition a job to InputRequired and emit event.
    pub fn needs_input(&mut self, id: &str, question: String) {
        if let Some(job) = self.jobs.get_mut(id) {
            job.state = JobState::InputRequired;
            job.last_activity = Instant::now();
            if job.meta.interrupt_on.on_input_required {
                let _ = self.event_tx.send(JobEvent::JobNeedsInput {
                    job_id: id.to_string(),
                    question,
                });
            }
        }
    }

    /// Resume a job from InputRequired back to Working.
    pub fn resume(&mut self, id: &str) {
        if let Some(job) = self.jobs.get_mut(id) {
            if job.state == JobState::InputRequired {
                job.state = JobState::Working;
                job.last_activity = Instant::now();
            }
        }
    }

    /// Emit a pattern-matched event (job stays Working).
    pub fn pattern_matched(&mut self, id: &str, pattern: String, context: String) {
        if let Some(job) = self.jobs.get_mut(id) {
            job.last_activity = Instant::now();
            job.notifications.push(JobNotification {
                timestamp: Instant::now(),
                message: format!("pattern matched: {}", pattern),
            });
            if job.meta.interrupt_on.on_pattern_matched {
                let _ = self.event_tx.send(JobEvent::PatternMatched {
                    job_id: id.to_string(),
                    pattern,
                    context,
                });
            }
        }
    }

    /// Add a notification to a job's log.
    pub fn notify(&mut self, id: &str, message: String) {
        if let Some(job) = self.jobs.get_mut(id) {
            job.last_activity = Instant::now();
            job.notifications.push(JobNotification {
                timestamp: Instant::now(),
                message,
            });
        }
    }

    pub fn list(&self) -> Vec<&Job> {
        self.jobs.values().collect()
    }

    pub fn running(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.state == JobState::Working)
            .collect()
    }

    pub fn finished(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.state.is_terminal())
            .collect()
    }

    pub fn remove(&mut self, id: &str) -> Option<Job> {
        self.jobs.remove(id)
    }

    /// Garbage collect jobs past their TTL.
    pub fn gc(&mut self) {
        let now = Instant::now();
        self.jobs.retain(|_, job| {
            if job.state.is_terminal() {
                let elapsed = now.duration_since(job.last_activity);
                elapsed.as_secs() < job.meta.ttl_after_completion_s
            } else {
                true
            }
        });
    }

    fn transition(&mut self, id: &str, new_state: JobState, summary: Option<String>) {
        let (notify_policy, batch_id, should_interrupt) = {
            let Some(job) = self.jobs.get_mut(id) else {
                return;
            };
            if job.state.is_terminal() {
                return;
            }
            job.state = new_state.clone();
            job.last_activity = Instant::now();
            job.result_summary = summary;
            let should_interrupt = match &new_state {
                JobState::Completed => job.meta.interrupt_on.on_complete,
                JobState::Failed => job.meta.interrupt_on.on_failed,
                _ => false,
            };
            (
                job.notify_policy.clone(),
                job.batch_id.clone(),
                should_interrupt,
            )
        };

        match notify_policy {
            NotifyPolicy::OnCompletion => {
                if should_interrupt {
                    if let Some(job) = self.jobs.get(id) {
                        let _ = self.event_tx.send(JobEvent::JobCompleted {
                            job: self.make_summary(job),
                        });
                    }
                }
            }
            NotifyPolicy::OnBatchCompletion => {
                if let Some(batch_id) = batch_id {
                    self.check_batch_completion(&batch_id);
                }
            }
            NotifyPolicy::Informational => {}
        }
    }

    fn check_batch_completion(&self, batch_id: &str) {
        let batch_jobs: Vec<&Job> = self
            .jobs
            .values()
            .filter(|j| j.batch_id.as_deref() == Some(batch_id))
            .collect();

        if batch_jobs.is_empty() {
            return;
        }

        let all_done = batch_jobs.iter().all(|j| j.state.is_terminal());

        if all_done {
            let summaries: Vec<JobSummary> =
                batch_jobs.iter().map(|j| self.make_summary(j)).collect();
            let _ = self.event_tx.send(JobEvent::BatchCompleted {
                batch_id: batch_id.to_string(),
                job_summaries: summaries,
            });
        }
    }

    fn make_summary(&self, job: &Job) -> JobSummary {
        JobSummary {
            id: job.id.clone(),
            source: job.source.clone(),
            description: job.description.clone(),
            state: job.state.clone(),
            duration: job.created_at.elapsed(),
        }
    }
}

/// Thread-safe handle to the job registry.
pub type SharedJobRegistry = Arc<Mutex<JobRegistry>>;

/// Create a new shared job registry and its event receiver.
pub fn create_job_registry() -> (SharedJobRegistry, mpsc::UnboundedReceiver<JobEvent>) {
    let (registry, event_rx) = JobRegistry::new();
    (Arc::new(Mutex::new(registry)), event_rx)
}
