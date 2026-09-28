use super::{transport::Hub, Assessment, Fit, INPUT_TOKENS, OUTPUT_TOKENS, RESERVE_BYTES};
use crate::hf_models::HfModelVariant;
use anyhow::{ensure, Result};
use eredu::api::*;
use eredu_core::artifact::GgufCompanionRole;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, OnceLock},
};
use tokio::sync::{Mutex, Semaphore};

static FORECASTS: OnceLock<Mutex<HashMap<String, Arc<GenerationForecast>>>> = OnceLock::new();
static INSPECTIONS: Semaphore = Semaphore::const_new(2);
const HEADER_LIMIT: usize = 16 * 1024 * 1024;

pub(super) async fn assess(
    hub: &Hub,
    repo: &str,
    variant: &HfModelVariant,
    manifest: &Value,
    available: u64,
) -> Result<Assessment> {
    let revision = manifest["sha"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing immutable model revision"))?;
    ensure!(
        revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid model revision"
    );
    let key = format!(
        "{repo}@{revision}:{}:{}",
        variant.format,
        variant.filename.as_deref().unwrap_or("default")
    );
    let cache = FORECASTS.get_or_init(Default::default);
    let _permit = INSPECTIONS.acquire().await?;
    let cached = cache.lock().await.get(&key).cloned();
    let forecast = match cached {
        Some(forecast) => forecast,
        None => {
            let metadata = fetch_metadata(hub, repo, revision, variant, manifest).await?;
            let result = tokio::task::spawn_blocking(move || -> Result<_> {
                let inspection = inspect_model_metadata(
                    &metadata,
                    &MetadataInspectionOptions::new(BackendId::new("mlx")?),
                )?;
                if !inspection.report().is_compatible() {
                    return Ok(Err(inspection
                        .report()
                        .issues
                        .iter()
                        .map(|issue| issue.detail.clone())
                        .collect::<Vec<_>>()));
                }
                let calibration = ForecastCalibration::default();
                let mut options = GenerationMemoryOptions::new(
                    InputTokenCount::text(INPUT_TOKENS),
                    GenerationMemoryPlacement::Unified,
                );
                options.max_output_tokens = Some(OUTPUT_TOKENS);
                // Match eredu's managed allocator-cache default without opening a device.
                options.backend_overhead = calibration.allocator_overhead(256 * 1024 * 1024, 0);
                Ok(Ok(forecast_inspected_generation(
                    &inspection,
                    &options,
                    &calibration,
                )?))
            })
            .await??;
            let forecast = match result {
                Ok(forecast) => Arc::new(forecast),
                Err(reasons) => {
                    return Ok(Assessment {
                        fit: Fit::Incompatible,
                        revision: Some(revision.into()),
                        reasons,
                        ..super::unknown_assessment(available)
                    })
                }
            };
            let mut cache = cache.lock().await;
            if cache.len() >= 128 {
                cache.clear();
            }
            cache.insert(key, forecast.clone());
            forecast
        }
    };
    // The immutable forecast is reused; point-in-time capacity is never cached.
    let mut request = forecast.request.clone();
    for domain in &mut request.domains {
        domain.budget.available_bytes = (available > 0).then_some(available);
        domain.budget.reserve_bytes = RESERVE_BYTES;
    }
    let estimate = estimate_generation_memory(&request)?;
    let peak = estimate.domains.first().map(|domain| &domain.overall_peak);
    let mut reasons = estimate.uncertainties.clone();
    reasons.extend(forecast.estimate.assumptions.clone());
    reasons.push("Text-only, fully resident, one request, no draft model; eredu forecast calibration and 256 MiB allocator-cache allowance.".into());
    if variant.backend_id == "llamacpp" {
        reasons.push("An eredu fit is used as a llama.cpp fit estimate. An eredu shortfall does not prove llama.cpp cannot run this model.".into());
    }
    Ok(Assessment {
        fit: match estimate.fit {
            MemoryFit::LikelyFit => Fit::LikelyFit,
            MemoryFit::LikelyShortfall => Fit::LikelyShortfall,
            MemoryFit::InsufficientInformation => Fit::Unknown,
        },
        lower_bytes: peak.map(|peak| peak.lower_bytes),
        upper_bytes: peak.and_then(|peak| peak.upper_bytes),
        revision: Some(revision.into()),
        reasons,
        ..super::unknown_assessment(available)
    })
}

async fn fetch_metadata(
    hub: &Hub,
    repo: &str,
    revision: &str,
    variant: &HfModelVariant,
    manifest: &Value,
) -> Result<ArtifactMetadata> {
    let names: BTreeSet<&str> = manifest["siblings"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing model file listing"))?
        .iter()
        .filter_map(|file| file["rfilename"].as_str())
        .collect();
    let mut sidecars = BTreeMap::new();
    let mut total_header_bytes = 0;
    let checkpoint = if variant.format == "safetensors" {
        let config = hub.file(repo, revision, "config.json").await?;
        let index = if names.contains("model.safetensors.index.json") {
            Some(
                hub.file(repo, revision, "model.safetensors.index.json")
                    .await?,
            )
        } else {
            None
        };
        let members: BTreeSet<String> = match &index {
            Some(index) => serde_json::from_slice::<Value>(index)?["weight_map"]
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Invalid SafeTensors index"))?
                .values()
                .map(|name| {
                    name.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| anyhow::anyhow!("Invalid shard name"))
                })
                .collect::<Result<_>>()?,
            None => names
                .iter()
                .filter(|name| name.ends_with(".safetensors") && !name.contains('/'))
                .map(|name| (*name).into())
                .collect(),
        };
        ensure!(
            !members.is_empty() && members.len() <= 256,
            "Incomplete or oversized SafeTensors shard set"
        );
        let mut headers = Vec::new();
        for member in members {
            ensure!(
                names.contains(member.as_str()),
                "Missing indexed SafeTensors shard"
            );
            let (prefix, file_len) = hub.range(repo, revision, &member, 0, 7).await?;
            let length = u64::from_le_bytes(
                prefix
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid SafeTensors prefix"))?,
            );
            ensure!(
                length > 0 && length <= HEADER_LIMIT as u64 - 8 && length + 8 <= file_len,
                "Invalid or oversized SafeTensors header"
            );
            let (bytes, checked_len) = hub.range(repo, revision, &member, 0, length + 7).await?;
            ensure!(
                file_len == checked_len,
                "Model length changed during inspection"
            );
            total_header_bytes += bytes.len();
            ensure!(
                total_header_bytes <= 64 * 1024 * 1024,
                "Model headers exceed inspection budget"
            );
            headers.push(SafetensorsHeader {
                member,
                bytes,
                file_len,
            });
        }
        for name in [
            "preprocessor_config.json",
            "video_preprocessor_config.json",
            "processor_config.json",
        ] {
            if names.contains(name) {
                sidecars.insert(name.into(), hub.file(repo, revision, name).await?);
            }
        }
        CheckpointMetadata::SafeTensors {
            config,
            index,
            headers,
        }
    } else {
        let (members, projector) =
            crate::hf_models::forecast_gguf_members(repo, variant, manifest)?;
        ensure!(
            !members.is_empty() && members.len() <= 256,
            "Incomplete or oversized GGUF shard set"
        );
        let mut headers = Vec::new();
        for member in members {
            let header = gguf_header(hub, repo, revision, &member).await?;
            total_header_bytes += header.bytes.len();
            ensure!(
                total_header_bytes <= 64 * 1024 * 1024,
                "Model headers exceed inspection budget"
            );
            headers.push(header);
        }
        let companions = match projector {
            Some(member) => vec![GgufCompanionMetadata {
                role: GgufCompanionRole::MediaProjector,
                headers: vec![gguf_header(hub, repo, revision, &member).await?],
            }],
            None => Vec::new(),
        };
        CheckpointMetadata::Gguf {
            headers,
            companions,
        }
    };
    Ok(ArtifactMetadata {
        provenance: MetadataProvenance {
            source: format!("{repo}:{}", variant.variant_id),
            revision: revision.into(),
        },
        checkpoint,
        sidecars,
    })
}

async fn gguf_header(hub: &Hub, repo: &str, revision: &str, member: &str) -> Result<GgufHeader> {
    let (mut bytes, file_len) = hub.range(repo, revision, member, 0, 65535).await?;
    loop {
        match super::gguf::header_end(&bytes)? {
            Ok(end) => {
                bytes.truncate(end);
                return Ok(GgufHeader {
                    member: member.into(),
                    bytes,
                    file_len,
                });
            }
            Err(required) => {
                let end = required.max(bytes.len() * 2).min(HEADER_LIMIT) as u64;
                ensure!(
                    end > bytes.len() as u64 && end <= file_len,
                    "Incomplete or oversized GGUF header"
                );
                let (next, checked_len) = hub
                    .range(repo, revision, member, bytes.len() as u64, end - 1)
                    .await?;
                ensure!(
                    file_len == checked_len,
                    "Model length changed during inspection"
                );
                bytes.extend_from_slice(&next);
            }
        }
    }
}
