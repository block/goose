use std::collections::HashMap;
use std::fs;
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};

use goose::config::APP_STRATEGY;
use goose::recipe::read_recipe_file_content::read_recipe_file;
use goose::recipe::Recipe;

use std::path::Path;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const RECIPE_ID_HASH_SEED: u64 = 0x1234567890abcdef;

pub struct RecipeManifestWithPath {
    pub id: String,
    pub name: String,
    pub is_global: bool,
    pub recipe: Recipe,
    pub file_path: PathBuf,
    pub last_modified: String,
}

fn short_id_from_path(path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write_u64(RECIPE_ID_HASH_SEED);
    path.hash(&mut hasher);
    let h = hasher.finish();
    format!("{:016x}", h)
}

fn load_recipes_from_path(path: &PathBuf) -> Result<Vec<RecipeManifestWithPath>> {
    let mut recipe_manifests_with_path = Vec::new();
    if path.exists() {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            if path.extension() == Some("yaml".as_ref()) {
                let Ok(recipe_file) = read_recipe_file(path.clone()) else {
                    continue;
                };
                let Ok(recipe) = Recipe::from_content(&recipe_file.content) else {
                    continue;
                };
                let Ok(recipe_metadata) = RecipeManifestMetadata::from_yaml_file(&path) else {
                    continue;
                };
                let Ok(last_modified) = fs::metadata(path.clone()).map(|m| {
                    chrono::DateTime::<chrono::Utc>::from(m.modified().unwrap()).to_rfc3339()
                }) else {
                    continue;
                };

                let manifest_with_path = RecipeManifestWithPath {
                    id: short_id_from_path(recipe_file.file_path.to_string_lossy().as_ref()),
                    name: recipe_metadata.name,
                    is_global: recipe_metadata.is_global,
                    recipe,
                    file_path: recipe_file.file_path,
                    last_modified,
                };
                recipe_manifests_with_path.push(manifest_with_path);
            }
        }
    }
    Ok(recipe_manifests_with_path)
}

pub fn get_all_recipes_manifests() -> Result<Vec<RecipeManifestWithPath>> {
    let current_dir = std::env::current_dir()?;
    let local_recipe_path = current_dir.join(".goose/recipes");

    let global_recipe_path = choose_app_strategy(APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .config_dir()
        .join("recipes");

    let mut recipe_manifests_with_path = Vec::new();

    recipe_manifests_with_path.extend(load_recipes_from_path(&local_recipe_path)?);
    recipe_manifests_with_path.extend(load_recipes_from_path(&global_recipe_path)?);
    recipe_manifests_with_path.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(recipe_manifests_with_path)
}

fn get_recipe_temp_file_path() -> std::path::PathBuf {
    std::env::temp_dir().join("goose_recipe_file_map.json")
}

pub fn save_recipe_file_hash_map(hash_map: &HashMap<String, std::path::PathBuf>) -> Result<()> {
    let temp_path = get_recipe_temp_file_path();
    let json_data = serde_json::to_string(hash_map)
        .map_err(|e| anyhow::anyhow!("Failed to serialize hash map: {}", e))?;
    fs::write(temp_path, json_data)
        .map_err(|e| anyhow::anyhow!("Failed to write recipe id to file: {}", e))?;
    Ok(())
}

fn load_recipe_file_hash_map() -> Result<HashMap<String, std::path::PathBuf>> {
    let temp_path = get_recipe_temp_file_path();
    if !temp_path.exists() {
        return Ok(HashMap::new());
    }
    let json_data = fs::read_to_string(temp_path)
        .map_err(|e| anyhow::anyhow!("Failed to read recipe id to file: {}", e))?;
    let hash_map = serde_json::from_str(&json_data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize hash map: {}", e))?;
    Ok(hash_map)
}

pub fn find_recipe_file_path_by_id(recipe_id: &str) -> Result<PathBuf> {
    let recipe_file_hash_map = load_recipe_file_hash_map()?;
    recipe_file_hash_map
        .get(recipe_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Recipe not found with id: {}", recipe_id))
}

pub fn create_temp_deeplink_recipe_file(content: &str) -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let file_name = format!("recipe_deeplink_{}.yaml", uuid::Uuid::new_v4());
    let temp_path = temp_dir.join(file_name);

    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;

    Ok(temp_path)
}

// this is a temporary struct to deserilize the UI recipe files. should not be used for other purposes.
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
struct RecipeManifestMetadata {
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
