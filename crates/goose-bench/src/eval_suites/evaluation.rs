use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

pub type Model = (String, String);
pub type Extension = String;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum EvalMetricValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl fmt::Display for EvalMetricValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalMetricValue::Integer(i) => write!(f, "{}", i),
            EvalMetricValue::Float(fl) => write!(f, "{:.2}", fl),
            EvalMetricValue::String(s) => write!(f, "{}", s),
            EvalMetricValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}
#[derive(Debug, Serialize)]
pub struct EvalMetric {
    pub name: String,
    pub value: EvalMetricValue,
}

#[derive(Debug, Default)]
pub struct ExtensionRequirements {
    pub builtin: Vec<String>,
    pub external: Vec<String>,
    pub remote: Vec<String>,
}

#[async_trait]
pub trait Evaluation: Send + Sync {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> Result<Vec<(String, EvalMetricValue)>>;

    fn name(&self) -> &str;

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: Vec::new(),
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}
