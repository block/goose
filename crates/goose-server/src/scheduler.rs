use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use etcetera::AppStrategy;

use anyhow::Result;
use base64::engine::{general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{job::JobId, Job, JobScheduler};

use etcetera::choose_app_strategy;

use goose::{
    message::Message,
    recipe::Recipe,
    agents::{SessionConfig},
    session,
};

use crate::{state::AppState, APP_STRATEGY};

#[derive(Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub source: String,
    pub cron: String,
    pub last_run: Option<DateTime<Utc>>,
}

pub struct Scheduler {
    scheduler: JobScheduler,
    jobs: Arc<Mutex<HashMap<String, (JobId, ScheduledJob)>>>,
    state: Arc<AppState>,
    storage: std::path::PathBuf,
}

impl Scheduler {
    pub async fn new(state: Arc<AppState>) -> Result<Arc<Self>> {
        let scheduler = JobScheduler::new().await?;
        let storage = choose_app_strategy(APP_STRATEGY.clone())?
            .data_dir()
            .join("schedules.json");

        let sched = Arc::new(Self {
            scheduler,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            state,
            storage,
        });
        sched.load_jobs().await?;
        sched.scheduler.start().await?;
        Ok(sched)
    }

    async fn load_jobs(self: &Arc<Self>) -> Result<()> {
        if let Ok(data) = tokio::fs::read_to_string(&self.storage).await {
            if let Ok(list) = serde_json::from_str::<Vec<ScheduledJob>>(&data) {
                for job_to_load in list {
                    // Clone job_to_load as self.add will consume it if we don't want to pass a reference or re-clone inside add.
                    // However, self.add already expects ownership of 'job: ScheduledJob'.
                    let _ = self.add(job_to_load).await; 
                }
            }
        }
        Ok(())
    }

    async fn persist(&self) -> Result<()> {
        let jobs_guard = self.jobs.lock().await;
        let list: Vec<ScheduledJob> = jobs_guard.values().map(|(_, j)| j.clone()).collect();
        if let Some(parent) = self.storage.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let data = serde_json::to_string_pretty(&list)?;
        tokio::fs::write(&self.storage, data).await?;
        Ok(())
    }

    pub async fn add(self: &Arc<Self>, job: ScheduledJob) -> Result<()> {
        let job_id_str_for_closure = job.id.clone(); // Clone for the closure to capture
        let cron = job.cron.clone();
        let state = self.state.clone();
        let jobs_arc = self.jobs.clone(); // Arc clone, cheap
        let job_for_closure = job.clone(); // Clone the whole job for the closure

        let cron_task = Job::new_async(cron.as_str(), move |_uuid, _l| {
            let state_clone_for_async = state.clone();
            let jobs_arc_clone_for_async = jobs_arc.clone();
            let captured_job_id_str = job_id_str_for_closure.clone(); // Clone the string ID again for the async block
            let job_to_run = job_for_closure.clone();
            Box::pin(async move {
                {
                    let mut jobs_map_guard = jobs_arc_clone_for_async.lock().await;
                    if let Some((_, scheduled_job_in_map)) = jobs_map_guard.get_mut(&captured_job_id_str) {
                        scheduled_job_in_map.last_run = Some(Utc::now());
                    }
                }
                if let Err(e) = run_job(state_clone_for_async, job_to_run).await {
                    tracing::error!("scheduled job error: {:?}", e);
                }
            })
        })?;
        
        let scheduler_internal_uuid = self.scheduler.add(cron_task).await?;
        // Insert the original 'job' (which contains the 'id' String) into the map.
        // The key is job.id (String), the value is (scheduler's JobId (Uuid), original ScheduledJob struct)
        self.jobs.lock().await.insert(job.id.clone(), (scheduler_internal_uuid, job));
        self.persist().await?;
        Ok(())
    }

    pub async fn list(&self) -> Vec<ScheduledJob> {
        self.jobs
            .lock()
            .await
            .values()
            .map(|(_, j)| j.clone())
            .collect()
    }

    pub async fn remove(&self, id: &str) -> Result<()> {
        if let Some((job_id, _)) = self.jobs.lock().await.remove(id) {
            let _ = self.scheduler.remove(&job_id).await;
        }
        self.persist().await?;
        Ok(())
    }
}

async fn run_job(state: Arc<AppState>, job: ScheduledJob) -> Result<()> {
    let recipe = load_recipe(&job.source).await?;
    execute_recipe(state, recipe).await
}

async fn load_recipe(source: &str) -> Result<Recipe> {
    if Path::new(source).exists() {
        let content = tokio::fs::read_to_string(source).await?;
        parse_recipe(&content)
    } else if let Some(idx) = source.find("config=") {
        let encoded = &source[idx + 7..];
        let bytes = BASE64_STANDARD.decode(encoded)?;
        let json = String::from_utf8(bytes)?;
        parse_recipe(&json)
    } else {
        let bytes = BASE64_STANDARD.decode(source)?;
        let json = String::from_utf8(bytes)?;
        parse_recipe(&json)
    }
}

fn parse_recipe(content: &str) -> Result<Recipe> {
    if let Ok(r) = serde_json::from_str::<Recipe>(content) {
        Ok(r)
    } else {
        Ok(serde_yaml::from_str::<Recipe>(content)?)
    }
}

async fn execute_recipe(state: Arc<AppState>, recipe: Recipe) -> Result<()> {
    
    
    use futures::StreamExt as _;

    let agent = state.get_agent().await?;
    if let Some(instructions) = recipe.instructions.clone() {
        agent.override_system_prompt(instructions).await;
    }

    if let Some(prompt) = recipe.prompt.clone() {
        let messages = vec![Message::user().with_text(prompt)];
        let mut stream = agent
            .reply(
                &messages,
                Some(SessionConfig {
                    id: session::Identifier::Name(session::generate_session_id()),
                    working_dir: std::env::current_dir()?,
                }),
            )
            .await?;
        while let Some(_m) = stream.next().await {}
    }

    Ok(())
}
