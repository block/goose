use std::fs;
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use anyhow::Result;

use goose::recipe::local_recipes::list_local_recipes;
use goose::recipe::Recipe;

use std::path::Path;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub struct RecipeManifestWithPath {
    pub id: String,
    pub name: String,
    pub recipe: Recipe,
    pub file_path: PathBuf,
    pub last_modified: String,
}

fn short_id_from_path(path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let h = hasher.finish();
    format!("{:016x}", h)
}

pub fn get_all_recipes_manifests() -> Result<Vec<RecipeManifestWithPath>> {
    let recipes_with_path = list_local_recipes()?;
    let mut recipe_manifests_with_path = Vec::new();
    for (file_path, recipe) in recipes_with_path {
        let Ok(last_modified) = fs::metadata(file_path.clone())
            .map(|m| chrono::DateTime::<chrono::Utc>::from(m.modified().unwrap()).to_rfc3339())
        else {
            continue;
        };
        let recipe_metadata =
            RecipeManifestMetadata::from_yaml_file(&file_path).unwrap_or_else(|_| {
                RecipeManifestMetadata {
                    name: recipe.title.clone(),
                }
            });

        let manifest_with_path = RecipeManifestWithPath {
            id: short_id_from_path(file_path.to_string_lossy().as_ref()),
            name: recipe_metadata.name,
            recipe,
            file_path,
            last_modified,
        };
        recipe_manifests_with_path.push(manifest_with_path);
    }
    recipe_manifests_with_path.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(recipe_manifests_with_path)
}

// this is a temporary struct to deserilize the UI recipe files. should not be used for other purposes.
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
struct RecipeManifestMetadata {
    pub name: String,
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
    }
}
