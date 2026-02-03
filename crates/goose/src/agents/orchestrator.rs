//! Agent Orchestrator for coordinating multiple specialist agents
//!
//! The AgentOrchestrator manages complex development workflows by coordinating
//! specialist agents (CodeAgent, TestAgent, DeployAgent) to work together on
//! multi-step tasks with dependencies and handoffs.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::agents::specialists::{SpecialistAgent, SpecialistContext, SpecialistFactory};
use crate::approval::ApprovalPreset;

/// Type of specialist agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    /// Focused on code generation and architecture
    Code,
    /// Focused on testing and quality assurance
    Test,
    /// Focused on deployment and infrastructure
    Deploy,
    /// Focused on documentation and communication
    Docs,
    /// Focused on security analysis and compliance
    Security,
    /// General-purpose agent for coordination
    Coordinator,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Code => write!(f, "code"),
            AgentRole::Test => write!(f, "test"),
            AgentRole::Deploy => write!(f, "deploy"),
            AgentRole::Docs => write!(f, "docs"),
            AgentRole::Security => write!(f, "security"),
            AgentRole::Coordinator => write!(f, "coordinator"),
        }
    }
}

/// Status of a workflow task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is waiting to be started
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed with errors
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task is blocked waiting for dependencies
    Blocked,
    /// Task is being retried after failure
    Retrying,
    /// Task was skipped
    Skipped,
}

/// A workflow task assigned to a specialist agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub role: AgentRole,
    pub status: TaskStatus,
    pub dependencies: Vec<Uuid>,
    pub estimated_duration: Option<Duration>,
    pub actual_duration: Option<Duration>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<TaskResult>,
    pub error: Option<String>,
    pub priority: TaskPriority,
    pub metadata: HashMap<String, String>,
    pub retry_count: u32,
    pub progress_percentage: u8,
}

/// Priority level for workflow tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Result of a completed workflow task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: String,
    pub files_modified: Vec<String>,
    pub artifacts: Vec<String>,
    pub metrics: HashMap<String, serde_json::Value>,
}

/// A complete development workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub tasks: HashMap<Uuid, WorkflowTask>,
    pub task_order: VecDeque<Uuid>,
    pub status: WorkflowStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_estimated_duration: Option<Duration>,
    pub total_actual_duration: Option<Duration>,
    pub success_rate: f64,
}

/// Status of the overall workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    /// Workflow is being planned
    Planning,
    /// Workflow is ready to execute
    Ready,
    /// Workflow is currently executing
    Executing,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
    /// Workflow is paused
    Paused,
}

/// Configuration for agent orchestration - aligned with CLI expectations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum number of workflows that can run concurrently
    pub max_concurrent_workflows: usize,
    /// Maximum number of tasks that can run concurrently across all workflows
    pub max_concurrent_tasks: usize,
    /// Default timeout for individual tasks
    pub task_timeout: Duration,
    /// Number of retry attempts for failed tasks
    pub retry_attempts: u32,
    /// Approval policy for command execution
    pub approval_policy: ApprovalPreset,
    /// Whether to enable parallel task execution within workflows
    pub enable_parallel_execution: bool,
    /// Maximum size of the task queue
    pub task_queue_size: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workflows: 5,
            max_concurrent_tasks: 10,
            task_timeout: Duration::from_secs(3600), // 1 hour
            retry_attempts: 3,
            approval_policy: ApprovalPreset::Safe,
            enable_parallel_execution: true,
            task_queue_size: 100,
        }
    }
}

/// Task override configuration for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOverride {
    pub skip: bool,
    pub timeout: Option<Duration>,
    pub custom_config: HashMap<String, serde_json::Value>,
}

/// The main orchestrator for coordinating multiple agents
pub struct AgentOrchestrator {
    config: OrchestratorConfig,
    specialist_agents: RwLock<HashMap<AgentRole, Box<dyn SpecialistAgent>>>,
    active_workflows: RwLock<HashMap<Uuid, Arc<Mutex<Workflow>>>>,
    task_queue: Mutex<VecDeque<(Uuid, Uuid)>>, // (workflow_id, task_id)
    execution_stats: RwLock<ExecutionStats>,
    running_tasks: RwLock<usize>,
    running_workflows: RwLock<usize>,
}

/// Statistics for workflow execution
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub workflows_started: u64,
    pub workflows_completed: u64,
    pub workflows_failed: u64,
    pub tasks_executed: u64,
    pub tasks_failed: u64,
    pub tasks_retried: u64,
    pub average_workflow_duration: f64,
    pub average_task_duration: f64,
    pub success_rate: f64,
    pub total_agents: usize,
    pub available_agents: usize,
}

/// Agent pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPoolStatistics {
    pub total_agents: usize,
    pub available_agents: usize,
    pub busy_agents: usize,
    pub agents_by_role: HashMap<String, usize>,
}

impl AgentOrchestrator {
    /// Create a new AgentOrchestrator with default configuration (sync version for Default impl)
    pub fn new() -> Self {
        Self {
            config: OrchestratorConfig::default(),
            specialist_agents: RwLock::new(HashMap::new()),
            active_workflows: RwLock::new(HashMap::new()),
            task_queue: Mutex::new(VecDeque::new()),
            execution_stats: RwLock::new(ExecutionStats::default()),
            running_tasks: RwLock::new(0),
            running_workflows: RwLock::new(0),
        }
    }

    /// Create a new AgentOrchestrator with custom configuration (async version for CLI compatibility)
    pub async fn with_config(config: OrchestratorConfig) -> Result<Self> {
        let mut orchestrator = Self {
            config,
            specialist_agents: RwLock::new(HashMap::new()),
            active_workflows: RwLock::new(HashMap::new()),
            task_queue: Mutex::new(VecDeque::new()),
            execution_stats: RwLock::new(ExecutionStats::default()),
            running_tasks: RwLock::new(0),
            running_workflows: RwLock::new(0),
        };

        // Initialize default specialist agents
        orchestrator.initialize_specialist_agents().await?;

        Ok(orchestrator)
    }

    /// Initialize all specialist agents
    async fn initialize_specialist_agents(&mut self) -> Result<()> {
        let agents = SpecialistFactory::create_all()?;
        let mut specialist_agents = self.specialist_agents.write().await;

        for (role, agent) in agents {
            tracing::info!("Initialized {} specialist agent", role);
            specialist_agents.insert(role, agent);
        }

        // Update stats
        let mut stats = self.execution_stats.write().await;
        stats.total_agents = specialist_agents.len();
        stats.available_agents = specialist_agents.len();

        Ok(())
    }

    /// Check if orchestrator is ready
    pub fn is_ready(&self) -> bool {
        true
    }

    /// Get available agent roles
    pub async fn get_available_agent_roles(&self) -> Vec<AgentRole> {
        let agents = self.specialist_agents.read().await;
        agents.keys().cloned().collect()
    }

    /// Get agent pool statistics
    pub async fn get_agent_pool_statistics(&self) -> Result<AgentPoolStatistics> {
        let agents = self.specialist_agents.read().await;
        let running = *self.running_tasks.read().await;

        let mut agents_by_role = HashMap::new();
        for role in agents.keys() {
            agents_by_role.insert(role.to_string(), 1);
        }

        Ok(AgentPoolStatistics {
            total_agents: agents.len(),
            available_agents: agents.len().saturating_sub(running),
            busy_agents: running.min(agents.len()),
            agents_by_role,
        })
    }

    /// Register a specialist agent with the orchestrator
    pub async fn register_specialist(
        &self,
        role: AgentRole,
        agent: Box<dyn SpecialistAgent>,
    ) -> Result<()> {
        let mut agents = self.specialist_agents.write().await;
        agents.insert(role, agent);
        tracing::info!("Registered {} specialist agent with orchestrator", role);
        Ok(())
    }

    /// Get a reference to a specialist agent
    pub async fn get_specialist(&self, role: AgentRole) -> Option<AgentRole> {
        let agents = self.specialist_agents.read().await;
        if agents.contains_key(&role) {
            Some(role)
        } else {
            None
        }
    }

    /// Create a new workflow with tasks
    pub async fn create_workflow(&self, name: String, description: String) -> Result<Uuid> {
        // Check concurrent workflow limit
        let running = *self.running_workflows.read().await;
        if running >= self.config.max_concurrent_workflows {
            return Err(anyhow::anyhow!(
                "Maximum concurrent workflows ({}) reached",
                self.config.max_concurrent_workflows
            ));
        }

        let workflow_id = Uuid::new_v4();
        let workflow = Workflow {
            id: workflow_id,
            name: name.clone(),
            description,
            tasks: HashMap::new(),
            task_order: VecDeque::new(),
            status: WorkflowStatus::Planning,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            total_estimated_duration: None,
            total_actual_duration: None,
            success_rate: 0.0,
        };

        let mut active_workflows = self.active_workflows.write().await;
        active_workflows.insert(workflow_id, Arc::new(Mutex::new(workflow)));

        tracing::info!("Created workflow '{}' with id {}", name, workflow_id);
        Ok(workflow_id)
    }

    /// Add a task to a workflow
    pub async fn add_task(
        &self,
        workflow_id: Uuid,
        name: String,
        description: String,
        role: AgentRole,
        dependencies: Vec<Uuid>,
        priority: TaskPriority,
    ) -> Result<Uuid> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let mut workflow = workflow_arc.lock().await;

        let task_id = Uuid::new_v4();
        let task = WorkflowTask {
            id: task_id,
            name: name.clone(),
            description,
            role,
            status: TaskStatus::Pending,
            dependencies,
            estimated_duration: None,
            actual_duration: None,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            priority,
            metadata: HashMap::new(),
            retry_count: 0,
            progress_percentage: 0,
        };

        workflow.tasks.insert(task_id, task);
        workflow.task_order.push_back(task_id);

        tracing::debug!(
            "Added task '{}' ({}) to workflow {}",
            name,
            task_id,
            workflow_id
        );
        Ok(task_id)
    }

    /// Start execution of a workflow
    pub async fn start_workflow(&self, workflow_id: Uuid) -> Result<()> {
        {
            let active_workflows = self.active_workflows.read().await;
            let workflow_arc = active_workflows
                .get(&workflow_id)
                .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

            let mut workflow = workflow_arc.lock().await;
            workflow.status = WorkflowStatus::Executing;
            workflow.started_at = Some(chrono::Utc::now());
        }

        // Increment running workflows
        {
            let mut running = self.running_workflows.write().await;
            *running += 1;
        }

        // Update stats
        {
            let mut stats = self.execution_stats.write().await;
            stats.workflows_started += 1;
        }

        tracing::info!("Started workflow {} execution", workflow_id);

        // Queue ready tasks for execution
        self.queue_ready_tasks(workflow_id).await?;

        Ok(())
    }

    /// Queue tasks that are ready for execution (dependencies met)
    async fn queue_ready_tasks(&self, workflow_id: Uuid) -> Result<()> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let workflow = workflow_arc.lock().await;

        let mut task_queue = self.task_queue.lock().await;

        // Check queue size limit
        if task_queue.len() >= self.config.task_queue_size {
            tracing::warn!("Task queue full, cannot queue more tasks");
            return Ok(());
        }

        for (task_id, task) in &workflow.tasks {
            if task.status == TaskStatus::Pending && self.dependencies_met(&workflow, task)? {
                // Check if already in queue
                if !task_queue.iter().any(|(_, tid)| tid == task_id) {
                    task_queue.push_back((workflow_id, *task_id));
                    tracing::debug!("Queued task {} for execution", task_id);
                }
            }
        }

        Ok(())
    }

    /// Check if all dependencies for a task are completed
    fn dependencies_met(&self, workflow: &Workflow, task: &WorkflowTask) -> Result<bool> {
        for dep_id in &task.dependencies {
            if let Some(dep_task) = workflow.tasks.get(dep_id) {
                if dep_task.status != TaskStatus::Completed {
                    return Ok(false);
                }
            } else {
                return Err(anyhow::anyhow!("Dependency task {} not found", dep_id));
            }
        }
        Ok(true)
    }

    /// Execute the next available task from the queue
    pub async fn execute_next_task(&self) -> Result<bool> {
        // Check concurrent task limit
        let running = *self.running_tasks.read().await;
        if running >= self.config.max_concurrent_tasks {
            return Ok(false);
        }

        let task_info = {
            let mut queue = self.task_queue.lock().await;
            queue.pop_front()
        };

        if let Some((workflow_id, task_id)) = task_info {
            self.execute_task(workflow_id, task_id).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Execute a specific task using the appropriate specialist agent
    async fn execute_task(&self, workflow_id: Uuid, task_id: Uuid) -> Result<()> {
        // Increment running tasks
        {
            let mut running = self.running_tasks.write().await;
            *running += 1;
        }

        let start_time = chrono::Utc::now();

        // Get task details and update status
        let (role, description, working_dir, metadata) = {
            let active_workflows = self.active_workflows.read().await;
            let workflow_arc = active_workflows
                .get(&workflow_id)
                .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

            let mut workflow = workflow_arc.lock().await;
            let task = workflow
                .tasks
                .get_mut(&task_id)
                .ok_or_else(|| anyhow::anyhow!("Task {} not found", task_id))?;

            task.status = TaskStatus::InProgress;
            task.started_at = Some(start_time);
            task.progress_percentage = 10;

            (
                task.role,
                task.description.clone(),
                ".".to_string(), // Default working directory
                task.metadata.clone(),
            )
        };

        // Execute task with the appropriate specialist agent
        let result = self
            .execute_with_specialist(role, description.clone(), working_dir, metadata)
            .await;

        let end_time = chrono::Utc::now();
        let duration = end_time
            .signed_duration_since(start_time)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(0));

        // Update task with result
        let task_failed = result.is_err();
        {
            let active_workflows = self.active_workflows.read().await;
            let workflow_arc = active_workflows
                .get(&workflow_id)
                .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

            let mut workflow = workflow_arc.lock().await;
            if let Some(task) = workflow.tasks.get_mut(&task_id) {
                task.completed_at = Some(end_time);
                task.actual_duration = Some(duration);
                task.progress_percentage = 100;

                match result {
                    Ok(task_result) => {
                        task.status = TaskStatus::Completed;
                        task.result = Some(task_result);
                        tracing::info!("Task {} completed successfully in {:?}", task_id, duration);
                    }
                    Err(e) => {
                        // Check if we should retry
                        if task.retry_count < self.config.retry_attempts {
                            task.retry_count += 1;
                            task.status = TaskStatus::Retrying;
                            task.error =
                                Some(format!("Attempt {} failed: {}", task.retry_count, e));
                            tracing::warn!(
                                "Task {} failed, will retry (attempt {}/{}): {}",
                                task_id,
                                task.retry_count,
                                self.config.retry_attempts,
                                e
                            );

                            // Re-queue the task
                            let mut queue = self.task_queue.lock().await;
                            queue.push_back((workflow_id, task_id));
                        } else {
                            task.status = TaskStatus::Failed;
                            task.error = Some(e.to_string());
                            tracing::error!(
                                "Task {} failed after {} attempts: {}",
                                task_id,
                                self.config.retry_attempts,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Decrement running tasks
        {
            let mut running = self.running_tasks.write().await;
            *running = running.saturating_sub(1);
        }

        // Update stats
        {
            let mut stats = self.execution_stats.write().await;
            stats.tasks_executed += 1;
            if task_failed {
                stats.tasks_failed += 1;
            }
            // Update average task duration
            let total_tasks = stats.tasks_executed as f64;
            stats.average_task_duration = (stats.average_task_duration * (total_tasks - 1.0)
                + duration.as_secs_f64())
                / total_tasks;
        }

        // Queue any new tasks that became ready
        self.queue_ready_tasks(workflow_id).await?;

        // Check if workflow is complete
        self.update_workflow_status(workflow_id).await?;

        Ok(())
    }

    /// Execute a task using the appropriate specialist agent
    async fn execute_with_specialist(
        &self,
        role: AgentRole,
        task_description: String,
        working_dir: String,
        metadata: HashMap<String, String>,
    ) -> Result<TaskResult> {
        let agents = self.specialist_agents.read().await;
        let agent = agents
            .get(&role)
            .ok_or_else(|| anyhow::anyhow!("No {} specialist agent registered", role))?;

        tracing::info!(
            "Executing task with {} agent: {}",
            agent.name(),
            task_description
        );

        // Build specialist context
        let mut context_metadata = HashMap::new();
        for (k, v) in metadata {
            context_metadata.insert(k, serde_json::Value::String(v));
        }

        let context = SpecialistContext {
            task: task_description,
            working_dir,
            target_files: Vec::new(),
            dependencies: HashMap::new(),
            metadata: context_metadata,
            language: None,
            framework: None,
            environment: None,
        };

        // Check if agent can handle this task
        if !agent.can_handle(&context).await {
            return Err(anyhow::anyhow!(
                "{} agent cannot handle this task",
                agent.name()
            ));
        }

        // Execute the task with timeout
        let timeout_duration = self.config.task_timeout;
        let result = tokio::time::timeout(timeout_duration, agent.execute(context)).await;

        match result {
            Ok(Ok(task_result)) => {
                // Validate the result
                if agent.validate_result(&task_result).await? {
                    Ok(task_result)
                } else {
                    Err(anyhow::anyhow!("Task result validation failed"))
                }
            }
            Ok(Err(e)) => Err(anyhow::anyhow!("Task execution failed: {}", e)),
            Err(_) => Err(anyhow::anyhow!(
                "Task execution timed out after {:?}",
                timeout_duration
            )),
        }
    }

    /// Update workflow status based on task completion
    async fn update_workflow_status(&self, workflow_id: Uuid) -> Result<()> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let mut workflow = workflow_arc.lock().await;

        let mut all_complete = true;
        let mut any_failed = false;
        let mut completed_count = 0;
        let total_count = workflow.tasks.len();

        for task in workflow.tasks.values() {
            match task.status {
                TaskStatus::Pending
                | TaskStatus::InProgress
                | TaskStatus::Blocked
                | TaskStatus::Retrying => {
                    all_complete = false;
                }
                TaskStatus::Failed => {
                    any_failed = true;
                    all_complete = false;
                }
                TaskStatus::Completed => {
                    completed_count += 1;
                }
                TaskStatus::Cancelled | TaskStatus::Skipped => {
                    // These count as complete for workflow purposes
                }
            }
        }

        // Calculate success rate
        if total_count > 0 {
            workflow.success_rate = completed_count as f64 / total_count as f64;
        }

        // Update workflow status if all tasks are done
        if all_complete || any_failed {
            let end_time = chrono::Utc::now();
            workflow.completed_at = Some(end_time);

            if let Some(start_time) = workflow.started_at {
                workflow.total_actual_duration = Some(
                    end_time
                        .signed_duration_since(start_time)
                        .to_std()
                        .unwrap_or_else(|_| Duration::from_secs(0)),
                );
            }

            if any_failed {
                workflow.status = WorkflowStatus::Failed;
                tracing::warn!("Workflow {} failed", workflow_id);

                let mut stats = self.execution_stats.write().await;
                stats.workflows_failed += 1;
            } else {
                workflow.status = WorkflowStatus::Completed;
                tracing::info!("Workflow {} completed successfully", workflow_id);

                let mut stats = self.execution_stats.write().await;
                stats.workflows_completed += 1;

                // Update average workflow duration
                if let Some(duration) = workflow.total_actual_duration {
                    let total_workflows = stats.workflows_completed as f64;
                    stats.average_workflow_duration = (stats.average_workflow_duration
                        * (total_workflows - 1.0)
                        + duration.as_secs_f64())
                        / total_workflows;
                }

                // Update success rate
                if stats.workflows_started > 0 {
                    stats.success_rate =
                        stats.workflows_completed as f64 / stats.workflows_started as f64;
                }
            }

            // Decrement running workflows
            let mut running = self.running_workflows.write().await;
            *running = running.saturating_sub(1);
        }

        Ok(())
    }

    /// Check if a workflow is complete
    pub async fn is_workflow_complete(&self, workflow_id: Uuid) -> Result<bool> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let workflow = workflow_arc.lock().await;

        Ok(matches!(
            workflow.status,
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        ))
    }

    /// Get the current status of a workflow
    pub async fn get_workflow_status(&self, workflow_id: Uuid) -> Result<WorkflowStatus> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let workflow = workflow_arc.lock().await;
        Ok(workflow.status.clone())
    }

    /// Get all tasks in a workflow
    pub async fn get_workflow_tasks(&self, workflow_id: Uuid) -> Result<Vec<WorkflowTask>> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let workflow = workflow_arc.lock().await;
        Ok(workflow.tasks.values().cloned().collect())
    }

    /// Get a specific workflow
    pub async fn get_workflow(&self, workflow_id: Uuid) -> Result<Workflow> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let workflow = workflow_arc.lock().await;
        Ok(workflow.clone())
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> ExecutionStats {
        self.execution_stats.read().await.clone()
    }

    /// Cancel a workflow
    pub async fn cancel_workflow(&self, workflow_id: Uuid) -> Result<()> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let mut workflow = workflow_arc.lock().await;
        workflow.status = WorkflowStatus::Cancelled;
        workflow.completed_at = Some(chrono::Utc::now());

        // Cancel all pending/in-progress tasks
        for task in workflow.tasks.values_mut() {
            if matches!(
                task.status,
                TaskStatus::Pending | TaskStatus::InProgress | TaskStatus::Retrying
            ) {
                task.status = TaskStatus::Cancelled;
            }
        }

        // Decrement running workflows if it was running
        {
            let mut running = self.running_workflows.write().await;
            *running = running.saturating_sub(1);
        }

        tracing::info!("Cancelled workflow {}", workflow_id);
        Ok(())
    }

    /// Pause a workflow
    pub async fn pause_workflow(&self, workflow_id: Uuid) -> Result<()> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let mut workflow = workflow_arc.lock().await;
        if workflow.status == WorkflowStatus::Executing {
            workflow.status = WorkflowStatus::Paused;
            tracing::info!("Paused workflow {}", workflow_id);
        }
        Ok(())
    }

    /// Resume a paused workflow
    pub async fn resume_workflow(&self, workflow_id: Uuid) -> Result<()> {
        let active_workflows = self.active_workflows.read().await;
        let workflow_arc = active_workflows
            .get(&workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow {} not found", workflow_id))?;

        let mut workflow = workflow_arc.lock().await;
        if workflow.status == WorkflowStatus::Paused {
            workflow.status = WorkflowStatus::Executing;
            tracing::info!("Resumed workflow {}", workflow_id);
        }
        drop(workflow);
        drop(active_workflows);

        // Re-queue ready tasks
        self.queue_ready_tasks(workflow_id).await?;
        Ok(())
    }

    /// List all active workflows
    pub async fn list_workflows(&self) -> Vec<(Uuid, WorkflowStatus)> {
        let active_workflows = self.active_workflows.read().await;
        let mut result = Vec::new();

        for (id, workflow_arc) in active_workflows.iter() {
            let workflow = workflow_arc.lock().await;
            result.push((*id, workflow.status.clone()));
        }

        result
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self {
            config: OrchestratorConfig::default(),
            specialist_agents: RwLock::new(HashMap::new()),
            active_workflows: RwLock::new(HashMap::new()),
            task_queue: Mutex::new(VecDeque::new()),
            execution_stats: RwLock::new(ExecutionStats::default()),
            running_tasks: RwLock::new(0),
            running_workflows: RwLock::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_concurrent_workflows, 5);
        assert_eq!(config.max_concurrent_tasks, 10);
        assert_eq!(config.task_timeout, Duration::from_secs(3600));
        assert_eq!(config.retry_attempts, 3);
        assert!(config.enable_parallel_execution);
    }

    #[test]
    fn test_task_status_variants() {
        assert_ne!(TaskStatus::Pending, TaskStatus::InProgress);
        assert_ne!(TaskStatus::Completed, TaskStatus::Failed);
        assert_ne!(TaskStatus::Retrying, TaskStatus::Skipped);
    }

    #[test]
    fn test_workflow_status_variants() {
        assert_ne!(WorkflowStatus::Planning, WorkflowStatus::Executing);
        assert_ne!(WorkflowStatus::Completed, WorkflowStatus::Failed);
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Low < TaskPriority::Medium);
        assert!(TaskPriority::Medium < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Critical);
    }

    #[test]
    fn test_agent_role_display() {
        assert_eq!(format!("{}", AgentRole::Code), "code");
        assert_eq!(format!("{}", AgentRole::Test), "test");
        assert_eq!(format!("{}", AgentRole::Deploy), "deploy");
        assert_eq!(format!("{}", AgentRole::Docs), "docs");
        assert_eq!(format!("{}", AgentRole::Security), "security");
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = AgentOrchestrator::with_config(OrchestratorConfig::default())
            .await
            .unwrap();
        assert!(orchestrator.is_ready());
    }

    #[tokio::test]
    async fn test_workflow_lifecycle() {
        let orchestrator = AgentOrchestrator::with_config(OrchestratorConfig::default())
            .await
            .unwrap();

        // Create workflow
        let workflow_id = orchestrator
            .create_workflow("Test Workflow".to_string(), "A test workflow".to_string())
            .await
            .unwrap();

        // Add task
        let task_id = orchestrator
            .add_task(
                workflow_id,
                "Test Task".to_string(),
                "A test task".to_string(),
                AgentRole::Code,
                vec![],
                TaskPriority::Medium,
            )
            .await
            .unwrap();

        assert_ne!(task_id, Uuid::nil());

        // Get workflow
        let workflow = orchestrator.get_workflow(workflow_id).await.unwrap();
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.tasks.len(), 1);
    }
}
