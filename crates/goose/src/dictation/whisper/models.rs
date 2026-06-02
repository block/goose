use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WhisperModel {
    /// Model identifier (e.g., "tiny", "base", "small")
    pub id: &'static str,
    /// Model file size in MB
    pub size_mb: u32,
    /// Download URL from HuggingFace
    pub url: &'static str,
    /// Description
    pub description: &'static str,
}

const MODELS: &[WhisperModel] = &[
    WhisperModel {
        id: "tiny",
        size_mb: 40,
        url: "https://huggingface.co/oxide-lab/whisper-tiny-GGUF/resolve/main/model-tiny-q80.gguf",
        description: "Fastest, ~2-3x realtime on CPU (5-10x with GPU)",
    },
    WhisperModel {
        id: "base",
        size_mb: 78,
        url: "https://huggingface.co/oxide-lab/whisper-base-GGUF/resolve/main/whisper-base-q8_0.gguf",
        description: "Good balance, ~1.5-2x realtime on CPU (4-8x with GPU)",
    },
    WhisperModel {
        id: "small",
        size_mb: 247,
        url: "https://huggingface.co/oxide-lab/whisper-small-GGUF/resolve/main/whisper-small-q8_0.gguf",
        description: "High accuracy, ~0.8-1x realtime on CPU (3-5x with GPU)",
    },
    WhisperModel {
        id: "medium",
        size_mb: 777,
        url: "https://huggingface.co/oxide-lab/whisper-medium-GGUF/resolve/main/whisper-medium-q8_0.gguf",
        description: "Highest accuracy, ~0.5x realtime on CPU (2-4x with GPU)",
    },
];

impl WhisperModel {
    pub fn local_path(&self) -> PathBuf {
        let filename = self.url.rsplit('/').next().unwrap_or("");
        Paths::in_data_dir("models").join(filename)
    }

    pub fn is_downloaded(&self) -> bool {
        self.local_path().exists()
    }

    pub fn config(&self) -> Config {
        match self.id {
            "tiny" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 384,
                encoder_attention_heads: 6,
                encoder_layers: 4,
                decoder_attention_heads: 6,
                decoder_layers: 4,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            "base" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 512,
                encoder_attention_heads: 8,
                encoder_layers: 6,
                decoder_attention_heads: 8,
                decoder_layers: 6,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            "small" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 768,
                encoder_attention_heads: 12,
                encoder_layers: 12,
                decoder_attention_heads: 12,
                decoder_layers: 12,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            "medium" => Config {
                num_mel_bins: 80,
                max_source_positions: 1500,
                d_model: 1024,
                encoder_attention_heads: 16,
                encoder_layers: 24,
                decoder_attention_heads: 16,
                decoder_layers: 24,
                vocab_size: 51865,
                suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                max_target_positions: 448,
            },
            _ => {
                tracing::warn!("Unknown model '{}', falling back to tiny config", self.id);
                Config {
                    num_mel_bins: 80,
                    max_source_positions: 1500,
                    d_model: 384,
                    encoder_attention_heads: 6,
                    encoder_layers: 4,
                    decoder_attention_heads: 6,
                    decoder_layers: 4,
                    vocab_size: 51865,
                    suppress_tokens: SUPPRESS_TOKENS.to_vec(),
                    max_target_positions: 448,
                }
            }
        }
    }
}

pub fn available_models() -> &'static [WhisperModel] {
    MODELS
}

pub fn get_model(id: &str) -> Option<&'static WhisperModel> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn recommend_model() -> &'static str {
    let has_gpu_or_metal = Device::new_cuda(0).is_ok() || Device::new_metal(0).is_ok();

    if has_gpu_or_metal {
        "small"
    } else {
        let cpu_count = sys_info::cpu_num().unwrap_or(1) as u64;
        let cpu_speed_mhz = sys_info::cpu_speed().unwrap_or(0);

        if cpu_count * cpu_speed_mhz >= 16_000 {
            "base"
        } else {
            "tiny"
        }
    }
}
