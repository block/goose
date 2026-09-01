//! Application-independent skill discovery and rendering.

mod arguments;

use anyhow::{bail, Context, Result};
use goose_sdk_types::custom_requests::{SourceEntry, SourceType};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillScope {
    Global,
    Project,
    Plugin,
    Builtin,
}

#[derive(Debug, Clone)]
pub struct SkillRoot {
    pub path: PathBuf,
    pub scope: SkillScope,
    /// Lower values win duplicate-name resolution.
    pub precedence: u32,
    pub writable: bool,
    /// Preserve the supplied root path rather than canonicalizing discovered
    /// skill directories. Hosts use this for plugin paths with stable identity.
    pub preserve_path: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SkillDiscoveryOptions {
    pub roots: Vec<SkillRoot>,
    pub working_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SkillDiagnostic {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct SkillDiscovery {
    pub skills: Vec<SourceEntry>,
    pub diagnostics: Vec<SkillDiagnostic>,
}

#[derive(Deserialize)]
struct Frontmatter {
    name: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    metadata: HashMap<String, Value>,
}

fn parse_frontmatter(content: &str) -> Result<(Frontmatter, String)> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .context("SKILL.md must begin with YAML frontmatter")?;
    let (header, body) = rest
        .split_once("\n---\n")
        .or_else(|| rest.split_once("\r\n---\r\n"))
        .context("SKILL.md has unterminated YAML frontmatter")?;
    Ok((serde_yaml::from_str(header)?, body.to_string()))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn walk(root: &Path, files: &mut Vec<PathBuf>, visited: &mut HashSet<PathBuf>) {
    let Ok(canonical) = root.canonicalize() else {
        return;
    };
    if !visited.insert(canonical) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if !matches!(
                path.file_name().and_then(|n| n.to_str()),
                Some(".git" | ".hg" | ".svn")
            ) {
                walk(&path, files, visited);
            }
        } else if path.is_file() {
            files.push(path);
        }
    }
}

/// Discovers all valid skills. Roots are visited by ascending precedence and
/// then path; the first skill with a given name wins. Bad entries are reported
/// without hiding valid entries.
pub fn discover_skills(options: &SkillDiscoveryOptions) -> Result<SkillDiscovery> {
    let mut roots = options.roots.clone();
    roots.sort_by(|a, b| {
        a.precedence
            .cmp(&b.precedence)
            .then_with(|| a.path.cmp(&b.path))
    });
    let mut discovery = SkillDiscovery::default();
    let mut names = HashSet::new();
    for root in roots {
        let mut files = Vec::new();
        walk(&root.path, &mut files, &mut HashSet::new());
        for skill_file in files
            .into_iter()
            .filter(|path| path.file_name().and_then(|n| n.to_str()) == Some("SKILL.md"))
        {
            let result = (|| -> Result<SourceEntry> {
                let content = std::fs::read_to_string(&skill_file)?;
                let (frontmatter, body) = parse_frontmatter(&content)?;
                let name = frontmatter.name.context("missing skill name")?;
                if !valid_name(&name) {
                    bail!("invalid skill name '{name}'");
                }
                let discovered_dir = skill_file.parent().context("SKILL.md has no parent")?;
                let canonical_skill_dir = discovered_dir.canonicalize()?;
                let skill_dir = if root.preserve_path {
                    let relative = discovered_dir.strip_prefix(&root.path)?;
                    root.path.join(relative)
                } else {
                    canonical_skill_dir.clone()
                };
                let mut supporting = Vec::new();
                let mut all_files = Vec::new();
                walk(discovered_dir, &mut all_files, &mut HashSet::new());
                for file in all_files {
                    if file.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
                        if let Ok(canonical) = file.canonicalize() {
                            if canonical.starts_with(&canonical_skill_dir) {
                                let relative = canonical.strip_prefix(&canonical_skill_dir)?;
                                supporting
                                    .push(skill_dir.join(relative).to_string_lossy().into_owned());
                            }
                        }
                    }
                }
                supporting.sort();
                Ok(SourceEntry {
                    source_type: if root.scope == SkillScope::Builtin {
                        SourceType::BuiltinSkill
                    } else {
                        SourceType::Skill
                    },
                    name,
                    description: frontmatter.description,
                    content: body,
                    path: skill_dir.to_string_lossy().into_owned(),
                    global: matches!(root.scope, SkillScope::Global | SkillScope::Builtin),
                    writable: root.writable,
                    supporting_files: supporting,
                    properties: frontmatter.metadata,
                })
            })();
            match result {
                Ok(skill) if names.insert(skill.name.clone()) => discovery.skills.push(skill),
                Ok(_) => {}
                Err(error) => discovery.diagnostics.push(SkillDiagnostic {
                    path: skill_file,
                    message: error.to_string(),
                }),
            }
        }
    }
    Ok(discovery)
}

pub fn skill_argument_hint(skill: &SourceEntry) -> Option<String> {
    skill
        .properties
        .get("argument-hint")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

pub fn skill_argument_names(skill: &SourceEntry) -> Vec<String> {
    skill
        .properties
        .get("arguments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn loaded_skill_context_with_args(skill: &SourceEntry, args: Option<&str>) -> Result<String> {
    let content = match args {
        Some(args) => {
            arguments::apply_skill_arguments(&skill.content, args, &skill_argument_names(skill))?
        }
        None => skill.content.clone(),
    };
    let mut output = format!(
        "# Loaded Skill: {} ({})\n\n{}\n\n## Content\n\n{}\n",
        skill.name, skill.source_type, skill.description, content
    );
    if !skill.supporting_files.is_empty() {
        output.push_str(&format!("\n## Supporting Files\n\nSkill directory: {}\n\nRelative paths in this skill resolve from the skill directory. The shell tool runs in the session working directory, so use the resolved path below or `cd` into the skill directory before running supporting scripts.\n\n", skill.path));
        for file in &skill.supporting_files {
            if let Ok(relative) = Path::new(file).strip_prefix(&skill.path) {
                let relative = relative.to_string_lossy().replace('\\', "/");
                output.push_str(&format!(
                    "- {relative} → {} (load_skill(name: \"{}/{relative}\"))\n",
                    file.replace('\\', "/"),
                    skill.name
                ));
            }
        }
    }
    Ok(output)
}

/// Resolves an existing supporting file while rejecting absolute paths,
/// traversal, directories, and symlinks escaping the skill directory.
pub fn resolve_supporting_file(skill: &SourceEntry, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("supporting file path must be relative and may not traverse parents");
    }
    let root = Path::new(&skill.path).canonicalize()?;
    let resolved = root.join(relative).canonicalize()?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
        bail!("supporting file is outside the skill directory or is not a file");
    }
    Ok(resolved)
}
