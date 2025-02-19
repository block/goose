use crate::eval_suites::evaluation::EvaluationReport;
use crate::eval_suites::{BenchAgent, Evaluation};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct FlappyBird {}

impl FlappyBird {
    pub fn new() -> Self {FlappyBird {}}
}

#[async_trait]
impl Evaluation for FlappyBird {
    async fn run(&self, mut agent: Box<dyn BenchAgent>) -> anyhow::Result<EvaluationReport> {
        let metrics = Vec::new();
        let _ = agent.prompt("What can you do?".to_string()).await;
        Ok(EvaluationReport::new(metrics))
    }
}

register_evaluation!("core", FlappyBird);