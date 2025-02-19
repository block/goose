use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use goose::message::Message;
use goose_bench::eval_suites::{BenchAgent, EvaluationSuiteFactory};

#[async_trait]
impl BenchAgent for Session {
    async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
        self.headless_start(p).await?;
        Ok(self.message_history())
    }
}

pub async fn run_benchmark(suites: Vec<String>) {
    let suites = EvaluationSuiteFactory::available_evaluations()
        .into_iter()
        .filter(|&s| suites.contains(&s.to_string()))
        .collect::<Vec<_>>();

    for suite in suites {
        let evaluations = match EvaluationSuiteFactory::create(suite) {
            Some(evaluations) => evaluations,
            None => continue,
        };
        for evaluation in evaluations {
            let session = build_session(None, false, Vec::new(), Vec::new()).await;
            let _ = match evaluation.run(Box::new(session)).await {
                Ok(report) => report,
                _ => continue,
            };
        }
    }
}
