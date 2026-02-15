use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::recipe::Recipe;
use crate::registry::manifest::{
    AgentDependency, AgentDetail, AuthorInfo, RecipeDetail, RegistryEntry, RegistryEntryDetail,
    RegistryEntryKind,
};

/// Generate a RegistryEntry from a Recipe
pub fn recipe_to_registry_entry(recipe: &Recipe) -> RegistryEntry {
    let extension_names: Vec<String> = recipe
        .extensions
        .as_ref()
        .map(|exts| exts.iter().map(|ext| ext.name()).collect())
        .unwrap_or_default();

    let parameters: Vec<String> = recipe
        .parameters
        .as_ref()
        .map(|params| params.iter().map(|p| p.key.clone()).collect())
        .unwrap_or_default();

    let author = recipe.author.as_ref().map(|a| AuthorInfo {
        name: a.contact.clone(),
        contact: a.metadata.clone(),
        url: None,
    });

    RegistryEntry {
        name: recipe.title.clone(),
        kind: RegistryEntryKind::Recipe,
        description: recipe.description.clone(),
        version: Some(recipe.version.clone()),
        author,
        tags: Vec::new(),
        detail: RegistryEntryDetail::Recipe(RecipeDetail {
            instructions: recipe.instructions.clone(),
            prompt: recipe.prompt.clone(),
            extension_names,
            parameters,
        }),
        ..Default::default()
    }
}

/// Generate a publishable agent manifest RegistryEntry.
///
/// This creates a complete agent entry with all fields needed for publishing
/// to a registry, including dependencies on required MCP extensions.
pub fn generate_agent_manifest(
    name: &str,
    description: &str,
    instructions: &str,
    model: Option<&str>,
    required_extensions: Vec<String>,
) -> RegistryEntry {
    let dependencies: Vec<AgentDependency> = required_extensions
        .iter()
        .map(|ext| AgentDependency {
            dep_type: RegistryEntryKind::Tool,
            name: ext.clone(),
            version: None,
            required: true,
        })
        .collect();

    RegistryEntry {
        name: name.to_string(),
        kind: RegistryEntryKind::Agent,
        description: description.to_string(),
        version: Some("0.1.0".to_string()),
        detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
            instructions: instructions.to_string(),
            model: model.map(String::from),
            recommended_models: model.into_iter().map(String::from).collect(),
            capabilities: Vec::new(),
            domains: Vec::new(),
            input_content_types: vec!["text/plain".into()],
            output_content_types: vec!["text/markdown".into()],
            required_extensions: required_extensions.clone(),
            dependencies,
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// Generate a publishable agent manifest from a Recipe.
///
/// Extracts extension names as dependencies and maps recipe metadata
/// to agent manifest fields.
pub fn recipe_to_agent_manifest(recipe: &Recipe) -> RegistryEntry {
    let extension_names: Vec<String> = recipe
        .extensions
        .as_ref()
        .map(|exts| exts.iter().map(|ext| ext.name()).collect())
        .unwrap_or_default();

    let model = recipe
        .settings
        .as_ref()
        .and_then(|s| s.goose_model.as_deref())
        .map(String::from);

    let author = recipe.author.as_ref().map(|a| AuthorInfo {
        name: a.contact.clone(),
        contact: a.metadata.clone(),
        url: None,
    });

    let dependencies: Vec<AgentDependency> = extension_names
        .iter()
        .map(|ext| AgentDependency {
            dep_type: RegistryEntryKind::Tool,
            name: ext.clone(),
            version: None,
            required: true,
        })
        .collect();

    RegistryEntry {
        name: recipe.title.clone(),
        kind: RegistryEntryKind::Agent,
        description: recipe.description.clone(),
        version: Some(recipe.version.clone()),
        author,
        tags: Vec::new(),
        detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
            instructions: recipe.instructions.clone().unwrap_or_default(),
            model,
            recommended_models: Vec::new(),
            capabilities: Vec::new(),
            domains: Vec::new(),
            input_content_types: vec!["text/plain".into()],
            output_content_types: vec!["text/markdown".into()],
            required_extensions: extension_names,
            dependencies,
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// Validate a manifest file at the given path
pub fn validate_manifest(path: &Path) -> Result<RegistryEntry> {
    let content = std::fs::read_to_string(path)?;

    let entry: RegistryEntry = if path.extension().is_some_and(|e| e == "json") {
        serde_json::from_str(&content)?
    } else {
        serde_yaml::from_str(&content)?
    };

    if entry.name.is_empty() {
        bail!("Manifest name is required");
    }

    Ok(entry)
}

/// Validate a manifest is ready for publishing and return any issues found.
pub fn validate_for_publish(path: &Path) -> Result<Vec<String>> {
    let entry = validate_manifest(path)?;
    Ok(entry.validate_for_publish())
}

/// Write a manifest to disk
pub fn write_manifest(entry: &RegistryEntry, path: &Path) -> Result<PathBuf> {
    let content = if path.extension().is_some_and(|e| e == "json") {
        serde_json::to_string_pretty(entry)?
    } else {
        serde_yaml::to_string(entry)?
    };

    std::fs::write(path, &content)?;
    Ok(path.to_path_buf())
}

/// Initialize a publishable agent manifest in the given directory
pub fn init_manifest(dir: &Path, name: &str, description: &str) -> Result<PathBuf> {
    let manifest_path = dir.join("agent.yaml");
    if manifest_path.exists() {
        bail!("agent.yaml already exists in {}", dir.display());
    }

    let entry = generate_agent_manifest(
        name,
        description,
        "You are a helpful AI agent.",
        None,
        vec!["developer".into()],
    );
    write_manifest(&entry, &manifest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::extension::ExtensionConfig;
    use crate::recipe::Recipe;

    fn test_recipe() -> Recipe {
        Recipe {
            title: "Test Recipe".to_string(),
            description: "A test recipe".to_string(),
            version: "1.0.0".to_string(),
            extensions: Some(vec![ExtensionConfig::Builtin {
                name: "developer".to_string(),
                display_name: None,
                description: String::new(),
                timeout: None,
                bundled: None,
                available_tools: Vec::new(),
            }]),
            instructions: Some("Do the thing".into()),
            prompt: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
            retry: None,
        }
    }

    #[test]
    fn test_recipe_to_registry_entry() {
        let recipe = test_recipe();
        let entry = recipe_to_registry_entry(&recipe);

        assert_eq!(entry.name, "Test Recipe");
        assert_eq!(entry.kind, RegistryEntryKind::Recipe);
        assert_eq!(entry.version, Some("1.0.0".to_string()));

        if let RegistryEntryDetail::Recipe(detail) = &entry.detail {
            assert_eq!(detail.extension_names, vec!["developer"]);
        } else {
            panic!("Expected RecipeDetail");
        }
    }

    #[test]
    fn test_generate_agent_manifest() {
        let entry = generate_agent_manifest(
            "my-agent",
            "Does things",
            "You are a helpful agent.",
            Some("claude-sonnet-4"),
            vec!["developer".into(), "memory".into()],
        );

        assert_eq!(entry.name, "my-agent");
        assert_eq!(entry.kind, RegistryEntryKind::Agent);
        assert_eq!(entry.version, Some("0.1.0".to_string()));

        if let RegistryEntryDetail::Agent(detail) = &entry.detail {
            assert_eq!(detail.instructions, "You are a helpful agent.");
            assert_eq!(detail.model, Some("claude-sonnet-4".into()));
            assert_eq!(detail.recommended_models, vec!["claude-sonnet-4"]);
            assert_eq!(detail.required_extensions, vec!["developer", "memory"]);
            assert_eq!(detail.dependencies.len(), 2);
            assert_eq!(detail.dependencies[0].name, "developer");
            assert!(detail.dependencies[0].required);
        } else {
            panic!("Expected AgentDetail");
        }
    }

    #[test]
    fn test_recipe_to_agent_manifest() {
        let recipe = test_recipe();
        let entry = recipe_to_agent_manifest(&recipe);

        assert_eq!(entry.name, "Test Recipe");
        assert_eq!(entry.kind, RegistryEntryKind::Agent);

        if let RegistryEntryDetail::Agent(detail) = &entry.detail {
            assert_eq!(detail.instructions, "Do the thing");
            assert_eq!(detail.required_extensions, vec!["developer"]);
            assert_eq!(detail.dependencies.len(), 1);
            assert_eq!(detail.dependencies[0].dep_type, RegistryEntryKind::Tool);
            assert_eq!(detail.dependencies[0].name, "developer");
        } else {
            panic!("Expected AgentDetail");
        }
    }

    #[test]
    fn test_validate_manifest_roundtrip() {
        let entry = generate_agent_manifest(
            "test",
            "A test agent",
            "You are a test agent.",
            None,
            vec!["developer".into()],
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.yaml");

        write_manifest(&entry, &path).unwrap();
        let loaded = validate_manifest(&path).unwrap();

        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.kind, RegistryEntryKind::Agent);

        if let RegistryEntryDetail::Agent(detail) = &loaded.detail {
            assert_eq!(detail.required_extensions, vec!["developer"]);
        } else {
            panic!("Expected AgentDetail");
        }
    }

    #[test]
    fn test_validate_for_publish() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.yaml");

        let mut entry = generate_agent_manifest(
            "my-agent",
            "Does things",
            "You are helpful.",
            Some("claude-sonnet-4"),
            vec!["developer".into()],
        );
        entry.license = Some("Apache-2.0".into());
        entry.author = Some(AuthorInfo {
            name: Some("Test Author".into()),
            contact: None,
            url: None,
        });
        if let RegistryEntryDetail::Agent(ref mut detail) = entry.detail {
            detail.capabilities = vec!["coding".into()];
        }

        write_manifest(&entry, &path).unwrap();

        let issues = validate_for_publish(&path).unwrap();
        assert!(
            issues.is_empty(),
            "Expected no issues but got: {:?}",
            issues
        );
    }

    #[test]
    fn test_validate_for_publish_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.yaml");

        let entry = RegistryEntry {
            name: "bare-agent".into(),
            kind: RegistryEntryKind::Agent,
            detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                instructions: String::new(),
                model: None,
                recommended_models: vec![],
                capabilities: vec![],
                domains: vec![],
                input_content_types: vec![],
                output_content_types: vec![],
                required_extensions: vec![],
                dependencies: vec![],
                ..Default::default()
            })),
            ..Default::default()
        };

        write_manifest(&entry, &path).unwrap();
        let issues = validate_for_publish(&path).unwrap();

        assert!(issues.iter().any(|i| i.contains("description")));
        assert!(issues.iter().any(|i| i.contains("version")));
        assert!(issues.iter().any(|i| i.contains("instructions")));
    }

    #[test]
    fn test_init_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = init_manifest(dir.path(), "my-project", "My project agent").unwrap();

        assert!(path.exists());
        let entry = validate_manifest(&path).unwrap();
        assert_eq!(entry.name, "my-project");

        if let RegistryEntryDetail::Agent(detail) = &entry.detail {
            assert_eq!(detail.required_extensions, vec!["developer"]);
            assert_eq!(detail.dependencies.len(), 1);
        } else {
            panic!("Expected AgentDetail");
        }
    }

    #[test]
    fn test_init_manifest_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        init_manifest(dir.path(), "first", "First").unwrap();
        let result = init_manifest(dir.path(), "second", "Second");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_name_fails() {
        let entry = RegistryEntry {
            name: String::new(),
            kind: RegistryEntryKind::Agent,
            ..Default::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.yaml");
        write_manifest(&entry, &path).unwrap();
        let result = validate_manifest(&path);
        assert!(result.is_err());
    }
}
