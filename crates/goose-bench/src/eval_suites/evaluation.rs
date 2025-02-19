use anyhow::Result;


pub type Model = (String, String);
pub type Extension = String;

#[derive(Debug)]
pub enum EvaluationMetric {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug)]
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

pub trait Evaluation: Send + Sync {
    fn run(&self) -> Result<EvaluationReport>;
    fn models(&self) -> Vec<Model>;
    fn extensions(&self) -> Vec<Extension>;
}
