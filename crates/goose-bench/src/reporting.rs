use crate::bench_session::BenchAgentError;
use crate::eval_suites::EvalMetricValue;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a single evaluation result
#[derive(Default, Deserialize, Serialize)]
pub struct EvaluationResult {
    pub name: String,
    pub metrics: Vec<(String, EvalMetricValue)>,
    pub errors: Vec<BenchAgentError>,
}

/// Represents results for an entire suite
#[derive(Default, Deserialize, Serialize)]
pub struct SuiteResult {
    pub name: String,
    pub evaluations: Vec<EvaluationResult>,
}

/// Contains all benchmark results and metadata
#[derive(Default, Deserialize, Serialize)]
pub struct BenchmarkResults {
    pub provider: String,
    pub start_time: String,
    pub suites: Vec<SuiteResult>,
}

impl EvaluationResult {
    pub fn new(name: String) -> Self {
        Self {
            name,
            metrics: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_metric(&mut self, name: String, metric: EvalMetricValue) {
        self.metrics.push((name, metric));
    }

    pub fn add_error(&mut self, error: BenchAgentError) {
        self.errors.push(error);
    }
}

impl SuiteResult {
    pub fn new(name: String) -> Self {
        Self {
            name,
            evaluations: Vec::new(),
        }
    }

    pub fn add_evaluation(&mut self, eval: EvaluationResult) {
        self.evaluations.push(eval);
    }
}

impl BenchmarkResults {
    pub fn new(provider: String) -> Self {
        Self {
            provider,
            start_time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            suites: Vec::new(),
        }
    }

    pub fn add_suite(&mut self, suite: SuiteResult) {
        self.suites.push(suite);
    }

    /// Generate a summary of the benchmark results
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("Benchmark Summary - {}\n", self.provider));
        summary.push_str(&format!("Run at: {}\n\n", self.start_time));

        for suite in &self.suites {
            summary.push_str(&format!(
                "Suite: {} ({} evaluations)\n",
                suite.name,
                suite.evaluations.len()
            ));

            // Count total metrics and errors
            let total_metrics: usize = suite.evaluations.iter().map(|e| e.metrics.len()).sum();
            let total_errors: usize = suite.evaluations.iter().map(|e| e.errors.len()).sum();

            summary.push_str(&format!("  Total metrics: {}\n", total_metrics));
            if total_errors > 0 {
                summary.push_str(&format!("  Total errors: {}\n", total_errors));
            }
        }

        summary
    }
}

impl fmt::Display for BenchmarkResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Benchmark Results")?;
        writeln!(f, "Provider: {}", self.provider)?;
        writeln!(f, "Start Time: {}", self.start_time)?;
        writeln!(f)?;

        for suite in &self.suites {
            writeln!(f, "Suite: {}", suite.name)?;

            for eval in &suite.evaluations {
                writeln!(f, "  Evaluation: {}", eval.name)?;
                for (metric_name, metric_value) in &eval.metrics {
                    writeln!(f, "    {}: {}", metric_name, metric_value)?;
                }
                if !eval.errors.is_empty() {
                    writeln!(f, "    Errors:")?;
                    for error in &eval.errors {
                        writeln!(
                            f,
                            "      [{}] {}: {}",
                            error.timestamp.format("%H:%M:%S"),
                            error.level,
                            error.message
                        )?;
                    }
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
