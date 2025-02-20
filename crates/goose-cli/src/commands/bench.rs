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
fn cwd_todo_remove() {
    match std::env::current_dir() {
        Ok(path) => println!("Current directory is: {:?}", path),
        Err(e) => eprintln!("Failed to get current directory: {}", e)
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


    let _ = cwd_todo_remove();
    let current_time = Local::now().format("%H:%M:%S").to_string();
    let current_date = Local::now().format("%Y-%m-%d").to_string();
    if let _ = WorkDir::work_from(format!("./benchmark-{}", &provider_name)) {
        let _ = cwd_todo_remove();
        for suite in suites {
            if let _ = WorkDir::work_from(format!("./{}", &suite)) {
                let _ = cwd_todo_remove();
                let evaluations = match EvaluationSuiteFactory::create(suite) {
                    Some(evaluations) => evaluations,
                    None => continue,
                };
                if let _ = WorkDir::work_from(format!("./{}-{}", &current_date, current_time)) {
                    let _ = cwd_todo_remove();
                    for evaluation in evaluations {
                        if let _ = WorkDir::work_from(format!("./{}", &evaluation.name())) {
                            let _ = cwd_todo_remove();
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
