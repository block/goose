use crate::config::paths::Paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WhisperModel {
    /// Model identifier (e.g., "tiny", "base", "small")
    pub id: String,
    /// Display name
    pub name: String,
    /// Model file size in bytes
    pub size_bytes: u64,
    /// Model file size formatted for display (e.g., "40MB")
    pub size_display: String,
    /// Quality tier: "fast", "balanced", "accurate"
    pub quality: String,
    /// Download URL from HuggingFace
    pub url: String,
    /// Whether this model is currently downloaded
    pub downloaded: bool,
    /// Recommended for which hardware tier
    pub recommended_for: Vec<String>, // e.g., ["low_end", "mid_range", "high_end"]
    /// Relative transcription speed (multiplier of realtime, e.g., "3.5x")
    pub speed: String,
    /// Description
    pub description: String,
}

impl WhisperModel {
    pub fn filename(&self) -> String {
        // Tiny model has different naming convention
        if self.id == "tiny" {
            "model-tiny-q80.gguf".to_string()
        } else {
            format!("whisper-{}-q8_0.gguf", self.id)
        }
    }

    pub fn local_path(&self) -> PathBuf {
        Paths::in_data_dir("models").join(self.filename())
    }

    pub fn is_downloaded(&self) -> bool {
        self.local_path().exists()
    }
}

pub fn available_models() -> Vec<WhisperModel> {
    let models = [
        ("tiny", "Tiny", 40 * 1024 * 1024, "40MB", "fast", "https://huggingface.co/oxide-lab/whisper-tiny-GGUF/resolve/main/model-tiny-q80.gguf", &["low_end", "laptop"][..], "~2-3x (CPU)", "Fastest model, good for quick transcription. 5-10x faster with GPU."),
        ("base", "Base", 78 * 1024 * 1024, "78MB", "balanced", "https://huggingface.co/oxide-lab/whisper-base-GGUF/resolve/main/whisper-base-q8_0.gguf", &["mid_range"][..], "~1.5-2x (CPU)", "Good balance of speed and accuracy. 4-8x faster with GPU."),
        ("small", "Small", 247 * 1024 * 1024, "247MB", "accurate", "https://huggingface.co/oxide-lab/whisper-small-GGUF/resolve/main/whisper-small-q8_0.gguf", &["high_end", "desktop"][..], "~0.8-1x (CPU)", "High accuracy. 3-5x faster with GPU."),
        ("medium", "Medium", 777 * 1024 * 1024, "777MB", "very_accurate", "https://huggingface.co/oxide-lab/whisper-medium-GGUF/resolve/main/whisper-medium-q8_0.gguf", &["high_end"][..], "~0.5x (CPU)", "Highest accuracy. 2-4x faster with GPU. Requires powerful machine."),
    ];

    models
        .iter()
        .map(
            |(id, name, size_bytes, size_display, quality, url, recommended_for, speed, description)| {
                WhisperModel {
                    id: id.to_string(),
                    name: name.to_string(),
                    size_bytes: *size_bytes,
                    size_display: size_display.to_string(),
                    quality: quality.to_string(),
                    url: url.to_string(),
                    downloaded: false,
                    recommended_for: recommended_for.iter().map(|s| s.to_string()).collect(),
                    speed: speed.to_string(),
                    description: description.to_string(),
                }
            },
        )
        .collect()
}

