use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

const HF_API_BASE: &str = "https://huggingface.co/api/models";
const HF_DOWNLOAD_BASE: &str = "https://huggingface.co";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HfModelInfo {
    pub repo_id: String,
    pub author: String,
    pub model_name: String,
    pub downloads: u64,
    pub gguf_files: Vec<HfGgufFile>,
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

fn quant_info(quant: &str) -> QuantInfo {
    match quant {
        "IQ1_S" => QuantInfo {
            description: "Extremely small, very low quality",
            quality_rank: 1,
        },
        "IQ1_M" => QuantInfo {
            description: "Extremely small, very low quality",
            quality_rank: 2,
        },
        "IQ2_XXS" => QuantInfo {
            description: "Very small, low quality",
            quality_rank: 3,
        },
        "IQ2_XS" => QuantInfo {
            description: "Very small, low quality",
            quality_rank: 4,
        },
        "IQ2_S" => QuantInfo {
            description: "Very small, low quality",
            quality_rank: 5,
        },
        "IQ2_M" => QuantInfo {
            description: "Very small, low quality",
            quality_rank: 6,
        },
        "Q2_K" | "Q2_K_S" => QuantInfo {
            description: "Small, low quality",
            quality_rank: 7,
        },
        "IQ3_XXS" => QuantInfo {
            description: "Very small, moderate quality loss",
            quality_rank: 8,
        },
        "IQ3_XS" | "IQ3_S" => QuantInfo {
            description: "Small, moderate quality loss",
            quality_rank: 9,
        },
        "Q3_K_S" => QuantInfo {
            description: "Small, moderate quality loss",
            quality_rank: 10,
        },
        "IQ3_M" => QuantInfo {
            description: "Small, moderate quality loss",
            quality_rank: 11,
        },
        "Q3_K_M" => QuantInfo {
            description: "Small, balanced quality/size",
            quality_rank: 12,
        },
        "Q3_K_L" => QuantInfo {
            description: "Medium-small, decent quality",
            quality_rank: 13,
        },
        "IQ4_XS" => QuantInfo {
            description: "Medium, good quality",
            quality_rank: 14,
        },
        "IQ4_NL" => QuantInfo {
            description: "Medium, good quality",
            quality_rank: 15,
        },
        "Q4_0" => QuantInfo {
            description: "Medium, good quality",
            quality_rank: 16,
        },
        "Q4_1" => QuantInfo {
            description: "Medium, good quality",
            quality_rank: 17,
        },
        "Q4_K_S" => QuantInfo {
            description: "Medium, good quality/size balance",
            quality_rank: 18,
        },
        "Q4_K_M" => QuantInfo {
            description: "Medium, recommended balance of quality and size",
            quality_rank: 19,
        },
        "Q5_0" => QuantInfo {
            description: "Medium-large, high quality",
            quality_rank: 20,
        },
        "Q5_1" => QuantInfo {
            description: "Medium-large, high quality",
            quality_rank: 21,
        },
        "Q5_K_S" => QuantInfo {
            description: "Medium-large, high quality",
            quality_rank: 22,
        },
        "Q5_K_M" => QuantInfo {
            description: "Medium-large, very high quality",
            quality_rank: 23,
        },
        "Q6_K" => QuantInfo {
            description: "Large, near-lossless quality",
            quality_rank: 24,
        },
        "Q8_0" => QuantInfo {
            description: "Large, near-lossless quality",
            quality_rank: 25,
        },
        "F16" => QuantInfo {
            description: "Full size, original quality (16-bit)",
            quality_rank: 26,
        },
        "BF16" => QuantInfo {
            description: "Full size, original quality (bfloat16)",
            quality_rank: 27,
        },
        "F32" => QuantInfo {
            description: "Full size, original quality (32-bit)",
            quality_rank: 28,
        },
        "MXFP4_MOE" => QuantInfo {
            description: "Medium, mixed-precision 4-bit for MoE models",
            quality_rank: 18,
        },
        "TQ1_0" => QuantInfo {
            description: "Tiny, ternary quantization",
            quality_rank: 1,
        },
        "Q2_K_XL" | "Q3_K_XL" | "Q4_K_XL" => QuantInfo {
            description: "Extended-layer variant",
            quality_rank: 15,
        },
        "Q2_K_L" => QuantInfo {
            description: "Small, low quality (large variant)",
            quality_rank: 8,
        },
        "Q4_K_L" => QuantInfo {
            description: "Medium, good quality (large variant)",
            quality_rank: 20,
        },
        _ => QuantInfo {
            description: "",
            quality_rank: 15,
        },
    }
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
    let basename = filename.rsplit('/').next().unwrap_or(filename);
    let stem = basename.trim_end_matches(".gguf");
    if let Some(pos) = stem.rfind("-of-") {
        stem.get(..pos)
            .and_then(|before| before.rsplit('-').next())
            .map(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false)
    } else {
        false
    }
}

fn build_download_url(repo_id: &str, filename: &str) -> String {
    format!("{}/{}/resolve/main/{}", HF_DOWNLOAD_BASE, repo_id, filename)
}

/// Collect single-file GGUFs into quantization variants (sharded files are excluded).
fn group_into_variants(repo_id: &str, files: Vec<HfApiSibling>) -> Vec<HfQuantVariant> {
    let mut variants: Vec<HfQuantVariant> = files
        .into_iter()
        .filter(|s| {
            s.rfilename.ends_with(".gguf")
                && !is_shard_file(&s.rfilename)
                && parse_quantization(&s.rfilename) != "unknown"
        })
        .map(|s| {
            let quant = parse_quantization(&s.rfilename);
            let info = quant_info(&quant);
            let download_url = build_download_url(repo_id, &s.rfilename);
            HfQuantVariant {
                quantization: quant,
                size_bytes: s.size.unwrap_or(0),
                filename: s.rfilename,
                download_url,
                description: info.description,
                quality_rank: info.quality_rank,
            }
        })
        .collect();

    variants.sort_by_key(|v| v.quality_rank);
    variants
}

pub async fn search_gguf_models(query: &str, limit: usize) -> Result<Vec<HfModelInfo>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}?search={}&filter=gguf&sort=downloads&direction=-1&limit={}",
        HF_API_BASE, query, limit
    );

    let response = client
        .get(&url)
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
            })
        })
        .collect();

    Ok(results)
}

/// Fetch GGUF files for a repo and return them grouped by quantization.
pub async fn get_repo_gguf_variants(repo_id: &str) -> Result<Vec<HfQuantVariant>> {
    let client = reqwest::Client::new();
    let url = format!("{}/{}?blobs=true", HF_API_BASE, repo_id);

    let response = client
        .get(&url)
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
    let url = format!("{}/{}?blobs=true", HF_API_BASE, repo_id);

    let response = client
        .get(&url)
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

    let files = siblings
        .into_iter()
        .filter(|s| s.rfilename.ends_with(".gguf"))
        .filter(|s| !is_shard_file(&s.rfilename))
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

    Ok((repo_id.to_string(), quant.to_string()))
}

/// Resolve a model spec to a specific GGUF file from the repo.
pub async fn resolve_model_spec(spec: &str) -> Result<(String, HfGgufFile)> {
    let (repo_id, quant) = parse_model_spec(spec)?;
    let files = get_repo_gguf_files(&repo_id).await?;

    let file = files
        .into_iter()
        .find(|f| f.quantization.eq_ignore_ascii_case(&quant))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No GGUF file with quantization '{}' found in {}",
                quant,
                repo_id
            )
        })?;

    Ok((repo_id, file))
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
                quality_rank: 7,
            },
            HfQuantVariant {
                quantization: "Q4_K_M".into(),
                size_bytes: 4_000_000_000,
                filename: "m-Q4_K_M.gguf".into(),
                download_url: String::new(),
                description: "Medium",
                quality_rank: 19,
            },
            HfQuantVariant {
                quantization: "Q8_0".into(),
                size_bytes: 8_000_000_000,
                filename: "m-Q8_0.gguf".into(),
                download_url: String::new(),
                description: "Large",
                quality_rank: 25,
            },
        ];

        assert_eq!(recommend_variant(&variants, 5_000_000_000), Some(1));
        assert_eq!(recommend_variant(&variants, 10_000_000_000), Some(2));
        assert_eq!(recommend_variant(&variants, 1_000_000_000), None);
    }
}
