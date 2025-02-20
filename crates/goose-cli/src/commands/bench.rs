use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use chrono::Local;
use goose::config::Config;
use goose::message::Message;
use goose_bench::eval_suites::{BenchAgent, EvaluationSuiteFactory};
use goose_bench::work_dir::WorkDir;

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

    let config = Config::global();
    let provider_name: String = config
        .get("GOOSE_PROVIDER")
        .expect("No provider configured. Run 'goose configure' first");


    let current_time = Local::now().format("%H:%M:%S").to_string();
    let current_date = Local::now().format("%Y-%m-%d").to_string();
    if let Ok(_) = WorkDir::work_from(format!("./benchmark-{}", &provider_name)) {
        for suite in suites {
            if let Ok(_) = WorkDir::work_from(format!("./{}", &suite)) {
                let evaluations = match EvaluationSuiteFactory::create(suite) {
                    Some(evaluations) => evaluations,
                    None => continue,
                };
                if let Ok(_) = WorkDir::work_from(format!("./{}-{}", &current_date, current_time)) {
                    for evaluation in evaluations {
                        if let Ok(_) = WorkDir::work_from(format!("./{}", &evaluation.name())) {
                            let session = build_session(None, false, Vec::new(), Vec::new()).await;
                            let _ = match evaluation.run(Box::new(session)).await {
                                Ok(report) => report,
                                _ => continue,
                            };
                        }
                    }
                }
            }
        }
    }
}
