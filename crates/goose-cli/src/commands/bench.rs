use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use goose::message::Message;
use goose_bench::eval_suites::{BenchAgent, EvaluationReport, EvaluationSuiteFactory};
// use std::error::Error;

// cli flag for suite_name [done]
// default suite_name called core [done]
// pass session messages in to run [done]
// eval suite = suite_name / eval_name / test_file_name [done]
// use session config expecting external proc to manage swapping out config


#[async_trait]
impl BenchAgent for Session {
    async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
        self.headless_start(p).await?;
        Ok(self.message_history())
    }
}

pub async fn run_benchmark(suites: Vec<String>) {
    let mut all_reports: Vec<EvaluationReport> = vec![];

    let suites = EvaluationSuiteFactory::available_evaluations()
        .into_iter()
        .filter(|&s| suites.contains(&s.to_string()))
        .collect::<Vec<_>>();

    for suite in suites {
        let evaluations = match EvaluationSuiteFactory::create(&suite) {
            Some(evaluations) => evaluations,
            None => continue,
        };
        for evaluation in evaluations {
            let session = build_session(None, false, Vec::new(), Vec::new()).await;
            let report = match evaluation.run(Box::new(session)).await {
                Ok(report) => report,
                _ => continue,
            };

            // print report?
            all_reports.push(report);
        }
    }

    // let summary = report_summary(all_reports)?
    // print summary?
}