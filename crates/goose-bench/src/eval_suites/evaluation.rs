use crate::work_dir::WorkDir;
use anyhow::Result;
use async_trait::async_trait;
use goose::message::Message;
use serde::Serialize;

pub type Model = (String, String);
pub type Extension = String;

#[derive(Debug, Serialize)]
pub enum EvaluationMetric {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

#[async_trait]
pub trait BenchAgent: Send + Sync {
    async fn prompt(&mut self, p: String) -> Result<Vec<Message>>;
}

#[async_trait]
pub trait Evaluation: Send + Sync {
    async fn run(
        &self,
        agent: Box<dyn BenchAgent>,
        run_loc: &mut WorkDir,
    ) -> Result<Vec<(String, EvaluationMetric)>>;
    fn name(&self) -> &str;
}