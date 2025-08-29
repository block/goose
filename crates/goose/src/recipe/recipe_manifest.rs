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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    fn create_test_manifest() -> RecipeManifest {
        RecipeManifest {
            name: "test_recipe".to_string(),
            recipe: Recipe::builder()
                .title("Test Recipe")
                .description("A test recipe")
                .instructions("Test instructions")
                .build()
                .unwrap(),
            is_global: false,
            last_modified: "2025-01-01T00:00:00.000Z".to_string(),
            is_archived: false,
        }
    }
    
    #[test]
    fn test_save_and_load_yaml_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_manifest.yml");
        
        let manifest = create_test_manifest();
        
        manifest.clone().save_to_yaml_file(&file_path).unwrap();
        
        let loaded_manifest = RecipeManifest::from_yaml_file(&file_path).unwrap();
        
        assert_eq!(loaded_manifest.name, manifest.name);
        assert_eq!(loaded_manifest.recipe.title, manifest.recipe.title);
        assert_eq!(loaded_manifest.is_global, manifest.is_global);
        assert_eq!(loaded_manifest.is_archived, manifest.is_archived);
    }
    
    #[test]
    fn test_archive() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_manifest.yml");
        
        let manifest = create_test_manifest();
        manifest.save_to_yaml_file(&file_path).unwrap();
        
        RecipeManifest::archive(&file_path).unwrap();
        
        let archived_manifest = RecipeManifest::from_yaml_file(&file_path).unwrap();
        
        assert!(archived_manifest.is_archived);
    }
    
    #[test]
    fn test_from_yaml_file_nonexistent() {
        let result = RecipeManifest::from_yaml_file(&std::path::Path::new("nonexistent.yml"));
        assert!(result.is_err());
    }
}
