use super::{
    Assessment, Compatibility, Fit, Quality, Suitability, INPUT_TOKENS, OUTPUT_TOKENS,
    RESERVE_BYTES,
};
use crate::hf_models::HfModelVariant;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
mod build;
#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
pub use build::{explain, rebuild, revalidate_lineage};

#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
mod worker;
#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
pub use worker::run_worker;

pub const SCHEMA_VERSION: u32 = 1;
pub const POLICY_VERSION: u32 = 6;
pub const ALLOCATOR_BYTES: u64 = 256 * 1024 * 1024;
pub const DEFAULT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/model-catalogue.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub allocator_bytes: u64,
    pub reserve_bytes: u64,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            input_tokens: INPUT_TOKENS,
            output_tokens: OUTPUT_TOKENS,
            allocator_bytes: ALLOCATOR_BYTES,
            reserve_bytes: RESERVE_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    pub schema_version: u32,
    pub policy_version: u32,
    pub eredu_revision: String,
    pub generated_at: String,
    pub profile: Profile,
    pub benchmarks: Vec<String>,
    pub quality_index_version: String,
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub repo_id: String,
    pub revision: String,
    pub downloads: u64,
    #[serde(serialize_with = "serialize_quality")]
    pub quality: Quality,
    pub variants: Vec<Variant>,
}

fn serialize_quality<S: serde::Serializer>(
    quality: &Quality,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    let mut value = serde_json::to_value(quality).map_err(serde::ser::Error::custom)?;
    value.as_object_mut().unwrap().remove("catalogueOrder");
    value.serialize(serializer)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<super::lineage::Verification>,
    pub variant: HfModelVariant,
    pub files: Vec<File>,
    pub projector: Option<String>,
    pub suitability_evidence: Vec<String>,
    pub forecast_assumptions: Vec<String>,
    pub memory: MemoryForecast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryForecast {
    pub lower_bytes: u64,
    pub upper_bytes: Option<u64>,
    pub uncertainties: Vec<String>,
}

/// The catalogue profile is cold, fully resident and unified: no resident bytes
/// are subtracted, and the local reserve is added to both eredu bounds.
pub fn memory_fit(memory: &MemoryForecast, available: u64) -> Fit {
    if available == 0 {
        Fit::Unknown
    } else if u128::from(memory.lower_bytes) + u128::from(RESERVE_BYTES) > u128::from(available) {
        Fit::LikelyShortfall
    } else if memory
        .upper_bytes
        .is_some_and(|upper| u128::from(upper) + u128::from(RESERVE_BYTES) <= u128::from(available))
    {
        Fit::LikelyFit
    } else {
        Fit::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub name: String,
    pub size_bytes: u64,
}

pub fn eredu_revision() -> &'static str {
    include_str!("../../Cargo.toml")
        .lines()
        .find(|line| line.starts_with("eredu = "))
        .and_then(|line| line.split("rev = \"").nth(1))
        .and_then(|line| line.split('"').next())
        .expect("eredu dependency must have a pinned revision")
}

pub fn valid_revision(revision: &str) -> bool {
    revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn validate(catalogue: &Catalogue) -> Result<()> {
    validate_inner(catalogue, true)
}

fn validate_inner(catalogue: &Catalogue, require_lineage: bool) -> Result<()> {
    ensure!(
        catalogue.schema_version == SCHEMA_VERSION && catalogue.policy_version == POLICY_VERSION,
        "Local model catalogue schema/policy is stale; rebuild it"
    );
    ensure!(
        catalogue.eredu_revision == eredu_revision(),
        "Local model catalogue uses a different eredu revision; rebuild it"
    );
    ensure!(
        catalogue.profile == Profile::default(),
        "Local model catalogue forecast profile is stale; rebuild it"
    );
    ensure!(
        catalogue.benchmarks == super::quality::BENCHMARKS
            && !catalogue.quality_index_version.is_empty(),
        "Local model catalogue benchmark policy is stale"
    );
    ensure!(
        !catalogue.models.is_empty(),
        "Local model catalogue is empty"
    );
    chrono::DateTime::parse_from_rfc3339(&catalogue.generated_at)?;
    let mut repos = std::collections::HashSet::new();
    for model in &catalogue.models {
        ensure!(
            repos.insert(&model.repo_id)
                && model.repo_id.split('/').count() == 2
                && model.repo_id.split('/').all(|part| !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)))
                && valid_revision(&model.revision),
            "Invalid or duplicate catalogue model"
        );
        ensure!(
            super::quality::has_recommendation_evidence(Some(&model.quality))
                && !model.variants.is_empty(),
            "Missing catalogue quality evidence or variants"
        );
        let mut variants = std::collections::HashSet::new();
        for item in &model.variants {
            if require_lineage && model.quality.inherited {
                let lineage = item.lineage.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Missing benchmark lineage verification: {}", model.repo_id)
                })?;
                ensure!(
                    valid_revision(&lineage.evaluated_revision),
                    "Invalid evaluated revision"
                );
                lineage
                    .artifact
                    .matches(&lineage.evaluated, &item.variant.format)?;
            }
            ensure!(
                variants.insert(&item.variant.variant_id)
                    && !item.files.is_empty()
                    && !item.suitability_evidence.is_empty(),
                "Invalid catalogue variant"
            );
            ensure!(
                matches!(item.variant.format.as_str(), "gguf" | "safetensors"),
                "Unknown catalogue format"
            );
            ensure!(
                item.variant.model_id.starts_with(&model.repo_id),
                "Variant belongs to a different model"
            );
            let mut filenames = std::collections::HashSet::new();
            for file in &item.files {
                ensure!(
                    filenames.insert(&file.name)
                        && !file.name.is_empty()
                        && file
                            .name
                            .split('/')
                            .all(|part| !part.is_empty() && part != "." && part != "..")
                        && !file.name.contains('\\'),
                    "Invalid catalogue filename"
                );
            }
            ensure!(
                item.variant
                    .filename
                    .as_ref()
                    .is_none_or(|name| filenames.contains(name))
                    && item
                        .projector
                        .as_ref()
                        .is_none_or(|name| filenames.contains(name)),
                "Missing catalogue weight/projector file"
            );
            ensure!(
                item.memory
                    .upper_bytes
                    .is_none_or(|upper| upper >= item.memory.lower_bytes),
                "Invalid memory forecast interval"
            );
        }
    }
    Ok(())
}

pub fn bundled() -> Result<&'static Catalogue> {
    static CATALOGUE: OnceLock<Result<Catalogue, String>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| {
            let load = || -> Result<Catalogue> {
                let mut catalogue: Catalogue =
                    serde_json::from_str(include_str!("../../data/model-catalogue.json"))?;
                validate(&catalogue)?;
                for (position, model) in catalogue.models.iter_mut().enumerate() {
                    model.quality.catalogue_order = position as u64 + 1;
                }
                Ok(catalogue)
            };
            load().map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| anyhow::anyhow!(error.clone()))
}

pub fn find(repo: &str) -> Result<Option<&'static Model>> {
    Ok(bundled()?.models.iter().find(|model| model.repo_id == repo))
}

pub fn assess(model: &Model, variant: &Variant, available: u64) -> Result<Assessment> {
    let mut assessment = super::unknown_assessment(available);
    assessment.revision = Some(model.revision.clone());
    assessment.suitability = Suitability::Suitable;
    assessment.reasons = variant.suitability_evidence.clone();
    assessment
        .reasons
        .extend(variant.forecast_assumptions.iter().cloned());
    assessment
        .reasons
        .extend(variant.memory.uncertainties.iter().cloned());
    if super::supported() {
        assessment.compatibility = Compatibility::Compatible;
        assessment.fit = memory_fit(&variant.memory, available);
        assessment.lower_bytes = Some(variant.memory.lower_bytes);
        assessment.upper_bytes = variant.memory.upper_bytes;
    } else {
        assessment
            .reasons
            .push("Recommendations require a build with eredu support on Apple silicon.".into());
    }
    Ok(assessment)
}

pub fn available_variant(item: &Variant) -> HfModelVariant {
    let mut variant = item.variant.clone();
    if variant.format == "safetensors" {
        variant.unsupported_reason = crate::eredu::unavailable_reason().map(str::to_owned);
        variant.supported = variant.unsupported_reason.is_none();
    }
    variant
}
