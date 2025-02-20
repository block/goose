use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use chrono::Local;
use goose::config::Config;
use goose::message::Message;
use goose_bench::eval_suites::{BenchAgent, Evaluation, EvaluationMetric, EvaluationSuiteFactory};
use goose_bench::work_dir::WorkDir;

#[async_trait]
impl BenchAgent for Session {
    async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
        println!("{}", p);
        self.headless_start(p).await?;
        Ok(self.message_history())
    }
}

async fn run_eval(evaluation: Box<dyn Evaluation>) -> anyhow::Result<Vec<EvaluationMetric>> {
    if let Ok(_) = WorkDir::work_from(format!("./{}", &evaluation.name())) {
        let session = build_session(None, false, Vec::new(), Vec::new()).await;
        let report = evaluation.run(Box::new(session)).await;
        report
    } else {
        Ok(vec![])
    }
}

async fn run_suite(suite: &str) -> anyhow::Result<()> {
    if let Ok(_) = WorkDir::work_from(format!("./{}", &suite)) {
        if let Some(evals) = EvaluationSuiteFactory::create(suite) {
            for eval in evals {
                run_eval(eval).await?;
            }
        }
    }

    Ok(())
}

pub async fn run_benchmark(suites: Vec<String>) -> anyhow::Result<()> {
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
    if let Ok(_) = WorkDir::work_from(format!("./benchmark-{}", &provider_name)) {
        if let Ok(_) = WorkDir::work_from(format!("./{}-{}", &current_date, current_time)) {
            for suite in suites {
                run_suite(suite).await?;
            }
        }
    }
    Ok(())
}
