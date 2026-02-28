//! Model recommendation engine for local inference.
//!
//! Provides hardware-aware model recommendations by:
//! - Detecting available memory (GPU VRAM or system RAM)
//! - Estimating model memory requirements including KV cache
//! - Scoring models based on quality and fit
//! - Estimating inference speed

use super::hf_models::HfQuantVariant;
use super::local_model_registry::{
    get_registry, is_featured_model, LocalModelEntry, FEATURED_MODELS,
};
use super::InferenceRuntime;
use llama_cpp_2::{list_llama_ggml_backend_devices, LlamaBackendDeviceType};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

/// Bytes per parameter for each quantization level.
/// Used to estimate parameter count from file size and for memory calculations.
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

/// Get bytes-per-parameter for a quantization level.
pub fn quant_bpp(quant: &str) -> f64 {
    let upper = quant.to_uppercase();
    QUANT_BPP
        .iter()
        .find(|(q, _)| *q == upper)
        .map(|(_, bpp)| *bpp)
        .unwrap_or(0.58) // Default to Q4_K_M-ish
}

/// Estimate parameter count in billions from file size and quantization.
pub fn estimate_params_billion(size_bytes: u64, quant: &str) -> f64 {
    let bpp = quant_bpp(quant);
    if bpp == 0.0 || size_bytes == 0 {
        return 0.0;
    }
    (size_bytes as f64) / bpp / 1e9
}

/// Estimate memory required in GB to run a model with given context length.
///
/// Memory formula: model_weights + KV_cache + overhead
/// - model_weights = params_b * bpp (already in the file size)
/// - KV_cache ≈ 0.000008 * params_b * context_length (in GB)
/// - overhead ≈ 0.5 GB for runtime buffers
pub fn estimate_memory_gb(size_bytes: u64, quant: &str, context_length: u32) -> f64 {
    let params_b = estimate_params_billion(size_bytes, quant);
    let model_mem_gb = size_bytes as f64 / 1e9;
    let kv_cache_gb = 0.000008 * params_b * context_length as f64;
    let overhead_gb = 0.5;
    model_mem_gb + kv_cache_gb + overhead_gb
}

/// Speed tier for user-friendly display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SpeedTier {
    Fast,
    Medium,
    Slow,
}

/// How the model will be executed based on available memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Fully loaded in GPU VRAM - fastest
    Gpu,
    /// Partial GPU with CPU offload - slower due to memory transfers
    CpuOffload,
    /// CPU only, no GPU acceleration
    CpuOnly,
}

impl RunMode {
    /// Speed multiplier for this run mode (relative to full GPU).
    pub fn speed_multiplier(&self) -> f64 {
        match self {
            RunMode::Gpu => 1.0,
            RunMode::CpuOffload => 0.5,
            RunMode::CpuOnly => 0.3,
        }
    }
}

/// Predict how the model will run based on memory requirements and availability.
pub fn predict_run_mode(model_memory_gb: f64, vram_gb: f64, has_gpu: bool) -> RunMode {
    if !has_gpu {
        return RunMode::CpuOnly;
    }

    // If model fits in VRAM with some headroom, it'll run fully on GPU
    if model_memory_gb <= vram_gb * 0.95 {
        RunMode::Gpu
    } else if model_memory_gb <= vram_gb * 2.0 {
        // Model is larger than VRAM but not massively so - will offload to CPU
        RunMode::CpuOffload
    } else {
        // Model is way too large for GPU - effectively CPU only
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
        if tps >= 80.0 {
            SpeedTier::Fast
        } else if tps >= 40.0 {
            SpeedTier::Medium
        } else {
            SpeedTier::Slow
        }
    }
}

/// Quantization speed multiplier - lower precision is faster.
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

/// Get the number of CPU cores available.
pub fn cpu_core_count() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

/// Get available GPU/accelerator VRAM in GB.
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

/// Estimate inference speed in tokens per second.
///
/// Accounts for model size, quantization, GPU availability, core count, and run mode.
/// A large model on a small GPU will be penalized for CPU offloading.
pub fn estimate_speed_tps(params_billion: f64, quant: &str, has_gpu: bool) -> f64 {
    estimate_speed_tps_with_memory(params_billion, quant, has_gpu, available_vram_gb())
}

/// Estimate inference speed with explicit VRAM availability.
/// This allows callers to provide known memory values for more accurate estimates.
pub fn estimate_speed_tps_with_memory(
    params_billion: f64,
    quant: &str,
    has_gpu: bool,
    vram_gb: f64,
) -> f64 {
    // Base speed constant by backend (empirically calibrated)
    // These represent rough tok/s for a 1B parameter model
    let base_k = if has_gpu {
        // GPU backends - Metal/CUDA/etc average around 160-220 for 1B
        180.0
    } else {
        // CPU-only is much slower
        if cfg!(target_arch = "aarch64") {
            80.0 // ARM CPUs (Apple Silicon CPU-only, etc)
        } else {
            60.0 // x86 CPUs
        }
    };

    // Speed scales inversely with parameter count
    let params = params_billion.max(0.1);
    let base_tps = base_k / params;

    // 10% bonus for systems with 8+ cores (matches llmfit)
    let core_mult = if cpu_core_count() >= 8 { 1.1 } else { 1.0 };

    // Estimate model memory to predict run mode
    // Use a rough approximation: params * 0.6 for Q4_K_M-ish models + overhead
    let model_mem_gb = params_billion * quant_bpp(quant) + 1.0;
    let run_mode = predict_run_mode(model_mem_gb, vram_gb, has_gpu);

    // Apply all multipliers
    base_tps * quant_speed_multiplier(quant) * core_mult * run_mode.speed_multiplier()
}

/// Check if a GPU/accelerator is available for inference.
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

/// Result of model fit analysis.
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
    /// Compute a combined score for ranking models.
    /// Higher is better. Balances fit (headroom) with quality.
    pub fn score(&self) -> f64 {
        if !self.fits {
            return -1.0;
        }
        // Normalize quality to 0-1 range (quality_rank is 1-28)
        let quality_normalized = self.quality_rank as f64 / 28.0;
        // Normalize headroom: 0% = 0, 50%+ = 1
        let headroom_normalized = (self.headroom_percent / 50.0).min(1.0);
        // Weight quality more heavily than headroom
        0.7 * quality_normalized + 0.3 * headroom_normalized
    }
}

/// Analyze how well a model fits the available memory.
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

/// Get available memory for inference in bytes.
///
/// Prefers GPU/accelerator memory if available, falls back to CPU/system RAM.
/// On Apple Silicon with unified memory, this returns the available unified memory.
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

/// Default context length used for recommendations when not specified.
const DEFAULT_CONTEXT_LENGTH: u32 = 8192;

/// Recommend the best local model from the registry for the current hardware.
///
/// Returns the model ID of the recommended model. Prefers:
/// 1. Downloaded models that fit in memory
/// 2. Highest quality model that fits
/// 3. Falls back to smallest featured model if nothing fits
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

/// Recommend the best quantization variant for a given model based on available memory.
///
/// Returns the index of the recommended variant in the input slice.
/// Memory requirements (including KV cache and overhead) are calculated by analyze_model_fit.
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

    #[test]
    fn test_quant_bpp() {
        assert!((quant_bpp("Q4_K_M") - 0.58).abs() < 0.01);
        assert!((quant_bpp("Q8_0") - 1.05).abs() < 0.01);
        assert!((quant_bpp("F16") - 2.0).abs() < 0.01);
        assert!((quant_bpp("unknown") - 0.58).abs() < 0.01); // Default
    }

    #[test]
    fn test_estimate_params_billion() {
        // A 4GB Q4_K_M file should be ~7B params
        let params = estimate_params_billion(4_000_000_000, "Q4_K_M");
        assert!(params > 6.0 && params < 8.0);

        // A 1GB Q4_K_M file should be ~1.7B params
        let params = estimate_params_billion(1_000_000_000, "Q4_K_M");
        assert!(params > 1.5 && params < 2.0);
    }

    #[test]
    fn test_estimate_memory_gb() {
        // 4GB file with 8K context
        let mem = estimate_memory_gb(4_000_000_000, "Q4_K_M", 8192);
        // Should be slightly more than 4GB (file + KV cache + overhead)
        assert!(mem > 4.0 && mem < 5.0);
    }

    #[test]
    fn test_analyze_model_fit() {
        // 4GB model with 8GB available
        let fit = analyze_model_fit(4_000_000_000, "Q4_K_M", 19, 8192, 8_000_000_000);
        assert!(fit.fits);
        assert!(fit.headroom_percent > 0.0);

        // 4GB model with 3GB available - shouldn't fit
        let fit = analyze_model_fit(4_000_000_000, "Q4_K_M", 19, 8192, 3_000_000_000);
        assert!(!fit.fits);
        assert!(fit.headroom_percent < 0.0);
    }

    #[test]
    fn test_model_fit_score() {
        // High quality, good headroom
        let fit1 = ModelFit {
            fits: true,
            estimated_memory_gb: 4.0,
            available_memory_gb: 8.0,
            headroom_percent: 50.0,
            quality_rank: 25,
            params_billion: 7.0,
        };

        // Lower quality, same headroom
        let fit2 = ModelFit {
            fits: true,
            estimated_memory_gb: 4.0,
            available_memory_gb: 8.0,
            headroom_percent: 50.0,
            quality_rank: 10,
            params_billion: 7.0,
        };

        assert!(fit1.score() > fit2.score());

        // Doesn't fit
        let fit3 = ModelFit {
            fits: false,
            estimated_memory_gb: 10.0,
            available_memory_gb: 8.0,
            headroom_percent: -25.0,
            quality_rank: 25,
            params_billion: 14.0,
        };
        assert!(fit3.score() < 0.0);
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

        // Memory estimation includes KV cache + overhead:
        // Q2_K (2GB file): ~2GB + 0.28GB KV + 0.5GB overhead = ~2.78GB needed
        // Q4_K_M (4GB file): ~4GB + 0.45GB KV + 0.5GB overhead = ~4.95GB needed
        // Q8_0 (8GB file): ~8GB + 0.50GB KV + 0.5GB overhead = ~9.0GB needed
        //
        // Scoring balances quality (70%) with headroom (30%), so a model with
        // comfortable headroom may beat a higher-quality model that barely fits.

        // With 6GB available: Q4_K_M fits (4.95 < 6), Q8_0 doesn't (9.0 > 6)
        assert_eq!(recommend_variant(&variants, 6_000_000_000), Some(1));

        // With 10GB available: Q8_0 fits with ~10% headroom
        // Q8_0: headroom = (10-9)/10 = 10%, score = 0.7*0.89 + 0.3*0.2 = 0.68
        // Q4_K_M: headroom = (10-4.95)/10 = 50%, score = 0.7*0.68 + 0.3*1.0 = 0.78
        // Q4_K_M wins due to better headroom balance
        assert_eq!(recommend_variant(&variants, 10_000_000_000), Some(1));

        // With 16GB available: Q8_0 has plenty of headroom
        // Q8_0: headroom = (16-9)/16 = 44%, score = 0.7*0.89 + 0.3*0.88 = 0.89
        // Q4_K_M: headroom capped at 1.0, score = 0.7*0.68 + 0.3*1.0 = 0.78
        assert_eq!(recommend_variant(&variants, 16_000_000_000), Some(2));

        // With 2GB available: nothing fits (Q2_K needs ~2.78GB)
        assert_eq!(recommend_variant(&variants, 2_000_000_000), None);

        // With 4GB available: Q2_K fits (2.78 < 4)
        assert_eq!(recommend_variant(&variants, 4_000_000_000), Some(0));
    }

    #[test]
    fn test_predict_run_mode() {
        // Model fits in VRAM
        assert_eq!(predict_run_mode(4.0, 8.0, true), RunMode::Gpu);

        // Model slightly larger than VRAM - will offload
        assert_eq!(predict_run_mode(10.0, 8.0, true), RunMode::CpuOffload);

        // Model way too large for VRAM
        assert_eq!(predict_run_mode(20.0, 8.0, true), RunMode::CpuOnly);

        // No GPU
        assert_eq!(predict_run_mode(4.0, 8.0, false), RunMode::CpuOnly);
        assert_eq!(predict_run_mode(4.0, 0.0, true), RunMode::CpuOnly);
    }

    #[test]
    fn test_estimate_speed_tps_with_memory() {
        // 1B model fitting in 16GB VRAM should be fast (full GPU)
        let tps = estimate_speed_tps_with_memory(1.0, "Q4_K_M", true, 16.0);
        assert!(tps > 100.0, "1B on GPU should be >100 tps, got {}", tps);

        // 7B model fitting in 16GB VRAM
        let tps = estimate_speed_tps_with_memory(7.0, "Q4_K_M", true, 16.0);
        assert!(tps > 20.0, "7B on GPU should be >20 tps, got {}", tps);

        // 7B model on small 4GB VRAM - will need offload, slower
        let tps_offload = estimate_speed_tps_with_memory(7.0, "Q4_K_M", true, 4.0);
        let tps_full = estimate_speed_tps_with_memory(7.0, "Q4_K_M", true, 16.0);
        assert!(
            tps_offload < tps_full,
            "Offload should be slower: {} vs {}",
            tps_offload,
            tps_full
        );

        // CPU is slower than GPU
        let cpu_tps = estimate_speed_tps_with_memory(7.0, "Q4_K_M", false, 0.0);
        assert!(tps_full > cpu_tps, "GPU should be faster than CPU");

        // Lower quant is faster
        let q4_tps = estimate_speed_tps_with_memory(7.0, "Q4_K_M", true, 16.0);
        let q8_tps = estimate_speed_tps_with_memory(7.0, "Q8_0", true, 16.0);
        assert!(q4_tps > q8_tps, "Q4 should be faster than Q8");
    }

    #[test]
    fn test_speed_tier() {
        assert_eq!(SpeedTier::from_tps(100.0), SpeedTier::Fast);
        assert_eq!(SpeedTier::from_tps(80.0), SpeedTier::Fast);
        assert_eq!(SpeedTier::from_tps(79.0), SpeedTier::Medium);
        assert_eq!(SpeedTier::from_tps(40.0), SpeedTier::Medium);
        assert_eq!(SpeedTier::from_tps(39.0), SpeedTier::Slow);
        assert_eq!(SpeedTier::from_tps(10.0), SpeedTier::Slow);
    }
}
