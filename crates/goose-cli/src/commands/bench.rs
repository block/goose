use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use chrono::Local;
use goose::config::Config;
use goose::message::Message;
use goose_bench::eval_suites::{BenchAgent, EvaluationMetric, EvaluationSuiteFactory};
use goose_bench::work_dir::WorkDir;

#[async_trait]
impl BenchAgent for Session {
    async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
        self.headless_start(p).await?;
        Ok(self.message_history())
    }
}

async fn run_eval(mut evaluation: Box<dyn BenchAgent>) -> anyhow::Result<Vec<EvaluationMetric>> {
    let _ = WorkDir::work_from(format!("./{}", &evaluation.name()));
    let session = build_session(None, false, Vec::new(), Vec::new()).await;
    let report = evaluation.run(Box::new(session))?.await;
    report
}

async fn run_suite(suite: &str, current_time: &String, current_date: &String) -> anyhow::Result<()> {
    let _ = WorkDir::work_from(format!("./{}", &suite))?;
    let _ = WorkDir::work_from(format!("./{}-{}", &current_date, current_time))?;
    for Some(evaluation) in EvaluationSuiteFactory::create(suite) {
        run_eval(evaluation)?.await;
    }
    Ok(())
}

pub async fn run_benchmark(suites: Vec<String>) {
    let suites = EvaluationSuiteFactory::available_evaluations()
        .into_iter()
        .filter(|&s| suites.contains(&s.to_string()))
        .collect::<Vec<_>>();

    let config = Config::global();
    let provider_name: String = config
        .get("GOOSE_PROVIDER")
        .expect("No provider configured. Run 'goose configure' first");


    let current_time = Local::now().format("%H:%M:%S").to_string();
    let current_date = Local::now().format("%Y-%m-%d").to_string();
    let _ = WorkDir::work_from(format!("./benchmark-{}", &provider_name))?;
    for suite in suites {
        run_suite(suite, &current_time, &current_date)?.await;
    }
}
