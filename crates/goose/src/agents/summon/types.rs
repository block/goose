//! Data types for the summon extension — sources, delegate params, background tasks, metadata.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::Deserialize;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::registry::manifest::AgentDistribution;

// ---------------------------------------------------------------------------
// Source — a loadable recipe/skill/agent discovered from filesystem or registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub kind: SourceKind,
    pub description: String,
    pub path: PathBuf,
    pub content: String,
    pub distribution: Option<AgentDistribution>,
    pub a2a_url: Option<String>,
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
    /// Format the source content for loading into context
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

// ---------------------------------------------------------------------------
// DelegateParams — parameters for the delegate tool
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct DelegateParams {
    pub instructions: Option<String>,
    pub source: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub extensions: Option<Vec<String>>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    /// Agent mode to use (e.g., "code", "review", "architect")
    pub mode: Option<String>,
    #[serde(default)]
    pub r#async: bool,
}

// ---------------------------------------------------------------------------
// BackgroundTask / CompletedTask — async task tracking
// ---------------------------------------------------------------------------

pub struct BackgroundTask {
    pub id: String,
    pub description: String,
    pub started_at: Instant,
    pub turns: Arc<AtomicU32>,
    pub last_activity: Arc<AtomicU64>,
    pub handle: JoinHandle<Result<String>>,
    pub cancellation_token: CancellationToken,
}

pub struct CompletedTask {
    pub id: String,
    pub description: String,
    pub result: Result<String, String>,
    pub turns_taken: u32,
    pub duration: Duration,
}

// ---------------------------------------------------------------------------
// Metadata structs — parsed from YAML frontmatter in skill/agent files
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct SkillMetadata {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentMetadata {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub default_mode: Option<String>,
    #[serde(default)]
    pub modes: Vec<AgentModeEntry>,
    #[serde(default)]
    #[allow(dead_code)]
    pub required_extensions: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct AgentModeEntry {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub instructions_file: Option<String>,
    #[serde(default)]
    pub tool_groups: Vec<String>,
}

// ---------------------------------------------------------------------------
// Frontmatter parsing
// ---------------------------------------------------------------------------

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
        distribution: None,
        a2a_url: None,
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
        distribution: None,
        a2a_url: None,
    })
}
