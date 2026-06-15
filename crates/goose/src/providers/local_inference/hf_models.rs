use anyhow::{bail, Result};
use futures::StreamExt;
use hf_hub::progress::{DownloadEvent, FileStatus, ProgressEvent, ProgressHandler};
use hf_hub::repository::{ModelInfo, RepoSibling};
use hf_hub::{HFClient, HFRepository, RepoTypeModel};

use super::local_model_registry::{get_registry, model_id_from_repo, LocalModelStorage, ShardFile};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::providers::huggingface_auth;

use utoipa::ToSchema;

const HF_API_BASE: &str = "https://huggingface.co/api/models";
const HF_DOWNLOAD_BASE: &str = "https://huggingface.co";
const LLAMACPP_BACKEND_ID: &str = "llamacpp";
const MLX_BACKEND_ID: &str = "mlx";
const GGUF_FORMAT: &str = "gguf";
const MLX_FORMAT: &str = "mlx-safetensors";
const MLX_VARIANT_ID: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HfModelInfo {
    pub repo_id: String,
    pub author: String,
    pub model_name: String,
    pub downloads: u64,
    pub gguf_files: Vec<HfGgufFile>,
    #[serde(default)]
    pub variants: Vec<HfModelVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HfModelVariant {
    pub variant_id: String,
    pub label: String,
    pub backend_id: String,
    pub format: String,
    pub model_id: String,
    pub download_id: String,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    pub description: String,
    pub quality_rank: u8,
    #[serde(default)]
    pub sharded: bool,
    #[serde(default = "default_supported")]
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<String>,
}

fn default_supported() -> bool {
    true
}

/// A single downloadable GGUF file (used internally and for downloads).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HfGgufFile {
    pub filename: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub download_url: String,
}

/// A quantization variant — groups sharded files into one logical entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HfQuantVariant {
    pub quantization: String,
    pub size_bytes: u64,
    pub filename: String,
    pub download_url: String,
    pub description: &'static str,
    pub quality_rank: u8,
    #[serde(default)]
    pub sharded: bool,
}

impl HfQuantVariant {
    pub fn to_model_variant(&self, repo_id: &str) -> HfModelVariant {
        let model_id = model_id_from_repo(repo_id, &self.quantization);
        HfModelVariant {
            variant_id: self.quantization.clone(),
            label: self.quantization.clone(),
            backend_id: LLAMACPP_BACKEND_ID.to_string(),
            format: GGUF_FORMAT.to_string(),
            model_id: model_id.clone(),
            download_id: model_id,
            size_bytes: self.size_bytes,
            filename: Some(self.filename.clone()),
            download_url: Some(self.download_url.clone()),
            description: self.description.to_string(),
            quality_rank: self.quality_rank,
            sharded: self.sharded,
            supported: true,
            unsupported_reason: None,
        }
    }
}

/// Result of resolving a model spec — may contain multiple shard files.
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    pub files: Vec<HfGgufFile>,
    pub total_size: u64,
    pub mmproj: Option<HfGgufFile>,
}

#[derive(Debug, Clone)]
pub enum ResolvedLocalModel {
    Gguf {
        repo_id: String,
        quantization: String,
        resolved: ResolvedModel,
        local_paths: Vec<std::path::PathBuf>,
        mmproj_path: Option<std::path::PathBuf>,
        storage: LocalModelStorage,
    },
    Mlx {
        repo_id: String,
        variant_id: String,
        snapshot_path: std::path::PathBuf,
        total_size: u64,
    },
}

impl ResolvedLocalModel {
    pub fn repo_id(&self) -> &str {
        match self {
            Self::Gguf { repo_id, .. } | Self::Mlx { repo_id, .. } => repo_id,
        }
    }

    pub fn variant_id(&self) -> &str {
        match self {
            Self::Gguf { quantization, .. } => quantization,
            Self::Mlx { variant_id, .. } => variant_id,
        }
    }

    pub fn backend_id(&self) -> &'static str {
        match self {
            Self::Gguf { .. } => "llamacpp",
            Self::Mlx { .. } => "mlx",
        }
    }

    pub fn model_id(&self) -> String {
        match self {
            Self::Gguf {
                repo_id,
                quantization,
                ..
            } => model_id_from_repo(repo_id, quantization),
            Self::Mlx { repo_id, .. } => repo_id.clone(),
        }
    }

    pub fn total_size(&self) -> u64 {
        match self {
            Self::Gguf { resolved, .. } => resolved.total_size,
            Self::Mlx { total_size, .. } => *total_size,
        }
    }

    pub fn storage(&self) -> LocalModelStorage {
        match self {
            Self::Gguf { storage, .. } => *storage,
            Self::Mlx { .. } => LocalModelStorage::HuggingFaceCache,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HfApiModel {
    id: Option<String>,
    author: Option<String>,
    downloads: Option<u64>,
    siblings: Option<Vec<HfApiSibling>>,
}

#[derive(Debug, Deserialize)]
struct HfApiSibling {
    rfilename: String,
    #[serde(default)]
    size: Option<u64>,
}

struct QuantInfo {
    description: &'static str,
    quality_rank: u8,
}

// quality_rank groups quants by bit-level so that all N-bit variants sort
// together. Within a group, higher rank = higher quality.
//
//   1-bit:  10–19      4-bit:  40–49      8-bit:  80–89
//   2-bit:  20–29      5-bit:  50–59      16-bit: 90–94
//   3-bit:  30–39      6-bit:  60–69      32-bit: 95–99
//
const QUANT_TABLE: &[(&str, &str, u8)] = &[
    // 1-bit
    ("TQ1_0", "Tiny, ternary quantization", 10),
    ("IQ1_S", "Extremely small, very low quality", 11),
    ("IQ1_M", "Extremely small, very low quality", 12),
    // 2-bit
    ("IQ2_XXS", "Very small, low quality", 20),
    ("IQ2_XS", "Very small, low quality", 21),
    ("IQ2_S", "Very small, low quality", 22),
    ("IQ2_M", "Very small, low quality", 23),
    ("Q2_K", "Small, low quality", 24),
    ("Q2_K_S", "Small, low quality", 24),
    ("Q2_K_L", "Small, low quality", 25),
    ("Q2_K_XL", "Small, low quality", 26),
    // 3-bit
    ("IQ3_XXS", "Very small, moderate quality loss", 30),
    ("IQ3_XS", "Small, moderate quality loss", 31),
    ("IQ3_S", "Small, moderate quality loss", 32),
    ("IQ3_M", "Small, moderate quality loss", 33),
    ("Q3_K_S", "Small, moderate quality loss", 34),
    ("Q3_K_M", "Small, balanced quality/size", 35),
    ("Q3_K_L", "Medium-small, decent quality", 36),
    ("Q3_K_XL", "Medium-small, decent quality", 37),
    // 4-bit
    ("IQ4_XS", "Medium, good quality", 40),
    ("IQ4_NL", "Medium, good quality", 41),
    ("Q4_0", "Medium, good quality", 42),
    ("Q4_1", "Medium, good quality", 43),
    ("Q4_K_S", "Medium, good quality/size balance", 44),
    (
        "Q4_K_M",
        "Medium, recommended balance of quality and size",
        45,
    ),
    ("Q4_K_L", "Medium, good quality", 46),
    ("Q4_K_XL", "Medium, good quality", 47),
    (
        "MXFP4_MOE",
        "Medium, mixed-precision 4-bit for MoE models",
        48,
    ),
    // 5-bit
    ("Q5_0", "Medium-large, high quality", 50),
    ("Q5_1", "Medium-large, high quality", 51),
    ("Q5_K_S", "Medium-large, high quality", 52),
    ("Q5_K_M", "Medium-large, very high quality", 53),
    ("Q5_K_XL", "Medium-large, very high quality", 54),
    // 6-bit
    ("Q6_K", "Large, near-lossless quality", 60),
    ("Q6_K_XL", "Large, near-lossless quality", 61),
    // 8-bit
    ("Q8_0", "Large, near-lossless quality", 80),
    ("Q8_K_XL", "Large, near-lossless quality", 81),
    // 16-bit
    ("F16", "Full size, original quality (16-bit)", 90),
    ("BF16", "Full size, original quality (bfloat16)", 91),
    // 32-bit
    ("F32", "Full size, original quality (32-bit)", 95),
];

fn quant_info(quant: &str) -> QuantInfo {
    QUANT_TABLE
        .iter()
        .find(|(name, _, _)| *name == quant)
        .map(|(_, description, quality_rank)| QuantInfo {
            description,
            quality_rank: *quality_rank,
        })
        .unwrap_or(QuantInfo {
            description: "",
            quality_rank: 45,
        })
}

pub fn parse_quantization_from_filename(filename: &str) -> String {
    parse_quantization(filename)
}

fn parse_quantization(filename: &str) -> String {
    // Strip directory prefix (e.g. "Q5_K_M/Model-Q5_K_M-00001-of-00002.gguf")
    let basename = filename.rsplit('/').next().unwrap_or(filename);
    let stem = basename.trim_end_matches(".gguf");

    // Strip shard suffix like "-00001-of-00004"
    let stem = if let Some(pos) = stem.rfind("-of-") {
        stem.get(..pos)
            .and_then(|s| s.rsplit_once('-').map(|(prefix, _)| prefix))
            .unwrap_or(stem)
    } else {
        stem
    };

    // The quantization tag is typically the last hyphen-separated component
    // that looks like a quant identifier (starts with Q, IQ, F, BF, TQ, MXFP, etc.)
    // e.g. "Qwen3-Coder-Next-Q4_K_M" -> "Q4_K_M"
    //      "Model-UD-IQ1_M" -> "IQ1_M"
    if let Some((_, candidate)) = stem.rsplit_once('-') {
        if looks_like_quant(candidate) {
            return candidate.to_string();
        }
    }

    // Fallback: try dot separator (e.g. "model.Q4_K_M")
    if let Some((_, candidate)) = stem.rsplit_once('.') {
        if looks_like_quant(candidate) {
            return candidate.to_string();
        }
    }

    "unknown".to_string()
}

fn quant_bits(quantization: &str) -> u8 {
    let digits: String = quantization
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

fn mmproj_precision_preference(quantization: &str) -> u8 {
    match quantization.to_uppercase().as_str() {
        "BF16" => 3,
        "F16" => 2,
        "F32" => 1,
        _ => 0,
    }
}

fn looks_like_quant(s: &str) -> bool {
    let upper = s.to_uppercase();
    upper.starts_with("Q")
        || upper.starts_with("IQ")
        || upper.starts_with("TQ")
        || upper.starts_with("MXFP")
        || upper == "F16"
        || upper == "F32"
        || upper == "BF16"
}

fn is_shard_file(filename: &str) -> bool {
    // Matches patterns like "-00001-of-00003.gguf"
    parse_shard_index(filename).is_some()
}

/// Parse the shard index (1-based) from a filename like "model-BF16-00001-of-00002.gguf".
fn parse_shard_index(filename: &str) -> Option<u32> {
    let basename = filename.rsplit('/').next().unwrap_or(filename);
    let stem = basename.trim_end_matches(".gguf");
    let pos = stem.rfind("-of-")?;
    let before = stem.get(..pos)?;
    let idx_str = before.rsplit('-').next()?;
    if !idx_str.is_empty() && idx_str.chars().all(|c| c.is_ascii_digit()) {
        idx_str.parse().ok()
    } else {
        None
    }
}

/// Parse the total shard count from a filename like "model-BF16-00001-of-00002.gguf".
fn parse_shard_total(filename: &str) -> Option<u32> {
    let basename = filename.rsplit('/').next().unwrap_or(filename);
    let stem = basename.trim_end_matches(".gguf");
    let pos = stem.rfind("-of-")?;
    let total_str = stem.get(pos + 4..)?;
    total_str.parse().ok()
}

fn build_download_url(repo_id: &str, filename: &str) -> String {
    format!("{}/{}/resolve/main/{}", HF_DOWNLOAD_BASE, repo_id, filename)
}

pub fn hf_authorization_header(token: Option<&str>) -> Option<String> {
    token
        .filter(|token| !token.is_empty())
        .map(|token| format!("Bearer {}", token))
}

fn apply_hf_auth(request: reqwest::RequestBuilder, token: Option<&str>) -> reqwest::RequestBuilder {
    if let Some(header) = hf_authorization_header(token) {
        request.header("Authorization", header)
    } else {
        request
    }
}

async fn optional_hf_token(
    token: impl std::future::Future<Output = Result<Option<String>>>,
) -> Option<String> {
    token.await.ok().flatten()
}

fn parent_components(filename: &str) -> Vec<&str> {
    filename.rsplit_once('/').map_or(Vec::new(), |(parent, _)| {
        parent.split('/').filter(|part| !part.is_empty()).collect()
    })
}

fn is_prefix(prefix: &[&str], parts: &[&str]) -> bool {
    prefix.len() <= parts.len() && prefix.iter().zip(parts).all(|(a, b)| a == b)
}

fn select_best_mmproj(
    repo_id: &str,
    siblings: &[HfApiSibling],
    model_filename: &str,
    model_quantization: &str,
) -> Option<HfGgufFile> {
    let model_dir = parent_components(model_filename);
    let model_bits = quant_bits(model_quantization);

    siblings
        .iter()
        .filter(|s| {
            let lowercase = s.rfilename.to_lowercase();
            lowercase.ends_with(".gguf") && lowercase.contains("mmproj")
        })
        .filter_map(|s| {
            let mmproj_dir = parent_components(&s.rfilename);
            if !is_prefix(&mmproj_dir, &model_dir) {
                return None;
            }

            let quantization = parse_quantization(&s.rfilename);
            let bits = quant_bits(&quantization);
            let diff = bits.abs_diff(model_bits);
            let proximity = u8::MAX - diff;

            Some((
                mmproj_dir.len(),
                proximity,
                mmproj_precision_preference(&quantization),
                s,
                quantization,
            ))
        })
        .max_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| b.3.rfilename.cmp(&a.3.rfilename))
        })
        .map(|(_, _, _, sibling, quantization)| HfGgufFile {
            filename: sibling.rfilename.clone(),
            size_bytes: sibling.size.unwrap_or(0),
            quantization,
            download_url: build_download_url(repo_id, &sibling.rfilename),
        })
}

/// Derive the expected model filename stem from a repo_id.
/// e.g. "unsloth/gemma-4-26B-A4B-it-GGUF" → "gemma-4-26b-a4b-it" (lowercased)
fn model_stem_from_repo(repo_id: &str) -> String {
    let repo_name = repo_id.rsplit('/').next().unwrap_or(repo_id);
    let stem = repo_name
        .strip_suffix("-GGUF")
        .or_else(|| repo_name.strip_suffix("-gguf"))
        .unwrap_or(repo_name);
    stem.to_lowercase()
}

/// Check whether a GGUF file belongs to the main model (vs auxiliary files like mmproj).
/// Matches files whose basename starts with the model stem derived from the repo name.
fn is_model_file(filename: &str, model_stem_lower: &str) -> bool {
    let basename = filename.rsplit('/').next().unwrap_or(filename);
    basename.to_lowercase().starts_with(model_stem_lower)
}

/// Collect GGUF files into quantization variants.
/// Single-file quants use the file directly.
/// Sharded quants (multiple files for one quantization) aggregate sizes and use the
/// first shard filename as the representative — the download path must handle all shards.
fn group_into_variants(repo_id: &str, files: Vec<HfApiSibling>) -> Vec<HfQuantVariant> {
    use std::collections::HashMap;

    let stem = model_stem_from_repo(repo_id);

    let gguf_files: Vec<_> = files
        .into_iter()
        .filter(|s| {
            s.rfilename.ends_with(".gguf")
                && is_model_file(&s.rfilename, &stem)
                && parse_quantization(&s.rfilename) != "unknown"
        })
        .collect();

    // Separate single files from shards
    let mut single_files: Vec<&HfApiSibling> = Vec::new();
    let mut shard_groups: HashMap<String, Vec<&HfApiSibling>> = HashMap::new();

    for file in &gguf_files {
        if is_shard_file(&file.rfilename) {
            let quant = parse_quantization(&file.rfilename);
            shard_groups.entry(quant).or_default().push(file);
        } else {
            single_files.push(file);
        }
    }

    let mut variants: Vec<HfQuantVariant> = Vec::new();
    let mut seen_quants: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Add single-file variants
    for s in single_files {
        let quant = parse_quantization(&s.rfilename);
        seen_quants.insert(quant.clone());
        let info = quant_info(&quant);
        let download_url = build_download_url(repo_id, &s.rfilename);
        variants.push(HfQuantVariant {
            quantization: quant,
            size_bytes: s.size.unwrap_or(0),
            filename: s.rfilename.clone(),
            download_url,
            description: info.description,
            quality_rank: info.quality_rank,
            sharded: false,
        });
    }

    // Add shard-only variants (quants that only exist as sharded files)
    for (quant, mut shards) in shard_groups {
        if seen_quants.contains(&quant) {
            continue;
        }
        shards.sort_by(|a, b| a.rfilename.cmp(&b.rfilename));
        let total_size: u64 = shards.iter().map(|s| s.size.unwrap_or(0)).sum();
        let info = quant_info(&quant);
        let first_filename = &shards[0].rfilename;
        let download_url = build_download_url(repo_id, first_filename);
        variants.push(HfQuantVariant {
            quantization: quant,
            size_bytes: total_size,
            filename: first_filename.clone(),
            download_url,
            description: info.description,
            quality_rank: info.quality_rank,
            sharded: true,
        });
    }

    // Sort descending by quality_rank, then by size descending as tiebreaker
    variants.sort_by(|a, b| {
        b.quality_rank
            .cmp(&a.quality_rank)
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
    });
    variants
}

pub async fn search_local_models(query: &str, limit: usize) -> Result<Vec<HfModelInfo>> {
    let mut results = Vec::new();

    if looks_like_repo_id(query) {
        if let Some(model) = get_local_model_info_for_repo(query).await? {
            results.push(model);
        }
    } else if let Some(model) = get_exact_name_local_model_info(query).await? {
        results.push(model);
    }

    let mut gguf_results = search_gguf_models(query, limit).await?;
    for model in &mut gguf_results {
        let gguf_variants = get_repo_gguf_variants(&model.repo_id)
            .await
            .unwrap_or_default();
        model.variants = gguf_variants
            .iter()
            .map(|variant| variant.to_model_variant(&model.repo_id))
            .collect();
    }

    results.extend(gguf_results);
    append_optional_mlx_results(&mut results, search_mlx_models(query, limit).await, query);
    dedupe_models(&mut results);
    results.sort_by(|a, b| {
        model_search_rank(query, a)
            .cmp(&model_search_rank(query, b))
            .then_with(|| b.downloads.cmp(&a.downloads))
    });
    results.truncate(limit);
    Ok(results)
}

fn append_optional_mlx_results(
    results: &mut Vec<HfModelInfo>,
    mlx_results: Result<Vec<HfModelInfo>>,
    query: &str,
) {
    match mlx_results {
        Ok(models) => results.extend(models),
        Err(error) => tracing::warn!(
            query,
            error = %error,
            "Failed to search MLX models; returning non-MLX results"
        ),
    }
}

pub async fn search_gguf_models(query: &str, limit: usize) -> Result<Vec<HfModelInfo>> {
    let client = reqwest::Client::new();
    let token = optional_hf_token(huggingface_auth::resolve_token_async()).await;
    let url = format!(
        "{}?search={}&filter=gguf&sort=downloads&direction=-1&limit={}",
        HF_API_BASE, query, limit
    );

    let response = apply_hf_auth(client.get(&url), token.as_deref())
        .header("User-Agent", "goose-ai-agent")
        .send()
        .await?;

    if !response.status().is_success() {
        bail!("HuggingFace API returned status {}", response.status());
    }

    let models: Vec<HfApiModel> = response.json().await?;

    let results = models
        .into_iter()
        .filter_map(|m| {
            let repo_id = m.id?;
            let siblings = m.siblings.unwrap_or_default();

            // The search endpoint may not include `siblings`; parse whatever
            // is available. Files are fetched on-demand via `get_repo_gguf_variants`.
            let gguf_files: Vec<HfGgufFile> = siblings
                .into_iter()
                .filter(|s| s.rfilename.ends_with(".gguf"))
                .map(|s| {
                    let quantization = parse_quantization(&s.rfilename);
                    let download_url = build_download_url(&repo_id, &s.rfilename);
                    HfGgufFile {
                        filename: s.rfilename,
                        size_bytes: s.size.unwrap_or(0),
                        quantization,
                        download_url,
                    }
                })
                .collect();

            let author = m
                .author
                .unwrap_or_else(|| repo_id.split('/').next().unwrap_or_default().to_string());
            let model_name = repo_id
                .split('/')
                .next_back()
                .unwrap_or(&repo_id)
                .to_string();

            Some(HfModelInfo {
                repo_id,
                author,
                model_name,
                downloads: m.downloads.unwrap_or(0),
                gguf_files,
                variants: Vec::new(),
            })
        })
        .collect();

    Ok(results)
}

/// Fetch GGUF files for a repo and return them grouped by quantization.
pub async fn get_repo_gguf_variants(repo_id: &str) -> Result<Vec<HfQuantVariant>> {
    let client = reqwest::Client::new();
    let token = optional_hf_token(huggingface_auth::resolve_token_async()).await;
    let url = format!("{}/{}?blobs=true", HF_API_BASE, repo_id);

    let response = apply_hf_auth(client.get(&url), token.as_deref())
        .header("User-Agent", "goose-ai-agent")
        .send()
        .await?;

    if !response.status().is_success() {
        bail!(
            "HuggingFace API returned status {} for repo {}",
            response.status(),
            repo_id
        );
    }

    let model: HfApiModel = response.json().await?;
    let siblings = model.siblings.unwrap_or_default();

    Ok(group_into_variants(repo_id, siblings))
}

/// Fetch raw GGUF files (kept for resolve_model_spec).
pub async fn get_repo_gguf_files(repo_id: &str) -> Result<Vec<HfGgufFile>> {
    let client = reqwest::Client::new();
    let token = optional_hf_token(huggingface_auth::resolve_token_async()).await;
    let url = format!("{}/{}?blobs=true", HF_API_BASE, repo_id);

    let response = apply_hf_auth(client.get(&url), token.as_deref())
        .header("User-Agent", "goose-ai-agent")
        .send()
        .await?;

    if !response.status().is_success() {
        bail!(
            "HuggingFace API returned status {} for repo {}",
            response.status(),
            repo_id
        );
    }

    let model: HfApiModel = response.json().await?;
    let siblings = model.siblings.unwrap_or_default();

    let stem = model_stem_from_repo(repo_id);

    let files = siblings
        .into_iter()
        .filter(|s| s.rfilename.ends_with(".gguf"))
        .filter(|s| !is_shard_file(&s.rfilename))
        .filter(|s| is_model_file(&s.rfilename, &stem))
        .map(|s| {
            let quantization = parse_quantization(&s.rfilename);
            let download_url = build_download_url(repo_id, &s.rfilename);
            HfGgufFile {
                filename: s.rfilename,
                size_bytes: s.size.unwrap_or(0),
                quantization,
                download_url,
            }
        })
        .collect();

    Ok(files)
}

/// Parse a model spec like "bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M" into (repo_id, quantization).
pub fn parse_model_spec(spec: &str) -> Result<(String, String)> {
    let (repo_id, quant) = spec.rsplit_once(':').ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid model spec '{}': expected format 'user/repo:quantization'",
            spec
        )
    })?;

    if !repo_id.contains('/') {
        bail!("Invalid repo_id '{}': expected format 'user/repo'", repo_id);
    }

    if quant.is_empty() {
        bail!(
            "Invalid model spec '{}': expected format 'user/repo:quantization'",
            spec
        );
    }

    Ok((repo_id.to_string(), quant.to_string()))
}

/// Resolve a model spec to all GGUF files for that quantization (handles shards).
pub async fn resolve_model_spec_full(spec: &str) -> Result<(String, ResolvedModel)> {
    let (repo_id, quant) = parse_model_spec(spec)?;

    let client = reqwest::Client::new();
    let token = optional_hf_token(huggingface_auth::resolve_token_async()).await;
    let url = format!("{}/{}?blobs=true", HF_API_BASE, repo_id);
    let response = apply_hf_auth(client.get(&url), token.as_deref())
        .header("User-Agent", "goose-ai-agent")
        .send()
        .await?;

    if !response.status().is_success() {
        bail!(
            "HuggingFace API returned status {} for repo {}",
            response.status(),
            repo_id
        );
    }

    let model: HfApiModel = response.json().await?;
    let siblings = model.siblings.unwrap_or_default();
    let stem = model_stem_from_repo(&repo_id);

    // Collect all GGUF files matching the quantization
    let matching: Vec<_> = siblings
        .iter()
        .filter(|s| {
            s.rfilename.ends_with(".gguf")
                && is_model_file(&s.rfilename, &stem)
                && parse_quantization(&s.rfilename).eq_ignore_ascii_case(&quant)
        })
        .collect();

    if matching.is_empty() {
        bail!(
            "No GGUF file with quantization '{}' found in {}",
            quant,
            repo_id
        );
    }

    // Separate single files from shards
    let mut single_files: Vec<&HfApiSibling> = Vec::new();
    let mut shard_files: Vec<&HfApiSibling> = Vec::new();
    for &f in &matching {
        if is_shard_file(&f.rfilename) {
            shard_files.push(f);
        } else {
            single_files.push(f);
        }
    }

    // Prefer single file if available
    if let Some(single) = single_files.first() {
        let mmproj = select_best_mmproj(&repo_id, &siblings, &single.rfilename, &quant);
        let file = HfGgufFile {
            filename: single.rfilename.clone(),
            size_bytes: single.size.unwrap_or(0),
            quantization: quant,
            download_url: build_download_url(&repo_id, &single.rfilename),
        };
        let total_size = file.size_bytes;
        return Ok((
            repo_id,
            ResolvedModel {
                files: vec![file],
                total_size,
                mmproj,
            },
        ));
    }

    // Use shards, sorted by filename so shard 1 is first
    shard_files.sort_by(|a, b| a.rfilename.cmp(&b.rfilename));

    // Validate shard set completeness: every file must parse to the same
    // -of-N total, and indices must be contiguous 1..=N.
    let expected_total = parse_shard_total(&shard_files[0].rfilename).ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot parse shard total from '{}'",
            shard_files[0].rfilename
        )
    })?;
    if shard_files.len() != expected_total as usize {
        bail!(
            "Incomplete shard set for '{}' in {}: found {} of {} shards",
            quant,
            repo_id,
            shard_files.len(),
            expected_total
        );
    }
    for (i, shard) in shard_files.iter().enumerate() {
        let shard_total = parse_shard_total(&shard.rfilename);
        if shard_total != Some(expected_total) {
            bail!(
                "Inconsistent shard totals for '{}' in {}: shard '{}' has total {:?}, expected {}",
                quant,
                repo_id,
                shard.rfilename,
                shard_total,
                expected_total
            );
        }
        let idx = parse_shard_index(&shard.rfilename);
        if idx != Some((i + 1) as u32) {
            bail!(
                "Non-contiguous shard set for '{}' in {}: expected shard {} but found {:?}",
                quant,
                repo_id,
                i + 1,
                idx
            );
        }
    }

    let files: Vec<HfGgufFile> = shard_files
        .iter()
        .map(|s| HfGgufFile {
            filename: s.rfilename.clone(),
            size_bytes: s.size.unwrap_or(0),
            quantization: quant.clone(),
            download_url: build_download_url(&repo_id, &s.rfilename),
        })
        .collect();
    let total_size: u64 = files.iter().map(|f| f.size_bytes).sum();

    let mmproj = select_best_mmproj(&repo_id, &siblings, &files[0].filename, &quant);

    Ok((
        repo_id,
        ResolvedModel {
            files,
            total_size,
            mmproj,
        },
    ))
}

/// Resolve a model spec to a specific GGUF file from the repo.
pub async fn resolve_model_spec(spec: &str) -> Result<(String, HfGgufFile)> {
    let (repo_id, resolved) = resolve_model_spec_full(spec).await?;
    if resolved.files.len() > 1 {
        bail!(
            "Model '{}' is sharded ({} files) — use resolve_model_spec_full instead",
            spec,
            resolved.files.len()
        );
    }
    Ok((repo_id, resolved.files.into_iter().next().unwrap()))
}

/// Recommend which quantization variant to use based on available memory.
pub fn recommend_variant(
    variants: &[HfQuantVariant],
    available_memory_bytes: u64,
) -> Option<usize> {
    // We need ~10-20% overhead beyond model size for inference context.
    // Pick the highest-quality variant that fits.
    let usable = (available_memory_bytes as f64 * 0.85) as u64;

    let mut best: Option<usize> = None;
    for (i, v) in variants.iter().enumerate() {
        if v.size_bytes <= usable {
            match best {
                Some(bi) if variants[bi].quality_rank < v.quality_rank => best = Some(i),
                None => best = Some(i),
                _ => {}
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quantization() {
        assert_eq!(parse_quantization("Model-Q4_K_M.gguf"), "Q4_K_M");
        assert_eq!(parse_quantization("Model-Q8_0.gguf"), "Q8_0");
        assert_eq!(parse_quantization("Model-IQ4_NL.gguf"), "IQ4_NL");
        assert_eq!(parse_quantization("Model-F16.gguf"), "F16");
        assert_eq!(parse_quantization("random-name.gguf"), "unknown");
    }

    #[test]
    fn test_hf_authorization_header() {
        assert_eq!(
            hf_authorization_header(Some("hf_test")).as_deref(),
            Some("Bearer hf_test")
        );
        assert_eq!(hf_authorization_header(Some("")), None);
        assert_eq!(hf_authorization_header(None), None);
    }

    #[test]
    fn test_parse_quantization_with_directory() {
        assert_eq!(
            parse_quantization("Q5_K_M/Model-Q5_K_M-00001-of-00002.gguf"),
            "Q5_K_M"
        );
    }

    #[test]
    fn test_parse_quantization_extended_tags() {
        assert_eq!(parse_quantization("Model-MXFP4_MOE.gguf"), "MXFP4_MOE");
        assert_eq!(parse_quantization("Model-UD-TQ1_0.gguf"), "TQ1_0");
        assert_eq!(parse_quantization("Model-Q2_K_L.gguf"), "Q2_K_L");
        assert_eq!(parse_quantization("Model-UD-Q4_K_XL.gguf"), "Q4_K_XL");
        assert_eq!(parse_quantization("Model-UD-IQ1_M.gguf"), "IQ1_M");
    }

    #[test]
    fn test_is_shard_file() {
        assert!(is_shard_file("Q5_K_M/Model-Q5_K_M-00001-of-00002.gguf"));
        assert!(is_shard_file("Model-BF16-00003-of-00004.gguf"));
        assert!(!is_shard_file("Model-Q4_K_M.gguf"));
    }

    #[test]
    fn test_parse_model_spec() {
        let (repo, quant) =
            parse_model_spec("bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M").unwrap();
        assert_eq!(repo, "bartowski/Llama-3.2-1B-Instruct-GGUF");
        assert_eq!(quant, "Q4_K_M");
    }

    #[test]
    fn test_parse_model_spec_invalid() {
        assert!(parse_model_spec("no-colon").is_err());
        assert!(parse_model_spec("noslash:Q4_K_M").is_err());
        assert!(parse_model_spec("owner/repo:").is_err());
    }

    #[test]
    fn test_dedupe_models_merges_variants_for_same_repo() {
        let repo_id = "mixed/repo".to_string();
        let gguf_variant = HfModelVariant {
            variant_id: "Q4_K_M".to_string(),
            label: "Q4_K_M".to_string(),
            backend_id: LLAMACPP_BACKEND_ID.to_string(),
            format: GGUF_FORMAT.to_string(),
            model_id: "mixed/repo:Q4_K_M".to_string(),
            download_id: "mixed/repo:Q4_K_M".to_string(),
            size_bytes: 4,
            filename: Some("model-Q4_K_M.gguf".to_string()),
            download_url: Some("https://example.test/model-Q4_K_M.gguf".to_string()),
            description: "Medium".to_string(),
            quality_rank: 45,
            sharded: false,
            supported: true,
            unsupported_reason: None,
        };
        let mlx_variant = HfModelVariant {
            variant_id: MLX_VARIANT_ID.to_string(),
            label: "Default".to_string(),
            backend_id: MLX_BACKEND_ID.to_string(),
            format: MLX_FORMAT.to_string(),
            model_id: repo_id.clone(),
            download_id: repo_id.clone(),
            size_bytes: 8,
            filename: None,
            download_url: None,
            description: "MLX".to_string(),
            quality_rank: 91,
            sharded: true,
            supported: true,
            unsupported_reason: None,
        };
        let mut models = vec![
            HfModelInfo {
                repo_id: repo_id.clone(),
                author: "mixed".to_string(),
                model_name: "repo".to_string(),
                downloads: 1,
                gguf_files: vec![HfGgufFile {
                    filename: "model-Q4_K_M.gguf".to_string(),
                    size_bytes: 4,
                    quantization: "Q4_K_M".to_string(),
                    download_url: "https://example.test/model-Q4_K_M.gguf".to_string(),
                }],
                variants: vec![gguf_variant],
            },
            HfModelInfo {
                repo_id,
                author: "mixed".to_string(),
                model_name: "repo".to_string(),
                downloads: 2,
                gguf_files: Vec::new(),
                variants: vec![mlx_variant],
            },
        ];

        dedupe_models(&mut models);

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].downloads, 2);
        assert_eq!(models[0].gguf_files.len(), 1);
        assert_eq!(models[0].variants.len(), 2);
        assert!(models[0]
            .variants
            .iter()
            .any(|variant| variant.backend_id == LLAMACPP_BACKEND_ID));
        assert!(models[0]
            .variants
            .iter()
            .any(|variant| variant.backend_id == MLX_BACKEND_ID));
    }

    fn sibling(filename: &str) -> RepoSibling {
        RepoSibling {
            rfilename: filename.to_string(),
            size: Some(1),
            lfs: None,
        }
    }

    fn mlx_siblings(tokenizer_files: &[&str]) -> Vec<RepoSibling> {
        let mut siblings = vec![sibling("config.json"), sibling("model.safetensors")];
        siblings.extend(tokenizer_files.iter().map(|filename| sibling(filename)));
        siblings
    }

    #[test]
    fn mlx_compatible_repo_accepts_supported_tokenizer_formats() {
        let config = Some(serde_json::json!({ "model_type": "llama" }));

        for tokenizer_files in [
            vec!["tokenizer.json"],
            vec!["tokenizer.model"],
            vec!["tokenizer.tiktoken"],
            vec!["vocab.json", "merges.txt"],
        ] {
            assert!(
                is_mlx_compatible_repo(&config, &mlx_siblings(&tokenizer_files)),
                "{tokenizer_files:?}"
            );
        }
    }

    #[test]
    fn mlx_compatible_repo_rejects_incomplete_tokenizer_files() {
        let config = Some(serde_json::json!({ "model_type": "llama" }));

        for tokenizer_files in [
            vec!["tokenizer_config.json"],
            vec!["vocab.json"],
            vec!["merges.txt"],
        ] {
            assert!(
                !is_mlx_compatible_repo(&config, &mlx_siblings(&tokenizer_files)),
                "{tokenizer_files:?}"
            );
        }
    }

    fn test_model(repo_id: &str) -> HfModelInfo {
        HfModelInfo {
            repo_id: repo_id.to_string(),
            author: repo_id.split_once('/').unwrap().0.to_string(),
            model_name: repo_id.split_once('/').unwrap().1.to_string(),
            downloads: 1,
            gguf_files: Vec::new(),
            variants: Vec::new(),
        }
    }

    #[test]
    fn append_optional_mlx_results_extends_on_success() {
        let mut results = vec![test_model("gguf/repo")];

        append_optional_mlx_results(
            &mut results,
            Ok(vec![test_model("mlx/repo")]),
            "search-query",
        );

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|model| model.repo_id == "gguf/repo"));
        assert!(results.iter().any(|model| model.repo_id == "mlx/repo"));
    }

    #[test]
    fn append_optional_mlx_results_preserves_existing_on_error() {
        let mut results = vec![test_model("gguf/repo")];

        append_optional_mlx_results(
            &mut results,
            Err(anyhow::anyhow!("hf-hub unavailable")),
            "search-query",
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].repo_id, "gguf/repo");
    }

    #[test]
    fn test_recommend_variant() {
        let variants = vec![
            HfQuantVariant {
                quantization: "Q2_K".into(),
                size_bytes: 2_000_000_000,
                filename: "m-Q2_K.gguf".into(),
                download_url: String::new(),
                description: "Small",
                quality_rank: 24,
                sharded: false,
            },
            HfQuantVariant {
                quantization: "Q4_K_M".into(),
                size_bytes: 4_000_000_000,
                filename: "m-Q4_K_M.gguf".into(),
                download_url: String::new(),
                description: "Medium",
                quality_rank: 45,
                sharded: false,
            },
            HfQuantVariant {
                quantization: "Q8_0".into(),
                size_bytes: 8_000_000_000,
                filename: "m-Q8_0.gguf".into(),
                download_url: String::new(),
                description: "Large",
                quality_rank: 80,
                sharded: false,
            },
        ];

        assert_eq!(recommend_variant(&variants, 5_000_000_000), Some(1));
        assert_eq!(recommend_variant(&variants, 10_000_000_000), Some(2));
        assert_eq!(recommend_variant(&variants, 1_000_000_000), None);
    }

    #[test]
    fn test_model_stem_from_repo() {
        assert_eq!(
            model_stem_from_repo("unsloth/gemma-4-26B-A4B-it-GGUF"),
            "gemma-4-26b-a4b-it"
        );
        assert_eq!(
            model_stem_from_repo("bartowski/Llama-3.2-3B-Instruct-GGUF"),
            "llama-3.2-3b-instruct"
        );
        assert_eq!(model_stem_from_repo("someone/SomeModel"), "somemodel");
    }

    #[test]
    fn hf_download_progress_init_preserves_cancelled_reservation() {
        let model_id = "test-cancelled-hf-progress-init";
        let download_id = format!("{}-model", model_id);
        let manager = crate::download_manager::get_download_manager();
        manager.set_progress(crate::download_manager::DownloadProgress {
            model_id: download_id.clone(),
            status: crate::download_manager::DownloadStatus::Cancelled,
            bytes_downloaded: 0,
            total_bytes: 0,
            progress_percent: 0.0,
            speed_bps: None,
            eta_seconds: None,
            error: None,
            task_exited: false,
        });

        HfDownloadProgress::new(model_id.to_string(), 42).init();

        let progress = manager.get_progress(&download_id).expect("progress");
        assert_eq!(
            progress.status,
            crate::download_manager::DownloadStatus::Cancelled
        );
        assert!(!progress.task_exited);

        manager.update_progress(&download_id, |progress| {
            progress.task_exited = true;
        });
        manager.clear_completed(&download_id);
    }

    #[test]
    fn test_is_model_file() {
        let stem = "gemma-3-27b-it";
        assert!(is_model_file("gemma-3-27b-it-Q4_K_M.gguf", stem));
        assert!(is_model_file(
            "BF16/gemma-3-27b-it-BF16-00001-of-00002.gguf",
            stem
        ));
        assert!(!is_model_file("mmproj-BF16.gguf", stem));
        assert!(!is_model_file("vision-encoder-Q4_K_M.gguf", stem));
    }

    #[test]
    fn test_group_into_variants_filters_auxiliary_files() {
        let files = vec![
            HfApiSibling {
                rfilename: "gemma-3-27b-it-Q4_K_M.gguf".into(),
                size: Some(4_000_000_000),
            },
            HfApiSibling {
                rfilename: "mmproj-BF16.gguf".into(),
                size: Some(800_000_000),
            },
        ];
        let variants = group_into_variants("unsloth/gemma-3-27b-it-GGUF", files);
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].quantization, "Q4_K_M");
    }

    #[test]
    fn test_group_into_variants_includes_shard_only_quants() {
        let files = vec![
            HfApiSibling {
                rfilename: "BF16/gemma-3-27b-it-BF16-00001-of-00002.gguf".into(),
                size: Some(40_000_000_000),
            },
            HfApiSibling {
                rfilename: "BF16/gemma-3-27b-it-BF16-00002-of-00002.gguf".into(),
                size: Some(10_000_000_000),
            },
            HfApiSibling {
                rfilename: "gemma-3-27b-it-Q4_K_M.gguf".into(),
                size: Some(4_000_000_000),
            },
        ];
        let variants = group_into_variants("unsloth/gemma-3-27b-it-GGUF", files);
        assert_eq!(variants.len(), 2);
        // Sorted descending by quality_rank: BF16 (91) > Q4_K_M (45)
        assert_eq!(variants[0].quantization, "BF16");
        assert!(variants[0].sharded);
        assert_eq!(variants[0].size_bytes, 50_000_000_000);
        assert_eq!(variants[1].quantization, "Q4_K_M");
        assert!(!variants[1].sharded);
    }

    #[test]
    fn test_group_into_variants_sorted_descending() {
        let files = vec![
            HfApiSibling {
                rfilename: "Model-IQ1_S.gguf".into(),
                size: Some(500_000_000),
            },
            HfApiSibling {
                rfilename: "Model-Q4_K_M.gguf".into(),
                size: Some(4_000_000_000),
            },
            HfApiSibling {
                rfilename: "Model-Q8_0.gguf".into(),
                size: Some(8_000_000_000),
            },
        ];
        let variants = group_into_variants("someone/Model-GGUF", files);
        assert_eq!(variants.len(), 3);
        assert_eq!(variants[0].quantization, "Q8_0");
        assert_eq!(variants[1].quantization, "Q4_K_M");
        assert_eq!(variants[2].quantization, "IQ1_S");
    }

    #[test]
    fn test_select_best_mmproj_prefers_closest_precision() {
        let files = vec![
            HfApiSibling {
                rfilename: "mmproj-F32.gguf".into(),
                size: Some(3_000),
            },
            HfApiSibling {
                rfilename: "mmproj-BF16.gguf".into(),
                size: Some(2_000),
            },
        ];

        let mmproj =
            select_best_mmproj("someone/model-GGUF", &files, "model-Q4_K_M.gguf", "Q4_K_M")
                .unwrap();

        assert_eq!(mmproj.filename, "mmproj-BF16.gguf");
        assert_eq!(mmproj.quantization, "BF16");
    }

    #[tokio::test]
    async fn optional_hf_token_returns_resolved_token() {
        let token = optional_hf_token(async { Ok(Some("token".to_string())) }).await;

        assert_eq!(token.as_deref(), Some("token"));
    }

    #[tokio::test]
    async fn optional_hf_token_ignores_resolution_errors() {
        let token =
            optional_hf_token(async { Err(anyhow::anyhow!("refresh token revoked")) }).await;

        assert_eq!(token, None);
    }

    #[test]
    fn test_select_best_mmproj_prefers_bf16_over_f16_tie() {
        let files = vec![
            HfApiSibling {
                rfilename: "mmproj-F16.gguf".into(),
                size: Some(2_000),
            },
            HfApiSibling {
                rfilename: "mmproj-BF16.gguf".into(),
                size: Some(2_000),
            },
        ];

        let mmproj =
            select_best_mmproj("someone/model-GGUF", &files, "model-Q8_0.gguf", "Q8_0").unwrap();

        assert_eq!(mmproj.filename, "mmproj-BF16.gguf");
    }

    #[test]
    fn test_select_best_mmproj_prefers_nearest_directory() {
        let files = vec![
            HfApiSibling {
                rfilename: "mmproj-BF16.gguf".into(),
                size: Some(2_000),
            },
            HfApiSibling {
                rfilename: "Q4_K_M/mmproj-F32.gguf".into(),
                size: Some(3_000),
            },
        ];

        let mmproj = select_best_mmproj(
            "someone/model-GGUF",
            &files,
            "Q4_K_M/model-Q4_K_M.gguf",
            "Q4_K_M",
        )
        .unwrap();

        assert_eq!(mmproj.filename, "Q4_K_M/mmproj-F32.gguf");
    }

    #[test]
    fn test_select_best_mmproj_ignores_sibling_directories() {
        let files = vec![
            HfApiSibling {
                rfilename: "Q8_0/mmproj-BF16.gguf".into(),
                size: Some(2_000),
            },
            HfApiSibling {
                rfilename: "mmproj-F32.gguf".into(),
                size: Some(3_000),
            },
        ];

        let mmproj = select_best_mmproj(
            "someone/model-GGUF",
            &files,
            "Q4_K_M/model-Q4_K_M.gguf",
            "Q4_K_M",
        )
        .unwrap();

        assert_eq!(mmproj.filename, "mmproj-F32.gguf");
    }
}

async fn hf_client() -> Result<HFClient> {
    let mut builder = HFClient::builder().user_agent("goose-ai-agent");
    if let Some(token) = optional_hf_token(huggingface_auth::resolve_token_async()).await {
        builder = builder.token(token);
    }
    builder.build().map_err(Into::into)
}

fn model_repo(client: &HFClient, repo_id: &str) -> Result<HFRepository<RepoTypeModel>> {
    let (owner, name) = split_repo_id(repo_id)?;
    Ok(client.model(owner, name))
}

fn split_repo_id(repo_id: &str) -> Result<(&str, &str)> {
    repo_id
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid repo id '{}': expected owner/name", repo_id))
}

async fn search_mlx_models(query: &str, limit: usize) -> Result<Vec<HfModelInfo>> {
    let mut results = search_mlx_models_with_query(query, limit).await?;
    if !query.contains('/') {
        results
            .extend(search_mlx_models_with_query(&format!("mlx-community/{query}"), limit).await?);
        results.extend(search_mlx_models_with_query(&format!("google/{query}"), limit).await?);
    }
    dedupe_models(&mut results);
    results.truncate(limit);
    Ok(results)
}

async fn search_mlx_models_with_query(query: &str, limit: usize) -> Result<Vec<HfModelInfo>> {
    let client = hf_client().await?;
    let stream = client
        .list_models()
        .search(query.to_string())
        .sort("downloads".to_string())
        .limit(limit.saturating_mul(5).max(limit))
        .send()?;
    futures::pin_mut!(stream);

    let mut results = Vec::new();
    while let Some(info) = stream.next().await {
        let info = info?;
        if results.len() >= limit {
            break;
        }
        if let Some(model) = get_local_model_info_for_repo_with_client(&client, &info.id).await? {
            results.push(model);
        }
    }

    Ok(results)
}

async fn get_local_model_info_for_repo(repo_id: &str) -> Result<Option<HfModelInfo>> {
    let client = hf_client().await?;
    get_local_model_info_for_repo_with_client(&client, repo_id).await
}

async fn get_local_model_info_for_repo_with_client(
    client: &HFClient,
    repo_id: &str,
) -> Result<Option<HfModelInfo>> {
    let repo = model_repo(client, repo_id)?;
    let info = repo
        .info()
        .expand(vec![
            "siblings".to_string(),
            "config".to_string(),
            "safetensors".to_string(),
        ])
        .send()
        .await?;
    model_info_to_local_model_info(info).await
}

async fn get_exact_name_local_model_info(model_name: &str) -> Result<Option<HfModelInfo>> {
    for owner in ["google", "mlx-community"] {
        let repo_id = format!("{owner}/{model_name}");
        if let Ok(Some(model)) = get_local_model_info_for_repo(&repo_id).await {
            return Ok(Some(model));
        }
    }
    Ok(None)
}

async fn model_info_to_local_model_info(info: ModelInfo) -> Result<Option<HfModelInfo>> {
    let repo_id = info.id.clone();
    let mut variants: Vec<HfModelVariant> = get_repo_gguf_variants(&repo_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|variant| variant.to_model_variant(&repo_id))
        .collect();
    variants.extend(mlx_variants_from_model_info(&repo_id, &info));

    if variants.is_empty() {
        return Ok(None);
    }

    let author = info
        .author
        .unwrap_or_else(|| repo_id.split('/').next().unwrap_or_default().to_string());
    let model_name = repo_id
        .split('/')
        .next_back()
        .unwrap_or(&repo_id)
        .to_string();

    Ok(Some(HfModelInfo {
        repo_id,
        author,
        model_name,
        downloads: info.downloads.unwrap_or(0),
        gguf_files: Vec::new(),
        variants,
    }))
}

pub async fn get_repo_local_variants(repo_id: &str) -> Result<Vec<HfModelVariant>> {
    let mut variants: Vec<HfModelVariant> = get_repo_gguf_variants(repo_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|variant| variant.to_model_variant(repo_id))
        .collect();
    variants.extend(get_repo_mlx_variants(repo_id).await.unwrap_or_default());
    variants.sort_by(|a, b| {
        a.backend_id
            .cmp(&b.backend_id)
            .then_with(|| b.quality_rank.cmp(&a.quality_rank))
            .then_with(|| a.variant_id.cmp(&b.variant_id))
    });
    Ok(variants)
}

pub async fn get_repo_mlx_variants(repo_id: &str) -> Result<Vec<HfModelVariant>> {
    let client = hf_client().await?;
    let repo = model_repo(&client, repo_id)?;
    let info = repo
        .info()
        .expand(vec![
            "siblings".to_string(),
            "config".to_string(),
            "safetensors".to_string(),
        ])
        .send()
        .await?;
    Ok(mlx_variants_from_model_info(repo_id, &info))
}

fn mlx_variants_from_model_info(repo_id: &str, info: &ModelInfo) -> Vec<HfModelVariant> {
    let siblings = info.siblings.as_deref().unwrap_or(&[]);

    if !is_mlx_compatible_repo(&info.config, siblings) {
        return Vec::new();
    }

    let size_bytes = mlx_download_filenames(siblings)
        .into_iter()
        .filter_map(|filename| {
            siblings
                .iter()
                .find(|s| s.rfilename == filename)
                .and_then(|s| s.size)
        })
        .sum();
    let variant_id = mlx_variant_id(repo_id, &info.config);

    vec![HfModelVariant {
        variant_id: variant_id.clone(),
        label: mlx_variant_label(&variant_id),
        backend_id: MLX_BACKEND_ID.to_string(),
        format: MLX_FORMAT.to_string(),
        model_id: repo_id.to_string(),
        download_id: repo_id.to_string(),
        size_bytes,
        filename: None,
        download_url: None,
        description: mlx_variant_description(&info.config),
        quality_rank: 91,
        sharded: siblings
            .iter()
            .filter(|s| s.rfilename.ends_with(".safetensors"))
            .count()
            > 1,
        supported: is_mlx_runtime_supported(&info.config)
            && cfg!(target_os = "macos")
            && cfg!(feature = "mlx"),
        unsupported_reason: mlx_unsupported_reason(&info.config),
    }]
}

fn is_mlx_compatible_repo(config: &Option<serde_json::Value>, siblings: &[RepoSibling]) -> bool {
    let has_config = siblings.iter().any(|s| s.rfilename == "config.json");
    let has_tokenizer = has_mlx_tokenizer(siblings);
    let has_safetensors = siblings
        .iter()
        .any(|s| s.rfilename.ends_with(".safetensors"));

    has_config && has_tokenizer && has_safetensors && mlx_model_type(config).is_some()
}

fn has_mlx_tokenizer(siblings: &[RepoSibling]) -> bool {
    let has_file = |filename: &str| siblings.iter().any(|s| s.rfilename == filename);
    siblings
        .iter()
        .any(|s| is_standalone_mlx_tokenizer_file(&s.rfilename))
        || (has_file("vocab.json") && has_file("merges.txt"))
}

fn is_standalone_mlx_tokenizer_file(filename: &str) -> bool {
    filename == "tokenizer.json" || filename.ends_with(".model") || filename.ends_with(".tiktoken")
}

fn mlx_model_type(config: &Option<serde_json::Value>) -> Option<&str> {
    config
        .as_ref()
        .and_then(|config| config.get("model_type"))
        .and_then(|value| value.as_str())
}

fn is_mlx_runtime_supported_model_type(model_type: &str) -> bool {
    matches!(model_type, "gemma4" | "gemma4_text" | "llama" | "qwen3")
}

fn is_mlx_moe_model(config: &Option<serde_json::Value>) -> bool {
    config
        .as_ref()
        .and_then(|config| config.get("text_config"))
        .and_then(|text_config| text_config.get("enable_moe_block"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn is_mlx_runtime_supported(config: &Option<serde_json::Value>) -> bool {
    mlx_model_type(config).is_some_and(is_mlx_runtime_supported_model_type)
        && !is_mlx_moe_model(config)
}

fn mlx_unsupported_reason(config: &Option<serde_json::Value>) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return Some("MLX requires macOS".to_string());
    }
    if !cfg!(feature = "mlx") {
        return Some("MLX support was not compiled in".to_string());
    }

    let model_type = mlx_model_type(config)?;
    if !is_mlx_runtime_supported_model_type(model_type) {
        return Some(format!(
            "MLX backend does not support '{}' models yet",
            model_type
        ));
    }
    if is_mlx_moe_model(config) {
        return Some("MLX backend does not support Gemma 4 MoE models yet".to_string());
    }
    None
}

fn mlx_variant_description(config: &Option<serde_json::Value>) -> String {
    match mlx_unsupported_reason(config) {
        None => "MLX safetensors snapshot".to_string(),
        Some(reason) => format!("MLX safetensors snapshot ({reason})"),
    }
}

fn mlx_download_filenames(siblings: &[RepoSibling]) -> Vec<String> {
    siblings
        .iter()
        .filter(|s| should_download_for_mlx(&s.rfilename))
        .map(|s| s.rfilename.clone())
        .collect()
}

fn looks_like_repo_id(query: &str) -> bool {
    let Some((owner, repo)) = query.split_once('/') else {
        return false;
    };
    !owner.is_empty() && !repo.is_empty() && !repo.contains('/')
}

fn model_search_rank(query: &str, model: &HfModelInfo) -> u8 {
    let query = query.to_lowercase();
    let repo_id = model.repo_id.to_lowercase();
    let model_name = model.model_name.to_lowercase();

    if repo_id == query {
        0
    } else if model_name == query {
        1
    } else if repo_id.ends_with(&format!("/{query}")) {
        2
    } else if repo_id.contains(&query) {
        3
    } else {
        4
    }
}

fn dedupe_models(models: &mut Vec<HfModelInfo>) {
    let mut merged: Vec<HfModelInfo> = Vec::with_capacity(models.len());
    for model in std::mem::take(models) {
        if let Some(existing) = merged
            .iter_mut()
            .find(|existing| existing.repo_id == model.repo_id)
        {
            merge_model_info(existing, model);
        } else {
            merged.push(model);
        }
    }
    *models = merged;
}

fn merge_model_info(existing: &mut HfModelInfo, duplicate: HfModelInfo) {
    existing.downloads = existing.downloads.max(duplicate.downloads);

    let mut filenames: std::collections::HashSet<String> = existing
        .gguf_files
        .iter()
        .map(|file| file.filename.clone())
        .collect();
    existing.gguf_files.extend(
        duplicate
            .gguf_files
            .into_iter()
            .filter(|file| filenames.insert(file.filename.clone())),
    );

    let mut variant_keys: std::collections::HashSet<(String, String)> = existing
        .variants
        .iter()
        .map(|variant| (variant.backend_id.clone(), variant.variant_id.clone()))
        .collect();
    existing
        .variants
        .extend(duplicate.variants.into_iter().filter(|variant| {
            variant_keys.insert((variant.backend_id.clone(), variant.variant_id.clone()))
        }));
}

fn should_download_for_mlx(filename: &str) -> bool {
    filename.ends_with(".safetensors")
        || filename == "config.json"
        || is_standalone_mlx_tokenizer_file(filename)
        || filename == "tokenizer_config.json"
        || filename == "generation_config.json"
        || filename == "special_tokens_map.json"
        || filename == "model.safetensors.index.json"
        || filename == "vocab.json"
        || filename == "merges.txt"
        || filename == "added_tokens.json"
}

fn mlx_variant_id(repo_id: &str, config: &Option<serde_json::Value>) -> String {
    let repo_lower = repo_id.to_lowercase();
    for marker in ["bf16", "f16", "fp16", "f32", "fp32", "4bit", "8bit"] {
        if repo_lower.contains(marker) {
            return marker.to_string();
        }
    }
    config
        .as_ref()
        .and_then(|config| config.get("torch_dtype"))
        .and_then(|value| value.as_str())
        .map(|dtype| dtype.replace("float", "f"))
        .unwrap_or_else(|| MLX_VARIANT_ID.to_string())
}

fn mlx_variant_label(variant_id: &str) -> String {
    if variant_id == MLX_VARIANT_ID {
        "MLX".to_string()
    } else {
        format!("MLX {}", variant_id.to_uppercase())
    }
}

pub async fn resolve_local_model_selection(
    repo_id: &str,
    backend_id: &str,
    variant_id: Option<&str>,
) -> Result<ResolvedLocalModel> {
    match backend_id {
        MLX_BACKEND_ID => resolve_mlx_model(repo_id, variant_id.unwrap_or(MLX_VARIANT_ID)).await,
        LLAMACPP_BACKEND_ID => {
            let quantization = variant_id.ok_or_else(|| {
                anyhow::anyhow!("llama.cpp model '{}' is missing a quantization", repo_id)
            })?;
            resolve_gguf_model(repo_id, quantization).await
        }
        _ => bail!("Unknown local inference backend '{}'", backend_id),
    }
}

fn snapshot_root_for_file(
    path: &std::path::Path,
    repo_filename: &str,
) -> Option<std::path::PathBuf> {
    let mut root = path.to_path_buf();
    for _ in 0..repo_filename.split('/').count() {
        root.pop();
    }
    Some(root)
}

async fn resolve_gguf_model(repo_id: &str, quantization: &str) -> Result<ResolvedLocalModel> {
    let spec = format!("{}:{}", repo_id, quantization);
    let (_repo, resolved) = resolve_model_spec_full(&spec).await?;
    let (local_paths, mmproj_path) =
        download_gguf_to_hf_cache(repo_id, quantization, &resolved).await?;
    Ok(ResolvedLocalModel::Gguf {
        repo_id: repo_id.to_string(),
        quantization: quantization.to_string(),
        resolved,
        local_paths,
        mmproj_path,
        storage: LocalModelStorage::HuggingFaceCache,
    })
}

async fn download_gguf_to_hf_cache(
    repo_id: &str,
    quantization: &str,
    resolved: &ResolvedModel,
) -> Result<(Vec<std::path::PathBuf>, Option<std::path::PathBuf>)> {
    let (owner, name) = split_repo_id(repo_id)?;
    let model_id = model_id_from_repo(repo_id, quantization);
    let total_size = resolved
        .files
        .iter()
        .chain(resolved.mmproj.iter())
        .map(|file| file.size_bytes)
        .sum();
    let progress = HfDownloadProgress::new(model_id, total_size);
    progress.init();
    let client = hf_client().await?;
    let repo = client.model(owner.to_string(), name.to_string());
    let mut paths = Vec::with_capacity(resolved.files.len());
    for file in &resolved.files {
        let path = match repo
            .download_file()
            .filename(file.filename.clone())
            .progress(progress.clone())
            .send()
            .await
            .map_err(anyhow::Error::from)
        {
            Ok(path) => path,
            Err(error) => {
                progress.fail(&error);
                return Err(error);
            }
        };
        progress.finish_file(file.size_bytes);
        paths.push(path);
    }

    let mmproj_path = if let Some(mmproj) = &resolved.mmproj {
        let path = match repo
            .download_file()
            .filename(mmproj.filename.clone())
            .progress(progress.clone())
            .send()
            .await
            .map_err(anyhow::Error::from)
        {
            Ok(path) => path,
            Err(error) => {
                progress.fail(&error);
                return Err(error);
            }
        };
        progress.finish_file(mmproj.size_bytes);
        Some(path)
    } else {
        None
    };

    progress.complete();
    Ok((paths, mmproj_path))
}

pub async fn resolve_local_model_spec(spec: &str) -> Result<ResolvedLocalModel> {
    match parse_model_spec(spec) {
        Ok((repo_id, quantization)) => return resolve_gguf_model(&repo_id, &quantization).await,
        Err(error) if spec.contains(':') => return Err(error),
        Err(_) => {}
    }

    if looks_like_repo_id(spec) {
        let variants = get_repo_local_variants(spec).await?;
        let mlx_variants: Vec<_> = variants
            .iter()
            .filter(|variant| variant.backend_id == MLX_BACKEND_ID)
            .collect();
        if mlx_variants.len() == 1
            && !variants
                .iter()
                .any(|variant| variant.backend_id == LLAMACPP_BACKEND_ID)
        {
            return resolve_mlx_model(spec, &mlx_variants[0].variant_id).await;
        }
        bail!(
            "Model spec '{}' is ambiguous; choose one of: {}",
            spec,
            variants
                .iter()
                .map(|variant| variant.download_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let (repo_id, quantization) = parse_model_spec(spec)?;
    resolve_gguf_model(&repo_id, &quantization).await
}

async fn resolve_mlx_model(repo_id: &str, variant_id: &str) -> Result<ResolvedLocalModel> {
    let variants = get_repo_mlx_variants(repo_id).await?;
    if !variants
        .iter()
        .any(|variant| variant.variant_id == variant_id)
    {
        bail!("No MLX variant '{}' found in {}", variant_id, repo_id);
    }
    let (owner, name) = split_repo_id(repo_id)?;
    let client = hf_client().await?;
    let repo = client.model(owner.to_string(), name.to_string());
    let info = repo
        .info()
        .expand(vec!["siblings".to_string()])
        .send()
        .await?;
    let siblings = info.siblings.as_deref().unwrap_or(&[]);
    let filenames = mlx_download_filenames(siblings);
    let total_size = filenames
        .iter()
        .filter_map(|filename| {
            siblings
                .iter()
                .find(|s| s.rfilename == *filename)
                .and_then(|s| s.size)
        })
        .sum();
    let progress = HfDownloadProgress::new(repo_id.to_string(), total_size);
    progress.init();
    let mut snapshot_path = None;
    for filename in filenames {
        let file_size = siblings
            .iter()
            .find(|s| s.rfilename == filename)
            .and_then(|s| s.size)
            .unwrap_or(0);
        let path = match repo
            .download_file()
            .filename(filename.clone())
            .progress(progress.clone())
            .send()
            .await
            .map_err(anyhow::Error::from)
        {
            Ok(path) => path,
            Err(error) => {
                progress.fail(&error);
                return Err(error);
            }
        };
        if snapshot_path.is_none() {
            snapshot_path = snapshot_root_for_file(&path, &filename);
        }
        progress.finish_file(file_size);
    }
    progress.complete();
    let snapshot_path = snapshot_path
        .ok_or_else(|| anyhow::anyhow!("MLX model {} has no downloadable files", repo_id))?;
    let total_size = if total_size > 0 {
        total_size
    } else {
        dir_size(&snapshot_path)
    };
    Ok(ResolvedLocalModel::Mlx {
        repo_id: repo_id.to_string(),
        variant_id: variant_id.to_string(),
        snapshot_path,
        total_size,
    })
}

#[derive(Clone)]
struct HfDownloadProgress {
    model_id: String,
    total_bytes: u64,
    completed_bytes: Arc<Mutex<u64>>,
    state: Arc<Mutex<HfDownloadState>>,
}

#[derive(Default)]
struct HfDownloadState {
    bytes_downloaded: u64,
    current_file_total_bytes: u64,
    speed_bps: Option<u64>,
}

impl HfDownloadProgress {
    fn new(model_id: String, total_bytes: u64) -> Self {
        Self {
            model_id,
            total_bytes,
            completed_bytes: Arc::new(Mutex::new(0)),
            state: Arc::new(Mutex::new(HfDownloadState::default())),
        }
    }

    fn init(&self) {
        let manager = crate::download_manager::get_download_manager();
        let download_id = format!("{}-model", self.model_id);
        if manager.get_progress(&download_id).is_some() {
            manager.update_progress(&download_id, |progress| {
                if progress.status != crate::download_manager::DownloadStatus::Cancelled {
                    progress.status = crate::download_manager::DownloadStatus::Downloading;
                    progress.bytes_downloaded = 0;
                    progress.total_bytes = self.total_bytes;
                    progress.progress_percent = 0.0;
                    progress.speed_bps = None;
                    progress.eta_seconds = None;
                    progress.error = None;
                    progress.task_exited = false;
                }
            });
        } else {
            manager.set_progress(crate::download_manager::DownloadProgress {
                model_id: download_id,
                status: crate::download_manager::DownloadStatus::Downloading,
                bytes_downloaded: 0,
                total_bytes: self.total_bytes,
                progress_percent: 0.0,
                speed_bps: None,
                eta_seconds: None,
                error: None,
                task_exited: false,
            });
        }
    }

    fn is_cancelled(&self) -> bool {
        crate::download_manager::get_download_manager()
            .get_progress(&format!("{}-model", self.model_id))
            .is_some_and(|progress| {
                progress.status == crate::download_manager::DownloadStatus::Cancelled
            })
    }

    fn complete(&self) {
        crate::download_manager::get_download_manager().update_progress(
            &format!("{}-model", self.model_id),
            |progress| {
                if progress.status != crate::download_manager::DownloadStatus::Cancelled {
                    progress.status = crate::download_manager::DownloadStatus::Completed;
                    progress.progress_percent = 100.0;
                }
                progress.task_exited = true;
            },
        );
    }

    fn fail(&self, error: impl ToString) {
        crate::download_manager::get_download_manager().update_progress(
            &format!("{}-model", self.model_id),
            |progress| {
                if progress.status != crate::download_manager::DownloadStatus::Cancelled {
                    progress.status = crate::download_manager::DownloadStatus::Failed;
                    progress.error = Some(error.to_string());
                }
                progress.task_exited = true;
            },
        );
    }

    fn finish_file(&self, size_bytes: u64) {
        if let Ok(mut completed_bytes) = self.completed_bytes.lock() {
            *completed_bytes = completed_bytes.saturating_add(size_bytes);
        }
        if let Ok(mut state) = self.state.lock() {
            state.bytes_downloaded = 0;
            state.current_file_total_bytes = 0;
        }
        self.update_progress_from_state();
    }

    fn update_progress_from_state(&self) {
        let completed_bytes = self.completed_bytes.lock().map(|value| *value).unwrap_or(0);
        if let Ok(state) = self.state.lock() {
            let bytes_downloaded = completed_bytes.saturating_add(state.bytes_downloaded);
            update_download_manager_progress(
                &self.model_id,
                bytes_downloaded.min(self.total_bytes),
                self.total_bytes.max(state.current_file_total_bytes),
                state.speed_bps,
            );
        }
    }
}

impl ProgressHandler for HfDownloadProgress {
    fn on_progress(&self, event: &ProgressEvent) {
        let ProgressEvent::Download(event) = event else {
            return;
        };
        match event {
            DownloadEvent::Start { total_bytes, .. } => {
                if let Ok(mut state) = self.state.lock() {
                    state.current_file_total_bytes = *total_bytes;
                }
                self.update_progress_from_state();
            }
            DownloadEvent::Progress { files } => {
                if self.is_cancelled() {
                    return;
                }
                let bytes_downloaded = files
                    .iter()
                    .map(|file| {
                        if file.status == FileStatus::Complete {
                            file.total_bytes
                        } else {
                            file.bytes_completed
                        }
                    })
                    .max()
                    .unwrap_or(0);
                if let Ok(mut state) = self.state.lock() {
                    state.bytes_downloaded = state.bytes_downloaded.max(bytes_downloaded);
                }
                self.update_progress_from_state();
            }
            DownloadEvent::AggregateProgress {
                bytes_completed,
                total_bytes,
                bytes_per_sec,
            } => {
                if self.is_cancelled() {
                    return;
                }
                if let Ok(mut state) = self.state.lock() {
                    state.bytes_downloaded = *bytes_completed;
                    state.current_file_total_bytes =
                        (*total_bytes).max(state.current_file_total_bytes);
                    state.speed_bps = bytes_per_sec.map(|speed| speed as u64);
                }
                self.update_progress_from_state();
            }
            DownloadEvent::Complete => {}
        }
    }
}

pub fn register_resolved_model(resolved: ResolvedLocalModel, source: &str) -> Result<String> {
    let model_id = resolved.model_id();
    let repo_id = resolved.repo_id().to_string();
    let variant_id = resolved.variant_id().to_string();
    let backend_id = resolved.backend_id().to_string();
    let storage = resolved.storage();

    let entry = match resolved {
        ResolvedLocalModel::Gguf {
            resolved,
            local_paths,
            mmproj_path,
            ..
        } => {
            let first_file = &resolved.files[0];
            let first_local_path = local_paths
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Resolved GGUF model has no local files"))?;
            let shard_files: Vec<ShardFile> = resolved
                .files
                .iter()
                .skip(1)
                .zip(local_paths.iter().skip(1))
                .map(|(file, local_path)| ShardFile {
                    filename: file.filename.clone(),
                    local_path: local_path.clone(),
                    source_url: file.download_url.clone(),
                    size_bytes: file.size_bytes,
                })
                .collect();
            let settings = super::local_model_registry::default_settings_for_model(&model_id);
            super::local_model_registry::LocalModelEntry {
                id: model_id.clone(),
                repo_id,
                filename: first_file.filename.clone(),
                quantization: variant_id,
                local_path: first_local_path,
                source_url: first_file.download_url.clone(),
                backend_id: settings.backend_id.clone(),
                storage,
                settings,
                size_bytes: resolved.total_size,
                mmproj_path,
                mmproj_source_url: resolved
                    .mmproj
                    .as_ref()
                    .map(|mmproj| mmproj.download_url.clone()),
                mmproj_size_bytes: resolved
                    .mmproj
                    .as_ref()
                    .map(|mmproj| mmproj.size_bytes)
                    .unwrap_or(0),
                mmproj_checked: true,
                shard_files,
            }
        }
        ResolvedLocalModel::Mlx {
            snapshot_path,
            total_size,
            ..
        } => {
            let mut settings = super::local_model_registry::default_settings_for_model(&model_id);
            settings.backend_id = Some(backend_id.clone());
            super::local_model_registry::LocalModelEntry {
                id: model_id.clone(),
                repo_id,
                filename: variant_id.clone(),
                quantization: variant_id,
                local_path: snapshot_path,
                source_url: source.to_string(),
                backend_id: Some(backend_id),
                storage,
                settings,
                size_bytes: total_size,
                mmproj_path: None,
                mmproj_source_url: None,
                mmproj_size_bytes: 0,
                mmproj_checked: true,
                shard_files: vec![],
            }
        }
    };

    let mut registry = get_registry()
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to acquire registry lock"))?;
    registry.add_model(entry)?;
    Ok(model_id)
}

fn update_download_manager_progress(
    model_id: &str,
    bytes_downloaded: u64,
    total_bytes: u64,
    speed_bps: Option<u64>,
) {
    crate::download_manager::get_download_manager().update_progress(
        &format!("{}-model", model_id),
        |progress| {
            if progress.status == crate::download_manager::DownloadStatus::Cancelled {
                return;
            }
            progress.bytes_downloaded = bytes_downloaded;
            progress.total_bytes = total_bytes;
            progress.progress_percent = if total_bytes > 0 {
                (bytes_downloaded as f64 / total_bytes as f64 * 100.0) as f32
            } else {
                0.0
            };
            progress.speed_bps = speed_bps;
        },
    );
}

fn dir_size(path: &std::path::Path) -> u64 {
    if path.is_file() {
        return std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            total += dir_size(&entry.path());
        }
    }
    total
}
