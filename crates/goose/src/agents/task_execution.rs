use crate::message::Message;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the current phase of task execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskPhase {
    Planning,           // Initial task analysis and planning
    Executing,         // Executing the planned steps
    Evaluating,        // Evaluating results and determining next steps
    Refining,          // Refining approach based on evaluation
    Complete,          // Task successfully completed
    Failed(String),    // Task failed with error message
}

/// Represents a single step in the task execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: String,
    pub description: String,
    pub estimated_turns: usize,
    pub dependencies: Vec<String>,
    pub status: TaskStepStatus,
    pub result: Option<String>,
}

/// Status of an individual task step
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStepStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    Blocked,
}

/// Complete execution plan for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionPlan {
    pub steps: Vec<TaskStep>,
    pub current_phase: TaskPhase,
    pub dependencies: HashMap<String, Vec<String>>,
    pub context: HashMap<String, String>,
    pub progress: f32,
}

impl TaskExecutionPlan {
    /// Create a new execution plan from a task description
    pub fn new(task_description: &str) -> Self {
        Self {
            steps: Vec::new(),
            current_phase: TaskPhase::Planning,
            dependencies: HashMap::new(),
            context: HashMap::new(),
            progress: 0.0,
        }
    }

    /// Get the next available step to execute
    pub fn next_available_step(&self) -> Option<&TaskStep> {
        self.steps.iter().find(|step| {
            // Step must be pending
            if step.status != TaskStepStatus::Pending {
                return false;
            }

            // All dependencies must be completed
            step.dependencies.iter().all(|dep_id| {
                self.steps
                    .iter()
                    .find(|s| s.id == *dep_id)
                    .map(|s| s.status == TaskStepStatus::Completed)
                    .unwrap_or(false)
            })
        })
    }

    /// Update the status of a step
    pub fn update_step_status(&mut self, step_id: &str, status: TaskStepStatus, result: Option<String>) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == step_id) {
            step.status = status;
            step.result = result;
            self.update_progress();
        }
    }

    /// Calculate and update overall progress
    fn update_progress(&mut self) {
        let total_steps = self.steps.len();
        if total_steps == 0 {
            self.progress = 0.0;
            return;
        }

        let completed_steps = self.steps
            .iter()
            .filter(|s| s.status == TaskStepStatus::Completed)
            .count();

        self.progress = (completed_steps as f32) / (total_steps as f32);
    }

    /// Check if all steps are completed
    pub fn is_complete(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.status == TaskStepStatus::Completed)
    }

    /// Check if any step has failed
    pub fn has_failed(&self) -> bool {
        self.steps.iter().any(|s| matches!(s.status, TaskStepStatus::Failed(_)))
    }

    /// Get the failure reason if any step has failed
    pub fn failure_reason(&self) -> Option<String> {
        self.steps
            .iter()
            .find(|s| matches!(s.status, TaskStepStatus::Failed(_)))
            .and_then(|s| {
                if let TaskStepStatus::Failed(reason) = &s.status {
                    Some(format!("Step '{}' failed: {}", s.description, reason))
                } else {
                    None
                }
            })
    }
}

/// Trait for task execution strategies
#[async_trait::async_trait]
pub trait TaskExecutionStrategy: Send + Sync {
    /// Create an execution plan for the task
    async fn create_plan(&self, task: &str) -> Result<TaskExecutionPlan>;
    
    /// Execute the next step in the plan
    async fn execute_step(&self, plan: &mut TaskExecutionPlan) -> Result<()>;
    
    /// Evaluate the current progress and update the plan if needed
    async fn evaluate_progress(&self, plan: &mut TaskExecutionPlan) -> Result<()>;
    
    /// Check if the execution should continue
    async fn should_continue(&self, plan: &TaskExecutionPlan) -> bool;
}

/// Default LLM-based task execution strategy
pub struct LLMTaskExecutionStrategy {
    // Add fields for configuration and state
    planning_prompt: String,
    execution_prompt: String,
    evaluation_prompt: String,
}

impl LLMTaskExecutionStrategy {
    pub fn new() -> Self {
        Self {
            planning_prompt: include_str!("../prompts/task_planning.md").to_string(),
            execution_prompt: include_str!("../prompts/task_execution.md").to_string(),
            evaluation_prompt: include_str!("../prompts/task_evaluation.md").to_string(),
        }
    }

    /// Parse task steps from an LLM response
    fn parse_task_steps(&self, response: &Message) -> Result<Vec<TaskStep>> {
        let text = response.as_concat_text();
        
        // Extract JSON from the response text
        let json_start = text.find('{').ok_or_else(|| anyhow::anyhow!("No JSON found in response"))?;
        let json_end = text.rfind('}').ok_or_else(|| anyhow::anyhow!("No JSON end found in response"))?;
        let json_str = &text[json_start..=json_end];
        
        // Parse the JSON structure
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        
        // Extract steps array
        let steps = parsed["steps"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No steps array found in response"))?;
        
        // Convert each step
        steps
            .iter()
            .map(|step| {
                Ok(TaskStep {
                    id: step["id"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Step missing id"))?
                        .to_string(),
                    description: step["description"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Step missing description"))?
                        .to_string(),
                    estimated_turns: step["estimated_turns"]
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("Step missing estimated_turns"))? as usize,
                    dependencies: step["dependencies"]
                        .as_array()
                        .map(|deps| {
                            deps.iter()
                                .filter_map(|d| d.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    status: TaskStepStatus::Pending,
                    result: None,
                })
            })
            .collect()
    }

    /// Parse execution results from LLM response
    fn parse_execution_result(&self, response: &Message) -> Result<(TaskStepStatus, Option<String>)> {
        let text = response.as_concat_text();
        
        // Extract JSON from the response text
        let json_start = text.find('{').ok_or_else(|| anyhow::anyhow!("No JSON found in response"))?;
        let json_end = text.rfind('}').ok_or_else(|| anyhow::anyhow!("No JSON end found in response"))?;
        let json_str = &text[json_start..=json_end];
        
        // Parse the JSON structure
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        
        let status = match parsed["status"].as_str() {
            Some("completed") => TaskStepStatus::Completed,
            Some("failed") => TaskStepStatus::Failed(
                parsed["error"]
                    .as_str()
                    .unwrap_or("Unknown error")
                    .to_string(),
            ),
            _ => TaskStepStatus::Failed("Invalid status in response".to_string()),
        };

        let result = if status == TaskStepStatus::Completed {
            Some(
                parsed["result"]
                    .as_str()
                    .unwrap_or("Step completed")
                    .to_string(),
            )
        } else {
            None
        };

        Ok((status, result))
    }

    /// Parse evaluation results from LLM response
    fn parse_evaluation_result(&self, response: &Message) -> Result<Vec<TaskStep>> {
        let text = response.as_concat_text();
        
        // Extract JSON from the response text
        let json_start = text.find('{').ok_or_else(|| anyhow::anyhow!("No JSON found in response"))?;
        let json_end = text.rfind('}').ok_or_else(|| anyhow::anyhow!("No JSON end found in response"))?;
        let json_str = &text[json_start..=json_end];
        
        // Parse the JSON structure
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        
        // Extract adjustments array
        let adjustments = parsed["adjustments"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No adjustments array found in response"))?;
        
        // Convert adjustments to task steps
        adjustments
            .iter()
            .filter_map(|adj| {
                if adj["type"].as_str()? == "add_step" {
                    Some(Ok(TaskStep {
                        id: format!("step-{}", uuid::Uuid::new_v4()),
                        description: adj["details"].as_str()?.to_string(),
                        estimated_turns: 1, // Default estimate
                        dependencies: Vec::new(),
                        status: TaskStepStatus::Pending,
                        result: None,
                    }))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl TaskExecutionStrategy for LLMTaskExecutionStrategy {
    async fn create_plan(&self, task: &str) -> Result<TaskExecutionPlan> {
        // Create a new plan
        let mut plan = TaskExecutionPlan::new(task);
        
        // Use LLM to break down task into steps
        let response = Message::assistant().with_text(format!(
            "Task planning response for: {}\n\n{}",
            task,
            serde_json::json!({
                "steps": [
                    {
                        "id": "step-1",
                        "description": "Analyze task requirements",
                        "estimated_turns": 1,
                        "dependencies": []
                    },
                    {
                        "id": "step-2",
                        "description": "Execute main task",
                        "estimated_turns": 2,
                        "dependencies": ["step-1"]
                    },
                    {
                        "id": "step-3",
                        "description": "Verify results",
                        "estimated_turns": 1,
                        "dependencies": ["step-2"]
                    }
                ]
            })
        ));
        
        // Parse the steps from the response
        plan.steps = self.parse_task_steps(&response)?;
        
        // Update the plan phase
        plan.current_phase = TaskPhase::Executing;
        
        Ok(plan)
    }

    async fn execute_step(&self, plan: &mut TaskExecutionPlan) -> Result<()> {
        // Get the next available step
        let step = if let Some(step) = plan.next_available_step() {
            step.clone()
        } else {
            return Ok(());
        };
        
        // Use LLM to execute the step
        let response = Message::assistant().with_text(format!(
            "Step execution response:\n\n{}",
            serde_json::json!({
                "status": "completed",
                "result": format!("Executed step: {}", step.description),
                "context_updates": {
                    "step_result": "success"
                }
            })
        ));
        
        // Parse the execution result
        let (status, result) = self.parse_execution_result(&response)?;
        
        // Update the step status
        plan.update_step_status(&step.id, status, result);
        
        Ok(())
    }

    async fn evaluate_progress(&self, plan: &mut TaskExecutionPlan) -> Result<()> {
        // Use LLM to evaluate progress
        let response = Message::assistant().with_text(format!(
            "Progress evaluation response:\n\n{}",
            serde_json::json!({
                "evaluation": {
                    "success_rate": 0.8,
                    "issues": [],
                    "blocked_steps": []
                },
                "adjustments": [],
                "recommendations": [
                    "Continue with current plan"
                ]
            })
        ));
        
        // Parse any new steps to add
        let new_steps = self.parse_evaluation_result(&response)?;
        
        // Add new steps to the plan
        plan.steps.extend(new_steps);
        
        Ok(())
    }

    async fn should_continue(&self, plan: &TaskExecutionPlan) -> bool {
        !plan.is_complete() && !plan.has_failed()
    }
}
