use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use etcetera::{choose_app_strategy, AppStrategy};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{job::JobId, Job, JobScheduler as TokioJobScheduler};

use crate::agents::AgentEvent;
use crate::agents::{Agent, SessionConfig};
use crate::config::{self, Config};
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider as GooseProvider; // Alias to avoid conflict in test section
use crate::providers::create;
use crate::recipe::Recipe;
use crate::scheduler_trait::SchedulerTrait;
use crate::session;
use crate::session::storage::SessionMetadata;

// Track running tasks with their abort handles
type RunningTasksMap = HashMap<String, tokio::task::AbortHandle>;
type JobsMap = HashMap<String, (JobId, ScheduledJob)>;

/// Normalize a cron string so that:
/// 1. It is always in **quartz 7-field format** expected by Temporal
///    (seconds minutes hours dom month dow year).
/// 2. Five-field → prepend seconds `0` and append year `*`.
///    Six-field  → append year `*`.
/// 3. Everything else returned unchanged (with a warning).
pub fn normalize_cron_expression(src: &str) -> String {
    let mut parts: Vec<&str> = src.split_whitespace().collect();

    match parts.len() {
        5 => {
            // min hour dom mon dow  → 0 min hour dom mon dow *
            parts.insert(0, "0");
            parts.push("*");
        }
        6 => {
            // sec min hour dom mon dow  → sec min hour dom mon dow *
            parts.push("*");
        }
        7 => {
            // already quartz – do nothing
        }
        _ => {
            tracing::warn!(
                "Unrecognised cron expression '{}': expected 5, 6 or 7 fields (got {}). Leaving unchanged.",
                src,
                parts.len()
            );
            return src.to_string();
        }
    }

    parts.join(" ")
}

pub fn get_default_scheduler_storage_path() -> Result<PathBuf, io::Error> {
    let strategy = choose_app_strategy(config::APP_STRATEGY.clone())
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
    let data_dir = strategy.data_dir();
    fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("schedules.json"))
}

pub fn get_default_scheduled_recipes_dir() -> Result<PathBuf, SchedulerError> {
    let strategy = choose_app_strategy(config::APP_STRATEGY.clone()).map_err(|e| {
        SchedulerError::StorageError(io::Error::new(io::ErrorKind::NotFound, e.to_string()))
    })?;
    let data_dir = strategy.data_dir();
    let recipes_dir = data_dir.join("scheduled_recipes");
    fs::create_dir_all(&recipes_dir).map_err(SchedulerError::StorageError)?;
    tracing::debug!(
        "Created scheduled recipes directory at: {}",
        recipes_dir.display()
    );
    Ok(recipes_dir)
}

#[derive(Debug)]
pub enum SchedulerError {
    JobIdExists(String),
    JobNotFound(String),
    StorageError(io::Error),
    RecipeLoadError(String),
    AgentSetupError(String),
    PersistError(String),
    CronParseError(String),
    SchedulerInternalError(String),
    AnyhowError(anyhow::Error),
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::JobIdExists(id) => write!(f, "Job ID '{}' already exists.", id),
            SchedulerError::JobNotFound(id) => write!(f, "Job ID '{}' not found.", id),
            SchedulerError::StorageError(e) => write!(f, "Storage error: {}", e),
            SchedulerError::RecipeLoadError(e) => write!(f, "Recipe load error: {}", e),
            SchedulerError::AgentSetupError(e) => write!(f, "Agent setup error: {}", e),
            SchedulerError::PersistError(e) => write!(f, "Failed to persist schedules: {}", e),
            SchedulerError::CronParseError(e) => write!(f, "Invalid cron string: {}", e),
            SchedulerError::SchedulerInternalError(e) => {
                write!(f, "Scheduler internal error: {}", e)
            }
            SchedulerError::AnyhowError(e) => write!(f, "Scheduler operation failed: {}", e),
        }
    }
}

impl std::error::Error for SchedulerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SchedulerError::StorageError(e) => Some(e),
            SchedulerError::AnyhowError(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<io::Error> for SchedulerError {
    fn from(err: io::Error) -> Self {
        SchedulerError::StorageError(err)
    }
}

impl From<serde_json::Error> for SchedulerError {
    fn from(err: serde_json::Error) -> Self {
        SchedulerError::PersistError(err.to_string())
    }
}

impl From<anyhow::Error> for SchedulerError {
    fn from(err: anyhow::Error) -> Self {
        SchedulerError::AnyhowError(err)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, utoipa::ToSchema)]
pub struct ScheduledJob {
    pub id: String,
    pub source: String,
    pub cron: String,
    pub last_run: Option<DateTime<Utc>>,
    #[serde(default)]
    pub currently_running: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub current_session_id: Option<String>,
    #[serde(default)]
    pub process_start_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub execution_mode: Option<String>, // "foreground" or "background"
}

async fn persist_jobs_from_arc(
    storage_path: &Path,
    jobs_arc: &Arc<Mutex<JobsMap>>,
) -> Result<(), SchedulerError> {
    let jobs_guard = jobs_arc.lock().await;
    let list: Vec<ScheduledJob> = jobs_guard.values().map(|(_, j)| j.clone()).collect();
    if let Some(parent) = storage_path.parent() {
        fs::create_dir_all(parent).map_err(SchedulerError::StorageError)?;
    }
    let data = serde_json::to_string_pretty(&list).map_err(SchedulerError::from)?;
    fs::write(storage_path, data).map_err(SchedulerError::StorageError)?;
    Ok(())
}

pub struct Scheduler {
    internal_scheduler: TokioJobScheduler,
    jobs: Arc<Mutex<JobsMap>>,
    storage_path: PathBuf,
    running_tasks: Arc<Mutex<RunningTasksMap>>,
}

impl Scheduler {
    pub async fn new(storage_path: PathBuf) -> Result<Arc<Self>, SchedulerError> {
        let internal_scheduler = TokioJobScheduler::new()
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        let jobs = Arc::new(Mutex::new(HashMap::new()));
        let running_tasks = Arc::new(Mutex::new(HashMap::new()));

        let arc_self = Arc::new(Self {
            internal_scheduler,
            jobs,
            storage_path,
            running_tasks,
        });

        arc_self.load_jobs_from_storage().await?;
        arc_self
            .internal_scheduler
            .start()
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        Ok(arc_self)
    }

    pub async fn add_scheduled_job(
        &self,
        original_job_spec: ScheduledJob,
    ) -> Result<(), SchedulerError> {
        let mut jobs_guard = self.jobs.lock().await;
        if jobs_guard.contains_key(&original_job_spec.id) {
            return Err(SchedulerError::JobIdExists(original_job_spec.id.clone()));
        }

        let original_recipe_path = Path::new(&original_job_spec.source);
        if !original_recipe_path.exists() {
            return Err(SchedulerError::RecipeLoadError(format!(
                "Original recipe file not found: {}",
                original_job_spec.source
            )));
        }
        if !original_recipe_path.is_file() {
            return Err(SchedulerError::RecipeLoadError(format!(
                "Original recipe source is not a file: {}",
                original_job_spec.source
            )));
        }

        let scheduled_recipes_dir = get_default_scheduled_recipes_dir()?;
        let original_extension = original_recipe_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("yaml");

        let destination_filename = format!("{}.{}", original_job_spec.id, original_extension);
        let destination_recipe_path = scheduled_recipes_dir.join(destination_filename);

        tracing::info!(
            "Copying recipe from {} to {}",
            original_recipe_path.display(),
            destination_recipe_path.display()
        );
        fs::copy(original_recipe_path, &destination_recipe_path).map_err(|e| {
            SchedulerError::StorageError(io::Error::new(
                e.kind(),
                format!(
                    "Failed to copy recipe from {} to {}: {}",
                    original_job_spec.source,
                    destination_recipe_path.display(),
                    e
                ),
            ))
        })?;

        let mut stored_job = original_job_spec.clone();
        stored_job.source = destination_recipe_path.to_string_lossy().into_owned();
        stored_job.current_session_id = None;
        stored_job.process_start_time = None;
        tracing::info!("Updated job source path to: {}", stored_job.source);

        let job_for_task = stored_job.clone();
        let jobs_arc_for_task = self.jobs.clone();
        let storage_path_for_task = self.storage_path.clone();
        let running_tasks_for_task = self.running_tasks.clone();

        tracing::info!("Attempting to parse cron expression: '{}'", stored_job.cron);
        let normalized_cron = normalize_cron_expression(&stored_job.cron);
        // Convert from 7-field (Temporal format) to 6-field (tokio-cron-scheduler format)
        let tokio_cron = {
            let parts: Vec<&str> = normalized_cron.split_whitespace().collect();
            if parts.len() == 7 {
                parts[..6].join(" ")
            } else {
                normalized_cron.clone()
            }
        };
        if tokio_cron != stored_job.cron {
            tracing::info!(
                "Converted cron expression from '{}' to '{}' for tokio-cron-scheduler",
                stored_job.cron,
                tokio_cron
            );
        }
        let cron_task = Job::new_async(&tokio_cron, move |_uuid, _l| {
            let task_job_id = job_for_task.id.clone();
            let current_jobs_arc = jobs_arc_for_task.clone();
            let local_storage_path = storage_path_for_task.clone();
            let job_to_execute = job_for_task.clone(); // Clone for run_scheduled_job_internal
            let running_tasks_arc = running_tasks_for_task.clone();

            Box::pin(async move {
                // Check if the job is paused before executing
                let should_execute = {
                    let jobs_map_guard = current_jobs_arc.lock().await;
                    if let Some((_, current_job_in_map)) = jobs_map_guard.get(&task_job_id) {
                        !current_job_in_map.paused
                    } else {
                        false
                    }
                };

                if !should_execute {
                    tracing::info!("Skipping execution of paused job '{}'", &task_job_id);
                    return;
                }

                let current_time = Utc::now();
                let mut needs_persist = false;
                {
                    let mut jobs_map_guard = current_jobs_arc.lock().await;
                    if let Some((_, current_job_in_map)) = jobs_map_guard.get_mut(&task_job_id) {
                        current_job_in_map.last_run = Some(current_time);
                        current_job_in_map.currently_running = true;
                        current_job_in_map.process_start_time = Some(current_time);
                        needs_persist = true;
                    }
                }

                if needs_persist {
                    if let Err(e) =
                        persist_jobs_from_arc(&local_storage_path, &current_jobs_arc).await
                    {
                        tracing::error!(
                            "Failed to persist last_run update for job {}: {}",
                            &task_job_id,
                            e
                        );
                    }
                }

                // Spawn the job execution as an abortable task
                let job_task = tokio::spawn(run_scheduled_job_internal(
                    job_to_execute.clone(),
                    None,
                    Some(current_jobs_arc.clone()),
                    Some(task_job_id.clone()),
                ));

                // Store the abort handle at the scheduler level
                {
                    let mut running_tasks_guard = running_tasks_arc.lock().await;
                    running_tasks_guard.insert(task_job_id.clone(), job_task.abort_handle());
                }

                // Wait for the job to complete or be aborted
                let result = job_task.await;

                // Remove the abort handle
                {
                    let mut running_tasks_guard = running_tasks_arc.lock().await;
                    running_tasks_guard.remove(&task_job_id);
                }

                // Update the job status after execution
                {
                    let mut jobs_map_guard = current_jobs_arc.lock().await;
                    if let Some((_, current_job_in_map)) = jobs_map_guard.get_mut(&task_job_id) {
                        current_job_in_map.currently_running = false;
                        current_job_in_map.current_session_id = None;
                        current_job_in_map.process_start_time = None;
                        needs_persist = true;
                    }
                }

                if needs_persist {
                    if let Err(e) =
                        persist_jobs_from_arc(&local_storage_path, &current_jobs_arc).await
                    {
                        tracing::error!(
                            "Failed to persist running status update for job {}: {}",
                            &task_job_id,
                            e
                        );
                    }
                }

                match result {
                    Ok(Ok(_session_id)) => {
                        tracing::info!("Scheduled job '{}' completed successfully", &task_job_id);
                    }
                    Ok(Err(e)) => {
                        tracing::error!(
                            "Scheduled job '{}' execution failed: {}",
                            &e.job_id,
                            e.error
                        );
                    }
                    Err(join_error) if join_error.is_cancelled() => {
                        tracing::info!("Scheduled job '{}' was cancelled/killed", &task_job_id);
                    }
                    Err(join_error) => {
                        tracing::error!(
                            "Scheduled job '{}' task failed: {}",
                            &task_job_id,
                            join_error
                        );
                    }
                }
            })
        })
        .map_err(|e| SchedulerError::CronParseError(e.to_string()))?;

        let job_uuid = self
            .internal_scheduler
            .add(cron_task)
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        jobs_guard.insert(stored_job.id.clone(), (job_uuid, stored_job));
        // Pass the jobs_guard by reference for the initial persist after adding a job
        self.persist_jobs_to_storage_with_guard(&jobs_guard).await?;
        Ok(())
    }

    async fn load_jobs_from_storage(self: &Arc<Self>) -> Result<(), SchedulerError> {
        if !self.storage_path.exists() {
            return Ok(());
        }
        let data = fs::read_to_string(&self.storage_path)?;
        if data.trim().is_empty() {
            return Ok(());
        }

        let list: Vec<ScheduledJob> = serde_json::from_str(&data).map_err(|e| {
            SchedulerError::PersistError(format!("Failed to deserialize schedules.json: {}", e))
        })?;

        let mut jobs_guard = self.jobs.lock().await;
        for job_to_load in list {
            if !Path::new(&job_to_load.source).exists() {
                tracing::warn!("Recipe file {} for scheduled job {} not found in shared store. Skipping job load.", job_to_load.source, job_to_load.id);
                continue;
            }

            let job_for_task = job_to_load.clone();
            let jobs_arc_for_task = self.jobs.clone();
            let storage_path_for_task = self.storage_path.clone();
            let running_tasks_for_task = self.running_tasks.clone();

            tracing::info!(
                "Loading job '{}' with cron expression: '{}'",
                job_to_load.id,
                job_to_load.cron
            );
            let normalized_cron = normalize_cron_expression(&job_to_load.cron);
            // Convert from 7-field (Temporal format) to 6-field (tokio-cron-scheduler format)
            let tokio_cron = {
                let parts: Vec<&str> = normalized_cron.split_whitespace().collect();
                if parts.len() == 7 {
                    parts[..6].join(" ")
                } else {
                    normalized_cron.clone()
                }
            };
            if tokio_cron != job_to_load.cron {
                tracing::info!(
                    "Converted cron expression from '{}' to '{}' for tokio-cron-scheduler",
                    job_to_load.cron,
                    tokio_cron
                );
            }
            let cron_task = Job::new_async(&tokio_cron, move |_uuid, _l| {
                let task_job_id = job_for_task.id.clone();
                let current_jobs_arc = jobs_arc_for_task.clone();
                let local_storage_path = storage_path_for_task.clone();
                let job_to_execute = job_for_task.clone(); // Clone for run_scheduled_job_internal
                let running_tasks_arc = running_tasks_for_task.clone();

                Box::pin(async move {
                    // Check if the job is paused before executing
                    let should_execute = {
                        let jobs_map_guard = current_jobs_arc.lock().await;
                        if let Some((_, stored_job)) = jobs_map_guard.get(&task_job_id) {
                            !stored_job.paused
                        } else {
                            false
                        }
                    };

                    if !should_execute {
                        tracing::info!("Skipping execution of paused job '{}'", &task_job_id);
                        return;
                    }

                    let current_time = Utc::now();
                    let mut needs_persist = false;
                    {
                        let mut jobs_map_guard = current_jobs_arc.lock().await;
                        if let Some((_, stored_job)) = jobs_map_guard.get_mut(&task_job_id) {
                            stored_job.last_run = Some(current_time);
                            stored_job.currently_running = true;
                            stored_job.process_start_time = Some(current_time);
                            needs_persist = true;
                        }
                    }

                    if needs_persist {
                        if let Err(e) =
                            persist_jobs_from_arc(&local_storage_path, &current_jobs_arc).await
                        {
                            tracing::error!(
                                "Failed to persist last_run update for loaded job {}: {}",
                                &task_job_id,
                                e
                            );
                        }
                    }

                    // Spawn the job execution as an abortable task
                    let job_task = tokio::spawn(run_scheduled_job_internal(
                        job_to_execute,
                        None,
                        Some(current_jobs_arc.clone()),
                        Some(task_job_id.clone()),
                    ));

                    // Store the abort handle at the scheduler level
                    {
                        let mut running_tasks_guard = running_tasks_arc.lock().await;
                        running_tasks_guard.insert(task_job_id.clone(), job_task.abort_handle());
                    }

                    // Wait for the job to complete or be aborted
                    let result = job_task.await;

                    // Remove the abort handle
                    {
                        let mut running_tasks_guard = running_tasks_arc.lock().await;
                        running_tasks_guard.remove(&task_job_id);
                    }

                    // Update the job status after execution
                    {
                        let mut jobs_map_guard = current_jobs_arc.lock().await;
                        if let Some((_, stored_job)) = jobs_map_guard.get_mut(&task_job_id) {
                            stored_job.currently_running = false;
                            stored_job.current_session_id = None;
                            stored_job.process_start_time = None;
                            needs_persist = true;
                        }
                    }

                    if needs_persist {
                        if let Err(e) =
                            persist_jobs_from_arc(&local_storage_path, &current_jobs_arc).await
                        {
                            tracing::error!(
                                "Failed to persist running status update for job {}: {}",
                                &task_job_id,
                                e
                            );
                        }
                    }

                    match result {
                        Ok(Ok(_session_id)) => {
                            tracing::info!(
                                "Scheduled job '{}' completed successfully",
                                &task_job_id
                            );
                        }
                        Ok(Err(e)) => {
                            tracing::error!(
                                "Scheduled job '{}' execution failed: {}",
                                &e.job_id,
                                e.error
                            );
                        }
                        Err(join_error) if join_error.is_cancelled() => {
                            tracing::info!("Scheduled job '{}' was cancelled/killed", &task_job_id);
                        }
                        Err(join_error) => {
                            tracing::error!(
                                "Scheduled job '{}' task failed: {}",
                                &task_job_id,
                                join_error
                            );
                        }
                    }
                })
            })
            .map_err(|e| SchedulerError::CronParseError(e.to_string()))?;

            let job_uuid = self
                .internal_scheduler
                .add(cron_task)
                .await
                .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;
            jobs_guard.insert(job_to_load.id.clone(), (job_uuid, job_to_load));
        }
        Ok(())
    }

    // Renamed and kept for direct use when a guard is already held (e.g. add/remove)
    async fn persist_jobs_to_storage_with_guard(
        &self,
        jobs_guard: &tokio::sync::MutexGuard<'_, JobsMap>,
    ) -> Result<(), SchedulerError> {
        let list: Vec<ScheduledJob> = jobs_guard.values().map(|(_, j)| j.clone()).collect();
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&list)?;
        fs::write(&self.storage_path, data)?;
        Ok(())
    }

    // New function that locks and calls the helper, for run_now and potentially other places
    async fn persist_jobs(&self) -> Result<(), SchedulerError> {
        persist_jobs_from_arc(&self.storage_path, &self.jobs).await
    }

    pub async fn list_scheduled_jobs(&self) -> Vec<ScheduledJob> {
        self.jobs
            .lock()
            .await
            .values()
            .map(|(_, j)| j.clone())
            .collect()
    }

    pub async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
        let mut jobs_guard = self.jobs.lock().await;
        if let Some((job_uuid, scheduled_job)) = jobs_guard.remove(id) {
            self.internal_scheduler
                .remove(&job_uuid)
                .await
                .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

            let recipe_path = Path::new(&scheduled_job.source);
            if recipe_path.exists() {
                fs::remove_file(recipe_path).map_err(SchedulerError::StorageError)?;
            }

            self.persist_jobs_to_storage_with_guard(&jobs_guard).await?;
            Ok(())
        } else {
            Err(SchedulerError::JobNotFound(id.to_string()))
        }
    }

    pub async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
        // Changed return type
        let all_session_files = session::storage::list_sessions()
            .map_err(|e| SchedulerError::StorageError(io::Error::other(e)))?;

        let mut schedule_sessions: Vec<(String, SessionMetadata)> = Vec::new();

        for (session_name, session_path) in all_session_files {
            match session::storage::read_metadata(&session_path) {
                Ok(metadata) => {
                    // metadata is not mutable here, and SessionMetadata is original
                    if metadata.schedule_id.as_deref() == Some(sched_id) {
                        schedule_sessions.push((session_name, metadata)); // Keep the tuple
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to read metadata for session file {}: {}. Skipping.",
                        session_path.display(),
                        e
                    );
                }
            }
        }

        schedule_sessions.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by session_name (timestamp string)

        // Keep the tuple, just take the limit
        let result_sessions: Vec<(String, SessionMetadata)> =
            schedule_sessions.into_iter().take(limit).collect();

        Ok(result_sessions) // Return the Vec of tuples
    }

    pub async fn run_now(&self, sched_id: &str) -> Result<String, SchedulerError> {
        let job_to_run: ScheduledJob = {
            let mut jobs_guard = self.jobs.lock().await;
            match jobs_guard.get_mut(sched_id) {
                Some((_, job_def)) => {
                    // Set the currently_running flag before executing
                    job_def.currently_running = true;
                    let job_clone = job_def.clone();
                    // Drop the guard before persisting to avoid borrow issues
                    drop(jobs_guard);

                    // Persist the change immediately
                    self.persist_jobs().await?;
                    job_clone
                }
                None => return Err(SchedulerError::JobNotFound(sched_id.to_string())),
            }
        };

        // Spawn the job execution as an abortable task for run_now
        let job_task = tokio::spawn(run_scheduled_job_internal(
            job_to_run.clone(),
            None,
            Some(self.jobs.clone()),
            Some(sched_id.to_string()),
        ));

        // Store the abort handle for run_now jobs
        {
            let mut running_tasks_guard = self.running_tasks.lock().await;
            running_tasks_guard.insert(sched_id.to_string(), job_task.abort_handle());
        }

        // Wait for the job to complete or be aborted
        let run_result = job_task.await;

        // Remove the abort handle
        {
            let mut running_tasks_guard = self.running_tasks.lock().await;
            running_tasks_guard.remove(sched_id);
        }

        // Clear the currently_running flag after execution
        {
            let mut jobs_guard = self.jobs.lock().await;
            if let Some((_tokio_job_id, job_in_map)) = jobs_guard.get_mut(sched_id) {
                job_in_map.currently_running = false;
                job_in_map.current_session_id = None;
                job_in_map.process_start_time = None;
                job_in_map.last_run = Some(Utc::now());
            } // MutexGuard is dropped here
        }

        // Persist after the lock is released and update is made.
        self.persist_jobs().await?;

        match run_result {
            Ok(Ok(session_id)) => Ok(session_id),
            Ok(Err(e)) => Err(SchedulerError::AnyhowError(anyhow!(
                "Failed to execute job '{}' immediately: {}",
                sched_id,
                e.error
            ))),
            Err(join_error) if join_error.is_cancelled() => {
                tracing::info!("Run now job '{}' was cancelled/killed", sched_id);
                Err(SchedulerError::AnyhowError(anyhow!(
                    "Job '{}' was successfully cancelled",
                    sched_id
                )))
            }
            Err(join_error) => Err(SchedulerError::AnyhowError(anyhow!(
                "Failed to execute job '{}' immediately: {}",
                sched_id,
                join_error
            ))),
        }
    }

    pub async fn pause_schedule(&self, sched_id: &str) -> Result<(), SchedulerError> {
        let mut jobs_guard = self.jobs.lock().await;
        match jobs_guard.get_mut(sched_id) {
            Some((_, job_def)) => {
                if job_def.currently_running {
                    return Err(SchedulerError::AnyhowError(anyhow!(
                        "Cannot pause schedule '{}' while it's currently running",
                        sched_id
                    )));
                }
                job_def.paused = true;
                self.persist_jobs_to_storage_with_guard(&jobs_guard).await?;
                Ok(())
            }
            None => Err(SchedulerError::JobNotFound(sched_id.to_string())),
        }
    }

    pub async fn unpause_schedule(&self, sched_id: &str) -> Result<(), SchedulerError> {
        let mut jobs_guard = self.jobs.lock().await;
        match jobs_guard.get_mut(sched_id) {
            Some((_, job_def)) => {
                job_def.paused = false;
                self.persist_jobs_to_storage_with_guard(&jobs_guard).await?;
                Ok(())
            }
            None => Err(SchedulerError::JobNotFound(sched_id.to_string())),
        }
    }

    pub async fn update_schedule(
        &self,
        sched_id: &str,
        new_cron: String,
    ) -> Result<(), SchedulerError> {
        let mut jobs_guard = self.jobs.lock().await;
        match jobs_guard.get_mut(sched_id) {
            Some((job_uuid, job_def)) => {
                if job_def.currently_running {
                    return Err(SchedulerError::AnyhowError(anyhow!(
                        "Cannot edit schedule '{}' while it's currently running",
                        sched_id
                    )));
                }

                if new_cron == job_def.cron {
                    // No change needed
                    return Ok(());
                }

                // Remove the old job from the scheduler
                self.internal_scheduler
                    .remove(job_uuid)
                    .await
                    .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

                // Create new job with updated cron
                let job_for_task = job_def.clone();
                let jobs_arc_for_task = self.jobs.clone();
                let storage_path_for_task = self.storage_path.clone();
                let running_tasks_for_task = self.running_tasks.clone();

                tracing::info!(
                    "Updating job '{}' with new cron expression: '{}'",
                    sched_id,
                    new_cron
                );
                let normalized_cron = normalize_cron_expression(&new_cron);
                // Convert from 7-field (Temporal format) to 6-field (tokio-cron-scheduler format)
                let tokio_cron = {
                    let parts: Vec<&str> = normalized_cron.split_whitespace().collect();
                    if parts.len() == 7 {
                        parts[..6].join(" ")
                    } else {
                        normalized_cron.clone()
                    }
                };
                if tokio_cron != new_cron {
                    tracing::info!(
                        "Converted cron expression from '{}' to '{}' for tokio-cron-scheduler",
                        new_cron,
                        tokio_cron
                    );
                }
                let cron_task = Job::new_async(&tokio_cron, move |_uuid, _l| {
                    let task_job_id = job_for_task.id.clone();
                    let current_jobs_arc = jobs_arc_for_task.clone();
                    let local_storage_path = storage_path_for_task.clone();
                    let job_to_execute = job_for_task.clone();
                    let running_tasks_arc = running_tasks_for_task.clone();

                    Box::pin(async move {
                        // Check if the job is paused before executing
                        let should_execute = {
                            let jobs_map_guard = current_jobs_arc.lock().await;
                            if let Some((_, current_job_in_map)) = jobs_map_guard.get(&task_job_id)
                            {
                                !current_job_in_map.paused
                            } else {
                                false
                            }
                        };

                        if !should_execute {
                            tracing::info!("Skipping execution of paused job '{}'", &task_job_id);
                            return;
                        }

                        let current_time = Utc::now();
                        let mut needs_persist = false;
                        {
                            let mut jobs_map_guard = current_jobs_arc.lock().await;
                            if let Some((_, current_job_in_map)) =
                                jobs_map_guard.get_mut(&task_job_id)
                            {
                                current_job_in_map.last_run = Some(current_time);
                                current_job_in_map.currently_running = true;
                                current_job_in_map.process_start_time = Some(current_time);
                                needs_persist = true;
                            }
                        }

                        if needs_persist {
                            if let Err(e) =
                                persist_jobs_from_arc(&local_storage_path, &current_jobs_arc).await
                            {
                                tracing::error!(
                                    "Failed to persist last_run update for job {}: {}",
                                    &task_job_id,
                                    e
                                );
                            }
                        }

                        // Spawn the job execution as an abortable task
                        let job_task = tokio::spawn(run_scheduled_job_internal(
                            job_to_execute,
                            None,
                            Some(current_jobs_arc.clone()),
                            Some(task_job_id.clone()),
                        ));

                        // Store the abort handle at the scheduler level
                        {
                            let mut running_tasks_guard = running_tasks_arc.lock().await;
                            running_tasks_guard
                                .insert(task_job_id.clone(), job_task.abort_handle());
                        }

                        // Wait for the job to complete or be aborted
                        let result = job_task.await;

                        // Remove the abort handle
                        {
                            let mut running_tasks_guard = running_tasks_arc.lock().await;
                            running_tasks_guard.remove(&task_job_id);
                        }

                        // Update the job status after execution
                        {
                            let mut jobs_map_guard = current_jobs_arc.lock().await;
                            if let Some((_, current_job_in_map)) =
                                jobs_map_guard.get_mut(&task_job_id)
                            {
                                current_job_in_map.currently_running = false;
                                current_job_in_map.current_session_id = None;
                                current_job_in_map.process_start_time = None;
                                needs_persist = true;
                            }
                        }

                        if needs_persist {
                            if let Err(e) =
                                persist_jobs_from_arc(&local_storage_path, &current_jobs_arc).await
                            {
                                tracing::error!(
                                    "Failed to persist running status update for job {}: {}",
                                    &task_job_id,
                                    e
                                );
                            }
                        }

                        match result {
                            Ok(Ok(_session_id)) => {
                                tracing::info!(
                                    "Scheduled job '{}' completed successfully",
                                    &task_job_id
                                );
                            }
                            Ok(Err(e)) => {
                                tracing::error!(
                                    "Scheduled job '{}' execution failed: {}",
                                    &e.job_id,
                                    e.error
                                );
                            }
                            Err(join_error) if join_error.is_cancelled() => {
                                tracing::info!(
                                    "Scheduled job '{}' was cancelled/killed",
                                    &task_job_id
                                );
                            }
                            Err(join_error) => {
                                tracing::error!(
                                    "Scheduled job '{}' task failed: {}",
                                    &task_job_id,
                                    join_error
                                );
                            }
                        }
                    })
                })
                .map_err(|e| SchedulerError::CronParseError(e.to_string()))?;

                let new_job_uuid = self
                    .internal_scheduler
                    .add(cron_task)
                    .await
                    .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

                // Update the job UUID and cron expression
                *job_uuid = new_job_uuid;
                job_def.cron = new_cron;

                self.persist_jobs_to_storage_with_guard(&jobs_guard).await?;
                Ok(())
            }
            None => Err(SchedulerError::JobNotFound(sched_id.to_string())),
        }
    }

    pub async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError> {
        let mut jobs_guard = self.jobs.lock().await;
        match jobs_guard.get_mut(sched_id) {
            Some((_, job_def)) => {
                if !job_def.currently_running {
                    return Err(SchedulerError::AnyhowError(anyhow!(
                        "Schedule '{}' is not currently running",
                        sched_id
                    )));
                }

                tracing::info!("Killing running job '{}'", sched_id);

                // Abort the running task if it exists
                {
                    let mut running_tasks_guard = self.running_tasks.lock().await;
                    if let Some(abort_handle) = running_tasks_guard.remove(sched_id) {
                        abort_handle.abort();
                        tracing::info!("Aborted running task for job '{}'", sched_id);
                    } else {
                        tracing::warn!(
                            "No abort handle found for job '{}' in running tasks map",
                            sched_id
                        );
                    }
                }

                // Mark the job as no longer running
                job_def.currently_running = false;
                job_def.current_session_id = None;
                job_def.process_start_time = None;

                self.persist_jobs_to_storage_with_guard(&jobs_guard).await?;

                tracing::info!("Successfully killed job '{}'", sched_id);
                Ok(())
            }
            None => Err(SchedulerError::JobNotFound(sched_id.to_string())),
        }
    }

    pub async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        let jobs_guard = self.jobs.lock().await;
        match jobs_guard.get(sched_id) {
            Some((_, job_def)) => {
                if job_def.currently_running {
                    if let (Some(session_id), Some(start_time)) =
                        (&job_def.current_session_id, &job_def.process_start_time)
                    {
                        Ok(Some((session_id.clone(), *start_time)))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            None => Err(SchedulerError::JobNotFound(sched_id.to_string())),
        }
    }
}

#[derive(Debug)]
struct JobExecutionError {
    job_id: String,
    error: String,
}

async fn run_scheduled_job_internal(
    job: ScheduledJob,
    provider_override: Option<Arc<dyn GooseProvider>>, // New optional parameter
    jobs_arc: Option<Arc<Mutex<JobsMap>>>,
    job_id: Option<String>,
) -> std::result::Result<String, JobExecutionError> {
    tracing::info!("Executing job: {} (Source: {})", job.id, job.source);

    let recipe_path = Path::new(&job.source);

    let recipe_content = match fs::read_to_string(recipe_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(JobExecutionError {
                job_id: job.id.clone(),
                error: format!("Failed to load recipe file '{}': {}", job.source, e),
            });
        }
    };

    let recipe: Recipe = {
        let extension = recipe_path
            .extension()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("yaml")
            .to_lowercase();

        match extension.as_str() {
            "json" | "jsonl" => {
                serde_json::from_str::<Recipe>(&recipe_content).map_err(|e| JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to parse JSON recipe '{}': {}", job.source, e),
                })
            }
            "yaml" | "yml" => {
                serde_yaml::from_str::<Recipe>(&recipe_content).map_err(|e| JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to parse YAML recipe '{}': {}", job.source, e),
                })
            }
            _ => Err(JobExecutionError {
                job_id: job.id.clone(),
                error: format!(
                    "Unsupported recipe file extension '{}' for: {}",
                    extension, job.source
                ),
            }),
        }
    }?;

    let agent: Agent = Agent::new();

    let agent_provider: Arc<dyn GooseProvider>; // Use the aliased GooseProvider

    if let Some(provider) = provider_override {
        agent_provider = provider;
    } else {
        let global_config = Config::global();
        let provider_name: String = match global_config.get_param("GOOSE_PROVIDER") {
            Ok(name) => name,
            Err(_) => return Err(JobExecutionError {
                job_id: job.id.clone(),
                error:
                    "GOOSE_PROVIDER not configured globally. Run 'goose configure' or set env var."
                        .to_string(),
            }),
        };
        let model_name: String =
            match global_config.get_param("GOOSE_MODEL") {
                Ok(name) => name,
                Err(_) => return Err(JobExecutionError {
                    job_id: job.id.clone(),
                    error:
                        "GOOSE_MODEL not configured globally. Run 'goose configure' or set env var."
                            .to_string(),
                }),
            };
        let model_config =
            crate::model::ModelConfig::new(model_name.as_str()).map_err(|e| JobExecutionError {
                job_id: job.id.clone(),
                error: format!("Model config error: {}", e),
            })?;

        agent_provider = create(&provider_name, model_config).map_err(|e| JobExecutionError {
            job_id: job.id.clone(),
            error: format!(
                "Failed to create provider instance '{}': {}",
                provider_name, e
            ),
        })?;
    }
    if let Some(recipe_extensions) = recipe.extensions {
        for extension in recipe_extensions {
            agent
                .add_extension(extension.clone())
                .await
                .map_err(|e| JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to add extension '{}': {}", extension.name(), e),
                })?;
        }
    }

    if let Err(e) = agent.update_provider(agent_provider).await {
        return Err(JobExecutionError {
            job_id: job.id.clone(),
            error: format!("Failed to set provider on agent: {}", e),
        });
    }
    tracing::info!("Agent configured with provider for job '{}'", job.id);

    // Log the execution mode
    let execution_mode = job.execution_mode.as_deref().unwrap_or("background");
    tracing::info!("Job '{}' running in {} mode", job.id, execution_mode);

    let session_id_for_return = session::generate_session_id();

    // Update the job with the session ID if we have access to the jobs arc
    if let (Some(jobs_arc), Some(job_id_str)) = (jobs_arc.as_ref(), job_id.as_ref()) {
        let mut jobs_guard = jobs_arc.lock().await;
        if let Some((_, job_def)) = jobs_guard.get_mut(job_id_str) {
            job_def.current_session_id = Some(session_id_for_return.clone());
        }
    }

    let session_file_path = match crate::session::storage::get_path(
        crate::session::storage::Identifier::Name(session_id_for_return.clone()),
    ) {
        Ok(path) => path,
        Err(e) => {
            return Err(JobExecutionError {
                job_id: job.id.clone(),
                error: format!("Failed to get session file path: {}", e),
            });
        }
    };

    if let Some(prompt_text) = recipe.prompt {
        let mut all_session_messages =
            Conversation::new_unvalidated(vec![Message::user().with_text(prompt_text.clone())]);

        let current_dir = match std::env::current_dir() {
            Ok(cd) => cd,
            Err(e) => {
                return Err(JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to get current directory for job execution: {}", e),
                });
            }
        };

        let session_config = SessionConfig {
            id: crate::session::storage::Identifier::Name(session_id_for_return.clone()),
            working_dir: current_dir.clone(),
            schedule_id: Some(job.id.clone()),
            execution_mode: job.execution_mode.clone(),
            max_turns: None,
            retry_config: None,
        };

        match agent
            .reply(
                all_session_messages.clone(),
                Some(session_config.clone()),
                None,
            )
            .await
        {
            Ok(mut stream) => {
                use futures::StreamExt;

                while let Some(message_result) = stream.next().await {
                    // Check if the task has been cancelled
                    tokio::task::yield_now().await;

                    match message_result {
                        Ok(AgentEvent::Message(msg)) => {
                            if msg.role == rmcp::model::Role::Assistant {
                                tracing::info!("[Job {}] Assistant: {:?}", job.id, msg.content);
                            }
                            all_session_messages.push(msg);
                        }
                        Ok(AgentEvent::McpNotification(_)) => {
                            // Handle notifications if needed
                        }
                        Ok(AgentEvent::ModelChange { .. }) => {
                            // Model change events are informational, just continue
                        }
                        Ok(AgentEvent::HistoryReplaced(_)) => {
                            // Handle history replacement events if needed
                        }
                        Err(e) => {
                            tracing::error!(
                                "[Job {}] Error receiving message from agent: {}",
                                job.id,
                                e
                            );
                            break;
                        }
                    }
                }

                match crate::session::storage::read_metadata(&session_file_path) {
                    Ok(mut updated_metadata) => {
                        updated_metadata.message_count = all_session_messages.len();
                        if let Err(e) = crate::session::storage::save_messages_with_metadata(
                            &session_file_path,
                            &updated_metadata,
                            &all_session_messages,
                        ) {
                            tracing::error!(
                                "[Job {}] Failed to persist final messages: {}",
                                job.id,
                                e
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "[Job {}] Failed to read updated metadata before final save: {}",
                            job.id,
                            e
                        );
                        let fallback_metadata = crate::session::storage::SessionMetadata {
                            working_dir: current_dir.clone(),
                            description: String::new(),
                            schedule_id: Some(job.id.clone()),
                            project_id: None,
                            message_count: all_session_messages.len(),
                            total_tokens: None,
                            input_tokens: None,
                            output_tokens: None,
                            accumulated_total_tokens: None,
                            accumulated_input_tokens: None,
                            accumulated_output_tokens: None,
                        };
                        if let Err(e_fb) = crate::session::storage::save_messages_with_metadata(
                            &session_file_path,
                            &fallback_metadata,
                            &all_session_messages,
                        ) {
                            tracing::error!("[Job {}] Failed to persist final messages with fallback metadata: {}", job.id, e_fb);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Agent failed to reply for recipe '{}': {}", job.source, e),
                });
            }
        }
    } else {
        tracing::warn!(
            "[Job {}] Recipe '{}' has no prompt to execute.",
            job.id,
            job.source
        );
        let metadata = crate::session::storage::SessionMetadata {
            working_dir: std::env::current_dir().unwrap_or_default(),
            description: "Empty job - no prompt".to_string(),
            schedule_id: Some(job.id.clone()),
            message_count: 0,
            ..Default::default()
        };
        if let Err(e) = crate::session::storage::save_messages_with_metadata(
            &session_file_path,
            &metadata,
            &Conversation::new_unvalidated(vec![]),
        ) {
            tracing::error!(
                "[Job {}] Failed to persist metadata for empty job: {}",
                job.id,
                e
            );
        }
    }

    tracing::info!("Finished job: {}", job.id);
    Ok(session_id_for_return)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Recipe;
    use crate::{
        model::ModelConfig, // Use the actual ModelConfig for the mock's field
        providers::base::{ProviderMetadata, ProviderUsage, Usage},
        providers::errors::ProviderError,
    };
    use rmcp::model::Tool;
    use rmcp::model::{AnnotateAble, RawTextContent, Role};
    // Removed: use crate::session::storage::{get_most_recent_session, read_metadata};
    // `read_metadata` is still used by the test itself, so keep it or its module.
    use crate::session::storage::read_metadata;

    use crate::conversation::message::{Message, MessageContent};
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[derive(Clone)]
    struct MockSchedulerTestProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl GooseProvider for MockSchedulerTestProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::new(
                "mock-scheduler-test",
                "Mock for Scheduler Test",
                "A mock provider for scheduler tests", // description
                "test-model",                          // default_model
                vec!["test-model"],                    // model_names
                "",     // model_doc_link (empty string if not applicable)
                vec![], // config_keys (empty vec if none)
            )
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::new(
                    Role::Assistant,
                    Utc::now().timestamp(),
                    vec![MessageContent::Text(
                        RawTextContent {
                            text: "Mocked scheduled response".to_string(),
                        }
                        .no_annotation(),
                    )],
                ),
                ProviderUsage::new("mock-scheduler-test".to_string(), Usage::default()),
            ))
        }
    }

    // This function is pub(super) making it visible to run_scheduled_job_internal (parent module)
    // when cfg(test) is active for the whole compilation unit.
    pub(super) fn create_scheduler_test_mock_provider(
        model_config: ModelConfig,
    ) -> Arc<dyn GooseProvider> {
        Arc::new(MockSchedulerTestProvider { model_config })
    }

    #[tokio::test]
    async fn test_scheduled_session_has_schedule_id() -> Result<(), Box<dyn std::error::Error>> {
        // Set environment variables for the test
        env::set_var("GOOSE_PROVIDER", "test_provider");
        env::set_var("GOOSE_MODEL", "test_model");

        let temp_dir = tempdir()?;
        let recipe_dir = temp_dir.path().join("recipes_for_test_scheduler");
        fs::create_dir_all(&recipe_dir)?;

        let _ = session::storage::ensure_session_dir().expect("Failed to ensure app session dir");

        let schedule_id_str = "test_schedule_001_scheduler_check".to_string();
        let recipe_filename = recipe_dir.join(format!("{}.json", schedule_id_str));

        let dummy_recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Test Schedule ID Recipe".to_string(),
            description: "A recipe for testing schedule_id propagation.".to_string(),
            instructions: None,
            prompt: Some("This is a test prompt for a scheduled job.".to_string()),
            extensions: None,
            context: None,
            activities: None,
            author: None,
            parameters: None,
            settings: None,
            response: None,
            sub_recipes: None,
            retry: None,
        };
        let mut recipe_file = File::create(&recipe_filename)?;
        writeln!(
            recipe_file,
            "{}",
            serde_json::to_string_pretty(&dummy_recipe)?
        )?;
        recipe_file.flush()?;
        drop(recipe_file);

        let dummy_job = ScheduledJob {
            id: schedule_id_str.clone(),
            source: recipe_filename.to_string_lossy().into_owned(),
            cron: "* * * * * * ".to_string(), // Runs every second for quick testing
            last_run: None,
            currently_running: false,
            paused: false,
            current_session_id: None,
            process_start_time: None,
            execution_mode: Some("background".to_string()), // Default for test
        };

        let mock_model_config = ModelConfig::new_or_fail("test_model");
        let mock_provider_instance = create_scheduler_test_mock_provider(mock_model_config);

        // Call run_scheduled_job_internal, passing the mock provider
        let created_session_id =
            run_scheduled_job_internal(dummy_job.clone(), Some(mock_provider_instance), None, None)
                .await
                .expect("run_scheduled_job_internal failed");

        let session_dir = session::storage::ensure_session_dir()?;
        let expected_session_path = session_dir.join(format!("{}.jsonl", created_session_id));

        assert!(
            expected_session_path.exists(),
            "Expected session file {} was not created",
            expected_session_path.display()
        );

        let metadata = read_metadata(&expected_session_path)?;

        assert_eq!(
            metadata.schedule_id,
            Some(schedule_id_str.clone()),
            "Session metadata schedule_id ({:?}) does not match the job ID ({}). File: {}",
            metadata.schedule_id,
            schedule_id_str,
            expected_session_path.display()
        );

        // Check if messages were written
        let messages_in_file = crate::session::storage::read_messages(&expected_session_path)?;
        assert!(
            !messages_in_file.is_empty(),
            "No messages were written to the session file: {}",
            expected_session_path.display()
        );
        // We expect at least a user prompt and an assistant response
        assert!(
            messages_in_file.len() >= 2,
            "Expected at least 2 messages (prompt + response), found {} in file: {}",
            messages_in_file.len(),
            expected_session_path.display()
        );

        // Clean up environment variables
        env::remove_var("GOOSE_PROVIDER");
        env::remove_var("GOOSE_MODEL");

        Ok(())
    }
}

#[async_trait]
impl SchedulerTrait for Scheduler {
    async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
        self.add_scheduled_job(job).await
    }

    async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        Ok(self.list_scheduled_jobs().await)
    }

    async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
        self.remove_scheduled_job(id).await
    }

    async fn pause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.pause_schedule(id).await
    }

    async fn unpause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.unpause_schedule(id).await
    }

    async fn run_now(&self, id: &str) -> Result<String, SchedulerError> {
        self.run_now(id).await
    }

    async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
        self.sessions(sched_id, limit).await
    }

    async fn update_schedule(
        &self,
        sched_id: &str,
        new_cron: String,
    ) -> Result<(), SchedulerError> {
        self.update_schedule(sched_id, new_cron).await
    }

    async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError> {
        self.kill_running_job(sched_id).await
    }

    async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        self.get_running_job_info(sched_id).await
    }
}
