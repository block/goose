use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct FlappyBird {}

impl FlappyBird {
    pub fn new() -> Self {
        FlappyBird {}
    }
}

#[async_trait]
impl Evaluation for FlappyBird {
    async fn run(&self, mut agent: Box<dyn BenchAgent>) -> anyhow::Result<Vec<EvaluationMetric>> {
        let metrics = Vec::new();
        let _ = agent.prompt("What can you do?".to_string()).await;
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "flappy_bird"
    }
}

register_evaluation!("core", FlappyBird);
