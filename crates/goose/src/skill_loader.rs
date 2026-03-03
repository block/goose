use crate::agents::builtin_skills;
use crate::config::paths::Paths;
use crate::recipe::{Recipe, RECIPE_FILE_EXTENSIONS};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub kind: SourceKind,
    pub description: String,
    pub path: PathBuf,
    pub content: String,
    pub supporting_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    Subrecipe,
    Recipe,
    Skill,
    Agent,
    BuiltinSkill,
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceKind::Subrecipe => write!(f, "subrecipe"),
            SourceKind::Recipe => write!(f, "recipe"),
            SourceKind::Skill => write!(f, "skill"),
            SourceKind::Agent => write!(f, "agent"),
            SourceKind::BuiltinSkill => write!(f, "builtin skill"),
        }
    }
}

impl Source {
    pub fn to_load_text(&self) -> String {
        format!(
            "## {} ({})\n\n{}\n\n### Content\n\n{}",
            self.name, self.kind, self.description, self.content
        )
    }
}

pub fn kind_plural(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Subrecipe => "Subrecipes",
        SourceKind::Recipe => "Recipes",
        SourceKind::Skill => "Skills",
        SourceKind::Agent => "Agents",
        SourceKind::BuiltinSkill => "Builtin Skills",
    }
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

#[derive(Debug, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentMetadata {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

pub fn parse_frontmatter<T: for<'de> Deserialize<'de>>(content: &str) -> Option<(T, String)> {
    let parts: Vec<&str> = content.split("---").collect();
    if parts.len() < 3 {
        return None;
    }

    let yaml_content = parts[1].trim();
    let metadata: T = match serde_yaml::from_str(yaml_content) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to parse frontmatter: {}", e);
            return None;
        }
    };

    let body = parts[2..].join("---").trim().to_string();
    Some((metadata, body))
}

pub fn parse_skill_content(content: &str, path: PathBuf) -> Option<Source> {
    let (metadata, body): (SkillMetadata, String) = parse_frontmatter(content)?;

    Some(Source {
        name: metadata.name,
        kind: SourceKind::Skill,
        description: metadata.description,
        path,
        content: body,
        supporting_files: Vec::new(),
    })
}

pub fn parse_agent_content(content: &str, path: PathBuf) -> Option<Source> {
    let (metadata, body): (AgentMetadata, String) = parse_frontmatter(content)?;

    let description = metadata.description.unwrap_or_else(|| {
        let model_info = metadata
            .model
            .as_ref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        format!("Agent{}", model_info)
    });

    Some(Source {
        name: metadata.name,
        kind: SourceKind::Agent,
        description,
        path,
        content: body,
        supporting_files: Vec::new(),
    })
}

pub fn scan_skills_from_dir(dir: &Path, seen: &mut HashSet<String>) -> Vec<Source> {
    let mut sources = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return sources,
    };

    for entry in entries.flatten() {
        let skill_dir = entry.path();
        if !skill_dir.is_dir() {
            continue;
        }

        let skill_file = skill_dir.join("SKILL.md");
        if !skill_file.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&skill_file) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read skill file {}: {}", skill_file.display(), e);
                continue;
            }
        };

        if let Some(mut source) = parse_skill_content(&content, skill_dir.clone()) {
            if !seen.contains(&source.name) {
                source.supporting_files = find_supporting_files(&skill_dir, &skill_file);
                seen.insert(source.name.clone());
                sources.push(source);
            }
        }
    }
    sources
}

pub fn scan_recipes_from_dir(
    dir: &Path,
    kind: SourceKind,
    sources: &mut Vec<Source>,
    seen: &mut HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !RECIPE_FILE_EXTENSIONS.contains(&ext) {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if name.is_empty() || seen.contains(&name) {
            continue;
        }

        match Recipe::from_file_path(&path) {
            Ok(recipe) => {
                seen.insert(name.clone());
                sources.push(Source {
                    name,
                    kind,
                    description: recipe.description.clone(),
                    path: path.clone(),
                    content: recipe.instructions.clone().unwrap_or_default(),
                    supporting_files: Vec::new(),
                });
            }
            Err(e) => {
                warn!("Failed to parse recipe {}: {}", path.display(), e);
            }
        }
    }
}

pub fn scan_agents_from_dir(dir: &Path, sources: &mut Vec<Source>, seen: &mut HashSet<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read agent file {}: {}", path.display(), e);
                continue;
            }
        };

        if let Some(source) = parse_agent_content(&content, path) {
            if !seen.contains(&source.name) {
                seen.insert(source.name.clone());
                sources.push(source);
            }
        }
    }
}

pub fn discover_filesystem_sources(working_dir: &Path) -> Vec<Source> {
    let mut sources: Vec<Source> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let home = dirs::home_dir();
    let config = Paths::config_dir();

    let local_recipe_dirs: Vec<PathBuf> = vec![
        working_dir.to_path_buf(),
        working_dir.join(".goose/recipes"),
    ];

    let global_recipe_dirs: Vec<PathBuf> = std::env::var("GOOSE_RECIPE_PATH")
        .ok()
        .into_iter()
        .flat_map(|p| {
            let sep = if cfg!(windows) { ';' } else { ':' };
            p.split(sep).map(PathBuf::from).collect::<Vec<_>>()
        })
        .chain([config.join("recipes")])
        .collect();

    let local_skill_dirs: Vec<PathBuf> = vec![
        working_dir.join(".goose/skills"),
        working_dir.join(".claude/skills"),
        working_dir.join(".agents/skills"),
    ];

    let global_skill_dirs: Vec<PathBuf> = [
        Some(config.join("skills")),
        home.as_ref().map(|h| h.join(".claude/skills")),
        home.as_ref().map(|h| h.join(".config/agents/skills")),
    ]
    .into_iter()
    .flatten()
    .collect();

    let local_agent_dirs: Vec<PathBuf> = vec![
        working_dir.join(".goose/agents"),
        working_dir.join(".claude/agents"),
    ];

    let global_agent_dirs: Vec<PathBuf> = [
        Some(config.join("agents")),
        home.as_ref().map(|h| h.join(".claude/agents")),
    ]
    .into_iter()
    .flatten()
    .collect();

    for dir in local_recipe_dirs {
        scan_recipes_from_dir(&dir, SourceKind::Recipe, &mut sources, &mut seen);
    }

    for dir in local_skill_dirs {
        sources.extend(scan_skills_from_dir(&dir, &mut seen));
    }

    for dir in local_agent_dirs {
        scan_agents_from_dir(&dir, &mut sources, &mut seen);
    }

    for dir in global_recipe_dirs {
        scan_recipes_from_dir(&dir, SourceKind::Recipe, &mut sources, &mut seen);
    }

    for dir in global_skill_dirs {
        sources.extend(scan_skills_from_dir(&dir, &mut seen));
    }

    for dir in global_agent_dirs {
        scan_agents_from_dir(&dir, &mut sources, &mut seen);
    }

    for content in builtin_skills::get_all() {
        if let Some(source) = parse_skill_content(content, PathBuf::new()) {
            if !seen.contains(&source.name) {
                seen.insert(source.name.clone());
                sources.push(Source {
                    kind: SourceKind::BuiltinSkill,
                    ..source
                });
            }
        }
    }

    sources
}

pub fn find_supporting_files(directory: &Path, skill_file: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(directory) {
        Ok(e) => e,
        Err(_) => return files,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path != skill_file {
            files.push(path);
        } else if path.is_dir() {
            if let Ok(sub_entries) = std::fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        files.push(sub_path);
                    }
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_frontmatter_parsing() {
        let skill = r#"---
name: test-skill
description: A test skill
---
Skill body here."#;
        let source = parse_skill_content(skill, PathBuf::new()).unwrap();
        assert_eq!(source.name, "test-skill");
        assert_eq!(source.kind, SourceKind::Skill);
        assert!(source.content.contains("Skill body"));

        let agent = r#"---
name: reviewer
model: sonnet
---
You review code."#;
        let source = parse_agent_content(agent, PathBuf::new()).unwrap();
        assert_eq!(source.name, "reviewer");
        assert!(source.description.contains("sonnet"));

        assert!(parse_skill_content("no frontmatter", PathBuf::new()).is_none());
        assert!(parse_skill_content("---\nunclosed", PathBuf::new()).is_none());
    }

    #[test]
    fn test_source_discovery_and_priority() {
        let temp_dir = TempDir::new().unwrap();

        let goose_skill = temp_dir.path().join(".goose/skills/my-skill");
        fs::create_dir_all(&goose_skill).unwrap();
        fs::write(
            goose_skill.join("SKILL.md"),
            "---\nname: my-skill\ndescription: goose version\n---\nContent",
        )
        .unwrap();

        let claude_skill = temp_dir.path().join(".claude/skills/my-skill");
        fs::create_dir_all(&claude_skill).unwrap();
        fs::write(
            claude_skill.join("SKILL.md"),
            "---\nname: my-skill\ndescription: claude version\n---\nContent",
        )
        .unwrap();

        let recipes = temp_dir.path().join(".goose/recipes");
        fs::create_dir_all(&recipes).unwrap();
        fs::write(
            recipes.join("test.yaml"),
            "title: Test\ndescription: A recipe\ninstructions: Do it",
        )
        .unwrap();

        let sources = discover_filesystem_sources(temp_dir.path());

        let skill = sources.iter().find(|s| s.name == "my-skill").unwrap();
        assert_eq!(skill.description, "goose version");

        assert!(sources
            .iter()
            .any(|s| s.name == "test" && s.kind == SourceKind::Recipe));

        assert!(sources.iter().any(|s| s.kind == SourceKind::BuiltinSkill));
    }

    #[test]
    fn test_skill_supporting_files_discovered() {
        let temp_dir = TempDir::new().unwrap();

        let skill_dir = temp_dir.path().join(".goose/skills/my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A skill with scripts\n---\nRun check_all.sh",
        )
        .unwrap();
        fs::write(skill_dir.join("myscript.sh"), "#!/bin/bash\necho ok").unwrap();
        fs::create_dir(skill_dir.join("templates")).unwrap();
        fs::write(skill_dir.join("templates/report.txt"), "template content").unwrap();

        let sources = discover_filesystem_sources(temp_dir.path());

        let skill = sources.iter().find(|s| s.name == "my-skill").unwrap();
        assert_eq!(skill.path, skill_dir);
        assert_eq!(skill.supporting_files.len(), 2);

        let file_names: Vec<String> = skill
            .supporting_files
            .iter()
            .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        assert!(file_names.contains(&"myscript.sh".to_string()));
        assert!(file_names.contains(&"report.txt".to_string()));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hello", 3), "...");
        assert_eq!(truncate("hello", 2), "...");
        assert_eq!(truncate("hi", 5), "hi");
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_kind_plural() {
        assert_eq!(kind_plural(SourceKind::Skill), "Skills");
        assert_eq!(kind_plural(SourceKind::Recipe), "Recipes");
        assert_eq!(kind_plural(SourceKind::Agent), "Agents");
        assert_eq!(kind_plural(SourceKind::Subrecipe), "Subrecipes");
        assert_eq!(kind_plural(SourceKind::BuiltinSkill), "Builtin Skills");
    }
}
