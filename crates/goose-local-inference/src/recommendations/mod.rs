//! Automatic quality discovery and metadata-only, revision-pinned memory forecasts.
#[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
mod forecast;
pub mod gguf;
pub mod quality;
mod transport;

use anyhow::Result;
use futures::{stream, StreamExt};
use goose_sdk_types::custom_requests::{
    LocalInferenceHfModelInfoDto, LocalInferenceHfModelVariantDto, LocalInferenceMemoryFit as Fit,
    LocalInferenceModelAssessmentDto as Assessment, LocalInferenceModelQualityDto as Quality,
};
use serde_json::Value;
use std::collections::HashSet;
use transport::Hub;

pub const INPUT_TOKENS: u64 = 4096;
pub const OUTPUT_TOKENS: u64 = 1024;
pub const RESERVE_BYTES: u64 = 1024 * 1024 * 1024;

pub fn unknown_assessment(available: u64) -> Assessment {
    Assessment {
        input_tokens: INPUT_TOKENS,
        output_tokens: OUTPUT_TOKENS,
        available_memory_bytes: available,
        reserve_bytes: RESERVE_BYTES,
        ..Default::default()
    }
}

pub fn supported() -> bool {
    cfg!(all(
        feature = "mlx",
        target_os = "macos",
        target_arch = "aarch64"
    ))
}

pub async fn discover(query: &str, limit: usize) -> Result<Vec<LocalInferenceHfModelInfoDto>> {
    let hub = Hub::new().await?;
    let query = query.trim();
    let mut candidates = Vec::new();
    if !query.is_empty() {
        let value = hub
            .json(
                "api/models",
                &[
                    ("search", query),
                    ("sort", "downloads"),
                    ("direction", "-1"),
                    ("limit", &limit.to_string()),
                ],
            )
            .await?;
        candidates.extend(value.as_array().cloned().unwrap_or_default());
        if query.split('/').count() == 2 {
            if let Ok(model) = hub.model(query).await {
                candidates.insert(0, model);
            }
        }
    } else {
        if !supported() {
            return Ok(Vec::new());
        }
        let index = quality::QualityIndex::fetch(&hub).await;
        let popular = stream::iter(["gguf", "mlx"].into_iter().map(|format| {
            let hub = &hub;
            async move {
                hub.json(
                    "api/models",
                    &[
                        ("filter", format),
                        ("sort", "trendingScore"),
                        ("direction", "-1"),
                        ("limit", "12"),
                    ],
                )
                .await
            }
        }))
        .buffered(2)
        .collect::<Vec<_>>()
        .await;
        for result in popular {
            if let Ok(Value::Array(models)) = result {
                candidates.extend(models);
            }
        }
        let derived = stream::iter(index.candidates.iter().map(|parent| {
            let hub = &hub;
            async move {
                let mut models = quality::derived_models(hub, parent)
                    .await
                    .unwrap_or_default();
                models.push(serde_json::json!({"id": parent}));
                models
            }
        }))
        .buffered(3)
        .collect::<Vec<_>>()
        .await;
        let popular = candidates;
        let mut benchmark_models = Vec::new();
        for position in 0..derived.iter().map(Vec::len).max().unwrap_or(0) {
            for family in &derived {
                if let Some(model) = family.get(position) {
                    benchmark_models.push(model.clone());
                }
            }
        }
        candidates = Vec::new();
        for index in 0..popular.len().max(benchmark_models.len()) {
            if let Some(model) = benchmark_models.get(index) {
                candidates.push(model.clone());
            }
            if let Some(model) = popular.get(index) {
                candidates.push(model.clone());
            }
        }
    }
    let mut seen = HashSet::new();
    candidates.retain(|model| {
        model["id"]
            .as_str()
            .is_some_and(|id| seen.insert(id.to_owned()))
    });
    let mut models: Vec<_> = candidates
        .into_iter()
        .filter_map(|model| {
            let repo = model["id"].as_str()?;
            let (author, name) = repo.split_once('/')?;
            Some(LocalInferenceHfModelInfoDto {
                repo_id: repo.into(),
                author: author.into(),
                model_name: name.into(),
                downloads: model["downloads"].as_u64().unwrap_or(0),
                ..Default::default()
            })
        })
        .collect();
    // Discovery is cheap. Tensor metadata is fetched later, progressively, for displayed repos.
    if !query.is_empty() {
        models.truncate(limit);
    } else {
        models.truncate(80);
    }
    Ok(models)
}

pub async fn repo_variants(
    repo: &str,
    available: u64,
    assess: bool,
    variant_id: Option<&str>,
) -> Result<(
    Vec<crate::hf_models::HfModelVariant>,
    Option<Quality>,
    Vec<Assessment>,
)> {
    let hub = Hub::new().await?;
    let manifest = hub.model(repo).await?;
    let variants = crate::hf_models::variants_from_manifest(repo, &manifest)?;
    let mut unknown = unknown_assessment(available);
    if !assess || variants.is_empty() {
        let assessments = vec![unknown; variants.len()];
        return Ok((variants, None, assessments));
    }
    if !supported() {
        unknown.reasons.push(
            "Memory forecasting requires a build with eredu support on Apple silicon.".into(),
        );
        let assessments = vec![unknown; variants.len()];
        return Ok((variants, None, assessments));
    }
    let index = quality::QualityIndex::fetch(&hub).await;
    let quality = index.for_model(repo, &manifest);
    let assessments = stream::iter(variants.iter().map(|variant| {
        let (hub, manifest) = (&hub, &manifest);
        async move {
            if variant_id.is_some_and(|id| id != variant.variant_id) {
                return unknown_assessment(available);
            }
            #[cfg(all(feature = "mlx", target_os = "macos", target_arch = "aarch64"))]
            let result = forecast::assess(hub, repo, variant, manifest, available).await;
            #[cfg(not(all(feature = "mlx", target_os = "macos", target_arch = "aarch64")))]
            let result: Result<Assessment> = {
                let _ = (hub, repo, variant, manifest);
                Err(anyhow::anyhow!("Eredu forecasting unavailable"))
            };
            result.unwrap_or_else(|error| Assessment {
                reasons: vec![format!("Memory estimate unavailable: {error}")],
                ..unknown_assessment(available)
            })
        }
    }))
    .buffered(2)
    .collect()
    .await;
    Ok((variants, quality, assessments))
}

pub fn recommended_variant(
    variants: &[LocalInferenceHfModelVariantDto],
    quality: Option<&Quality>,
) -> Option<String> {
    if !quality::has_recommendation_evidence(quality) {
        return None;
    }
    variants
        .iter()
        .filter(|variant| {
            variant.supported
                && variant
                    .assessment
                    .as_ref()
                    .is_some_and(|a| a.fit == Fit::LikelyFit)
        })
        .max_by_key(|variant| variant.quality_rank)
        .map(|variant| variant.variant_id.clone())
}
