use include_dir::{include_dir, Dir};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::Recipe;

static BUNDLED_SUBRECIPES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/prompts/subrecipes");

#[derive(Debug, Clone)]
pub struct BundledSubrecipe {
    pub name: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub parameters: Vec<(String, String)>,
}

pub static BUNDLED_SUBRECIPES: Lazy<HashMap<String, BundledSubrecipe>> = Lazy::new(|| {
    let mut recipes = HashMap::new();

    for file in BUNDLED_SUBRECIPES_DIR.files() {
        let path = file.path();
        let extension = path.extension().and_then(|e| e.to_str());
        if !matches!(extension, Some("yaml") | Some("yml")) {
            continue;
        }

        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let content = match std::str::from_utf8(file.contents()) {
            Ok(c) => c.to_string(),
            Err(e) => {
                tracing::warn!("Failed to read bundled subrecipe '{}': {}", name, e);
                continue;
            }
        };

        let recipe = match Recipe::from_content(&content) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to parse bundled subrecipe '{}': {}", name, e);
                continue;
            }
        };

        let parameters = recipe
            .parameters
            .as_ref()
            .map(|params| {
                params
                    .iter()
                    .map(|p| (p.key.clone(), p.description.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let bundled = BundledSubrecipe {
            name: name.clone(),
            title: recipe.title,
            description: recipe.description,
            content,
            parameters,
        };

        recipes.insert(name, bundled);
    }

    recipes
});

pub fn get_bundled_subrecipe(name: &str) -> Option<&'static BundledSubrecipe> {
    BUNDLED_SUBRECIPES.get(name)
}

pub fn list_bundled_subrecipes() -> Vec<&'static BundledSubrecipe> {
    BUNDLED_SUBRECIPES.values().collect()
}

pub fn bundled_subrecipe_names() -> Vec<&'static str> {
    BUNDLED_SUBRECIPES.keys().map(|s| s.as_str()).collect()
}

pub fn is_bundled_subrecipe(name: &str) -> bool {
    BUNDLED_SUBRECIPES.contains_key(name)
}

pub fn build_recipe_from_bundled(
    name: &str,
    parameters: &HashMap<String, String>,
) -> anyhow::Result<Recipe> {
    let bundled = get_bundled_subrecipe(name)
        .ok_or_else(|| anyhow::anyhow!("Bundled subrecipe '{}' not found", name))?;

    crate::recipe::build_recipe::build_recipe_from_template(
        bundled.content.clone(),
        &std::path::PathBuf::new(),
        parameters
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        None::<fn(&str, &str) -> Result<String, anyhow::Error>>,
    )
    .map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_subrecipes_load() {
        let recipes = list_bundled_subrecipes();
        assert!(!recipes.is_empty());
    }

    #[test]
    fn test_investigator_exists() {
        let investigator = get_bundled_subrecipe("investigator");
        assert!(investigator.is_some());

        let investigator = investigator.unwrap();
        assert_eq!(investigator.name, "investigator");
        assert!(!investigator.title.is_empty());
        assert!(!investigator.description.is_empty());
        assert!(!investigator.parameters.is_empty());
    }

    #[test]
    fn test_is_bundled_subrecipe() {
        assert!(is_bundled_subrecipe("investigator"));
        assert!(!is_bundled_subrecipe("nonexistent_recipe"));
    }

    #[test]
    fn test_build_recipe_from_bundled() {
        let mut params = HashMap::new();
        params.insert(
            "objective".to_string(),
            "Find all usages of the foo function".to_string(),
        );

        let recipe = build_recipe_from_bundled("investigator", &params);
        assert!(recipe.is_ok(), "Should build recipe: {:?}", recipe.err());

        let recipe = recipe.unwrap();
        assert!(recipe.instructions.is_some());
        let instructions = recipe.instructions.unwrap();
        assert!(instructions.contains("Find all usages of the foo function"));
    }

    #[test]
    fn test_bundled_subrecipe_names() {
        let names = bundled_subrecipe_names();
        assert!(names.contains(&"investigator"));
    }
}
