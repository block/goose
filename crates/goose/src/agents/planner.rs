//! Planning system for structured agent execution
//!
//! This module provides planning capabilities that allow the agent to create
//! explicit plans before executing tasks. Plans consist of steps with dependencies,
//! tool hints, and validation criteria.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    /// Plan is being created
    #[default]
    Creating,
    /// Plan is ready for execution
    Ready,
    /// Plan is currently being executed
    InProgress,
    /// Plan completed successfully
    Completed,
    /// Plan failed
    Failed,
    /// Plan was cancelled
    Cancelled,
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanStatus::Creating => write!(f, "creating"),
            PlanStatus::Ready => write!(f, "ready"),
            PlanStatus::InProgress => write!(f, "in_progress"),
            PlanStatus::Completed => write!(f, "completed"),
            PlanStatus::Failed => write!(f, "failed"),
            PlanStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Status of a plan step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    /// Step is pending execution
    #[default]
    Pending,
    /// Step is currently being executed
    InProgress,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Step was skipped
    Skipped,
}

/// A single step in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Unique identifier for this step
    pub id: usize,
    /// Human-readable description of what this step does
    pub description: String,
    /// Suggested tools to use for this step
    #[serde(default)]
    pub tool_hints: Vec<String>,
    /// How to verify this step completed successfully
    pub validation: Option<String>,
    /// IDs of steps that must complete before this one
    #[serde(default)]
    pub dependencies: Vec<usize>,
    /// Current status of this step
    #[serde(default)]
    pub status: StepStatus,
    /// Output or result from this step
    pub output: Option<String>,
    /// Error message if step failed
    pub error: Option<String>,
}

impl PlanStep {
    pub fn new(id: usize, description: impl Into<String>) -> Self {
        Self {
            id,
            description: description.into(),
            tool_hints: Vec::new(),
            validation: None,
            dependencies: Vec::new(),
            status: StepStatus::Pending,
            output: None,
            error: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tool_hints = tools;
        self
    }

    pub fn with_validation(mut self, validation: impl Into<String>) -> Self {
        self.validation = Some(validation.into());
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<usize>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn is_ready(&self, completed_steps: &[usize]) -> bool {
        self.status == StepStatus::Pending
            && self
                .dependencies
                .iter()
                .all(|dep| completed_steps.contains(dep))
    }

    pub fn mark_in_progress(&mut self) {
        self.status = StepStatus::InProgress;
    }

    pub fn mark_completed(&mut self, output: Option<String>) {
        self.status = StepStatus::Completed;
        self.output = output;
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = StepStatus::Failed;
        self.error = Some(error.into());
    }

    pub fn mark_skipped(&mut self) {
        self.status = StepStatus::Skipped;
    }
}

/// A complete plan for executing a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// The original goal/task this plan addresses
    pub goal: String,
    /// Steps to execute
    pub steps: Vec<PlanStep>,
    /// Index of the current step being executed
    pub current_step: usize,
    /// Overall plan status
    pub status: PlanStatus,
    /// Additional context or notes about the plan
    pub notes: Option<String>,
    /// Metadata about the plan
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Plan {
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            steps: Vec::new(),
            current_step: 0,
            status: PlanStatus::Creating,
            notes: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_steps(mut self, steps: Vec<PlanStep>) -> Self {
        self.steps = steps;
        self
    }

    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }

    pub fn mark_ready(&mut self) {
        self.status = PlanStatus::Ready;
    }

    pub fn mark_in_progress(&mut self) {
        self.status = PlanStatus::InProgress;
    }

    pub fn mark_completed(&mut self) {
        self.status = PlanStatus::Completed;
    }

    pub fn mark_failed(&mut self) {
        self.status = PlanStatus::Failed;
    }

    /// Get the current step being executed
    pub fn current(&self) -> Option<&PlanStep> {
        self.steps.get(self.current_step)
    }

    /// Get mutable reference to current step
    pub fn current_mut(&mut self) -> Option<&mut PlanStep> {
        self.steps.get_mut(self.current_step)
    }

    /// Get IDs of all completed steps
    pub fn completed_step_ids(&self) -> Vec<usize> {
        self.steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .map(|s| s.id)
            .collect()
    }

    /// Get the next step that is ready to execute
    pub fn next_ready_step(&self) -> Option<usize> {
        let completed = self.completed_step_ids();
        self.steps.iter().position(|s| s.is_ready(&completed))
    }

    /// Advance to the next ready step
    pub fn advance(&mut self) -> bool {
        if let Some(next_idx) = self.next_ready_step() {
            self.current_step = next_idx;
            true
        } else {
            false
        }
    }

    /// Check if all steps are completed
    pub fn is_complete(&self) -> bool {
        self.steps
            .iter()
            .all(|s| s.status == StepStatus::Completed || s.status == StepStatus::Skipped)
    }

    /// Check if any step has failed
    pub fn has_failures(&self) -> bool {
        self.steps.iter().any(|s| s.status == StepStatus::Failed)
    }

    /// Get remaining steps (pending or in progress)
    pub fn remaining_steps(&self) -> Vec<&PlanStep> {
        self.steps
            .iter()
            .filter(|s| s.status == StepStatus::Pending || s.status == StepStatus::InProgress)
            .collect()
    }

    /// Get a summary of plan progress
    pub fn progress_summary(&self) -> String {
        let total = self.steps.len();
        let completed = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let failed = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Failed)
            .count();
        let skipped = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Skipped)
            .count();

        format!(
            "Progress: {}/{} completed, {} failed, {} skipped",
            completed, total, failed, skipped
        )
    }

    /// Format the plan for display to user
    pub fn format_for_display(&self) -> String {
        let mut output = format!("## Plan: {}\n\n", self.goal);

        if let Some(notes) = &self.notes {
            output.push_str(&format!("*{}*\n\n", notes));
        }

        output.push_str("### Steps:\n\n");

        for (idx, step) in self.steps.iter().enumerate() {
            let status_icon = match step.status {
                StepStatus::Pending => "⬜",
                StepStatus::InProgress => "🔄",
                StepStatus::Completed => "✅",
                StepStatus::Failed => "❌",
                StepStatus::Skipped => "⏭️",
            };

            let current_marker = if idx == self.current_step {
                " 👈"
            } else {
                ""
            };

            output.push_str(&format!(
                "{}. {} {}{}\n",
                idx + 1,
                status_icon,
                step.description,
                current_marker
            ));

            if !step.tool_hints.is_empty() {
                output.push_str(&format!("   Tools: {}\n", step.tool_hints.join(", ")));
            }

            if let Some(validation) = &step.validation {
                output.push_str(&format!("   Verify: {}\n", validation));
            }

            if !step.dependencies.is_empty() {
                let deps: Vec<String> = step
                    .dependencies
                    .iter()
                    .map(|d| format!("#{}", d + 1))
                    .collect();
                output.push_str(&format!("   After: {}\n", deps.join(", ")));
            }

            output.push('\n');
        }

        output.push_str(&format!("\n{}\n", self.progress_summary()));

        output
    }

    /// Format the plan as context for the LLM
    pub fn format_for_llm(&self) -> String {
        let current = self.current();
        let remaining = self.remaining_steps();

        let mut context = format!(
            "CURRENT PLAN:\nGoal: {}\nStatus: {}\n\n",
            self.goal, self.status
        );

        if let Some(step) = current {
            context.push_str(&format!(
                "CURRENT STEP ({}/{}):\n{}\n",
                self.current_step + 1,
                self.steps.len(),
                step.description
            ));

            if !step.tool_hints.is_empty() {
                context.push_str(&format!(
                    "Suggested tools: {}\n",
                    step.tool_hints.join(", ")
                ));
            }

            if let Some(validation) = &step.validation {
                context.push_str(&format!("Success criteria: {}\n", validation));
            }
        }

        if remaining.len() > 1 {
            context.push_str("\nREMAINING STEPS:\n");
            for step in remaining.iter().skip(1).take(3) {
                context.push_str(&format!("- {}\n", step.description));
            }
            if remaining.len() > 4 {
                context.push_str(&format!("... and {} more steps\n", remaining.len() - 4));
            }
        }

        context
    }
}

/// Context provided to the planner for creating a plan
#[derive(Debug, Clone)]
pub struct PlanContext {
    /// The task/goal to plan for
    pub task: String,
    /// Available tools the agent can use
    pub available_tools: Vec<String>,
    /// Working directory
    pub working_dir: String,
    /// Project type if known
    pub project_type: Option<String>,
    /// Any additional context
    pub additional_context: Option<String>,
}

impl PlanContext {
    pub fn new(task: impl Into<String>) -> Self {
        Self {
            task: task.into(),
            available_tools: Vec::new(),
            working_dir: ".".to_string(),
            project_type: None,
            additional_context: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.available_tools = tools;
        self
    }

    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = dir.into();
        self
    }

    pub fn with_project_type(mut self, project_type: impl Into<String>) -> Self {
        self.project_type = Some(project_type.into());
        self
    }

    pub fn with_additional_context(mut self, context: impl Into<String>) -> Self {
        self.additional_context = Some(context.into());
        self
    }
}

/// Trait for plan generators
#[async_trait::async_trait]
pub trait Planner: Send + Sync {
    /// Create a plan for the given context
    async fn create_plan(&self, context: &PlanContext) -> Result<Plan>;

    /// Refine an existing plan based on feedback
    async fn refine_plan(&self, plan: &Plan, feedback: &str) -> Result<Plan>;

    /// Get the planner's name/type
    fn name(&self) -> &str;
}

/// Simple rule-based planner for common task patterns
pub struct SimplePatternPlanner;

impl SimplePatternPlanner {
    pub fn new() -> Self {
        Self
    }

    fn detect_task_type(task: &str) -> TaskType {
        let task_lower = task.to_lowercase();

        if task_lower.contains("fix") || task_lower.contains("bug") || task_lower.contains("error")
        {
            TaskType::BugFix
        } else if task_lower.contains("add")
            || task_lower.contains("implement")
            || task_lower.contains("create")
        {
            TaskType::NewFeature
        } else if task_lower.contains("refactor")
            || task_lower.contains("clean")
            || task_lower.contains("improve")
        {
            TaskType::Refactor
        } else if task_lower.contains("test") {
            TaskType::Testing
        } else if task_lower.contains("document") || task_lower.contains("readme") {
            TaskType::Documentation
        } else {
            TaskType::General
        }
    }

    fn create_steps_for_task_type(task_type: TaskType, _context: &PlanContext) -> Vec<PlanStep> {
        match task_type {
            TaskType::BugFix => vec![
                PlanStep::new(
                    0,
                    "Understand the bug: Read error messages and identify affected code",
                )
                .with_tools(vec!["read_file".to_string(), "search".to_string()]),
                PlanStep::new(1, "Locate the source: Find where the bug originates")
                    .with_tools(vec!["search".to_string(), "read_file".to_string()])
                    .with_dependencies(vec![0]),
                PlanStep::new(2, "Implement the fix: Modify the code to resolve the issue")
                    .with_tools(vec!["edit_file".to_string(), "write_file".to_string()])
                    .with_dependencies(vec![1]),
                PlanStep::new(3, "Verify the fix: Run tests to ensure the bug is fixed")
                    .with_tools(vec!["bash".to_string()])
                    .with_validation("Tests pass and bug is no longer reproducible".to_string())
                    .with_dependencies(vec![2]),
            ],
            TaskType::NewFeature => vec![
                PlanStep::new(0, "Analyze requirements: Understand what needs to be built")
                    .with_tools(vec!["read_file".to_string()]),
                PlanStep::new(1, "Design the solution: Plan the implementation approach")
                    .with_dependencies(vec![0]),
                PlanStep::new(
                    2,
                    "Implement core functionality: Write the main feature code",
                )
                .with_tools(vec!["write_file".to_string(), "edit_file".to_string()])
                .with_dependencies(vec![1]),
                PlanStep::new(3, "Add tests: Write tests for the new feature")
                    .with_tools(vec!["write_file".to_string()])
                    .with_dependencies(vec![2]),
                PlanStep::new(4, "Verify implementation: Run all tests")
                    .with_tools(vec!["bash".to_string()])
                    .with_validation("All tests pass".to_string())
                    .with_dependencies(vec![3]),
            ],
            TaskType::Refactor => vec![
                PlanStep::new(
                    0,
                    "Understand current code: Read and analyze existing implementation",
                )
                .with_tools(vec!["read_file".to_string(), "search".to_string()]),
                PlanStep::new(
                    1,
                    "Ensure test coverage: Verify tests exist before refactoring",
                )
                .with_tools(vec!["bash".to_string()])
                .with_dependencies(vec![0]),
                PlanStep::new(
                    2,
                    "Refactor code: Apply improvements while preserving behavior",
                )
                .with_tools(vec!["edit_file".to_string()])
                .with_dependencies(vec![1]),
                PlanStep::new(3, "Verify refactoring: Run tests to ensure nothing broke")
                    .with_tools(vec!["bash".to_string()])
                    .with_validation("All existing tests still pass".to_string())
                    .with_dependencies(vec![2]),
            ],
            TaskType::Testing => vec![
                PlanStep::new(
                    0,
                    "Analyze code to test: Understand the code that needs testing",
                )
                .with_tools(vec!["read_file".to_string()]),
                PlanStep::new(1, "Identify test cases: Determine what scenarios to test")
                    .with_dependencies(vec![0]),
                PlanStep::new(2, "Write tests: Implement the test cases")
                    .with_tools(vec!["write_file".to_string()])
                    .with_dependencies(vec![1]),
                PlanStep::new(3, "Run tests: Execute and verify tests pass")
                    .with_tools(vec!["bash".to_string()])
                    .with_validation("New tests pass".to_string())
                    .with_dependencies(vec![2]),
            ],
            TaskType::Documentation => vec![
                PlanStep::new(0, "Review code/feature: Understand what to document")
                    .with_tools(vec!["read_file".to_string()]),
                PlanStep::new(1, "Write documentation: Create or update documentation")
                    .with_tools(vec!["write_file".to_string(), "edit_file".to_string()])
                    .with_dependencies(vec![0]),
                PlanStep::new(2, "Review documentation: Ensure accuracy and completeness")
                    .with_dependencies(vec![1]),
            ],
            TaskType::General => vec![
                PlanStep::new(0, "Understand the task: Gather context and requirements")
                    .with_tools(vec!["read_file".to_string(), "search".to_string()]),
                PlanStep::new(1, "Execute the task: Perform the required work")
                    .with_tools(vec![
                        "edit_file".to_string(),
                        "write_file".to_string(),
                        "bash".to_string(),
                    ])
                    .with_dependencies(vec![0]),
                PlanStep::new(
                    2,
                    "Verify completion: Check that the task is done correctly",
                )
                .with_dependencies(vec![1]),
            ],
        }
    }
}

impl Default for SimplePatternPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
enum TaskType {
    BugFix,
    NewFeature,
    Refactor,
    Testing,
    Documentation,
    General,
}

#[async_trait::async_trait]
impl Planner for SimplePatternPlanner {
    async fn create_plan(&self, context: &PlanContext) -> Result<Plan> {
        let task_type = Self::detect_task_type(&context.task);
        let steps = Self::create_steps_for_task_type(task_type, context);

        let mut plan = Plan::new(&context.task).with_steps(steps);
        plan.mark_ready();

        Ok(plan)
    }

    async fn refine_plan(&self, plan: &Plan, feedback: &str) -> Result<Plan> {
        // Simple implementation: just add a note about the feedback
        let mut refined = plan.clone();
        refined.notes = Some(format!("Refined based on feedback: {}", feedback));
        Ok(refined)
    }

    fn name(&self) -> &str {
        "simple_pattern_planner"
    }
}

/// LLM-based planner that uses a language model to generate plans
pub struct LlmPlanner {
    /// System prompt for plan generation
    system_prompt: String,
}

impl LlmPlanner {
    pub fn new() -> Self {
        Self {
            system_prompt: Self::default_system_prompt(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    fn default_system_prompt() -> String {
        r#"You are a planning assistant. Your job is to create clear, actionable plans for software development tasks.

When given a task, you must output a JSON plan with the following structure:
{
  "goal": "the main goal/task",
  "notes": "any high-level notes about the approach",
  "steps": [
    {
      "id": 0,
      "description": "clear description of what to do",
      "tool_hints": ["suggested_tool_1", "suggested_tool_2"],
      "validation": "how to verify this step is complete",
      "dependencies": []
    },
    {
      "id": 1,
      "description": "next step description",
      "tool_hints": ["tool"],
      "validation": "verification criteria",
      "dependencies": [0]
    }
  ]
}

Guidelines:
1. Break down tasks into 3-7 concrete steps
2. Each step should be independently verifiable
3. Include appropriate tool hints (read_file, write_file, edit_file, bash, search, etc.)
4. Set dependencies correctly - steps should only depend on prior steps they actually need
5. Be specific in descriptions - avoid vague language
6. Include validation criteria for important steps

Output ONLY the JSON, no additional text."#.to_string()
    }

    /// Parse the LLM response into a Plan
    #[allow(dead_code)]
    fn parse_plan_response(response: &str, goal: &str) -> Result<Plan> {
        // Try to extract JSON from the response
        let json_str = Self::extract_json(response)?;

        // Parse the JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse plan JSON: {}", e))?;

        let mut plan = Plan::new(parsed.get("goal").and_then(|v| v.as_str()).unwrap_or(goal));

        if let Some(notes) = parsed.get("notes").and_then(|v| v.as_str()) {
            plan.notes = Some(notes.to_string());
        }

        if let Some(steps_array) = parsed.get("steps").and_then(|v| v.as_array()) {
            for (idx, step_value) in steps_array.iter().enumerate() {
                let description = step_value
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown step")
                    .to_string();

                let mut step = PlanStep::new(idx, description);

                if let Some(tools) = step_value.get("tool_hints").and_then(|v| v.as_array()) {
                    step.tool_hints = tools
                        .iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect();
                }

                if let Some(validation) = step_value.get("validation").and_then(|v| v.as_str()) {
                    step.validation = Some(validation.to_string());
                }

                if let Some(deps) = step_value.get("dependencies").and_then(|v| v.as_array()) {
                    step.dependencies = deps
                        .iter()
                        .filter_map(|d| d.as_u64().map(|n| n as usize))
                        .collect();
                }

                plan.add_step(step);
            }
        }

        plan.mark_ready();
        Ok(plan)
    }

    /// Extract JSON from a response that might contain markdown or other text
    #[allow(dead_code)]
    fn extract_json(response: &str) -> Result<String> {
        // First, try to parse the whole response as JSON
        if serde_json::from_str::<serde_json::Value>(response).is_ok() {
            return Ok(response.to_string());
        }

        // Try to find JSON in markdown code blocks
        if let Some(start) = response.find("```json") {
            let rest = response.get(start + 7..).unwrap_or("");
            if let Some(end) = rest.find("```") {
                if let Some(json_content) = rest.get(..end) {
                    return Ok(json_content.trim().to_string());
                }
            }
        }

        // Try to find JSON in generic code blocks
        if let Some(start) = response.find("```") {
            let after_marker = response.get(start + 3..).unwrap_or("");
            if let Some(newline_pos) = after_marker.find('\n') {
                let after_newline = after_marker.get(newline_pos + 1..).unwrap_or("");
                if let Some(end) = after_newline.find("```") {
                    if let Some(potential_json) = after_newline.get(..end) {
                        let trimmed = potential_json.trim();
                        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                            return Ok(trimmed.to_string());
                        }
                    }
                }
            }
        }

        // Try to find a JSON object anywhere in the response
        if let Some(start) = response.find('{') {
            // Find the matching closing brace using char indices
            let mut depth = 0;
            let mut end_byte_pos = start;
            let suffix = response.get(start..).unwrap_or("");
            for (byte_offset, c) in suffix.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end_byte_pos = start + byte_offset + c.len_utf8();
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth == 0 {
                if let Some(potential_json) = response.get(start..end_byte_pos) {
                    if serde_json::from_str::<serde_json::Value>(potential_json).is_ok() {
                        return Ok(potential_json.to_string());
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Could not find valid JSON in response"))
    }

    /// Generate the prompt for plan creation
    pub fn generate_planning_prompt(&self, context: &PlanContext) -> String {
        let mut prompt = format!(
            "Create a plan for the following task:\n\nTASK: {}\n",
            context.task
        );

        if !context.available_tools.is_empty() {
            prompt.push_str(&format!(
                "\nAVAILABLE TOOLS: {}\n",
                context.available_tools.join(", ")
            ));
        }

        if let Some(project_type) = &context.project_type {
            prompt.push_str(&format!("\nPROJECT TYPE: {}\n", project_type));
        }

        prompt.push_str(&format!("\nWORKING DIRECTORY: {}\n", context.working_dir));

        if let Some(additional) = &context.additional_context {
            prompt.push_str(&format!("\nADDITIONAL CONTEXT:\n{}\n", additional));
        }

        prompt
    }

    /// Generate the prompt for plan refinement
    pub fn generate_refinement_prompt(&self, plan: &Plan, feedback: &str) -> String {
        format!(
            r#"Refine the following plan based on the feedback provided.

CURRENT PLAN:
{}

FEEDBACK:
{}

Output the refined plan in the same JSON format."#,
            serde_json::to_string_pretty(plan).unwrap_or_default(),
            feedback
        )
    }

    /// Get the system prompt
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }
}

impl Default for LlmPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Planner for LlmPlanner {
    async fn create_plan(&self, context: &PlanContext) -> Result<Plan> {
        // This is a placeholder - actual LLM call would happen in the agent
        // For now, fall back to simple pattern planner
        let simple = SimplePatternPlanner::new();
        simple.create_plan(context).await
    }

    async fn refine_plan(&self, plan: &Plan, feedback: &str) -> Result<Plan> {
        // Placeholder - would use LLM in actual implementation
        let mut refined = plan.clone();
        refined.notes = Some(format!(
            "{}. Refined based on: {}",
            refined.notes.as_deref().unwrap_or(""),
            feedback
        ));
        Ok(refined)
    }

    fn name(&self) -> &str {
        "llm_planner"
    }
}

/// Manages planning for the agent
pub struct PlanManager {
    /// The current active plan
    current_plan: Option<Plan>,
    /// The planner implementation to use
    planner: Box<dyn Planner>,
    /// Whether planning is enabled
    enabled: bool,
}

impl PlanManager {
    pub fn new() -> Self {
        Self {
            current_plan: None,
            planner: Box::new(SimplePatternPlanner::new()),
            enabled: false,
        }
    }

    pub fn with_planner(mut self, planner: Box<dyn Planner>) -> Self {
        self.planner = planner;
        self
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn has_plan(&self) -> bool {
        self.current_plan.is_some()
    }

    pub fn current_plan(&self) -> Option<&Plan> {
        self.current_plan.as_ref()
    }

    pub fn current_plan_mut(&mut self) -> Option<&mut Plan> {
        self.current_plan.as_mut()
    }

    pub fn set_plan(&mut self, plan: Plan) {
        self.current_plan = Some(plan);
    }

    pub fn clear_plan(&mut self) {
        self.current_plan = None;
    }

    /// Create a new plan for the given context
    pub async fn create_plan(&mut self, context: &PlanContext) -> Result<&Plan> {
        let plan = self.planner.create_plan(context).await?;
        self.current_plan = Some(plan);
        Ok(self.current_plan.as_ref().unwrap())
    }

    /// Refine the current plan based on feedback
    pub async fn refine_plan(&mut self, feedback: &str) -> Result<&Plan> {
        let current = self
            .current_plan
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No plan to refine"))?;
        let refined = self.planner.refine_plan(current, feedback).await?;
        self.current_plan = Some(refined);
        Ok(self.current_plan.as_ref().unwrap())
    }

    /// Advance to the next step in the plan
    pub fn advance_plan(&mut self) -> bool {
        if let Some(plan) = &mut self.current_plan {
            plan.advance()
        } else {
            false
        }
    }

    /// Mark the current step as completed
    pub fn complete_current_step(&mut self, output: Option<String>) {
        if let Some(plan) = &mut self.current_plan {
            if let Some(step) = plan.current_mut() {
                step.mark_completed(output);
            }
        }
    }

    /// Mark the current step as failed
    pub fn fail_current_step(&mut self, error: impl Into<String>) {
        if let Some(plan) = &mut self.current_plan {
            if let Some(step) = plan.current_mut() {
                step.mark_failed(error);
            }
        }
    }

    /// Get the context for the current step (for injecting into LLM prompt)
    pub fn get_step_context(&self) -> Option<String> {
        self.current_plan.as_ref().map(|p| p.format_for_llm())
    }

    /// Check if the plan is complete
    pub fn is_plan_complete(&self) -> bool {
        self.current_plan
            .as_ref()
            .map(|p| p.is_complete())
            .unwrap_or(true)
    }
}

impl Default for PlanManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_step_creation() {
        let step = PlanStep::new(0, "Test step")
            .with_tools(vec!["tool1".to_string(), "tool2".to_string()])
            .with_validation("Check something")
            .with_dependencies(vec![]);

        assert_eq!(step.id, 0);
        assert_eq!(step.description, "Test step");
        assert_eq!(step.tool_hints.len(), 2);
        assert!(step.validation.is_some());
        assert_eq!(step.status, StepStatus::Pending);
    }

    #[test]
    fn test_plan_step_status_transitions() {
        let mut step = PlanStep::new(0, "Test step");
        assert_eq!(step.status, StepStatus::Pending);

        step.mark_in_progress();
        assert_eq!(step.status, StepStatus::InProgress);

        step.mark_completed(Some("Done".to_string()));
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.output, Some("Done".to_string()));
    }

    #[test]
    fn test_plan_creation() {
        let plan = Plan::new("Implement feature X").with_steps(vec![
            PlanStep::new(0, "Step 1"),
            PlanStep::new(1, "Step 2").with_dependencies(vec![0]),
        ]);

        assert_eq!(plan.goal, "Implement feature X");
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.status, PlanStatus::Creating);
    }

    #[test]
    fn test_plan_step_readiness() {
        let step1 = PlanStep::new(0, "Step 1");
        let step2 = PlanStep::new(1, "Step 2").with_dependencies(vec![0]);

        assert!(step1.is_ready(&[]));
        assert!(!step2.is_ready(&[]));
        assert!(step2.is_ready(&[0]));
    }

    #[test]
    fn test_plan_progress() {
        let mut plan = Plan::new("Test goal").with_steps(vec![
            PlanStep::new(0, "Step 1"),
            PlanStep::new(1, "Step 2").with_dependencies(vec![0]),
            PlanStep::new(2, "Step 3").with_dependencies(vec![1]),
        ]);

        plan.mark_ready();
        assert!(!plan.is_complete());

        plan.steps[0].mark_completed(None);
        assert!(!plan.is_complete());
        assert!(plan.advance());
        assert_eq!(plan.current_step, 1);

        plan.steps[1].mark_completed(None);
        plan.steps[2].mark_completed(None);
        assert!(plan.is_complete());
    }

    #[tokio::test]
    async fn test_simple_pattern_planner() {
        let planner = SimplePatternPlanner::new();
        let context = PlanContext::new("Fix the login bug");

        let plan = planner.create_plan(&context).await.unwrap();
        assert_eq!(plan.status, PlanStatus::Ready);
        assert!(!plan.steps.is_empty());

        // Bug fix should have specific steps
        assert!(plan
            .steps
            .iter()
            .any(|s| s.description.contains("Understand")));
        assert!(plan.steps.iter().any(|s| s.description.contains("Verify")));
    }

    #[test]
    fn test_plan_format_for_display() {
        let plan = Plan::new("Test goal").with_steps(vec![
            PlanStep::new(0, "First step"),
            PlanStep::new(1, "Second step").with_dependencies(vec![0]),
        ]);

        let display = plan.format_for_display();
        assert!(display.contains("Test goal"));
        assert!(display.contains("First step"));
        assert!(display.contains("Second step"));
    }

    #[test]
    fn test_llm_planner_json_extraction() {
        // Test extracting JSON from markdown code block
        let response = r#"Here's the plan:
```json
{"goal": "test", "steps": []}
```"#;
        let json = LlmPlanner::extract_json(response).unwrap();
        assert!(json.contains("test"));

        // Test extracting raw JSON
        let response = r#"{"goal": "test", "steps": []}"#;
        let json = LlmPlanner::extract_json(response).unwrap();
        assert!(json.contains("test"));

        // Test extracting JSON from text
        let response = r#"The plan is: {"goal": "test", "steps": []} and that's it"#;
        let json = LlmPlanner::extract_json(response).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_llm_planner_parse_response() {
        let response = r#"{
            "goal": "Fix the bug",
            "notes": "Important fix",
            "steps": [
                {
                    "id": 0,
                    "description": "Find the bug",
                    "tool_hints": ["search", "read_file"],
                    "validation": "Bug located",
                    "dependencies": []
                },
                {
                    "id": 1,
                    "description": "Fix it",
                    "tool_hints": ["edit_file"],
                    "dependencies": [0]
                }
            ]
        }"#;

        let plan = LlmPlanner::parse_plan_response(response, "fallback goal").unwrap();
        assert_eq!(plan.goal, "Fix the bug");
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].tool_hints.len(), 2);
        assert_eq!(plan.steps[1].dependencies, vec![0]);
    }

    #[tokio::test]
    async fn test_plan_manager() {
        let mut manager = PlanManager::new();
        assert!(!manager.has_plan());

        let context = PlanContext::new("Test task");
        manager.create_plan(&context).await.unwrap();
        assert!(manager.has_plan());

        let plan = manager.current_plan().unwrap();
        assert_eq!(plan.status, PlanStatus::Ready);
    }

    #[test]
    fn test_plan_manager_step_tracking() {
        let mut manager = PlanManager::new();
        let mut plan = Plan::new("Test").with_steps(vec![
            PlanStep::new(0, "Step 1"),
            PlanStep::new(1, "Step 2").with_dependencies(vec![0]),
        ]);
        plan.mark_ready();
        manager.set_plan(plan);

        manager.complete_current_step(Some("Done".to_string()));
        assert!(manager.advance_plan());

        let plan = manager.current_plan().unwrap();
        assert_eq!(plan.current_step, 1);
        assert_eq!(plan.steps[0].status, StepStatus::Completed);
    }
}
