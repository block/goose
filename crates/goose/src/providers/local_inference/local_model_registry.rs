use crate::config::paths::Paths;
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
        }
    }
}

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
}

impl LocalModelEntry {
    pub fn is_downloaded(&self) -> bool {
        self.local_path.exists()
    }

    pub fn file_size(&self) -> u64 {
        std::fs::metadata(&self.local_path)
            .map(|m| m.len())
            .unwrap_or(0)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LocalModelRegistry {
    pub models: Vec<LocalModelEntry>,
}

fn registry_path() -> PathBuf {
    Paths::in_data_dir("models/registry.json")
}

/// The 4 legacy hardcoded model definitions for migration.
struct LegacyModel {
    id: &'static str,
    display_name: &'static str,
    repo_id: &'static str,
    filename: &'static str,
    quantization: &'static str,
    source_url: &'static str,
}

const LEGACY_MODELS: &[LegacyModel] = &[
    LegacyModel {
        id: "llama-3.2-1b",
        display_name: "Llama 3.2 1B Instruct",
        repo_id: "bartowski/Llama-3.2-1B-Instruct-GGUF",
        filename: "Llama-3.2-1B-Instruct-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
    },
    LegacyModel {
        id: "llama-3.2-3b",
        display_name: "Llama 3.2 3B Instruct",
        repo_id: "bartowski/Llama-3.2-3B-Instruct-GGUF",
        filename: "Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
    },
    LegacyModel {
        id: "hermes-2-pro-7b",
        display_name: "Hermes 2 Pro Llama-3 7B",
        repo_id: "NousResearch/Hermes-2-Pro-Llama-3-8B-GGUF",
        filename: "Hermes-2-Pro-Llama-3-8B-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/NousResearch/Hermes-2-Pro-Llama-3-8B-GGUF/resolve/main/Hermes-2-Pro-Llama-3-8B-Q4_K_M.gguf",
    },
    LegacyModel {
        id: "mistral-small-22b",
        display_name: "Mistral Small 22B Instruct",
        repo_id: "bartowski/Mistral-Small-Instruct-2409-GGUF",
        filename: "Mistral-Small-Instruct-2409-Q4_K_M.gguf",
        quantization: "Q4_K_M",
        source_url: "https://huggingface.co/bartowski/Mistral-Small-Instruct-2409-GGUF/resolve/main/Mistral-Small-Instruct-2409-Q4_K_M.gguf",
    },
];

impl LocalModelRegistry {
    pub fn load() -> Result<Self> {
        let path = registry_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        let registry: Self = serde_json::from_str(&data)?;
        Ok(registry)
    }

    pub fn save(&self) -> Result<()> {
        let path = registry_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Migrate model IDs from the old `author--repo--variant` format to the
    /// HuggingFace-style `author/repo:variant` format. Only touches IDs that
    /// contain `--` (the old separator); legacy featured models with short IDs
    /// like `llama-3.2-1b` are left alone.
    pub fn migrate_model_ids(&mut self) {
        let mut changed = false;
        for entry in &mut self.models {
            if entry.id.contains("--") {
                let new_id = model_id_from_repo(&entry.repo_id, &entry.quantization);
                if entry.id != new_id {
                    entry.id = new_id;
                    changed = true;
                }
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
                });
                changed = true;
            }
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
