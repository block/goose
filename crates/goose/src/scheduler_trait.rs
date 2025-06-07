use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::scheduler::{ScheduledJob, SchedulerError};
use crate::session::storage::SessionMetadata;

/// Common trait for all scheduler implementations
#[async_trait]
pub trait SchedulerTrait: Send + Sync {
    /// Add a new scheduled job
    async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError>;

    /// List all scheduled jobs
    async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError>;

    /// Remove a scheduled job by ID
    async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError>;

    /// Pause a scheduled job
    async fn pause_schedule(&self, id: &str) -> Result<(), SchedulerError>;

    /// Unpause a scheduled job
    async fn unpause_schedule(&self, id: &str) -> Result<(), SchedulerError>;

    /// Run a job immediately
    async fn run_now(&self, id: &str) -> Result<String, SchedulerError>;

    /// Get sessions for a scheduled job
    async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError>;

    /// Update a schedule's cron expression
    async fn update_schedule(&self, sched_id: &str, new_cron: String)
        -> Result<(), SchedulerError>;

    /// Kill a running job
    async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError>;

    /// Get information about a running job
    async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError>;
}
