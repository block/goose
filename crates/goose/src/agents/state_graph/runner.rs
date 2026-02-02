use super::{CodeTestFixState, GraphState, StateGraph, StateGraphConfig, StateGraphEvent};
use crate::test_parsers::{parse_test_output, TestFramework, TestResult};
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Callback types for StateGraph operations
pub type CodeGenFn = Box<dyn Fn(&str, &CodeTestFixState) -> Result<Vec<String>> + Send + Sync>;
pub type TestRunFn = Box<dyn Fn(&CodeTestFixState) -> Result<Vec<TestResult>> + Send + Sync>;
pub type FixApplyFn =
    Box<dyn Fn(&[TestResult], &CodeTestFixState) -> Result<Vec<String>> + Send + Sync>;

/// Runner for executing StateGraph with provided callbacks
pub struct StateGraphRunner {
    graph: StateGraph,
    event_rx: Option<mpsc::Receiver<StateGraphEvent>>,
}

impl StateGraphRunner {
    pub fn new(config: StateGraphConfig) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let mut graph = StateGraph::new(config);
        graph.set_event_channel(tx);
        Self {
            graph,
            event_rx: Some(rx),
        }
    }

    pub fn with_defaults(working_dir: PathBuf) -> Self {
        Self::new(StateGraphConfig {
            max_iterations: 10,
            max_fix_attempts: 3,
            test_command: None,
            working_dir,
            use_done_gate: true,
            project_type: None,
        })
    }

    pub fn with_test_command(mut self, cmd: &str) -> Self {
        self.graph.config_mut().test_command = Some(cmd.to_string());
        self
    }

    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<StateGraphEvent>> {
        self.event_rx.take()
    }

    pub async fn run(
        &mut self,
        task: &str,
        code_gen: CodeGenFn,
        test_run: TestRunFn,
        fix_apply: FixApplyFn,
    ) -> Result<bool> {
        self.graph.run(task, code_gen, test_run, fix_apply).await
    }

    pub fn current_state(&self) -> GraphState {
        self.graph.current_state()
    }

    pub fn iteration(&self) -> usize {
        self.graph.iteration()
    }
}

/// Pre-built test runner using shell commands
pub struct ShellTestRunner {
    test_command: String,
    #[allow(dead_code)]
    working_dir: PathBuf,
    framework: TestFramework,
}

impl ShellTestRunner {
    pub fn new(test_command: &str, working_dir: PathBuf) -> Self {
        let framework = TestFramework::detect_from_command(test_command);
        Self {
            test_command: test_command.to_string(),
            working_dir,
            framework,
        }
    }

    pub fn with_framework(mut self, framework: TestFramework) -> Self {
        self.framework = framework;
        self
    }

    pub async fn run_tests(&self) -> Result<Vec<TestResult>> {
        use crate::agents::retry::execute_shell_command;
        use std::time::Duration;

        let timeout = Duration::from_secs(300);
        let output = execute_shell_command(&self.test_command, timeout).await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        let results = parse_test_output(&combined, self.framework.clone());

        if results.is_empty() && !output.status.success() {
            warn!(
                "Test command failed but no results parsed. Exit code: {:?}",
                output.status.code()
            );
        }

        info!(
            "Ran {} tests, {} passed",
            results.len(),
            results.iter().filter(|r| r.is_passed()).count()
        );

        Ok(results)
    }

    pub fn into_callback(self) -> TestRunFn {
        let runner = std::sync::Arc::new(self);
        Box::new(move |_state| {
            let runner = runner.clone();
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async { runner.run_tests().await })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let runner = StateGraphRunner::with_defaults(PathBuf::from("."));
        assert_eq!(runner.current_state(), GraphState::Code);
        assert_eq!(runner.iteration(), 0);
    }

    #[test]
    fn test_shell_test_runner_framework_detection() {
        let runner = ShellTestRunner::new("cargo test", PathBuf::from("."));
        assert_eq!(runner.framework, TestFramework::Cargo);

        let runner = ShellTestRunner::new("pytest tests/", PathBuf::from("."));
        assert_eq!(runner.framework, TestFramework::Pytest);

        let runner = ShellTestRunner::new("npm test", PathBuf::from("."));
        assert_eq!(runner.framework, TestFramework::Jest);
    }
}
