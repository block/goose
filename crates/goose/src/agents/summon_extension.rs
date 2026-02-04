//! Summon Extension - Unified tooling for recipes, skills, and subagents
//!
//! Provides two tools:
//! - `load`: Inject knowledge into current context or discover available sources
//! - `delegate`: Run tasks in isolated subagents (sync or async)

use crate::agents::builtin_skills;
use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::subagent_handler::{
    run_complete_subagent_task, run_subagent_task_with_callback, OnMessageCallback,
};
use crate::agents::subagent_task_config::{TaskConfig, DEFAULT_SUBAGENT_MAX_TURNS};
use crate::agents::AgentConfig;
use crate::config::paths::Paths;
use crate::providers;
use crate::recipe::build_recipe::build_recipe_from_template;
use crate::recipe::local_recipes::load_local_recipe_file;
use crate::recipe::{Recipe, Settings, RECIPE_FILE_EXTENSIONS};
use crate::session::extension_data::{EnabledExtensionsState, ExtensionState};
use crate::session::SessionType;
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolsCapability,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub static EXTENSION_NAME: &str = "summon";

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub kind: SourceKind,
    pub description: String,
    pub path: PathBuf,
    pub content: String,
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

/// Get the plural form of a source kind for display
fn kind_plural(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Subrecipe => "Subrecipes",
        SourceKind::Recipe => "Recipes",
        SourceKind::Skill => "Skills",
        SourceKind::Agent => "Agents",
        SourceKind::BuiltinSkill => "Builtin Skills",
    }
}

/// Truncate a string to a maximum length, adding "..." if truncated
/// Handles UTF-8 properly by using char boundaries
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct DelegateParams {
    pub instructions: Option<String>,
    pub source: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub extensions: Option<Vec<String>>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub r#async: bool,
}

/// Active background task
pub struct BackgroundTask {
    pub id: String,
    pub description: String,
    pub started_at: Instant,
    pub turns: Arc<AtomicU32>,
    pub last_activity: Arc<AtomicU64>,
    pub handle: JoinHandle<Result<String>>,
}

/// Completed background task awaiting result retrieval
pub struct CompletedTask {
    pub id: String,
    pub description: String,
    pub result: Result<String, String>,
    pub turns_taken: u32,
    pub duration: Duration,
}

#[derive(Debug, Deserialize)]
struct SkillMetadata {
    name: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct AgentMetadata {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

fn parse_frontmatter<T: for<'de> Deserialize<'de>>(content: &str) -> Option<(T, String)> {
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

fn parse_skill_content(content: &str, path: PathBuf) -> Option<Source> {
    let (metadata, body): (SkillMetadata, String) = parse_frontmatter(content)?;

    Some(Source {
        name: metadata.name,
        kind: SourceKind::Skill,
        description: metadata.description,
        path,
        content: body,
    })
}

fn parse_agent_content(content: &str, path: PathBuf) -> Option<Source> {
    let (metadata, body): (AgentMetadata, String) = parse_frontmatter(content)?;

    let description = metadata.description.unwrap_or_else(|| {
        let model_info = metadata
            .model
            .as_ref()
            .map(|m| format!(" ({})", translate_model_shorthand(m)))
            .unwrap_or_default();
        format!("Claude agent{}", model_info)
    });

    Some(Source {
        name: metadata.name,
        kind: SourceKind::Agent,
        description,
        path,
        content: body,
    })
}

fn translate_model_shorthand(shorthand: &str) -> &str {
    match shorthand.to_lowercase().as_str() {
        "sonnet" => "claude-sonnet-4-20250514",
        "opus" => "claude-opus-4-20250514",
        "haiku" => "claude-haiku-3-20250514",
        _ => shorthand,
    }
}

/// Round duration for MOIM display to avoid prompt cache invalidation
fn round_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", (secs / 10) * 10)
    } else {
        format!("{}m", secs / 60)
    }
}

/// Get current epoch milliseconds
fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Get maximum number of concurrent background tasks
fn max_background_tasks() -> usize {
    std::env::var("GOOSE_MAX_BACKGROUND_TASKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)
}

fn generate_task_id() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = current_epoch_millis() % 100000;
    format!("task_{:05}_{:04}", timestamp, count % 10000)
}

pub struct SummonClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    source_cache: Mutex<Option<(Instant, PathBuf, Vec<Source>)>>,
    background_tasks: Mutex<HashMap<String, BackgroundTask>>,
    completed_tasks: Mutex<HashMap<String, CompletedTask>>,
}

impl SummonClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tasks: None,
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Summon".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Load knowledge and delegate tasks to subagents using the summon extension."
                    .to_string(),
            ),
        };

        Ok(Self {
            info,
            context,
            source_cache: Mutex::new(None),
            background_tasks: Mutex::new(HashMap::new()),
            completed_tasks: Mutex::new(HashMap::new()),
        })
    }

    fn create_load_tool(&self) -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Name of the source to load. If omitted, lists all available sources."
                }
            }
        });

        Tool::new(
            "load",
            "Load knowledge into your current context or discover available sources.\n\n\
             Call with no arguments to list all available sources (recipes, skills, agents).\n\
             Call with a source name to load its content into your context.\n\n\
             Examples:\n\
             - load() → Lists available sources\n\
             - load(source: \"rust-patterns\") → Loads the rust-patterns skill\n\n\
             Use this when you want to learn an approach or adopt expertise without delegating."
                .to_string(),
            schema.as_object().unwrap().clone(),
        )
    }

    fn create_delegate_tool(&self) -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "instructions": {
                    "type": "string",
                    "description": "Task instructions. Required for ad-hoc tasks."
                },
                "source": {
                    "type": "string",
                    "description": "Name of a recipe, skill, or agent to run."
                },
                "parameters": {
                    "type": "object",
                    "additionalProperties": true,
                    "description": "Parameters for the source (only valid with source)."
                },
                "extensions": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Extensions to enable. Omit to inherit all, empty array for none."
                },
                "provider": {
                    "type": "string",
                    "description": "Override LLM provider."
                },
                "model": {
                    "type": "string",
                    "description": "Override model."
                },
                "temperature": {
                    "type": "number",
                    "description": "Override temperature."
                },
                "async": {
                    "type": "boolean",
                    "default": false,
                    "description": "Run in background (default: false)."
                }
            }
        });

        Tool::new(
            "delegate",
            "Delegate a task to a subagent that runs independently with its own context.\n\n\
             Modes:\n\
             1. Ad-hoc: Provide `instructions` for a custom task\n\
             2. Source-based: Provide `source` name to run a recipe, skill, or agent\n\
             3. Combined: Provide both `source` and `instructions` for additional context\n\n\
             Options:\n\
             - `extensions`: Limit which extensions the subagent can use\n\
             - `provider`, `model`, `temperature`: Override model/provider settings\n\
             - `async`: Run in background (default: false)\n\n\
             For parallel execution, make multiple delegate calls in the same message.\n\
             Background tasks report status automatically in context."
                .to_string(),
            schema.as_object().unwrap().clone(),
        )
    }

    async fn get_working_dir(&self, session_id: &str) -> PathBuf {
        self.context
            .session_manager
            .get_session(session_id, false)
            .await
            .ok()
            .map(|s| s.working_dir)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    async fn get_sources(&self, session_id: &str, working_dir: &Path) -> Vec<Source> {
        let fs_sources = self.get_filesystem_sources(working_dir).await;

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut sources: Vec<Source> = Vec::new();

        self.add_subrecipes(session_id, &mut sources, &mut seen)
            .await;

        for source in fs_sources {
            if !seen.contains(&source.name) {
                seen.insert(source.name.clone());
                sources.push(source);
            }
        }

        sources.sort_by(|a, b| (&a.kind, &a.name).cmp(&(&b.kind, &b.name)));
        sources
    }

    async fn get_filesystem_sources(&self, working_dir: &Path) -> Vec<Source> {
        let mut cache = self.source_cache.lock().await;
        if let Some((cached_at, cached_dir, sources)) = cache.as_ref() {
            if cached_dir == working_dir && cached_at.elapsed() < Duration::from_secs(60) {
                return sources.clone();
            }
        }
        let sources = self.discover_filesystem_sources(working_dir);
        *cache = Some((Instant::now(), working_dir.to_path_buf(), sources.clone()));
        sources
    }

    async fn resolve_source(
        &self,
        session_id: &str,
        name: &str,
        working_dir: &Path,
    ) -> Option<Source> {
        let sources = self.get_sources(session_id, working_dir).await;
        let mut source = sources.into_iter().find(|s| s.name == name)?;

        if source.kind == SourceKind::Subrecipe && source.content.is_empty() {
            source.content = self.load_subrecipe_content(session_id, &source.name).await;
        }

        Some(source)
    }

    async fn load_subrecipe_content(&self, session_id: &str, name: &str) -> String {
        let session = match self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
        {
            Ok(s) => s,
            Err(_) => return String::new(),
        };

        let sub_recipes = match session.recipe.as_ref().and_then(|r| r.sub_recipes.as_ref()) {
            Some(sr) => sr,
            None => return String::new(),
        };

        let sr = match sub_recipes.iter().find(|sr| sr.name == name) {
            Some(sr) => sr,
            None => return String::new(),
        };

        match load_local_recipe_file(&sr.path) {
            Ok(recipe_file) => match Recipe::from_content(&recipe_file.content) {
                Ok(recipe) => recipe.instructions.unwrap_or_default(),
                Err(_) => recipe_file.content,
            },
            Err(_) => String::new(),
        }
    }

    fn discover_filesystem_sources(&self, working_dir: &Path) -> Vec<Source> {
        let mut sources: Vec<Source> = Vec::new();
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        self.add_local_recipes(working_dir, &mut sources, &mut seen_names);
        self.add_local_skills(working_dir, &mut sources, &mut seen_names);
        self.add_local_agents(working_dir, &mut sources, &mut seen_names);
        self.add_recipe_path_recipes(&mut sources, &mut seen_names);
        self.add_global_recipes(&mut sources, &mut seen_names);
        self.add_global_skills(&mut sources, &mut seen_names);
        self.add_global_agents(&mut sources, &mut seen_names);
        self.add_builtin_skills(&mut sources, &mut seen_names);

        sources
    }

    async fn add_subrecipes(
        &self,
        session_id: &str,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let session = match self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
        {
            Ok(s) => s,
            Err(_) => return,
        };

        let sub_recipes = match session.recipe.as_ref().and_then(|r| r.sub_recipes.as_ref()) {
            Some(sr) => sr,
            None => return,
        };

        for sr in sub_recipes {
            if seen.contains(&sr.name) {
                continue;
            }
            seen.insert(sr.name.clone());
            sources.push(Source {
                name: sr.name.clone(),
                kind: SourceKind::Subrecipe,
                description: sr
                    .description
                    .clone()
                    .unwrap_or_else(|| format!("Subrecipe from {}", sr.path)),
                path: PathBuf::from(&sr.path),
                content: String::new(),
            });
        }
    }

    fn add_local_recipes(
        &self,
        working_dir: &Path,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let dirs = [
            working_dir.to_path_buf(),
            working_dir.join(".goose/recipes"),
        ];
        for dir in dirs {
            self.scan_recipes_dir(&dir, SourceKind::Recipe, sources, seen);
        }
    }

    fn add_local_skills(
        &self,
        working_dir: &Path,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let dirs = [
            working_dir.join(".goose/skills"),
            working_dir.join(".claude/skills"),
            working_dir.join(".agents/skills"),
        ];
        for dir in dirs {
            self.scan_skills_dir(&dir, sources, seen);
        }
    }

    fn add_local_agents(
        &self,
        working_dir: &Path,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let dirs = [
            working_dir.join(".goose/agents"),
            working_dir.join(".claude/agents"),
        ];
        for dir in dirs {
            self.scan_agents_dir(&dir, sources, seen);
        }
    }

    fn add_recipe_path_recipes(
        &self,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let recipe_path = match std::env::var("GOOSE_RECIPE_PATH") {
            Ok(p) => p,
            Err(_) => return,
        };

        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in recipe_path.split(separator) {
            self.scan_recipes_dir(Path::new(dir), SourceKind::Recipe, sources, seen);
        }
    }

    fn add_global_recipes(
        &self,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let dir = Paths::config_dir().join("recipes");
        self.scan_recipes_dir(&dir, SourceKind::Recipe, sources, seen);
    }

    fn add_global_skills(
        &self,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let mut dirs = vec![Paths::config_dir().join("skills")];

        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".claude/skills"));
            dirs.push(home.join(".config/agents/skills"));
        }

        for dir in dirs {
            self.scan_skills_dir(&dir, sources, seen);
        }
    }

    fn add_global_agents(
        &self,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let mut dirs = vec![Paths::config_dir().join("agents")];

        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".claude/agents"));
        }

        for dir in dirs {
            self.scan_agents_dir(&dir, sources, seen);
        }
    }

    fn add_builtin_skills(
        &self,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
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
    }

    fn scan_recipes_dir(
        &self,
        dir: &Path,
        kind: SourceKind,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
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
                    });
                }
                Err(e) => {
                    warn!("Failed to parse recipe {}: {}", path.display(), e);
                }
            }
        }
    }

    fn scan_skills_dir(
        &self,
        dir: &Path,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
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

            if let Some(source) = parse_skill_content(&content, skill_file) {
                if !seen.contains(&source.name) {
                    seen.insert(source.name.clone());
                    sources.push(source);
                }
            }
        }
    }

    fn scan_agents_dir(
        &self,
        dir: &Path,
        sources: &mut Vec<Source>,
        seen: &mut std::collections::HashSet<String>,
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

    async fn handle_load(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        self.cleanup_completed_tasks().await;

        let source_name = arguments
            .as_ref()
            .and_then(|args| args.get("source"))
            .and_then(|v| v.as_str());

        let working_dir = self.get_working_dir(session_id).await;

        if source_name.is_none() {
            return self.handle_load_discovery(session_id, &working_dir).await;
        }

        let name = source_name.unwrap();

        if name.starts_with("task_") {
            return self.handle_load_task_result(name).await;
        }

        self.handle_load_source(session_id, name, &working_dir)
            .await
    }

    async fn handle_load_task_result(&self, task_id: &str) -> Result<Vec<Content>, String> {
        let mut completed = self.completed_tasks.lock().await;

        if let Some(task) = completed.remove(task_id) {
            let status = if task.result.is_ok() {
                "✓ Completed"
            } else {
                "✗ Failed"
            };
            let output = match task.result {
                Ok(output) => output,
                Err(error) => format!("Error: {}", error),
            };

            return Ok(vec![Content::text(format!(
                "# Background Task Result: {}\n\n\
                 **Task:** {}\n\
                 **Status:** {}\n\
                 **Duration:** {} ({} turns)\n\n\
                 ## Output\n\n{}",
                task_id,
                task.description,
                status,
                round_duration(task.duration),
                task.turns_taken,
                output
            ))]);
        }

        drop(completed);

        let running = self.background_tasks.lock().await;
        if running.contains_key(task_id) {
            return Err(format!(
                "Task '{}' is still running. Check back later.",
                task_id
            ));
        }

        Err(format!("Task '{}' not found.", task_id))
    }

    async fn handle_load_discovery(
        &self,
        session_id: &str,
        working_dir: &Path,
    ) -> Result<Vec<Content>, String> {
        {
            let mut cache = self.source_cache.lock().await;
            *cache = None;
        }

        let sources = self.get_sources(session_id, working_dir).await;
        let completed = self.completed_tasks.lock().await;

        if sources.is_empty() && completed.is_empty() {
            return Ok(vec![Content::text(
                "No sources available for load/delegate.\n\n\
                 Sources are discovered from:\n\
                 • Current recipe's sub_recipes\n\
                 • .goose/recipes/, .goose/skills/, .goose/agents/\n\
                 • ~/.config/goose/recipes/, skills/, agents/\n\
                 • GOOSE_RECIPE_PATH directories\n\
                 • Builtin skills",
            )]);
        }

        let mut output = String::from("Available sources for load/delegate:\n");

        if !completed.is_empty() {
            output.push_str("\nCompleted Tasks (awaiting retrieval):\n");
            let mut sorted_completed: Vec<_> = completed.values().collect();
            sorted_completed.sort_by_key(|t| &t.id);
            for task in sorted_completed {
                let status = if task.result.is_ok() {
                    "completed"
                } else {
                    "failed"
                };
                output.push_str(&format!(
                    "• {} - \"{}\" ({})\n",
                    task.id, task.description, status
                ));
            }
        }

        for kind in [
            SourceKind::Subrecipe,
            SourceKind::Recipe,
            SourceKind::Skill,
            SourceKind::Agent,
            SourceKind::BuiltinSkill,
        ] {
            let kind_sources: Vec<_> = sources.iter().filter(|s| s.kind == kind).collect();
            if !kind_sources.is_empty() {
                output.push_str(&format!("\n{}:\n", kind_plural(kind)));
                for source in kind_sources {
                    output.push_str(&format!(
                        "• {} - {}\n",
                        source.name,
                        truncate(&source.description, 60)
                    ));
                }
            }
        }

        output.push_str("\nUse load(source: \"name\") to load into context.\n");
        output.push_str("Use delegate(source: \"name\") to run as subagent.");

        Ok(vec![Content::text(output)])
    }

    /// Handle load mode - load a specific source by name
    async fn handle_load_source(
        &self,
        session_id: &str,
        name: &str,
        working_dir: &Path,
    ) -> Result<Vec<Content>, String> {
        let source = self.resolve_source(session_id, name, working_dir).await;

        match source {
            Some(source) => {
                let content = source.to_load_text();

                let output = format!(
                    "# Loaded: {} ({})\n\n{}\n\n---\nThis knowledge is now available in your context.",
                    source.name, source.kind, content
                );

                Ok(vec![Content::text(output)])
            }
            None => {
                let sources = self.get_sources(session_id, working_dir).await;
                let suggestions: Vec<&str> = sources
                    .iter()
                    .filter(|s| {
                        s.name.to_lowercase().contains(&name.to_lowercase())
                            || name.to_lowercase().contains(&s.name.to_lowercase())
                    })
                    .take(3)
                    .map(|s| s.name.as_str())
                    .collect();

                let error_msg = if suggestions.is_empty() {
                    format!(
                        "Source '{}' not found. Use load() to see available sources.",
                        name
                    )
                } else {
                    format!(
                        "Source '{}' not found. Did you mean: {}?",
                        name,
                        suggestions.join(", ")
                    )
                };

                Err(error_msg)
            }
        }
    }

    async fn handle_delegate(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, String> {
        self.cleanup_completed_tasks().await;

        let params: DelegateParams = arguments
            .map(|args| serde_json::from_value(serde_json::Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Invalid parameters: {}", e))?
            .unwrap_or(DelegateParams {
                instructions: None,
                source: None,
                parameters: None,
                extensions: None,
                provider: None,
                model: None,
                temperature: None,
                r#async: false,
            });

        self.validate_delegate_params(&params)?;

        let session = self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?;

        if session.session_type == SessionType::SubAgent {
            return Err("Delegated tasks cannot spawn further delegations".to_string());
        }

        if params.r#async {
            return self.handle_async_delegate(session_id, params).await;
        }

        let working_dir = session.working_dir.clone();
        let recipe = self
            .build_delegate_recipe(&params, session_id, &working_dir)
            .await?;

        let task_config = self
            .build_task_config(&params, &recipe, &session)
            .await
            .map_err(|e| format!("Failed to build task config: {}", e))?;

        let agent_config = AgentConfig::new(
            self.context.session_manager.clone(),
            crate::config::permission::PermissionManager::instance(),
            None,
            crate::config::GooseMode::Auto,
        );

        let subagent_session = self
            .context
            .session_manager
            .create_session(
                working_dir,
                "Delegated task".to_string(),
                SessionType::SubAgent,
            )
            .await
            .map_err(|e| format!("Failed to create subagent session: {}", e))?;

        let result = run_complete_subagent_task(
            agent_config,
            recipe,
            task_config,
            true,
            subagent_session.id,
            Some(cancellation_token),
        )
        .await
        .map_err(|e| format!("Delegation failed: {}", e))?;

        Ok(vec![Content::text(result)])
    }

    fn validate_delegate_params(&self, params: &DelegateParams) -> Result<(), String> {
        if params.instructions.is_none() && params.source.is_none() {
            return Err("Must provide 'instructions' or 'source' (or both)".to_string());
        }

        if params.parameters.is_some() && params.source.is_none() {
            return Err("'parameters' can only be used with 'source'".to_string());
        }

        Ok(())
    }

    async fn build_delegate_recipe(
        &self,
        params: &DelegateParams,
        session_id: &str,
        working_dir: &Path,
    ) -> Result<Recipe, String> {
        if let Some(source_name) = &params.source {
            self.build_source_recipe(source_name, params, session_id, working_dir)
                .await
        } else {
            self.build_adhoc_recipe(params)
        }
    }

    fn build_adhoc_recipe(&self, params: &DelegateParams) -> Result<Recipe, String> {
        let task = params
            .instructions
            .as_ref()
            .ok_or("Instructions required for ad-hoc task")?;

        Recipe::builder()
            .version("1.0.0")
            .title("Delegated Task")
            .description("Ad-hoc delegated task")
            .prompt(task)
            .build()
            .map_err(|e| format!("Failed to build recipe: {}", e))
    }

    async fn build_source_recipe(
        &self,
        source_name: &str,
        params: &DelegateParams,
        session_id: &str,
        working_dir: &Path,
    ) -> Result<Recipe, String> {
        let source = self
            .resolve_source(session_id, source_name, working_dir)
            .await
            .ok_or_else(|| format!("Source '{}' not found", source_name))?;

        let mut recipe = match source.kind {
            SourceKind::Recipe | SourceKind::Subrecipe => {
                self.build_recipe_from_source(&source, params, session_id)
                    .await?
            }
            SourceKind::Skill | SourceKind::BuiltinSkill => {
                self.build_recipe_from_skill(&source, params)?
            }
            SourceKind::Agent => self.build_recipe_from_agent(&source, params)?,
        };

        if let Some(extra_instructions) = &params.instructions {
            if recipe.prompt.is_some() {
                let current_prompt = recipe.prompt.take().unwrap();
                recipe.prompt = Some(format!("{}\n\n{}", current_prompt, extra_instructions));
            } else {
                recipe.prompt = Some(extra_instructions.clone());
            }
        }

        Ok(recipe)
    }

    async fn build_recipe_from_source(
        &self,
        source: &Source,
        params: &DelegateParams,
        session_id: &str,
    ) -> Result<Recipe, String> {
        let session = self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?;

        if source.kind == SourceKind::Subrecipe {
            let sub_recipes = session.recipe.as_ref().and_then(|r| r.sub_recipes.as_ref());

            if let Some(sub_recipes) = sub_recipes {
                if let Some(sr) = sub_recipes.iter().find(|sr| sr.name == source.name) {
                    let recipe_file = load_local_recipe_file(&sr.path).map_err(|e| {
                        format!("Failed to load subrecipe '{}': {}", source.name, e)
                    })?;

                    let mut merged: HashMap<String, String> = HashMap::new();
                    if let Some(values) = &sr.values {
                        for (k, v) in values {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                    if let Some(provided_params) = &params.parameters {
                        for (k, v) in provided_params {
                            let value_str = match v {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            merged.insert(k.clone(), value_str);
                        }
                    }
                    let param_values: Vec<(String, String)> = merged.into_iter().collect();

                    return build_recipe_from_template(
                        recipe_file.content,
                        &recipe_file.parent_dir,
                        param_values,
                        None::<fn(&str, &str) -> Result<String, anyhow::Error>>,
                    )
                    .map_err(|e| format!("Failed to build subrecipe: {}", e));
                }
            }
        }

        let recipe_file = load_local_recipe_file(source.path.to_str().unwrap_or(""))
            .map_err(|e| format!("Failed to load recipe '{}': {}", source.name, e))?;

        let param_values: Vec<(String, String)> = params
            .parameters
            .as_ref()
            .map(|p| {
                p.iter()
                    .map(|(k, v)| {
                        let value_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        (k.clone(), value_str)
                    })
                    .collect()
            })
            .unwrap_or_default();

        build_recipe_from_template(
            recipe_file.content,
            &recipe_file.parent_dir,
            param_values,
            None::<fn(&str, &str) -> Result<String, anyhow::Error>>,
        )
        .map_err(|e| format!("Failed to build recipe: {}", e))
    }

    fn build_recipe_from_skill(
        &self,
        source: &Source,
        params: &DelegateParams,
    ) -> Result<Recipe, String> {
        let mut builder = Recipe::builder()
            .version("1.0.0")
            .title(format!("Skill: {}", source.name))
            .description(source.description.clone())
            .instructions(&source.content);

        if params.instructions.is_none() {
            builder = builder.prompt("Apply the skill knowledge to produce a useful result.");
        }

        builder
            .build()
            .map_err(|e| format!("Failed to build recipe from skill: {}", e))
    }

    fn build_recipe_from_agent(
        &self,
        source: &Source,
        params: &DelegateParams,
    ) -> Result<Recipe, String> {
        let agent_content = if source.path.as_os_str().is_empty() {
            return Err("Agent source has no path".to_string());
        } else {
            std::fs::read_to_string(&source.path)
                .map_err(|e| format!("Failed to read agent file: {}", e))?
        };

        let (metadata, _): (AgentMetadata, String) =
            parse_frontmatter(&agent_content).ok_or("Failed to parse agent frontmatter")?;

        let model = metadata
            .model
            .map(|m| translate_model_shorthand(&m).to_string());

        let settings = model.map(|m| Settings {
            goose_model: Some(m),
            goose_provider: params.provider.clone(),
            temperature: params.temperature,
            max_turns: None,
        });

        let mut builder = Recipe::builder()
            .version("1.0.0")
            .title(format!("Agent: {}", source.name))
            .description(source.description.clone())
            .instructions(&source.content);

        if let Some(settings) = settings {
            builder = builder.settings(settings);
        }

        if params.instructions.is_none() {
            builder = builder.prompt("Proceed with your expertise to produce a useful result.");
        }

        builder
            .build()
            .map_err(|e| format!("Failed to build recipe from agent: {}", e))
    }

    async fn build_task_config(
        &self,
        params: &DelegateParams,
        recipe: &Recipe,
        session: &crate::session::Session,
    ) -> Result<TaskConfig, anyhow::Error> {
        let provider = self.resolve_provider(params, recipe, session).await?;

        let mut extensions = self.resolve_extensions(session)?;

        if let Some(filter) = &params.extensions {
            if filter.is_empty() {
                extensions = Vec::new();
            } else {
                extensions.retain(|ext| filter.contains(&ext.name()));
            }
        }

        let max_turns = self.resolve_max_turns(session);

        let mut task_config =
            TaskConfig::new(provider, &session.id, &session.working_dir, extensions);
        task_config.max_turns = Some(max_turns);

        Ok(task_config)
    }

    async fn resolve_provider(
        &self,
        params: &DelegateParams,
        recipe: &Recipe,
        session: &crate::session::Session,
    ) -> Result<Arc<dyn crate::providers::base::Provider>, anyhow::Error> {
        let provider_name = params
            .provider
            .clone()
            .or_else(|| {
                recipe
                    .settings
                    .as_ref()
                    .and_then(|s| s.goose_provider.clone())
            })
            .or_else(|| session.provider_name.clone())
            .ok_or_else(|| anyhow::anyhow!("No provider configured"))?;

        let mut model_config = session
            .model_config
            .clone()
            .unwrap_or_else(|| crate::model::ModelConfig::new("default").unwrap());

        if let Some(model) = &params.model {
            model_config.model_name = translate_model_shorthand(model).to_string();
        } else if let Some(model) = recipe
            .settings
            .as_ref()
            .and_then(|s| s.goose_model.as_ref())
        {
            model_config.model_name = model.clone();
        }

        if let Some(temp) = params.temperature {
            model_config = model_config.with_temperature(Some(temp));
        } else if let Some(temp) = recipe.settings.as_ref().and_then(|s| s.temperature) {
            model_config = model_config.with_temperature(Some(temp));
        }

        providers::create(&provider_name, model_config).await
    }

    fn resolve_extensions(
        &self,
        session: &crate::session::Session,
    ) -> Result<Vec<crate::agents::ExtensionConfig>, anyhow::Error> {
        let extensions = EnabledExtensionsState::from_extension_data(&session.extension_data)
            .map(|s| s.extensions)
            .unwrap_or_else(crate::config::get_enabled_extensions);

        Ok(extensions)
    }

    fn resolve_max_turns(&self, session: &crate::session::Session) -> usize {
        std::env::var("GOOSE_SUBAGENT_MAX_TURNS")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| {
                session
                    .recipe
                    .as_ref()
                    .and_then(|r| r.settings.as_ref())
                    .and_then(|s| s.max_turns)
            })
            .unwrap_or(DEFAULT_SUBAGENT_MAX_TURNS)
    }

    async fn cleanup_completed_tasks(&self) {
        let finished: Vec<(String, BackgroundTask)> = {
            let mut tasks = self.background_tasks.lock().await;
            let ids: Vec<String> = tasks
                .iter()
                .filter(|(_, t)| t.handle.is_finished())
                .map(|(id, _)| id.clone())
                .collect();
            ids.into_iter()
                .filter_map(|id| tasks.remove(&id).map(|t| (id, t)))
                .collect()
        };

        let mut completed = self.completed_tasks.lock().await;

        for (id, task) in finished {
            let duration = task.started_at.elapsed();
            let turns_taken = task.turns.load(Ordering::Relaxed);

            let result = match task.handle.await {
                Ok(Ok(output)) => {
                    info!("Background task {} completed successfully", id);
                    Ok(output)
                }
                Ok(Err(e)) => {
                    warn!("Background task {} failed: {}", id, e);
                    Err(e.to_string())
                }
                Err(e) => {
                    warn!("Background task {} panicked: {}", id, e);
                    Err(format!("Task panicked: {}", e))
                }
            };

            completed.insert(
                id.clone(),
                CompletedTask {
                    id,
                    description: task.description,
                    result,
                    turns_taken,
                    duration,
                },
            );
        }
    }

    /// Get task description from params for MOIM display
    fn get_task_description(params: &DelegateParams) -> String {
        if let Some(source) = &params.source {
            if let Some(instructions) = &params.instructions {
                format!("{}: {}", source, truncate(instructions, 30))
            } else {
                source.clone()
            }
        } else if let Some(instructions) = &params.instructions {
            truncate(instructions, 40)
        } else {
            "Unknown task".to_string()
        }
    }

    async fn handle_async_delegate(
        &self,
        session_id: &str,
        params: DelegateParams,
    ) -> Result<Vec<Content>, String> {
        let task_count = self.background_tasks.lock().await.len();
        let max_tasks = max_background_tasks();
        if task_count >= max_tasks {
            return Err(format!(
                "Maximum {} background tasks already running. Wait for completion or use sync mode.",
                max_tasks
            ));
        }

        let session = self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?;

        let working_dir = session.working_dir.clone();
        let recipe = self
            .build_delegate_recipe(&params, session_id, &working_dir)
            .await?;

        let task_config = self
            .build_task_config(&params, &recipe, &session)
            .await
            .map_err(|e| format!("Failed to build task config: {}", e))?;

        let task_id = generate_task_id();
        let description = truncate(&Self::get_task_description(&params), 40);

        let agent_config = AgentConfig::new(
            self.context.session_manager.clone(),
            crate::config::permission::PermissionManager::instance(),
            None,
            crate::config::GooseMode::Auto,
        );

        let subagent_session = self
            .context
            .session_manager
            .create_session(
                working_dir.clone(),
                format!("Background task: {}", task_id),
                SessionType::SubAgent,
            )
            .await
            .map_err(|e| format!("Failed to create subagent session: {}", e))?;

        let turns = Arc::new(AtomicU32::new(0));
        let last_activity = Arc::new(AtomicU64::new(current_epoch_millis()));

        let turns_clone = Arc::clone(&turns);
        let last_activity_clone = Arc::clone(&last_activity);
        let subagent_session_id = subagent_session.id.clone();

        let on_message: OnMessageCallback = Arc::new(move |_msg| {
            turns_clone.fetch_add(1, Ordering::Relaxed);
            last_activity_clone.store(current_epoch_millis(), Ordering::Relaxed);
        });

        let handle = tokio::spawn(async move {
            run_subagent_task_with_callback(
                agent_config,
                recipe,
                task_config,
                true,
                subagent_session_id,
                Some(CancellationToken::new()),
                Some(on_message),
            )
            .await
        });

        let task = BackgroundTask {
            id: task_id.clone(),
            description: description.clone(),
            started_at: Instant::now(),
            turns,
            last_activity,
            handle,
        };

        self.background_tasks
            .lock()
            .await
            .insert(task_id.clone(), task);

        Ok(vec![Content::text(format!(
            "Task {} started in background: \"{}\"\nStatus will appear in context.",
            task_id, description
        ))])
    }
}

#[async_trait]
impl McpClientTrait for SummonClient {
    async fn list_tools(
        &self,
        session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        self.cleanup_completed_tasks().await;

        let is_subagent = self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
            .map(|s| s.session_type == SessionType::SubAgent)
            .unwrap_or(false);

        let mut tools = vec![self.create_load_tool()];

        if !is_subagent {
            tools.push(self.create_delegate_tool());
        }

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "load" => self.handle_load(session_id, arguments).await,
            "delegate" => {
                self.handle_delegate(session_id, arguments, cancellation_token)
                    .await
            }
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match content {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                error
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn get_moim(&self, _session_id: &str) -> Option<String> {
        self.cleanup_completed_tasks().await;

        let running = self.background_tasks.lock().await;
        let completed = self.completed_tasks.lock().await;

        if running.is_empty() && completed.is_empty() {
            return None;
        }

        let mut lines = vec!["Background tasks:".to_string()];
        let now = current_epoch_millis();

        let mut shortest_elapsed_secs: Option<u64> = None;

        let mut sorted_running: Vec<_> = running.values().collect();
        sorted_running.sort_by_key(|t| &t.id);

        for task in sorted_running {
            let elapsed = task.started_at.elapsed();
            let idle_ms = now.saturating_sub(task.last_activity.load(Ordering::Relaxed));

            let elapsed_secs = elapsed.as_secs();
            shortest_elapsed_secs =
                Some(shortest_elapsed_secs.map_or(elapsed_secs, |s| s.min(elapsed_secs)));

            lines.push(format!(
                "• {}: \"{}\" - running {}, {} turns, idle {}",
                task.id,
                task.description,
                round_duration(elapsed),
                task.turns.load(Ordering::Relaxed),
                round_duration(Duration::from_millis(idle_ms)),
            ));
        }

        let mut sorted_completed: Vec<_> = completed.values().collect();
        sorted_completed.sort_by_key(|t| &t.id);

        for task in sorted_completed {
            let status = if task.result.is_ok() {
                "completed"
            } else {
                "failed"
            };
            lines.push(format!(
                "• {}: \"{}\" - {} in {} ({} turns) - use load(\"{}\") to get result",
                task.id,
                task.description,
                status,
                round_duration(task.duration),
                task.turns_taken,
                task.id
            ));
        }

        if let Some(shortest) = shortest_elapsed_secs {
            let sleep_secs = 300u64.saturating_sub(shortest).max(10);
            lines.push(format!(
                "\n→ if you're idle, sleep {} to wait for running tasks",
                sleep_secs
            ));
        }

        Some(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_context() -> PlatformExtensionContext {
        PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(crate::session::SessionManager::instance()),
        }
    }

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
        assert!(source.description.contains("claude-sonnet-4-20250514"));

        assert!(parse_skill_content("no frontmatter", PathBuf::new()).is_none());
        assert!(parse_skill_content("---\nunclosed", PathBuf::new()).is_none());
    }

    #[tokio::test]
    async fn test_source_discovery_and_priority() {
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

        let client = SummonClient::new(create_test_context()).unwrap();
        let sources = client.discover_filesystem_sources(temp_dir.path());

        let skill = sources.iter().find(|s| s.name == "my-skill").unwrap();
        assert_eq!(skill.description, "goose version");

        assert!(sources
            .iter()
            .any(|s| s.name == "test" && s.kind == SourceKind::Recipe));

        assert!(sources.iter().any(|s| s.kind == SourceKind::BuiltinSkill));
    }

    #[tokio::test]
    async fn test_client_tools_and_unknown_tool() {
        let client = SummonClient::new(create_test_context()).unwrap();

        let result = client
            .list_tools("test", None, CancellationToken::new())
            .await
            .unwrap();
        let names: Vec<_> = result.tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"load") && names.contains(&"delegate"));

        let result = client
            .call_tool("test", "unknown", None, CancellationToken::new())
            .await
            .unwrap();
        assert!(result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_duration_rounding_for_moim() {
        assert_eq!(round_duration(Duration::from_secs(5)), "0s");
        assert_eq!(round_duration(Duration::from_secs(15)), "10s");
        assert_eq!(round_duration(Duration::from_secs(59)), "50s");

        assert_eq!(round_duration(Duration::from_secs(60)), "1m");
        assert_eq!(round_duration(Duration::from_secs(90)), "1m");
        assert_eq!(round_duration(Duration::from_secs(120)), "2m");
    }

    #[test]
    fn test_task_description_formatting() {
        let make_params = |source: Option<&str>, instructions: Option<&str>| DelegateParams {
            source: source.map(String::from),
            instructions: instructions.map(String::from),
            parameters: None,
            extensions: None,
            provider: None,
            model: None,
            temperature: None,
            r#async: false,
        };

        assert_eq!(
            SummonClient::get_task_description(&make_params(Some("recipe"), None)),
            "recipe"
        );
        assert_eq!(
            SummonClient::get_task_description(&make_params(None, Some("do stuff"))),
            "do stuff"
        );
        assert_eq!(
            SummonClient::get_task_description(&make_params(Some("r"), Some("task"))),
            "r: task"
        );

        let long = "x".repeat(100);
        let desc = SummonClient::get_task_description(&make_params(None, Some(&long)));
        assert!(desc.len() <= 43 && desc.ends_with("..."));
    }

    fn extract_text(content: &Content) -> &str {
        use rmcp::model::RawContent;
        match &content.raw {
            RawContent::Text(t) => t.text.as_str(),
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_async_task_result_lifecycle() {
        let client = SummonClient::new(create_test_context()).unwrap();
        let temp_dir = TempDir::new().unwrap();

        // 1. Loading unknown task returns error
        let result = client.handle_load_task_result("task_unknown").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        // 2. Add a running task - loading it should fail with "still running"
        {
            let mut running = client.background_tasks.lock().await;
            running.insert(
                "task_running".to_string(),
                BackgroundTask {
                    id: "task_running".to_string(),
                    description: "Running task".to_string(),
                    started_at: Instant::now(),
                    turns: Arc::new(AtomicU32::new(2)),
                    last_activity: Arc::new(AtomicU64::new(current_epoch_millis())),
                    handle: tokio::spawn(async {
                        tokio::time::sleep(Duration::from_secs(1000)).await;
                        Ok("done".to_string())
                    }),
                },
            );
        }
        let result = client.handle_load_task_result("task_running").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("still running"));

        // 3. MOIM shows running task with sleep hint
        let moim = client.get_moim("test").await.unwrap();
        assert!(moim.contains("task_running"));
        assert!(moim.contains("running"));
        assert!(moim.contains("sleep"));

        // 4. Add completed tasks (success and failure)
        {
            let mut completed = client.completed_tasks.lock().await;
            completed.insert(
                "task_success".to_string(),
                CompletedTask {
                    id: "task_success".to_string(),
                    description: "Successful task".to_string(),
                    result: Ok("Task completed successfully with output".to_string()),
                    turns_taken: 5,
                    duration: Duration::from_secs(60),
                },
            );
            completed.insert(
                "task_failed".to_string(),
                CompletedTask {
                    id: "task_failed".to_string(),
                    description: "Failed task".to_string(),
                    result: Err("Something went wrong".to_string()),
                    turns_taken: 3,
                    duration: Duration::from_secs(30),
                },
            );
        }

        // 5. MOIM shows completed tasks with load hints
        let moim = client.get_moim("test").await.unwrap();
        assert!(moim.contains("task_success"));
        assert!(moim.contains("task_failed"));
        assert!(moim.contains(r#"use load("task_success") to get result"#));
        assert!(moim.contains(r#"use load("task_failed") to get result"#));

        // 6. Discovery lists completed tasks
        let discovery = client
            .handle_load_discovery("test", temp_dir.path())
            .await
            .unwrap();
        let discovery_text = extract_text(&discovery[0]);
        assert!(discovery_text.contains("Completed Tasks (awaiting retrieval)"));
        assert!(discovery_text.contains("task_success"));
        assert!(discovery_text.contains("task_failed"));

        // 7. Load successful task - verify output format and removal
        let result = client
            .handle_load_task_result("task_success")
            .await
            .unwrap();
        let text = extract_text(&result[0]);
        assert!(text.contains("task_success"));
        assert!(text.contains("Successful task"));
        assert!(text.contains("✓ Completed"));
        assert!(text.contains("1m"));
        assert!(text.contains("5 turns"));
        assert!(text.contains("Task completed successfully with output"));

        // Verify task was removed after retrieval
        assert!(!client
            .completed_tasks
            .lock()
            .await
            .contains_key("task_success"));

        // 8. Load failed task - verify error format
        let result = client.handle_load_task_result("task_failed").await.unwrap();
        let text = extract_text(&result[0]);
        assert!(text.contains("✗ Failed"));
        assert!(text.contains("Error: Something went wrong"));

        // 9. Loading same task again returns "not found"
        let result = client.handle_load_task_result("task_failed").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        // 10. MOIM with only running tasks shows sleep hint, no completed section
        let moim = client.get_moim("test").await.unwrap();
        assert!(moim.contains("task_running"));
        assert!(moim.contains("sleep"));
        assert!(!moim.contains("task_success"));
        assert!(!moim.contains("task_failed"));
    }
}
