use anyhow::Result;
use chrono::Utc;
use std::{fs, path::Path};

use crate::recipe::Recipe;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct RecipeManifest {
    pub name: String,
    pub recipe: Recipe,
    #[serde(rename = "isGlobal")]
    pub is_global: bool,
    #[serde(rename = "lastModified")]
    pub last_modified: String,
    #[serde(rename = "isArchived")]
    pub is_archived: bool,
}

impl RecipeManifest {
    pub fn from_yaml_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))?;
        let manifest = serde_yaml::from_str::<Self>(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?;
        Ok(manifest)
    }

    pub fn save_to_yaml_file(self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(&self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize YAML: {}", e))?;
        fs::write(path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write file {}: {}", path.display(), e))?;
        Ok(())
    }

    pub fn archive(file_path: &Path) -> Result<()> {
        let mut manifest = Self::from_yaml_file(file_path)?;
        manifest.is_archived = true;
        manifest.last_modified = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        manifest.save_to_yaml_file(file_path).unwrap();
        Ok(())
    }
}
