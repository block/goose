//! WorkflowEngine - Complex development pipeline orchestration
//!
//! The WorkflowEngine provides predefined workflow templates for common development
//! scenarios and coordinates their execution across specialist agents.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::orchestrator::{AgentOrchestrator, AgentRole, TaskPriority, TaskStatus};

/// Predefined workflow template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub name: String,
    pub description: String,
    pub category: WorkflowCategory,
    pub tasks: Vec<TaskTemplate>,
    pub estimated_duration: std::time::Duration,
    pub complexity: WorkflowComplexity,
}

/// Category of workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCategory {
    /// Full application development from scratch
    FullStack,
    /// Microservice development
    Microservice,
    /// Frontend application
    Frontend,
    /// Backend API development
    Backend,
    /// DevOps and infrastructure
    DevOps,
    /// Data processing pipeline
    DataPipeline,
    /// Machine learning workflow
    MachineLearning,
    /// Testing and QA
    Testing,
    /// Documentation and guides
    Documentation,
    /// Security assessment
    Security,
}

/// Workflow complexity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowComplexity {
    Simple,
    Moderate,
    Complex,
    Expert,
}

/// Template for a task within a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub name: String,
    pub description: String,
    pub role: AgentRole,
    pub dependencies: Vec<String>, // Task names this depends on
    pub priority: TaskPriority,
    pub estimated_duration: std::time::Duration,
    pub required_skills: Vec<String>,
    pub validation_criteria: Vec<String>,
}

/// Configuration for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionConfig {
    /// Target working directory
    pub working_dir: String,
    /// Programming language preference
    pub language: Option<String>,
    /// Framework preference
    pub framework: Option<String>,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Custom parameters for the workflow
    pub parameters: HashMap<String, serde_json::Value>,
    /// Override default task configurations
    pub task_overrides: HashMap<String, TaskOverride>,
}

/// Override configuration for specific tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOverride {
    pub skip: bool,
    pub timeout: Option<std::time::Duration>,
    pub custom_config: HashMap<String, serde_json::Value>,
}

/// The main workflow engine
pub struct WorkflowEngine {
    orchestrator: Arc<AgentOrchestrator>,
    templates: RwLock<HashMap<String, WorkflowTemplate>>,
    active_executions: RwLock<HashMap<Uuid, WorkflowExecution>>,
}

/// Active workflow execution state
#[derive(Debug)]
pub struct WorkflowExecution {
    pub workflow_id: Uuid,
    pub template_name: String,
    pub config: WorkflowExecutionConfig,
    pub task_mapping: HashMap<String, Uuid>, // Template task name -> actual task ID
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub status: WorkflowExecutionStatus,
}

/// Status of workflow execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowExecutionStatus {
    Preparing,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl WorkflowEngine {
    /// Create a new WorkflowEngine with orchestrator
    pub async fn new(orchestrator: Arc<AgentOrchestrator>) -> Result<Self> {
        let mut engine = Self {
            orchestrator,
            templates: RwLock::new(HashMap::new()),
            active_executions: RwLock::new(HashMap::new()),
        };

        // Load default templates
        engine.load_default_templates().await?;
        Ok(engine)
    }

    /// Load default workflow templates
    async fn load_default_templates(&mut self) -> Result<()> {
        let mut templates = self.templates.write().await;

        // Full-stack web application workflow
        templates.insert(
            "fullstack_webapp".to_string(),
            WorkflowTemplate {
                name: "Full-Stack Web Application".to_string(),
                description:
                    "Complete web application with frontend, backend, database, and deployment"
                        .to_string(),
                category: WorkflowCategory::FullStack,
                estimated_duration: std::time::Duration::from_secs(14400), // 4 hours
                complexity: WorkflowComplexity::Complex,
                tasks: vec![
                    TaskTemplate {
                        name: "project_setup".to_string(),
                        description: "Initialize project structure and dependencies".to_string(),
                        role: AgentRole::Code,
                        dependencies: vec![],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(600), // 10 min
                        required_skills: vec!["project_structure".to_string()],
                        validation_criteria: vec!["Project files exist".to_string()],
                    },
                    TaskTemplate {
                        name: "backend_api".to_string(),
                        description: "Create REST API with database integration".to_string(),
                        role: AgentRole::Code,
                        dependencies: vec!["project_setup".to_string()],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(3600), // 1 hour
                        required_skills: vec!["api_design", "database"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec![
                            "API endpoints respond".to_string(),
                            "Database connected".to_string(),
                        ],
                    },
                    TaskTemplate {
                        name: "frontend_ui".to_string(),
                        description: "Build user interface and integrate with API".to_string(),
                        role: AgentRole::Code,
                        dependencies: vec!["backend_api".to_string()],
                        priority: TaskPriority::Medium,
                        estimated_duration: std::time::Duration::from_secs(2400), // 40 min
                        required_skills: vec!["ui_development", "api_integration"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec![
                            "UI renders correctly".to_string(),
                            "API calls work".to_string(),
                        ],
                    },
                    TaskTemplate {
                        name: "comprehensive_tests".to_string(),
                        description: "Create unit, integration, and end-to-end tests".to_string(),
                        role: AgentRole::Test,
                        dependencies: vec!["frontend_ui".to_string()],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(1800), // 30 min
                        required_skills: vec!["testing", "automation"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec![
                            "All tests pass".to_string(),
                            "Coverage > 80%".to_string(),
                        ],
                    },
                    TaskTemplate {
                        name: "deployment_setup".to_string(),
                        description: "Configure deployment pipeline and infrastructure".to_string(),
                        role: AgentRole::Deploy,
                        dependencies: vec!["comprehensive_tests".to_string()],
                        priority: TaskPriority::Medium,
                        estimated_duration: std::time::Duration::from_secs(1200), // 20 min
                        required_skills: vec!["deployment", "ci_cd"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec![
                            "Deployment succeeds".to_string(),
                            "Health checks pass".to_string(),
                        ],
                    },
                    TaskTemplate {
                        name: "documentation".to_string(),
                        description: "Generate API documentation and user guides".to_string(),
                        role: AgentRole::Docs,
                        dependencies: vec!["deployment_setup".to_string()],
                        priority: TaskPriority::Low,
                        estimated_duration: std::time::Duration::from_secs(900), // 15 min
                        required_skills: vec!["documentation", "api_docs"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec![
                            "Documentation complete".to_string(),
                            "Examples work".to_string(),
                        ],
                    },
                    TaskTemplate {
                        name: "security_audit".to_string(),
                        description: "Perform security analysis and vulnerability assessment"
                            .to_string(),
                        role: AgentRole::Security,
                        dependencies: vec!["deployment_setup".to_string()],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(1200), // 20 min
                        required_skills: vec!["security", "vulnerability_assessment"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec![
                            "No critical vulnerabilities".to_string(),
                            "Security headers configured".to_string(),
                        ],
                    },
                ],
            },
        );

        // Microservice workflow
        templates.insert(
            "microservice".to_string(),
            WorkflowTemplate {
                name: "Microservice Development".to_string(),
                description: "Single microservice with API, tests, and containerization"
                    .to_string(),
                category: WorkflowCategory::Microservice,
                estimated_duration: std::time::Duration::from_secs(7200), // 2 hours
                complexity: WorkflowComplexity::Moderate,
                tasks: vec![
                    TaskTemplate {
                        name: "service_setup".to_string(),
                        description: "Initialize microservice structure".to_string(),
                        role: AgentRole::Code,
                        dependencies: vec![],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(300),
                        required_skills: vec!["microservice_architecture".to_string()],
                        validation_criteria: vec!["Service structure created".to_string()],
                    },
                    TaskTemplate {
                        name: "api_implementation".to_string(),
                        description: "Implement REST API endpoints".to_string(),
                        role: AgentRole::Code,
                        dependencies: vec!["service_setup".to_string()],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(2400),
                        required_skills: vec!["api_development".to_string()],
                        validation_criteria: vec!["Endpoints functional".to_string()],
                    },
                    TaskTemplate {
                        name: "unit_tests".to_string(),
                        description: "Create comprehensive unit tests".to_string(),
                        role: AgentRole::Test,
                        dependencies: vec!["api_implementation".to_string()],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(1200),
                        required_skills: vec!["unit_testing".to_string()],
                        validation_criteria: vec![
                            "Tests pass".to_string(),
                            "Coverage > 85%".to_string(),
                        ],
                    },
                    TaskTemplate {
                        name: "containerization".to_string(),
                        description: "Create Docker container and deployment config".to_string(),
                        role: AgentRole::Deploy,
                        dependencies: vec!["unit_tests".to_string()],
                        priority: TaskPriority::Medium,
                        estimated_duration: std::time::Duration::from_secs(900),
                        required_skills: vec!["docker", "containerization"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec![
                            "Container builds".to_string(),
                            "Service runs in container".to_string(),
                        ],
                    },
                    TaskTemplate {
                        name: "api_documentation".to_string(),
                        description: "Generate OpenAPI documentation".to_string(),
                        role: AgentRole::Docs,
                        dependencies: vec!["containerization".to_string()],
                        priority: TaskPriority::Medium,
                        estimated_duration: std::time::Duration::from_secs(600),
                        required_skills: vec!["api_documentation".to_string()],
                        validation_criteria: vec!["OpenAPI spec generated".to_string()],
                    },
                ],
            },
        );

        // Testing workflow
        templates.insert(
            "comprehensive_testing".to_string(),
            WorkflowTemplate {
                name: "Comprehensive Testing Suite".to_string(),
                description: "Full testing suite with unit, integration, and E2E tests".to_string(),
                category: WorkflowCategory::Testing,
                estimated_duration: std::time::Duration::from_secs(5400), // 1.5 hours
                complexity: WorkflowComplexity::Moderate,
                tasks: vec![
                    TaskTemplate {
                        name: "test_setup".to_string(),
                        description: "Configure testing framework and environment".to_string(),
                        role: AgentRole::Test,
                        dependencies: vec![],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(600),
                        required_skills: vec!["test_framework_setup".to_string()],
                        validation_criteria: vec!["Test environment ready".to_string()],
                    },
                    TaskTemplate {
                        name: "unit_testing".to_string(),
                        description: "Create comprehensive unit tests".to_string(),
                        role: AgentRole::Test,
                        dependencies: vec!["test_setup".to_string()],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(1800),
                        required_skills: vec!["unit_testing", "mocking"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec!["Unit tests cover all functions".to_string()],
                    },
                    TaskTemplate {
                        name: "integration_testing".to_string(),
                        description: "Test component interactions".to_string(),
                        role: AgentRole::Test,
                        dependencies: vec!["unit_testing".to_string()],
                        priority: TaskPriority::High,
                        estimated_duration: std::time::Duration::from_secs(1200),
                        required_skills: vec!["integration_testing".to_string()],
                        validation_criteria: vec!["Integration points tested".to_string()],
                    },
                    TaskTemplate {
                        name: "e2e_testing".to_string(),
                        description: "End-to-end user workflow testing".to_string(),
                        role: AgentRole::Test,
                        dependencies: vec!["integration_testing".to_string()],
                        priority: TaskPriority::Medium,
                        estimated_duration: std::time::Duration::from_secs(1500),
                        required_skills: vec!["e2e_testing", "automation"]
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                        validation_criteria: vec!["User workflows function".to_string()],
                    },
                    TaskTemplate {
                        name: "performance_testing".to_string(),
                        description: "Load and performance testing".to_string(),
                        role: AgentRole::Test,
                        dependencies: vec!["e2e_testing".to_string()],
                        priority: TaskPriority::Low,
                        estimated_duration: std::time::Duration::from_secs(1200),
                        required_skills: vec!["performance_testing".to_string()],
                        validation_criteria: vec!["Performance metrics acceptable".to_string()],
                    },
                ],
            },
        );

        Ok(())
    }

    /// List available workflow templates
    pub async fn list_templates(&self) -> Vec<WorkflowTemplate> {
        let templates = self.templates.read().await;
        templates.values().cloned().collect()
    }

    /// Get a specific template by name
    pub async fn get_template(&self, name: &str) -> Option<WorkflowTemplate> {
        let templates = self.templates.read().await;
        templates.get(name).cloned()
    }

    /// Execute a workflow template
    pub async fn execute_workflow(
        &self,
        template_name: &str,
        config: WorkflowExecutionConfig,
    ) -> Result<Uuid> {
        let template = self
            .get_template(template_name)
            .await
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        // Create workflow in orchestrator
        let workflow_id = self
            .orchestrator
            .create_workflow(template.name.clone(), template.description.clone())
            .await?;

        let mut task_mapping = HashMap::new();
        let mut dependency_map: HashMap<String, Vec<Uuid>> = HashMap::new();

        // Create tasks in dependency order
        for task_template in &template.tasks {
            // Skip if overridden to skip
            if let Some(override_config) = config.task_overrides.get(&task_template.name) {
                if override_config.skip {
                    continue;
                }
            }

            // Resolve dependencies
            let mut dependencies = Vec::new();
            for dep_name in &task_template.dependencies {
                if let Some(dep_id) = task_mapping.get(dep_name) {
                    dependencies.push(*dep_id);
                }
            }

            let task_id = self
                .orchestrator
                .add_task(
                    workflow_id,
                    task_template.name.clone(),
                    task_template.description.clone(),
                    task_template.role,
                    dependencies.clone(),
                    task_template.priority,
                )
                .await?;

            task_mapping.insert(task_template.name.clone(), task_id);
            dependency_map.insert(task_template.name.clone(), dependencies);
        }

        // Create execution record
        let execution = WorkflowExecution {
            workflow_id,
            template_name: template_name.to_string(),
            config,
            task_mapping,
            started_at: chrono::Utc::now(),
            status: WorkflowExecutionStatus::Preparing,
        };

        // Store execution state
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(workflow_id, execution);
        }

        // Start workflow execution
        self.orchestrator.start_workflow(workflow_id).await?;

        // Update status
        {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(&workflow_id) {
                execution.status = WorkflowExecutionStatus::Running;
            }
        }

        tracing::info!(
            "Started workflow execution: {} ({})",
            template.name,
            workflow_id
        );
        Ok(workflow_id)
    }

    /// Get execution status
    pub async fn get_execution_status(&self, workflow_id: Uuid) -> Option<WorkflowExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.get(&workflow_id).map(|e| e.status.clone())
    }

    /// Cancel workflow execution
    pub async fn cancel_workflow(&self, workflow_id: Uuid) -> Result<()> {
        self.orchestrator.cancel_workflow(workflow_id).await?;

        let mut executions = self.active_executions.write().await;
        if let Some(execution) = executions.get_mut(&workflow_id) {
            execution.status = WorkflowExecutionStatus::Cancelled;
        }

        Ok(())
    }

    /// Register a custom workflow template
    pub async fn register_template(&self, template: WorkflowTemplate) -> Result<()> {
        let mut templates = self.templates.write().await;
        templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Execute tasks in the workflow engine
    pub async fn run_execution_loop(&self) -> Result<()> {
        loop {
            // Execute next available task
            let executed = self.orchestrator.execute_next_task().await?;

            if !executed {
                // No tasks to execute, wait a bit
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            // Update completed workflows
            self.update_workflow_statuses().await?;
        }
    }

    /// Update workflow statuses based on task completion
    async fn update_workflow_statuses(&self) -> Result<()> {
        let mut executions = self.active_executions.write().await;
        let mut completed_workflows = Vec::new();

        for (workflow_id, execution) in executions.iter_mut() {
            if execution.status == WorkflowExecutionStatus::Running
                && self.orchestrator.is_workflow_complete(*workflow_id).await?
            {
                let workflow_status = self.orchestrator.get_workflow_status(*workflow_id).await?;
                execution.status = match workflow_status {
                    super::orchestrator::WorkflowStatus::Completed => {
                        WorkflowExecutionStatus::Completed
                    }
                    super::orchestrator::WorkflowStatus::Failed => WorkflowExecutionStatus::Failed,
                    super::orchestrator::WorkflowStatus::Cancelled => {
                        WorkflowExecutionStatus::Cancelled
                    }
                    _ => WorkflowExecutionStatus::Running,
                };

                if execution.status != WorkflowExecutionStatus::Running {
                    completed_workflows.push(*workflow_id);
                }
            }
        }

        // Log completion
        for workflow_id in completed_workflows {
            if let Some(execution) = executions.get(&workflow_id) {
                tracing::info!(
                    "Workflow {} completed with status: {:?}",
                    execution.template_name,
                    execution.status
                );
            }
        }

        Ok(())
    }
}

impl Default for WorkflowExecutionConfig {
    fn default() -> Self {
        Self {
            working_dir: ".".to_string(),
            language: None,
            framework: None,
            environment: "development".to_string(),
            parameters: HashMap::new(),
            task_overrides: HashMap::new(),
        }
    }
}

/// Task information for workflow display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTaskInfo {
    pub name: String,
    pub status: TaskStatus,
    pub progress_percentage: u8,
    pub error: Option<String>,
}

/// Workflow result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub status: WorkflowExecutionStatus,
    pub total_duration: Option<Duration>,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub artifacts: Option<Vec<WorkflowArtifact>>,
}

/// Artifact generated by workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowArtifact {
    pub artifact_type: String,
    pub path: String,
}

/// Failure details for workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDetails {
    pub failed_task: Option<String>,
    pub error_message: String,
    pub stack_trace: Option<String>,
}

/// Execution summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub id: Uuid,
    pub template_name: String,
    pub status: WorkflowExecutionStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
}

impl WorkflowEngine {
    /// Get failure details for a workflow
    pub async fn get_failure_details(&self, workflow_id: Uuid) -> Result<FailureDetails> {
        let tasks = self.orchestrator.get_workflow_tasks(workflow_id).await?;

        let failed_task = tasks.iter().find(|t| t.status == TaskStatus::Failed);

        if let Some(task) = failed_task {
            Ok(FailureDetails {
                failed_task: Some(task.name.clone()),
                error_message: task
                    .error
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_string()),
                stack_trace: None,
            })
        } else {
            Ok(FailureDetails {
                failed_task: None,
                error_message: "No failure details available".to_string(),
                stack_trace: None,
            })
        }
    }

    /// Get workflow tasks with current status
    pub async fn get_workflow_tasks(&self, workflow_id: Uuid) -> Result<Vec<WorkflowTaskInfo>> {
        let tasks = self.orchestrator.get_workflow_tasks(workflow_id).await?;

        Ok(tasks
            .into_iter()
            .map(|t| WorkflowTaskInfo {
                name: t.name,
                status: t.status,
                progress_percentage: t.progress_percentage,
                error: t.error,
            })
            .collect())
    }

    /// Check if workflow is complete
    pub async fn is_complete(&self, workflow_id: Uuid) -> Result<bool> {
        self.orchestrator.is_workflow_complete(workflow_id).await
    }

    /// Get workflow result
    pub async fn get_workflow_result(&self, workflow_id: Uuid) -> Result<WorkflowResult> {
        let workflow = self.orchestrator.get_workflow(workflow_id).await?;
        let tasks = self.orchestrator.get_workflow_tasks(workflow_id).await?;

        let completed_tasks = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed_tasks = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();

        // Collect artifacts from completed tasks
        let artifacts: Vec<WorkflowArtifact> = tasks
            .iter()
            .filter_map(|t| t.result.as_ref())
            .flat_map(|r| {
                r.artifacts.iter().map(|a| WorkflowArtifact {
                    artifact_type: "file".to_string(),
                    path: a.clone(),
                })
            })
            .collect();

        let status = {
            let executions = self.active_executions.read().await;
            executions
                .get(&workflow_id)
                .map(|e| e.status.clone())
                .unwrap_or_else(|| match workflow.status {
                    super::orchestrator::WorkflowStatus::Completed => {
                        WorkflowExecutionStatus::Completed
                    }
                    super::orchestrator::WorkflowStatus::Failed => WorkflowExecutionStatus::Failed,
                    super::orchestrator::WorkflowStatus::Cancelled => {
                        WorkflowExecutionStatus::Cancelled
                    }
                    super::orchestrator::WorkflowStatus::Paused => WorkflowExecutionStatus::Paused,
                    _ => WorkflowExecutionStatus::Running,
                })
        };

        Ok(WorkflowResult {
            status,
            total_duration: workflow.total_actual_duration,
            completed_tasks,
            failed_tasks,
            artifacts: if artifacts.is_empty() {
                None
            } else {
                Some(artifacts)
            },
        })
    }

    /// List workflow executions
    pub async fn list_executions(&self, limit: Option<usize>) -> Result<Vec<ExecutionSummary>> {
        let executions = self.active_executions.read().await;

        let mut summaries: Vec<ExecutionSummary> = Vec::new();

        for (workflow_id, execution) in executions.iter() {
            let tasks = self
                .orchestrator
                .get_workflow_tasks(*workflow_id)
                .await
                .unwrap_or_default();
            let completed_tasks = tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Completed)
                .count();
            let failed_tasks = tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Failed)
                .count();

            let workflow = self.orchestrator.get_workflow(*workflow_id).await.ok();
            let end_time = workflow.as_ref().and_then(|w| w.completed_at);

            summaries.push(ExecutionSummary {
                id: *workflow_id,
                template_name: execution.template_name.clone(),
                status: execution.status.clone(),
                start_time: execution.started_at,
                end_time,
                completed_tasks,
                failed_tasks,
            });
        }

        // Sort by start time descending
        summaries.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        // Apply limit
        if let Some(max) = limit {
            summaries.truncate(max);
        }

        Ok(summaries)
    }

    /// Get execution statistics
    pub async fn get_execution_statistics(&self) -> Result<ExecutionStatistics> {
        let stats = self.orchestrator.get_stats().await;

        Ok(ExecutionStatistics {
            total_workflows: stats.workflows_started,
            completed_workflows: stats.workflows_completed,
            failed_workflows: stats.workflows_failed,
            average_duration: Duration::from_secs_f64(stats.average_workflow_duration),
            success_rate: stats.success_rate,
        })
    }
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    pub total_workflows: u64,
    pub completed_workflows: u64,
    pub failed_workflows: u64,
    pub average_duration: Duration,
    pub success_rate: f64,
}
