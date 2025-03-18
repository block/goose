use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric, ExtensionRequirements};
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
        mut agent: Box<dyn BenchAgent>,
        _work_dir: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        println!("ExampleEval - run");
        let mut metrics = Vec::new();

        let _ = agent.prompt("What can you do?".to_string()).await;

        metrics.push((
            "example_metric".to_string(),
            EvaluationMetric::Boolean(true),
        ));

        metrics.push(("example_count".to_string(), EvaluationMetric::Integer(42)));

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
