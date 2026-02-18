use chrono::{DateTime, Utc};
use crate::config::paths::Paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::download_task::DownloadError;
use super::model_id::ModelIdentifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_id: String,
    pub organization: String,
    pub model_name: String,
    pub variant: Option<String>,
    pub file_name: String,
    pub file_size: u64,
    pub checksum: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub downloaded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelInfo {
    pub model_id: String,
    pub normalized_id: String,
    pub file_path: PathBuf,
    pub file_size: u64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub downloaded_at: DateTime<Utc>,
    pub checksum: Option<String>,
    pub metadata: ModelMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetadataFile {
    version: String,
    models: HashMap<String, ModelMetadata>,
}

impl Default for MetadataFile {
    fn default() -> Self {
        Self {
            version: "1".to_string(),
            models: HashMap::new(),
        }
    }
}

pub struct StorageManager {
    models_dir: PathBuf,
    metadata_path: PathBuf,
    metadata: Arc<RwLock<HashMap<String, ModelMetadata>>>,
}

impl StorageManager {
    pub fn new() -> Result<Self, DownloadError> {
        let models_dir = Paths::in_data_dir("models");
        let metadata_path = models_dir.join("metadata.json");

        // Create models directory if it doesn't exist
        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        // Load existing metadata
        let metadata = Self::load_metadata(&metadata_path)?;

        Ok(Self {
            models_dir,
            metadata_path,
            metadata: Arc::new(RwLock::new(metadata)),
        })
    }

    fn load_metadata(path: &PathBuf) -> Result<HashMap<String, ModelMetadata>, DownloadError> {
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let contents = fs::read_to_string(path)?;
        let metadata_file: MetadataFile = serde_json::from_str(&contents)
            .map_err(|e| DownloadError::MetadataError(format!("Failed to parse metadata: {}", e)))?;

        Ok(metadata_file.models)
    }

    fn save_metadata(&self, metadata: &HashMap<String, ModelMetadata>) -> Result<(), DownloadError> {
        let metadata_file = MetadataFile {
            version: "1".to_string(),
            models: metadata.clone(),
        };

        let json = serde_json::to_string_pretty(&metadata_file)
            .map_err(|e| DownloadError::MetadataError(format!("Failed to serialize metadata: {}", e)))?;

        // Write to temporary file first
        let temp_path = self.metadata_path.with_extension("tmp");
        let mut file = File::create(&temp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;

        // Atomic rename
        fs::rename(&temp_path, &self.metadata_path)?;

        Ok(())
    }

    pub async fn model_exists(&self, normalized_id: &str) -> bool {
        self.metadata.read().await.contains_key(normalized_id)
    }

    pub async fn get_model_metadata(&self, normalized_id: &str) -> Option<ModelMetadata> {
        self.metadata.read().await.get(normalized_id).cloned()
    }

    pub async fn list_models(&self) -> Vec<LocalModelInfo> {
        let metadata = self.metadata.read().await;
        metadata
            .iter()
            .map(|(normalized_id, meta)| {
                let file_path = self.get_model_path(normalized_id);
                LocalModelInfo {
                    model_id: meta.model_id.clone(),
                    normalized_id: normalized_id.clone(),
                    file_path,
                    file_size: meta.file_size,
                    downloaded_at: meta.downloaded_at,
                    checksum: meta.checksum.clone(),
                    metadata: meta.clone(),
                }
            })
            .collect()
    }

    pub async fn add_model_metadata(
        &self,
        identifier: &ModelIdentifier,
        file_path: &PathBuf,
        checksum: Option<String>,
    ) -> Result<(), DownloadError> {
        let file_size = fs::metadata(file_path)?.len();

        let metadata = ModelMetadata {
            model_id: identifier.original.clone(),
            organization: identifier.organization.clone(),
            model_name: identifier.model_name.clone(),
            variant: identifier.variant.clone(),
            file_name: file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("model.gguf")
                .to_string(),
            file_size,
            checksum,
            downloaded_at: Utc::now(),
        };

        let mut meta_map = self.metadata.write().await;
        meta_map.insert(identifier.normalized.clone(), metadata);
        self.save_metadata(&meta_map)?;

        Ok(())
    }

    pub async fn remove_model_metadata(&self, normalized_id: &str) -> Result<(), DownloadError> {
        let mut meta_map = self.metadata.write().await;
        meta_map.remove(normalized_id);
        self.save_metadata(&meta_map)?;
        Ok(())
    }

    pub fn get_model_path(&self, normalized_id: &str) -> PathBuf {
        self.models_dir.join(format!("{}.gguf", normalized_id))
    }

    pub fn get_partial_path(&self, normalized_id: &str) -> PathBuf {
        self.models_dir.join(format!("{}.gguf.partial", normalized_id))
    }

    pub fn partial_exists(&self, normalized_id: &str) -> bool {
        self.get_partial_path(normalized_id).exists()
    }

    pub fn get_partial_size(&self, normalized_id: &str) -> Result<u64, std::io::Error> {
        let path = self.get_partial_path(normalized_id);
        Ok(fs::metadata(path)?.len())
    }

    pub fn delete_partial(&self, normalized_id: &str) -> Result<(), std::io::Error> {
        let path = self.get_partial_path(normalized_id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn finalize_download(&self, normalized_id: &str) -> Result<PathBuf, std::io::Error> {
        let partial_path = self.get_partial_path(normalized_id);
        let final_path = self.get_model_path(normalized_id);
        fs::rename(partial_path, &final_path)?;
        Ok(final_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization() {
        let mut models = HashMap::new();
        models.insert(
            "test_model".to_string(),
            ModelMetadata {
                model_id: "org/model".to_string(),
                organization: "org".to_string(),
                model_name: "model".to_string(),
                variant: Some("variant".to_string()),
                file_name: "model.gguf".to_string(),
                file_size: 1024,
                checksum: Some("abc123".to_string()),
                downloaded_at: Utc::now(),
            },
        );

        let metadata_file = MetadataFile {
            version: "1".to_string(),
            models,
        };

        let json = serde_json::to_string(&metadata_file).unwrap();
        let deserialized: MetadataFile = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, "1");
        assert_eq!(deserialized.models.len(), 1);
    }

    #[test]
    fn test_get_model_path() {
        let storage = StorageManager::new().unwrap();
        let path = storage.get_model_path("test_model");
        assert!(path.to_string_lossy().contains("models"));
        assert!(path.to_string_lossy().ends_with("test_model.gguf"));
    }

    #[test]
    fn test_get_partial_path() {
        let storage = StorageManager::new().unwrap();
        let path = storage.get_partial_path("test_model");
        assert!(path.to_string_lossy().contains("models"));
        assert!(path.to_string_lossy().ends_with("test_model.gguf.partial"));
    }
}
