use crate::session::build_session;
use crate::Session;
use async_trait::async_trait;
use chrono::Local;
use goose::config::Config;
use goose::message::Message;
use goose_bench::eval_suites::{BenchAgent, Evaluation, EvaluationMetric, EvaluationSuiteFactory};
use goose_bench::work_dir::WorkDir;
use std::path::PathBuf;

#[async_trait]
impl BenchAgent for Session {
    async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
        println!("{}", p);
        self.headless_start(p).await?;
        Ok(self.message_history())
    }
}

#[allow(clippy::redundant_pattern_matching)]
async fn run_eval(
    evaluation: Box<dyn Evaluation>,
    work_dir: &mut WorkDir,
) -> anyhow::Result<Vec<EvaluationMetric>> {
    if let Ok(work_dir) = work_dir.move_to(format!("./{}", &evaluation.name())) {
        let session = build_session(None, false, Vec::new(), Vec::new()).await;
        evaluation.run(Box::new(session), work_dir).await
    } else {
        Ok(vec![])
    }
}

#[allow(clippy::redundant_pattern_matching)]
async fn run_suite(suite: &str, work_dir: &mut WorkDir) -> anyhow::Result<()> {
    if let Ok(work_dir) = work_dir.move_to(format!("./{}", &suite)) {
        if let Some(evals) = EvaluationSuiteFactory::create(suite) {
            for eval in evals {
                run_eval(eval, work_dir).await?;
            }
        }
    }

    Ok(())
}

#[allow(clippy::redundant_pattern_matching)]
pub async fn run_benchmark(suites: Vec<String>, include_dirs: Vec<PathBuf>) -> anyhow::Result<()> {
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
    if let Ok(mut work_dir) = WorkDir::at(
        format!("./benchmark-{}", &provider_name),
        include_dirs.clone(),
    ) {
        if let Ok(work_dir) = work_dir.move_to(format!("./{}-{}", &current_date, current_time)) {
            for suite in suites {
                run_suite(suite, work_dir).await?;
            }
        }
    }
    Ok(())
}
