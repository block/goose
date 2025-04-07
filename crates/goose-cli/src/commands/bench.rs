use crate::logging;
use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use goose::config::Config;
use goose::message::Message;
use goose_bench::bench_work_dir::BenchmarkWorkDir;
use goose_bench::eval_suites::{BenchAgent, BenchAgentError, Evaluation, EvaluationSuite};
use goose_bench::reporting::{BenchmarkResults, EvaluationResult, SuiteResult};
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
        self.session.get_total_token_usage().ok().flatten()
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
        requirements.remote,
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

pub async fn run_benchmark(
    selectors: Vec<String>,
    include_dirs: Vec<PathBuf>,
) -> anyhow::Result<BenchmarkResults> {
    let config = Config::global();
    let goose_model: String = config
        .get_param("GOOSE_MODEL")
        .expect("No model configured. Run 'goose configure' first");
    let provider_name: String = config
        .get_param("GOOSE_PROVIDER")
        .expect("No provider configured. Run 'goose configure' first");

    let mut results = BenchmarkResults::new(provider_name.clone());

    let work_dir = Mutex::new(BenchmarkWorkDir::new(
        format!("{}-{}", provider_name, goose_model),
        include_dirs.clone(),
    ));

    for (suite, evals) in EvaluationSuite::select(selectors).iter() {
        let mut suite_result = SuiteResult::new(suite.clone());
        for eval_selector in evals {
            if let Some(eval) = EvaluationSuite::from(eval_selector) {
                let mut work_dir = work_dir.lock().await;
                work_dir.set_eval(eval_selector);
                let eval_result = run_eval(eval, &mut work_dir).await?;
                suite_result.add_evaluation(eval_result);
            }
        }

        results.add_suite(suite_result);
    }

    Ok(results)
}

pub async fn list_selectors() -> anyhow::Result<()> {
    let selector_eval_counts = EvaluationSuite::available_selectors();
    let mut keys: Vec<_> = selector_eval_counts.keys().collect();
    keys.sort();
    let max_key_len = keys.iter().map(|k| k.len()).max().unwrap_or(0);
    println!(
        "selector {} => Eval Count",
        " ".repeat(max_key_len - "selector".len())
    );
    println!("{}", "-".repeat(max_key_len + 6));
    for selector in keys {
        println!(
            "{} {} => {}",
            selector,
            " ".repeat(max_key_len - selector.len()),
            selector_eval_counts.get(selector).unwrap()
        );
    }
    Ok(())
}
