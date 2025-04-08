use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{EvalMetricValue, Evaluation, ExtensionRequirements};
use crate::register_evaluation;
use async_trait::async_trait;
// use std::fs;

pub struct ExampleEval {}

impl ExampleEval {
    pub fn new() -> Self {
        ExampleEval {}
    }
}

#[async_trait]
impl Evaluation for ExampleEval {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        _run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("ExampleEval - run");
        let mut metrics = Vec::new();

        let _ = agent.prompt("What can you do?".to_string()).await;

        metrics.push(("example_metric".to_string(), EvalMetricValue::Boolean(true)));

        metrics.push(("example_count".to_string(), EvalMetricValue::Integer(42)));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "example_eval"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements::default() // Example eval doesn't require any extensions
    }
}

register_evaluation!(ExampleEval);
