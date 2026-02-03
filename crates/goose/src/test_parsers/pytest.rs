use serde::Deserialize;

use super::{ParseError, TestFramework, TestOutputParser, TestResult, TestStatus};

#[derive(Debug, Deserialize)]
pub struct PytestJsonReport {
    #[serde(default)]
    pub tests: Vec<PytestTest>,
    #[serde(default)]
    pub summary: Option<PytestSummary>,
    #[serde(default)]
    pub collectors: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct PytestTest {
    pub nodeid: String,
    pub outcome: String,
    #[serde(default)]
    pub longrepr: Option<String>,
    #[serde(default)]
    pub lineno: Option<u32>,
    #[serde(default)]
    pub keywords: Option<Vec<String>>,
    #[serde(default)]
    pub duration: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct PytestSummary {
    #[serde(default)]
    pub passed: u32,
    #[serde(default)]
    pub failed: u32,
    #[serde(default)]
    pub error: u32,
    #[serde(default)]
    pub skipped: u32,
    #[serde(default)]
    pub total: u32,
}

#[derive(Debug, Deserialize)]
struct PytestReportPlugin {
    #[serde(default)]
    pub tests: Vec<PytestReportTest>,
}

#[derive(Debug, Deserialize)]
struct PytestReportTest {
    pub nodeid: String,
    pub outcome: String,
    #[serde(default)]
    pub call: Option<PytestCall>,
    #[serde(default)]
    pub lineno: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PytestCall {
    #[serde(default)]
    pub longrepr: Option<String>,
    #[serde(default)]
    pub crash: Option<PytestCrash>,
}

#[derive(Debug, Deserialize)]
struct PytestCrash {
    #[serde(default)]
    #[allow(dead_code)]
    pub path: Option<String>,
    #[serde(default)]
    pub lineno: Option<u32>,
    #[serde(default)]
    pub message: Option<String>,
}

pub struct PytestParser;

impl TestOutputParser for PytestParser {
    fn framework(&self) -> TestFramework {
        TestFramework::Pytest
    }

    fn parse(&self, output: &str) -> Result<Vec<TestResult>, ParseError> {
        if let Ok(report) = serde_json::from_str::<PytestJsonReport>(output) {
            return Ok(report.tests.into_iter().map(convert_pytest_test).collect());
        }

        if let Ok(report) = serde_json::from_str::<PytestReportPlugin>(output) {
            return Ok(report
                .tests
                .into_iter()
                .map(convert_pytest_report_test)
                .collect());
        }

        if let Ok(tests) = serde_json::from_str::<Vec<PytestTest>>(output) {
            return Ok(tests.into_iter().map(convert_pytest_test).collect());
        }

        parse_pytest_text_output(output)
    }
}

fn convert_pytest_test(test: PytestTest) -> TestResult {
    let (file, test_name) = parse_nodeid(&test.nodeid);

    TestResult {
        file,
        line: test.lineno,
        test_name,
        status: match test.outcome.to_lowercase().as_str() {
            "passed" => TestStatus::Passed,
            "failed" => TestStatus::Failed,
            "skipped" => TestStatus::Skipped,
            "error" => TestStatus::Error,
            _ => TestStatus::Error,
        },
        message: test.longrepr,
        expected: None,
        actual: None,
    }
}

fn convert_pytest_report_test(test: PytestReportTest) -> TestResult {
    let (file, test_name) = parse_nodeid(&test.nodeid);

    let (message, line) = if let Some(call) = &test.call {
        let msg = call
            .longrepr
            .clone()
            .or_else(|| call.crash.as_ref().and_then(|c| c.message.clone()));
        let ln = test
            .lineno
            .or_else(|| call.crash.as_ref().and_then(|c| c.lineno));
        (msg, ln)
    } else {
        (None, test.lineno)
    };

    TestResult {
        file,
        line,
        test_name,
        status: match test.outcome.to_lowercase().as_str() {
            "passed" => TestStatus::Passed,
            "failed" => TestStatus::Failed,
            "skipped" => TestStatus::Skipped,
            "error" => TestStatus::Error,
            _ => TestStatus::Error,
        },
        message,
        expected: None,
        actual: None,
    }
}

fn parse_nodeid(nodeid: &str) -> (String, String) {
    if let Some(pos) = nodeid.find("::") {
        // Using get() with char_indices to safely handle UTF-8
        let (file, rest) = nodeid.split_at(pos);
        let test_name = rest.strip_prefix("::").unwrap_or(rest).replace("::", ".");
        (file.to_string(), test_name)
    } else {
        ("unknown".to_string(), nodeid.to_string())
    }
}

fn parse_pytest_text_output(output: &str) -> Result<Vec<TestResult>, ParseError> {
    let mut results = Vec::new();
    let lines: Vec<&str> = output.lines().collect();

    for line in lines.iter() {
        let line = line.trim();

        if line.starts_with("PASSED")
            || line.starts_with("FAILED")
            || line.starts_with("ERROR")
            || line.starts_with("SKIPPED")
        {
            let status = if line.starts_with("PASSED") {
                TestStatus::Passed
            } else if line.starts_with("FAILED") {
                TestStatus::Failed
            } else if line.starts_with("SKIPPED") {
                TestStatus::Skipped
            } else {
                TestStatus::Error
            };

            let test_info = line.split_whitespace().nth(1).unwrap_or("unknown");
            let (file, test_name) = parse_nodeid(test_info);

            results.push(TestResult {
                file,
                line: None,
                test_name,
                status,
                message: None,
                expected: None,
                actual: None,
            });
        }

        if line.contains("::")
            && (line.ends_with(" PASSED") || line.ends_with(" FAILED") || line.ends_with(" ERROR"))
        {
            let parts: Vec<&str> = line.rsplitn(2, ' ').collect();
            if parts.len() == 2 {
                let test_info = parts[1].trim();
                let status_str = parts[0];

                let (file, test_name) = parse_nodeid(test_info);
                let status = match status_str {
                    "PASSED" => TestStatus::Passed,
                    "FAILED" => TestStatus::Failed,
                    "ERROR" => TestStatus::Error,
                    _ => TestStatus::Skipped,
                };

                results.push(TestResult {
                    file,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nodeid() {
        let (file, test) = parse_nodeid("tests/test_math.py::test_add");
        assert_eq!(file, "tests/test_math.py");
        assert_eq!(test, "test_add");

        let (file, test) = parse_nodeid("tests/test_math.py::TestClass::test_method");
        assert_eq!(file, "tests/test_math.py");
        assert_eq!(test, "TestClass.test_method");
    }

    #[test]
    fn test_parse_pytest_json() {
        let json = r#"{
            "tests": [
                {"nodeid": "tests/test_math.py::test_add", "outcome": "passed"},
                {"nodeid": "tests/test_math.py::test_sub", "outcome": "failed", "longrepr": "assert 1 == 2"}
            ]
        }"#;

        let results = PytestParser.parse(json).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].status, TestStatus::Passed);
        assert_eq!(results[1].status, TestStatus::Failed);
        assert_eq!(results[1].message, Some("assert 1 == 2".to_string()));
    }

    #[test]
    fn test_parse_pytest_text() {
        let output = r#"
tests/test_math.py::test_add PASSED
tests/test_math.py::test_sub FAILED
        "#;

        let results = PytestParser.parse(output).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].test_name, "test_add");
        assert_eq!(results[1].status, TestStatus::Failed);
    }
}
