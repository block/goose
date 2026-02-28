//! Summon Extension - Unified tooling for recipes, skills, and subagents
//!
//! Provides two tools:
//! - `load`: Inject knowledge into current context or discover available sources
//! - `delegate`: Run tasks in isolated subagents (sync or async)

use crate::agents::builtin_skills;
use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::subagent_handler::{run_subagent_task, OnMessageCallback, SubagentRunParams};
use crate::agents::subagent_task_config::{TaskConfig, DEFAULT_SUBAGENT_MAX_TURNS};
use crate::agents::AgentConfig;
use crate::config::paths::Paths;
use crate::config::Config;
use crate::providers;
use crate::recipe::build_recipe::build_recipe_from_template;
use crate::recipe::local_recipes::load_local_recipe_file;
use crate::recipe::{Recipe, Settings, RECIPE_FILE_EXTENSIONS};
use crate::session::extension_data::EnabledExtensionsState;
use crate::session::SessionType;
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, ServerNotification, Tool, ToolsCapability,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Mutex};

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
    /// Format the source content for loading into context
    pub fn to_load_text(&self) -> String {
        format!(
            "## {} ({})\n\n{}\n\n### Content\n\n{}",
            self.name, self.kind, self.description, self.content
        )
    }
}

fn kind_plural(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Subrecipe => "Subrecipes",
        SourceKind::Recipe => "Recipes",
        SourceKind::Skill => "Skills",
        SourceKind::Agent => "Agents",
        SourceKind::BuiltinSkill => "Builtin Skills",
    }
}

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

pub struct BackgroundTask {
    pub id: String,
    pub description: String,
    pub started_at: Instant,
    pub turns: Arc<AtomicU32>,
    pub last_activity: Arc<AtomicU64>,
    pub handle: JoinHandle<Result<String>>,
    pub cancellation_token: CancellationToken,
    pub notification_buffer: Arc<Mutex<Vec<ServerNotification>>>,
}

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
        supporting_files: Vec::new(),
    })
}

fn parse_agent_content(content: &str, path: PathBuf) -> Option<Source> {
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

/// Scan a directory for skill subdirectories containing SKILL.md files.
/// Returns discovered skills, skipping any whose names are already in `seen`.
fn scan_skills_from_dir(dir: &Path, seen: &mut std::collections::HashSet<String>) -> Vec<Source> {
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

fn scan_recipes_from_dir(
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
                    supporting_files: Vec::new(),
                });
            }
            Err(e) => {
                warn!("Failed to parse recipe {}: {}", path.display(), e);
            }
        }
    }
}

fn scan_agents_from_dir(
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

fn discover_filesystem_sources(working_dir: &Path) -> Vec<Source> {
    let mut sources: Vec<Source> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

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

/// Collect all files in a skill directory (excluding SKILL.md itself),
/// recursing one level into subdirectories.
fn find_supporting_files(directory: &Path, skill_file: &Path) -> Vec<PathBuf> {
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

fn round_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", (secs / 10) * 10)
    } else {
        format!("{}m", secs / 60)
    }
}

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

fn is_session_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('_').collect();
    parts.len() == 2 && parts[0].len() == 8 && parts[0].chars().all(|c| c.is_ascii_digit())
}

pub struct SummonClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    source_cache: Mutex<Option<(Instant, PathBuf, Vec<Source>)>>,
    background_tasks: Mutex<HashMap<String, BackgroundTask>>,
    completed_tasks: Mutex<HashMap<String, CompletedTask>>,
    notification_subscribers: Arc<Mutex<Vec<mpsc::Sender<ServerNotification>>>>,
}

impl Drop for SummonClient {
    fn drop(&mut self) {
        // Best-effort cancellation of running tasks on shutdown
        if let Ok(tasks) = self.background_tasks.try_lock() {
            for task in tasks.values() {
                task.cancellation_token.cancel();
            }
        }
    }
}

impl SummonClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let instructions = if let Some(session) = &context.session {
            let mut instructions = "".to_string();
            let sources = discover_filesystem_sources(&session.working_dir);

            let mut skills: Vec<&Source> = sources
                .iter()
                .filter(|s| s.kind == SourceKind::Skill || s.kind == SourceKind::BuiltinSkill)
                .collect();

            skills.sort_by(|a, b| (&a.name, &a.path).cmp(&(&b.name, &b.path)));

            if !skills.is_empty() {
                instructions.push_str("\n\nYou have these skills at your disposal, when it is clear they can help you solve a problem or you are asked to use them:");
                for skill in &skills {
                    instructions.push_str(&format!("\n• {} - {}", skill.name, skill.description));
                }
            }
            Some(instructions)
        } else {
            None
        };

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
                extensions: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Summon".to_string()),
                version: "1.0.0".to_string(),
                description: None,
                icons: None,
                website_url: None,
            },
            instructions,
        };

        Ok(Self {
            info,
            context,
            source_cache: Mutex::new(None),
            background_tasks: Mutex::new(HashMap::new()),
            completed_tasks: Mutex::new(HashMap::new()),
            notification_subscribers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn spawn_notification_bridge(
        mut notif_rx: tokio::sync::mpsc::UnboundedReceiver<ServerNotification>,
        subscribers: Arc<Mutex<Vec<mpsc::Sender<ServerNotification>>>>,
        buffer: Arc<Mutex<Vec<ServerNotification>>>,
    ) {
        tokio::spawn(async move {
            while let Some(notification) = notif_rx.recv().await {
                let mut subs = subscribers.lock().await;
                if subs.is_empty() {
                    drop(subs);
                    buffer.lock().await.push(notification);
                } else {
                    subs.retain(|tx| match tx.try_send(notification.clone()) {
                        Ok(()) => true,
                        Err(mpsc::error::TrySendError::Full(_)) => true,
                        Err(mpsc::error::TrySendError::Closed(_)) => false,
                    });
                }
            }
        });
    }

    fn create_load_tool(&self) -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Name of the source to load. If omitted, lists all available sources."
                },
                "cancel": {
                    "type": "boolean",
                    "default": false,
                    "description": "For running background tasks: cancel and return output."
                }
            }
        });

        Tool::new(
            "load",
            "Load knowledge into your current context or discover available sources.\n\n\
             Call with no arguments to list all available sources (subrecipes, recipes, skills, agents).\n\
             Call with a source name to load its content into your context.\n\
             For background tasks: load(source: \"task_id\") waits for the task and returns the result.\n\
             To cancel a running task: load(source: \"task_id\", cancel: true) stops and returns output.\n\n\
             Examples:\n\
             - load() → Lists available sources\n\
             - load(source: \"rust-patterns\") → Loads the rust-patterns skill\n\
             - load(source: \"20260219_1\") → Waits for background task, then returns result"
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
             2. Source-based: Provide `source` name to run a subrecipe, recipe, skill, or agent\n\
             3. Combined: Pair a source with a task (e.g., source: \"rust-patterns\", instructions: \"review auth.rs\")\n\n\
             Effective Delegation:\n\
             - Delegates know only instructions + source content\n\
             - Delegates cannot coordinate. Same-file work = conflicts.\n\
             - Parallel: async: true, then load(taskId) to wait and get results. Single: sync.\n\n\
             Research (read-only): parallelize freely - delegates explore and report back.\n\
             Work (writes): partition files strictly - no two delegates touch the same file.\n\n\
             Decompose → async delegates → load(taskId) for each → synthesize."
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
        discover_filesystem_sources(working_dir)
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

            let description = self.build_subrecipe_description(sr).await;

            sources.push(Source {
                name: sr.name.clone(),
                kind: SourceKind::Subrecipe,
                description,
                path: PathBuf::from(&sr.path),
                content: String::new(),
                supporting_files: Vec::new(),
            });
        }
    }

    async fn build_subrecipe_description(&self, sr: &crate::recipe::SubRecipe) -> String {
        if let Some(desc) = &sr.description {
            return desc.clone();
        }

        if let Ok(recipe_file) = load_local_recipe_file(&sr.path) {
            if let Ok(recipe) = Recipe::from_content(&recipe_file.content) {
                let mut desc = recipe.description.clone();

                if let Some(params) = &recipe.parameters {
                    let param_names: Vec<&str> = params.iter().map(|p| p.key.as_str()).collect();
                    if !param_names.is_empty() {
                        let params_str = param_names.join(", ");
                        desc = format!("{} (params: {})", desc, params_str);
                    }
                }

                return desc;
            }
        }

        format!("Subrecipe from {}", sr.path)
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

        let cancel = arguments
            .as_ref()
            .and_then(|args| args.get("cancel"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let working_dir = self.get_working_dir(session_id).await;

        if source_name.is_none() {
            return self.handle_load_discovery(session_id, &working_dir).await;
        }

        let name = source_name.unwrap();

        if is_session_id(name) {
            return self.handle_load_task_result(name, cancel).await;
        }

        self.handle_load_source(session_id, name, &working_dir)
            .await
    }

    async fn handle_load_task_result(
        &self,
        task_id: &str,
        cancel: bool,
    ) -> Result<Vec<Content>, String> {
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

        let mut running = self.background_tasks.lock().await;
        if running.contains_key(task_id) {
            if cancel {
                let task = running.remove(task_id).unwrap();
                drop(running);

                task.cancellation_token.cancel();

                let duration = task.started_at.elapsed();
                let turns_taken = task.turns.load(Ordering::Relaxed);

                let mut handle = task.handle;
                let output = tokio::select! {
                    result = &mut handle => {
                        match result {
                            Ok(Ok(s)) => s,
                            Ok(Err(e)) => format!("Error: {}", e),
                            Err(e) => format!("Task panicked: {}", e),
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {
                        handle.abort();
                        "Task did not stop in time (aborted)".to_string()
                    }
                };

                return Ok(vec![Content::text(format!(
                    "# Background Task Result: {}\n\n\
                     **Task:** {}\n\
                     **Status:** ⊘ Cancelled\n\
                     **Duration:** {} ({} turns)\n\n\
                     ## Output\n\n{}",
                    task_id,
                    task.description,
                    round_duration(duration),
                    turns_taken,
                    output
                ))]);
            }

            // Wait for the running task to complete, keeping the tool call
            // alive so notifications (subagent tool calls) stream in real time.
            let mut task = running.remove(task_id).unwrap();
            drop(running);

            let buffered = {
                let mut buf = task.notification_buffer.lock().await;
                std::mem::take(&mut *buf)
            };
            if !buffered.is_empty() {
                let subs = self.notification_subscribers.lock().await;
                for notif in buffered {
                    for tx in subs.iter() {
                        let _ = tx.try_send(notif.clone());
                    }
                }
            }

            tokio::select! {
                result = &mut task.handle => {
                    let output = match result {
                        Ok(Ok(s)) => s,
                        Ok(Err(e)) => format!("Error: {}", e),
                        Err(e) => format!("Task panicked: {}", e),
                    };

                    return Ok(vec![Content::text(format!(
                        "# Background Task Result: {}\n\n\
                         **Task:** {}\n\
                         **Status:** ✓ Completed\n\
                         **Duration:** {} ({} turns)\n\n\
                         ## Output\n\n{}",
                        task_id,
                        task.description,
                        round_duration(task.started_at.elapsed()),
                        task.turns.load(Ordering::Relaxed),
                        output
                    ))]);
                }
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    self.background_tasks.lock().await.insert(task_id.to_string(), task);

                    return Err(format!(
                        "Task '{task_id}' is still running after waiting 5 min. \
                         Use load(source: \"{task_id}\") to wait again, or \
                         load(source: \"{task_id}\", cancel: true) to stop."
                    ));
                }
            }
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

                let mut output = format!(
                    "# Loaded: {} ({})\n\n{}\n",
                    source.name, source.kind, content
                );

                if !source.supporting_files.is_empty() {
                    let mut other_files: Vec<std::path::PathBuf> = Vec::new();
                    let canonical_skill_dir = source
                        .path
                        .canonicalize()
                        .unwrap_or_else(|_| source.path.clone());

                    for file in &source.supporting_files {
                        let is_md = file
                            .extension()
                            .is_some_and(|ext| ext == "md" || ext == "mdx");

                        if let Ok(relative) = file.strip_prefix(&source.path) {
                            let safe_to_inline = is_md
                                && file
                                    .canonicalize()
                                    .map(|p| p.starts_with(&canonical_skill_dir))
                                    .unwrap_or(false);

                            if safe_to_inline {
                                match std::fs::read_to_string(file) {
                                    Ok(file_content) => {
                                        output.push_str(&format!(
                                            "\n## {}\n\n{}\n",
                                            relative.display(),
                                            file_content.trim()
                                        ));
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to read supporting file {}: {}",
                                            file.display(),
                                            e
                                        );
                                        other_files.push(relative.to_path_buf());
                                    }
                                }
                            } else {
                                other_files.push(relative.to_path_buf());
                            }
                        }
                    }

                    if !other_files.is_empty() {
                        output.push_str(&format!(
                            "\n## Other Files\n\nSkill directory: {}\n\n",
                            source.path.display()
                        ));
                        for relative in &other_files {
                            output.push_str(&format!("- {}\n", relative.display()));
                        }
                        output.push_str(
                            "\nUse the file tools to read these files or run scripts as directed.\n",
                        );
                    }
                }

                output.push_str("\n---\nThis knowledge is now available in your context.");

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
            true, // disable session naming for subagents
            crate::agents::GoosePlatform::GooseCli,
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

        let (notif_tx, notif_rx) = tokio::sync::mpsc::unbounded_channel::<ServerNotification>();
        Self::spawn_notification_bridge(
            notif_rx,
            Arc::clone(&self.notification_subscribers),
            Arc::new(Mutex::new(Vec::new())),
        );

        let result = run_subagent_task(SubagentRunParams {
            config: agent_config,
            recipe,
            task_config,
            return_last_only: true,
            session_id: subagent_session.id,
            cancellation_token: Some(cancellation_token),
            on_message: None,
            notification_tx: Some(notif_tx),
        })
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

        let model = metadata.model;

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

        let mut extensions = EnabledExtensionsState::extensions_or_default(
            Some(&session.extension_data),
            Config::global(),
        );

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
            .or_else(|| {
                Config::global()
                    .get_param::<String>("GOOSE_SUBAGENT_PROVIDER")
                    .ok()
            })
            .or_else(|| session.provider_name.clone())
            .ok_or_else(|| anyhow::anyhow!("No provider configured"))?;

        let mut model_config = session.model_config.clone().map(Ok).unwrap_or_else(|| {
            crate::model::ModelConfig::new("default")
                .map(|c| c.with_canonical_limits(&provider_name))
        })?;

        if let Some(model) = &params.model {
            model_config.model_name = model.clone();
        } else if let Some(model) = recipe
            .settings
            .as_ref()
            .and_then(|s| s.goose_model.as_ref())
        {
            model_config.model_name = model.clone();
        } else if let Ok(model) = Config::global().get_param::<String>("GOOSE_SUBAGENT_MODEL") {
            model_config.model_name = model;
        }

        if let Some(temp) = params.temperature {
            model_config = model_config.with_temperature(Some(temp));
        } else if let Some(temp) = recipe.settings.as_ref().and_then(|s| s.temperature) {
            model_config = model_config.with_temperature(Some(temp));
        }

        providers::create(&provider_name, model_config, Vec::new()).await
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

        let description = truncate(&Self::get_task_description(&params), 40);

        let agent_config = AgentConfig::new(
            self.context.session_manager.clone(),
            crate::config::permission::PermissionManager::instance(),
            None,
            crate::config::GooseMode::Auto,
            true, // disable session naming for subagents
            crate::agents::GoosePlatform::GooseCli,
        );

        let subagent_session = self
            .context
            .session_manager
            .create_session(working_dir, description.clone(), SessionType::SubAgent)
            .await
            .map_err(|e| format!("Failed to create subagent session: {}", e))?;

        let task_id = subagent_session.id.clone();

        let turns = Arc::new(AtomicU32::new(0));
        let last_activity = Arc::new(AtomicU64::new(current_epoch_millis()));

        let turns_clone = Arc::clone(&turns);
        let last_activity_clone = Arc::clone(&last_activity);

        let on_message: OnMessageCallback = Arc::new(move |_msg| {
            turns_clone.fetch_add(1, Ordering::Relaxed);
            last_activity_clone.store(current_epoch_millis(), Ordering::Relaxed);
        });

        let task_token = CancellationToken::new();
        let task_token_clone = task_token.clone();

        let notification_buffer = Arc::new(Mutex::new(Vec::new()));

        let (notif_tx, notif_rx) = tokio::sync::mpsc::unbounded_channel::<ServerNotification>();
        Self::spawn_notification_bridge(
            notif_rx,
            Arc::clone(&self.notification_subscribers),
            Arc::clone(&notification_buffer),
        );

        let handle = tokio::spawn(async move {
            run_subagent_task(SubagentRunParams {
                config: agent_config,
                recipe,
                task_config,
                return_last_only: true,
                session_id: subagent_session.id,
                cancellation_token: Some(task_token_clone),
                on_message: Some(on_message),
                notification_tx: Some(notif_tx),
            })
            .await
        });

        let task = BackgroundTask {
            id: task_id.clone(),
            description: description.clone(),
            started_at: Instant::now(),
            turns,
            last_activity,
            handle,
            cancellation_token: task_token,
            notification_buffer,
        };

        self.background_tasks
            .lock()
            .await
            .insert(task_id.clone(), task);

        Ok(vec![Content::text(format!(
            "Task {} started in background: \"{}\"\n\
             Continue with other work. When you need the result, use load(source: \"{}\").",
            task_id, description, task_id
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
        _working_dir: Option<&str>,
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

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        let (tx, rx) = mpsc::channel(16);
        self.notification_subscribers.lock().await.push(tx);
        rx
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

        let mut sorted_running: Vec<_> = running.values().collect();
        sorted_running.sort_by_key(|t| &t.id);

        for task in sorted_running {
            let elapsed = task.started_at.elapsed();
            let idle_ms = now.saturating_sub(task.last_activity.load(Ordering::Relaxed));

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

        if !running.is_empty() {
            lines.push(
                "\n→ Use load(source: \"<id>\") to wait for a task, or load(source: \"<id>\", cancel: true) to stop it"
                    .to_string(),
            );
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
            session: None,
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
        assert!(source.description.contains("sonnet"));

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
    async fn test_skill_supporting_files_discovered() {
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

        let client = SummonClient::new(create_test_context()).unwrap();
        let sources = client.discover_filesystem_sources(temp_dir.path());

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

    #[tokio::test]
    async fn test_load_source_inlines_markdown_supporting_files() {
        let temp_dir = TempDir::new().unwrap();

        let skill_dir = temp_dir.path().join(".goose/skills/my-skill");
        fs::create_dir_all(skill_dir.join("references")).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A skill\n---\nSee references.",
        )
        .unwrap();
        fs::write(
            skill_dir.join("references/ops.md"),
            "# Ops Guide\n\nDo the thing.",
        )
        .unwrap();
        fs::write(skill_dir.join("run.sh"), "#!/bin/bash\necho ok").unwrap();

        let client = SummonClient::new(create_test_context()).unwrap();
        let result = client
            .handle_load_source("test", "my-skill", temp_dir.path())
            .await
            .unwrap();

        let text = &result[0].as_text().expect("expected text content").text;

        assert!(
            text.contains("Ops Guide"),
            "md file content should be inlined"
        );
        assert!(text.contains("Do the thing."));
        assert!(text.contains("run.sh"), "non-md file should be listed");
        assert!(
            !text.contains("#!/bin/bash"),
            "script content should not be inlined"
        );
        assert!(
            text.contains("Use the file tools"),
            "non-md files should have file tools message"
        );
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
            .call_tool("test", "unknown", None, None, CancellationToken::new())
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

    #[test]
    fn test_is_session_id() {
        assert!(is_session_id("20260204_1"));
        assert!(is_session_id("20260204_42"));
        assert!(is_session_id("20260204_999"));
        assert!(!is_session_id("task_12345_0001"));
        assert!(!is_session_id("my-recipe"));
        assert!(!is_session_id("2026020_1"));
        assert!(!is_session_id("20260204"));
    }

    #[tokio::test]
    async fn test_async_task_result_lifecycle() {
        let client = SummonClient::new(create_test_context()).unwrap();
        let temp_dir = TempDir::new().unwrap();

        let result = client.handle_load_task_result("20260204_999", false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        {
            use crate::agents::subagent_handler::create_tool_notification;
            use crate::conversation::message::MessageContent;
            use rmcp::model::CallToolRequestParams;

            let tool_call = CallToolRequestParams {
                meta: None,
                task: None,
                name: "developer__shell".to_string().into(),
                arguments: Some(
                    serde_json::json!({"command": "ls"})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            };
            let content = MessageContent::tool_request("req1", Ok(tool_call));
            let notif = create_tool_notification(&content, "20260204_1").unwrap();

            let buffer = Arc::new(Mutex::new(vec![notif]));

            let mut running = client.background_tasks.lock().await;
            running.insert(
                "20260204_1".to_string(),
                BackgroundTask {
                    id: "20260204_1".to_string(),
                    description: "Running task".to_string(),
                    started_at: Instant::now(),
                    turns: Arc::new(AtomicU32::new(2)),
                    last_activity: Arc::new(AtomicU64::new(current_epoch_millis())),
                    handle: tokio::spawn(async {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        Ok("done".to_string())
                    }),
                    cancellation_token: CancellationToken::new(),
                    notification_buffer: buffer,
                },
            );
        }

        let mut subscriber = client.subscribe().await;

        let result = client
            .handle_load_task_result("20260204_1", false)
            .await
            .expect("load should wait and return result");
        let text = extract_text(&result[0]);
        assert!(text.contains("Completed"));
        assert!(text.contains("done"));

        let notif = subscriber
            .try_recv()
            .expect("subscriber should receive buffered notification");
        if let ServerNotification::LoggingMessageNotification(log) = notif {
            let data = log.params.data.as_object().unwrap();
            assert_eq!(
                data.get("subagent_id").and_then(|v| v.as_str()),
                Some("20260204_1")
            );
        } else {
            panic!("expected logging notification");
        }

        {
            let mut completed = client.completed_tasks.lock().await;
            completed.insert(
                "20260204_2".to_string(),
                CompletedTask {
                    id: "20260204_2".to_string(),
                    description: "Successful task".to_string(),
                    result: Ok("Task completed successfully with output".to_string()),
                    turns_taken: 5,
                    duration: Duration::from_secs(60),
                },
            );
            completed.insert(
                "20260204_3".to_string(),
                CompletedTask {
                    id: "20260204_3".to_string(),
                    description: "Failed task".to_string(),
                    result: Err("Something went wrong".to_string()),
                    turns_taken: 3,
                    duration: Duration::from_secs(30),
                },
            );
        }

        let moim = client.get_moim("test").await.unwrap();
        assert!(moim.contains("20260204_2"));
        assert!(moim.contains("20260204_3"));
        assert!(moim.contains(r#"use load("20260204_2") to get result"#));
        assert!(moim.contains(r#"use load("20260204_3") to get result"#));

        let discovery = client
            .handle_load_discovery("test", temp_dir.path())
            .await
            .unwrap();
        let discovery_text = extract_text(&discovery[0]);
        assert!(discovery_text.contains("Completed Tasks (awaiting retrieval)"));
        assert!(discovery_text.contains("20260204_2"));
        assert!(discovery_text.contains("20260204_3"));

        let result = client
            .handle_load_task_result("20260204_2", false)
            .await
            .unwrap();
        let text = extract_text(&result[0]);
        assert!(text.contains("20260204_2"));
        assert!(text.contains("Successful task"));
        assert!(text.contains("✓ Completed"));
        assert!(text.contains("1m"));
        assert!(text.contains("5 turns"));
        assert!(text.contains("Task completed successfully with output"));

        assert!(!client
            .completed_tasks
            .lock()
            .await
            .contains_key("20260204_2"));

        let result = client
            .handle_load_task_result("20260204_3", false)
            .await
            .unwrap();
        let text = extract_text(&result[0]);
        assert!(text.contains("✗ Failed"));
        assert!(text.contains("Error: Something went wrong"));

        let result = client.handle_load_task_result("20260204_3", false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        // All tasks consumed -- moim should be empty
        assert!(client.get_moim("test").await.is_none());
    }

    #[tokio::test]
    async fn test_cancel_running_task() {
        let client = SummonClient::new(create_test_context()).unwrap();
        let token = CancellationToken::new();

        {
            let mut running = client.background_tasks.lock().await;
            running.insert(
                "20260204_1".to_string(),
                BackgroundTask {
                    id: "20260204_1".to_string(),
                    description: "Cancellable task".to_string(),
                    started_at: Instant::now(),
                    turns: Arc::new(AtomicU32::new(3)),
                    last_activity: Arc::new(AtomicU64::new(current_epoch_millis())),
                    handle: tokio::spawn(async {
                        tokio::time::sleep(Duration::from_secs(1000)).await;
                        Ok("should not see this".to_string())
                    }),
                    cancellation_token: token.clone(),
                    notification_buffer: Arc::new(Mutex::new(Vec::new())),
                },
            );
        }

        let result = client
            .handle_load_task_result("20260204_1", true)
            .await
            .unwrap();
        let text = extract_text(&result[0]);
        assert!(text.contains("Cancelled"));
        assert!(text.contains("20260204_1"));
        assert!(text.contains("Cancellable task"));
        assert!(token.is_cancelled());
        assert!(!client
            .background_tasks
            .lock()
            .await
            .contains_key("20260204_1"));
    }
}
