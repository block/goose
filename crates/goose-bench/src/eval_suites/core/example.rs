use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct ExampleEval {}

impl ExampleEval {
    pub fn new() -> Self {
        ExampleEval {}
    }
}

#[async_trait]
impl Evaluation for ExampleEval {
    async fn run(&self, mut agent: Box<dyn BenchAgent>) -> anyhow::Result<Vec<EvaluationMetric>> {
        println!("ExampleEval - run");
        let metrics = Vec::new();
        let _ = agent.prompt("What can you do?".to_string()).await;
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "flappy_bird"
    }
}

register_evaluation!("core", ExampleEval);
