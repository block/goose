use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{job::JobId, Job, JobScheduler as TokioJobScheduler};

use crate::agents::AgentEvent;
use crate::agents::{Agent, SessionConfig};
use crate::config::paths::Paths;
use crate::config::Config;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider as GooseProvider;
use crate::providers::create;
use crate::recipe::Recipe;
use crate::scheduler_trait::SchedulerTrait;
use crate::session::session_manager::SessionType;
use crate::session::{Session, SessionManager};

type RunningTasksMap = HashMap<String, tokio::task::AbortHandle>;
type JobsMap = HashMap<String, (JobId, ScheduledJob)>;

pub fn normalize_cron_expression(src: &str) -> String {
    let mut parts: Vec<&str> = src.split_whitespace().collect();

    match parts.len() {
        5 => {
            parts.insert(0, "0");
            parts.push("*");
        }
        6 => {
            parts.push("*");
        }
        7 => {}
        _ => {
            tracing::warn!(
                "Unrecognised cron expression '{}': expected 5, 6 or 7 fields (got {})",
                src,
                parts.len()
            );
            return src.to_string();
        }
    }

    parts.join(" ")
}

fn to_tokio_cron(normalized: &str) -> String {
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.len() == 7 {
        parts[..6].join(" ")
    } else {
        normalized.to_string()
    }
}

pub fn get_default_scheduler_storage_path() -> Result<PathBuf, io::Error> {
    let data_dir = Paths::data_dir();
    fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("schedules.json"))
}

pub fn get_default_scheduled_recipes_dir() -> Result<PathBuf, SchedulerError> {
    let data_dir = Paths::data_dir();
    let recipes_dir = data_dir.join("scheduled_recipes");
    fs::create_dir_all(&recipes_dir).map_err(SchedulerError::StorageError)?;
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
}

async fn persist_jobs(
    storage_path: &Path,
    jobs: &Arc<Mutex<JobsMap>>,
) -> Result<(), SchedulerError> {
    let jobs_guard = jobs.lock().await;
    let list: Vec<ScheduledJob> = jobs_guard.values().map(|(_, j)| j.clone()).collect();
    drop(jobs_guard);

    if let Some(parent) = storage_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(&list)?;
    fs::write(storage_path, data)?;
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

    fn create_cron_task(&self, job: ScheduledJob) -> Result<Job, SchedulerError> {
        let job_for_task = job.clone();
        let jobs_arc = self.jobs.clone();
        let storage_path = self.storage_path.clone();
        let running_tasks_arc = self.running_tasks.clone();

        let normalized_cron = normalize_cron_expression(&job.cron);
        let tokio_cron = to_tokio_cron(&normalized_cron);

        Job::new_async(&tokio_cron, move |_uuid, _l| {
            let task_job_id = job_for_task.id.clone();
            let current_jobs_arc = jobs_arc.clone();
            let local_storage_path = storage_path.clone();
            let job_to_execute = job_for_task.clone();
            let running_tasks = running_tasks_arc.clone();

            Box::pin(async move {
                let should_execute = {
                    let jobs_guard = current_jobs_arc.lock().await;
                    jobs_guard
                        .get(&task_job_id)
                        .map(|(_, j)| !j.paused)
                        .unwrap_or(false)
                };

                if !should_execute {
                    return;
                }

                let current_time = Utc::now();
                {
                    let mut jobs_guard = current_jobs_arc.lock().await;
                    if let Some((_, job)) = jobs_guard.get_mut(&task_job_id) {
                        job.last_run = Some(current_time);
                        job.currently_running = true;
                        job.process_start_time = Some(current_time);
                    }
                }

                if let Err(e) = persist_jobs(&local_storage_path, &current_jobs_arc).await {
                    tracing::error!("Failed to persist job status: {}", e);
                }

                let job_task = tokio::spawn(execute_job(
                    job_to_execute,
                    Some(current_jobs_arc.clone()),
                    Some(task_job_id.clone()),
                ));

                {
                    let mut tasks = running_tasks.lock().await;
                    tasks.insert(task_job_id.clone(), job_task.abort_handle());
                }

                let result = job_task.await;

                {
                    let mut tasks = running_tasks.lock().await;
                    tasks.remove(&task_job_id);
                }

                {
                    let mut jobs_guard = current_jobs_arc.lock().await;
                    if let Some((_, job)) = jobs_guard.get_mut(&task_job_id) {
                        job.currently_running = false;
                        job.current_session_id = None;
                        job.process_start_time = None;
                    }
                }

                if let Err(e) = persist_jobs(&local_storage_path, &current_jobs_arc).await {
                    tracing::error!("Failed to persist job completion: {}", e);
                }

                match result {
                    Ok(Ok(_)) => tracing::info!("Job '{}' completed", task_job_id),
                    Ok(Err(e)) => tracing::error!("Job '{}' failed: {}", e.job_id, e.error),
                    Err(e) if e.is_cancelled() => tracing::info!("Job '{}' cancelled", task_job_id),
                    Err(e) => tracing::error!("Job '{}' task error: {}", task_job_id, e),
                }
            })
        })
        .map_err(|e| SchedulerError::CronParseError(e.to_string()))
    }

    pub async fn add_scheduled_job(
        &self,
        original_job_spec: ScheduledJob,
    ) -> Result<(), SchedulerError> {
        {
            let jobs_guard = self.jobs.lock().await;
            if jobs_guard.contains_key(&original_job_spec.id) {
                return Err(SchedulerError::JobIdExists(original_job_spec.id.clone()));
            }
        }

        let original_recipe_path = Path::new(&original_job_spec.source);
        if !original_recipe_path.is_file() {
            return Err(SchedulerError::RecipeLoadError(format!(
                "Recipe file not found: {}",
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

        fs::copy(original_recipe_path, &destination_recipe_path)?;

        let mut stored_job = original_job_spec;
        stored_job.source = destination_recipe_path.to_string_lossy().into_owned();
        stored_job.current_session_id = None;
        stored_job.process_start_time = None;

        let cron_task = self.create_cron_task(stored_job.clone())?;

        let job_uuid = self
            .internal_scheduler
            .add(cron_task)
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        {
            let mut jobs_guard = self.jobs.lock().await;
            jobs_guard.insert(stored_job.id.clone(), (job_uuid, stored_job));
        }

        persist_jobs(&self.storage_path, &self.jobs).await?;
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

        let list: Vec<ScheduledJob> = serde_json::from_str(&data)?;

        for job_to_load in list {
            if !Path::new(&job_to_load.source).exists() {
                tracing::warn!("Recipe file {} not found, skipping", job_to_load.source);
                continue;
            }

            let cron_task = self.create_cron_task(job_to_load.clone())?;

            let job_uuid = self
                .internal_scheduler
                .add(cron_task)
                .await
                .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

            let mut jobs_guard = self.jobs.lock().await;
            jobs_guard.insert(job_to_load.id.clone(), (job_uuid, job_to_load));
        }
        Ok(())
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
        let (job_uuid, recipe_path) = {
            let mut jobs_guard = self.jobs.lock().await;
            match jobs_guard.remove(id) {
                Some((uuid, job)) => (uuid, job.source.clone()),
                None => return Err(SchedulerError::JobNotFound(id.to_string())),
            }
        };

        self.internal_scheduler
            .remove(&job_uuid)
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        let path = Path::new(&recipe_path);
        if path.exists() {
            fs::remove_file(path)?;
        }

        persist_jobs(&self.storage_path, &self.jobs).await?;
        Ok(())
    }

    pub async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, Session)>, SchedulerError> {
        let all_sessions = SessionManager::list_sessions()
            .await
            .map_err(|e| SchedulerError::StorageError(io::Error::other(e)))?;

        let mut schedule_sessions: Vec<(String, Session)> = all_sessions
            .into_iter()
            .filter(|s| s.schedule_id.as_deref() == Some(sched_id))
            .map(|s| (s.id.clone(), s))
            .collect();

        schedule_sessions.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
        schedule_sessions.truncate(limit);

        Ok(schedule_sessions)
    }

    pub async fn run_now(&self, sched_id: &str) -> Result<String, SchedulerError> {
        let job_to_run = {
            let mut jobs_guard = self.jobs.lock().await;
            match jobs_guard.get_mut(sched_id) {
                Some((_, job)) => {
                    job.currently_running = true;
                    job.clone()
                }
                None => return Err(SchedulerError::JobNotFound(sched_id.to_string())),
            }
        };

        persist_jobs(&self.storage_path, &self.jobs).await?;

        let job_task = tokio::spawn(execute_job(
            job_to_run,
            Some(self.jobs.clone()),
            Some(sched_id.to_string()),
        ));

        {
            let mut tasks = self.running_tasks.lock().await;
            tasks.insert(sched_id.to_string(), job_task.abort_handle());
        }

        let result = job_task.await;

        {
            let mut tasks = self.running_tasks.lock().await;
            tasks.remove(sched_id);
        }

        {
            let mut jobs_guard = self.jobs.lock().await;
            if let Some((_, job)) = jobs_guard.get_mut(sched_id) {
                job.currently_running = false;
                job.current_session_id = None;
                job.process_start_time = None;
                job.last_run = Some(Utc::now());
            }
        }

        persist_jobs(&self.storage_path, &self.jobs).await?;

        match result {
            Ok(Ok(session_id)) => Ok(session_id),
            Ok(Err(e)) => Err(SchedulerError::AnyhowError(anyhow!(
                "Job '{}' failed: {}",
                sched_id,
                e.error
            ))),
            Err(e) if e.is_cancelled() => Err(SchedulerError::AnyhowError(anyhow!(
                "Job '{}' cancelled",
                sched_id
            ))),
            Err(e) => Err(SchedulerError::AnyhowError(anyhow!(
                "Job '{}' task error: {}",
                sched_id,
                e
            ))),
        }
    }

    pub async fn pause_schedule(&self, sched_id: &str) -> Result<(), SchedulerError> {
        {
            let mut jobs_guard = self.jobs.lock().await;
            match jobs_guard.get_mut(sched_id) {
                Some((_, job)) => {
                    if job.currently_running {
                        return Err(SchedulerError::AnyhowError(anyhow!(
                            "Cannot pause running schedule '{}'",
                            sched_id
                        )));
                    }
                    job.paused = true;
                }
                None => return Err(SchedulerError::JobNotFound(sched_id.to_string())),
            }
        }

        persist_jobs(&self.storage_path, &self.jobs).await
    }

    pub async fn unpause_schedule(&self, sched_id: &str) -> Result<(), SchedulerError> {
        {
            let mut jobs_guard = self.jobs.lock().await;
            match jobs_guard.get_mut(sched_id) {
                Some((_, job)) => job.paused = false,
                None => return Err(SchedulerError::JobNotFound(sched_id.to_string())),
            }
        }

        persist_jobs(&self.storage_path, &self.jobs).await
    }

    pub async fn update_schedule(
        &self,
        sched_id: &str,
        new_cron: String,
    ) -> Result<(), SchedulerError> {
        let (old_uuid, updated_job) = {
            let mut jobs_guard = self.jobs.lock().await;
            match jobs_guard.get_mut(sched_id) {
                Some((uuid, job)) => {
                    if job.currently_running {
                        return Err(SchedulerError::AnyhowError(anyhow!(
                            "Cannot update running schedule '{}'",
                            sched_id
                        )));
                    }
                    if new_cron == job.cron {
                        return Ok(());
                    }
                    job.cron = new_cron.clone();
                    (*uuid, job.clone())
                }
                None => return Err(SchedulerError::JobNotFound(sched_id.to_string())),
            }
        };

        self.internal_scheduler
            .remove(&old_uuid)
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        let cron_task = self.create_cron_task(updated_job)?;
        let new_uuid = self
            .internal_scheduler
            .add(cron_task)
            .await
            .map_err(|e| SchedulerError::SchedulerInternalError(e.to_string()))?;

        {
            let mut jobs_guard = self.jobs.lock().await;
            if let Some((uuid, _)) = jobs_guard.get_mut(sched_id) {
                *uuid = new_uuid;
            }
        }

        persist_jobs(&self.storage_path, &self.jobs).await
    }

    pub async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError> {
        {
            let jobs_guard = self.jobs.lock().await;
            match jobs_guard.get(sched_id) {
                Some((_, job)) if !job.currently_running => {
                    return Err(SchedulerError::AnyhowError(anyhow!(
                        "Schedule '{}' is not running",
                        sched_id
                    )));
                }
                None => return Err(SchedulerError::JobNotFound(sched_id.to_string())),
                _ => {}
            }
        }

        {
            let mut tasks = self.running_tasks.lock().await;
            if let Some(handle) = tasks.remove(sched_id) {
                handle.abort();
            }
        }

        {
            let mut jobs_guard = self.jobs.lock().await;
            if let Some((_, job)) = jobs_guard.get_mut(sched_id) {
                job.currently_running = false;
                job.current_session_id = None;
                job.process_start_time = None;
            }
        }

        persist_jobs(&self.storage_path, &self.jobs).await
    }

    pub async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        let jobs_guard = self.jobs.lock().await;
        match jobs_guard.get(sched_id) {
            Some((_, job)) if job.currently_running => {
                match (&job.current_session_id, &job.process_start_time) {
                    (Some(sid), Some(start)) => Ok(Some((sid.clone(), *start))),
                    _ => Ok(None),
                }
            }
            Some(_) => Ok(None),
            None => Err(SchedulerError::JobNotFound(sched_id.to_string())),
        }
    }
}

#[derive(Debug)]
struct JobExecutionError {
    job_id: String,
    error: String,
}

#[cfg(not(test))]
async fn execute_job(
    job: ScheduledJob,
    jobs_arc: Option<Arc<Mutex<JobsMap>>>,
    job_id: Option<String>,
) -> Result<String, JobExecutionError> {
    run_scheduled_job_internal(job, None, jobs_arc, job_id).await
}

#[cfg(test)]
async fn execute_job(
    job: ScheduledJob,
    _jobs_arc: Option<Arc<Mutex<JobsMap>>>,
    _job_id: Option<String>,
) -> Result<String, JobExecutionError> {
    Ok(format!("test-session-{}", job.id))
}

async fn run_scheduled_job_internal(
    job: ScheduledJob,
    provider_override: Option<Arc<dyn GooseProvider>>,
    jobs_arc: Option<Arc<Mutex<JobsMap>>>,
    job_id: Option<String>,
) -> std::result::Result<String, JobExecutionError> {
    let recipe_path = Path::new(&job.source);

    let recipe_content = fs::read_to_string(recipe_path).map_err(|e| JobExecutionError {
        job_id: job.id.clone(),
        error: format!("Failed to load recipe: {}", e),
    })?;

    let recipe: Recipe = {
        let extension = recipe_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("yaml")
            .to_lowercase();

        match extension.as_str() {
            "json" | "jsonl" => {
                serde_json::from_str(&recipe_content).map_err(|e| JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to parse JSON recipe: {}", e),
                })
            }
            _ => serde_yaml::from_str(&recipe_content).map_err(|e| JobExecutionError {
                job_id: job.id.clone(),
                error: format!("Failed to parse YAML recipe: {}", e),
            }),
        }?
    };

    let agent = Agent::new();

    let agent_provider = match provider_override {
        Some(p) => p,
        None => {
            let config = Config::global();
            let provider_name = config.get_goose_provider().map_err(|_| JobExecutionError {
                job_id: job.id.clone(),
                error: "GOOSE_PROVIDER not configured".to_string(),
            })?;
            let model_name = config.get_goose_model().map_err(|_| JobExecutionError {
                job_id: job.id.clone(),
                error: "GOOSE_MODEL not configured".to_string(),
            })?;
            let model_config =
                crate::model::ModelConfig::new(&model_name).map_err(|e| JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Model config error: {}", e),
                })?;

            create(&provider_name, model_config)
                .await
                .map_err(|e| JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to create provider: {}", e),
                })?
        }
    };

    if let Some(ref extensions) = recipe.extensions {
        for ext in extensions {
            agent
                .add_extension(ext.clone())
                .await
                .map_err(|e| JobExecutionError {
                    job_id: job.id.clone(),
                    error: format!("Failed to add extension: {}", e),
                })?;
        }
    }

    agent
        .update_provider(agent_provider)
        .await
        .map_err(|e| JobExecutionError {
            job_id: job.id.clone(),
            error: format!("Failed to set provider: {}", e),
        })?;

    let current_dir = std::env::current_dir().map_err(|e| JobExecutionError {
        job_id: job.id.clone(),
        error: format!("Failed to get current directory: {}", e),
    })?;

    let session = SessionManager::create_session(
        current_dir,
        format!("Scheduled job: {}", job.id),
        SessionType::Scheduled,
    )
    .await
    .map_err(|e| JobExecutionError {
        job_id: job.id.clone(),
        error: format!("Failed to create session: {}", e),
    })?;

    if let (Some(jobs), Some(jid)) = (jobs_arc.as_ref(), job_id.as_ref()) {
        let mut jobs_guard = jobs.lock().await;
        if let Some((_, job_def)) = jobs_guard.get_mut(jid) {
            job_def.current_session_id = Some(session.id.clone());
        }
    }

    let prompt_text = recipe
        .prompt
        .as_ref()
        .or(recipe.instructions.as_ref())
        .unwrap();

    let user_message = Message::user().with_text(prompt_text);
    let mut conversation = Conversation::new_unvalidated(vec![user_message.clone()]);

    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: Some(job.id.clone()),
        max_turns: None,
        retry_config: None,
    };

    let session_id = Some(session_config.id.clone());
    match crate::session_context::with_session_id(session_id, async {
        agent.reply(user_message, session_config, None).await
    })
    .await
    {
        Ok(mut stream) => {
            use futures::StreamExt;

            while let Some(message_result) = stream.next().await {
                tokio::task::yield_now().await;

                match message_result {
                    Ok(AgentEvent::Message(msg)) => {
                        conversation.push(msg);
                    }
                    Ok(AgentEvent::HistoryReplaced(updated)) => {
                        conversation = updated;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Error in agent stream: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            return Err(JobExecutionError {
                job_id: job.id.clone(),
                error: format!("Agent failed: {}", e),
            });
        }
    }

    if let Err(e) = SessionManager::update_session(&session.id)
        .schedule_id(Some(job.id.clone()))
        .recipe(Some(recipe))
        .apply()
        .await
    {
        tracing::error!("Failed to update session: {}", e);
    }

    Ok(session.id)
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
    ) -> Result<Vec<(String, Session)>, SchedulerError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    fn create_test_recipe_file(dir: &Path, job_id: &str) -> String {
        let recipe_path = dir.join(format!("{}.yaml", job_id));
        fs::write(
            &recipe_path,
            r#"
version: "1.0.0"
title: "Test Recipe"
description: "Test"
prompt: "Test prompt"
"#,
        )
        .unwrap();
        recipe_path.to_string_lossy().into_owned()
    }

    #[tokio::test]
    async fn test_job_runs_on_schedule() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("schedules.json");
        let scheduler = Scheduler::new(storage_path).await.unwrap();
        let recipe_source = create_test_recipe_file(temp_dir.path(), "scheduled_job");

        let job = ScheduledJob {
            id: "scheduled_job".to_string(),
            source: recipe_source,
            cron: "* * * * * *".to_string(),
            last_run: None,
            currently_running: false,
            paused: false,
            current_session_id: None,
            process_start_time: None,
        };

        scheduler.add_scheduled_job(job).await.unwrap();
        sleep(Duration::from_millis(1500)).await;

        let jobs = scheduler.list_scheduled_jobs().await;
        assert!(jobs[0].last_run.is_some(), "Job should have run");
    }

    #[tokio::test]
    async fn test_paused_job_does_not_run() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("schedules.json");
        let scheduler = Scheduler::new(storage_path).await.unwrap();
        let recipe_source = create_test_recipe_file(temp_dir.path(), "paused_job");

        let job = ScheduledJob {
            id: "paused_job".to_string(),
            source: recipe_source,
            cron: "* * * * * *".to_string(),
            last_run: None,
            currently_running: false,
            paused: false,
            current_session_id: None,
            process_start_time: None,
        };

        scheduler.add_scheduled_job(job).await.unwrap();
        scheduler.pause_schedule("paused_job").await.unwrap();
        sleep(Duration::from_millis(1500)).await;

        let jobs = scheduler.list_scheduled_jobs().await;
        assert!(jobs[0].last_run.is_none(), "Paused job should not run");
    }
}
