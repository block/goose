use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;

use crate::registry::manifest::{
    AgentDetail, RecipeDetail, RegistryEntry, RegistryEntryDetail, RegistryEntryKind, SkillDetail,
};
use crate::registry::source::RegistrySource;

/// Discovers registry entries from a GitHub repository using the `gh` CLI.
///
/// Scans the repository tree for:
/// - `recipes/*.yaml` → Recipe entries
/// - `skills/*/SKILL.md` → Skill entries
/// - `agents/*.md` → Agent entries
pub struct GitHubRegistrySource {
    owner: String,
    repo: String,
    branch: String,
    /// Optional subdirectory prefix (e.g. "registry/")
    path_prefix: String,
}

impl GitHubRegistrySource {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: "main".to_string(),
            path_prefix: String::new(),
        }
    }

    pub fn with_branch(mut self, branch: &str) -> Self {
        self.branch = branch.to_string();
        self
    }

    pub fn with_path_prefix(mut self, prefix: &str) -> Self {
        self.path_prefix = prefix.to_string();
        self
    }

    fn source_uri(&self, path: &str) -> String {
        format!(
            "github://{}/{}/{}{}",
            self.owner, self.repo, self.branch, path
        )
    }

    fn list_dir(&self, dir: &str) -> Result<Vec<(String, String)>> {
        let url = format!(
            "repos/{}/{}/contents/{}{}",
            self.owner, self.repo, self.path_prefix, dir
        );
        let output = Command::new("gh")
            .args([
                "api",
                &url,
                "-q",
                r#".[] | select(.type == "file" or .type == "dir") | "(.name)	(.type)"#,
            ])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, '\t').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect())
    }

    fn fetch_file(&self, path: &str) -> Result<String> {
        let url = format!(
            "repos/{}/{}/contents/{}{}",
            self.owner, self.repo, self.path_prefix, path
        );
        let output = Command::new("gh")
            .args(["api", &url, "-q", ".content"])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("failed to fetch {}", path);
        }

        let b64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace(['\n', '"'], "");

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64.as_bytes())?;
        Ok(String::from_utf8(bytes)?)
    }

    fn scan_recipes(&self) -> Vec<RegistryEntry> {
        let entries = match self.list_dir("recipes") {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        entries
            .into_iter()
            .filter(|(name, _)| name.ends_with(".yaml") || name.ends_with(".yml"))
            .filter_map(|(name, _)| {
                let path = format!("recipes/{}", name);
                let content = self.fetch_file(&path).ok()?;
                self.parse_recipe_yaml(&name, &content)
            })
            .collect()
    }

    fn parse_recipe_yaml(&self, filename: &str, content: &str) -> Option<RegistryEntry> {
        let mapping: serde_yaml::Mapping = serde_yaml::from_str(content).ok()?;
        let yaml_key = |k: &str| serde_yaml::Value::String(k.into());

        let title = mapping
            .get(yaml_key("title"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let description = mapping
            .get(yaml_key("description"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let version = mapping
            .get(yaml_key("version"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let extension_names: Vec<String> = mapping
            .get(yaml_key("extensions"))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| {
                        v.as_str().map(String::from).or_else(|| {
                            v.as_mapping()?
                                .get(yaml_key("name"))?
                                .as_str()
                                .map(String::from)
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let parameters: Vec<String> = mapping
            .get(yaml_key("parameters"))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| {
                        v.as_mapping()?
                            .get(yaml_key("key"))?
                            .as_str()
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let prompt = mapping
            .get(yaml_key("prompt"))
            .and_then(|v| v.as_str())
            .map(|s| s.chars().take(200).collect());
        let instructions = mapping
            .get(yaml_key("instructions"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let stem = filename
            .strip_suffix(".yaml")
            .or_else(|| filename.strip_suffix(".yml"))
            .unwrap_or(filename);

        Some(RegistryEntry {
            name: title.to_string(),
            kind: RegistryEntryKind::Recipe,
            description: description.to_string(),
            version,
            source_uri: Some(self.source_uri(&format!("/recipes/{}", filename))),
            detail: RegistryEntryDetail::Recipe(RecipeDetail {
                instructions,
                prompt,
                extension_names,
                parameters,
            }),
            tags: vec![stem.to_string()],
            ..Default::default()
        })
    }

    fn scan_skills(&self) -> Vec<RegistryEntry> {
        let dirs = match self.list_dir("skills") {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        dirs.into_iter()
            .filter(|(_, typ)| typ == "dir")
            .filter_map(|(dir_name, _)| {
                let path = format!("skills/{}/SKILL.md", dir_name);
                let content = self.fetch_file(&path).ok()?;
                let (fm_name, description, body) = parse_frontmatter(&content);
                let entry_name = fm_name.unwrap_or_else(|| dir_name.clone());
                let uri = self.source_uri(&format!("/skills/{}/SKILL.md", dir_name));
                Some(RegistryEntry {
                    name: entry_name,
                    kind: RegistryEntryKind::Skill,
                    description: description.unwrap_or_default(),
                    source_uri: Some(uri),
                    detail: RegistryEntryDetail::Skill(SkillDetail {
                        content: body,
                        builtin: false,
                    }),
                    ..Default::default()
                })
            })
            .collect()
    }

    fn scan_agents(&self) -> Vec<RegistryEntry> {
        let entries = match self.list_dir("agents") {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        entries
            .into_iter()
            .filter(|(name, _)| name.ends_with(".md"))
            .filter_map(|(name, _)| {
                let path = format!("agents/{}", name);
                let content = self.fetch_file(&path).ok()?;
                let (fm_name, description, body) = parse_frontmatter(&content);
                let stem = name.strip_suffix(".md").unwrap_or(&name);
                Some(RegistryEntry {
                    name: fm_name.unwrap_or_else(|| stem.to_string()),
                    kind: RegistryEntryKind::Agent,
                    description: description.unwrap_or_default(),
                    source_uri: Some(self.source_uri(&format!("/agents/{}", name))),
                    detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                        instructions: body,
                        model: None,
                        recommended_models: Vec::new(),
                        capabilities: Vec::new(),
                        domains: Vec::new(),
                        input_content_types: Vec::new(),
                        output_content_types: Vec::new(),
                        required_extensions: Vec::new(),
                        dependencies: Vec::new(),
                        ..Default::default()
                    })),
                    ..Default::default()
                })
            })
            .collect()
    }
}

fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>, String) {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return (None, None, content.to_string());
    }

    let after_first = match trimmed.get(3..) {
        Some(s) => s.trim_start_matches([' ', '\t']).trim_start_matches('\n'),
        None => return (None, None, content.to_string()),
    };

    let end_pos = match after_first.find("\n---") {
        Some(p) => p,
        None => return (None, None, content.to_string()),
    };

    let fm_block = match after_first.get(..end_pos) {
        Some(s) => s,
        None => return (None, None, content.to_string()),
    };
    let body = match after_first.get(end_pos + 4..) {
        Some(s) => s.trim_start().to_string(),
        None => String::new(),
    };

    let mut name = None;
    let mut description = None;

    for line in fm_block.lines() {
        if let Some(val) = line.strip_prefix("name:") {
            name = Some(val.trim().trim_matches('"').to_string());
        } else if let Some(val) = line.strip_prefix("description:") {
            description = Some(val.trim().trim_matches('"').to_string());
        }
    }

    (name, description, body)
}

#[async_trait]
impl RegistrySource for GitHubRegistrySource {
    fn name(&self) -> &str {
        "github"
    }

    async fn search(
        &self,
        query: Option<&str>,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Vec<RegistryEntry>> {
        let mut entries = Vec::new();

        let want_recipes = kind.is_none() || kind == Some(RegistryEntryKind::Recipe);
        let want_skills = kind.is_none() || kind == Some(RegistryEntryKind::Skill);
        let want_agents = kind.is_none() || kind == Some(RegistryEntryKind::Agent);

        if want_recipes {
            entries.extend(self.scan_recipes());
        }
        if want_skills {
            entries.extend(self.scan_skills());
        }
        if want_agents {
            entries.extend(self.scan_agents());
        }

        if let Some(q) = query {
            let q_lower = q.to_lowercase();
            entries.retain(|e| {
                e.name.to_lowercase().contains(&q_lower)
                    || e.description.to_lowercase().contains(&q_lower)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
            });
        }

        Ok(entries)
    }

    async fn get(
        &self,
        name: &str,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Option<RegistryEntry>> {
        let entries = self.search(Some(name), kind).await?;
        Ok(entries.into_iter().find(|e| e.name == name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\nname: my-skill\ndescription: A test skill\n---\nSkill body here.";
        let (name, desc, body) = parse_frontmatter(content);
        assert_eq!(name, Some("my-skill".to_string()));
        assert_eq!(desc, Some("A test skill".to_string()));
        assert!(body.contains("Skill body here"));
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "Just plain markdown.";
        let (name, desc, body) = parse_frontmatter(content);
        assert!(name.is_none());
        assert!(desc.is_none());
        assert_eq!(body, "Just plain markdown.");
    }

    #[test]
    fn test_source_uri_format() {
        let source = GitHubRegistrySource::new("block", "goose");
        assert_eq!(
            source.source_uri("/recipes/test.yaml"),
            "github://block/goose/main/recipes/test.yaml"
        );
    }

    #[test]
    fn test_parse_recipe_yaml() {
        let source = GitHubRegistrySource::new("block", "goose");
        let yaml = "title: Test Recipe\ndescription: A test\nversion: \"1.0\"\nprompt: Do things\nextensions:\n  - developer\nparameters:\n  - key: name\n    type: string";
        let entry = source.parse_recipe_yaml("test.yaml", yaml).unwrap();
        assert_eq!(entry.name, "Test Recipe");
        assert_eq!(entry.kind, RegistryEntryKind::Recipe);
        assert_eq!(entry.description, "A test");
        if let RegistryEntryDetail::Recipe(ref detail) = entry.detail {
            assert_eq!(detail.extension_names, vec!["developer"]);
            assert_eq!(detail.parameters, vec!["name"]);
        } else {
            panic!("expected Recipe detail");
        }
    }
}
