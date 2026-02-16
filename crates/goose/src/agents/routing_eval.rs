//! Routing evaluation framework for measuring IntentRouter accuracy.
//!
//! Provides YAML-based test sets, an evaluation runner, per-agent/per-mode
//! accuracy metrics, a confusion matrix, and a human-readable report.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::intent_router::IntentRouter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingEvalCase {
    pub input: String,
    pub expected_agent: String,
    pub expected_mode: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingEvalSet {
    pub test_cases: Vec<RoutingEvalCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingEvalResult {
    pub input: String,
    pub expected_agent: String,
    pub expected_mode: String,
    pub actual_agent: String,
    pub actual_mode: String,
    pub confidence: f32,
    pub reasoning: String,
    pub agent_correct: bool,
    pub mode_correct: bool,
    pub fully_correct: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingEvalMetrics {
    pub total: usize,
    pub correct: usize,
    pub agent_correct: usize,
    pub overall_accuracy: f64,
    pub agent_accuracy: f64,
    pub mode_accuracy_given_agent: f64,
    pub per_agent: HashMap<String, AgentMetrics>,
    pub per_mode: HashMap<String, ModeMetrics>,
    pub confusion_matrix: Vec<ConfusionEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentMetrics {
    pub total: usize,
    pub correct: usize,
    pub accuracy: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeMetrics {
    pub total: usize,
    pub correct: usize,
    pub accuracy: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfusionEntry {
    pub expected: String,
    pub actual: String,
    pub count: usize,
}

pub fn load_eval_set(yaml: &str) -> Result<RoutingEvalSet, serde_yaml::Error> {
    serde_yaml::from_str(yaml)
}

pub fn evaluate(router: &IntentRouter, test_set: &RoutingEvalSet) -> Vec<RoutingEvalResult> {
    test_set
        .test_cases
        .iter()
        .map(|tc| {
            let decision = router.route(&tc.input);
            let agent_correct =
                decision.agent_name.to_lowercase() == tc.expected_agent.to_lowercase();
            let mode_correct = decision.mode_slug == tc.expected_mode;
            RoutingEvalResult {
                input: tc.input.clone(),
                expected_agent: tc.expected_agent.clone(),
                expected_mode: tc.expected_mode.clone(),
                actual_agent: decision.agent_name.clone(),
                actual_mode: decision.mode_slug.clone(),
                confidence: decision.confidence,
                reasoning: decision.reasoning.clone(),
                agent_correct,
                mode_correct: agent_correct && mode_correct,
                fully_correct: agent_correct && mode_correct,
            }
        })
        .collect()
}

pub fn compute_metrics(results: &[RoutingEvalResult]) -> RoutingEvalMetrics {
    let total = results.len();
    let correct = results.iter().filter(|r| r.fully_correct).count();
    let agent_correct = results.iter().filter(|r| r.agent_correct).count();

    let mut per_agent: HashMap<String, (usize, usize)> = HashMap::new();
    for r in results {
        let entry = per_agent.entry(r.expected_agent.clone()).or_default();
        entry.0 += 1;
        if r.agent_correct {
            entry.1 += 1;
        }
    }

    let mut per_mode: HashMap<String, (usize, usize)> = HashMap::new();
    for r in results {
        let entry = per_mode.entry(r.expected_mode.clone()).or_default();
        entry.0 += 1;
        if r.fully_correct {
            entry.1 += 1;
        }
    }

    let mut confusion: HashMap<(String, String), usize> = HashMap::new();
    for r in results.iter().filter(|r| !r.agent_correct) {
        *confusion
            .entry((r.expected_agent.clone(), r.actual_agent.clone()))
            .or_default() += 1;
    }

    let agent_correct_count = results.iter().filter(|r| r.agent_correct).count();

    RoutingEvalMetrics {
        total,
        correct,
        agent_correct,
        overall_accuracy: if total > 0 {
            correct as f64 / total as f64
        } else {
            0.0
        },
        agent_accuracy: if total > 0 {
            agent_correct as f64 / total as f64
        } else {
            0.0
        },
        mode_accuracy_given_agent: if agent_correct_count > 0 {
            results
                .iter()
                .filter(|r| r.agent_correct && r.mode_correct)
                .count() as f64
                / agent_correct_count as f64
        } else {
            0.0
        },
        per_agent: per_agent
            .into_iter()
            .map(|(k, (t, c))| {
                (
                    k,
                    AgentMetrics {
                        total: t,
                        correct: c,
                        accuracy: if t > 0 { c as f64 / t as f64 } else { 0.0 },
                    },
                )
            })
            .collect(),
        per_mode: per_mode
            .into_iter()
            .map(|(k, (t, c))| {
                (
                    k,
                    ModeMetrics {
                        total: t,
                        correct: c,
                        accuracy: if t > 0 { c as f64 / t as f64 } else { 0.0 },
                    },
                )
            })
            .collect(),
        confusion_matrix: confusion
            .into_iter()
            .map(|((expected, actual), count)| ConfusionEntry {
                expected,
                actual,
                count,
            })
            .collect(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max).collect::<String>())
    }
}

pub fn format_report(metrics: &RoutingEvalMetrics, results: &[RoutingEvalResult]) -> String {
    let mut report = String::new();

    report.push_str("======================================================\n");
    report.push_str("        Routing Evaluation Report\n");
    report.push_str("======================================================\n\n");

    report.push_str(&format!(
        "Total: {} | Correct: {} ({:.1}%) | Agent: {:.1}% | Mode|Agent: {:.1}%\n\n",
        metrics.total,
        metrics.correct,
        metrics.overall_accuracy * 100.0,
        metrics.agent_accuracy * 100.0,
        metrics.mode_accuracy_given_agent * 100.0,
    ));

    report.push_str("Per-Agent Accuracy:\n");
    let mut agents: Vec<_> = metrics.per_agent.iter().collect();
    agents.sort_by_key(|(k, _)| (*k).clone());
    for (agent, m) in &agents {
        let bar_len = (m.accuracy * 20.0) as usize;
        let bar = format!("{}{}", "=".repeat(bar_len), ".".repeat(20 - bar_len));
        report.push_str(&format!(
            "  {:20} {:>2}/{:<2} ({:>5.1}%) {}\n",
            truncate(agent, 20),
            m.correct,
            m.total,
            m.accuracy * 100.0,
            bar
        ));
    }

    report.push_str("\nPer-Mode Accuracy:\n");
    let mut modes: Vec<_> = metrics.per_mode.iter().collect();
    modes.sort_by(|(_, a), (_, b)| {
        b.accuracy
            .partial_cmp(&a.accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (mode, m) in &modes {
        let bar_len = (m.accuracy * 20.0) as usize;
        let bar = format!("{}{}", "=".repeat(bar_len), ".".repeat(20 - bar_len));
        report.push_str(&format!(
            "  {:15} {:>2}/{:<2} ({:>5.1}%) {}\n",
            truncate(mode, 15),
            m.correct,
            m.total,
            m.accuracy * 100.0,
            bar
        ));
    }

    if !metrics.confusion_matrix.is_empty() {
        report.push_str("\nConfusion (misrouted):\n");
        for entry in &metrics.confusion_matrix {
            report.push_str(&format!(
                "  {} -> {}: {} case(s)\n",
                entry.expected, entry.actual, entry.count
            ));
        }
    }

    let failures: Vec<_> = results.iter().filter(|r| !r.fully_correct).collect();
    if !failures.is_empty() {
        report.push_str(&format!("\nFailed Cases ({}):\n", failures.len()));
        for r in &failures {
            report.push_str(&format!(
                "  X \"{}\" expected {}/{}, got {}/{} (conf={:.2})\n",
                truncate(&r.input, 50),
                r.expected_agent,
                r.expected_mode,
                r.actual_agent,
                r.actual_mode,
                r.confidence,
            ));
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_YAML: &str = r#"
test_cases:
  - input: "What time is it in Tokyo?"
    expected_agent: "Goose Agent"
    expected_mode: "assistant"
  - input: "Tell me a joke about programming"
    expected_agent: "Goose Agent"
    expected_mode: "assistant"
  - input: "Summarize this article for me"
    expected_agent: "Goose Agent"
    expected_mode: "assistant"
  - input: "What is the meaning of life?"
    expected_agent: "Goose Agent"
    expected_mode: "assistant"
  - input: "Help me write an email to my boss"
    expected_agent: "Goose Agent"
    expected_mode: "assistant"
  - input: "Write a REST API endpoint for user registration"
    expected_agent: "Coding Agent"
    expected_mode: "backend"
  - input: "Fix the database connection pool timeout issue"
    expected_agent: "Coding Agent"
    expected_mode: "backend"
  - input: "Implement a caching layer with Redis"
    expected_agent: "Coding Agent"
    expected_mode: "backend"
  - input: "Create a migration to add a users table"
    expected_agent: "Coding Agent"
    expected_mode: "backend"
  - input: "Debug the null pointer exception in the payment service"
    expected_agent: "Coding Agent"
    expected_mode: "backend"
  - input: "Build a responsive navigation bar with Tailwind CSS"
    expected_agent: "Coding Agent"
    expected_mode: "frontend"
  - input: "Fix the React component re-rendering issue"
    expected_agent: "Coding Agent"
    expected_mode: "frontend"
  - input: "Create a dark mode toggle for the dashboard"
    expected_agent: "Coding Agent"
    expected_mode: "frontend"
  - input: "Design the microservices architecture for our e-commerce platform"
    expected_agent: "Coding Agent"
    expected_mode: "architect"
  - input: "Create an architecture decision record for the new auth system"
    expected_agent: "Coding Agent"
    expected_mode: "architect"
  - input: "Review this code for SQL injection vulnerabilities"
    expected_agent: "Coding Agent"
    expected_mode: "security"
  - input: "Audit the authentication flow for security issues"
    expected_agent: "Coding Agent"
    expected_mode: "security"
  - input: "Check for hardcoded secrets in the repository"
    expected_agent: "Coding Agent"
    expected_mode: "security"
  - input: "Write unit tests for the UserService class"
    expected_agent: "Coding Agent"
    expected_mode: "qa"
  - input: "Create integration tests for the payment API"
    expected_agent: "Coding Agent"
    expected_mode: "qa"
  - input: "Set up end-to-end testing with Playwright"
    expected_agent: "Coding Agent"
    expected_mode: "qa"
  - input: "Create a product requirements document for the new feature"
    expected_agent: "Coding Agent"
    expected_mode: "pm"
  - input: "Write user stories for the shopping cart feature"
    expected_agent: "Coding Agent"
    expected_mode: "pm"
  - input: "Set up Kubernetes deployment manifests for the API"
    expected_agent: "Coding Agent"
    expected_mode: "sre"
  - input: "Configure Prometheus monitoring and alerting"
    expected_agent: "Coding Agent"
    expected_mode: "sre"
  - input: "Create a Dockerfile for the Node.js application"
    expected_agent: "Coding Agent"
    expected_mode: "sre"
  - input: "Set up CI/CD pipeline with GitHub Actions"
    expected_agent: "Coding Agent"
    expected_mode: "sre"
  - input: "Set up SAST scanning in the CI pipeline"
    expected_agent: "Coding Agent"
    expected_mode: "devsecops"
  - input: "Configure dependency vulnerability scanning"
    expected_agent: "Coding Agent"
    expected_mode: "devsecops"
  # QA Agent test cases
  - input: "Analyze the codebase for anti-patterns and code smells"
    expected_agent: "QA Agent"
    expected_mode: "analyze"
  - input: "Find complexity hotspots and maintainability issues"
    expected_agent: "QA Agent"
    expected_mode: "analyze"
  - input: "Design a test strategy for the payment processing module"
    expected_agent: "QA Agent"
    expected_mode: "test-design"
  - input: "Generate test cases for the user registration flow"
    expected_agent: "QA Agent"
    expected_mode: "test-design"
  - input: "Audit the test coverage and find gaps in our test suite"
    expected_agent: "QA Agent"
    expected_mode: "coverage-audit"
  - input: "What is the test coverage for the auth module?"
    expected_agent: "QA Agent"
    expected_mode: "coverage-audit"
  - input: "Review this pull request for correctness and reliability"
    expected_agent: "QA Agent"
    expected_mode: "review"
  - input: "Check this code for concurrency bugs and race conditions"
    expected_agent: "QA Agent"
    expected_mode: "review"
"#;

    #[test]
    fn test_load_eval_set() {
        let set = load_eval_set(TEST_YAML).expect("YAML should parse");
        assert_eq!(set.test_cases.len(), 37);
    }

    #[test]
    fn test_evaluate_produces_results() {
        let set = load_eval_set(TEST_YAML).unwrap();
        let router = IntentRouter::new();
        let results = evaluate(&router, &set);
        assert_eq!(results.len(), 37);
        for r in &results {
            assert!(!r.actual_agent.is_empty());
            assert!(!r.actual_mode.is_empty());
        }
    }

    #[test]
    fn test_general_prompts_route_to_goose() {
        let set = load_eval_set(TEST_YAML).unwrap();
        let router = IntentRouter::new();
        let results = evaluate(&router, &set);
        let goose: Vec<_> = results
            .iter()
            .filter(|r| r.expected_agent == "Goose Agent")
            .collect();
        let correct = goose.iter().filter(|r| r.agent_correct).count();
        let acc = correct as f64 / goose.len() as f64;
        assert!(
            acc >= 0.80,
            "General prompts should route to Goose Agent >= 80%, got {:.1}%",
            acc * 100.0
        );
    }

    #[test]
    fn test_coding_prompts_baseline() {
        let set = load_eval_set(TEST_YAML).unwrap();
        let router = IntentRouter::new();
        let results = evaluate(&router, &set);
        let coding: Vec<_> = results
            .iter()
            .filter(|r| r.expected_agent == "Coding Agent")
            .collect();
        let correct = coding.iter().filter(|r| r.agent_correct).count();
        let acc = correct as f64 / coding.len() as f64;
        // Keyword router baseline: ~33-48% agent-level accuracy.
        // This is a regression guard, not a quality target.
        assert!(
            acc >= 0.25,
            "Coding prompts should route to Coding Agent >= 25% (baseline), got {:.1}%",
            acc * 100.0
        );
    }

    #[test]
    fn test_compute_metrics() {
        let set = load_eval_set(TEST_YAML).unwrap();
        let router = IntentRouter::new();
        let results = evaluate(&router, &set);
        let metrics = compute_metrics(&results);
        assert_eq!(metrics.total, 37);
        assert!(metrics.overall_accuracy >= 0.0 && metrics.overall_accuracy <= 1.0);
        assert!(metrics.agent_accuracy >= 0.0 && metrics.agent_accuracy <= 1.0);
        assert!(!metrics.per_agent.is_empty());
        assert!(!metrics.per_mode.is_empty());
    }

    #[test]
    fn test_format_report() {
        let set = load_eval_set(TEST_YAML).unwrap();
        let router = IntentRouter::new();
        let results = evaluate(&router, &set);
        let metrics = compute_metrics(&results);
        let report = format_report(&metrics, &results);
        assert!(report.contains("Routing Evaluation Report"));
        assert!(report.contains("Per-Agent Accuracy"));
        assert!(report.contains("Per-Mode Accuracy"));
    }

    #[test]
    fn test_full_report_output() {
        let set = load_eval_set(TEST_YAML).unwrap();
        let router = IntentRouter::new();
        let results = evaluate(&router, &set);
        let metrics = compute_metrics(&results);
        let report = format_report(&metrics, &results);
        println!("\n{}", report);
    }
}
