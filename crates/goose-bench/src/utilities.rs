use crate::bench_session::{BenchAgent, BenchAgentError};
use async_trait::async_trait;
use goose::message::Message;
use std::collections::HashMap;

pub struct NoOpBenchAgent {}

#[async_trait]
impl BenchAgent for NoOpBenchAgent {
    async fn prompt(&mut self, _p: String) -> anyhow::Result<Vec<Message>> {
        Ok(Vec::new())
    }

    async fn get_errors(&self) -> Vec<BenchAgentError> {
        Vec::new()
    }

    async fn get_token_usage(&self) -> Option<i32> {
        Some(0)
    }
}

pub fn union_hashmaps<K, V>(maps: Vec<HashMap<K, V>>) -> HashMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: Clone,
{
    // We can use the fold method to accumulate all maps into one
    maps.into_iter().fold(HashMap::new(), |mut result, map| {
        // For each map in the vector, extend the result with its entries
        result.extend(map);
        result
    })
}
