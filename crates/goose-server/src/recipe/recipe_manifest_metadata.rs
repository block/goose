use anyhow::Result;
use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct RecipeManifestMetadata {
    pub name: String,
    #[serde(rename = "isGlobal")]
    pub is_global: bool,
}

impl RecipeManifestMetadata {
    pub fn from_yaml_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))?;
        let metadata = serde_yaml::from_str::<Self>(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?;
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_from_yaml_file_success() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_recipe.yaml");

        let yaml_content = r#"
name: "Test Recipe"
isGlobal: true
recipe: recipe_content
"#;

        fs::write(&file_path, yaml_content).unwrap();

        let result = RecipeManifestMetadata::from_yaml_file(&file_path).unwrap();

        assert_eq!(result.name, "Test Recipe");
        assert_eq!(result.is_global, true);
    }
}
