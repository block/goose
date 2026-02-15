use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::manifest::{RegistryEntry, RegistryEntryDetail, RegistryEntryKind};
use crate::agents::ExtensionConfig;
use crate::config::extensions::{set_extension, ExtensionEntry};

/// Get the global install directory for a given artifact kind
pub fn install_dir(kind: RegistryEntryKind) -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("goose");

    let subdir = match kind {
        RegistryEntryKind::Skill => "skills",
        RegistryEntryKind::Agent => "agents",
        RegistryEntryKind::Recipe => "recipes",
        RegistryEntryKind::Tool => "extensions",
    };

    Ok(config_dir.join(subdir))
}

/// Install a registry entry to the local filesystem
pub fn install_entry(entry: &RegistryEntry) -> Result<PathBuf> {
    match entry.kind {
        RegistryEntryKind::Skill => install_skill(entry),
        RegistryEntryKind::Agent => install_agent(entry),
        RegistryEntryKind::Recipe => install_recipe(entry),
        RegistryEntryKind::Tool => install_tool(entry),
    }
}

/// Remove an installed entry
pub fn remove_entry(name: &str, kind: RegistryEntryKind) -> Result<()> {
    match kind {
        RegistryEntryKind::Skill => remove_skill(name),
        RegistryEntryKind::Agent => remove_agent(name),
        RegistryEntryKind::Recipe => remove_recipe(name),
        RegistryEntryKind::Tool => remove_tool(name),
    }
}

/// Check if an entry is installed
pub fn is_installed(name: &str, kind: RegistryEntryKind) -> bool {
    match kind {
        RegistryEntryKind::Skill => {
            let dir = install_dir(kind).ok();
            dir.is_some_and(|d| d.join(name).join("SKILL.md").exists())
        }
        RegistryEntryKind::Agent => {
            let dir = install_dir(kind).ok();
            dir.is_some_and(|d| d.join(format!("{}.md", name)).exists())
        }
        RegistryEntryKind::Recipe => {
            let dir = install_dir(kind).ok();
            dir.is_some_and(|d| d.join(format!("{}.yaml", name)).exists())
        }
        RegistryEntryKind::Tool => crate::config::extensions::get_extension_by_name(name).is_some(),
    }
}

/// List installed entries of a given kind
pub fn list_installed(kind: RegistryEntryKind) -> Result<Vec<String>> {
    match kind {
        RegistryEntryKind::Tool => Ok(crate::config::extensions::get_all_extension_names()),
        RegistryEntryKind::Skill => {
            let dir = install_dir(kind)?;
            list_subdirs(&dir)
        }
        RegistryEntryKind::Agent => {
            let dir = install_dir(kind)?;
            list_files_with_ext(&dir, "md")
        }
        RegistryEntryKind::Recipe => {
            let dir = install_dir(kind)?;
            list_files_with_ext(&dir, "yaml")
        }
    }
}

fn install_skill(entry: &RegistryEntry) -> Result<PathBuf> {
    let dir = install_dir(RegistryEntryKind::Skill)?.join(&entry.name);
    fs::create_dir_all(&dir)?;

    let content = match &entry.detail {
        RegistryEntryDetail::Skill(detail) => {
            format!(
                "---\nname: {}\ndescription: {}\n---\n\n{}",
                entry.name, entry.description, detail.content
            )
        }
        _ => format!(
            "---\nname: {}\ndescription: {}\n---\n",
            entry.name, entry.description
        ),
    };

    let path = dir.join("SKILL.md");
    fs::write(&path, &content)?;
    Ok(path)
}

fn install_agent(entry: &RegistryEntry) -> Result<PathBuf> {
    let dir = install_dir(RegistryEntryKind::Agent)?;
    fs::create_dir_all(&dir)?;

    let content = match &entry.detail {
        RegistryEntryDetail::Agent(detail) => {
            let mut front = format!(
                "---\nname: {}\ndescription: {}",
                entry.name, entry.description
            );
            if let Some(model) = &detail.model {
                front.push_str(&format!("\nmodel: {}", model));
            }
            front.push_str("\n---\n\n");
            front.push_str(&detail.instructions);
            front
        }
        _ => format!(
            "---\nname: {}\ndescription: {}\n---\n",
            entry.name, entry.description
        ),
    };

    let path = dir.join(format!("{}.md", entry.name));
    fs::write(&path, &content)?;
    Ok(path)
}

fn install_recipe(entry: &RegistryEntry) -> Result<PathBuf> {
    let dir = install_dir(RegistryEntryKind::Recipe)?;
    fs::create_dir_all(&dir)?;

    // If we have the original local path, copy the file
    if let Some(local_path) = &entry.local_path {
        if local_path.exists() {
            let dest = dir.join(format!("{}.yaml", entry.name));
            fs::copy(local_path, &dest)?;
            return Ok(dest);
        }
    }

    // Otherwise generate a minimal recipe YAML
    let yaml = match &entry.detail {
        RegistryEntryDetail::Recipe(detail) => {
            let mut y = format!(
                "version: \"1.0.0\"\ntitle: {}\ndescription: {}",
                entry.name, entry.description
            );
            if let Some(instructions) = &detail.instructions {
                y.push_str(&format!("\ninstructions: {}", instructions));
            }
            if let Some(prompt) = &detail.prompt {
                y.push_str(&format!("\nprompt: {}", prompt));
            }
            y
        }
        _ => format!(
            "version: \"1.0.0\"\ntitle: {}\ndescription: {}",
            entry.name, entry.description
        ),
    };

    let path = dir.join(format!("{}.yaml", entry.name));
    fs::write(&path, &yaml)?;
    Ok(path)
}

fn install_tool(entry: &RegistryEntry) -> Result<PathBuf> {
    // For tools, we add to the goose config
    if let RegistryEntryDetail::Tool(detail) = &entry.detail {
        let config = match &detail.transport {
            super::manifest::ToolTransport::Stdio { cmd, args } => ExtensionConfig::Stdio {
                name: entry.name.clone(),
                description: entry.description.clone(),
                cmd: cmd.clone(),
                args: args.clone(),
                envs: Default::default(),
                env_keys: detail.env_keys.clone(),
                timeout: None,
                bundled: Some(false),
                available_tools: Vec::new(),
            },
            super::manifest::ToolTransport::StreamableHttp { uri } => {
                ExtensionConfig::StreamableHttp {
                    name: entry.name.clone(),
                    description: entry.description.clone(),
                    uri: uri.clone(),
                    envs: Default::default(),
                    env_keys: detail.env_keys.clone(),
                    headers: Default::default(),
                    timeout: None,
                    bundled: Some(false),
                    available_tools: Vec::new(),
                }
            }
            super::manifest::ToolTransport::Builtin => ExtensionConfig::Builtin {
                name: entry.name.clone(),
                description: entry.description.clone(),
                display_name: None,
                timeout: None,
                bundled: Some(true),
                available_tools: Vec::new(),
            },
        };

        set_extension(ExtensionEntry {
            enabled: true,
            config,
        });
    }

    let config_path = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("goose")
        .join("config.yaml");
    Ok(config_path)
}

fn remove_skill(name: &str) -> Result<()> {
    let dir = install_dir(RegistryEntryKind::Skill)?.join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

fn remove_agent(name: &str) -> Result<()> {
    let path = install_dir(RegistryEntryKind::Agent)?.join(format!("{}.md", name));
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

fn remove_recipe(name: &str) -> Result<()> {
    let path = install_dir(RegistryEntryKind::Recipe)?.join(format!("{}.yaml", name));
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

fn remove_tool(name: &str) -> Result<()> {
    crate::config::extensions::remove_extension(name);
    Ok(())
}

fn list_subdirs(dir: &Path) -> Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

fn list_files_with_ext(dir: &Path, ext: &str) -> Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == ext) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::manifest::{AgentDetail, SkillDetail};

    #[test]
    fn test_install_and_remove_skill() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("skills").join("test-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let entry = RegistryEntry {
            name: "test-skill".into(),
            kind: RegistryEntryKind::Skill,
            description: "A test skill".into(),
            detail: RegistryEntryDetail::Skill(SkillDetail {
                content: "Do something useful".into(),
                builtin: false,
            }),
            ..Default::default()
        };

        // Install to the temp dir
        let skill_path = skill_dir.join("SKILL.md");
        let content = format!(
            "---\nname: {}\ndescription: {}\n---\n\n{}",
            entry.name, entry.description, "Do something useful"
        );
        std::fs::write(&skill_path, &content).unwrap();

        assert!(skill_path.exists());

        // Remove
        std::fs::remove_dir_all(&skill_dir).unwrap();
        assert!(!skill_dir.exists());
    }

    #[test]
    fn test_install_and_remove_agent() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        let entry = RegistryEntry {
            name: "test-agent".into(),
            kind: RegistryEntryKind::Agent,
            description: "A test agent".into(),
            detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                instructions: "You are a helpful agent".into(),
                model: Some("gpt-4o".into()),
                ..Default::default()
            })),
            ..Default::default()
        };

        let agent_path = agents_dir.join("test-agent.md");
        let content = format!(
            "---\nname: {}\ndescription: {}\nmodel: gpt-4o\n---\n\nYou are a helpful agent",
            entry.name, entry.description
        );
        std::fs::write(&agent_path, &content).unwrap();

        assert!(agent_path.exists());
        let read = std::fs::read_to_string(&agent_path).unwrap();
        assert!(read.contains("test-agent"));
        assert!(read.contains("gpt-4o"));

        std::fs::remove_file(&agent_path).unwrap();
        assert!(!agent_path.exists());
    }

    #[test]
    fn test_list_installed_files() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path();

        // Create some .md files
        std::fs::write(agents_dir.join("alpha.md"), "agent alpha").unwrap();
        std::fs::write(agents_dir.join("beta.md"), "agent beta").unwrap();
        std::fs::write(agents_dir.join("not-an-agent.txt"), "ignore me").unwrap();

        let names = list_files_with_ext(agents_dir, "md").unwrap();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_list_installed_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path();

        std::fs::create_dir_all(skills_dir.join("skill-a")).unwrap();
        std::fs::create_dir_all(skills_dir.join("skill-b")).unwrap();
        std::fs::write(skills_dir.join("not-a-dir.txt"), "ignore").unwrap();

        let names = list_subdirs(skills_dir).unwrap();
        assert_eq!(names, vec!["skill-a", "skill-b"]);
    }

    #[test]
    fn test_list_nonexistent_dir() {
        let names = list_files_with_ext(Path::new("/nonexistent/path"), "md").unwrap();
        assert!(names.is_empty());
    }
}
