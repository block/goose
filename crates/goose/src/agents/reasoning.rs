//! Advanced reasoning modes for agentic execution
//!
//! This module implements state-of-the-art reasoning patterns:
//! - **ReAct**: Reasoning + Acting interleaved (Yao et al., 2022)
//! - **Chain-of-Thought (CoT)**: Step-by-step reasoning before action
//! - **Tree-of-Thoughts (ToT)**: Branching with evaluation (Yao et al., 2023)
//!
//! These patterns enable more deliberate problem-solving with explicit
//! reasoning traces, self-reflection, and backtracking capabilities.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reasoning mode to use for agent execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningMode {
    /// Standard execution without explicit reasoning
    #[default]
    Standard,
    /// Chain-of-Thought: Linear step-by-step reasoning before action
    ChainOfThought,
    /// ReAct: Interleaved Reasoning + Acting
    ReAct,
    /// Tree-of-Thoughts: Branching with evaluation and backtracking
    TreeOfThoughts,
}

impl std::fmt::Display for ReasoningMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReasoningMode::Standard => write!(f, "standard"),
            ReasoningMode::ChainOfThought => write!(f, "chain_of_thought"),
            ReasoningMode::ReAct => write!(f, "react"),
            ReasoningMode::TreeOfThoughts => write!(f, "tree_of_thoughts"),
        }
    }
}

impl std::str::FromStr for ReasoningMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "standard" | "default" | "none" => Ok(ReasoningMode::Standard),
            "cot" | "chain_of_thought" | "chain-of-thought" => Ok(ReasoningMode::ChainOfThought),
            "react" | "re-act" | "reasoning_acting" => Ok(ReasoningMode::ReAct),
            "tot" | "tree_of_thoughts" | "tree-of-thoughts" => Ok(ReasoningMode::TreeOfThoughts),
            _ => Err(anyhow::anyhow!(
                "Unknown reasoning mode: {}. Valid options: standard, cot, react, tot",
                s
            )),
        }
    }
}

/// A single thought in a reasoning trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// Unique identifier for this thought
    pub id: usize,
    /// The thought content (reasoning)
    pub content: String,
    /// Type of thought
    pub thought_type: ThoughtType,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// When this thought was generated
    pub timestamp: DateTime<Utc>,
    /// Parent thought ID (for branching in ToT)
    pub parent_id: Option<usize>,
    /// Evaluation score (for ToT)
    pub evaluation: Option<f32>,
}

impl Thought {
    pub fn new(id: usize, content: impl Into<String>, thought_type: ThoughtType) -> Self {
        Self {
            id,
            content: content.into(),
            thought_type,
            confidence: 1.0,
            timestamp: Utc::now(),
            parent_id: None,
            evaluation: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_parent(mut self, parent_id: usize) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_evaluation(mut self, score: f32) -> Self {
        self.evaluation = Some(score.clamp(0.0, 1.0));
        self
    }
}

/// Type of thought in the reasoning trace
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThoughtType {
    /// Initial thought about the problem
    Initial,
    /// Analysis of the current situation
    Analysis,
    /// Planning or strategizing
    Planning,
    /// Observation from an action result
    Observation,
    /// Reflection on progress or approach
    Reflection,
    /// Hypothesis to test
    Hypothesis,
    /// Conclusion or decision
    Conclusion,
    /// Self-critique or error recognition
    Critique,
}

impl std::fmt::Display for ThoughtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThoughtType::Initial => write!(f, "Initial"),
            ThoughtType::Analysis => write!(f, "Analysis"),
            ThoughtType::Planning => write!(f, "Planning"),
            ThoughtType::Observation => write!(f, "Observation"),
            ThoughtType::Reflection => write!(f, "Reflection"),
            ThoughtType::Hypothesis => write!(f, "Hypothesis"),
            ThoughtType::Conclusion => write!(f, "Conclusion"),
            ThoughtType::Critique => write!(f, "Critique"),
        }
    }
}

/// An action taken during reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonedAction {
    /// Unique identifier for this action
    pub id: usize,
    /// The thought that led to this action
    pub thought_id: usize,
    /// Description of the action
    pub description: String,
    /// Tool to use (if any)
    pub tool: Option<String>,
    /// Tool parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Result of the action
    pub result: Option<ActionResult>,
    /// When this action was taken
    pub timestamp: DateTime<Utc>,
}

impl ReasonedAction {
    pub fn new(id: usize, thought_id: usize, description: impl Into<String>) -> Self {
        Self {
            id,
            thought_id,
            description: description.into(),
            tool: None,
            parameters: HashMap::new(),
            result: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    pub fn with_parameters(mut self, params: HashMap<String, serde_json::Value>) -> Self {
        self.parameters = params;
        self
    }

    pub fn set_result(&mut self, result: ActionResult) {
        self.result = Some(result);
    }
}

/// Result of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Output from the action
    pub output: String,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

impl ActionResult {
    pub fn success(output: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            duration_ms,
        }
    }

    pub fn failure(error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            duration_ms,
        }
    }
}

/// A complete ReAct trace showing the interleaved reasoning and acting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActTrace {
    /// The task being addressed
    pub task: String,
    /// All thoughts in the trace
    pub thoughts: Vec<Thought>,
    /// All actions taken
    pub actions: Vec<ReasonedAction>,
    /// Current reasoning mode
    pub mode: ReasoningMode,
    /// Whether the task is complete
    pub complete: bool,
    /// Final answer/result if complete
    pub final_answer: Option<String>,
    /// When the trace was started
    pub started_at: DateTime<Utc>,
    /// When the trace was completed
    pub completed_at: Option<DateTime<Utc>>,
}

impl ReActTrace {
    /// Create a new ReAct trace
    pub fn new(task: impl Into<String>, mode: ReasoningMode) -> Self {
        Self {
            task: task.into(),
            thoughts: Vec::new(),
            actions: Vec::new(),
            mode,
            complete: false,
            final_answer: None,
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Add a thought to the trace
    pub fn add_thought(&mut self, content: impl Into<String>, thought_type: ThoughtType) -> usize {
        let id = self.thoughts.len();
        let thought = Thought::new(id, content, thought_type);
        self.thoughts.push(thought);
        id
    }

    /// Add a thought with parent (for ToT branching)
    pub fn add_branching_thought(
        &mut self,
        content: impl Into<String>,
        thought_type: ThoughtType,
        parent_id: usize,
    ) -> usize {
        let id = self.thoughts.len();
        let thought = Thought::new(id, content, thought_type).with_parent(parent_id);
        self.thoughts.push(thought);
        id
    }

    /// Add an action linked to a thought
    pub fn add_action(&mut self, thought_id: usize, description: impl Into<String>) -> usize {
        let id = self.actions.len();
        let action = ReasonedAction::new(id, thought_id, description);
        self.actions.push(action);
        id
    }

    /// Record the result of an action
    pub fn record_action_result(&mut self, action_id: usize, result: ActionResult) {
        if let Some(action) = self.actions.get_mut(action_id) {
            action.set_result(result);
        }
    }

    /// Add an observation thought based on action result
    pub fn add_observation(&mut self, action_id: usize, observation: impl Into<String>) -> usize {
        let thought_id = self
            .actions
            .get(action_id)
            .map(|a| a.thought_id)
            .unwrap_or(0);
        self.add_branching_thought(observation, ThoughtType::Observation, thought_id)
    }

    /// Mark the trace as complete
    pub fn mark_complete(&mut self, answer: Option<String>) {
        self.complete = true;
        self.final_answer = answer;
        self.completed_at = Some(Utc::now());
    }

    /// Get the last thought
    pub fn last_thought(&self) -> Option<&Thought> {
        self.thoughts.last()
    }

    /// Get the last action
    pub fn last_action(&self) -> Option<&ReasonedAction> {
        self.actions.last()
    }

    /// Get thoughts of a specific type
    pub fn thoughts_of_type(&self, thought_type: ThoughtType) -> Vec<&Thought> {
        self.thoughts
            .iter()
            .filter(|t| t.thought_type == thought_type)
            .collect()
    }

    /// Get the reasoning chain (thoughts in order)
    pub fn reasoning_chain(&self) -> Vec<&Thought> {
        self.thoughts.iter().collect()
    }

    /// Format the trace for display
    pub fn format_for_display(&self) -> String {
        let mut output = format!("## ReAct Trace: {}\n", self.task);
        output.push_str(&format!("Mode: {}\n\n", self.mode));

        for thought in &self.thoughts {
            output.push_str(&format!(
                "**[{}]** {}\n",
                thought.thought_type, thought.content
            ));

            // Find any actions linked to this thought
            for action in self.actions.iter().filter(|a| a.thought_id == thought.id) {
                output.push_str(&format!("  → Action: {}\n", action.description));
                if let Some(result) = &action.result {
                    if result.success {
                        output.push_str(&format!("  ✓ Result: {}\n", result.output));
                    } else {
                        output.push_str(&format!(
                            "  ✗ Error: {}\n",
                            result.error.as_deref().unwrap_or("unknown")
                        ));
                    }
                }
            }
            output.push('\n');
        }

        if let Some(answer) = &self.final_answer {
            output.push_str(&format!("**Final Answer:** {}\n", answer));
        }

        output
    }

    /// Format the trace for LLM context
    pub fn format_for_llm(&self) -> String {
        let mut context = String::from("REASONING TRACE:\n\n");

        // Include recent thoughts and actions
        let recent_thoughts: Vec<_> = self.thoughts.iter().rev().take(5).collect();
        for thought in recent_thoughts.into_iter().rev() {
            context.push_str(&format!(
                "[{}]: {}\n",
                thought.thought_type, thought.content
            ));

            for action in self.actions.iter().filter(|a| a.thought_id == thought.id) {
                context.push_str(&format!("ACTION: {}\n", action.description));
                if let Some(result) = &action.result {
                    if result.success {
                        let truncated = if result.output.len() > 500 {
                            format!("{}...", &result.output[..500])
                        } else {
                            result.output.clone()
                        };
                        context.push_str(&format!("RESULT: {}\n", truncated));
                    } else {
                        context.push_str(&format!(
                            "ERROR: {}\n",
                            result.error.as_deref().unwrap_or("failed")
                        ));
                    }
                }
            }
        }

        context
    }
}

/// Configuration for reasoning behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// The reasoning mode to use
    pub mode: ReasoningMode,
    /// Maximum number of reasoning steps
    pub max_steps: usize,
    /// Maximum depth for ToT branching
    pub max_depth: usize,
    /// Branching factor for ToT
    pub branching_factor: usize,
    /// Minimum confidence to proceed with action
    pub min_confidence: f32,
    /// Whether to enable self-critique
    pub enable_critique: bool,
    /// Whether to save reasoning traces
    pub save_traces: bool,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            mode: ReasoningMode::Standard,
            max_steps: 20,
            max_depth: 3,
            branching_factor: 3,
            min_confidence: 0.5,
            enable_critique: true,
            save_traces: true,
        }
    }
}

impl ReasoningConfig {
    /// Create a config for ReAct mode
    pub fn react() -> Self {
        Self {
            mode: ReasoningMode::ReAct,
            max_steps: 15,
            enable_critique: true,
            ..Default::default()
        }
    }

    /// Create a config for Chain-of-Thought mode
    pub fn chain_of_thought() -> Self {
        Self {
            mode: ReasoningMode::ChainOfThought,
            max_steps: 10,
            enable_critique: false,
            ..Default::default()
        }
    }

    /// Create a config for Tree-of-Thoughts mode
    pub fn tree_of_thoughts(branching_factor: usize, max_depth: usize) -> Self {
        Self {
            mode: ReasoningMode::TreeOfThoughts,
            max_steps: 30,
            max_depth,
            branching_factor,
            enable_critique: true,
            ..Default::default()
        }
    }
}

/// Manages reasoning traces and provides reasoning prompts
pub struct ReasoningManager {
    /// Configuration for reasoning
    config: ReasoningConfig,
    /// Current active trace
    current_trace: Option<ReActTrace>,
    /// History of completed traces
    trace_history: Vec<ReActTrace>,
}

impl ReasoningManager {
    /// Create a new reasoning manager
    pub fn new(config: ReasoningConfig) -> Self {
        Self {
            config,
            current_trace: None,
            trace_history: Vec::new(),
        }
    }

    /// Create with ReAct mode
    pub fn react() -> Self {
        Self::new(ReasoningConfig::react())
    }

    /// Create with Chain-of-Thought mode
    pub fn chain_of_thought() -> Self {
        Self::new(ReasoningConfig::chain_of_thought())
    }

    /// Get the current configuration
    pub fn config(&self) -> &ReasoningConfig {
        &self.config
    }

    /// Set the reasoning mode
    pub fn set_mode(&mut self, mode: ReasoningMode) {
        self.config.mode = mode;
    }

    /// Start a new reasoning trace
    pub fn start_trace(&mut self, task: impl Into<String>) -> &mut ReActTrace {
        // Save any existing trace
        if let Some(trace) = self.current_trace.take() {
            self.trace_history.push(trace);
        }

        self.current_trace = Some(ReActTrace::new(task, self.config.mode));
        self.current_trace.as_mut().unwrap()
    }

    /// Get the current trace
    pub fn current_trace(&self) -> Option<&ReActTrace> {
        self.current_trace.as_ref()
    }

    /// Get mutable reference to current trace
    pub fn current_trace_mut(&mut self) -> Option<&mut ReActTrace> {
        self.current_trace.as_mut()
    }

    /// Complete the current trace
    pub fn complete_trace(&mut self, answer: Option<String>) {
        if let Some(trace) = &mut self.current_trace {
            trace.mark_complete(answer);
        }
        if let Some(trace) = self.current_trace.take() {
            self.trace_history.push(trace);
        }
    }

    /// Get the trace history
    pub fn history(&self) -> &[ReActTrace] {
        &self.trace_history
    }

    /// Generate a ReAct-style system prompt
    pub fn generate_react_prompt(&self) -> String {
        r#"You are an AI assistant that uses the ReAct (Reasoning + Acting) framework.

For each step, you must:
1. THINK: Reason about what to do next based on the task and any observations
2. ACT: Choose an action to take (tool call)
3. OBSERVE: Analyze the result of the action

Format your response as:

THOUGHT: <your reasoning about the current situation and what to do next>
ACTION: <tool name>
ACTION_INPUT: <tool parameters as JSON>

After receiving the result, continue with:

OBSERVATION: <analysis of the action result>
THOUGHT: <next reasoning step>
...

When you have enough information to answer, respond with:

THOUGHT: <final reasoning>
FINAL_ANSWER: <your complete answer>

Important:
- Always think before acting
- Be explicit about your reasoning
- Learn from observations
- If an approach isn't working, try something different"#
            .to_string()
    }

    /// Generate a Chain-of-Thought prompt
    pub fn generate_cot_prompt(&self) -> String {
        r#"You are an AI assistant that uses step-by-step reasoning.

Before taking any action, think through the problem systematically:

1. Understand the problem completely
2. Break it down into sub-problems
3. Reason through each sub-problem
4. Synthesize the solution

Format your reasoning as:

STEP 1: <first reasoning step>
STEP 2: <second reasoning step>
...
CONCLUSION: <your conclusion and planned action>

Then provide your action or answer."#
            .to_string()
    }

    /// Generate a Tree-of-Thoughts prompt
    pub fn generate_tot_prompt(&self) -> String {
        format!(
            r#"You are an AI assistant that explores multiple solution paths.

For complex problems, generate {} alternative approaches and evaluate each:

BRANCH 1: <first approach>
EVALUATION: <assess viability, score 1-10>

BRANCH 2: <second approach>
EVALUATION: <assess viability, score 1-10>

...

SELECTED: <which branch to pursue and why>

Then proceed with the selected approach. If it doesn't work, backtrack and try another branch."#,
            self.config.branching_factor
        )
    }

    /// Get the appropriate system prompt for the current mode
    pub fn get_system_prompt(&self) -> String {
        match self.config.mode {
            ReasoningMode::Standard => String::new(),
            ReasoningMode::ChainOfThought => self.generate_cot_prompt(),
            ReasoningMode::ReAct => self.generate_react_prompt(),
            ReasoningMode::TreeOfThoughts => self.generate_tot_prompt(),
        }
    }

    /// Check if we've exceeded the maximum steps
    pub fn is_at_limit(&self) -> bool {
        self.current_trace
            .as_ref()
            .map(|t| t.thoughts.len() >= self.config.max_steps)
            .unwrap_or(false)
    }

    /// Get reasoning context for the LLM
    pub fn get_context(&self) -> Option<String> {
        self.current_trace.as_ref().map(|t| t.format_for_llm())
    }
}

impl Default for ReasoningManager {
    fn default() -> Self {
        Self::new(ReasoningConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_mode_parsing() {
        assert_eq!(
            "react".parse::<ReasoningMode>().unwrap(),
            ReasoningMode::ReAct
        );
        assert_eq!(
            "cot".parse::<ReasoningMode>().unwrap(),
            ReasoningMode::ChainOfThought
        );
        assert_eq!(
            "tot".parse::<ReasoningMode>().unwrap(),
            ReasoningMode::TreeOfThoughts
        );
        assert_eq!(
            "standard".parse::<ReasoningMode>().unwrap(),
            ReasoningMode::Standard
        );
    }

    #[test]
    fn test_react_trace_creation() {
        let mut trace = ReActTrace::new("Find the bug", ReasoningMode::ReAct);

        let t1 = trace.add_thought(
            "I need to first understand the error message",
            ThoughtType::Initial,
        );
        let a1 = trace.add_action(t1, "Read the error log file");

        trace.record_action_result(
            a1,
            ActionResult::success("Error: undefined variable 'foo'", 100),
        );

        let t2 = trace.add_observation(a1, "The error is about an undefined variable 'foo'");

        assert_eq!(trace.thoughts.len(), 2);
        assert_eq!(trace.actions.len(), 1);
        assert_eq!(trace.thoughts[t2].parent_id, Some(t1));
    }

    #[test]
    fn test_reasoning_manager() {
        let mut manager = ReasoningManager::react();

        let trace = manager.start_trace("Fix authentication bug");
        trace.add_thought("First, I need to find the auth code", ThoughtType::Initial);

        assert!(manager.current_trace().is_some());
        assert_eq!(manager.config().mode, ReasoningMode::ReAct);

        manager.complete_trace(Some(
            "Fixed the bug by updating the token validation".to_string(),
        ));
        assert!(manager.current_trace().is_none());
        assert_eq!(manager.history().len(), 1);
    }

    #[test]
    fn test_trace_formatting() {
        let mut trace = ReActTrace::new("Test task", ReasoningMode::ReAct);
        trace.add_thought("Initial analysis", ThoughtType::Initial);
        let t_id = trace.add_thought("Planning approach", ThoughtType::Planning);
        let a_id = trace.add_action(t_id, "Execute test command");
        trace.record_action_result(a_id, ActionResult::success("Tests passed", 500));

        let display = trace.format_for_display();
        assert!(display.contains("Initial analysis"));
        assert!(display.contains("Planning approach"));
        assert!(display.contains("Execute test command"));
    }

    #[test]
    fn test_thought_types() {
        let thought = Thought::new(0, "Test", ThoughtType::Analysis)
            .with_confidence(0.8)
            .with_evaluation(0.9);

        assert_eq!(thought.confidence, 0.8);
        assert_eq!(thought.evaluation, Some(0.9));
    }

    #[test]
    fn test_config_presets() {
        let react_config = ReasoningConfig::react();
        assert_eq!(react_config.mode, ReasoningMode::ReAct);
        assert!(react_config.enable_critique);

        let cot_config = ReasoningConfig::chain_of_thought();
        assert_eq!(cot_config.mode, ReasoningMode::ChainOfThought);
        assert!(!cot_config.enable_critique);

        let tot_config = ReasoningConfig::tree_of_thoughts(4, 5);
        assert_eq!(tot_config.mode, ReasoningMode::TreeOfThoughts);
        assert_eq!(tot_config.branching_factor, 4);
        assert_eq!(tot_config.max_depth, 5);
    }
}
