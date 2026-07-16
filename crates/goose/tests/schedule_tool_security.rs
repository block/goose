use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use goose::agents::{Agent, AgentConfig, GoosePlatform};
use goose::config::permission::PermissionManager;
use goose::config::GooseMode;
use goose::scheduler::{ScheduledJob, SchedulerError};
use goose::scheduler_trait::SchedulerTrait;
use goose::session::{Session, SessionManager};
use tempfile::TempDir;

struct MockScheduler {
    jobs: tokio::sync::Mutex<Vec<ScheduledJob>>,
    validated_recipes: tokio::sync::Mutex<Vec<Vec<u8>>>,
}

impl MockScheduler {
    fn new() -> Self {
        Self {
            jobs: tokio::sync::Mutex::new(Vec::new()),
            validated_recipes: tokio::sync::Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl SchedulerTrait for MockScheduler {
    async fn add_scheduled_job(
        &self,
        job: ScheduledJob,
        _copy: bool,
    ) -> Result<(), SchedulerError> {
        self.jobs.lock().await.push(job);
        Ok(())
    }

    async fn add_scheduled_job_with_recipe(
        &self,
        job: ScheduledJob,
        validated_recipe: Vec<u8>,
    ) -> Result<(), SchedulerError> {
        self.jobs.lock().await.push(job);
        self.validated_recipes.lock().await.push(validated_recipe);
        Ok(())
    }

    async fn schedule_recipe(
        &self,
        _recipe_path: PathBuf,
        _cron_schedule: Option<String>,
    ) -> Result<(), SchedulerError> {
        Ok(())
    }

    async fn list_scheduled_jobs(&self) -> Vec<ScheduledJob> {
        self.jobs.lock().await.clone()
    }

    async fn remove_scheduled_job(&self, _id: &str, _remove: bool) -> Result<(), SchedulerError> {
        Ok(())
    }

    async fn pause_schedule(&self, _id: &str) -> Result<(), SchedulerError> {
        Ok(())
    }

    async fn unpause_schedule(&self, _id: &str) -> Result<(), SchedulerError> {
        Ok(())
    }

    async fn run_now(&self, _id: &str) -> Result<String, SchedulerError> {
        Ok("test-session".to_string())
    }

    async fn sessions(
        &self,
        _sched_id: &str,
        _limit: usize,
    ) -> Result<Vec<(String, Session)>, SchedulerError> {
        Ok(Vec::new())
    }

    async fn update_schedule(
        &self,
        _sched_id: &str,
        _new_cron: String,
    ) -> Result<(), SchedulerError> {
        Ok(())
    }

    async fn kill_running_job(&self, _sched_id: &str) -> Result<(), SchedulerError> {
        Ok(())
    }

    async fn get_running_job_info(
        &self,
        _sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        Ok(None)
    }
}

fn agent_with_scheduler(temp_dir: &TempDir, scheduler: Arc<MockScheduler>) -> Agent {
    let data_dir = temp_dir.path().join("data");
    let session_manager = Arc::new(SessionManager::new(data_dir.clone()));
    let permission_manager = Arc::new(PermissionManager::new(data_dir));
    let config = AgentConfig::new(
        session_manager,
        permission_manager,
        Some(scheduler),
        GooseMode::Auto,
        false,
        GoosePlatform::GooseCli,
    );
    Agent::with_config(config)
}

async fn create_schedule(agent: &Agent, recipe_path: &Path) -> Result<(), String> {
    agent
        .handle_schedule_management(
            serde_json::json!({
                "action": "create",
                "recipe_path": recipe_path,
                "cron_expression": "0 * * * *"
            }),
            "test-request".to_string(),
        )
        .await
        .map(|_| ())
        .map_err(|error| error.message.to_string())
}

#[tokio::test]
async fn parse_errors_do_not_reflect_recipe_contents() {
    let temp_dir = TempDir::new().unwrap();
    let scheduler = Arc::new(MockScheduler::new());
    let agent = agent_with_scheduler(&temp_dir, scheduler.clone());
    let cases = [
        ("invalid.yaml", "yaml-secret-242", "Invalid YAML recipe"),
        ("invalid.json", "\"json-secret-242\"", "Invalid JSON recipe"),
    ];

    for (name, secret, expected) in cases {
        let path = temp_dir.path().join(name);
        std::fs::write(&path, secret).unwrap();
        let message = create_schedule(&agent, &path).await.unwrap_err();
        assert_eq!(message, expected);
        assert!(!message.contains(secret));
    }

    assert!(scheduler.jobs.lock().await.is_empty());
}

#[tokio::test]
async fn rejects_non_regular_recipe_path() {
    let temp_dir = TempDir::new().unwrap();
    let scheduler = Arc::new(MockScheduler::new());
    let agent = agent_with_scheduler(&temp_dir, scheduler.clone());

    let message = create_schedule(&agent, temp_dir.path()).await.unwrap_err();

    assert_eq!(message, "Recipe path must reference a regular file");
    assert!(scheduler.jobs.lock().await.is_empty());
}

#[tokio::test]
async fn rejects_oversized_recipe() {
    let temp_dir = TempDir::new().unwrap();
    let scheduler = Arc::new(MockScheduler::new());
    let agent = agent_with_scheduler(&temp_dir, scheduler.clone());
    let path = temp_dir.path().join("oversized.yaml");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(1_048_577)
        .unwrap();

    let message = create_schedule(&agent, &path).await.unwrap_err();

    assert_eq!(message, "Recipe file exceeds the 1048576 byte limit");
    assert!(scheduler.jobs.lock().await.is_empty());
}

#[tokio::test]
async fn accepts_valid_regular_recipe() {
    let temp_dir = TempDir::new().unwrap();
    let scheduler = Arc::new(MockScheduler::new());
    let agent = agent_with_scheduler(&temp_dir, scheduler.clone());
    let path = temp_dir.path().join("valid.yaml");
    let recipe = b"title: Valid recipe\ndescription: A small recipe\nprompt: Run safely\n";
    std::fs::write(&path, recipe).unwrap();

    create_schedule(&agent, &path).await.unwrap();

    assert_eq!(scheduler.jobs.lock().await.len(), 1);
    assert_eq!(
        scheduler.validated_recipes.lock().await.as_slice(),
        &[recipe.to_vec()]
    );
}
