//! Conservative evidence for transferring a checkpoint's benchmark scores to a conversion.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub architecture: String,
    pub dimensions: BTreeMap<String, u64>,
}

const DIMENSIONS: &[(&str, &str)] = &[
    ("hidden_size", "embedding_length"),
    ("num_hidden_layers", "block_count"),
    ("num_attention_heads", "attention.head_count"),
    ("num_key_value_heads", "attention.head_count_kv"),
    ("vocab_size", "vocab_size"),
];

impl Identity {
    pub fn from_config(config: &Value) -> Result<Self> {
        let config = config.get("text_config").unwrap_or(config);
        let architecture = config["model_type"]
            .as_str()
            .context("Missing model_type for benchmark lineage")?;
        let mut dimensions = BTreeMap::new();
        for (hf, _) in DIMENSIONS {
            let value = config[*hf]
                .as_u64()
                .or_else(|| {
                    (*hf == "num_key_value_heads")
                        .then(|| config["num_attention_heads"].as_u64())
                        .flatten()
                })
                .with_context(|| format!("Missing {hf} for benchmark lineage"))?;
            ensure!(value > 0, "Invalid {hf} for benchmark lineage");
            dimensions.insert((*hf).into(), value);
        }
        for (key, aliases) in [
            ("intermediate_size", &["intermediate_size"][..]),
            (
                "num_experts",
                &["num_local_experts", "n_routed_experts", "num_experts"][..],
            ),
        ] {
            if let Some(value) = aliases
                .iter()
                .find_map(|name| config[*name].as_u64())
                .filter(|v| *v > 0)
            {
                dimensions.insert(key.into(), value);
            }
        }
        Ok(Self {
            architecture: architecture.into(),
            dimensions,
        })
    }

    pub fn from_gguf(metadata: &serde_json::Map<String, Value>) -> Result<Self> {
        let architecture = metadata
            .get("general.architecture")
            .and_then(Value::as_str)
            .context("Missing GGUF architecture for benchmark lineage")?;
        let mut dimensions = BTreeMap::new();
        for (hf, gguf) in DIMENSIONS {
            let value = metadata
                .get(&format!("{architecture}.{gguf}"))
                .and_then(Value::as_u64)
                .or_else(|| {
                    (*hf == "vocab_size")
                        .then(|| {
                            metadata
                                .get("tokenizer.ggml.tokens.length")
                                .and_then(Value::as_u64)
                        })
                        .flatten()
                })
                .or_else(|| {
                    (*hf == "num_key_value_heads")
                        .then(|| {
                            metadata
                                .get(&format!("{architecture}.attention.head_count"))
                                .and_then(Value::as_u64)
                        })
                        .flatten()
                })
                .with_context(|| format!("Missing GGUF {gguf} for benchmark lineage"))?;
            ensure!(value > 0, "Invalid GGUF {gguf} for benchmark lineage");
            dimensions.insert((*hf).into(), value);
        }
        for (hf, gguf) in [
            ("intermediate_size", "feed_forward_length"),
            ("num_experts", "expert_count"),
        ] {
            if let Some(value) = metadata
                .get(&format!("{architecture}.{gguf}"))
                .and_then(Value::as_u64)
                .filter(|v| *v > 0)
            {
                dimensions.insert(hf.into(), value);
            }
        }
        Ok(Self {
            architecture: architecture.into(),
            dimensions,
        })
    }

    pub fn matches(&self, evaluated: &Self, format: &str) -> Result<()> {
        let normalized = |s: &str| s.strip_suffix("_text").unwrap_or(s).replace('_', "");
        // These HF architectures have established llama-container conversions.
        // This is a format mapping, not a model/repository allowlist.
        let llama_conversion = format == "gguf"
            && self.architecture == "llama"
            && matches!(
                evaluated.architecture.as_str(),
                "mistral" | "phi3" | "internlm3"
            );
        ensure!(
            normalized(&self.architecture) == normalized(&evaluated.architecture)
                || llama_conversion,
            "Benchmark lineage architecture mismatch: artifact {} versus evaluated {}",
            self.architecture,
            evaluated.architecture
        );
        for (name, _) in DIMENSIONS {
            let expected = evaluated
                .dimensions
                .get(*name)
                .filter(|value| **value > 0)
                .with_context(|| format!("Missing evaluated {name}"))?;
            ensure!(
                self.dimensions.get(*name) == Some(expected),
                "Benchmark lineage {name} mismatch: artifact {:?} versus evaluated {expected}",
                self.dimensions.get(*name)
            );
        }
        for (name, expected) in &evaluated.dimensions {
            ensure!(
                self.dimensions.get(name) == Some(expected),
                "Benchmark lineage {name} mismatch: artifact {:?} versus evaluated {expected}",
                self.dimensions.get(name)
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub evaluated_revision: String,
    pub evaluated: Identity,
    pub artifact: Identity,
}

/// Declarations of changed weights cannot establish quantization-only lineage.
/// The evaluated checkpoint may itself be distilled/modified; only *new* claims exclude a conversion.
pub fn preserves_modification_claims(
    repo: &str,
    info: &Value,
    parent: &str,
    parent_info: &Value,
) -> bool {
    fn claims(repo: &str, info: &Value) -> std::collections::BTreeSet<String> {
        let mut text = repo.rsplit('/').next().unwrap_or(repo).to_lowercase();
        for tag in info["tags"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !tag.starts_with("base_model:") {
                text.push(' ');
                text.push_str(&tag.to_lowercase());
            }
        }
        if let Some(name) = info["cardData"]["model_name"].as_str() {
            text.push(' ');
            text.push_str(&name.to_lowercase());
        }
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .filter_map(|word| {
                if word.starts_with("abliterat") {
                    Some("abliteration")
                } else if word.starts_with("distill") {
                    Some("distillation")
                } else if matches!(
                    word,
                    "finetune"
                        | "finetuned"
                        | "sft"
                        | "dpo"
                        | "adapter"
                        | "lora"
                        | "merge"
                        | "merged"
                        | "uncensored"
                ) {
                    Some(word)
                } else {
                    None
                }
            })
            .map(str::to_owned)
            .collect()
    }
    claims(repo, info).is_subset(&claims(parent, parent_info))
}

#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
pub(super) async fn evaluated(
    hub: &super::transport::Hub,
    repo: &str,
    manifest: &Value,
    parent: &str,
) -> Result<(String, Identity)> {
    ensure!(
        super::quality::quantized_parent(manifest) == Some(parent),
        "Benchmark lineage no longer declares an unambiguous quantization of {parent}"
    );
    let info = hub.model(parent).await?;
    ensure!(
        super::quality::preserves_evaluated_datasets(manifest, &info),
        "Benchmark lineage declares additional datasets (possible training or calibration)"
    );
    ensure!(
        preserves_modification_claims(repo, manifest, parent, &info),
        "Benchmark lineage declares a modified checkpoint; it needs its own benchmark results"
    );
    let revision = info["sha"]
        .as_str()
        .filter(|sha| super::catalogue::valid_revision(sha))
        .context("Missing evaluated checkpoint revision")?;
    let config = hub.file(parent, revision, "config.json").await?;
    Ok((
        revision.into(),
        Identity::from_config(&serde_json::from_slice(&config)?)?,
    ))
}

#[cfg(all(feature = "eredu", target_os = "macos", target_arch = "aarch64"))]
pub(super) async fn verify(
    hub: &super::transport::Hub,
    repo: &str,
    revision: &str,
    variant: &crate::hf_models::HfModelVariant,
    parent: &(String, Identity),
) -> Result<Verification> {
    let artifact = if variant.format == "safetensors" {
        Identity::from_config(&serde_json::from_slice(
            &hub.file(repo, revision, "config.json").await?,
        )?)?
    } else {
        let filename = variant
            .filename
            .as_deref()
            .context("Missing GGUF filename")?;
        let mut end = 65535;
        loop {
            let (bytes, length) = hub.range(repo, revision, filename, 0, end).await?;
            let metadata = super::gguf::prefix_metadata(&bytes)?;
            match Identity::from_gguf(&metadata) {
                Ok(identity) => break identity,
                Err(error)
                    if end + 1 >= length
                        || end >= 16 * 1024 * 1024 - 1
                        || super::gguf::header_end(&bytes)?.is_ok() =>
                {
                    return Err(error)
                }
                Err(_) => end = ((end + 1) * 2 - 1).min(length - 1),
            }
        }
    };
    artifact.matches(&parent.1, &variant.format)?;
    Ok(Verification {
        evaluated_revision: parent.0.clone(),
        evaluated: parent.1.clone(),
        artifact,
    })
}
