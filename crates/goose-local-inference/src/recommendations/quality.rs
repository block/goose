use super::transport::Hub;
use anyhow::Result;
use futures::{stream, StreamExt};
use goose_sdk_types::custom_requests::{
    LocalInferenceBenchmarkEvidenceDto as Evidence, LocalInferenceModelQualityDto as Quality,
};
use serde_json::Value;
use std::collections::HashMap;

// Benchmark policy is shared across models; model IDs are always discovered.
const BENCHMARKS: &[&str] = &[
    "TIGER-Lab/MMLU-Pro",
    "LiquidAI/ifstruct-v1.0",
    "harborframework/terminal-bench-2.0",
];

#[derive(Default)]
pub(super) struct QualityIndex {
    pub models: HashMap<String, Quality>,
    pub candidates: Vec<String>,
}

impl QualityIndex {
    pub async fn fetch(hub: &Hub) -> Self {
        let results = stream::iter(BENCHMARKS.iter().map(|benchmark| async move {
            (
                *benchmark,
                hub.json(&format!("api/datasets/{benchmark}/leaderboard"), &[])
                    .await,
            )
        }))
        .buffered(3)
        .collect::<Vec<_>>()
        .await;
        let mut index = Self::default();
        let mut candidate_bands = HashMap::new();
        for (benchmark, result) in results {
            let Ok(Value::Array(rows)) = result else {
                continue;
            };
            let participants = rows
                .iter()
                .filter_map(|row| row["rank"].as_u64())
                .max()
                .unwrap_or(0);
            // Sample every size band so large frontier checkpoints cannot crowd
            // all laptop-sized candidates out of the discovery budget.
            let mut bands = [0; 5];
            for row in rows {
                let Some(model) = row["modelId"].as_str() else {
                    continue;
                };
                let (Some(score), Some(rank)) = (row["value"].as_f64(), row["rank"].as_u64())
                else {
                    continue;
                };
                if !score.is_finite() || rank == 0 {
                    continue;
                }
                let quality = index.models.entry(model.into()).or_insert_with(|| Quality {
                    evaluated_model_id: model.into(),
                    ..Default::default()
                });
                if quality.evidence.iter().any(|e| e.benchmark == *benchmark) {
                    continue;
                }
                quality.evidence.push(Evidence {
                    benchmark: (*benchmark).into(),
                    score,
                    rank,
                    participants,
                    verified: row["verified"].as_bool().unwrap_or(false),
                    source_url: row["source"]["url"]
                        .as_str()
                        .filter(|url| url.starts_with("https://"))
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("https://huggingface.co/datasets/{benchmark}")),
                    notes: row["notes"].as_str().unwrap_or_default().into(),
                });
                let band = match row["num_parameters"].as_u64() {
                    Some(0..=4_000_000_000) => 0,
                    Some(4_000_000_001..=16_000_000_000) => 1,
                    Some(16_000_000_001..=64_000_000_000) => 2,
                    Some(_) => 3,
                    None => 4,
                };
                if bands[band] < 2 && !index.candidates.iter().any(|id| id == model) {
                    index.candidates.push(model.into());
                    candidate_bands.insert(model.to_owned(), band);
                    bands[band] += 1;
                }
            }
        }
        index.candidates.sort_by_key(|model| candidate_bands[model]);
        index
    }

    pub fn for_model(&self, repo: &str, info: &Value) -> Option<Quality> {
        if let Some(quality) = self.models.get(repo) {
            return Some(quality.clone());
        }
        let parent = quantized_parent(info)?;
        let mut quality = self.models.get(parent)?.clone();
        quality.inherited = true;
        Some(quality)
    }
}

pub fn quantized_parent(info: &Value) -> Option<&str> {
    let card = &info["cardData"];
    let parent = card["base_model"].as_str().or_else(|| {
        let models = card["base_model"].as_array()?;
        (models.len() == 1).then(|| models[0].as_str()).flatten()
    })?;
    let explicit = card["base_model_relation"].as_str();
    let tagged = info["tags"].as_array().is_some_and(|tags| {
        tags.iter()
            .any(|tag| tag.as_str() == Some(format!("base_model:quantized:{parent}").as_str()))
    });
    (explicit == Some("quantized") || (explicit.is_none() && tagged)).then_some(parent)
}

pub(super) async fn derived_models(hub: &Hub, parent: &str) -> Result<Vec<Value>> {
    let filter = format!("base_model:quantized:{parent}");
    let value = hub
        .json(
            "api/models",
            &[
                ("filter", &filter),
                ("sort", "downloads"),
                ("direction", "-1"),
                ("limit", "4"),
            ],
        )
        .await?;
    Ok(value.as_array().cloned().unwrap_or_default())
}

/// Keep benchmark cohorts separate rather than averaging incomparable scores.
pub fn compare_quality(a: Option<&Quality>, b: Option<&Quality>) -> std::cmp::Ordering {
    fn key(quality: Option<&Quality>) -> (usize, u64) {
        BENCHMARKS
            .iter()
            .enumerate()
            .find_map(|(index, benchmark)| {
                quality?
                    .evidence
                    .iter()
                    .find(|e| e.benchmark == *benchmark)
                    .map(|e| (index, e.rank))
            })
            .unwrap_or((BENCHMARKS.len(), u64::MAX))
    }
    key(a).cmp(&key(b))
}

pub fn has_recommendation_evidence(quality: Option<&Quality>) -> bool {
    quality.is_some_and(|quality| !quality.evidence.is_empty())
}
