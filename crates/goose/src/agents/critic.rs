//! Self-reflection and critique system for agent work validation
//!
//! This module provides the ability for the agent to evaluate its own work,
//! identify potential issues, and suggest improvements before considering
//! a task complete.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Severity level of a critique issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Minor issue - suggestions for improvement
    Low,
    /// Moderate issue - should be addressed
    Medium,
    /// Significant issue - must be addressed
    High,
    /// Critical issue - blocks completion
    Critical,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueSeverity::Low => write!(f, "low"),
            IssueSeverity::Medium => write!(f, "medium"),
            IssueSeverity::High => write!(f, "high"),
            IssueSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Category of issue found during critique
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    /// Code quality issues (style, complexity, etc.)
    CodeQuality,
    /// Potential bugs or logic errors
    Bug,
    /// Security vulnerabilities
    Security,
    /// Performance concerns
    Performance,
    /// Missing or incomplete functionality
    Incomplete,
    /// Test coverage gaps
    TestCoverage,
    /// Documentation issues
    Documentation,
    /// Type or compilation errors
    TypeError,
    /// Other issues
    Other(String),
}

impl std::fmt::Display for IssueCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueCategory::CodeQuality => write!(f, "code_quality"),
            IssueCategory::Bug => write!(f, "bug"),
            IssueCategory::Security => write!(f, "security"),
            IssueCategory::Performance => write!(f, "performance"),
            IssueCategory::Incomplete => write!(f, "incomplete"),
            IssueCategory::TestCoverage => write!(f, "test_coverage"),
            IssueCategory::Documentation => write!(f, "documentation"),
            IssueCategory::TypeError => write!(f, "type_error"),
            IssueCategory::Other(s) => write!(f, "{}", s),
        }
    }
}

/// A single issue identified during critique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueIssue {
    /// Severity of the issue
    pub severity: IssueSeverity,
    /// Category of the issue
    pub category: IssueCategory,
    /// File where the issue was found (if applicable)
    pub file: Option<String>,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Description of the issue
    pub description: String,
    /// Suggested fix or action
    pub suggestion: Option<String>,
}

impl CritiqueIssue {
    pub fn new(
        severity: IssueSeverity,
        category: IssueCategory,
        description: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category,
            file: None,
            line: None,
            description: description.into(),
            suggestion: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Check if this is a blocking issue (high or critical severity)
    pub fn is_blocking(&self) -> bool {
        matches!(self.severity, IssueSeverity::High | IssueSeverity::Critical)
    }

    /// Format for display
    pub fn format_display(&self) -> String {
        let mut parts = vec![format!(
            "[{}] {}: {}",
            self.severity, self.category, self.description
        )];

        if let Some(file) = &self.file {
            if let Some(line) = self.line {
                parts.push(format!("  at {}:{}", file, line));
            } else {
                parts.push(format!("  in {}", file));
            }
        }

        if let Some(suggestion) = &self.suggestion {
            parts.push(format!("  suggestion: {}", suggestion));
        }

        parts.join("\n")
    }
}

/// Result of a critique operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueResult {
    /// Name of the critic that produced this result
    pub critic_name: String,
    /// Issues found during critique
    pub issues: Vec<CritiqueIssue>,
    /// Overall assessment summary
    pub summary: String,
    /// Whether the work passes this critic's review
    pub passed: bool,
    /// Confidence level in the critique (0.0 to 1.0)
    pub confidence: f32,
}

impl CritiqueResult {
    pub fn new(critic_name: impl Into<String>) -> Self {
        Self {
            critic_name: critic_name.into(),
            issues: Vec::new(),
            summary: String::new(),
            passed: true,
            confidence: 1.0,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn with_issue(mut self, issue: CritiqueIssue) -> Self {
        if issue.is_blocking() {
            self.passed = false;
        }
        self.issues.push(issue);
        self
    }

    pub fn with_issues(mut self, issues: Vec<CritiqueIssue>) -> Self {
        for issue in issues {
            if issue.is_blocking() {
                self.passed = false;
            }
            self.issues.push(issue);
        }
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn fail(mut self) -> Self {
        self.passed = false;
        self
    }

    /// Count issues by severity
    pub fn issue_counts(&self) -> (usize, usize, usize, usize) {
        let mut low = 0;
        let mut medium = 0;
        let mut high = 0;
        let mut critical = 0;

        for issue in &self.issues {
            match issue.severity {
                IssueSeverity::Low => low += 1,
                IssueSeverity::Medium => medium += 1,
                IssueSeverity::High => high += 1,
                IssueSeverity::Critical => critical += 1,
            }
        }

        (low, medium, high, critical)
    }

    /// Get blocking issues only
    pub fn blocking_issues(&self) -> Vec<&CritiqueIssue> {
        self.issues.iter().filter(|i| i.is_blocking()).collect()
    }

    /// Format for display
    pub fn format_display(&self) -> String {
        let mut output = format!(
            "## Critique: {} ({})\n\n",
            self.critic_name,
            if self.passed { "PASSED" } else { "NEEDS WORK" }
        );

        output.push_str(&format!("{}\n\n", self.summary));

        if !self.issues.is_empty() {
            let (low, medium, high, critical) = self.issue_counts();
            output.push_str(&format!(
                "Issues: {} critical, {} high, {} medium, {} low\n\n",
                critical, high, medium, low
            ));

            for issue in &self.issues {
                output.push_str(&format!("{}\n\n", issue.format_display()));
            }
        }

        output
    }

    /// Format for LLM context
    pub fn format_for_llm(&self) -> String {
        let status = if self.passed { "PASSED" } else { "NEEDS_WORK" };
        let mut context = format!(
            "CRITIQUE RESULT [{}]: {}\nSummary: {}\n",
            self.critic_name, status, self.summary
        );

        if !self.issues.is_empty() {
            context.push_str("\nIssues to address:\n");
            for (i, issue) in self.issues.iter().enumerate() {
                context.push_str(&format!(
                    "{}. [{}] {}\n",
                    i + 1,
                    issue.severity,
                    issue.description
                ));
                if let Some(suggestion) = &issue.suggestion {
                    context.push_str(&format!("   Fix: {}\n", suggestion));
                }
            }
        }

        context
    }
}

/// Context provided to critics for evaluation
#[derive(Debug, Clone)]
pub struct CritiqueContext {
    /// Description of the task that was performed
    pub task_description: String,
    /// Files that were modified
    pub modified_files: Vec<String>,
    /// Working directory
    pub working_dir: String,
    /// Test output if tests were run
    pub test_output: Option<String>,
    /// Build output if build was run
    pub build_output: Option<String>,
    /// Additional context
    pub additional_context: Option<String>,
}

impl CritiqueContext {
    pub fn new(task_description: impl Into<String>) -> Self {
        Self {
            task_description: task_description.into(),
            modified_files: Vec::new(),
            working_dir: ".".to_string(),
            test_output: None,
            build_output: None,
            additional_context: None,
        }
    }

    pub fn with_modified_files(mut self, files: Vec<String>) -> Self {
        self.modified_files = files;
        self
    }

    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = dir.into();
        self
    }

    pub fn with_test_output(mut self, output: impl Into<String>) -> Self {
        self.test_output = Some(output.into());
        self
    }

    pub fn with_build_output(mut self, output: impl Into<String>) -> Self {
        self.build_output = Some(output.into());
        self
    }

    pub fn with_additional_context(mut self, context: impl Into<String>) -> Self {
        self.additional_context = Some(context.into());
        self
    }
}

/// Trait for implementing critics that evaluate agent work
#[async_trait::async_trait]
pub trait Critic: Send + Sync {
    /// Evaluate the work and return critique results
    async fn critique(&self, context: &CritiqueContext) -> Result<CritiqueResult>;

    /// Get the critic's name
    fn name(&self) -> &str;

    /// Get a description of what this critic evaluates
    fn description(&self) -> &str;
}

/// Simple critic that checks for common code issues using pattern matching
pub struct PatternCritic {
    /// Patterns to check for (pattern, severity, category, description)
    patterns: Vec<(String, IssueSeverity, IssueCategory, String)>,
}

impl PatternCritic {
    pub fn new() -> Self {
        Self {
            patterns: Self::default_patterns(),
        }
    }

    fn default_patterns() -> Vec<(String, IssueSeverity, IssueCategory, String)> {
        vec![
            // Stub markers
            (
                "todo!()".to_string(),
                IssueSeverity::High,
                IssueCategory::Incomplete,
                "Contains todo!() macro".to_string(),
            ),
            (
                "unimplemented!()".to_string(),
                IssueSeverity::High,
                IssueCategory::Incomplete,
                "Contains unimplemented!() macro".to_string(),
            ),
            (
                "TODO:".to_string(),
                IssueSeverity::Medium,
                IssueCategory::Incomplete,
                "Contains TODO comment".to_string(),
            ),
            (
                "FIXME:".to_string(),
                IssueSeverity::Medium,
                IssueCategory::Bug,
                "Contains FIXME comment".to_string(),
            ),
            (
                "XXX:".to_string(),
                IssueSeverity::Medium,
                IssueCategory::Bug,
                "Contains XXX comment".to_string(),
            ),
            (
                "HACK:".to_string(),
                IssueSeverity::Low,
                IssueCategory::CodeQuality,
                "Contains HACK comment".to_string(),
            ),
            // Security concerns
            (
                "unwrap()".to_string(),
                IssueSeverity::Low,
                IssueCategory::Bug,
                "Uses unwrap() which can panic".to_string(),
            ),
            (
                "expect(\"".to_string(),
                IssueSeverity::Low,
                IssueCategory::Bug,
                "Uses expect() which can panic".to_string(),
            ),
            // Debug code
            (
                "println!(\"DEBUG".to_string(),
                IssueSeverity::Low,
                IssueCategory::CodeQuality,
                "Contains debug println".to_string(),
            ),
            (
                "dbg!(".to_string(),
                IssueSeverity::Low,
                IssueCategory::CodeQuality,
                "Contains dbg! macro".to_string(),
            ),
            // Potential issues
            (
                "panic!(".to_string(),
                IssueSeverity::Medium,
                IssueCategory::Bug,
                "Contains explicit panic".to_string(),
            ),
            (
                "unsafe {".to_string(),
                IssueSeverity::Medium,
                IssueCategory::Security,
                "Contains unsafe block".to_string(),
            ),
        ]
    }

    pub fn with_pattern(
        mut self,
        pattern: impl Into<String>,
        severity: IssueSeverity,
        category: IssueCategory,
        description: impl Into<String>,
    ) -> Self {
        self.patterns
            .push((pattern.into(), severity, category, description.into()));
        self
    }

    fn check_content(&self, content: &str, file_path: &str) -> Vec<CritiqueIssue> {
        let mut issues = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for (pattern, severity, category, description) in &self.patterns {
                if line.contains(pattern) {
                    issues.push(
                        CritiqueIssue::new(*severity, category.clone(), description.clone())
                            .with_file(file_path)
                            .with_line(line_num + 1),
                    );
                }
            }
        }

        issues
    }
}

impl Default for PatternCritic {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Critic for PatternCritic {
    async fn critique(&self, context: &CritiqueContext) -> Result<CritiqueResult> {
        let mut result = CritiqueResult::new("pattern_critic").with_confidence(0.8);

        let working_dir = Path::new(&context.working_dir);

        for file_path in &context.modified_files {
            let full_path = working_dir.join(file_path);
            if full_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&full_path) {
                    let file_issues = self.check_content(&content, file_path);
                    result = result.with_issues(file_issues);
                }
            }
        }

        let (low, medium, high, critical) = result.issue_counts();
        let summary = if result.issues.is_empty() {
            "No pattern-based issues found".to_string()
        } else {
            format!(
                "Found {} issues ({} critical, {} high, {} medium, {} low)",
                result.issues.len(),
                critical,
                high,
                medium,
                low
            )
        };

        Ok(result.with_summary(summary))
    }

    fn name(&self) -> &str {
        "pattern_critic"
    }

    fn description(&self) -> &str {
        "Checks for common code patterns that indicate incomplete or problematic code"
    }
}

/// Critic that checks build output for errors
pub struct BuildCritic;

impl BuildCritic {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BuildCritic {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Critic for BuildCritic {
    async fn critique(&self, context: &CritiqueContext) -> Result<CritiqueResult> {
        let mut result = CritiqueResult::new("build_critic").with_confidence(0.95);

        if let Some(build_output) = &context.build_output {
            // Check for common error indicators
            let has_errors = build_output.contains("error[E")
                || build_output.contains("error:")
                || build_output.contains("FAILED")
                || build_output.contains("cannot find");

            if has_errors {
                // Try to extract specific errors
                for line in build_output.lines() {
                    if line.contains("error[E") || line.starts_with("error:") {
                        result = result.with_issue(CritiqueIssue::new(
                            IssueSeverity::Critical,
                            IssueCategory::TypeError,
                            line.trim().to_string(),
                        ));
                    }
                }

                if result.issues.is_empty() {
                    // Generic error if we couldn't parse specific ones
                    result = result.with_issue(
                        CritiqueIssue::new(
                            IssueSeverity::Critical,
                            IssueCategory::TypeError,
                            "Build failed with errors",
                        )
                        .with_suggestion("Review build output and fix compilation errors"),
                    );
                }
            }

            // Check for warnings
            let warning_count = build_output.matches("warning:").count();
            if warning_count > 0 {
                result = result.with_issue(
                    CritiqueIssue::new(
                        IssueSeverity::Low,
                        IssueCategory::CodeQuality,
                        format!("Build produced {} warnings", warning_count),
                    )
                    .with_suggestion("Consider addressing compiler warnings"),
                );
            }

            let summary = if result.passed {
                "Build succeeded".to_string()
            } else {
                format!(
                    "Build failed with {} errors",
                    result.blocking_issues().len()
                )
            };

            result = result.with_summary(summary);
        } else {
            result = result
                .with_summary("No build output provided")
                .with_confidence(0.0);
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "build_critic"
    }

    fn description(&self) -> &str {
        "Checks build output for compilation errors and warnings"
    }
}

/// Critic that checks test output
pub struct TestCritic;

impl TestCritic {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TestCritic {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Critic for TestCritic {
    async fn critique(&self, context: &CritiqueContext) -> Result<CritiqueResult> {
        let mut result = CritiqueResult::new("test_critic").with_confidence(0.9);

        if let Some(test_output) = &context.test_output {
            // Check for test failures
            let has_failures = test_output.contains("FAILED")
                || test_output.contains("test result: FAILED")
                || test_output.contains("failures:")
                || test_output.contains("AssertionError");

            if has_failures {
                // Try to extract specific failures
                let mut found_specific = false;
                for line in test_output.lines() {
                    if line.contains("FAILED") || line.contains("panicked") {
                        result = result.with_issue(CritiqueIssue::new(
                            IssueSeverity::High,
                            IssueCategory::Bug,
                            line.trim().to_string(),
                        ));
                        found_specific = true;
                    }
                }

                if !found_specific {
                    result = result.with_issue(
                        CritiqueIssue::new(IssueSeverity::High, IssueCategory::Bug, "Tests failed")
                            .with_suggestion("Review test output and fix failing tests"),
                    );
                }
            }

            // Check for skipped tests
            if test_output.contains("ignored") || test_output.contains("skipped") {
                result = result.with_issue(CritiqueIssue::new(
                    IssueSeverity::Low,
                    IssueCategory::TestCoverage,
                    "Some tests were skipped or ignored",
                ));
            }

            let summary = if result.passed {
                "All tests passed".to_string()
            } else {
                format!(
                    "Tests failed with {} issues",
                    result.blocking_issues().len()
                )
            };

            result = result.with_summary(summary);
        } else {
            result = result
                .with_summary("No test output provided")
                .with_confidence(0.0);
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "test_critic"
    }

    fn description(&self) -> &str {
        "Checks test output for failures and issues"
    }
}

/// Manages multiple critics and aggregates their results
pub struct CriticManager {
    critics: Vec<Box<dyn Critic>>,
}

impl CriticManager {
    pub fn new() -> Self {
        Self {
            critics: Vec::new(),
        }
    }

    /// Create with default critics
    pub fn with_defaults() -> Self {
        Self {
            critics: vec![
                Box::new(PatternCritic::new()),
                Box::new(BuildCritic::new()),
                Box::new(TestCritic::new()),
            ],
        }
    }

    pub fn add_critic(&mut self, critic: Box<dyn Critic>) {
        self.critics.push(critic);
    }

    /// Run all critics and aggregate results
    pub async fn critique(&self, context: &CritiqueContext) -> Result<AggregatedCritique> {
        let mut results = Vec::new();

        for critic in &self.critics {
            match critic.critique(context).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!("Critic {} failed: {}", critic.name(), e);
                    // Create a failed result for this critic
                    results.push(
                        CritiqueResult::new(critic.name())
                            .with_summary(format!("Critic failed: {}", e))
                            .with_confidence(0.0),
                    );
                }
            }
        }

        Ok(AggregatedCritique::from_results(results))
    }
}

impl Default for CriticManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated results from multiple critics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedCritique {
    /// Individual critique results
    pub results: Vec<CritiqueResult>,
    /// Overall pass/fail status
    pub passed: bool,
    /// Total issue count
    pub total_issues: usize,
    /// Blocking issue count
    pub blocking_issues: usize,
    /// Overall confidence (weighted average)
    pub confidence: f32,
}

impl AggregatedCritique {
    pub fn from_results(results: Vec<CritiqueResult>) -> Self {
        let passed = results.iter().all(|r| r.passed);
        let total_issues: usize = results.iter().map(|r| r.issues.len()).sum();
        let blocking_issues: usize = results.iter().map(|r| r.blocking_issues().len()).sum();

        let confidence = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.confidence).sum::<f32>() / results.len() as f32
        };

        Self {
            results,
            passed,
            total_issues,
            blocking_issues,
            confidence,
        }
    }

    /// Get all blocking issues across all critics
    pub fn all_blocking_issues(&self) -> Vec<&CritiqueIssue> {
        self.results
            .iter()
            .flat_map(|r| r.blocking_issues())
            .collect()
    }

    /// Format for display
    pub fn format_display(&self) -> String {
        let mut output = format!(
            "# Critique Summary: {}\n\n",
            if self.passed { "PASSED" } else { "NEEDS WORK" }
        );

        output.push_str(&format!(
            "Total issues: {} ({} blocking)\n",
            self.total_issues, self.blocking_issues
        ));
        output.push_str(&format!("Confidence: {:.0}%\n\n", self.confidence * 100.0));

        for result in &self.results {
            output.push_str(&format!("---\n{}\n", result.format_display()));
        }

        output
    }

    /// Format for LLM context
    pub fn format_for_llm(&self) -> String {
        let status = if self.passed { "PASSED" } else { "NEEDS_WORK" };
        let mut context = format!(
            "SELF-CRITIQUE RESULT: {}\nTotal issues: {}, Blocking: {}\n\n",
            status, self.total_issues, self.blocking_issues
        );

        for result in &self.results {
            if !result.passed || !result.issues.is_empty() {
                context.push_str(&result.format_for_llm());
                context.push('\n');
            }
        }

        if self.passed {
            context.push_str("All critics passed. Work appears complete.\n");
        } else {
            context.push_str(
                "\nACTION REQUIRED: Address the blocking issues before marking task complete.\n",
            );
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critique_issue_creation() {
        let issue = CritiqueIssue::new(IssueSeverity::High, IssueCategory::Bug, "Test issue")
            .with_file("main.rs")
            .with_line(42)
            .with_suggestion("Fix the bug");

        assert!(issue.is_blocking());
        assert_eq!(issue.file, Some("main.rs".to_string()));
        assert_eq!(issue.line, Some(42));
    }

    #[test]
    fn test_critique_result_aggregation() {
        let result = CritiqueResult::new("test")
            .with_issue(CritiqueIssue::new(
                IssueSeverity::Low,
                IssueCategory::CodeQuality,
                "minor",
            ))
            .with_issue(CritiqueIssue::new(
                IssueSeverity::High,
                IssueCategory::Bug,
                "major",
            ));

        assert!(!result.passed);
        let (low, _, high, _) = result.issue_counts();
        assert_eq!(low, 1);
        assert_eq!(high, 1);
    }

    #[tokio::test]
    async fn test_pattern_critic() {
        let critic = PatternCritic::new();
        let context = CritiqueContext::new("Test task")
            .with_modified_files(vec!["nonexistent.rs".to_string()]);

        let result = critic.critique(&context).await.unwrap();
        assert!(result.passed); // No files to check
    }

    #[tokio::test]
    async fn test_build_critic_with_errors() {
        let critic = BuildCritic::new();
        let context = CritiqueContext::new("Test task")
            .with_build_output("error[E0425]: cannot find value `x` in this scope");

        let result = critic.critique(&context).await.unwrap();
        assert!(!result.passed);
        assert!(!result.blocking_issues().is_empty());
    }

    #[tokio::test]
    async fn test_test_critic_with_failures() {
        let critic = TestCritic::new();
        let context = CritiqueContext::new("Test task")
            .with_test_output("test result: FAILED. 1 passed; 1 failed");

        let result = critic.critique(&context).await.unwrap();
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn test_critic_manager() {
        let manager = CriticManager::with_defaults();
        let context = CritiqueContext::new("Test task")
            .with_build_output("Compiling... Finished")
            .with_test_output("test result: ok. 5 passed");

        let aggregated = manager.critique(&context).await.unwrap();
        assert!(aggregated.passed);
    }

    #[test]
    fn test_aggregated_critique() {
        let results = vec![
            CritiqueResult::new("critic1").with_summary("Good"),
            CritiqueResult::new("critic2")
                .with_issue(CritiqueIssue::new(
                    IssueSeverity::High,
                    IssueCategory::Bug,
                    "Issue",
                ))
                .with_summary("Has issues"),
        ];

        let aggregated = AggregatedCritique::from_results(results);
        assert!(!aggregated.passed);
        assert_eq!(aggregated.total_issues, 1);
        assert_eq!(aggregated.blocking_issues, 1);
    }
}
