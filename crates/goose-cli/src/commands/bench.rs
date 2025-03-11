use crate::logging;
use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use goose::config::Config;
use goose::message::Message;
use goose_bench::bench_work_dir::BenchmarkWorkDir;
use goose_bench::eval_suites::{BenchAgent, BenchAgentError, Evaluation, EvaluationSuiteFactory};
use goose_bench::reporting::{BenchmarkResults, EvaluationResult, SuiteResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BenchSession {
    session: Session,
    errors: Arc<Mutex<Vec<BenchAgentError>>>,
}

impl BenchSession {
    pub fn new(session: Session) -> Self {
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Initialize logging with error capture
        logging::setup_logging(Some("bench"), Some(errors.clone()))
            .expect("Failed to initialize logging");

        Self { session, errors }
    }
}

#[async_trait]
impl BenchAgent for BenchSession {
    async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
        // Clear previous errors
        {
            let mut errors = self.errors.lock().await;
            errors.clear();
        }

        self.session.headless(p).await?;
        Ok(self.session.message_history())
    }

    async fn get_errors(&self) -> Vec<BenchAgentError> {
        let errors = self.errors.lock().await;
        errors.clone()
    }

    async fn get_token_usage(&self) -> Option<i32> {
        // Get token usage from the provider
        if let Ok(usage) = self.session.get_usage().await {
            // Sum up total tokens across all usage entries
            let total_tokens = usage
                .iter()
                .map(|u| u.usage.total_tokens.unwrap_or(0))
                .sum();
            Some(total_tokens)
        } else {
            None
        }
    }
}

// Wrapper struct to implement BenchAgent for Arc<Mutex<BenchSession>>
struct BenchAgentWrapper(Arc<Mutex<BenchSession>>);

#[async_trait]
impl BenchAgent for BenchAgentWrapper {
    async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
        let mut session = self.0.lock().await;
        session.prompt(p).await
    }

    async fn get_errors(&self) -> Vec<BenchAgentError> {
        let session = self.0.lock().await;
        session.get_errors().await
    }

    async fn get_token_usage(&self) -> Option<i32> {
        let session = self.0.lock().await;
        session.get_token_usage().await
    }
}

async fn run_eval(
    evaluation: Box<dyn Evaluation>,
    work_dir: &mut BenchmarkWorkDir,
) -> anyhow::Result<EvaluationResult> {
    let mut result = EvaluationResult::new(evaluation.name().to_string());

    let requirements = evaluation.required_extensions();

    // Create session with error capture
    let base_session = build_session(
        None,
        false,
        requirements.external,
        requirements.builtin,
        false,
    )
    .await;

    let bench_session = Arc::new(Mutex::new(BenchSession::new(base_session)));
    let bench_session_clone = bench_session.clone();

    if let Ok(metrics) = evaluation
        .run(Box::new(BenchAgentWrapper(bench_session)), work_dir)
        .await
    {
        for (name, metric) in metrics {
            result.add_metric(name, metric);
        }

        // Add any errors that occurred
        let agent = BenchAgentWrapper(bench_session_clone);
        for error in agent.get_errors().await {
            result.add_error(error);
        }
    }

    let current_dir = std::env::current_dir()?;
    let output_str = serde_json::to_string_pretty(&result)?;
    std::fs::write(current_dir.join("eval_result.json"), &output_str)?;

    Ok(result)
}

async fn run_suite(suite: &str, work_dir: &mut BenchmarkWorkDir) -> anyhow::Result<SuiteResult> {
    let mut suite_result = SuiteResult::new(suite.to_string());
    let eval_work_dir_guard = Mutex::new(work_dir);

    if let Some(evals) = EvaluationSuiteFactory::create(suite) {
        for eval in evals {
            let mut eval_work_dir = eval_work_dir_guard.lock().await;
            eval_work_dir.set_eval(eval.name());
            let eval_result = run_eval(eval, &mut eval_work_dir).await?;
            suite_result.add_evaluation(eval_result);
        }
    }

    Ok(suite_result)
}

pub async fn run_benchmark(
    suites: Vec<String>,
    include_dirs: Vec<PathBuf>,
) -> anyhow::Result<BenchmarkResults> {
    let suites = EvaluationSuiteFactory::available_evaluations()
        .into_iter()
        .filter(|&s| suites.contains(&s.to_string()))
        .collect::<Vec<_>>();

    let config = Config::global();
    let goose_model: String = config
        .get_param("GOOSE_MODEL")
        .expect("No model configured. Run 'goose configure' first");
    let provider_name: String = config
        .get_param("GOOSE_PROVIDER")
        .expect("No provider configured. Run 'goose configure' first");

    let mut results = BenchmarkResults::new(provider_name.clone());

    let suite_work_dir = Mutex::new(BenchmarkWorkDir::new(
        format!("{}-{}", provider_name, goose_model),
        include_dirs.clone(),
    ));

    for suite in suites {
        let mut work_dir = suite_work_dir.lock().await;
        work_dir.set_suite(suite);
        let suite_result = run_suite(suite, &mut work_dir).await?;
        results.add_suite(suite_result);
    }

    Ok(results)
}

pub async fn list_suites() -> anyhow::Result<HashMap<String, usize>> {
    let suites = EvaluationSuiteFactory::available_evaluations();
    let mut suite_counts = HashMap::new();

    for suite in suites {
        if let Some(evals) = EvaluationSuiteFactory::create(suite) {
            suite_counts.insert(suite.to_string(), evals.len());
        }
    }

    Ok(suite_counts)
}
