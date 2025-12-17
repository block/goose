use super::trainer::{TrainingConfig, TrainingProgress, TrainingResult};
use crate::training_data::schema::TrainingExample;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Status of a training job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Priority levels for training jobs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Training job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub backend: TrainingBackend,
    pub config: TrainingConfig,
    pub base_model_path: PathBuf,
    pub training_data_filter: TrainingDataFilter,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub tags: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub retry_count: usize,
    pub max_retries: usize,
}

/// Filter for selecting training data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataFilter {
    pub min_quality_score: Option<f32>,
    pub domain_tags: Option<Vec<String>>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub max_examples: Option<usize>,
    pub exclude_user_ids: Vec<String>,
    pub require_feedback: bool,
}

impl Default for TrainingDataFilter {
    fn default() -> Self {
        Self {
            min_quality_score: Some(0.7),
            domain_tags: None,
            date_range: None,
            max_examples: Some(10000),
            exclude_user_ids: Vec::new(),
            require_feedback: false,
        }
    }
}

/// Resource requirements for training jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_memory_gb: f32,
    pub preferred_device: String, // "cpu", "cuda", "metal"
    pub max_training_time_hours: Option<f32>,
    pub disk_space_gb: f32,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            min_memory_gb: 4.0,
            preferred_device: "cpu".to_string(),
            max_training_time_hours: Some(24.0),
            disk_space_gb: 10.0,
        }
    }
}

/// Training job manager that handles queuing, scheduling, and execution
pub struct TrainingJobManager {
    jobs: Arc<RwLock<HashMap<Uuid, TrainingJob>>>,
    job_queue: Arc<RwLock<VecDeque<Uuid>>>,
    running_jobs: Arc<RwLock<HashMap<Uuid, JoinHandle<Result<TrainingResult>>>>>,
    progress_receivers: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedReceiver<TrainingProgress>>>>,
    trainer_factory: Arc<dyn TrainerFactory>,
    max_concurrent_jobs: usize,
}

/// Factory trait for creating trainers
pub trait TrainerFactory: Send + Sync {
    fn create_trainer(&self, config: &TrainingConfig) -> Result<Box<dyn TrainerExecutor>>;
}

/// Executor trait for running training jobs
#[async_trait::async_trait]
pub trait TrainerExecutor: Send + Sync {
    async fn execute_job(
        &self,
        job: &TrainingJob,
        training_examples: Vec<TrainingExample>,
        progress_sender: mpsc::UnboundedSender<TrainingProgress>,
    ) -> Result<TrainingResult>;
}

impl TrainingJobManager {
    pub fn new(
        trainer_factory: Arc<dyn TrainerFactory>,
        max_concurrent_jobs: usize,
    ) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            job_queue: Arc::new(RwLock::new(VecDeque::new())),
            running_jobs: Arc::new(RwLock::new(HashMap::new())),
            progress_receivers: Arc::new(RwLock::new(HashMap::new())),
            trainer_factory,
            max_concurrent_jobs,
        }
    }

    /// Submit a new training job
    pub async fn submit_job(&self, mut job: TrainingJob) -> Result<Uuid> {
        job.id = Uuid::new_v4();
        job.status = JobStatus::Queued;
        job.created_at = Utc::now();

        info!("Submitting training job: {} ({})", job.name, job.id);

        // Validate job
        self.validate_job(&job).await?;

        // Store job
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job.id, job.clone());
        }

        // Add to queue based on priority
        {
            let jobs_read = self.jobs.read().await;
            let mut queue = self.job_queue.write().await;
            let position = queue.iter().position(|&id| {
                if let Some(queued_job) = jobs_read.get(&id) {
                    queued_job.priority < job.priority
                } else {
                    false
                }
            }).unwrap_or(queue.len());
            
            queue.insert(position, job.id);
        }

        // Try to start job if resources available
        self.try_start_next_job().await?;

        Ok(job.id)
    }

    /// Submit a new training job with pre-loaded training data
    pub async fn submit_job_with_data(
        &self,
        mut job: TrainingJob,
        training_examples: Vec<TrainingExample>,
    ) -> Result<Uuid> {
        job.id = Uuid::new_v4();
        job.status = JobStatus::Queued;
        job.created_at = Utc::now();

        info!(
            "Submitting training job with {} examples: {} ({})",
            training_examples.len(),
            job.name,
            job.id
        );

        // Validate job
        self.validate_job(&job).await?;

        // Store job
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job.id, job.clone());
        }

        // Add to queue based on priority
        {
            let jobs_read = self.jobs.read().await;
            let mut queue = self.job_queue.write().await;
            let position = queue
                .iter()
                .position(|&id| {
                    if let Some(queued_job) = jobs_read.get(&id) {
                        queued_job.priority < job.priority
                    } else {
                        false
                    }
                })
                .unwrap_or(queue.len());

            queue.insert(position, job.id);
        }

        // Start the job immediately with the provided data
        self.start_job_with_data(job.id, training_examples).await?;

        Ok(job.id)
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: Uuid) -> Option<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).map(|job| job.status.clone())
    }

    /// Get job details
    pub async fn get_job(&self, job_id: Uuid) -> Option<TrainingJob> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).cloned()
    }

    /// List all jobs with optional filtering
    pub async fn list_jobs(
        &self,
        status_filter: Option<JobStatus>,
        limit: Option<usize>,
    ) -> Vec<TrainingJob> {
        let jobs = self.jobs.read().await;
        let mut filtered_jobs: Vec<TrainingJob> = jobs
            .values()
            .filter(|job| {
                if let Some(ref status) = status_filter {
                    &job.status == status
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        filtered_jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = limit {
            filtered_jobs.truncate(limit);
        }

        filtered_jobs
    }

    /// Cancel a job
    pub async fn cancel_job(&self, job_id: Uuid) -> Result<()> {
        info!("Cancelling job: {}", job_id);

        // Update job status
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                match job.status {
                    JobStatus::Queued => {
                        job.status = JobStatus::Cancelled;
                        // Remove from queue
                        let mut queue = self.job_queue.write().await;
                        queue.retain(|&id| id != job_id);
                    }
                    JobStatus::Running => {
                        job.status = JobStatus::Cancelled;
                        // Cancel running task
                        let mut running_jobs = self.running_jobs.write().await;
                        if let Some(handle) = running_jobs.remove(&job_id) {
                            handle.abort();
                        }
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Cannot cancel job in status: {:?}", job.status));
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Job not found: {}", job_id));
            }
        }

        // Try to start next job
        self.try_start_next_job().await?;

        Ok(())
    }

    /// Retry a failed job
    pub async fn retry_job(&self, job_id: Uuid) -> Result<()> {
        info!("Retrying job: {}", job_id);

        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                if job.status != JobStatus::Failed {
                    return Err(anyhow::anyhow!("Can only retry failed jobs"));
                }

                if job.retry_count >= job.max_retries {
                    return Err(anyhow::anyhow!("Maximum retry attempts exceeded"));
                }

                job.retry_count += 1;
                job.status = JobStatus::Queued;
                job.started_at = None;
                job.completed_at = None;

                // Add back to queue
                let mut queue = self.job_queue.write().await;
                queue.push_back(job_id);
            } else {
                return Err(anyhow::anyhow!("Job not found: {}", job_id));
            }
        }

        self.try_start_next_job().await?;
        Ok(())
    }

    /// Get training progress for a job
    pub async fn get_progress(&self, job_id: Uuid) -> Option<Vec<TrainingProgress>> {
        let mut receivers = self.progress_receivers.write().await;
        if let Some(receiver) = receivers.get_mut(&job_id) {
            let mut progress_updates = Vec::new();
            
            // Collect all available progress updates
            while let Ok(progress) = receiver.try_recv() {
                progress_updates.push(progress);
            }
            
            if !progress_updates.is_empty() {
                Some(progress_updates)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get system statistics
    pub async fn get_stats(&self) -> JobManagerStats {
        let jobs = self.jobs.read().await;
        let queue = self.job_queue.read().await;
        let running_jobs = self.running_jobs.read().await;

        let mut stats_by_status = HashMap::new();
        for job in jobs.values() {
            *stats_by_status.entry(job.status.clone()).or_insert(0) += 1;
        }

        JobManagerStats {
            total_jobs: jobs.len(),
            queued_jobs: queue.len(),
            running_jobs: running_jobs.len(),
            max_concurrent_jobs: self.max_concurrent_jobs,
            stats_by_status,
        }
    }

    async fn try_start_next_job(&self) -> Result<()> {
        let running_count = {
            let running_jobs = self.running_jobs.read().await;
            running_jobs.len()
        };

        if running_count >= self.max_concurrent_jobs {
            debug!("Maximum concurrent jobs reached: {}", running_count);
            return Ok(());
        }

        let next_job_id = {
            let mut queue = self.job_queue.write().await;
            queue.pop_front()
        };

        if let Some(job_id) = next_job_id {
            self.start_job(job_id).await?;
        }

        Ok(())
    }

    async fn start_job(&self, job_id: Uuid) -> Result<()> {
        info!("Starting training job: {}", job_id);

        let job = {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Running;
                job.started_at = Some(Utc::now());
                job.clone()
            } else {
                return Err(anyhow::anyhow!("Job not found: {}", job_id));
            }
        };

        // Create progress channel
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
        {
            let mut receivers = self.progress_receivers.write().await;
            receivers.insert(job_id, progress_receiver);
        }

        // Create trainer
        let trainer = self.trainer_factory.create_trainer(&job.config)?;

        // Load training data
        let training_examples = self.load_training_data(&job.training_data_filter).await?;

        info!("Loaded {} training examples for job {}", training_examples.len(), job_id);

        // Start training task
        let jobs_clone = self.jobs.clone();
        let handle = tokio::spawn(async move {
            let result = trainer.execute_job(&job, training_examples, progress_sender).await;
            
            // Update job status
            {
                let mut jobs = jobs_clone.write().await;
                if let Some(mut job) = jobs.get_mut(&job_id) {
                    job.completed_at = Some(Utc::now());
                    job.status = if result.is_ok() {
                        JobStatus::Completed
                    } else {
                        JobStatus::Failed
                    };
                }
            }

            result
        });

        // Store handle
        {
            let mut running_jobs = self.running_jobs.write().await;
            running_jobs.insert(job_id, handle);
        }

        Ok(())
    }

    async fn start_job_with_data(
        &self,
        job_id: Uuid,
        training_examples: Vec<TrainingExample>,
    ) -> Result<()> {
        info!(
            "Starting training job with {} pre-loaded examples: {}",
            training_examples.len(),
            job_id
        );

        let job = {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Running;
                job.started_at = Some(Utc::now());
                job.clone()
            } else {
                return Err(anyhow::anyhow!("Job not found: {}", job_id));
            }
        };

        // Create progress channel
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
        {
            let mut receivers = self.progress_receivers.write().await;
            receivers.insert(job_id, progress_receiver);
        }

        // Create trainer
        let trainer = self.trainer_factory.create_trainer(&job.config)?;

        // Start training task with provided data
        let jobs_clone = self.jobs.clone();
        let handle = tokio::spawn(async move {
            info!("Executing training job {}", job_id);
            let result = trainer
                .execute_job(&job, training_examples, progress_sender)
                .await;

            // Update job status and log result
            {
                let mut jobs = jobs_clone.write().await;
                if let Some(mut job) = jobs.get_mut(&job_id) {
                    job.completed_at = Some(Utc::now());
                    match &result {
                        Ok(_) => {
                            info!("Training job {} completed successfully", job_id);
                            job.status = JobStatus::Completed;
                        }
                        Err(e) => {
                            error!("Training job {} failed: {}", job_id, e);
                            job.status = JobStatus::Failed;
                        }
                    }
                }
            }

            result
        });

        // Store handle
        {
            let mut running_jobs = self.running_jobs.write().await;
            running_jobs.insert(job_id, handle);
        }

        Ok(())
    }

    async fn validate_job(&self, job: &TrainingJob) -> Result<()> {
        // Validate base model path
        // For Axolotl backend we allow non-filesystem identifiers (e.g., Hugging Face repo IDs or Ollama tags like "qwen2.5:7b-instruct")
        if !matches!(job.backend, TrainingBackend::Axolotl) {
            if !job.base_model_path.exists() {
                return Err(anyhow::anyhow!("Base model path does not exist: {:?}", job.base_model_path));
            }
        }

        // Validate training configuration
        if job.config.batch_size == 0 {
            return Err(anyhow::anyhow!("Batch size must be greater than 0"));
        }

        if job.config.num_epochs == 0 {
            return Err(anyhow::anyhow!("Number of epochs must be greater than 0"));
        }

        if job.config.learning_rate <= 0.0 {
            return Err(anyhow::anyhow!("Learning rate must be greater than 0"));
        }

        // Validate resource requirements
        if job.resource_requirements.min_memory_gb <= 0.0 {
            return Err(anyhow::anyhow!("Minimum memory requirement must be greater than 0"));
        }

        Ok(())
    }

    async fn load_training_data(&self, filter: &TrainingDataFilter) -> Result<Vec<TrainingExample>> {
        // This would integrate with the training data storage system
        // For now, return empty vector
        debug!("Loading training data with filter: {:?}", filter);
        Ok(Vec::new())
    }
}

/// Statistics about the job manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobManagerStats {
    pub total_jobs: usize,
    pub queued_jobs: usize,
    pub running_jobs: usize,
    pub max_concurrent_jobs: usize,
    pub stats_by_status: HashMap<JobStatus, usize>,
}

/// Builder for creating training jobs
pub struct TrainingJobBuilder {
    job: TrainingJob,
}

impl TrainingJobBuilder {
    pub fn new(name: String, base_model_path: PathBuf) -> Self {
        Self {
            job: TrainingJob {
                id: Uuid::new_v4(),
                name,
                description: None,
                status: JobStatus::Queued,
                priority: JobPriority::Normal,
                backend: TrainingBackend::Axolotl,
                config: TrainingConfig::default(),
                base_model_path,
                training_data_filter: TrainingDataFilter::default(),
                created_at: Utc::now(),
                started_at: None,
                completed_at: None,
                created_by: "system".to_string(),
                tags: Vec::new(),
                resource_requirements: ResourceRequirements::default(),
                retry_count: 0,
                max_retries: 3,
            },
        }
    }

    pub fn description(mut self, description: String) -> Self {
        self.job.description = Some(description);
        self
    }

    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.job.priority = priority;
        self
    }

    pub fn config(mut self, config: TrainingConfig) -> Self {
        self.job.config = config;
        self
    }

    pub fn training_data_filter(mut self, filter: TrainingDataFilter) -> Self {
        self.job.training_data_filter = filter;
        self
    }

    pub fn created_by(mut self, created_by: String) -> Self {
        self.job.created_by = created_by;
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.job.tags = tags;
        self
    }

    pub fn resource_requirements(mut self, requirements: ResourceRequirements) -> Self {
        self.job.resource_requirements = requirements;
        self
    }

    pub fn max_retries(mut self, max_retries: usize) -> Self {
        self.job.max_retries = max_retries;
        self
    }

    pub fn build(self) -> TrainingJob {
        self.job
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_job_builder() {
        let job = TrainingJobBuilder::new(
            "test-job".to_string(),
            PathBuf::from("/tmp/model"),
        )
        .description("Test job description".to_string())
        .priority(JobPriority::High)
        .created_by("test-user".to_string())
        .tags(vec!["test".to_string(), "experiment".to_string()])
        .max_retries(5)
        .build();

        assert_eq!(job.name, "test-job");
        assert_eq!(job.description, Some("Test job description".to_string()));
        assert_eq!(job.priority, JobPriority::High);
        assert_eq!(job.created_by, "test-user");
        assert_eq!(job.tags, vec!["test", "experiment"]);
        assert_eq!(job.max_retries, 5);
    }

    #[test]
    fn test_training_data_filter_default() {
        let filter = TrainingDataFilter::default();
        assert_eq!(filter.min_quality_score, Some(0.7));
        assert_eq!(filter.max_examples, Some(10000));
        assert!(!filter.require_feedback);
    }

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Critical > JobPriority::High);
        assert!(JobPriority::High > JobPriority::Normal);
        assert!(JobPriority::Normal > JobPriority::Low);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrainingBackend {
    RustLoRA,
    Axolotl,
}
