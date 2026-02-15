use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;

use crate::registry::manifest::{
    AgentDetail, AuthorInfo, RecipeDetail, RegistryEntry, RegistryEntryDetail, RegistryEntryKind,
    SkillDetail,
};
use crate::registry::source::RegistrySource;

/// Scans local filesystem directories for registry entries.
///
/// Follows the same directory conventions as `summon_extension.rs`:
/// - Skills: `{root}/skills/{name}/SKILL.md`
/// - Agents: `{root}/agents/{name}.md`
/// - Recipes: `{root}/recipes/{name}.yaml` or `{name}.yml`
/// - Tools: read from goose config extensions
pub struct LocalRegistrySource {
    roots: Vec<PathBuf>,
}

impl LocalRegistrySource {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    /// Create a source with the default user config directory.
    pub fn from_default_paths() -> Result<Self> {
        let mut roots = Vec::new();

        if let Some(config_dir) = dirs::config_dir() {
            let goose_dir = config_dir.join("goose");
            if goose_dir.exists() {
                roots.push(goose_dir);
            }
        }

        Ok(Self { roots })
    }

    fn scan_all(&self) -> Vec<RegistryEntry> {
        let mut entries = Vec::new();
        for root in &self.roots {
            entries.extend(scan_skills(root));
            entries.extend(scan_agents(root));
            entries.extend(scan_recipes(root));
        }
        entries
    }
}

#[async_trait]
impl RegistrySource for LocalRegistrySource {
    fn name(&self) -> &str {
        "local"
    }

    async fn search(
        &self,
        query: Option<&str>,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Vec<RegistryEntry>> {
        let entries = self.scan_all();

        Ok(entries
            .into_iter()
            .filter(|e| kind.is_none() || kind == Some(e.kind))
            .filter(|e| {
                let Some(q) = query else { return true };
                let q_lower = q.to_lowercase();
                e.name.to_lowercase().contains(&q_lower)
                    || e.description.to_lowercase().contains(&q_lower)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
            })
            .collect())
    }

    async fn get(
        &self,
        name: &str,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Option<RegistryEntry>> {
        let entries = self.scan_all();
        Ok(entries
            .into_iter()
            .find(|e| kind.is_none_or(|k| e.kind == k) && e.name == name))
    }
}

fn scan_skills(root: &Path) -> Vec<RegistryEntry> {
    let skills_dir = root.join("skills");
    if !skills_dir.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let read_dir = match std::fs::read_dir(&skills_dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_file = path.join("SKILL.md");
        if !skill_file.is_file() {
            continue;
        }
        if let Some(entry) = parse_skill_file(&skill_file) {
            entries.push(entry);
        }
    }
    entries
}

fn scan_agents(root: &Path) -> Vec<RegistryEntry> {
    let agents_dir = root.join("agents");
    if !agents_dir.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let read_dir = match std::fs::read_dir(&agents_dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Some(entry) = parse_agent_file(&path) {
            entries.push(entry);
        }
    }
    entries
}

fn scan_recipes(root: &Path) -> Vec<RegistryEntry> {
    let recipes_dir = root.join("recipes");
    if !recipes_dir.is_dir() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let read_dir = match std::fs::read_dir(&recipes_dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }
        if let Some(entry) = parse_recipe_file(&path) {
            entries.push(entry);
        }
    }
    entries
}

/// Parse a SKILL.md file with YAML frontmatter (name, description) + markdown body.
fn parse_skill_file(path: &Path) -> Option<RegistryEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let (meta, body) = parse_frontmatter::<SkillFrontmatter>(&content)?;

    Some(RegistryEntry {
        name: meta.name,
        kind: RegistryEntryKind::Skill,
        description: meta.description,
        version: None,
        author: None,
        license: None,
        repository: None,
        icon: None,
        source_uri: None,
        local_path: Some(path.to_path_buf()),
        tags: meta.tags.unwrap_or_default(),
        detail: RegistryEntryDetail::Skill(SkillDetail {
            content: body,
            builtin: false,
        }),
        metadata: HashMap::new(),
    })
}

/// Parse an agent .md file with YAML frontmatter (name, description, model) + instructions.
fn parse_agent_file(path: &Path) -> Option<RegistryEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let (meta, body) = parse_frontmatter::<AgentFrontmatter>(&content)?;

    let description = meta.description.unwrap_or_else(|| {
        meta.model
            .as_ref()
            .map(|m| format!("Agent ({})", m))
            .unwrap_or_else(|| "Agent".into())
    });

    Some(RegistryEntry {
        name: meta.name,
        kind: RegistryEntryKind::Agent,
        description,
        version: meta.version,
        author: meta.author.map(|name| AuthorInfo {
            name: Some(name),
            ..Default::default()
        }),
        license: meta.license,
        repository: meta.repository,
        icon: meta.icon,
        source_uri: None,
        local_path: Some(path.to_path_buf()),
        tags: meta.tags.unwrap_or_default(),
        detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
            instructions: body,
            model: meta.model,
            capabilities: meta.capabilities.unwrap_or_default(),
            domains: meta.domains.unwrap_or_default(),
            required_extensions: meta.required_extensions.unwrap_or_default(),
            input_content_types: vec!["text/plain".into()],
            output_content_types: vec!["text/markdown".into()],
            ..Default::default()
        })),
        metadata: HashMap::new(),
    })
}

/// Parse a recipe .yaml file using Goose's Recipe struct fields.
fn parse_recipe_file(path: &Path) -> Option<RegistryEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let mapping = value.as_mapping()?;

    let yaml_key = |k: &str| serde_yaml::Value::String(k.into());

    let title = mapping
        .get(yaml_key("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let description = mapping
        .get(yaml_key("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let instructions = mapping
        .get(yaml_key("instructions"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let prompt = mapping
        .get(yaml_key("prompt"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let extension_names: Vec<String> = mapping
        .get(yaml_key("extensions"))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|ext| {
                    ext.as_mapping()
                        .and_then(|m| m.get(yaml_key("name")))
                        .and_then(|n| n.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    let parameters: Vec<String> = mapping
        .get(yaml_key("parameters"))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|p| {
                    p.as_mapping()
                        .and_then(|m| m.get(yaml_key("key")))
                        .and_then(|k| k.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    let author = mapping
        .get(yaml_key("author"))
        .and_then(|v| v.as_mapping())
        .map(|m| AuthorInfo {
            name: m
                .get(yaml_key("contact"))
                .and_then(|v| v.as_str())
                .map(String::from),
            contact: m
                .get(yaml_key("contact"))
                .and_then(|v| v.as_str())
                .map(String::from),
            url: None,
        });

    let version = mapping
        .get(yaml_key("version"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let display_name = if title.is_empty() {
        name.clone()
    } else {
        title
    };

    Some(RegistryEntry {
        name: display_name,
        kind: RegistryEntryKind::Recipe,
        description,
        version,
        author,
        license: None,
        repository: None,
        icon: None,
        source_uri: None,
        local_path: Some(path.to_path_buf()),
        tags: Vec::new(),
        detail: RegistryEntryDetail::Recipe(RecipeDetail {
            instructions,
            prompt,
            extension_names,
            parameters,
        }),
        metadata: HashMap::new(),
    })
}

/// Minimal frontmatter types for local file parsing.
#[derive(serde::Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct AgentFrontmatter {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    domains: Option<Vec<String>>,
    #[serde(default)]
    required_extensions: Option<Vec<String>>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    author: Option<String>,
}

/// Parse YAML frontmatter delimited by `---` from a markdown file.
fn parse_frontmatter<T: for<'de> serde::Deserialize<'de>>(content: &str) -> Option<(T, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = trimmed.get(3..)?;
    let end_pos = after_first.find("---")?;
    let yaml_content = after_first.get(..end_pos)?.trim();
    let body = after_first.get(end_pos + 3..)?.trim().to_string();

    let metadata: T = serde_yaml::from_str(yaml_content).ok()?;
    Some((metadata, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_skill_frontmatter() {
        let content = r#"---
name: test-skill
description: A test skill
tags:
  - testing
---
When the user asks about testing, do X.
"#;
        let (meta, body) = parse_frontmatter::<SkillFrontmatter>(content).unwrap();
        assert_eq!(meta.name, "test-skill");
        assert_eq!(meta.description, "A test skill");
        assert_eq!(meta.tags, Some(vec!["testing".into()]));
        assert!(body.contains("When the user asks about testing"));
    }

    #[test]
    fn parse_agent_frontmatter() {
        let content = r#"---
name: code-helper
description: Helps with code
model: claude-sonnet-4
---
You are a helpful coding assistant.
"#;
        let (meta, body) = parse_frontmatter::<AgentFrontmatter>(content).unwrap();
        assert_eq!(meta.name, "code-helper");
        assert_eq!(meta.description, Some("Helps with code".into()));
        assert_eq!(meta.model, Some("claude-sonnet-4".into()));
        assert!(body.contains("helpful coding assistant"));
    }

    #[test]
    fn parse_frontmatter_returns_none_without_delimiters() {
        let content = "No frontmatter here";
        let result = parse_frontmatter::<SkillFrontmatter>(content);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn local_source_scans_skills() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skills").join("my-skill");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("SKILL.md"),
            "---
name: my-skill
description: A skill
---
Do the thing.
",
        )
        .unwrap();

        let source = LocalRegistrySource::new(vec![tmp.path().to_path_buf()]);
        let results = source
            .search(None, Some(RegistryEntryKind::Skill))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "my-skill");
        assert_eq!(results[0].kind, RegistryEntryKind::Skill);
    }

    #[tokio::test]
    async fn local_source_scans_agents() {
        let tmp = TempDir::new().unwrap();
        let agents_dir = tmp.path().join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("reviewer.md"),
            "---
name: reviewer
description: Reviews code
model: gpt-4o
---
You review code.
",
        )
        .unwrap();

        let source = LocalRegistrySource::new(vec![tmp.path().to_path_buf()]);
        let results = source
            .search(None, Some(RegistryEntryKind::Agent))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "reviewer");
    }

    #[tokio::test]
    async fn local_source_scans_recipes() {
        let tmp = TempDir::new().unwrap();
        let recipes_dir = tmp.path().join("recipes");
        fs::create_dir_all(&recipes_dir).unwrap();
        fs::write(
            recipes_dir.join("my-recipe.yaml"),
            "version: \"1.0.0\"\ntitle: My Recipe\ndescription: Does things\ninstructions: Do X\n",
        )
        .unwrap();

        let source = LocalRegistrySource::new(vec![tmp.path().to_path_buf()]);
        let results = source
            .search(None, Some(RegistryEntryKind::Recipe))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "My Recipe");
    }

    #[tokio::test]
    async fn local_source_search_filters_by_query() {
        let tmp = TempDir::new().unwrap();
        let skills_dir_a = tmp.path().join("skills").join("alpha-skill");
        let skills_dir_b = tmp.path().join("skills").join("beta-skill");
        fs::create_dir_all(&skills_dir_a).unwrap();
        fs::create_dir_all(&skills_dir_b).unwrap();
        fs::write(
            skills_dir_a.join("SKILL.md"),
            "---
name: alpha-skill
description: Alpha things
---
Alpha body.
",
        )
        .unwrap();
        fs::write(
            skills_dir_b.join("SKILL.md"),
            "---
name: beta-skill
description: Beta things
---
Beta body.
",
        )
        .unwrap();

        let source = LocalRegistrySource::new(vec![tmp.path().to_path_buf()]);
        let results = source.search(Some("alpha"), None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "alpha-skill");
    }

    #[tokio::test]
    async fn local_source_get_by_name() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skills").join("target");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("SKILL.md"),
            "---
name: target
description: Target skill
---
Target body.
",
        )
        .unwrap();

        let source = LocalRegistrySource::new(vec![tmp.path().to_path_buf()]);
        let result = source
            .get("target", Some(RegistryEntryKind::Skill))
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "target");

        let result = source
            .get("nonexistent", Some(RegistryEntryKind::Skill))
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
