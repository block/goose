use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub mod runner;
pub mod state;

pub use runner::{ShellTestRunner, StateGraphRunner};
pub use state::{CodeTestFixState, TestResult, TestStatus};

use super::done_gate::{DoneGate, GateResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GraphState {
    #[default]
    Idle,
    Code,
    Test,
    Fix,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateGraphConfig {
    pub max_iterations: usize,
    pub max_fix_attempts: usize,
    pub test_command: Option<String>,
    pub working_dir: PathBuf,
    /// Whether to use DoneGate validation before transitioning to Done state
    #[serde(default)]
    pub use_done_gate: bool,
    /// Language/framework for done gate defaults (rust, node, python)
    #[serde(default)]
    pub project_type: Option<ProjectType>,
}

/// Project type for automatic done gate configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    #[default]
    Rust,
    Node,
    Python,
    Custom,
}

impl Default for StateGraphConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_fix_attempts: 3,
            test_command: None,
            working_dir: PathBuf::from("."),
            use_done_gate: true,
            project_type: None,
        }
    }
}

pub struct StateGraph {
    current_state: GraphState,
    config: StateGraphConfig,
    iteration: usize,
    fix_attempts: usize,
    state_data: CodeTestFixState,
    event_tx: Option<mpsc::Sender<StateGraphEvent>>,
    done_gate: Option<DoneGate>,
}

#[derive(Debug, Clone, Serialize)]
pub enum StateGraphEvent {
    StateTransition {
        from: GraphState,
        to: GraphState,
    },
    CodeGenerated {
        files: Vec<String>,
    },
    TestsRun {
        passed: usize,
        failed: usize,
        total: usize,
    },
    FixAttempted {
        attempt: usize,
        target_file: String,
    },
    DoneGateCheck {
        passed: bool,
        check_name: Option<String>,
        message: Option<String>,
    },
    Completed {
        success: bool,
        iterations: usize,
    },
}

/// Result of done gate verification
#[derive(Debug, Clone)]
pub enum DoneGateVerdict {
    /// All checks passed, ready to transition to Done
    Passed,
    /// Some check failed but can be fixed
    NeedsWork { check_name: String, message: String },
    /// Critical failure, cannot proceed
    Failed { reason: String },
    /// Done gate not configured, skip validation
    Skipped,
}

impl StateGraph {
    pub fn new(config: StateGraphConfig) -> Self {
        let done_gate = if config.use_done_gate {
            Some(Self::create_done_gate_for_project(&config))
        } else {
            None
        };

        Self {
            current_state: GraphState::Code,
            config,
            iteration: 0,
            fix_attempts: 0,
            state_data: CodeTestFixState::default(),
            event_tx: None,
            done_gate,
        }
    }

    /// Create appropriate DoneGate based on project type
    fn create_done_gate_for_project(config: &StateGraphConfig) -> DoneGate {
        match config.project_type {
            Some(ProjectType::Rust) | None => DoneGate::rust_defaults(),
            Some(ProjectType::Node) => DoneGate::node_defaults(),
            Some(ProjectType::Python) => DoneGate::python_defaults(),
            Some(ProjectType::Custom) => DoneGate::new(), // Empty gate, user adds custom checks
        }
    }

    /// Set a custom DoneGate (overrides auto-configured one)
    pub fn with_done_gate(mut self, gate: DoneGate) -> Self {
        self.done_gate = Some(gate);
        self
    }

    /// Disable the DoneGate
    pub fn without_done_gate(mut self) -> Self {
        self.done_gate = None;
        self
    }

    pub fn with_event_channel(mut self, tx: mpsc::Sender<StateGraphEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    pub fn set_event_channel(&mut self, tx: mpsc::Sender<StateGraphEvent>) {
        self.event_tx = Some(tx);
    }

    pub fn current_state(&self) -> GraphState {
        self.current_state
    }

    pub fn iteration(&self) -> usize {
        self.iteration
    }

    pub fn state_data(&self) -> &CodeTestFixState {
        &self.state_data
    }

    pub fn config(&self) -> &StateGraphConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut StateGraphConfig {
        &mut self.config
    }

    async fn emit_event(&self, event: StateGraphEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event).await;
        }
    }

    async fn transition_to(&mut self, new_state: GraphState) {
        let old_state = self.current_state;
        self.current_state = new_state;
        self.emit_event(StateGraphEvent::StateTransition {
            from: old_state,
            to: new_state,
        })
        .await;
        info!("StateGraph: {:?} -> {:?}", old_state, new_state);
    }

    /// Verify done gate and return the verdict
    async fn verify_done_gate(&self) -> DoneGateVerdict {
        let Some(gate) = &self.done_gate else {
            return DoneGateVerdict::Skipped;
        };

        match gate.verify(&self.config.working_dir) {
            Ok((result, check_results)) => {
                // Emit events for each check result
                for check in &check_results {
                    self.emit_event(StateGraphEvent::DoneGateCheck {
                        passed: check.passed,
                        check_name: Some(check.name.clone()),
                        message: Some(check.message.clone()),
                    })
                    .await;
                }

                match result {
                    GateResult::Done => DoneGateVerdict::Passed,
                    GateResult::ReEnterFix {
                        check_name,
                        message,
                    } => DoneGateVerdict::NeedsWork {
                        check_name,
                        message,
                    },
                    GateResult::Failed { reason } => DoneGateVerdict::Failed { reason },
                }
            }
            Err(e) => {
                error!("StateGraph: done gate verification error: {}", e);
                DoneGateVerdict::Failed {
                    reason: format!("Done gate error: {}", e),
                }
            }
        }
    }

    pub async fn run<F, G, H>(
        &mut self,
        task: &str,
        code_fn: F,
        test_fn: G,
        fix_fn: H,
    ) -> Result<bool>
    where
        F: Fn(&str, &CodeTestFixState) -> Result<Vec<String>>,
        G: Fn(&CodeTestFixState) -> Result<Vec<TestResult>>,
        H: Fn(&[TestResult], &CodeTestFixState) -> Result<Vec<String>>,
    {
        info!("StateGraph starting task: {}", task);
        self.state_data.task = task.to_string();
        self.current_state = GraphState::Code;

        loop {
            if self.iteration >= self.config.max_iterations {
                warn!(
                    "StateGraph: max iterations ({}) reached",
                    self.config.max_iterations
                );
                self.transition_to(GraphState::Failed).await;
                self.emit_event(StateGraphEvent::Completed {
                    success: false,
                    iterations: self.iteration,
                })
                .await;
                return Ok(false);
            }

            match self.current_state {
                GraphState::Idle => {
                    self.current_state = GraphState::Code;
                    continue;
                }

                GraphState::Code => {
                    self.iteration += 1;
                    info!("StateGraph: CODE phase (iteration {})", self.iteration);

                    match code_fn(task, &self.state_data) {
                        Ok(files) => {
                            self.state_data.generated_files = files.clone();
                            self.emit_event(StateGraphEvent::CodeGenerated { files })
                                .await;
                            self.transition_to(GraphState::Test).await;
                        }
                        Err(e) => {
                            error!("StateGraph: code generation failed: {}", e);
                            self.state_data.last_error = Some(e.to_string());
                            self.transition_to(GraphState::Failed).await;
                        }
                    }
                }

                GraphState::Test => {
                    info!("StateGraph: TEST phase");

                    match test_fn(&self.state_data) {
                        Ok(results) => {
                            let passed = results
                                .iter()
                                .filter(|r| r.status == TestStatus::Passed)
                                .count();
                            let failed = results
                                .iter()
                                .filter(|r| r.status == TestStatus::Failed)
                                .count();
                            let total = results.len();

                            self.emit_event(StateGraphEvent::TestsRun {
                                passed,
                                failed,
                                total,
                            })
                            .await;
                            self.state_data.test_results = results.clone();

                            if failed == 0 {
                                info!("StateGraph: all tests passed, checking done gate...");

                                // Run DoneGate validation if configured
                                match self.verify_done_gate().await {
                                    DoneGateVerdict::Passed => {
                                        info!("StateGraph: done gate passed!");
                                        self.transition_to(GraphState::Done).await;
                                    }
                                    DoneGateVerdict::NeedsWork {
                                        check_name,
                                        message,
                                    } => {
                                        info!(
                                            "StateGraph: done gate check '{}' failed: {}, entering FIX",
                                            check_name, message
                                        );
                                        self.state_data.last_error = Some(format!(
                                            "Done gate '{}' failed: {}",
                                            check_name, message
                                        ));
                                        self.fix_attempts = 0;
                                        self.transition_to(GraphState::Fix).await;
                                    }
                                    DoneGateVerdict::Failed { reason } => {
                                        error!(
                                            "StateGraph: done gate critically failed: {}",
                                            reason
                                        );
                                        self.state_data.last_error = Some(reason);
                                        self.transition_to(GraphState::Failed).await;
                                    }
                                    DoneGateVerdict::Skipped => {
                                        info!("StateGraph: done gate skipped (not configured)");
                                        self.transition_to(GraphState::Done).await;
                                    }
                                }
                            } else {
                                info!("StateGraph: {} tests failed, entering FIX phase", failed);
                                self.fix_attempts = 0;
                                self.transition_to(GraphState::Fix).await;
                            }
                        }
                        Err(e) => {
                            error!("StateGraph: test execution failed: {}", e);
                            self.state_data.last_error = Some(e.to_string());
                            self.transition_to(GraphState::Fix).await;
                        }
                    }
                }

                GraphState::Fix => {
                    self.fix_attempts += 1;
                    info!("StateGraph: FIX phase (attempt {})", self.fix_attempts);

                    if self.fix_attempts > self.config.max_fix_attempts {
                        warn!("StateGraph: max fix attempts reached, returning to CODE");
                        self.transition_to(GraphState::Code).await;
                        continue;
                    }

                    let failed_tests: Vec<_> = self
                        .state_data
                        .test_results
                        .iter()
                        .filter(|r| r.status == TestStatus::Failed)
                        .cloned()
                        .collect();

                    match fix_fn(&failed_tests, &self.state_data) {
                        Ok(fixed_files) => {
                            if let Some(file) = fixed_files.first() {
                                self.emit_event(StateGraphEvent::FixAttempted {
                                    attempt: self.fix_attempts,
                                    target_file: file.clone(),
                                })
                                .await;
                            }
                            self.state_data.fixed_files.extend(fixed_files);
                            self.transition_to(GraphState::Test).await;
                        }
                        Err(e) => {
                            error!("StateGraph: fix attempt failed: {}", e);
                            self.state_data.last_error = Some(e.to_string());
                            if self.fix_attempts >= self.config.max_fix_attempts {
                                self.transition_to(GraphState::Code).await;
                            }
                        }
                    }
                }

                GraphState::Done => {
                    info!("StateGraph: DONE - task completed successfully");
                    self.emit_event(StateGraphEvent::Completed {
                        success: true,
                        iterations: self.iteration,
                    })
                    .await;
                    return Ok(true);
                }

                GraphState::Failed => {
                    error!("StateGraph: FAILED - task could not be completed");
                    self.emit_event(StateGraphEvent::Completed {
                        success: false,
                        iterations: self.iteration,
                    })
                    .await;
                    return Ok(false);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config_no_gate() -> StateGraphConfig {
        StateGraphConfig {
            max_iterations: 5,
            max_fix_attempts: 3,
            use_done_gate: false, // Disable for unit tests
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_state_graph_success() {
        let config = test_config_no_gate();
        let mut graph = StateGraph::new(config);

        let result = graph
            .run(
                "test task",
                |_task, _state| Ok(vec!["main.rs".to_string()]),
                |_state| {
                    Ok(vec![TestResult {
                        file: "main.rs".to_string(),
                        line: Some(10),
                        test_name: "test_example".to_string(),
                        status: TestStatus::Passed,
                        message: None,
                        expected: None,
                        actual: None,
                    }])
                },
                |_failed, _state| Ok(vec![]),
            )
            .await
            .unwrap();

        assert!(result);
        assert_eq!(graph.current_state(), GraphState::Done);
    }

    #[tokio::test]
    async fn test_state_graph_fix_cycle() {
        let config = StateGraphConfig {
            max_iterations: 10,
            max_fix_attempts: 2,
            use_done_gate: false,
            ..Default::default()
        };
        let mut graph = StateGraph::new(config);
        let fix_count = std::sync::atomic::AtomicUsize::new(0);

        let result = graph
            .run(
                "test task",
                |_task, _state| Ok(vec!["main.rs".to_string()]),
                |state| {
                    if state.fixed_files.len() >= 2 {
                        Ok(vec![TestResult {
                            file: "main.rs".to_string(),
                            line: Some(10),
                            test_name: "test_example".to_string(),
                            status: TestStatus::Passed,
                            message: None,
                            expected: None,
                            actual: None,
                        }])
                    } else {
                        Ok(vec![TestResult {
                            file: "main.rs".to_string(),
                            line: Some(10),
                            test_name: "test_example".to_string(),
                            status: TestStatus::Failed,
                            message: Some("assertion failed".to_string()),
                            expected: Some("true".to_string()),
                            actual: Some("false".to_string()),
                        }])
                    }
                },
                |_failed, _state| {
                    fix_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(vec!["main.rs".to_string()])
                },
            )
            .await
            .unwrap();

        assert!(result);
        assert_eq!(graph.current_state(), GraphState::Done);
    }

    #[tokio::test]
    async fn test_state_graph_without_done_gate() {
        let config = StateGraphConfig {
            use_done_gate: false,
            ..Default::default()
        };
        let graph = StateGraph::new(config);
        assert!(graph.done_gate.is_none());
    }

    #[tokio::test]
    async fn test_state_graph_with_done_gate() {
        let config = StateGraphConfig {
            use_done_gate: true,
            project_type: Some(ProjectType::Rust),
            ..Default::default()
        };
        let graph = StateGraph::new(config);
        assert!(graph.done_gate.is_some());
    }

    #[tokio::test]
    async fn test_project_type_affects_gate() {
        let rust_config = StateGraphConfig {
            use_done_gate: true,
            project_type: Some(ProjectType::Rust),
            ..Default::default()
        };
        let rust_graph = StateGraph::new(rust_config);
        assert!(rust_graph.done_gate.is_some());

        let node_config = StateGraphConfig {
            use_done_gate: true,
            project_type: Some(ProjectType::Node),
            ..Default::default()
        };
        let node_graph = StateGraph::new(node_config);
        assert!(node_graph.done_gate.is_some());

        let python_config = StateGraphConfig {
            use_done_gate: true,
            project_type: Some(ProjectType::Python),
            ..Default::default()
        };
        let python_graph = StateGraph::new(python_config);
        assert!(python_graph.done_gate.is_some());
    }

    #[test]
    fn test_done_gate_verdict_variants() {
        let passed = DoneGateVerdict::Passed;
        assert!(matches!(passed, DoneGateVerdict::Passed));

        let needs_work = DoneGateVerdict::NeedsWork {
            check_name: "build".to_string(),
            message: "build failed".to_string(),
        };
        assert!(matches!(needs_work, DoneGateVerdict::NeedsWork { .. }));

        let failed = DoneGateVerdict::Failed {
            reason: "critical error".to_string(),
        };
        assert!(matches!(failed, DoneGateVerdict::Failed { .. }));

        let skipped = DoneGateVerdict::Skipped;
        assert!(matches!(skipped, DoneGateVerdict::Skipped));
    }
}
