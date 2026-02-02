use serde::{Deserialize, Serialize};

pub mod jest;
pub mod pytest;

pub use crate::agents::state_graph::state::{TestResult, TestStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestFramework {
    Pytest,
    Jest,
    Cargo,
    Go,
    Unknown,
}

impl TestFramework {
    pub fn detect_from_command(cmd: &str) -> Self {
        let cmd_lower = cmd.to_lowercase();
        if cmd_lower.contains("pytest") || cmd_lower.contains("python -m pytest") {
            Self::Pytest
        } else if cmd_lower.contains("jest")
            || cmd_lower.contains("npm test")
            || cmd_lower.contains("yarn test")
        {
            Self::Jest
        } else if cmd_lower.contains("cargo test") {
            Self::Cargo
        } else if cmd_lower.contains("go test") {
            Self::Go
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    InvalidJson(String),
    MissingField(String),
    FallbackToRaw(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidJson(msg) => write!(f, "Invalid JSON: {}", msg),
            ParseError::MissingField(field) => write!(f, "Missing field: {}", field),
            ParseError::FallbackToRaw(raw) => {
                write!(f, "Fallback to raw output: {} bytes", raw.len())
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub trait TestOutputParser {
    fn parse(&self, output: &str) -> Result<Vec<TestResult>, ParseError>;
    fn framework(&self) -> TestFramework;
}

pub fn parse_test_output(output: &str, framework: TestFramework) -> Vec<TestResult> {
    let result = match framework {
        TestFramework::Pytest => pytest::PytestParser.parse(output),
        TestFramework::Jest => jest::JestParser.parse(output),
        TestFramework::Cargo => parse_cargo_output(output),
        TestFramework::Go => parse_go_output(output),
        TestFramework::Unknown => parse_generic_output(output),
    };

    match result {
        Ok(results) => results,
        Err(ParseError::FallbackToRaw(raw)) => {
            vec![TestResult {
                file: "unknown".to_string(),
                line: None,
                test_name: "raw_output".to_string(),
                status: if raw.contains("FAILED") || raw.contains("FAIL") || raw.contains("error") {
                    TestStatus::Failed
                } else if raw.contains("PASSED") || raw.contains("PASS") || raw.contains("ok") {
                    TestStatus::Passed
                } else {
                    TestStatus::Error
                },
                message: Some(raw),
                expected: None,
                actual: None,
            }]
        }
        Err(e) => {
            vec![TestResult {
                file: "unknown".to_string(),
                line: None,
                test_name: "parse_error".to_string(),
                status: TestStatus::Error,
                message: Some(e.to_string()),
                expected: None,
                actual: None,
            }]
        }
    }
}

fn parse_cargo_output(output: &str) -> Result<Vec<TestResult>, ParseError> {
    let mut results = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("test ") && (line.contains(" ... ok") || line.contains(" ... FAILED")) {
            let parts: Vec<&str> = line.split(" ... ").collect();
            if parts.len() >= 2 {
                let test_name = parts[0].trim_start_matches("test ").to_string();
                let status = if parts[1].contains("ok") {
                    TestStatus::Passed
                } else if parts[1].contains("FAILED") {
                    TestStatus::Failed
                } else {
                    TestStatus::Skipped
                };

                results.push(TestResult {
                    file: "unknown".to_string(),
                    line: None,
                    test_name,
                    status,
                    message: None,
                    expected: None,
                    actual: None,
                });
            }
        }
    }

    if results.is_empty() {
        Err(ParseError::FallbackToRaw(output.to_string()))
    } else {
        Ok(results)
    }
}

fn parse_go_output(output: &str) -> Result<Vec<TestResult>, ParseError> {
    let mut results = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("--- PASS:") || line.starts_with("--- FAIL:") {
            let is_pass = line.starts_with("--- PASS:");
            let prefix = if is_pass { "--- PASS: " } else { "--- FAIL: " };
            let rest = line.trim_start_matches(prefix);

            let test_name = rest
                .split_whitespace()
                .next()
                .unwrap_or("unknown")
                .to_string();

            results.push(TestResult {
                file: "unknown".to_string(),
                line: None,
                test_name,
                status: if is_pass {
                    TestStatus::Passed
                } else {
                    TestStatus::Failed
                },
                message: None,
                expected: None,
                actual: None,
            });
        }
    }

    if results.is_empty() {
        Err(ParseError::FallbackToRaw(output.to_string()))
    } else {
        Ok(results)
    }
}

fn parse_generic_output(output: &str) -> Result<Vec<TestResult>, ParseError> {
    Err(ParseError::FallbackToRaw(output.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_detection() {
        assert!(matches!(
            TestFramework::detect_from_command("pytest tests/"),
            TestFramework::Pytest
        ));
        assert!(matches!(
            TestFramework::detect_from_command("npm test"),
            TestFramework::Jest
        ));
        assert!(matches!(
            TestFramework::detect_from_command("cargo test"),
            TestFramework::Cargo
        ));
        assert!(matches!(
            TestFramework::detect_from_command("go test ./..."),
            TestFramework::Go
        ));
    }

    #[test]
    fn test_cargo_output_parsing() {
        let output = r#"
running 3 tests
test tests::test_add ... ok
test tests::test_sub ... ok
test tests::test_mul ... FAILED
        "#;

        let results = parse_cargo_output(output).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, TestStatus::Passed);
        assert_eq!(results[2].status, TestStatus::Failed);
    }

    #[test]
    fn test_go_output_parsing() {
        let output = r#"
=== RUN   TestAdd
--- PASS: TestAdd (0.00s)
=== RUN   TestSub
--- FAIL: TestSub (0.00s)
        "#;

        let results = parse_go_output(output).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].test_name, "TestAdd");
        assert_eq!(results[0].status, TestStatus::Passed);
        assert_eq!(results[1].status, TestStatus::Failed);
    }
}
