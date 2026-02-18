use crate::config::paths::Paths;
use crate::dictation::download_manager::{get_download_manager, DownloadStatus};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum SamplingConfig {
    Greedy,
    Temperature {
        temperature: f32,
        top_k: i32,
        top_p: f32,
        min_p: f32,
        seed: Option<u32>,
    },
    MirostatV2 {
        tau: f32,
        eta: f32,
        seed: Option<u32>,
    },
}

impl Default for SamplingConfig {
    fn default() -> Self {
        SamplingConfig::Temperature {
            temperature: 0.8,
            top_k: 40,
            top_p: 0.95,
            min_p: 0.05,
            seed: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelSettings {
    pub context_size: Option<u32>,
    pub max_output_tokens: Option<usize>,
    #[serde(default)]
    pub sampling: SamplingConfig,
    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f32,
    #[serde(default = "default_repeat_last_n")]
    pub repeat_last_n: i32,
    #[serde(default)]
    pub frequency_penalty: f32,
    #[serde(default)]
    pub presence_penalty: f32,
    pub n_batch: Option<u32>,
    pub n_gpu_layers: Option<u32>,
    #[serde(default)]
    pub use_mlock: bool,
    pub flash_attention: Option<bool>,
    pub n_threads: Option<i32>,
    #[serde(default)]
    pub native_tool_calling: bool,
    #[serde(default)]
    pub use_jinja: bool,
}

fn default_repeat_penalty() -> f32 {
    1.0
}

fn default_repeat_last_n() -> i32 {
    64
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            context_size: None,
            max_output_tokens: None,
            sampling: SamplingConfig::default(),
            repeat_penalty: 1.0,
            repeat_last_n: 64,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            n_batch: None,
            n_gpu_layers: None,
            use_mlock: false,
            flash_attention: None,
            n_threads: None,
            native_tool_calling: false,
            use_jinja: false,
        }
    }
}

/// Featured models - just the HuggingFace specs.
/// Format: "author/repo-GGUF:quantization"
pub const FEATURED_MODELS: &[&str] = &[
    "bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M",
    "bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M",
    "bartowski/Hermes-2-Pro-Mistral-7B-GGUF:Q4_K_M",
    "bartowski/Mistral-Small-24B-Instruct-2501-GGUF:Q4_K_M",
];

/// Parse a model spec like "author/repo:quantization" into (repo_id, quantization)
pub fn parse_model_spec(spec: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = spec.rsplitn(2, ':').collect();
    if parts.len() == 2 {
        Some((parts[1], parts[0]))
    } else {
        None
    }
}

/// Check if a model ID corresponds to a featured model
pub fn is_featured_model(model_id: &str) -> bool {
    FEATURED_MODELS.iter().any(|spec| {
        if let Some((repo_id, quant)) = parse_model_spec(spec) {
            model_id_from_repo(repo_id, quant) == model_id
        } else {
            false
        }
    })
}

/// Get the spec for a featured model by model_id
pub fn get_featured_spec(model_id: &str) -> Option<&'static str> {
    FEATURED_MODELS.iter().find(|spec| {
        if let Some((repo_id, quant)) = parse_model_spec(spec) {
            model_id_from_repo(repo_id, quant) == model_id
        } else {
            false
        }
    }).copied()
}

/// Legacy model definitions for backwards compatibility.
pub struct LegacyModel {
    pub id: &'static str,
    pub display_name: &'static str,
    pub repo_id: &'static str,
    pub filename: &'static str,
    pub quantization: &'static str,
    pub source_url: &'static str,
}

pub const LEGACY_MODELS: &[LegacyModel] = &[
    LegacyModel {
        id: "llama-3.2-1b",
        display_name: "Llama 3.2 1B",
        repo_id: "bartowski/Llama-3.2-1B-Instruct-GGUF",
        filename: "Llama-3.2-1B-Instruct-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
    },
    LegacyModel {
        id: "llama-3.2-3b",
        display_name: "Llama 3.2 3B",
        repo_id: "bartowski/Llama-3.2-3B-Instruct-GGUF",
        filename: "Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
    },
    LegacyModel {
        id: "hermes-2-pro-7b",
        display_name: "Hermes 2 Pro 7B",
        repo_id: "bartowski/Hermes-2-Pro-Mistral-7B-GGUF",
        filename: "Hermes-2-Pro-Mistral-7B-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/bartowski/Hermes-2-Pro-Mistral-7B-GGUF/resolve/main/Hermes-2-Pro-Mistral-7B-Q4_K_M.gguf",
    },
    LegacyModel {
        id: "mistral-small-22b",
        display_name: "Mistral Small 24B",
        repo_id: "bartowski/Mistral-Small-24B-Instruct-2501-GGUF",
        filename: "Mistral-Small-24B-Instruct-2501-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/bartowski/Mistral-Small-24B-Instruct-2501-GGUF/resolve/main/Mistral-Small-24B-Instruct-2501-Q4_K_M.gguf",
    },
];

static REGISTRY: OnceLock<Mutex<LocalModelRegistry>> = OnceLock::new();

pub fn get_registry() -> &'static Mutex<LocalModelRegistry> {
    REGISTRY.get_or_init(|| {
        let mut registry = LocalModelRegistry::load().unwrap_or_default();
        registry.migrate_legacy_models();
        registry.migrate_model_ids();
        Mutex::new(registry)
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelEntry {
    pub id: String,
    pub display_name: String,
    pub repo_id: String,
    pub filename: String,
    pub quantization: String,
    pub local_path: PathBuf,
    pub source_url: String,
    #[serde(default)]
    pub settings: ModelSettings,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default = "default_context_limit")]
    pub context_limit: u32,
}

fn default_context_limit() -> u32 {
    8192 // Default context limit for most models
}

impl LocalModelEntry {
    /// Check if the model file is downloaded
    pub fn is_downloaded(&self) -> bool {
        self.local_path.exists()
    }

    /// Get the download status of this model
    pub fn download_status(&self) -> ModelDownloadStatus {
        if self.local_path.exists() {
            return ModelDownloadStatus::Downloaded;
        }

        // Check if there's an active download
        let download_id = format!("{}-model", self.id);
        let manager = get_download_manager();
        if let Some(progress) = manager.get_progress(&download_id) {
            return match progress.status {
                DownloadStatus::Downloading => ModelDownloadStatus::Downloading {
                    progress_percent: progress.progress_percent,
                    bytes_downloaded: progress.bytes_downloaded,
                    total_bytes: progress.total_bytes,
                    speed_bps: progress.speed_bps.unwrap_or(0),
                },
                DownloadStatus::Completed => ModelDownloadStatus::Downloaded,
                DownloadStatus::Failed => ModelDownloadStatus::NotDownloaded,
                DownloadStatus::Cancelled => ModelDownloadStatus::NotDownloaded,
            };
        }

        ModelDownloadStatus::NotDownloaded
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelDownloadStatus {
    NotDownloaded,
    Downloading {
        progress_percent: f32,
        bytes_downloaded: u64,
        total_bytes: u64,
        speed_bps: u64,
    },
    Downloaded,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalModelRegistry {
    #[serde(default)]
    pub models: Vec<LocalModelEntry>,
}

impl LocalModelRegistry {
    pub fn load() -> Result<Self> {
        let path = Paths::in_data_dir("models/registry.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Paths::in_data_dir("models/registry.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Migrate model IDs from old format to new HuggingFace-style format.
    pub fn migrate_model_ids(&mut self) {
        let mut changed = false;
        for entry in &mut self.models {
            // Check if this is an old-style ID (no colons)
            if !entry.id.contains(':') && !entry.repo_id.is_empty() {
                let new_id = model_id_from_repo(&entry.repo_id, &entry.quantization);
                entry.id = new_id;
                changed = true;
            }
        }
        if changed {
            let _ = self.save();
        }
    }

    /// Scan for legacy model files and add them to the registry if not already present.
    pub fn migrate_legacy_models(&mut self) {
        let mut changed = false;
        for legacy in LEGACY_MODELS {
            let legacy_path = Paths::in_data_dir("models").join(format!("{}.gguf", legacy.id));
            if legacy_path.exists() && !self.models.iter().any(|m| m.id == legacy.id) {
                self.models.push(LocalModelEntry {
                    id: legacy.id.to_string(),
                    display_name: legacy.display_name.to_string(),
                    repo_id: legacy.repo_id.to_string(),
                    filename: legacy.filename.to_string(),
                    quantization: legacy.quantization.to_string(),
                    local_path: legacy_path,
                    source_url: legacy.source_url.to_string(),
                    settings: ModelSettings::default(),
                    size_bytes: 0,
                    context_limit: 8192, // Default for legacy models
                });
                changed = true;
            }
        }
        if changed {
            let _ = self.save();
        }
    }

    /// Sync registry with featured models:
    /// - Add any featured models that are missing
    /// - Remove any non-downloaded, non-featured models
    pub fn sync_with_featured(&mut self, featured_entries: Vec<LocalModelEntry>) {
        let mut changed = false;

        // Add missing featured models
        for entry in featured_entries {
            if !self.models.iter().any(|m| m.id == entry.id) {
                self.models.push(entry);
                changed = true;
            }
        }

        // Remove non-downloaded, non-featured models
        let before_len = self.models.len();
        self.models.retain(|m| {
            m.is_downloaded() || is_featured_model(&m.id)
        });
        if self.models.len() != before_len {
            changed = true;
        }

        if changed {
            let _ = self.save();
        }
    }

    pub fn add_model(&mut self, entry: LocalModelEntry) -> Result<()> {
        if let Some(existing) = self.models.iter_mut().find(|m| m.id == entry.id) {
            *existing = entry;
        } else {
            self.models.push(entry);
        }
        self.save()
    }

    pub fn remove_model(&mut self, id: &str) -> Result<()> {
        self.models.retain(|m| m.id != id);
        self.save()
    }

    pub fn get_model(&self, id: &str) -> Option<&LocalModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn get_model_mut(&mut self, id: &str) -> Option<&mut LocalModelEntry> {
        self.models.iter_mut().find(|m| m.id == id)
    }

    pub fn has_model(&self, id: &str) -> bool {
        self.models.iter().any(|m| m.id == id)
    }

    pub fn get_model_settings(&self, id: &str) -> Option<&ModelSettings> {
        self.models.iter().find(|m| m.id == id).map(|m| &m.settings)
    }

    pub fn update_model_settings(&mut self, id: &str, settings: ModelSettings) -> Result<()> {
        let entry = self
            .models
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", id))?;
        entry.settings = settings;
        self.save()
    }

    pub fn list_models(&self) -> &[LocalModelEntry] {
        &self.models
    }
}

/// Generate a unique ID for a model from its repo_id and quantization.
/// Uses the HuggingFace convention: `author/model:variant`.
pub fn model_id_from_repo(repo_id: &str, quantization: &str) -> String {
    format!("{}:{}", repo_id, quantization)
}

/// Resolve a legacy short model ID (e.g., "llama-3.2-1b") to the full HuggingFace-style ID.
/// Returns None if the ID is not a known legacy ID.
pub fn resolve_legacy_model_id(legacy_id: &str) -> Option<String> {
    LEGACY_MODELS
        .iter()
        .find(|m| m.id == legacy_id)
        .map(|m| model_id_from_repo(m.repo_id, m.quantization))
}

/// Generate a display name from repo_id and quantization.
pub fn display_name_from_repo(repo_id: &str, quantization: &str) -> String {
    let model_name = repo_id
        .split('/')
        .next_back()
        .unwrap_or(repo_id)
        .trim_end_matches("-GGUF")
        .trim_end_matches("-gguf");
    format!("{} ({})", model_name, quantization)
}
