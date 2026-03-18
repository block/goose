//! Model recommendation engine for local inference.

use super::hf_models::HfQuantVariant;
use super::local_model_registry::{
    get_registry, is_featured_model, LocalModelEntry, FEATURED_MODELS,
};
use super::InferenceRuntime;
use llama_cpp_2::{list_llama_ggml_backend_devices, LlamaBackendDeviceType};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

const QUANT_BPP: &[(&str, f64)] = &[
    ("F32", 4.0),
    ("BF16", 2.0),
    ("F16", 2.0),
    ("Q8_0", 1.05),
    ("Q6_K", 0.80),
    ("Q5_K_M", 0.68),
    ("Q5_K_S", 0.68),
    ("Q5_1", 0.69),
    ("Q5_0", 0.63),
    ("Q4_K_M", 0.58),
    ("Q4_K_S", 0.56),
    ("Q4_K_L", 0.58),
    ("Q4_1", 0.56),
    ("Q4_0", 0.50),
    ("IQ4_XS", 0.52),
    ("IQ4_NL", 0.55),
    ("Q3_K_M", 0.48),
    ("Q3_K_S", 0.44),
    ("Q3_K_L", 0.51),
    ("IQ3_M", 0.44),
    ("IQ3_S", 0.42),
    ("IQ3_XS", 0.40),
    ("IQ3_XXS", 0.38),
    ("Q2_K", 0.37),
    ("Q2_K_S", 0.35),
    ("Q2_K_L", 0.40),
    ("IQ2_M", 0.32),
    ("IQ2_S", 0.30),
    ("IQ2_XS", 0.28),
    ("IQ2_XXS", 0.26),
    ("IQ1_M", 0.20),
    ("IQ1_S", 0.18),
    ("TQ1_0", 0.16),
    ("MXFP4_MOE", 0.58),
];

pub fn quant_bpp(quant: &str) -> f64 {
    let upper = quant.to_uppercase();
    QUANT_BPP
        .iter()
        .find(|(q, _)| *q == upper)
        .map(|(_, bpp)| *bpp)
        .unwrap_or(0.58) // Default to Q4_K_M-ish
}

pub fn estimate_params_billion(size_bytes: u64, quant: &str) -> f64 {
    let bpp = quant_bpp(quant);
    if bpp == 0.0 || size_bytes == 0 {
        return 0.0;
    }
    (size_bytes as f64) / bpp / 1e9
}

pub fn estimate_memory_gb(size_bytes: u64, quant: &str, context_length: u32) -> f64 {
    let params_b = estimate_params_billion(size_bytes, quant);
    let model_mem_gb = size_bytes as f64 / 1e9;
    let kv_cache_gb = 0.000008 * params_b * context_length as f64;
    let overhead_gb = 0.5;
    model_mem_gb + kv_cache_gb + overhead_gb
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SpeedTier {
    Fast,
    Medium,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Gpu,
    CpuOffload,
    CpuOnly,
}

impl RunMode {
    pub fn speed_multiplier(&self) -> f64 {
        match self {
            RunMode::Gpu => 1.0,
            RunMode::CpuOffload => 0.5,
            RunMode::CpuOnly => 0.3,
        }
    }
}

pub fn predict_run_mode(model_memory_gb: f64, vram_gb: f64, has_gpu: bool) -> RunMode {
    if !has_gpu {
        return RunMode::CpuOnly;
    }

    if model_memory_gb <= vram_gb * 0.95 {
        RunMode::Gpu
    } else if model_memory_gb <= vram_gb * 2.0 {
        RunMode::CpuOffload
    } else {
        RunMode::CpuOnly
    }
}

impl fmt::Display for SpeedTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpeedTier::Fast => write!(f, "fast"),
            SpeedTier::Medium => write!(f, "medium"),
            SpeedTier::Slow => write!(f, "slow"),
        }
    }
}

impl SpeedTier {
    pub fn from_tps(tps: f64) -> Self {
        if tps >= 20.0 {
            SpeedTier::Fast
        } else if tps >= 8.0 {
            SpeedTier::Medium
        } else {
            SpeedTier::Slow
        }
    }
}

fn quant_speed_multiplier(quant: &str) -> f64 {
    let upper = quant.to_uppercase();
    if upper.starts_with("F32") {
        0.5
    } else if upper.starts_with("F16") || upper.starts_with("BF16") {
        0.7
    } else if upper.starts_with("Q8") {
        0.85
    } else if upper.starts_with("Q6") {
        0.9
    } else if upper.starts_with("Q5") {
        0.95
    } else if upper.starts_with("Q4") || upper.starts_with("IQ4") {
        1.0
    } else if upper.starts_with("Q3") || upper.starts_with("IQ3") {
        1.05
    } else if upper.starts_with("Q2") || upper.starts_with("IQ2") {
        1.1
    } else if upper.starts_with("IQ1") || upper.starts_with("TQ1") {
        1.15
    } else {
        1.0
    }
}

pub fn cpu_core_count() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

pub fn available_vram_gb() -> f64 {
    let devices = list_llama_ggml_backend_devices();
    devices
        .iter()
        .filter(|d| {
            matches!(
                d.device_type,
                LlamaBackendDeviceType::Gpu
                    | LlamaBackendDeviceType::IntegratedGpu
                    | LlamaBackendDeviceType::Accelerator
            )
        })
        .map(|d| d.memory_free as f64 / 1e9)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0)
}

pub fn estimate_speed_tps(params_billion: f64, quant: &str, has_gpu: bool) -> f64 {
    let base_k = if has_gpu {
        300.0
    } else if cfg!(target_arch = "aarch64") {
        80.0
    } else {
        60.0
    };

    let params = params_billion.max(0.1);
    let base_tps = base_k / params;

    let core_mult = if cpu_core_count() >= 8 { 1.1 } else { 1.0 };

    let model_mem_gb = params_billion * quant_bpp(quant) + 1.0;
    let run_mode = predict_run_mode(model_mem_gb, available_vram_gb(), has_gpu);
    base_tps * quant_speed_multiplier(quant) * core_mult * run_mode.speed_multiplier()
}

pub fn has_gpu_accelerator() -> bool {
    let devices = list_llama_ggml_backend_devices();
    devices.iter().any(|d| {
        matches!(
            d.device_type,
            LlamaBackendDeviceType::Gpu
                | LlamaBackendDeviceType::IntegratedGpu
                | LlamaBackendDeviceType::Accelerator
        )
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelFit {
    /// Whether the model fits in available memory
    pub fits: bool,
    /// Estimated memory required in GB
    pub estimated_memory_gb: f64,
    /// Available memory in GB
    pub available_memory_gb: f64,
    /// Headroom as a percentage (0-100, negative if doesn't fit)
    pub headroom_percent: f64,
    /// Quality rank (1-28, higher is better quality)
    pub quality_rank: u8,
    /// Estimated parameters in billions
    pub params_billion: f64,
}

impl ModelFit {
    pub fn score(&self) -> f64 {
        if !self.fits {
            return -1.0;
        }
        // Normalize quality to 0-1 range (quality_rank is 1-28)
        let quality_normalized = self.quality_rank as f64 / 28.0;
        // Normalize headroom: prefer using more memory (larger models are better)
        // 20% headroom = 1.0 (ideal), 80%+ headroom = 0.25 (wasting capacity)
        let headroom_normalized = if self.headroom_percent <= 20.0 {
            1.0
        } else {
            // Linear decay from 1.0 at 20% to 0.25 at 80%
            1.0 - 0.75 * ((self.headroom_percent - 20.0) / 60.0).min(1.0)
        };
        // Weight quality more heavily than memory utilization
        0.7 * quality_normalized + 0.3 * headroom_normalized
    }
}

pub fn analyze_model_fit(
    size_bytes: u64,
    quant: &str,
    quality_rank: u8,
    context_length: u32,
    available_memory_bytes: u64,
) -> ModelFit {
    let estimated_memory_gb = estimate_memory_gb(size_bytes, quant, context_length);
    let available_memory_gb = available_memory_bytes as f64 / 1e9;
    let params_billion = estimate_params_billion(size_bytes, quant);

    let fits = estimated_memory_gb <= available_memory_gb;
    let headroom_percent = if available_memory_gb > 0.0 {
        ((available_memory_gb - estimated_memory_gb) / available_memory_gb) * 100.0
    } else {
        -100.0
    };

    ModelFit {
        fits,
        estimated_memory_gb,
        available_memory_gb,
        headroom_percent,
        quality_rank,
        params_billion,
    }
}

pub fn available_inference_memory_bytes(runtime: &InferenceRuntime) -> u64 {
    let _ = &runtime.backend();
    let devices = list_llama_ggml_backend_devices();

    // Find accelerator memory (GPU, integrated GPU, or other accelerator)
    let accel_memory = devices
        .iter()
        .filter(|d| {
            matches!(
                d.device_type,
                LlamaBackendDeviceType::Gpu
                    | LlamaBackendDeviceType::IntegratedGpu
                    | LlamaBackendDeviceType::Accelerator
            )
        })
        .map(|d| d.memory_free as u64)
        .max()
        .unwrap_or(0);

    if accel_memory > 0 {
        accel_memory
    } else {
        // Fall back to CPU memory
        devices
            .iter()
            .filter(|d| d.device_type == LlamaBackendDeviceType::Cpu)
            .map(|d| d.memory_free as u64)
            .max()
            .unwrap_or(0)
    }
}

const DEFAULT_CONTEXT_LENGTH: u32 = 8192;

pub fn recommend_local_model(runtime: &InferenceRuntime) -> String {
    let available_memory = available_inference_memory_bytes(runtime);

    if let Ok(registry) = get_registry().lock() {
        // Collect featured models with their fit analysis
        let mut candidates: Vec<(&LocalModelEntry, ModelFit)> = registry
            .list_models()
            .iter()
            .filter(|m| is_featured_model(&m.id) && m.size_bytes > 0)
            .map(|m| {
                let fit = analyze_model_fit(
                    m.size_bytes,
                    &m.quantization,
                    m.quality_rank,
                    DEFAULT_CONTEXT_LENGTH,
                    available_memory,
                );
                (m, fit)
            })
            .collect();

        // Sort by score (highest first)
        candidates.sort_by(|a, b| {
            b.1.score()
                .partial_cmp(&a.1.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return best fitting model
        if let Some((model, fit)) = candidates.first() {
            if fit.fits {
                return model.id.clone();
            }
        }

        // If nothing fits, return smallest model
        candidates.sort_by_key(|(m, _)| m.size_bytes);
        if let Some((model, _)) = candidates.first() {
            return model.id.clone();
        }
    }

    // Fallback to first featured model
    FEATURED_MODELS[0].to_string()
}

#[cfg(test)]
fn make_variant(quant: &str, size_bytes: u64) -> HfQuantVariant {
    use super::hf_models::quant_quality_rank;
    HfQuantVariant {
        quantization: quant.to_string(),
        size_bytes,
        filename: format!("model-{}.gguf", quant),
        download_url: String::new(),
        description: "",
        quality_rank: quant_quality_rank(quant),
    }
}

pub fn recommend_variant(
    variants: &[HfQuantVariant],
    available_memory_bytes: u64,
) -> Option<usize> {
    let mut best: Option<(usize, ModelFit)> = None;

    for (i, v) in variants.iter().enumerate() {
        let fit = analyze_model_fit(
            v.size_bytes,
            &v.quantization,
            v.quality_rank,
            DEFAULT_CONTEXT_LENGTH,
            available_memory_bytes,
        );

        if fit.fits {
            match &best {
                Some((_, best_fit)) if fit.score() > best_fit.score() => {
                    best = Some((i, fit));
                }
                None => {
                    best = Some((i, fit));
                }
                _ => {}
            }
        }
    }

    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn standard_variants() -> Vec<HfQuantVariant> {
        vec![
            make_variant("Q2_K", 3_000_000_000),
            make_variant("Q4_K_M", 5_000_000_000),
            make_variant("Q6_K", 7_000_000_000),
            make_variant("Q8_0", 9_000_000_000),
        ]
    }

    //        available memory          → expected quant
    #[test_case(4_500_000_000,  Some("Q2_K")   ; "tight memory only fits smallest")]
    #[test_case(7_000_000_000,  Some("Q4_K_M") ; "moderate memory picks mid quality")]
    #[test_case(9_000_000_000,  Some("Q6_K")   ; "good memory picks high quality")]
    #[test_case(24_000_000_000, Some("Q8_0")   ; "plenty of memory picks best quality")]
    #[test_case(2_000_000_000,  None           ; "insufficient memory recommends nothing")]
    fn recommend_variant_picks_best_quality_that_fits(
        available_bytes: u64,
        expected_quant: Option<&str>,
    ) {
        let variants = standard_variants();
        let result = recommend_variant(&variants, available_bytes);
        assert_eq!(
            result.map(|i| variants[i].quantization.as_str()),
            expected_quant,
        );
    }
}
