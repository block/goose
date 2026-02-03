//! Reflexion pattern for self-improvement through verbal reinforcement learning
//!
//! Based on the paper "Reflexion: Language Agents with Verbal Reinforcement Learning"
//! (Shinn et al., 2023) - https://arxiv.org/abs/2303.11366
//!
//! The Reflexion pattern enables agents to:
//! 1. Attempt a task
//! 2. Evaluate the outcome
//! 3. Generate verbal self-reflection on what went wrong
//! 4. Store reflections in episodic memory
//! 5. Use past reflections to improve future attempts
//!
//! This creates a form of meta-learning without model fine-tuning.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single action taken during a task attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptAction {
    /// Description of the action
    pub description: String,
    /// Tool used (if any)
    pub tool: Option<String>,
    /// Result of the action
    pub result: String,
    /// Whether this action succeeded
    pub success: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl AttemptAction {
    pub fn new(description: impl Into<String>, result: impl Into<String>, success: bool) -> Self {
        Self {
            description: description.into(),
            tool: None,
            result: result.into(),
            success,
            timestamp: Utc::now(),
        }
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }
}

/// Outcome of a task attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttemptOutcome {
    /// Task completed successfully
    Success,
    /// Task failed with an error
    Failure,
    /// Task partially completed
    Partial,
    /// Task timed out
    Timeout,
    /// Task was aborted/cancelled
    Aborted,
}

impl std::fmt::Display for AttemptOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttemptOutcome::Success => write!(f, "success"),
            AttemptOutcome::Failure => write!(f, "failure"),
            AttemptOutcome::Partial => write!(f, "partial"),
            AttemptOutcome::Timeout => write!(f, "timeout"),
            AttemptOutcome::Aborted => write!(f, "aborted"),
        }
    }
}

/// A complete attempt at a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAttempt {
    /// Unique identifier for this attempt
    pub attempt_id: String,
    /// The task that was attempted
    pub task: String,
    /// Sequence of actions taken
    pub actions: Vec<AttemptAction>,
    /// Final outcome
    pub outcome: AttemptOutcome,
    /// Error message if failed
    pub error: Option<String>,
    /// Time taken in milliseconds
    pub duration_ms: u64,
    /// When the attempt started
    pub started_at: DateTime<Utc>,
    /// When the attempt ended
    pub ended_at: DateTime<Utc>,
}

impl TaskAttempt {
    pub fn new(task: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            attempt_id: uuid::Uuid::new_v4().to_string(),
            task: task.into(),
            actions: Vec::new(),
            outcome: AttemptOutcome::Failure, // Default to failure until proven otherwise
            error: None,
            duration_ms: 0,
            started_at: now,
            ended_at: now,
        }
    }

    pub fn add_action(&mut self, action: AttemptAction) {
        self.actions.push(action);
    }

    pub fn complete(&mut self, outcome: AttemptOutcome, error: Option<String>) {
        self.outcome = outcome;
        self.error = error;
        self.ended_at = Utc::now();
        self.duration_ms = (self.ended_at - self.started_at).num_milliseconds() as u64;
    }

    pub fn is_success(&self) -> bool {
        self.outcome == AttemptOutcome::Success
    }

    /// Get a summary of what was tried
    pub fn summarize_actions(&self) -> String {
        self.actions
            .iter()
            .map(|a| {
                let status = if a.success { "✓" } else { "✗" };
                format!("{} {}", status, a.description)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A reflection on a failed attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    /// Unique identifier
    pub reflection_id: String,
    /// The task that was attempted
    pub task: String,
    /// Summary of what was tried
    pub attempt_summary: String,
    /// The outcome
    pub outcome: AttemptOutcome,
    /// What went wrong (diagnosis)
    pub diagnosis: String,
    /// Self-generated verbal reflection
    pub reflection_text: String,
    /// Specific lessons learned
    pub lessons: Vec<String>,
    /// Suggested improvements for future attempts
    pub improvements: Vec<String>,
    /// Confidence in this reflection (0.0 - 1.0)
    pub confidence: f32,
    /// When this reflection was generated
    pub created_at: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl Reflection {
    pub fn new(
        task: impl Into<String>,
        attempt_summary: impl Into<String>,
        outcome: AttemptOutcome,
    ) -> Self {
        Self {
            reflection_id: uuid::Uuid::new_v4().to_string(),
            task: task.into(),
            attempt_summary: attempt_summary.into(),
            outcome,
            diagnosis: String::new(),
            reflection_text: String::new(),
            lessons: Vec::new(),
            improvements: Vec::new(),
            confidence: 1.0,
            created_at: Utc::now(),
            tags: Vec::new(),
        }
    }

    pub fn with_diagnosis(mut self, diagnosis: impl Into<String>) -> Self {
        self.diagnosis = diagnosis.into();
        self
    }

    pub fn with_reflection(mut self, text: impl Into<String>) -> Self {
        self.reflection_text = text.into();
        self
    }

    pub fn with_lessons(mut self, lessons: Vec<String>) -> Self {
        self.lessons = lessons;
        self
    }

    pub fn with_improvements(mut self, improvements: Vec<String>) -> Self {
        self.improvements = improvements;
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Format for LLM context
    pub fn format_for_context(&self) -> String {
        let mut output = format!(
            "PAST REFLECTION ({})\nTask: {}\nOutcome: {}\n",
            self.created_at.format("%Y-%m-%d"),
            self.task,
            self.outcome
        );

        if !self.diagnosis.is_empty() {
            output.push_str(&format!("What went wrong: {}\n", self.diagnosis));
        }

        if !self.lessons.is_empty() {
            output.push_str("Lessons learned:\n");
            for lesson in &self.lessons {
                output.push_str(&format!("- {}\n", lesson));
            }
        }

        if !self.improvements.is_empty() {
            output.push_str("Improvements to try:\n");
            for improvement in &self.improvements {
                output.push_str(&format!("- {}\n", improvement));
            }
        }

        output
    }
}

/// Configuration for the Reflexion agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexionConfig {
    /// Maximum number of attempts before giving up
    pub max_attempts: usize,
    /// Whether to generate reflections automatically
    pub auto_reflect: bool,
    /// Minimum confidence score to use a reflection
    pub min_reflection_confidence: f32,
    /// Maximum number of reflections to include in context
    pub max_reflections_in_context: usize,
    /// Whether to persist reflections
    pub persist_reflections: bool,
}

impl Default for ReflexionConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            auto_reflect: true,
            min_reflection_confidence: 0.5,
            max_reflections_in_context: 3,
            persist_reflections: true,
        }
    }
}

/// Memory store for reflections
pub struct ReflectionMemory {
    /// All stored reflections
    reflections: Vec<Reflection>,
    /// Index by task (for quick lookup)
    task_index: HashMap<String, Vec<usize>>,
    /// Index by tag
    tag_index: HashMap<String, Vec<usize>>,
}

impl ReflectionMemory {
    pub fn new() -> Self {
        Self {
            reflections: Vec::new(),
            task_index: HashMap::new(),
            tag_index: HashMap::new(),
        }
    }

    /// Store a reflection
    pub fn store(&mut self, reflection: Reflection) {
        let idx = self.reflections.len();

        // Index by task keywords
        for word in reflection.task.split_whitespace() {
            let key = word.to_lowercase();
            if key.len() > 3 {
                // Skip short words
                self.task_index.entry(key).or_default().push(idx);
            }
        }

        // Index by tags
        for tag in &reflection.tags {
            self.tag_index.entry(tag.clone()).or_default().push(idx);
        }

        self.reflections.push(reflection);
    }

    /// Find relevant reflections for a task
    pub fn find_relevant(&self, task: &str, limit: usize) -> Vec<&Reflection> {
        let mut scores: HashMap<usize, f32> = HashMap::new();

        // Score by task keyword overlap
        for word in task.split_whitespace() {
            let key = word.to_lowercase();
            if let Some(indices) = self.task_index.get(&key) {
                for &idx in indices {
                    *scores.entry(idx).or_default() += 1.0;
                }
            }
        }

        // Sort by score and return top results
        let mut scored: Vec<_> = scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(limit)
            .filter_map(|(idx, _)| self.reflections.get(idx))
            .collect()
    }

    /// Find reflections by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&Reflection> {
        self.tag_index
            .get(tag)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.reflections.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all reflections
    pub fn all(&self) -> &[Reflection] {
        &self.reflections
    }

    /// Get the number of stored reflections
    pub fn len(&self) -> usize {
        self.reflections.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.reflections.is_empty()
    }

    /// Clear all reflections
    pub fn clear(&mut self) {
        self.reflections.clear();
        self.task_index.clear();
        self.tag_index.clear();
    }
}

impl Default for ReflectionMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// The Reflexion agent that learns from past failures
pub struct ReflexionAgent {
    /// Configuration
    config: ReflexionConfig,
    /// Memory of past reflections
    memory: ReflectionMemory,
    /// Current attempt (if any)
    current_attempt: Option<TaskAttempt>,
    /// Count of attempts for current task
    attempt_count: usize,
}

impl ReflexionAgent {
    /// Create a new Reflexion agent
    pub fn new(config: ReflexionConfig) -> Self {
        Self {
            config,
            memory: ReflectionMemory::new(),
            current_attempt: None,
            attempt_count: 0,
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(ReflexionConfig::default())
    }

    /// Get the configuration
    pub fn config(&self) -> &ReflexionConfig {
        &self.config
    }

    /// Get the reflection memory
    pub fn memory(&self) -> &ReflectionMemory {
        &self.memory
    }

    /// Start a new task attempt
    pub fn start_attempt(&mut self, task: impl Into<String>) -> &mut TaskAttempt {
        self.current_attempt = Some(TaskAttempt::new(task));
        self.attempt_count += 1;
        self.current_attempt.as_mut().unwrap()
    }

    /// Get the current attempt
    pub fn current_attempt(&self) -> Option<&TaskAttempt> {
        self.current_attempt.as_ref()
    }

    /// Get mutable reference to current attempt
    pub fn current_attempt_mut(&mut self) -> Option<&mut TaskAttempt> {
        self.current_attempt.as_mut()
    }

    /// Record an action in the current attempt
    pub fn record_action(&mut self, action: AttemptAction) {
        if let Some(attempt) = &mut self.current_attempt {
            attempt.add_action(action);
        }
    }

    /// Complete the current attempt
    pub fn complete_attempt(&mut self, outcome: AttemptOutcome, error: Option<String>) {
        if let Some(attempt) = &mut self.current_attempt {
            attempt.complete(outcome, error);
        }
    }

    /// Generate a reflection on the current (failed) attempt
    pub fn reflect(&mut self) -> Option<Reflection> {
        let attempt = self.current_attempt.take()?;

        if attempt.is_success() {
            // No reflection needed for successful attempts
            return None;
        }

        let reflection =
            Reflection::new(&attempt.task, attempt.summarize_actions(), attempt.outcome)
                .with_diagnosis(
                    attempt
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unknown error".to_string()),
                );

        if self.config.persist_reflections {
            self.memory.store(reflection.clone());
        }

        Some(reflection)
    }

    /// Generate a reflection with LLM-generated content
    pub fn reflect_with_content(
        &mut self,
        diagnosis: impl Into<String>,
        reflection_text: impl Into<String>,
        lessons: Vec<String>,
        improvements: Vec<String>,
    ) -> Option<Reflection> {
        let attempt = self.current_attempt.take()?;

        let reflection =
            Reflection::new(&attempt.task, attempt.summarize_actions(), attempt.outcome)
                .with_diagnosis(diagnosis)
                .with_reflection(reflection_text)
                .with_lessons(lessons)
                .with_improvements(improvements);

        if self.config.persist_reflections {
            self.memory.store(reflection.clone());
        }

        Some(reflection)
    }

    /// Get relevant reflections for a task
    pub fn get_relevant_reflections(&self, task: &str) -> Vec<&Reflection> {
        self.memory
            .find_relevant(task, self.config.max_reflections_in_context)
    }

    /// Check if we should continue trying
    pub fn should_continue(&self) -> bool {
        self.attempt_count < self.config.max_attempts
    }

    /// Reset for a new task
    pub fn reset(&mut self) {
        self.current_attempt = None;
        self.attempt_count = 0;
    }

    /// Get the number of attempts made
    pub fn attempts(&self) -> usize {
        self.attempt_count
    }

    /// Generate reflection prompt for LLM
    pub fn generate_reflection_prompt(attempt: &TaskAttempt) -> String {
        format!(
            r#"Reflect on this failed attempt:

TASK: {}

ACTIONS TAKEN:
{}

OUTCOME: {}
ERROR: {}

Please provide:
1. DIAGNOSIS: What went wrong and why?
2. REFLECTION: A detailed analysis of the failure
3. LESSONS: What can be learned from this failure? (list)
4. IMPROVEMENTS: What should be done differently next time? (list)

Format your response as:
DIAGNOSIS: <your diagnosis>
REFLECTION: <your reflection>
LESSONS:
- <lesson 1>
- <lesson 2>
IMPROVEMENTS:
- <improvement 1>
- <improvement 2>"#,
            attempt.task,
            attempt.summarize_actions(),
            attempt.outcome,
            attempt.error.as_deref().unwrap_or("None")
        )
    }

    /// Generate context with past reflections for a new attempt
    pub fn generate_context_with_reflections(&self, task: &str) -> String {
        let reflections = self.get_relevant_reflections(task);

        if reflections.is_empty() {
            return String::new();
        }

        let mut context = String::from("RELEVANT PAST REFLECTIONS:\n\n");
        for (i, reflection) in reflections.iter().enumerate() {
            context.push_str(&format!("--- Reflection {} ---\n", i + 1));
            context.push_str(&reflection.format_for_context());
            context.push('\n');
        }

        context.push_str("\nUse these lessons to improve your approach.\n");
        context
    }
}

impl Default for ReflexionAgent {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attempt_action() {
        let action = AttemptAction::new("Read file", "File contents...", true).with_tool("read");

        assert_eq!(action.tool, Some("read".to_string()));
        assert!(action.success);
    }

    #[test]
    fn test_task_attempt() {
        let mut attempt = TaskAttempt::new("Fix the bug");
        attempt.add_action(AttemptAction::new("Read code", "def foo()...", true));
        attempt.add_action(AttemptAction::new(
            "Apply fix",
            "Error: syntax error",
            false,
        ));
        attempt.complete(AttemptOutcome::Failure, Some("Syntax error".to_string()));

        assert!(!attempt.is_success());
        assert_eq!(attempt.actions.len(), 2);
        assert!(attempt.summarize_actions().contains("Read code"));
    }

    #[test]
    fn test_reflection_creation() {
        let reflection = Reflection::new("Fix bug", "Tried X, Y, Z", AttemptOutcome::Failure)
            .with_diagnosis("Approach was wrong")
            .with_lessons(vec!["Check types first".to_string()])
            .with_improvements(vec!["Use type hints".to_string()]);

        assert_eq!(reflection.lessons.len(), 1);
        assert_eq!(reflection.improvements.len(), 1);
    }

    #[test]
    fn test_reflection_memory() {
        let mut memory = ReflectionMemory::new();

        let r1 = Reflection::new(
            "Fix authentication bug",
            "Tried token refresh",
            AttemptOutcome::Failure,
        )
        .with_lessons(vec!["Check token expiry".to_string()]);

        let r2 = Reflection::new(
            "Fix database connection",
            "Tried reconnect",
            AttemptOutcome::Failure,
        );

        memory.store(r1);
        memory.store(r2);

        assert_eq!(memory.len(), 2);

        // Should find the auth-related reflection
        let relevant = memory.find_relevant("authentication error", 5);
        assert_eq!(relevant.len(), 1);
        assert!(relevant[0].task.contains("authentication"));
    }

    #[test]
    fn test_reflexion_agent() {
        let mut agent = ReflexionAgent::default_config();

        // Start first attempt (using longer words that will be indexed)
        agent.start_attempt("Debug authentication issue");
        agent.record_action(AttemptAction::new("Read file", "...", true));
        agent.record_action(AttemptAction::new("Apply fix", "Error", false));
        agent.complete_attempt(AttemptOutcome::Failure, Some("Fix failed".to_string()));

        assert!(agent.should_continue());

        // Generate reflection
        let reflection = agent.reflect_with_content(
            "Wrong approach",
            "I should have checked the types first",
            vec!["Always check types".to_string()],
            vec!["Add type validation".to_string()],
        );

        assert!(reflection.is_some());
        assert_eq!(agent.memory().len(), 1);

        // Start second attempt
        agent.start_attempt("Debug authentication issue");

        // Should have context from first attempt (using words >3 chars for matching)
        let context = agent.generate_context_with_reflections("Debug authentication issue");
        assert!(context.contains("Always check types"));
    }

    #[test]
    fn test_max_attempts() {
        let config = ReflexionConfig {
            max_attempts: 2,
            ..Default::default()
        };
        let mut agent = ReflexionAgent::new(config);

        agent.start_attempt("Task 1");
        agent.complete_attempt(AttemptOutcome::Failure, None);
        agent.reflect();
        assert!(agent.should_continue());

        agent.start_attempt("Task 1");
        agent.complete_attempt(AttemptOutcome::Failure, None);
        agent.reflect();
        assert!(!agent.should_continue());
    }

    #[test]
    fn test_reflection_formatting() {
        let reflection = Reflection::new("Test task", "Actions...", AttemptOutcome::Failure)
            .with_diagnosis("Error in logic")
            .with_lessons(vec!["Lesson 1".to_string(), "Lesson 2".to_string()])
            .with_improvements(vec!["Improvement 1".to_string()]);

        let formatted = reflection.format_for_context();
        assert!(formatted.contains("Test task"));
        assert!(formatted.contains("Lesson 1"));
        assert!(formatted.contains("Improvement 1"));
    }
}
