use anyhow::Result;
use async_trait::async_trait;
use goose::message::Message;

pub type Model = (String, String);
pub type Extension = String;

#[derive(Debug)]
pub enum EvaluationMetric {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

pub struct EvaluationReport {
    metrics: Vec<EvaluationMetric>,
}

impl Default for EvaluationReport {
    fn default() -> Self {
        Self { metrics: vec![] }
    }
}

impl EvaluationReport {
    pub fn new(metrics: Vec<EvaluationMetric>) -> Self {
        EvaluationReport { metrics }
    }
}

#[async_trait]
pub trait BenchAgent: Send + Sync {
    async fn prompt(&mut self, p: String) -> Result<Vec<Message>>;
}

#[async_trait]
pub trait Evaluation: Send + Sync {
    async fn run(&self, agent: Box<dyn BenchAgent>) -> Result<EvaluationReport>;
}
