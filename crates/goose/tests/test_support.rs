#![cfg(test)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tempfile::TempDir;
use tokio::sync::Mutex;

use goose::agents::Agent;
use goose::scheduler::{ScheduledJob, SchedulerError};
use goose::scheduler_trait::SchedulerTrait;
use goose::session::storage::SessionMetadata;

#[derive(Debug, Clone)]
pub enum MockBehavior {
    Success,
    NotFound(String),
    AlreadyExists(String),
    InternalError(String),
    JobCurrentlyRunning(String),
}

#[derive(Clone)]
pub struct ConfigurableMockScheduler {
    jobs: Arc<Mutex<HashMap<String, ScheduledJob>>>,
    running_jobs: Arc<Mutex<HashSet<String>>>,
    call_log: Arc<Mutex<Vec<String>>>,
    behaviors: Arc<Mutex<HashMap<String, MockBehavior>>>,
    sessions_data: Arc<Mutex<HashMap<String, Vec<(String, SessionMetadata)>>>>,
}

#[allow(dead_code)]
impl ConfigurableMockScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            running_jobs: Arc::new(Mutex::new(HashSet::new())),
            call_log: Arc::new(Mutex::new(Vec::new())),
            behaviors: Arc::new(Mutex::new(HashMap::new())),
            sessions_data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn with_behavior(self, method: &str, behavior: MockBehavior) -> Self {
        self.behaviors
            .lock()
            .await
            .insert(method.to_string(), behavior);
        self
    }

    pub async fn with_existing_job(self, job: ScheduledJob) -> Self {
        self.jobs.lock().await.insert(job.id.clone(), job);
        self
    }

    pub async fn with_running_job(self, job_id: &str) -> Self {
        self.running_jobs.lock().await.insert(job_id.to_string());
        self
    }

    pub async fn with_sessions_data(
        self,
        job_id: &str,
        sessions: Vec<(String, SessionMetadata)>,
    ) -> Self {
        self.sessions_data
            .lock()
            .await
            .insert(job_id.to_string(), sessions);
        self
    }

    pub async fn get_calls(&self) -> Vec<String> {
        self.call_log.lock().await.clone()
    }

    async fn log_call(&self, method: &str) {
        self.call_log.lock().await.push(method.to_string());
    }

    async fn get_behavior(&self, method: &str) -> MockBehavior {
        self.behaviors
            .lock()
            .await
            .get(method)
            .cloned()
            .unwrap_or(MockBehavior::Success)
    }
}

#[async_trait]
impl SchedulerTrait for ConfigurableMockScheduler {
    async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
        self.log_call("add_scheduled_job").await;

        match self.get_behavior("add_scheduled_job").await {
            MockBehavior::Success => {
                let mut jobs = self.jobs.lock().await;
                if jobs.contains_key(&job.id) {
                    return Err(SchedulerError::JobIdExists(job.id));
                }
                jobs.insert(job.id.clone(), job);
                Ok(())
            }
            MockBehavior::AlreadyExists(id) => Err(SchedulerError::JobIdExists(id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(()),
        }
    }

    async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.log_call("list_scheduled_jobs").await;

        match self.get_behavior("list_scheduled_jobs").await {
            MockBehavior::Success => {
                let jobs = self.jobs.lock().await;
                Ok(jobs.values().cloned().collect())
            }
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(vec![]),
        }
    }

    async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
        self.log_call("remove_scheduled_job").await;

        match self.get_behavior("remove_scheduled_job").await {
            MockBehavior::Success => {
                let mut jobs = self.jobs.lock().await;
                if jobs.remove(id).is_some() {
                    Ok(())
                } else {
                    Err(SchedulerError::JobNotFound(id.to_string()))
                }
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(()),
        }
    }

    async fn pause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.log_call("pause_schedule").await;

        match self.get_behavior("pause_schedule").await {
            MockBehavior::Success => {
                let jobs = self.jobs.lock().await;
                if jobs.contains_key(id) {
                    Ok(())
                } else {
                    Err(SchedulerError::JobNotFound(id.to_string()))
                }
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::JobCurrentlyRunning(job_id) => {
                Err(SchedulerError::AnyhowError(anyhow::anyhow!(
                    "Cannot pause schedule '{}' while it's currently running",
                    job_id
                )))
            }
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(()),
        }
    }

    async fn unpause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.log_call("unpause_schedule").await;

        match self.get_behavior("unpause_schedule").await {
            MockBehavior::Success => {
                let jobs = self.jobs.lock().await;
                if jobs.contains_key(id) {
                    Ok(())
                } else {
                    Err(SchedulerError::JobNotFound(id.to_string()))
                }
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(()),
        }
    }

    async fn run_now(&self, id: &str) -> Result<String, SchedulerError> {
        self.log_call("run_now").await;

        match self.get_behavior("run_now").await {
            MockBehavior::Success => {
                let jobs = self.jobs.lock().await;
                if jobs.contains_key(id) {
                    Ok(format!("{}_session_{}", id, chrono::Utc::now().timestamp()))
                } else {
                    Err(SchedulerError::JobNotFound(id.to_string()))
                }
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok("mock_session_123".to_string()),
        }
    }

    async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
        self.log_call("sessions").await;

        match self.get_behavior("sessions").await {
            MockBehavior::Success => {
                let sessions_data = self.sessions_data.lock().await;
                let sessions = sessions_data.get(sched_id).cloned().unwrap_or_default();
                Ok(sessions.into_iter().take(limit).collect())
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(vec![]),
        }
    }

    async fn update_schedule(
        &self,
        sched_id: &str,
        _new_cron: String,
    ) -> Result<(), SchedulerError> {
        self.log_call("update_schedule").await;

        match self.get_behavior("update_schedule").await {
            MockBehavior::Success => {
                let jobs = self.jobs.lock().await;
                if jobs.contains_key(sched_id) {
                    Ok(())
                } else {
                    Err(SchedulerError::JobNotFound(sched_id.to_string()))
                }
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(()),
        }
    }

    async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError> {
        self.log_call("kill_running_job").await;

        match self.get_behavior("kill_running_job").await {
            MockBehavior::Success => {
                let running_jobs = self.running_jobs.lock().await;
                if running_jobs.contains(sched_id) {
                    Ok(())
                } else {
                    Err(SchedulerError::AnyhowError(anyhow::anyhow!(
                        "Schedule '{}' is not currently running",
                        sched_id
                    )))
                }
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(()),
        }
    }

    async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        self.log_call("get_running_job_info").await;

        match self.get_behavior("get_running_job_info").await {
            MockBehavior::Success => {
                let running_jobs = self.running_jobs.lock().await;
                if running_jobs.contains(sched_id) {
                    Ok(Some((format!("{}_session", sched_id), Utc::now())))
                } else {
                    Ok(None)
                }
            }
            MockBehavior::NotFound(job_id) => Err(SchedulerError::JobNotFound(job_id)),
            MockBehavior::InternalError(msg) => Err(SchedulerError::SchedulerInternalError(msg)),
            _ => Ok(None),
        }
    }
}

// Helper for creating temp recipe files
pub struct TempRecipe {
    pub path: PathBuf,
    _temp_dir: TempDir, // Keep alive
}

pub fn create_temp_recipe(valid: bool, format: &str) -> TempRecipe {
    let temp_dir = tempfile::tempdir().unwrap();
    let filename = format!("test_recipe.{}", format);
    let path = temp_dir.path().join(filename);

    let content = if valid {
        match format {
            "json" => {
                r#"{
    "version": "1.0.0",
    "title": "Test Recipe",
    "description": "A test recipe",
    "prompt": "Hello world"
}"#
            }
            "yaml" | "yml" => {
                r#"version: "1.0.0"
title: "Test Recipe"
description: "A test recipe"
prompt: "Hello world"
"#
            }
            _ => panic!("Unsupported format: {}", format),
        }
    } else {
        match format {
            "json" => r#"{"invalid": json syntax"#,
            "yaml" | "yml" => "invalid:\n  - yaml: syntax: error",
            _ => "invalid content",
        }
    };

    std::fs::write(&path, content).unwrap();
    TempRecipe {
        path,
        _temp_dir: temp_dir,
    }
}

// Test builder for easy setup
pub struct ScheduleToolTestBuilder {
    scheduler: Arc<ConfigurableMockScheduler>,
}

impl ScheduleToolTestBuilder {
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(ConfigurableMockScheduler::new()),
        }
    }

    pub async fn with_scheduler_behavior(self, method: &str, behavior: MockBehavior) -> Self {
        {
            let mut behaviors = self.scheduler.behaviors.lock().await;
            behaviors.insert(method.to_string(), behavior);
        }
        self
    }

    pub async fn with_existing_job(self, job_id: &str, cron: &str) -> Self {
        let job = ScheduledJob {
            id: job_id.to_string(),
            source: "/tmp/test.json".to_string(),
            cron: cron.to_string(),
            last_run: None,
            currently_running: false,
            paused: false,
            current_session_id: None,
            process_start_time: None,
            execution_mode: Some("background".to_string()),
        };
        {
            let mut jobs = self.scheduler.jobs.lock().await;
            jobs.insert(job.id.clone(), job);
        }
        self
    }

    pub async fn with_running_job(self, job_id: &str) -> Self {
        {
            let mut running_jobs = self.scheduler.running_jobs.lock().await;
            running_jobs.insert(job_id.to_string());
        }
        self
    }

    pub async fn with_sessions_data(
        self,
        job_id: &str,
        sessions: Vec<(String, SessionMetadata)>,
    ) -> Self {
        {
            let mut sessions_data = self.scheduler.sessions_data.lock().await;
            sessions_data.insert(job_id.to_string(), sessions);
        }
        self
    }

    pub async fn build(self) -> (Agent, Arc<ConfigurableMockScheduler>) {
        let agent = Agent::new();
        agent.set_scheduler(self.scheduler.clone()).await;
        (agent, self.scheduler)
    }
}

// Helper function to create test session metadata
pub fn create_test_session_metadata(message_count: usize, working_dir: &str) -> SessionMetadata {
    SessionMetadata {
        message_count,
        working_dir: PathBuf::from(working_dir),
        description: "Test session".to_string(),
        schedule_id: Some("test_job".to_string()),
        project_id: None,
        total_tokens: Some(100),
        input_tokens: Some(50),
        output_tokens: Some(50),
        accumulated_total_tokens: Some(100),
        accumulated_input_tokens: Some(50),
        accumulated_output_tokens: Some(50),
    }
}
