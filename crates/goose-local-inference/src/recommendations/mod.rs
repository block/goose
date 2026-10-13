//! Automatic quality discovery and metadata-only, revision-pinned memory forecasts.
pub mod catalogue;
#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
mod forecast;
pub mod gguf;
pub mod lineage;
pub mod quality;
pub mod suitability;
mod transport;

use anyhow::Result;
use futures::{stream, StreamExt};
use goose_sdk_types::custom_requests::{
    LocalInferenceCompatibility as Compatibility, LocalInferenceHfModelInfoDto,
    LocalInferenceHfModelVariantDto, LocalInferenceMemoryFit as Fit,
    LocalInferenceModelAssessmentDto as Assessment, LocalInferenceModelQualityDto as Quality,
    LocalInferenceSuitability as Suitability,
};
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
        feature = "eredu",
        target_os = "macos",
        target_arch = "aarch64"
    ))
}

pub async fn discover(query: &str, limit: usize) -> Result<Vec<LocalInferenceHfModelInfoDto>> {
    let query = query.trim();
    if query.is_empty() {
        if !supported() {
            return Ok(Vec::new());
        }
        return Ok(catalogue::bundled()?
            .models
            .iter()
            .map(|model| {
                let (author, name) = model.repo_id.split_once('/').unwrap();
                LocalInferenceHfModelInfoDto {
                    repo_id: model.repo_id.clone(),
                    author: author.into(),
                    model_name: name.into(),
                    downloads: model.downloads,
                    quality: Some(model.quality.clone()),
                    ..Default::default()
                }
            })
            .collect());
    }
    let hub = Hub::new().await?;
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
    let mut candidates = value.as_array().cloned().unwrap_or_default();
    if query.split('/').count() == 2 {
        if let Ok(model) = hub.model(query).await {
            candidates.insert(0, model);
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
    models.truncate(limit);
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
    if let Ok(Some(model)) = catalogue::find(repo) {
        let variants = model
            .variants
            .iter()
            .map(catalogue::available_variant)
            .collect();
        let assessments = model
            .variants
            .iter()
            .map(|item| catalogue::assess(model, item, available))
            .collect::<Result<Vec<_>>>()?;
        return Ok((variants, Some(model.quality.clone()), assessments));
    }
    let hub = Hub::new().await?;
    let manifest = hub.model(repo).await?;
    let variants = crate::hf_models::variants_from_manifest(repo, &manifest)?;
    let mut unknown = unknown_assessment(available);
    let (purpose, reason) = suitability::assess(&hub, &manifest, None).await;
    unknown.suitability = purpose;
    unknown.reasons.push(reason);
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
    let quality = None;
    let (purpose, reason) = suitability::assess(&hub, &manifest, quality.as_ref()).await;
    unknown.suitability = purpose;
    unknown.reasons = vec![reason];
    let requests = variants
        .iter()
        .map(|variant| {
            let (hub, manifest, unknown) = (&hub, &manifest, &unknown);
            async move {
                if variant_id.is_some_and(|id| id != variant.variant_id)
                    || purpose == Suitability::Unsuitable
                {
                    return unknown.clone();
                }
                #[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
                let result = forecast::assess(hub, repo, variant, manifest, available).await;
                #[cfg(not(all(feature = "eredu", target_os = "macos", target_arch = "aarch64")))]
                let result: Result<Assessment> = {
                    let _ = (hub, repo, variant, manifest);
                    Err(anyhow::anyhow!("Eredu forecasting unavailable"))
                };
                let mut assessment = result.unwrap_or_else(|error| Assessment {
                    reasons: vec![format!("Assessment unavailable: {error}")],
                    ..unknown_assessment(available)
                });
                if purpose != Suitability::Unknown {
                    assessment.suitability = purpose;
                }
                assessment.reasons.extend(unknown.reasons.iter().cloned());
                assessment
            }
        })
        .collect::<Vec<_>>();
    let assessments = stream::iter(requests).buffered(2).collect().await;
    Ok((variants, quality, assessments))
}

pub fn recommendation_memory_eligible(assessment: &Assessment) -> bool {
    assessment.fit == Fit::LikelyFit
        || (assessment.fit == Fit::Unknown
            && assessment.upper_bytes.is_none()
            && assessment.available_memory_bytes > 0
            && assessment.lower_bytes.is_some_and(|lower| {
                assessment
                    .available_memory_bytes
                    .checked_sub(assessment.reserve_bytes)
                    .is_some_and(|budget| lower <= budget)
            }))
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
                && variant.assessment.as_ref().is_some_and(|a| {
                    recommendation_memory_eligible(a)
                        && a.compatibility == Compatibility::Compatible
                        && a.suitability == Suitability::Suitable
                })
        })
        .max_by_key(|variant| {
            (
                variant
                    .assessment
                    .as_ref()
                    .is_some_and(|a| a.fit == Fit::LikelyFit),
                variant.quality_rank,
            )
        })
        .map(|variant| variant.variant_id.clone())
}
