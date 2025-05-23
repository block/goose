use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use etcetera::{choose_app_strategy, AppStrategy};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{job::JobId, Job, JobScheduler as TokioJobScheduler};

use crate::agents::{Agent, SessionConfig};
use crate::config::{self, Config};
use crate::message::Message;
use crate::providers::create;
use crate::recipe::Recipe;
use crate::session;
use crate::session::storage::SessionMetadata;

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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ScheduledJob {
    pub id: String,
    pub source: String,
    pub cron: String,
    pub last_run: Option<DateTime<Utc>>,
}

async fn persist_jobs_from_arc(
    storage_path: &Path,
    jobs_arc: &Arc<Mutex<HashMap<String, (JobId, ScheduledJob)>>>,
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
    jobs: Arc<Mutex<HashMap<String, (JobId, ScheduledJob)>>>,
    storage_path: PathBuf,
}

impl Scheduler {
    pub async fn new(storage_path: PathBuf) -> Result<Arc<Self>, SchedulerError> {
        let internal_scheduler = TokioJobScheduler::new()
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        let jobs = Arc::new(Mutex::new(HashMap::new()));

        let arc_self = Arc::new(Self {
            internal_scheduler,
            jobs,
            storage_path,
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
        tracing::info!("Updated job source path to: {}", stored_job.source);

        let job_for_task = stored_job.clone();
        let jobs_arc_for_task = self.jobs.clone();
        let storage_path_for_task = self.storage_path.clone();

        let cron_task = Job::new_async(&stored_job.cron, move |_uuid, _l| {
            let task_job_id = job_for_task.id.clone();
            let current_jobs_arc = jobs_arc_for_task.clone();
            let local_storage_path = storage_path_for_task.clone();
            let job_to_execute = job_for_task.clone(); // Clone for run_scheduled_job_internal

            Box::pin(async move {
                let current_time = Utc::now();
                let mut needs_persist = false;
                {
                    let mut jobs_map_guard = current_jobs_arc.lock().await;
                    if let Some((_, current_job_in_map)) = jobs_map_guard.get_mut(&task_job_id) {
                        current_job_in_map.last_run = Some(current_time);
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

                if let Err(e) = run_scheduled_job_internal(job_to_execute).await {
                    tracing::error!(
                        "Scheduled job '{}' execution failed: {}",
                        &e.job_id,
                        e.error
                    );
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

            let cron_task = Job::new_async(&job_to_load.cron, move |_uuid, _l| {
                let task_job_id = job_for_task.id.clone();
                let current_jobs_arc = jobs_arc_for_task.clone();
                let local_storage_path = storage_path_for_task.clone();
                let job_to_execute = job_for_task.clone(); // Clone for run_scheduled_job_internal

                Box::pin(async move {
                    let current_time = Utc::now();
                    let mut needs_persist = false;
                    {
                        let mut jobs_map_guard = current_jobs_arc.lock().await;
                        if let Some((_, stored_job)) = jobs_map_guard.get_mut(&task_job_id) {
                            stored_job.last_run = Some(current_time);
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

                    if let Err(e) = run_scheduled_job_internal(job_to_execute).await {
                        tracing::error!(
                            "Scheduled job '{}' execution failed: {}",
                            &e.job_id,
                            e.error
                        );
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
        jobs_guard: &tokio::sync::MutexGuard<'_, HashMap<String, (JobId, ScheduledJob)>>,
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
    ) -> Result<Vec<SessionMetadata>, SchedulerError> {
        let all_session_files = session::storage::list_sessions()
            .map_err(|e| SchedulerError::StorageError(io::Error::new(io::ErrorKind::Other, e)))?;

        let mut schedule_sessions: Vec<(String, SessionMetadata)> = Vec::new();

        for (session_name, session_path) in all_session_files {
            match session::storage::read_metadata(&session_path) {
                Ok(metadata) => {
                    if metadata.schedule_id.as_deref() == Some(sched_id) {
                        schedule_sessions.push((session_name, metadata));
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

        schedule_sessions.sort_by(|a, b| b.0.cmp(&a.0));

        let result_metadata: Vec<SessionMetadata> = schedule_sessions
            .into_iter()
            .map(|(_, metadata)| metadata)
            .take(limit)
            .collect();

        Ok(result_metadata)
    }

    pub async fn run_now(&self, sched_id: &str) -> Result<String, SchedulerError> {
        let job_to_run: ScheduledJob = {
            let jobs_guard = self.jobs.lock().await;
            match jobs_guard.get(sched_id) {
                Some((_, job_def)) => job_def.clone(),
                None => return Err(SchedulerError::JobNotFound(sched_id.to_string())),
            }
        };

        let session_id = run_scheduled_job_internal(job_to_run.clone())
            .await
            .map_err(|e| {
                SchedulerError::AnyhowError(anyhow!(
                    "Failed to execute job '{}' immediately: {}",
                    sched_id,
                    e.error
                ))
            })?;

        {
            let mut jobs_guard = self.jobs.lock().await;
            if let Some((_tokio_job_id, job_in_map)) = jobs_guard.get_mut(sched_id) {
                job_in_map.last_run = Some(Utc::now());
            } // MutexGuard is dropped here
        }
        // Persist after the lock is released and update is made.
        self.persist_jobs().await?;

        Ok(session_id)
    }
}

#[derive(Debug)]
struct JobExecutionError {
    job_id: String,
    error: String,
}

async fn run_scheduled_job_internal(
    job: ScheduledJob,
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

    let global_config = Config::global();
    let provider_name: String =
        match global_config.get_param("GOOSE_PROVIDER") {
            Ok(name) => name,
            Err(_) => return Err(JobExecutionError {
                job_id: job.id.clone(),
                error:
                    "GOOSE_PROVIDER not configured globally. Run 'goose configure' or set env var."
                        .to_string(),
            }),
        };
    let model_name: String = match global_config.get_param("GOOSE_MODEL") {
        Ok(name) => name,
        Err(_) => {
            return Err(JobExecutionError {
                job_id: job.id.clone(),
                error: "GOOSE_MODEL not configured globally. Run 'goose configure' or set env var."
                    .to_string(),
            })
        }
    };
    let model_config = crate::model::ModelConfig::new(model_name);

    match create(&provider_name, model_config) {
        Ok(provider_instance) => {
            if let Err(e) = agent.update_provider(provider_instance).await {
                return Err(JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to set provider on agent: {}", e),
                });
            }
            tracing::info!(
                "Agent configured with provider '{}' for job '{}'",
                provider_name,
                job.id
            );
        }
        Err(e) => {
            return Err(JobExecutionError {
                job_id: job.id.clone(),
                error: format!(
                    "Failed to create provider instance '{}': {}",
                    provider_name, e
                ),
            });
        }
    }

    let session_id_for_return = session::generate_session_id();

    if let Some(prompt_text) = recipe.prompt {
        let messages = vec![Message::user().with_text(prompt_text)];
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
            id: session::Identifier::Name(session_id_for_return.clone()),
            working_dir: current_dir,
            schedule_id: Some(job.id.clone()),
        };

        match agent.reply(&messages, Some(session_config)).await {
            Ok(mut stream) => {
                use futures::StreamExt;
                while let Some(message_result) = stream.next().await {
                    match message_result {
                        Ok(msg) => {
                            if msg.role == mcp_core::role::Role::Assistant {
                                tracing::info!("[Job {}] Assistant: {:?}", job.id, msg.content);
                            }
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
    }

    tracing::info!("Finished job: {}", job.id);
    Ok(session_id_for_return)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Recipe;
    use crate::session::storage::{get_most_recent_session, read_metadata};
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

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
            cron: "* * * * * * ".to_string(),
            last_run: None,
        };

        let created_session_id = run_scheduled_job_internal(dummy_job.clone())
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

        // Clean up environment variables
        env::remove_var("GOOSE_PROVIDER");
        env::remove_var("GOOSE_MODEL");

        Ok(())
    }
}
