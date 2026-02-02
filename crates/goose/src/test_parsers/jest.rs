use serde::Deserialize;

use super::{ParseError, TestFramework, TestOutputParser, TestResult, TestStatus};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JestJsonReport {
    #[serde(default)]
    pub test_results: Vec<JestTestSuite>,
    #[serde(default)]
    pub num_failed_tests: u32,
    #[serde(default)]
    pub num_passed_tests: u32,
    #[serde(default)]
    pub num_pending_tests: u32,
    #[serde(default)]
    pub num_total_tests: u32,
    #[serde(default)]
    pub success: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JestTestSuite {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub assertion_results: Vec<JestAssertionResult>,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JestAssertionResult {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub failure_messages: Vec<String>,
    #[serde(default)]
    pub location: Option<JestLocation>,
}

#[derive(Debug, Deserialize)]
pub struct JestLocation {
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub column: Option<u32>,
}

pub struct JestParser;

impl TestOutputParser for JestParser {
    fn framework(&self) -> TestFramework {
        TestFramework::Jest
    }

    fn parse(&self, output: &str) -> Result<Vec<TestResult>, ParseError> {
        if let Ok(report) = serde_json::from_str::<JestJsonReport>(output) {
            let mut results = Vec::new();
            for suite in report.test_results {
                let file = suite.name.clone();
                for assertion in suite.assertion_results {
                    results.push(convert_jest_assertion(&file, assertion));
                }
            }
            if !results.is_empty() {
                return Ok(results);
            }
        }

        parse_jest_text_output(output)
    }
}

fn convert_jest_assertion(file: &str, assertion: JestAssertionResult) -> TestResult {
    let line = assertion.location.as_ref().and_then(|l| l.line);
    let message = if assertion.failure_messages.is_empty() {
        None
    } else {
        Some(assertion.failure_messages.join("\n"))
    };

    TestResult {
        file: file.to_string(),
        line,
        test_name: if assertion.full_name.is_empty() {
            assertion.title
        } else {
            assertion.full_name
        },
        status: match assertion.status.to_lowercase().as_str() {
            "passed" => TestStatus::Passed,
            "failed" => TestStatus::Failed,
            "pending" | "skipped" | "todo" => TestStatus::Skipped,
            _ => TestStatus::Error,
        },
        message,
        expected: None,
        actual: None,
    }
}

fn parse_jest_text_output(output: &str) -> Result<Vec<TestResult>, ParseError> {
    let mut results = Vec::new();
    let mut current_file = String::new();

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("PASS ") || line.starts_with("FAIL ") {
            current_file = line.split_whitespace().nth(1).unwrap_or("").to_string();
            continue;
        }

        if line.starts_with("✓") || line.starts_with("√") {
            let test_name = line.trim_start_matches(['✓', '√', ' ']).to_string();
            let test_name = test_name
                .split(" (")
                .next()
                .unwrap_or(&test_name)
                .trim()
                .to_string();

            results.push(TestResult {
                file: current_file.clone(),
                line: None,
                test_name,
                status: TestStatus::Passed,
                message: None,
                expected: None,
                actual: None,
            });
        }

        if line.starts_with("✕") || line.starts_with("×") {
            let test_name = line.trim_start_matches(['✕', '×', ' ']).to_string();
            let test_name = test_name
                .split(" (")
                .next()
                .unwrap_or(&test_name)
                .trim()
                .to_string();

            results.push(TestResult {
                file: current_file.clone(),
                line: None,
                test_name,
                status: TestStatus::Failed,
                message: None,
                expected: None,
                actual: None,
            });
        }

        if line.starts_with("○") {
            let test_name = line.trim_start_matches(['○', ' ']).to_string();
            let test_name = test_name
                .split(" (")
                .next()
                .unwrap_or(&test_name)
                .trim()
                .to_string();

            results.push(TestResult {
                file: current_file.clone(),
                line: None,
                test_name,
                status: TestStatus::Skipped,
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

#[allow(dead_code)]
fn extract_expected_actual(message: &str) -> (Option<String>, Option<String>) {
    let mut expected = None;
    let mut actual = None;

    for line in message.lines() {
        let line = line.trim();
        if line.starts_with("Expected:") {
            expected = Some(line.trim_start_matches("Expected:").trim().to_string());
        } else if line.starts_with("Received:") {
            actual = Some(line.trim_start_matches("Received:").trim().to_string());
        }
    }

    (expected, actual)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jest_json() {
        let json = r#"{
            "testResults": [
                {
                    "name": "src/__tests__/math.test.js",
                    "status": "failed",
                    "assertionResults": [
                        {"title": "adds numbers", "fullName": "Math adds numbers", "status": "passed", "failureMessages": []},
                        {"title": "subtracts numbers", "fullName": "Math subtracts numbers", "status": "failed", "failureMessages": ["Expected: 5\nReceived: 3"]}
                    ]
                }
            ],
            "numFailedTests": 1,
            "numPassedTests": 1,
            "success": false
        }"#;

        let results = JestParser.parse(json).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].status, TestStatus::Passed);
        assert_eq!(results[1].status, TestStatus::Failed);
        assert!(results[1].message.is_some());
    }

    #[test]
    fn test_parse_jest_text() {
        let output = r#"
PASS src/__tests__/math.test.js
  ✓ adds numbers (5 ms)
  ✓ multiplies numbers (2 ms)

FAIL src/__tests__/utils.test.js
  ✕ validates input (10 ms)
        "#;

        let results = JestParser.parse(output).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, TestStatus::Passed);
        assert_eq!(results[2].status, TestStatus::Failed);
    }

    #[test]
    fn test_extract_expected_actual() {
        let message = "Expected: 5\nReceived: 3";
        let (expected, actual) = extract_expected_actual(message);
        assert_eq!(expected, Some("5".to_string()));
        assert_eq!(actual, Some("3".to_string()));
    }
}
