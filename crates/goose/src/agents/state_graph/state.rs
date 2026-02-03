use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub file: String,
    pub line: Option<u32>,
    pub test_name: String,
    pub status: TestStatus,
    pub message: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl TestResult {
    pub fn passed(file: &str, test_name: &str) -> Self {
        Self {
            file: file.to_string(),
            line: None,
            test_name: test_name.to_string(),
            status: TestStatus::Passed,
            message: None,
            expected: None,
            actual: None,
        }
    }

    pub fn failed(file: &str, test_name: &str, message: &str) -> Self {
        Self {
            file: file.to_string(),
            line: None,
            test_name: test_name.to_string(),
            status: TestStatus::Failed,
            message: Some(message.to_string()),
            expected: None,
            actual: None,
        }
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_expected_actual(mut self, expected: &str, actual: &str) -> Self {
        self.expected = Some(expected.to_string());
        self.actual = Some(actual.to_string());
        self
    }

    pub fn is_passed(&self) -> bool {
        self.status == TestStatus::Passed
    }

    pub fn is_failed(&self) -> bool {
        self.status == TestStatus::Failed
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeTestFixState {
    pub task: String,
    pub generated_files: Vec<String>,
    pub test_results: Vec<TestResult>,
    pub fixed_files: Vec<String>,
    pub last_error: Option<String>,
    pub context: std::collections::HashMap<String, String>,
}

impl CodeTestFixState {
    pub fn new(task: &str) -> Self {
        Self {
            task: task.to_string(),
            ..Default::default()
        }
    }

    pub fn has_failures(&self) -> bool {
        self.test_results
            .iter()
            .any(|r| r.status == TestStatus::Failed)
    }

    pub fn failed_tests(&self) -> Vec<&TestResult> {
        self.test_results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .collect()
    }

    pub fn passed_tests(&self) -> Vec<&TestResult> {
        self.test_results
            .iter()
            .filter(|r| r.status == TestStatus::Passed)
            .collect()
    }

    pub fn failure_summary(&self) -> String {
        let failures = self.failed_tests();
        if failures.is_empty() {
            return "No failures".to_string();
        }

        failures
            .iter()
            .map(|f| {
                let location = match f.line {
                    Some(line) => format!("{}:{}", f.file, line),
                    None => f.file.clone(),
                };
                let msg = f.message.as_deref().unwrap_or("no message");
                format!("- {} in {}: {}", f.test_name, location, msg)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_builders() {
        let passed = TestResult::passed("main.rs", "test_add");
        assert_eq!(passed.status, TestStatus::Passed);

        let failed = TestResult::failed("main.rs", "test_sub", "expected 5, got 3")
            .with_line(42)
            .with_expected_actual("5", "3");

        assert_eq!(failed.status, TestStatus::Failed);
        assert_eq!(failed.line, Some(42));
        assert_eq!(failed.expected, Some("5".to_string()));
    }

    #[test]
    fn test_state_failure_summary() {
        let mut state = CodeTestFixState::new("test task");
        state.test_results = vec![
            TestResult::passed("main.rs", "test_add"),
            TestResult::failed("main.rs", "test_sub", "assertion failed").with_line(42),
        ];

        assert!(state.has_failures());
        assert_eq!(state.failed_tests().len(), 1);
        assert_eq!(state.passed_tests().len(), 1);

        let summary = state.failure_summary();
        assert!(summary.contains("test_sub"));
        assert!(summary.contains("42"));
    }
}
