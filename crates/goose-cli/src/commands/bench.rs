use goose::message::Message;
use crate::session::build_session;
use goose_bench::eval_suites::{EvaluationFactory, EvaluationReport};

// use std::error::Error;
// build custom run-func that constructs agent from session, then uses custom loop to manage collecting and returning agent messages.
async fn foo(ext) {
    let extension = Vec::new(); // todo
    let name = None;
    let mut session = build_session(name, false, extension, ext).await;
    let _ = session.headless_start(prompt).await;
}

pub async fn headless_start(&mut self, initial_message: String) -> anyhow::Result<()> {
    self.messages.push(Message::user().with_text(&initial_message));
    self.process_agent_response().await?;
    Ok(())
}

pub async fn run_benchmark() {
    let mut all_reports: Vec<EvaluationReport> = vec![];

    for eval in EvaluationFactory::available_evaluations() {
        let evaluation = match EvaluationFactory::create(&eval) {
            Some(evaluation) => evaluation,
            None => continue,
        };

        for (provider, model) in evaluation.models() {
            for ext in evaluation.extensions() {
                let report = match evaluation.run() {
                    Ok(report) => report,
                    _ => continue,
                };

                // print report?
                all_reports.push(report);
            }
        }
    }

    // let summary = report_summary(all_reports)?
    // print summary?
}
