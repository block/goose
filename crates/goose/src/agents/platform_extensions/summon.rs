use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::subagent_handler::{run_subagent_task, OnMessageCallback, SubagentRunParams};
use crate::agents::subagent_task_config::{TaskConfig, DEFAULT_SUBAGENT_MAX_TURNS};
use crate::agents::tool_execution::ToolCallContext;
use crate::agents::AgentConfig;
use crate::config::paths::Paths;
use crate::config::{Config, GooseMode};
use crate::providers;
use crate::recipe::build_recipe::build_recipe_from_template;
use crate::recipe::local_recipes::load_local_recipe_file;
use crate::recipe::{Recipe, RecipeParameter, Settings, RECIPE_FILE_EXTENSIONS};
use crate::session::extension_data::{EnabledExtensionsState, ExtensionState};
use crate::session::SessionType;
use crate::sources::parse_frontmatter;
use crate::utils::safe_truncate;
use anyhow::Result;
use async_trait::async_trait;
use goose_sdk_types::custom_requests::{SourceEntry, SourceType};
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, InitializeResult, JsonObject, ListToolsResult,
    MetaObject, ServerCapabilities, ServerNotification, Tool,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Mutex};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

pub static EXTENSION_NAME: &str = "summon";

const SUBAGENT_DESCRIPTION_BUDGET: usize = 160;

const TASK_LABEL_BUDGET: usize = 60;

const DEFAULT_WORKER_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const DEFAULT_MAX_WORKERS: usize = 8;

fn kind_plural(kind: SourceType) -> &'static str {
    match kind {
        SourceType::Subrecipe => "Subrecipes",
        SourceType::Recipe => "Recipes",
        SourceType::Agent => "Agents",
        _ => "Other",
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DelegateParams {
    pub instructions: Option<String>,
    pub source: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub extensions: Option<Vec<String>>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_turns: Option<usize>,
    pub context: Option<String>,
    pub working_dir: Option<String>,
    #[serde(default)]
    pub r#async: bool,
    pub worker: Option<String>,
}

pub struct PersistentWorker {
    pub configured: Arc<crate::agents::subagent_handler::ConfiguredSubagent>,
    pub session_id: String,
    pub identity: String,
    pub recipe: Recipe,
    pub task_config: TaskConfig,
    pub default_max_turns: usize,
    pub prompt_max_turns: AtomicUsize,
    pub busy: Arc<Mutex<()>>,
    pub last_used: AtomicU64,
    pub persisted: AtomicBool,
    pub notification_tx: tokio::sync::mpsc::UnboundedSender<ServerNotification>,
    pub creation: WorkerCreationParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCreationParams {
    pub source: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub extensions_filter: Option<Vec<String>>,
    pub extensions: HashSet<String>,
    pub provider: String,
    pub model_request: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub working_dir: PathBuf,
}

impl WorkerCreationParams {
    fn resolve(
        params: &DelegateParams,
        model_request: Option<String>,
        task_config: &TaskConfig,
    ) -> Self {
        let working_dir = task_config
            .parent_working_dir
            .canonicalize()
            .unwrap_or_else(|_| task_config.parent_working_dir.clone());
        Self {
            source: params.source.clone(),
            parameters: params.parameters.clone(),
            extensions_filter: params.extensions.clone(),
            extensions: task_config.extensions.iter().map(|e| e.name()).collect(),
            provider: task_config.provider.get_name().to_string(),
            model_request,
            model: task_config.model_config.model_name.clone(),
            temperature: task_config.model_config.temperature,
            working_dir,
        }
    }
}

pub enum WorkerSlot {
    Creating,
    Ready(Arc<PersistentWorker>),
}

// Two SummonClient instances can briefly coexist for one parent session
// (e.g. an agent evicted and recreated while an old handle finishes a call),
// so worker exclusion is shared process-wide instead of living on per-client
// state.
type SharedLockKey = (usize, String);
type SharedLockRegistry = std::sync::Mutex<HashMap<SharedLockKey, std::sync::Weak<Mutex<()>>>>;

static WORKER_BUSY_LOCKS: std::sync::LazyLock<SharedLockRegistry> =
    std::sync::LazyLock::new(Default::default);
static WORKER_DELEGATION_LOCKS: std::sync::LazyLock<SharedLockRegistry> =
    std::sync::LazyLock::new(Default::default);

fn shared_lock(
    registry: &SharedLockRegistry,
    session_manager: &crate::session::SessionManager,
    key: &str,
) -> Arc<Mutex<()>> {
    // Session ids are only unique within one store, so keys include the
    // store's identity; a live lock keeps its store alive, so a reused
    // address cannot alias a live entry.
    let key = (
        Arc::as_ptr(session_manager.storage()) as usize,
        key.to_string(),
    );
    let mut locks = registry.lock().unwrap();
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&key).and_then(std::sync::Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(key, Arc::downgrade(&lock));
    lock
}

// Removes the Creating placeholder if creation fails or the call is cancelled
// mid-await, so the worker name is not permanently stuck reporting busy.
struct CreationSlotGuard<'a> {
    workers: &'a std::sync::Mutex<HashMap<String, WorkerSlot>>,
    key: Option<String>,
}

impl CreationSlotGuard<'_> {
    fn release(mut self) -> String {
        self.key.take().expect("release called once")
    }
}

impl Drop for CreationSlotGuard<'_> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            if let Ok(mut workers) = self.workers.lock() {
                workers.remove(&key);
            }
        }
    }
}

// The resolved runtime state is persisted verbatim so a restored worker
// matches its in-memory counterpart even if sources or config changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRecord {
    pub session_id: String,
    #[serde(default)]
    pub identity: String,
    pub recipe: Recipe,
    pub default_max_turns: usize,
    pub extensions: Vec<crate::config::ExtensionConfig>,
    pub model_config: goose_providers::model::ModelConfig,
    pub creation: WorkerCreationParams,
}

impl WorkerRecord {
    fn from_worker(worker: &PersistentWorker) -> Self {
        Self {
            session_id: worker.session_id.clone(),
            identity: worker.identity.clone(),
            recipe: worker.recipe.clone(),
            default_max_turns: worker.default_max_turns,
            extensions: worker.task_config.extensions.clone(),
            model_config: worker.task_config.model_config.clone(),
            creation: worker.creation.clone(),
        }
    }

    fn to_delegate_params(&self, worker_name: &str) -> DelegateParams {
        DelegateParams {
            source: self.creation.source.clone(),
            parameters: self.creation.parameters.clone(),
            provider: Some(self.creation.provider.clone()),
            model: Some(
                self.creation
                    .model_request
                    .clone()
                    .unwrap_or_else(|| self.creation.model.clone()),
            ),
            temperature: self.creation.temperature,
            worker: Some(worker_name.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkerRegistryState {
    pub workers: HashMap<String, WorkerRecord>,
}

impl ExtensionState for WorkerRegistryState {
    const EXTENSION_NAME: &'static str = "summon_workers";
    const VERSION: &'static str = "v0";
}

// Stored in the worker session itself. Session ids are date-based and can be
// reused after deletion, so a record's session_id alone cannot prove the
// session is still the worker's; this marker ties them together.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkerIdentityState {
    pub identity: String,
}

impl ExtensionState for WorkerIdentityState {
    const EXTENSION_NAME: &'static str = "summon_worker_identity";
    const VERSION: &'static str = "v0";
}

fn worker_followup_user_text(params: &DelegateParams) -> String {
    let instructions = params
        .instructions
        .clone()
        .expect("validated by validate_worker_reuse_params");
    match &params.context {
        Some(context) => format!("Context:\n{}\n\n{}", context, instructions),
        None => instructions,
    }
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
    pub completed_at: Instant,
}

fn requested_model_override(params: &DelegateParams, recipe: &Recipe) -> Option<String> {
    params
        .model
        .clone()
        .or_else(|| recipe.settings.as_ref().and_then(|s| s.goose_model.clone()))
        .or_else(|| {
            Config::global()
                .get_param::<String>("GOOSE_SUBAGENT_MODEL")
                .ok()
        })
}

fn validate_worker_reuse_params(
    params: &DelegateParams,
    creation: &WorkerCreationParams,
    session_working_dir: &Path,
) -> Result<(), String> {
    let mut changed = Vec::new();
    if params.source.is_some() && params.source != creation.source {
        changed.push("source");
    }
    if params.parameters.is_some() && params.parameters != creation.parameters {
        changed.push("parameters");
    }
    if let Some(filter) = &params.extensions {
        let resent: HashSet<String> = filter.iter().cloned().collect();
        if Some(filter) != creation.extensions_filter.as_ref() && resent != creation.extensions {
            changed.push("extensions");
        }
    }
    if let Some(provider) = &params.provider {
        if provider != &creation.provider {
            changed.push("provider");
        }
    }
    if let Some(model) = &params.model {
        let matches = match &creation.model_request {
            Some(requested) => model == requested,
            None => model == &creation.model,
        };
        if !matches {
            changed.push("model");
        }
    }
    if let Some(temperature) = params.temperature {
        if Some(temperature) != creation.temperature {
            changed.push("temperature");
        }
    }
    if let Some(dir) = &params.working_dir {
        let resolved = resolve_working_dir(session_working_dir, dir).map_err(|e| e.to_string())?;
        if resolved != creation.working_dir {
            changed.push("working_dir");
        }
    }
    if !changed.is_empty() {
        return Err(format!(
            "Worker already exists; {} cannot change after creation. \
             Only 'instructions', 'context', and 'max_turns' apply to follow-up delegations.",
            changed.join(", ")
        ));
    }
    if params.instructions.is_none() {
        return Err("'instructions' is required when delegating to an existing worker".to_string());
    }
    Ok(())
}

fn merge_subrecipe_parameters(
    fixed_values: Option<&HashMap<String, String>>,
    provided_parameters: Option<&HashMap<String, serde_json::Value>>,
) -> HashMap<String, String> {
    let mut merged = fixed_values.cloned().unwrap_or_default();
    if let Some(provided_parameters) = provided_parameters {
        for (key, value) in provided_parameters {
            let value = match value {
                serde_json::Value::String(value) => value.clone(),
                other => other.to_string(),
            };
            merged.entry(key.clone()).or_insert(value);
        }
    }
    merged
}

/// Result from handle_load_task_result with structured metadata for the caller
#[derive(Debug)]
struct TaskLoadResult {
    content: Vec<ContentBlock>,
    status: &'static str,
    turns: Option<u32>,
    duration_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AgentMetadata {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

fn parse_agent_content(content: &str, path: &Path) -> Option<SourceEntry> {
    let (metadata, body): (AgentMetadata, String) = match parse_frontmatter(content) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return None,
        Err(e) => {
            // Missing fields means this file has valid YAML but isn't an agent — skip silently.
            // Only warn on actual YAML syntax errors.
            if e.to_string().contains("missing field") {
                return None;
            }
            warn!("Failed to parse agent file {}: {}", path.display(), e);
            return None;
        }
    };

    let description = metadata.description.unwrap_or_else(|| {
        let model_info = metadata
            .model
            .as_ref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        format!("Agent{}", model_info)
    });

    Some(SourceEntry {
        source_type: SourceType::Agent,
        name: metadata.name,
        description,
        content: body,
        path: path.to_string_lossy().into_owned(),
        global: false,
        writable: true,
        supporting_files: Vec::new(),
        properties: std::collections::HashMap::new(),
    })
}

fn scan_recipes_from_dir(
    dir: &Path,
    kind: SourceType,
    suppress_config_warnings: bool,
    sources: &mut Vec<SourceEntry>,
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
                sources.push(SourceEntry {
                    source_type: kind,
                    name,
                    description: recipe.description.clone(),
                    content: recipe.instructions.clone().unwrap_or_default(),
                    path: path.to_string_lossy().into_owned(),
                    global: false,
                    writable: true,
                    supporting_files: Vec::new(),
                    properties: std::collections::HashMap::new(),
                });
            }
            Err(e) => {
                // The working directory commonly contains project config like package.json
                // and tsconfig.json, which parse as valid JSON but lack Recipe fields. In that
                // case treat them as "not a recipe" rather than warning. Dedicated recipe
                // directories still warn so a real recipe with a typo is not silently dropped.
                if suppress_config_warnings && e.to_string().contains("missing field") {
                    continue;
                }
                warn!("Failed to parse recipe {}: {}", path.display(), e);
            }
        }
    }
}

fn scan_agents_from_dir(
    dir: &Path,
    sources: &mut Vec<SourceEntry>,
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

        if let Some(source) = parse_agent_content(&content, &path) {
            if !seen.contains(&source.name) {
                seen.insert(source.name.clone());
                sources.push(source);
            }
        }
    }
}

pub fn discover_filesystem_sources(working_dir: &Path) -> Vec<SourceEntry> {
    let mut sources: Vec<SourceEntry> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let home = dirs::home_dir();
    let config = Paths::config_dir();

    let local_recipe_dirs: Vec<PathBuf> = vec![
        working_dir.join(".goose/recipes"),
        working_dir.join(".agents/recipes"),
    ];

    let global_recipe_dirs: Vec<PathBuf> = std::env::var("GOOSE_RECIPE_PATH")
        .ok()
        .into_iter()
        .flat_map(|p| {
            let sep = if cfg!(windows) { ';' } else { ':' };
            p.split(sep).map(PathBuf::from).collect::<Vec<_>>()
        })
        .chain(
            [
                home.as_ref().map(|h| h.join(".goose/recipes")),
                Some(config.join("recipes")),
                home.as_ref().map(|h| h.join(".agents/recipes")),
            ]
            .into_iter()
            .flatten(),
        )
        .collect();

    let local_agent_dirs: Vec<PathBuf> = vec![
        working_dir.join(".goose/agents"),
        working_dir.join(".claude/agents"),
        working_dir.join(".agents/agents"),
    ];

    let global_agent_dirs: Vec<PathBuf> = [
        home.as_ref().map(|h| h.join(".goose/agents")),
        home.as_ref().map(|h| h.join(".agents/agents")),
        Some(config.join("agents")),
        home.as_ref().map(|h| h.join(".claude/agents")),
    ]
    .into_iter()
    .flatten()
    .collect();

    scan_recipes_from_dir(
        working_dir,
        SourceType::Recipe,
        true,
        &mut sources,
        &mut seen,
    );

    for dir in local_recipe_dirs {
        scan_recipes_from_dir(&dir, SourceType::Recipe, false, &mut sources, &mut seen);
    }

    for dir in local_agent_dirs {
        scan_agents_from_dir(&dir, &mut sources, &mut seen);
    }

    for dir in global_recipe_dirs {
        scan_recipes_from_dir(&dir, SourceType::Recipe, false, &mut sources, &mut seen);
    }

    for dir in global_agent_dirs {
        scan_agents_from_dir(&dir, &mut sources, &mut seen);
    }

    sources
}

fn build_instructions_with_context(context: &str, instructions: &str) -> String {
    let mut result = format!("# Reference Context\n\n{}", context);
    if !instructions.is_empty() {
        result.push_str(&format!("\n\n# Task Instructions\n\n{}", instructions));
    }
    result
}

fn build_subagent_instructions(session: Option<&crate::session::Session>) -> String {
    let Some(session) = session else {
        return String::new();
    };

    // filter the sources down to what we want even though currently that is what we get
    let mut sources: Vec<SourceEntry> = discover_filesystem_sources(&session.working_dir)
        .into_iter()
        .filter(|s| {
            matches!(
                s.source_type,
                SourceType::Agent | SourceType::Recipe | SourceType::Subrecipe
            )
        })
        .collect();

    // If the session is started from a recipe, also use the subrecipes for
    // that recipe as delegate targets
    if let Some(recipe) = session.recipe.as_ref() {
        if let Some(subs) = recipe.sub_recipes.as_ref() {
            let mut seen: std::collections::HashSet<String> =
                sources.iter().map(|s| s.name.clone()).collect();
            for sr in subs {
                if !seen.insert(sr.name.clone()) {
                    continue;
                }
                sources.push(SourceEntry {
                    source_type: SourceType::Subrecipe,
                    name: sr.name.clone(),
                    description: sr.description.clone().unwrap_or_default(),
                    content: String::new(),
                    path: sr.path.clone(),
                    global: false,
                    writable: false,
                    supporting_files: Vec::new(),
                    properties: std::collections::HashMap::new(),
                });
            }
        }
    }

    if sources.is_empty() {
        return String::new();
    }

    sources.sort_by(|a, b| (&a.source_type, &a.name).cmp(&(&b.source_type, &b.name)));
    let subagents: Vec<&SourceEntry> = sources.iter().collect();

    let names = subagents
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let mut out = String::new();
    out.push_str(
        "\n\nThe following named subagents are available in this session and \
         can be invoked through the `delegate` tool (run as a subagent) or \
         the `load` tool (read their instructions into your own context):\n",
    );

    let mut current_kind: Option<SourceType> = None;
    for s in &subagents {
        if current_kind != Some(s.source_type) {
            out.push_str(&format!("\n{}:", kind_plural(s.source_type)));
            current_kind = Some(s.source_type);
        }
        out.push_str(&format!(
            "\n• {} — {}",
            s.name,
            safe_truncate(&s.description, SUBAGENT_DESCRIPTION_BUDGET)
        ));
    }

    out.push_str(&format!(
        "\n\nWhen to call a subagent (one of [{names}]):\n\
         • `@<name>` in the user's message — always call that subagent.\n\
         • The user mentions a subagent by name without `@` — infer from \
         context whether they want it invoked, and if so, call it.\n\
         • The user's request strongly matches a subagent's description — \
         call it.\n\n\
         Calling a subagent normally means `delegate(source: \"<name>\", \
         instructions: ...)`, which runs it as an isolated subagent and \
         returns its result. Use `load(source: \"<name>\")` instead if you \
         only want to read the subagent's instructions into your own \
         context. For long-running work, pass `async: true` to `delegate` — \
         it returns a task id immediately, and you collect the result later \
         with `load(source: \"<task_id>\")`, which waits for completion.",
    ));

    out
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
    Config::global()
        .get_param::<usize>("GOOSE_MAX_BACKGROUND_TASKS")
        .unwrap_or(5)
}

fn completed_task_ttl() -> Duration {
    let secs = Config::global()
        .get_param::<u64>("GOOSE_COMPLETED_TASK_TTL_SECS")
        .unwrap_or(600);
    Duration::from_secs(secs)
}

fn is_session_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('_').collect();
    parts.len() == 2 && parts[0].len() == 8 && parts[0].chars().all(|c| c.is_ascii_digit())
}

pub struct SummonClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    source_cache: Mutex<Option<(Instant, PathBuf, Vec<SourceEntry>)>>,
    background_tasks: Mutex<HashMap<String, BackgroundTask>>,
    completed_tasks: Mutex<HashMap<String, CompletedTask>>,
    workers: std::sync::Mutex<HashMap<String, WorkerSlot>>,
    worker_idle_timeout: Duration,
    max_workers: usize,
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
        let info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(EXTENSION_NAME, "1.0.0").with_title("Summon"));

        Ok(Self {
            info,
            context,
            source_cache: Mutex::new(None),
            background_tasks: Mutex::new(HashMap::new()),
            completed_tasks: Mutex::new(HashMap::new()),
            workers: std::sync::Mutex::new(HashMap::new()),
            worker_idle_timeout: Config::global()
                .get_param::<u64>("GOOSE_WORKER_IDLE_TIMEOUT")
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_WORKER_IDLE_TIMEOUT),
            max_workers: Config::global()
                .get_param::<usize>("GOOSE_MAX_WORKERS")
                .unwrap_or(DEFAULT_MAX_WORKERS),
            notification_subscribers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    async fn create_subagent_session(
        &self,
        task_config: &TaskConfig,
        name: String,
    ) -> Result<crate::session::Session, String> {
        let session = self
            .context
            .session_manager
            .create_session(
                task_config.parent_working_dir.clone(),
                name,
                SessionType::SubAgent,
                GooseMode::Auto,
            )
            .await
            .map_err(|e| format!("Failed to create subagent session: {}", e))?;

        if !task_config.parent_session_id.is_empty() {
            self.context
                .session_manager
                .update(&session.id)
                .parent_session_id(Some(task_config.parent_session_id.clone()))
                .apply()
                .await
                .map_err(|e| format!("Failed to link subagent to parent session: {}", e))?;
        }

        Ok(session)
    }

    // Evictable only once persisted (so it can be rebuilt) and while idle.
    fn worker_evictable(worker: &PersistentWorker) -> bool {
        worker.persisted.load(Ordering::Relaxed) && worker.busy.try_lock().is_ok()
    }

    fn evict_idle_workers(&self, workers: &mut HashMap<String, WorkerSlot>) {
        let now = current_epoch_millis();
        let idle_limit = self.worker_idle_timeout.as_millis() as u64;
        workers.retain(|key, slot| match slot {
            WorkerSlot::Creating => true,
            WorkerSlot::Ready(worker) => {
                let idle = now.saturating_sub(worker.last_used.load(Ordering::Relaxed));
                let retain = idle <= idle_limit || !Self::worker_evictable(worker);
                if !retain {
                    debug!("Evicting idle worker '{}'", key);
                }
                retain
            }
        });
    }

    fn enforce_worker_capacity(
        &self,
        workers: &mut HashMap<String, WorkerSlot>,
    ) -> Result<(), String> {
        if workers.len() < self.max_workers {
            return Ok(());
        }
        let lru_key = workers
            .iter()
            .filter_map(|(key, slot)| match slot {
                WorkerSlot::Ready(worker) if Self::worker_evictable(worker) => {
                    Some((worker.last_used.load(Ordering::Relaxed), key.clone()))
                }
                _ => None,
            })
            .min()
            .map(|(_, key)| key);
        match lru_key {
            Some(key) => {
                debug!("Evicting worker '{}' to stay within the worker limit", key);
                workers.remove(&key);
                Ok(())
            }
            None => Err(format!(
                "Too many active workers ({}); wait for one to finish or reuse an existing worker",
                workers.len()
            )),
        }
    }

    async fn load_worker_record(
        &self,
        parent_session_id: &str,
        worker_name: &str,
    ) -> Result<Option<WorkerRecord>, String> {
        let Some(session) = self
            .context
            .session_manager
            .find_session(parent_session_id, false)
            .await
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        let Some(record) = WorkerRegistryState::from_extension_data(&session.extension_data)
            .and_then(|registry| registry.workers.get(worker_name).cloned())
        else {
            return Ok(None);
        };
        let worker_session = self
            .context
            .session_manager
            .find_session(&record.session_id, false)
            .await
            .map_err(|e| e.to_string())?
            .filter(|worker_session| {
                // A copied or imported parent session carries records that
                // still point at the original parent's worker sessions;
                // resuming those would write into another session's history.
                worker_session.parent_session_id.as_deref() == Some(parent_session_id)
                    && WorkerIdentityState::from_extension_data(&worker_session.extension_data)
                        .is_some_and(|marker| marker.identity == record.identity)
            });
        if worker_session.is_none() {
            warn!(
                "Session {} for stored worker '{}' is missing or not owned by session {}; discarding record",
                record.session_id, worker_name, parent_session_id
            );
            self.remove_worker_record(parent_session_id, worker_name)
                .await;
            return Ok(None);
        }
        Ok(Some(record))
    }

    async fn save_worker_record(
        &self,
        parent_session_id: &str,
        worker_name: &str,
        record: WorkerRecord,
    ) -> bool {
        let result = self
            .update_worker_registry(parent_session_id, |registry| {
                registry.workers.insert(worker_name.to_string(), record);
            })
            .await;
        match result {
            Ok(()) => true,
            Err(e) => {
                warn!(
                    "Failed to persist worker '{}'; it will not survive a restart: {}",
                    worker_name, e
                );
                false
            }
        }
    }

    async fn remove_worker_record(&self, parent_session_id: &str, worker_name: &str) {
        let result = self
            .update_worker_registry(parent_session_id, |registry| {
                registry.workers.remove(worker_name);
            })
            .await;
        if let Err(e) = result {
            warn!("Failed to remove stored worker '{}': {}", worker_name, e);
        }
    }

    async fn stamp_worker_identity(
        &self,
        worker_session_id: &str,
        identity: &str,
    ) -> Result<(), String> {
        let mut serialize_error = None;
        self.context
            .session_manager
            .update_extension_data(worker_session_id, |extension_data| {
                let state = WorkerIdentityState {
                    identity: identity.to_string(),
                };
                if let Err(e) = state.to_extension_data(extension_data) {
                    serialize_error = Some(e.to_string());
                }
            })
            .await
            .map_err(|e| format!("Failed to mark worker session: {}", e))?;
        serialize_error.map_or(Ok(()), Err)
    }

    async fn update_worker_registry(
        &self,
        parent_session_id: &str,
        f: impl FnOnce(&mut WorkerRegistryState),
    ) -> Result<(), String> {
        let mut serialize_error = None;
        self.context
            .session_manager
            .update_extension_data(parent_session_id, |extension_data| {
                let mut registry =
                    WorkerRegistryState::from_extension_data(extension_data).unwrap_or_default();
                f(&mut registry);
                if let Err(e) = registry.to_extension_data(extension_data) {
                    serialize_error = Some(e.to_string());
                }
            })
            .await
            .map_err(|e| e.to_string())?;
        serialize_error.map_or(Ok(()), Err)
    }

    // `buffer` holds notifications for a later flush to subscribers (background
    // tasks); pass None where no flush point exists.
    fn spawn_notification_bridge(
        mut notif_rx: tokio::sync::mpsc::UnboundedReceiver<ServerNotification>,
        subscribers: Arc<Mutex<Vec<mpsc::Sender<ServerNotification>>>>,
        buffer: Option<Arc<Mutex<Vec<ServerNotification>>>>,
    ) {
        tokio::spawn(async move {
            while let Some(notification) = notif_rx.recv().await {
                let mut subs = subscribers.lock().await;
                if subs.is_empty() {
                    drop(subs);
                    if let Some(buffer) = &buffer {
                        buffer.lock().await.push(notification);
                    }
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
                },
                "peek": {
                    "type": "boolean",
                    "default": false,
                    "description": "For running background tasks: check progress without blocking. Returns turn count, idle time, and recent tool activity."
                }
            }
        });

        Tool::new(
            "load",
            "Load knowledge into your current context or discover available sources.\n\n\
             Call with no arguments to list all available sources (subrecipes, recipes, agents).\n\
             Call with a source name to load its content into your context.\n\
             For background tasks: load(source: \"task_id\") waits for the task and returns the result.\n\
             To cancel a running task: load(source: \"task_id\", cancel: true) stops and returns output.\n\
             To check progress: load(source: \"task_id\", peek: true) returns status without blocking.\n\n\
             Examples:\n\
             - load() → Lists available sources\n\
             - load(source: \"deploy\") → Loads the deploy recipe\n\
             - load(source: \"20260219_1\") → Waits for background task, then returns result\n\
             - load(source: \"20260219_1\", peek: true) → Check task progress without waiting"
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
                    "description": "Name of a recipe or agent to run."
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
                "max_turns": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum turns for this delegate. Overrides recipe settings.max_turns and GOOSE_SUBAGENT_MAX_TURNS."
                },
                "context": {
                    "type": "string",
                    "description": "Reference context for this delegation. Use for background information, file contents, or constraints the delegate needs but that aren't part of the task instructions. One-shot delegates inject it into the system prompt; persistent workers prepend it to that delegation's message, so it is call-scoped and not part of the worker's permanent state."
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for the delegate. Must be within the parent session's working directory. Defaults to the parent's working directory."
                },
                "async": {
                    "type": "boolean",
                    "default": false,
                    "description": "Run in background (default: false)."
                },
                "worker": {
                    "type": "string",
                    "description": "Name of a persistent worker. The first delegation to a name creates the worker (source, extensions, model are fixed then); later delegations continue its session with full memory of prior work. Use for iterative work with the same executor. Not compatible with async."
                }
            }
        });

        Tool::new(
            "delegate",
            "Delegate a task to a subagent that runs independently with its own context.\n\n\
             Modes:\n\
             1. Ad-hoc: Provide `instructions` for a custom task\n\
             2. Source-based: Provide `source` name to run a subrecipe, recipe, or agent\n\
             3. Combined: Pair a source with a task (e.g., source: \"deploy\", instructions: \"deploy to staging\")\n\
             4. Persistent: Add `worker: \"name\"` to keep the delegate's session alive across calls - later delegations to the same worker retain full memory of prior work\n\n\
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

    async fn get_sources(&self, session_id: &str, working_dir: &Path) -> Vec<SourceEntry> {
        let fs_sources = self.get_filesystem_sources(working_dir).await;

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut sources: Vec<SourceEntry> = Vec::new();

        self.add_subrecipes(session_id, &mut sources, &mut seen)
            .await;

        for source in fs_sources {
            if !seen.contains(&source.name) {
                seen.insert(source.name.clone());
                sources.push(source);
            }
        }

        sources.sort_by(|a, b| (&a.source_type, &a.name).cmp(&(&b.source_type, &b.name)));
        sources
    }

    async fn get_filesystem_sources(&self, working_dir: &Path) -> Vec<SourceEntry> {
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
    ) -> Result<Option<SourceEntry>, String> {
        let sources = self.get_sources(session_id, working_dir).await;

        if let Some(mut source) = sources.iter().find(|s| s.name == name).cloned() {
            if source.source_type == SourceType::Subrecipe && source.content.is_empty() {
                source.content = self.load_subrecipe_content(session_id, &source.name).await;
            }
            return Ok(Some(source));
        }

        Ok(None)
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
                Ok(recipe) => {
                    let mut content = recipe.instructions.unwrap_or_default();
                    if let Some(params) = &recipe.parameters {
                        if !params.is_empty() {
                            content.push_str("\n\n");
                            content.push_str(&Self::format_parameters(params));
                        }
                    }
                    content
                }
                Err(_) => recipe_file.content,
            },
            Err(_) => String::new(),
        }
    }

    fn discover_filesystem_sources(&self, working_dir: &Path) -> Vec<SourceEntry> {
        discover_filesystem_sources(working_dir)
    }

    async fn add_subrecipes(
        &self,
        session_id: &str,
        sources: &mut Vec<SourceEntry>,
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

            sources.push(SourceEntry {
                source_type: SourceType::Subrecipe,
                name: sr.name.clone(),
                description,
                content: String::new(),
                path: sr.path.clone(),
                global: false,
                writable: true,
                supporting_files: Vec::new(),
                properties: std::collections::HashMap::new(),
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
                    if !params.is_empty() {
                        desc = format!("{}\n{}", desc, Self::format_parameters(params));
                    }
                }

                return desc;
            }
        }

        format!("Subrecipe from {}", sr.path)
    }

    fn format_parameters(params: &[RecipeParameter]) -> String {
        let mut out = String::from("Parameters:");
        for p in params {
            let mut detail = format!("\n  - {} ({}, {})", p.key, p.input_type, p.requirement);
            if let Some(default) = &p.default {
                detail.push_str(&format!(", default: \"{}\"", default));
            }
            if let Some(options) = &p.options {
                if !options.is_empty() {
                    detail.push_str(&format!(", options: [{}]", options.join(", ")));
                }
            }
            detail.push_str(&format!(": {}", p.description));
            out.push_str(&detail);
        }
        out
    }

    async fn handle_load(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
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

        let peek = arguments
            .as_ref()
            .and_then(|args| args.get("peek"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let working_dir = self.get_working_dir(session_id).await;

        if source_name.is_none() {
            return self
                .handle_load_discovery(session_id, &working_dir)
                .await
                .map(CallToolResult::success);
        }

        let name = source_name.unwrap();

        if is_session_id(name) {
            let task_result = self.handle_load_task_result(name, cancel, peek).await?;
            let mut meta = MetaObject::new();
            meta.0.insert(
                "subagent_session_id".to_string(),
                serde_json::Value::String(name.to_string()),
            );
            meta.0.insert(
                "task_status".to_string(),
                serde_json::Value::String(task_result.status.to_string()),
            );
            if let Some(turns) = task_result.turns {
                meta.0.insert(
                    "turns_taken".to_string(),
                    serde_json::Value::Number(turns.into()),
                );
            }
            if let Some(secs) = task_result.duration_secs {
                meta.0.insert(
                    "duration_secs".to_string(),
                    serde_json::Value::Number(secs.into()),
                );
            }
            return Ok(CallToolResult::success(task_result.content).with_meta(Some(meta)));
        }

        self.handle_load_source(session_id, name, &working_dir)
            .await
            .map(CallToolResult::success)
    }

    async fn handle_load_task_result(
        &self,
        task_id: &str,
        cancel: bool,
        peek: bool,
    ) -> Result<TaskLoadResult, String> {
        let mut completed = self.completed_tasks.lock().await;

        let completed_entry = if peek {
            completed.get(task_id).map(|task| {
                (
                    task.result.clone(),
                    task.description.clone(),
                    task.duration,
                    task.turns_taken,
                )
            })
        } else {
            completed.remove(task_id).map(|task| {
                (
                    task.result,
                    task.description,
                    task.duration,
                    task.turns_taken,
                )
            })
        };

        if let Some((result, description, duration, turns_taken)) = completed_entry {
            let status_key = match &result {
                Ok(_) => "completed",
                Err(e) if e.starts_with("Task panicked:") => "panicked",
                Err(_) => "failed",
            };
            let status = match status_key {
                "completed" => "✓ Completed",
                "panicked" => "✗ Panicked",
                _ => "✗ Failed",
            };
            let output = match result {
                Ok(output) => output,
                Err(error) => format!("Error: {}", error),
            };
            return Ok(TaskLoadResult {
                content: vec![ContentBlock::text(format!(
                    "# Background Task Result: {}\n\n\
                     **Task:** {}\n\
                     **Status:** {}\n\
                     **Duration:** {} ({} turns)\n\n\
                     ## Output\n\n{}",
                    task_id,
                    description,
                    status,
                    round_duration(duration),
                    turns_taken,
                    output
                ))],
                status: status_key,
                turns: Some(turns_taken),
                duration_secs: Some(duration.as_secs()),
            });
        }

        drop(completed);

        let mut running = self.background_tasks.lock().await;
        if running.contains_key(task_id) {
            if peek {
                let task = running.get(task_id).unwrap();
                let elapsed = task.started_at.elapsed();
                let turns_taken = task.turns.load(Ordering::Relaxed);
                let now = current_epoch_millis();
                let idle_ms = now.saturating_sub(task.last_activity.load(Ordering::Relaxed));
                let description = task.description.clone();

                let buffered_count = task.notification_buffer.lock().await.len();

                drop(running);

                let mut output = format!(
                    "# Background Task Status: {}\n\n**Task:** {}\n**Status:** ⏳ Running\n**Elapsed:** {}\n**Turns taken:** {}\n**Idle:** {}\n**Buffered tool calls:** {}",
                    task_id,
                    description,
                    round_duration(elapsed),
                    turns_taken,
                    round_duration(Duration::from_millis(idle_ms)),
                    buffered_count,
                );

                if buffered_count == 0 && turns_taken == 0 {
                    output.push_str("\n\n_Task is initialising (no tool activity yet)._");
                }

                return Ok(TaskLoadResult {
                    content: vec![ContentBlock::text(output)],
                    status: "running",
                    turns: Some(turns_taken),
                    duration_secs: Some(elapsed.as_secs()),
                });
            }

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

                return Ok(TaskLoadResult {
                    content: vec![ContentBlock::text(format!(
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
                    ))],
                    status: "cancelled",
                    turns: Some(turns_taken),
                    duration_secs: Some(duration.as_secs()),
                });
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
                    let (output, status_key) = match result {
                        Ok(Ok(s)) => (s, "completed"),
                        Ok(Err(e)) => (format!("Error: {}", e), "failed"),
                        Err(e) => (format!("Task panicked: {}", e), "panicked"),
                    };

                    let turns_taken = task.turns.load(Ordering::Relaxed);
                    let elapsed = task.started_at.elapsed();
                    let status_display = match status_key {
                        "completed" => "✓ Completed",
                        "panicked" => "✗ Panicked",
                        _ => "✗ Failed",
                    };
                    return Ok(TaskLoadResult {
                        content: vec![ContentBlock::text(format!(
                            "# Background Task Result: {}\n\n\
                             **Task:** {}\n\
                             **Status:** {}\n\
                             **Duration:** {} ({} turns)\n\n\
                             ## Output\n\n{}",
                            task_id,
                            task.description,
                            status_display,
                            round_duration(elapsed),
                            turns_taken,
                            output
                        ))],
                        status: status_key,
                        turns: Some(turns_taken),
                        duration_secs: Some(elapsed.as_secs()),
                    });
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
    ) -> Result<Vec<ContentBlock>, String> {
        {
            let mut cache = self.source_cache.lock().await;
            *cache = None;
        }

        let sources = self.get_sources(session_id, working_dir).await;
        let completed = self.completed_tasks.lock().await;

        if sources.is_empty() && completed.is_empty() {
            return Ok(vec![ContentBlock::text(
                "No sources available for load/delegate.\n\n\
                 Sources are discovered from:\n\
                 • Current recipe's sub_recipes\n\
                 • .agents/recipes/, .agents/agents/ (project-level)\n\
                 • ~/.agents/agents/ (global)\n\
                 • GOOSE_RECIPE_PATH directories",
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

        for kind in [SourceType::Subrecipe, SourceType::Recipe, SourceType::Agent] {
            let kind_sources: Vec<_> = sources.iter().filter(|s| s.source_type == kind).collect();
            if !kind_sources.is_empty() {
                output.push_str(&format!("\n{}:\n", kind_plural(kind)));
                for source in kind_sources {
                    output.push_str(&format!(
                        "• {} - {}\n",
                        source.name,
                        safe_truncate(&source.description, SUBAGENT_DESCRIPTION_BUDGET)
                    ));
                }
            }
        }

        output.push_str("\nUse load(source: \"name\") to load into context.\n");
        output.push_str("Use delegate(source: \"name\") to run as subagent.");

        Ok(vec![ContentBlock::text(output)])
    }

    async fn handle_load_source(
        &self,
        session_id: &str,
        name: &str,
        working_dir: &Path,
    ) -> Result<Vec<ContentBlock>, String> {
        let source = self.resolve_source(session_id, name, working_dir).await?;

        match source {
            Some(source) => {
                let content = source.to_load_text();

                let output = format!(
                    "# Loaded: {} ({})\n\n{}\n\n---\nThis knowledge is now available in your context.",
                    source.name, source.source_type, content
                );

                Ok(vec![ContentBlock::text(output)])
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
    ) -> Result<CallToolResult, String> {
        self.cleanup_completed_tasks().await;

        let params: DelegateParams = arguments
            .map(|args| serde_json::from_value(serde_json::Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Invalid parameters: {}", e))?
            .unwrap_or_default();

        let session = self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?;

        if session.session_type == SessionType::SubAgent {
            return Err("Delegated tasks cannot spawn further delegations".to_string());
        }

        if params.worker.is_some() {
            return self
                .handle_worker_delegate(&session, params, cancellation_token)
                .await;
        }

        self.validate_delegate_params(&params)?;

        if params.r#async {
            let (content, task_id) = self.handle_async_delegate(session_id, params).await?;
            let mut meta = MetaObject::new();
            meta.0.insert(
                "subagent_session_id".to_string(),
                serde_json::Value::String(task_id),
            );
            return Ok(CallToolResult::success(content).with_meta(Some(meta)));
        }

        let working_dir = session.working_dir.clone();
        let recipe = self
            .build_delegate_recipe(&params, session_id, &working_dir)
            .await?;

        let task_config = self
            .build_task_config(&params, &recipe, &session, None)
            .await
            .map_err(|e| format!("Failed to build task config: {}", e))?;

        // Subagents must use Auto until get_agent_messages forwards
        // ActionRequired messages to the parent. Until then, any mode
        // that requires approval will hang on the subagent's confirmation_rx.
        let agent_config = AgentConfig::new(
            self.context.session_manager.clone(),
            crate::config::permission::PermissionManager::instance(),
            None,
            GooseMode::Auto,
            true, // disable session naming for subagents
            crate::agents::GoosePlatform::GooseCli,
        )
        .with_use_login_shell_path(self.context.use_login_shell_path);

        let subagent_session = self
            .create_subagent_session(&task_config, "Delegated task".to_string())
            .await?;

        let (notif_tx, notif_rx) = tokio::sync::mpsc::unbounded_channel::<ServerNotification>();
        Self::spawn_notification_bridge(notif_rx, Arc::clone(&self.notification_subscribers), None);

        let subagent_session_id = subagent_session.id.clone();

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
        .await;

        let mut meta = MetaObject::new();
        meta.0.insert(
            "subagent_session_id".to_string(),
            serde_json::Value::String(subagent_session_id),
        );

        match result {
            Ok(text) => {
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]).with_meta(Some(meta)))
            }
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Delegation failed: {}",
                e
            ))])
            .with_meta(Some(meta))),
        }
    }

    async fn handle_worker_delegate(
        &self,
        session: &crate::session::Session,
        params: DelegateParams,
        cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, String> {
        if params.r#async {
            return Err(
                "Persistent workers run synchronously; 'async' cannot be combined with 'worker'"
                    .to_string(),
            );
        }
        let worker_name = params.worker.clone().unwrap_or_default();
        if worker_name.trim().is_empty() {
            return Err("'worker' must be a non-empty name".to_string());
        }
        if let Some(max) = params.max_turns {
            if max < 1 {
                return Err("'max_turns' must be at least 1".to_string());
            }
            if max > u32::MAX as usize {
                return Err(format!("'max_turns' must be at most {}", u32::MAX));
            }
        }
        let worker_key = format!("{}:{}", session.id, worker_name);

        let busy_error = || {
            format!(
                "Worker '{}' is busy with a previous delegation. Wait for it to finish.",
                worker_name
            )
        };

        // Held for the whole delegation so lookup-create-save cannot
        // interleave with another client delegating to the same worker name.
        let delegation_lock = shared_lock(
            &WORKER_DELEGATION_LOCKS,
            &self.context.session_manager,
            &worker_key,
        );
        let _delegation = delegation_lock.try_lock_owned().map_err(|_| busy_error())?;

        let stored = self.load_worker_record(&session.id, &worker_name).await?;

        let checked_out = {
            let mut workers = self.workers.lock().unwrap();
            self.evict_idle_workers(&mut workers);
            // The stored record is authoritative: drop a worker that no longer
            // matches it (session deleted or recreated by another client). With
            // no record, an unpersisted worker survives to retry its record write.
            if let Some(WorkerSlot::Ready(worker)) = workers.get(&worker_key) {
                let stale = match stored.as_ref() {
                    Some(record) => {
                        record.session_id != worker.session_id || record.identity != worker.identity
                    }
                    None => worker.persisted.load(Ordering::Relaxed),
                };
                if stale {
                    workers.remove(&worker_key);
                }
            }
            match workers.get(&worker_key) {
                Some(WorkerSlot::Ready(worker)) => {
                    let busy = worker
                        .busy
                        .clone()
                        .try_lock_owned()
                        .map_err(|_| busy_error())?;
                    Some((worker.clone(), busy))
                }
                Some(WorkerSlot::Creating) => return Err(busy_error()),
                None => {
                    if stored.is_none() {
                        self.validate_delegate_params(&params)?;
                    }
                    self.enforce_worker_capacity(&mut workers)?;
                    workers.insert(worker_key.clone(), WorkerSlot::Creating);
                    None
                }
            }
        };

        let (worker, user_text, _busy) = match (checked_out, stored) {
            (Some((worker, busy)), _) => {
                validate_worker_reuse_params(&params, &worker.creation, &session.working_dir)?;
                if !worker.persisted.load(Ordering::Relaxed)
                    && self
                        .save_worker_record(
                            &session.id,
                            &worker_name,
                            WorkerRecord::from_worker(&worker),
                        )
                        .await
                {
                    worker.persisted.store(true, Ordering::Relaxed);
                }
                let user_text = worker_followup_user_text(&params);
                (worker, user_text, busy)
            }
            (None, Some(record)) => {
                let creating = CreationSlotGuard {
                    workers: &self.workers,
                    key: Some(worker_key),
                };
                let creation_request = record.to_delegate_params(&worker_name);
                let (worker, _) = self
                    .create_worker(session, &creation_request, &worker_name, Some(&record))
                    .await
                    .map_err(|e| format!("Failed to restore worker '{}': {}", worker_name, e))?;
                let key = creating.release();
                self.workers
                    .lock()
                    .unwrap()
                    .insert(key, WorkerSlot::Ready(worker.clone()));
                // The busy lock is shared by worker session id, so another
                // client restoring the same record may already hold it.
                let busy = worker
                    .busy
                    .clone()
                    .try_lock_owned()
                    .map_err(|_| busy_error())?;
                validate_worker_reuse_params(&params, &worker.creation, &session.working_dir)?;
                let user_text = worker_followup_user_text(&params);
                (worker, user_text, busy)
            }
            (None, None) => {
                let creating = CreationSlotGuard {
                    workers: &self.workers,
                    key: Some(worker_key),
                };
                let (worker, user_text) = self
                    .create_worker(session, &params, &worker_name, None)
                    .await?;
                let busy = worker
                    .busy
                    .clone()
                    .try_lock_owned()
                    .expect("newly created worker is not busy");
                let key = creating.release();
                self.workers
                    .lock()
                    .unwrap()
                    .insert(key, WorkerSlot::Ready(worker.clone()));
                let saved = self
                    .save_worker_record(
                        &session.id,
                        &worker_name,
                        WorkerRecord::from_worker(&worker),
                    )
                    .await;
                if saved {
                    worker.persisted.store(true, Ordering::Relaxed);
                }
                (worker, user_text, busy)
            }
        };

        worker
            .last_used
            .store(current_epoch_millis(), Ordering::Relaxed);

        let max_turns = params.max_turns.unwrap_or(worker.default_max_turns);
        if worker.prompt_max_turns.load(Ordering::Relaxed) != max_turns {
            let task_config = worker.task_config.clone().with_max_turns(Some(max_turns));
            crate::agents::subagent_handler::refresh_subagent_prompt(
                &worker.configured,
                &task_config,
                &worker.session_id,
                worker.recipe.instructions.clone().unwrap_or_default(),
            )
            .await
            .map_err(|e| format!("Failed to update worker '{}' prompt: {}", worker_name, e))?;
            worker.prompt_max_turns.store(max_turns, Ordering::Relaxed);
        }

        let result = crate::agents::subagent_handler::run_configured_subagent_reply(
            &worker.configured,
            crate::agents::subagent_handler::SubagentReplyParams {
                session_id: &worker.session_id,
                user_text,
                max_turns: Some(max_turns as u32),
                retry_config: worker.recipe.retry.clone(),
                cancellation_token: Some(cancellation_token),
                on_message: None,
                notification_tx: Some(worker.notification_tx.clone()),
            },
        )
        .await;

        worker
            .last_used
            .store(current_epoch_millis(), Ordering::Relaxed);

        let mut meta = MetaObject::new();
        meta.0.insert(
            "subagent_session_id".to_string(),
            serde_json::Value::String(worker.session_id.clone()),
        );

        match result {
            Ok(text) => {
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]).with_meta(Some(meta)))
            }
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Worker delegation failed: {}",
                e
            ))])
            .with_meta(Some(meta))),
        }
    }

    async fn create_worker(
        &self,
        session: &crate::session::Session,
        params: &DelegateParams,
        worker_name: &str,
        restore: Option<&WorkerRecord>,
    ) -> Result<(Arc<PersistentWorker>, String), String> {
        let working_dir = session.working_dir.clone();
        let mut recipe = match restore {
            Some(record) => record.recipe.clone(),
            None => {
                // `context` is call-scoped; keep it out of the persistent recipe.
                let recipe_params = DelegateParams {
                    context: None,
                    ..params.clone()
                };
                self.build_delegate_recipe(&recipe_params, &session.id, &working_dir)
                    .await?
            }
        };

        // Resolve the model override once and pin it into the recipe, so the
        // stored creation params match the model build_task_config configures
        // even if global config changes mid-creation.
        let model_request = requested_model_override(params, &recipe);
        if let Some(model) = &model_request {
            match recipe.settings.as_mut() {
                Some(settings) => settings.goose_model = Some(model.clone()),
                None => {
                    recipe.settings = Some(Settings {
                        goose_provider: None,
                        goose_model: Some(model.clone()),
                        temperature: None,
                        max_turns: None,
                    })
                }
            }
        }

        let task_config = self
            .build_task_config(params, &recipe, session, restore)
            .await
            .map_err(|e| format!("Failed to build task config: {}", e))?;

        // Same constraint as one-shot delegates: subagents must run in Auto mode
        // until ActionRequired messages are forwarded to the parent.
        let agent_config = AgentConfig::new(
            self.context.session_manager.clone(),
            crate::config::permission::PermissionManager::instance(),
            None,
            GooseMode::Auto,
            true,
            crate::agents::GoosePlatform::GooseCli,
        )
        .with_use_login_shell_path(self.context.use_login_shell_path);

        let (worker_session_id, identity) = match restore {
            Some(record) => (record.session_id.clone(), record.identity.clone()),
            None => {
                let session_id = self
                    .create_subagent_session(&task_config, format!("Worker '{}'", worker_name))
                    .await?
                    .id;
                let identity = uuid::Uuid::new_v4().to_string();
                self.stamp_worker_identity(&session_id, &identity).await?;
                (session_id, identity)
            }
        };

        let configured = crate::agents::subagent_handler::configure_subagent_agent(
            agent_config,
            &recipe,
            &task_config,
            &worker_session_id,
        )
        .await
        .map_err(|e| format!("Failed to configure worker '{}': {}", worker_name, e))?;

        let (notif_tx, notif_rx) = tokio::sync::mpsc::unbounded_channel::<ServerNotification>();
        Self::spawn_notification_bridge(notif_rx, Arc::clone(&self.notification_subscribers), None);

        let default_max_turns = match restore {
            Some(record) => record.default_max_turns,
            None => {
                let resolved = recipe
                    .settings
                    .as_ref()
                    .and_then(|s| s.max_turns)
                    .unwrap_or_else(|| self.resolve_max_turns(session));
                if resolved == 0 || resolved > u32::MAX as usize {
                    return Err(format!(
                        "max_turns must be between 1 and {} (got {})",
                        u32::MAX,
                        resolved
                    ));
                }
                resolved
            }
        };
        let prompt_max_turns = task_config
            .max_turns
            .expect("TaskConfig always sets max_turns");

        let prompt = recipe
            .prompt
            .clone()
            .unwrap_or_else(|| "Begin.".to_string());
        let user_text = match &params.context {
            Some(context) => format!("Context:\n{}\n\n{}", context, prompt),
            None => prompt,
        };

        let worker = Arc::new(PersistentWorker {
            configured: Arc::new(configured),
            busy: shared_lock(
                &WORKER_BUSY_LOCKS,
                &self.context.session_manager,
                &worker_session_id,
            ),
            session_id: worker_session_id,
            identity,
            default_max_turns,
            prompt_max_turns: AtomicUsize::new(prompt_max_turns),
            last_used: AtomicU64::new(current_epoch_millis()),
            persisted: AtomicBool::new(restore.is_some()),
            notification_tx: notif_tx,
            creation: match restore {
                Some(record) => record.creation.clone(),
                None => WorkerCreationParams::resolve(params, model_request, &task_config),
            },
            recipe,
            task_config,
        });

        Ok((worker, user_text))
    }

    fn validate_delegate_params(&self, params: &DelegateParams) -> Result<(), String> {
        if params.instructions.is_none() && params.source.is_none() {
            return Err("Must provide 'instructions' or 'source' (or both)".to_string());
        }

        if params.parameters.is_some() && params.source.is_none() {
            return Err("'parameters' can only be used with 'source'".to_string());
        }

        if let Some(max) = params.max_turns {
            if max < 1 {
                return Err("'max_turns' must be at least 1".to_string());
            }
        }

        Ok(())
    }

    async fn build_delegate_recipe(
        &self,
        params: &DelegateParams,
        session_id: &str,
        working_dir: &Path,
    ) -> Result<Recipe, String> {
        let mut recipe = if let Some(source_name) = &params.source {
            self.build_source_recipe(source_name, params, session_id, working_dir)
                .await?
        } else {
            self.build_adhoc_recipe(params)?
        };

        if let Some(ref context) = params.context {
            let existing = recipe.instructions.unwrap_or_default();
            recipe.instructions = Some(build_instructions_with_context(context, &existing));
        }

        Ok(recipe)
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
            .await?
            .ok_or_else(|| format!("Source '{}' not found", source_name))?;

        let mut recipe = match source.source_type {
            SourceType::Recipe | SourceType::Subrecipe => {
                self.build_recipe_from_source(&source, params, session_id)
                    .await?
            }
            SourceType::Agent => self.build_recipe_from_agent(&source, params)?,
            _ => {
                return Err(format!(
                    "Source '{}' has kind '{}' which cannot be delegated from summon",
                    source_name, source.source_type
                ));
            }
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
        source: &SourceEntry,
        params: &DelegateParams,
        session_id: &str,
    ) -> Result<Recipe, String> {
        let session = self
            .context
            .session_manager
            .get_session(session_id, false)
            .await
            .map_err(|e| format!("Failed to get session: {}", e))?;

        if source.source_type == SourceType::Subrecipe {
            let sub_recipes = session.recipe.as_ref().and_then(|r| r.sub_recipes.as_ref());

            if let Some(sub_recipes) = sub_recipes {
                if let Some(sr) = sub_recipes.iter().find(|sr| sr.name == source.name) {
                    let recipe_file = load_local_recipe_file(&sr.path).map_err(|e| {
                        format!("Failed to load subrecipe '{}': {}", source.name, e)
                    })?;

                    let merged =
                        merge_subrecipe_parameters(sr.values.as_ref(), params.parameters.as_ref());
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

        let recipe_file = load_local_recipe_file(&source.path)
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

    fn build_recipe_from_agent(
        &self,
        source: &SourceEntry,
        params: &DelegateParams,
    ) -> Result<Recipe, String> {
        let agent_content = if source.path.is_empty() {
            return Err("Agent source has no path".to_string());
        } else {
            std::fs::read_to_string(&source.path)
                .map_err(|e| format!("Failed to read agent file: {}", e))?
        };

        let (metadata, _): (AgentMetadata, String) = parse_frontmatter(&agent_content)
            .map_err(|e| format!("Failed to parse agent frontmatter: {}", e))?
            .ok_or("No frontmatter found in agent file")?;

        let model = metadata.model;

        // max_turns is set later in build_task_config so it can incorporate params.max_turns
        // with the correct priority ordering; setting it here would cause it to be overridden
        // by the parent session's recipe instead.
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
        restore: Option<&WorkerRecord>,
    ) -> Result<TaskConfig, anyhow::Error> {
        let extensions = match restore {
            Some(record) => {
                // The worker may have changed its own extensions at runtime;
                // its session state is more current than the record.
                let worker_session = self
                    .context
                    .session_manager
                    .get_session(&record.session_id, false)
                    .await?;
                EnabledExtensionsState::from_extension_data(&worker_session.extension_data)
                    .map(|state| state.extensions)
                    .unwrap_or_else(|| record.extensions.clone())
            }
            None => {
                let mut extensions = EnabledExtensionsState::extensions_or_default(
                    Some(&session.extension_data),
                    Config::global(),
                );

                if let Some(filter) = &params.extensions {
                    if filter.is_empty() {
                        extensions = Vec::new();
                    } else {
                        let available_names: Vec<String> =
                            extensions.iter().map(|ext| ext.name()).collect();
                        extensions.retain(|ext| filter.contains(&ext.name()));
                        let unmatched: Vec<&str> = filter
                            .iter()
                            .filter(|name| !available_names.iter().any(|n| n == *name))
                            .map(String::as_str)
                            .collect();
                        if !unmatched.is_empty() {
                            warn!(
                                "Delegate requested extensions not available in session: {:?}. Available: {:?}",
                                unmatched, available_names
                            );
                        }
                    }
                }
                extensions
            }
        };

        // On restore the provider is rebuilt by name; model config and turn
        // budget come from the record so creation-time values cannot drift
        // and current config cannot fail re-resolution.
        let (provider, model_config) = match restore {
            Some(record) => {
                let provider = self
                    .build_provider(&record.creation.provider, &extensions)
                    .await?;
                (provider, record.model_config.clone())
            }
            None => {
                self.resolve_provider(params, recipe, session, &extensions)
                    .await?
            }
        };

        let max_turns = params
            .max_turns
            .or(restore.map(|record| record.default_max_turns))
            .or_else(|| recipe.settings.as_ref().and_then(|s| s.max_turns))
            .unwrap_or_else(|| self.resolve_max_turns(session));

        if max_turns == 0 || max_turns > u32::MAX as usize {
            anyhow::bail!(
                "max_turns must be between 1 and {} (got {})",
                u32::MAX,
                max_turns
            );
        }

        let effective_working_dir = match restore {
            // Re-validating against the parent's current working dir would
            // break restoration whenever the parent dir changed; only require
            // that the recorded dir still exists.
            Some(record) => {
                let dir = record.creation.working_dir.clone();
                if !dir.is_dir() {
                    anyhow::bail!(
                        "worker working_dir '{}' no longer exists",
                        dir.to_string_lossy()
                    );
                }
                dir
            }
            None => match &params.working_dir {
                Some(dir) => resolve_working_dir(&session.working_dir, dir)?,
                None => session.working_dir.clone(),
            },
        };

        let task_config = TaskConfig::new(
            provider,
            model_config,
            &session.id,
            &effective_working_dir,
            extensions,
        )
        .with_max_turns(Some(max_turns));

        Ok(task_config)
    }

    fn resolve_model_config(
        &self,
        params: &DelegateParams,
        recipe: &Recipe,
        session: &crate::session::Session,
        provider_name: &str,
    ) -> Result<goose_providers::model::ModelConfig, anyhow::Error> {
        let mut model_config = session.model_config.clone().map(Ok).unwrap_or_else(|| {
            crate::model_config::model_config_from_user_config(provider_name, "default")
        })?;

        let override_model = requested_model_override(params, recipe);

        if let Some(model) = override_model {
            if model != model_config.model_name {
                // Build the overridden config through the canonical session-settings
                // path. This materializes model-specific fields (context_limit,
                // max_tokens, reasoning) and env overrides for the *new* model, and
                // inherits only model-family-agnostic session state from the parent:
                // reasoning controls like `thinking_effort` and `budget_tokens` carry
                // over (with the child > parent > global-default precedence the helper
                // applies), while provider-specific request_params such as
                // `anthropic_beta` are dropped so they can't bleed into a child
                // targeting a different model family and trigger a 400 INVALID_ARGUMENT.
                let parent = model_config;
                let mut cfg =
                    crate::model_config::model_config_from_user_config_with_session_settings(
                        provider_name,
                        &model,
                        Some(&parent),
                        None,
                        None,
                    )?;
                // Remaining model-agnostic session settings the helper doesn't
                // touch, copied from the parent explicitly.
                cfg.toolshim = parent.toolshim;
                cfg.toolshim_model = parent.toolshim_model;
                cfg.temperature = cfg.temperature.or(parent.temperature);
                model_config = cfg;
            }
        }

        if let Some(temp) = params.temperature {
            model_config = model_config.with_temperature(Some(temp));
        } else if let Some(temp) = recipe.settings.as_ref().and_then(|s| s.temperature) {
            model_config = model_config.with_temperature(Some(temp));
        }

        Ok(model_config)
    }

    async fn resolve_provider(
        &self,
        params: &DelegateParams,
        recipe: &Recipe,
        session: &crate::session::Session,
        extensions: &[crate::config::ExtensionConfig],
    ) -> Result<
        (
            Arc<dyn crate::providers::base::Provider>,
            goose_providers::model::ModelConfig,
        ),
        anyhow::Error,
    > {
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

        let model_config = self.resolve_model_config(params, recipe, session, &provider_name)?;
        let provider = self.build_provider(&provider_name, extensions).await?;
        Ok((provider, model_config))
    }

    async fn build_provider(
        &self,
        provider_name: &str,
        extensions: &[crate::config::ExtensionConfig],
    ) -> Result<Arc<dyn crate::providers::base::Provider>, anyhow::Error> {
        match providers::get_from_registry(provider_name).await {
            Ok(entry) => entry.create(extensions.to_vec()).await,
            Err(error) => {
                let parent_provider = if let Some(extension_manager) = self
                    .context
                    .extension_manager
                    .as_ref()
                    .and_then(|weak| weak.upgrade())
                {
                    extension_manager.get_provider().lock().await.clone()
                } else {
                    None
                };

                match parent_provider {
                    Some(provider)
                        if provider.get_name() == provider_name
                            && !provider.manages_own_context() =>
                    {
                        Ok(provider)
                    }
                    _ => Err(error),
                }
            }
        }
    }

    fn resolve_max_turns(&self, session: &crate::session::Session) -> usize {
        session
            .recipe
            .as_ref()
            .and_then(|r| r.settings.as_ref())
            .and_then(|s| s.max_turns)
            .or_else(|| {
                std::env::var("GOOSE_SUBAGENT_MAX_TURNS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .or_else(|| {
                Config::global()
                    .get_param::<usize>("GOOSE_SUBAGENT_MAX_TURNS")
                    .ok()
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
                    completed_at: Instant::now(),
                },
            );
        }

        let ttl = completed_task_ttl();
        completed.retain(|_id, task| task.completed_at.elapsed() <= ttl);
    }

    fn get_task_description(params: &DelegateParams) -> String {
        match (&params.source, &params.instructions) {
            (Some(source), Some(instructions)) => format!("{}: {}", source, instructions),
            (Some(source), None) => source.clone(),
            (None, Some(instructions)) => instructions.clone(),
            (None, None) => "Unknown task".to_string(),
        }
    }

    async fn handle_async_delegate(
        &self,
        session_id: &str,
        params: DelegateParams,
    ) -> Result<(Vec<ContentBlock>, String), String> {
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
            .build_task_config(&params, &recipe, &session, None)
            .await
            .map_err(|e| format!("Failed to build task config: {}", e))?;

        let description = safe_truncate(&Self::get_task_description(&params), TASK_LABEL_BUDGET);

        // Subagents must use Auto until get_agent_messages forwards
        // ActionRequired messages to the parent. Until then, any mode
        // that requires approval will hang on the subagent's confirmation_rx.
        let agent_config = AgentConfig::new(
            self.context.session_manager.clone(),
            crate::config::permission::PermissionManager::instance(),
            None,
            GooseMode::Auto,
            true, // disable session naming for subagents
            crate::agents::GoosePlatform::GooseCli,
        )
        .with_use_login_shell_path(self.context.use_login_shell_path);

        let subagent_session = self
            .create_subagent_session(&task_config, description.clone())
            .await?;

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
            Some(Arc::clone(&notification_buffer)),
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

        let content = vec![ContentBlock::text(format!(
            "Task {} started in background: \"{}\"\n\
             Continue with other work. When you need the result, use load(source: \"{}\").",
            task_id, description, task_id
        ))];
        Ok((content, task_id))
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
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let session_id = &ctx.session_id;
        match name {
            "load" => match self.handle_load(session_id, arguments).await {
                Ok(result) => Ok(result),
                Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "Error: {}",
                    error
                ))])),
            },
            "delegate" => {
                match self
                    .handle_delegate(session_id, arguments, cancellation_token)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                        "Error: {}",
                        error
                    ))])),
                }
            }
            _ => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Error: Unknown tool: {}",
                name
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    fn get_instructions(&self) -> Option<String> {
        let instructions = build_subagent_instructions(self.context.session.as_deref());
        if instructions.is_empty() {
            None
        } else {
            Some(instructions)
        }
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

/// Resolve a requested `working_dir` override against the parent session
/// directory. Relative paths are joined to the parent dir; the result must
/// canonicalize to an existing directory contained within the parent dir.
fn resolve_working_dir(parent_dir: &Path, requested: &str) -> Result<PathBuf, anyhow::Error> {
    let requested_path = PathBuf::from(requested);
    let resolved = if requested_path.is_absolute() {
        requested_path
    } else {
        parent_dir.join(&requested_path)
    };
    let canonical = resolved
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("working_dir '{}' could not be resolved: {}", requested, e))?;
    let parent_canonical = parent_dir
        .canonicalize()
        .unwrap_or_else(|_| parent_dir.to_path_buf());
    if !canonical.starts_with(&parent_canonical) {
        anyhow::bail!(
            "working_dir '{}' is outside the parent session directory",
            requested
        );
    }
    if !canonical.is_dir() {
        anyhow::bail!("working_dir '{}' is not a directory", requested);
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_context() -> PlatformExtensionContext {
        PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(crate::session::SessionManager::instance()),
            session: None,
            use_login_shell_path: false,
        }
    }

    fn creation_params(working_dir: PathBuf) -> WorkerCreationParams {
        WorkerCreationParams {
            source: Some("sidekick".to_string()),
            parameters: None,
            extensions_filter: None,
            extensions: HashSet::from(["developer".to_string()]),
            provider: "anthropic".to_string(),
            model_request: None,
            model: "claude-sonnet-5".to_string(),
            temperature: None,
            working_dir,
        }
    }

    fn worker_session_dir() -> (TempDir, PathBuf) {
        let session_dir = TempDir::new().unwrap();
        fs::create_dir(session_dir.path().join("app")).unwrap();
        fs::create_dir(session_dir.path().join("other")).unwrap();
        let app_dir = resolve_working_dir(session_dir.path(), "app").unwrap();
        (session_dir, app_dir)
    }

    #[test]
    fn test_worker_reuse_rejects_changed_creation_params() {
        let (session_dir, app_dir) = worker_session_dir();
        let params = DelegateParams {
            instructions: Some("continue".to_string()),
            model: Some("claude-haiku-4-5".to_string()),
            extensions: Some(vec!["summon".to_string()]),
            working_dir: Some("other".to_string()),
            worker: Some("sidekick".to_string()),
            ..Default::default()
        };

        let err =
            validate_worker_reuse_params(&params, &creation_params(app_dir), session_dir.path())
                .unwrap_err();
        assert!(err.contains("extensions"));
        assert!(err.contains("model"));
        assert!(err.contains("working_dir"));
        assert!(!err.contains("source"));
    }

    #[test]
    fn test_worker_reuse_allows_resending_identical_creation_params() {
        let (session_dir, app_dir) = worker_session_dir();
        let params = DelegateParams {
            instructions: Some("continue".to_string()),
            source: Some("sidekick".to_string()),
            working_dir: Some("app".to_string()),
            worker: Some("sidekick".to_string()),
            ..Default::default()
        };

        assert!(validate_worker_reuse_params(
            &params,
            &creation_params(app_dir),
            session_dir.path()
        )
        .is_ok());
    }

    #[test]
    fn test_worker_reuse_allows_explicit_params_matching_resolved_values() {
        let (session_dir, app_dir) = worker_session_dir();
        let params = DelegateParams {
            instructions: Some("continue".to_string()),
            provider: Some("anthropic".to_string()),
            model: Some("claude-sonnet-5".to_string()),
            extensions: Some(vec!["developer".to_string()]),
            worker: Some("sidekick".to_string()),
            ..Default::default()
        };

        assert!(validate_worker_reuse_params(
            &params,
            &creation_params(app_dir),
            session_dir.path()
        )
        .is_ok());
    }

    #[test]
    fn test_worker_reuse_model_compared_against_original_request() {
        let (session_dir, app_dir) = worker_session_dir();
        let mut creation = creation_params(app_dir);
        creation.model_request = Some("gpt-5-high".to_string());
        creation.model = "gpt-5".to_string();

        let resend_original = DelegateParams {
            instructions: Some("continue".to_string()),
            model: Some("gpt-5-high".to_string()),
            worker: Some("sidekick".to_string()),
            ..Default::default()
        };
        assert!(
            validate_worker_reuse_params(&resend_original, &creation, session_dir.path()).is_ok()
        );

        let send_normalized = DelegateParams {
            instructions: Some("continue".to_string()),
            model: Some("gpt-5".to_string()),
            worker: Some("sidekick".to_string()),
            ..Default::default()
        };
        let err = validate_worker_reuse_params(&send_normalized, &creation, session_dir.path())
            .unwrap_err();
        assert!(err.contains("model"));
    }

    #[derive(Default)]
    struct RecordingProvider {
        calls: std::sync::Mutex<Vec<Vec<String>>>,
    }

    #[async_trait]
    impl crate::providers::base::Provider for RecordingProvider {
        fn get_name(&self) -> &str {
            "worker-test"
        }

        async fn stream(
            &self,
            _model_config: &goose_providers::model::ModelConfig,
            _system: &str,
            messages: &[crate::conversation::message::Message],
            _tools: &[Tool],
        ) -> Result<crate::providers::base::MessageStream, goose_providers::errors::ProviderError>
        {
            let reply_number = {
                let mut calls = self.calls.lock().unwrap();
                calls.push(messages.iter().map(|m| m.as_concat_text()).collect());
                calls.len()
            };
            let message = crate::conversation::message::Message::assistant()
                .with_text(format!("reply-{reply_number}"));
            let usage = crate::providers::base::ProviderUsage::new(
                "test-model".to_string(),
                crate::providers::base::Usage::new(Some(1), Some(1), Some(2)),
            );
            Ok(crate::providers::base::stream_from_single_message(
                message, usage,
            ))
        }
    }

    struct WorkerTestRig {
        client: SummonClient,
        provider: Arc<RecordingProvider>,
        session: crate::session::Session,
        _extension_manager: Arc<crate::agents::ExtensionManager>,
        _temp: TempDir,
    }

    async fn worker_test_rig() -> WorkerTestRig {
        let temp = TempDir::new().unwrap();
        let provider = Arc::new(RecordingProvider::default());
        let extension_manager = Arc::new(
            crate::agents::extension_manager::ExtensionManager::new_without_provider(
                temp.path().to_path_buf(),
            ),
        );
        *extension_manager.get_provider().lock().await =
            Some(provider.clone() as Arc<dyn crate::providers::base::Provider>);
        let mut context = extension_manager.get_context().clone();
        context.extension_manager = Some(Arc::downgrade(&extension_manager));
        let client = SummonClient::new(context).unwrap();
        let session = crate::session::Session {
            provider_name: Some("worker-test".to_string()),
            model_config: Some(goose_providers::model::ModelConfig::new("test-model")),
            working_dir: temp.path().to_path_buf(),
            ..Default::default()
        };

        WorkerTestRig {
            client,
            provider,
            session,
            _extension_manager: extension_manager,
            _temp: temp,
        }
    }

    fn worker_creation_delegate_params(instructions: &str) -> DelegateParams {
        DelegateParams {
            instructions: Some(instructions.to_string()),
            provider: Some("worker-test".to_string()),
            model: Some("test-model".to_string()),
            extensions: Some(vec![]),
            worker: Some("helper".to_string()),
            ..Default::default()
        }
    }

    fn worker_followup_delegate_params(instructions: &str) -> DelegateParams {
        DelegateParams {
            instructions: Some(instructions.to_string()),
            worker: Some("helper".to_string()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_worker_create_then_resume_keeps_state() {
        let rig = worker_test_rig().await;

        let first = rig
            .client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(first.is_error, Some(true));

        let second = rig
            .client
            .handle_worker_delegate(
                &rig.session,
                worker_followup_delegate_params("second task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(second.is_error, Some(true));

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        let resumed = calls[1].join("\n");
        assert!(resumed.contains("first task"));
        assert!(resumed.contains("reply-1"));
        assert!(resumed.contains("second task"));
    }

    async fn persisted_worker_rig() -> WorkerTestRig {
        let mut rig = worker_test_rig().await;
        let parent = rig
            .client
            .context
            .session_manager
            .create_session(
                rig.session.working_dir.clone(),
                "parent".to_string(),
                SessionType::User,
                GooseMode::Auto,
            )
            .await
            .unwrap();
        rig.session.id = parent.id;
        rig
    }

    async fn stored_worker_record(rig: &WorkerTestRig) -> Option<WorkerRecord> {
        let session = rig
            .client
            .context
            .session_manager
            .get_session(&rig.session.id, false)
            .await
            .unwrap();
        WorkerRegistryState::from_extension_data(&session.extension_data)
            .and_then(|registry| registry.workers.get("helper").cloned())
    }

    #[tokio::test]
    async fn test_worker_restored_from_record_after_restart() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let record = stored_worker_record(&rig)
            .await
            .expect("worker record persisted");
        assert_eq!(record.creation.provider, "worker-test");
        assert!(!record.session_id.is_empty());

        let restarted = SummonClient::new(rig.client.context.clone()).unwrap();
        let second = restarted
            .handle_worker_delegate(
                &rig.session,
                worker_followup_delegate_params("second task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(second.is_error, Some(true));

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        let resumed = calls[1].join("\n");
        assert!(resumed.contains("first task"));
        assert!(resumed.contains("reply-1"));
        assert!(resumed.contains("second task"));
    }

    #[tokio::test]
    async fn test_worker_busy_lock_is_shared_across_clients() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let worker = ready_worker(&rig.client);
        let _held = worker.busy.clone().try_lock_owned().unwrap();

        let second_client = SummonClient::new(rig.client.context.clone()).unwrap();
        let err = second_client
            .handle_worker_delegate(
                &rig.session,
                worker_followup_delegate_params("second task"),
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(err.contains("busy"), "expected busy error, got: {}", err);
    }

    #[tokio::test]
    async fn test_worker_stale_record_falls_back_to_fresh_creation() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let old_record = stored_worker_record(&rig).await.unwrap();
        rig.client
            .context
            .session_manager
            .delete_session(&old_record.session_id)
            .await
            .unwrap();

        let restarted = SummonClient::new(rig.client.context.clone()).unwrap();
        let second = restarted
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("fresh task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(second.is_error, Some(true));

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        let fresh = calls[1].join("\n");
        assert!(fresh.contains("fresh task"));
        assert!(!fresh.contains("first task"));

        let new_record = stored_worker_record(&rig).await.unwrap();
        assert_eq!(new_record.recipe.prompt.as_deref(), Some("fresh task"));
    }

    #[tokio::test]
    async fn test_worker_deleted_session_recreates_in_memory_worker() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let old_session_id = ready_worker(&rig.client).session_id.clone();
        rig.client
            .context
            .session_manager
            .delete_session(&old_session_id)
            .await
            .unwrap();

        let second = rig
            .client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("fresh task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(second.is_error, Some(true));

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        let fresh = calls[1].join("\n");
        assert!(fresh.contains("fresh task"));
        assert!(!fresh.contains("first task"));
    }

    #[tokio::test]
    async fn test_worker_record_rejects_recycled_session_id() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let old_session_id = ready_worker(&rig.client).session_id.clone();
        let session_manager = &rig.client.context.session_manager;
        session_manager
            .delete_session(&old_session_id)
            .await
            .unwrap();

        let impostor = session_manager
            .create_session(
                rig.session.working_dir.clone(),
                "impostor".to_string(),
                SessionType::SubAgent,
                GooseMode::Auto,
            )
            .await
            .unwrap();
        session_manager
            .update(&impostor.id)
            .parent_session_id(Some(rig.session.id.clone()))
            .apply()
            .await
            .unwrap();
        assert_eq!(impostor.id, old_session_id, "test needs id recycling");

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("fresh task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(ready_worker(&rig.client).session_id, impostor.id);
    }

    #[tokio::test]
    async fn test_worker_checkout_rejects_recycled_session_id_from_other_client() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let old_worker = ready_worker(&rig.client);
        let old_session_id = old_worker.session_id.clone();
        let old_identity = old_worker.identity.clone();
        rig.client
            .context
            .session_manager
            .delete_session(&old_session_id)
            .await
            .unwrap();

        let other_client = SummonClient::new(rig.client.context.clone()).unwrap();
        other_client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("fresh task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(
            ready_worker(&other_client).session_id,
            old_session_id,
            "test needs id recycling"
        );

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_followup_delegate_params("follow up"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let current = ready_worker(&rig.client);
        assert_ne!(current.identity, old_identity);
        assert_eq!(current.identity, ready_worker(&other_client).identity);

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 3);
        let followed = calls[2].join("\n");
        assert!(followed.contains("fresh task"));
        assert!(followed.contains("follow up"));
        assert!(!followed.contains("first task"));
    }

    #[tokio::test]
    async fn test_worker_clears_stale_final_output_before_reply() {
        let rig = worker_test_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let worker = ready_worker(&rig.client);
        worker
            .configured
            .agent
            .add_final_output_tool(crate::recipe::Response {
                json_schema: Some(serde_json::json!({"type": "object"})),
            })
            .await;
        worker
            .configured
            .agent
            .final_output_tool
            .lock()
            .await
            .as_mut()
            .unwrap()
            .final_output = Some("stale output".to_string());

        let mut params = worker_followup_delegate_params("second task");
        params.max_turns = Some(1);
        rig.client
            .handle_worker_delegate(&rig.session, params, CancellationToken::new())
            .await
            .unwrap();

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert!(
            calls.len() >= 2,
            "provider was not called for the second delegation"
        );
        assert!(calls[1].join("\n").contains("second task"));
    }

    #[tokio::test]
    async fn test_worker_creation_context_is_call_scoped() {
        let rig = persisted_worker_rig().await;

        let mut params = worker_creation_delegate_params("first task");
        params.context = Some("ephemeral context".to_string());
        rig.client
            .handle_worker_delegate(&rig.session, params, CancellationToken::new())
            .await
            .unwrap();

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert!(calls[0].join("\n").contains("ephemeral context"));

        let record = stored_worker_record(&rig).await.unwrap();
        let instructions = record.recipe.instructions.clone().unwrap_or_default();
        assert!(!instructions.contains("ephemeral context"));
    }

    #[tokio::test]
    async fn test_worker_record_owned_by_another_session_is_discarded() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        let record = stored_worker_record(&rig).await.unwrap();

        let session_manager = &rig.client.context.session_manager;
        let other = session_manager
            .create_session(
                rig.session.working_dir.clone(),
                "other-parent".to_string(),
                SessionType::User,
                GooseMode::Auto,
            )
            .await
            .unwrap();
        let mut other_stored = session_manager.get_session(&other.id, false).await.unwrap();
        let mut registry = WorkerRegistryState::default();
        registry
            .workers
            .insert("helper".to_string(), record.clone());
        registry
            .to_extension_data(&mut other_stored.extension_data)
            .unwrap();
        session_manager
            .update(&other.id)
            .extension_data(other_stored.extension_data)
            .apply()
            .await
            .unwrap();

        let mut other_session = rig.session.clone();
        other_session.id = other.id.clone();

        let restarted = SummonClient::new(rig.client.context.clone()).unwrap();
        let result = restarted
            .handle_worker_delegate(
                &other_session,
                worker_creation_delegate_params("second task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        let fresh = calls[1].join("\n");
        assert!(fresh.contains("second task"));
        assert!(
            !fresh.contains("first task"),
            "copied record must not resume the original session"
        );

        let other_registry = WorkerRegistryState::from_extension_data(
            &session_manager
                .get_session(&other.id, false)
                .await
                .unwrap()
                .extension_data,
        )
        .unwrap();
        assert_ne!(
            other_registry.workers.get("helper").unwrap().session_id,
            record.session_id
        );
    }

    fn ready_worker(client: &SummonClient) -> Arc<PersistentWorker> {
        match client.workers.lock().unwrap().values().next().unwrap() {
            WorkerSlot::Ready(worker) => worker.clone(),
            WorkerSlot::Creating => panic!("worker should be ready"),
        }
    }

    #[tokio::test]
    async fn test_worker_idle_eviction_rehydrates_transparently() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let original = ready_worker(&rig.client);
        original.last_used.store(0, Ordering::Relaxed);

        let second = rig
            .client
            .handle_worker_delegate(
                &rig.session,
                worker_followup_delegate_params("second task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_ne!(second.is_error, Some(true));

        let current = ready_worker(&rig.client);
        assert!(
            !Arc::ptr_eq(&original, &current),
            "idle worker should have been evicted and rebuilt"
        );

        let calls = rig.provider.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        let resumed = calls[1].join("\n");
        assert!(resumed.contains("first task"));
        assert!(resumed.contains("reply-1"));
        assert!(resumed.contains("second task"));
    }

    #[tokio::test]
    async fn test_worker_busy_is_never_evicted() {
        let rig = persisted_worker_rig().await;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let worker = ready_worker(&rig.client);
        assert!(worker.persisted.load(Ordering::Relaxed));
        let _busy = worker.busy.clone().try_lock_owned().unwrap();
        worker.last_used.store(0, Ordering::Relaxed);

        let mut workers = rig.client.workers.lock().unwrap();
        rig.client.evict_idle_workers(&mut workers);
        assert_eq!(workers.len(), 1);
    }

    #[tokio::test]
    async fn test_worker_cap_evicts_least_recently_used_idle_worker() {
        let mut rig = persisted_worker_rig().await;
        rig.client.max_workers = 1;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let mut params = worker_creation_delegate_params("other task");
        params.worker = Some("helper2".to_string());
        let second = rig
            .client
            .handle_worker_delegate(&rig.session, params, CancellationToken::new())
            .await
            .unwrap();
        assert_ne!(second.is_error, Some(true));

        let workers = rig.client.workers.lock().unwrap();
        assert_eq!(workers.len(), 1);
        assert!(workers.keys().all(|key| key.ends_with(":helper2")));
    }

    #[tokio::test]
    async fn test_worker_cap_rejects_creation_when_all_workers_busy() {
        let mut rig = worker_test_rig().await;
        rig.client.max_workers = 1;

        rig.client
            .handle_worker_delegate(
                &rig.session,
                worker_creation_delegate_params("first task"),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        let worker = ready_worker(&rig.client);
        let _busy = worker.busy.clone().try_lock_owned().unwrap();

        let mut params = worker_creation_delegate_params("other task");
        params.worker = Some("helper2".to_string());
        let err = rig
            .client
            .handle_worker_delegate(&rig.session, params, CancellationToken::new())
            .await
            .unwrap_err();
        assert!(err.contains("Too many active workers"));
    }

    #[test]
    fn test_agent_frontmatter_parsing() {
        let agent = r#"---
name: reviewer
model: sonnet
---
You review code."#;
        let source = parse_agent_content(agent, Path::new("")).unwrap();
        assert_eq!(source.name, "reviewer");
        assert!(source.description.contains("sonnet"));
    }

    #[test]
    fn test_resolve_working_dir_relative_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path().canonicalize().unwrap();
        let subdir = parent.join("sub");
        fs::create_dir(&subdir).unwrap();

        let resolved = resolve_working_dir(&parent, "sub").unwrap();
        assert_eq!(resolved, subdir.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_working_dir_rejects_traversal_outside_parent() {
        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path().join("parent");
        let sibling = temp_dir.path().join("sibling");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&sibling).unwrap();

        let err = resolve_working_dir(&parent, "../sibling").unwrap_err();
        assert!(
            err.to_string()
                .contains("outside the parent session directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_resolve_working_dir_rejects_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path().canonicalize().unwrap();
        let file = parent.join("a.txt");
        fs::write(&file, "hello").unwrap();

        let err = resolve_working_dir(&parent, "a.txt").unwrap_err();
        assert!(
            err.to_string().contains("is not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_resolve_working_dir_rejects_nonexistent_path() {
        let temp_dir = TempDir::new().unwrap();
        let parent = temp_dir.path().canonicalize().unwrap();

        let err = resolve_working_dir(&parent, "does-not-exist").unwrap_err();
        assert!(
            err.to_string().contains("could not be resolved"),
            "unexpected error: {err}"
        );
    }
    #[test]
    fn test_agent_scan_skips_non_agent_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let agents_dir = temp_dir.path().join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("README.md"),
            "---\ntitle: Notes\n---\nThis is not an agent.",
        )
        .unwrap();
        fs::write(
            agents_dir.join("notes.md"),
            "---\nauthor: someone\ntags: [docs]\n---\nJust documentation.",
        )
        .unwrap();
        fs::write(
            agents_dir.join("reviewer.md"),
            "---\nname: reviewer\nmodel: sonnet\n---\nYou review code.",
        )
        .unwrap();
        fs::write(agents_dir.join("plain.md"), "No frontmatter at all.").unwrap();
        fs::write(
            agents_dir.join("broken.md"),
            "---\nname: [unterminated\n---\nBroken YAML.",
        )
        .unwrap();

        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        scan_agents_from_dir(&agents_dir, &mut sources, &mut seen);

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "reviewer");
    }

    #[test]
    fn test_recipe_scan_skips_non_recipe_project_config_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"scripts":{"test":"cargo test"}}"#,
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("tsconfig.json"),
            r#"{"compilerOptions":{"strict":true}}"#,
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("valid.yaml"),
            "title: Valid\ndescription: Real recipe\ninstructions: Run valid steps",
        )
        .unwrap();

        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        scan_recipes_from_dir(
            temp_dir.path(),
            SourceType::Recipe,
            true,
            &mut sources,
            &mut seen,
        );

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "valid");
        assert_eq!(sources[0].description, "Real recipe");
    }

    #[tokio::test]
    async fn test_discover_recipes_and_agents() {
        let temp_dir = TempDir::new().unwrap();

        let recipes = temp_dir.path().join(".goose/recipes");
        fs::create_dir_all(&recipes).unwrap();
        fs::write(
            recipes.join("deploy.yaml"),
            "title: Deploy\ndescription: Deploy to production\ninstructions: Run deploy steps",
        )
        .unwrap();

        let agents = temp_dir.path().join(".goose/agents");
        fs::create_dir_all(&agents).unwrap();
        fs::write(
            agents.join("reviewer.md"),
            "---\nname: reviewer\nmodel: sonnet\ndescription: Code reviewer\n---\nYou review code.",
        )
        .unwrap();

        let client = SummonClient::new(create_test_context()).unwrap();
        let sources = client.discover_filesystem_sources(temp_dir.path());

        let recipe = sources
            .iter()
            .find(|s| s.name == "deploy" && s.source_type == SourceType::Recipe)
            .unwrap();
        assert_eq!(recipe.description, "Deploy to production");
        assert_eq!(recipe.content, "Run deploy steps");

        let agent = sources
            .iter()
            .find(|s| s.name == "reviewer" && s.source_type == SourceType::Agent)
            .unwrap();
        assert_eq!(agent.description, "Code reviewer");
        assert!(agent.content.contains("You review code"));
    }

    #[tokio::test]
    async fn test_recipe_deduplication_local_wins() {
        let temp_dir = TempDir::new().unwrap();

        let local = temp_dir.path().join(".goose/recipes");
        fs::create_dir_all(&local).unwrap();
        fs::write(
            local.join("deploy.yaml"),
            "title: Deploy\ndescription: Local deploy\ninstructions: local steps",
        )
        .unwrap();

        let also_local = temp_dir.path().join(".agents/recipes");
        fs::create_dir_all(&also_local).unwrap();
        fs::write(
            also_local.join("deploy.yaml"),
            "title: Deploy\ndescription: Agents deploy\ninstructions: agents steps",
        )
        .unwrap();

        let client = SummonClient::new(create_test_context()).unwrap();
        let sources = client.discover_filesystem_sources(temp_dir.path());

        let deploys: Vec<_> = sources.iter().filter(|s| s.name == "deploy").collect();
        assert_eq!(deploys.len(), 1);
    }

    #[tokio::test]
    async fn test_load_recipe_source() {
        let temp_dir = TempDir::new().unwrap();

        let recipes = temp_dir.path().join(".goose/recipes");
        fs::create_dir_all(&recipes).unwrap();
        fs::write(
            recipes.join("deploy.yaml"),
            "title: Deploy\ndescription: Deploy to production\ninstructions: Run deploy steps",
        )
        .unwrap();

        let client = SummonClient::new(create_test_context()).unwrap();
        let result = client
            .handle_load_source("test", "deploy", temp_dir.path())
            .await
            .unwrap();

        let text = &result[0].as_text().expect("expected text content").text;
        assert!(text.contains("deploy"));
        assert!(text.contains("Run deploy steps"));
        assert!(text.contains("now available in your context"));
    }

    #[tokio::test]
    async fn test_load_agent_source() {
        let temp_dir = TempDir::new().unwrap();

        let agents = temp_dir.path().join(".goose/agents");
        fs::create_dir_all(&agents).unwrap();
        fs::write(
            agents.join("reviewer.md"),
            "---\nname: reviewer\nmodel: sonnet\ndescription: Code reviewer\n---\nYou review code carefully.",
        )
        .unwrap();

        let client = SummonClient::new(create_test_context()).unwrap();
        let result = client
            .handle_load_source("test", "reviewer", temp_dir.path())
            .await
            .unwrap();

        let text = &result[0].as_text().expect("expected text content").text;
        assert!(text.contains("reviewer"));
        assert!(text.contains("You review code carefully"));
        assert!(text.contains("now available in your context"));
    }

    #[tokio::test]
    async fn test_load_nonexistent_source_suggests_similar() {
        let temp_dir = TempDir::new().unwrap();

        let recipes = temp_dir.path().join(".goose/recipes");
        fs::create_dir_all(&recipes).unwrap();
        fs::write(
            recipes.join("deploy.yaml"),
            "title: Deploy\ndescription: Deploy to production\ninstructions: steps",
        )
        .unwrap();

        let client = SummonClient::new(create_test_context()).unwrap();
        let err = client
            .handle_load_source("test", "deploy-prod", temp_dir.path())
            .await
            .unwrap_err();

        assert!(err.contains("not found"));
        assert!(err.contains("deploy"), "should suggest 'deploy': {}", err);
    }

    #[tokio::test]
    async fn test_load_completely_unknown_source() {
        let temp_dir = TempDir::new().unwrap();

        let client = SummonClient::new(create_test_context()).unwrap();
        let err = client
            .handle_load_source("test", "zzz-nonexistent", temp_dir.path())
            .await
            .unwrap_err();

        assert!(err.contains("not found"));
        assert!(err.contains("Use load()"));
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

        let ctx = ToolCallContext::new("test".to_string(), None, None);
        let result = client
            .call_tool(&ctx, "unknown", None, CancellationToken::new())
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
            ..Default::default()
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
        assert_eq!(
            SummonClient::get_task_description(&make_params(None, None)),
            "Unknown task"
        );
    }

    #[tokio::test]
    async fn test_context_injected_into_adhoc_recipe() {
        let temp_dir = TempDir::new().unwrap();
        let client = SummonClient::new(create_test_context()).unwrap();

        let params = DelegateParams {
            instructions: Some("do the task".to_string()),
            context: Some("background info".to_string()),
            ..Default::default()
        };

        let recipe = client
            .build_delegate_recipe(&params, "test", temp_dir.path())
            .await
            .unwrap();

        assert_eq!(
            recipe.instructions.as_deref(),
            Some("# Reference Context\n\nbackground info")
        );
        assert_eq!(recipe.prompt.as_deref(), Some("do the task"));
    }

    #[test]
    fn test_subrecipe_fixed_values_take_precedence_over_delegate_parameters() {
        let fixed = HashMap::from([("fixed".to_string(), "parent-value".to_string())]);
        let provided = HashMap::from([
            (
                "fixed".to_string(),
                serde_json::Value::String("delegate-value".to_string()),
            ),
            (
                "caller".to_string(),
                serde_json::Value::String("caller-value".to_string()),
            ),
        ]);

        let merged = merge_subrecipe_parameters(Some(&fixed), Some(&provided));

        assert_eq!(
            merged.get("fixed").map(String::as_str),
            Some("parent-value")
        );
        assert_eq!(
            merged.get("caller").map(String::as_str),
            Some("caller-value")
        );
    }

    #[test]
    fn test_build_instructions_with_context_wraps_existing_instructions() {
        assert_eq!(
            build_instructions_with_context("background info", "Run deploy steps"),
            "# Reference Context\n\nbackground info\n\n# Task Instructions\n\nRun deploy steps"
        );
        assert_eq!(
            build_instructions_with_context("background info", ""),
            "# Reference Context\n\nbackground info"
        );
    }

    #[test]
    fn test_validate_delegate_params_rejects_zero_max_turns() {
        let context = create_test_context();
        let client = SummonClient::new(context).unwrap();

        let params = DelegateParams {
            instructions: Some("do something".to_string()),
            max_turns: Some(0),
            ..Default::default()
        };
        let result = client.validate_delegate_params(&params);
        assert_eq!(result, Err("'max_turns' must be at least 1".to_string()));
    }

    #[test]
    fn test_validate_delegate_params_accepts_positive_max_turns() {
        let context = create_test_context();
        let client = SummonClient::new(context).unwrap();

        let params = DelegateParams {
            instructions: Some("do something".to_string()),
            max_turns: Some(5),
            ..Default::default()
        };
        assert!(client.validate_delegate_params(&params).is_ok());
    }

    #[test]
    #[serial]
    fn test_resolve_max_turns_recipe_overrides_env_var() {
        let context = create_test_context();
        let client = SummonClient::new(context).unwrap();

        let session = crate::session::Session {
            recipe: Some(crate::recipe::Recipe {
                version: "1.0.0".to_string(),
                title: String::new(),
                description: String::new(),
                instructions: None,
                prompt: None,
                extensions: None,
                settings: Some(crate::recipe::Settings {
                    goose_provider: None,
                    goose_model: None,
                    temperature: None,
                    max_turns: Some(10),
                }),
                activities: None,
                author: None,
                parameters: None,
                response: None,
                sub_recipes: None,
                retry: None,
            }),
            ..Default::default()
        };

        // Set env var to a different value — recipe should still win
        std::env::set_var("GOOSE_SUBAGENT_MAX_TURNS", "99");
        let result = client.resolve_max_turns(&session);
        std::env::remove_var("GOOSE_SUBAGENT_MAX_TURNS");

        assert_eq!(
            result, 10,
            "recipe settings.max_turns should take priority over env var"
        );
    }

    #[test]
    #[serial]
    fn test_resolve_max_turns_falls_back_to_env_var() {
        let context = create_test_context();
        let client = SummonClient::new(context).unwrap();

        let session = crate::session::Session::default(); // no recipe

        std::env::set_var("GOOSE_SUBAGENT_MAX_TURNS", "7");
        let result = client.resolve_max_turns(&session);
        std::env::remove_var("GOOSE_SUBAGENT_MAX_TURNS");

        assert_eq!(
            result, 7,
            "should fall back to GOOSE_SUBAGENT_MAX_TURNS env var"
        );
    }

    #[test]
    #[serial]
    fn test_resolve_max_turns_falls_back_to_default() {
        let context = create_test_context();
        let client = SummonClient::new(context).unwrap();

        let session = crate::session::Session::default(); // no recipe

        std::env::remove_var("GOOSE_SUBAGENT_MAX_TURNS");
        let result = client.resolve_max_turns(&session);

        assert_eq!(
            result,
            crate::agents::subagent_task_config::DEFAULT_SUBAGENT_MAX_TURNS,
            "should fall back to DEFAULT_SUBAGENT_MAX_TURNS"
        );
    }

    fn empty_recipe() -> crate::recipe::Recipe {
        crate::recipe::Recipe {
            version: "1.0.0".to_string(),
            title: String::new(),
            description: String::new(),
            instructions: None,
            prompt: None,
            extensions: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
            retry: None,
        }
    }

    #[tokio::test]
    async fn test_resolve_provider_reuses_unregistered_parent_provider() {
        let temp_dir = TempDir::new().unwrap();
        let parent_provider: Arc<dyn crate::providers::base::Provider> = Arc::new(
            crate::providers::testprovider::TestProvider::new_replaying(
                temp_dir.path().join("records.json").display().to_string(),
            )
            .unwrap(),
        );
        let extension_manager = Arc::new(
            crate::agents::extension_manager::ExtensionManager::new_without_provider(
                temp_dir.path().to_path_buf(),
            ),
        );
        *extension_manager.get_provider().lock().await = Some(Arc::clone(&parent_provider));
        let mut context = extension_manager.get_context().clone();
        context.extension_manager = Some(Arc::downgrade(&extension_manager));
        let client = SummonClient::new(context).unwrap();
        let session = crate::session::Session {
            provider_name: Some(parent_provider.get_name().to_string()),
            model_config: Some(goose_providers::model::ModelConfig::new("test-model")),
            ..Default::default()
        };

        let params = DelegateParams {
            provider: Some(parent_provider.get_name().to_string()),
            model: Some("test-model".to_string()),
            ..Default::default()
        };
        let (resolved_provider, _) = client
            .resolve_provider(&params, &empty_recipe(), &session, &[])
            .await
            .unwrap();

        assert!(Arc::ptr_eq(&parent_provider, &resolved_provider));
    }

    #[tokio::test]
    async fn test_build_task_config_recreates_registered_parent_provider() {
        let temp_dir = TempDir::new().unwrap();
        let parent_provider = providers::create("openai", Vec::new()).await.unwrap();
        let extension_manager = Arc::new(
            crate::agents::extension_manager::ExtensionManager::new_without_provider(
                temp_dir.path().to_path_buf(),
            ),
        );
        *extension_manager.get_provider().lock().await = Some(Arc::clone(&parent_provider));
        let mut context = extension_manager.get_context().clone();
        context.extension_manager = Some(Arc::downgrade(&extension_manager));
        let client = SummonClient::new(context).unwrap();
        let session = crate::session::Session {
            provider_name: Some(parent_provider.get_name().to_string()),
            model_config: Some(goose_providers::model::ModelConfig::new("test-model")),
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let params = DelegateParams {
            extensions: Some(Vec::new()),
            provider: Some(parent_provider.get_name().to_string()),
            model: Some("test-model".to_string()),
            ..Default::default()
        };

        let task_config = client
            .build_task_config(&params, &empty_recipe(), &session, None)
            .await
            .unwrap();

        assert!(!Arc::ptr_eq(&parent_provider, &task_config.provider));
        assert!(task_config.extensions.is_empty());
    }

    const PARENT_MODEL: &str = "claude-3-5-sonnet-20241022";
    const OVERRIDE_MODEL: &str = "claude-opus-4-6";
    const PROVIDER: &str = "anthropic";

    fn session_with(parent: goose_providers::model::ModelConfig) -> crate::session::Session {
        crate::session::Session {
            provider_name: Some(PROVIDER.to_string()),
            model_config: Some(parent),
            ..Default::default()
        }
    }

    fn resolve_with_override(
        model: Option<&str>,
        parent: goose_providers::model::ModelConfig,
    ) -> goose_providers::model::ModelConfig {
        let client = SummonClient::new(create_test_context()).unwrap();
        let params = DelegateParams {
            model: model.map(String::from),
            ..Default::default()
        };
        client
            .resolve_model_config(&params, &empty_recipe(), &session_with(parent), PROVIDER)
            .expect("resolve_model_config")
    }

    fn parent_config() -> goose_providers::model::ModelConfig {
        goose_providers::model::ModelConfig::new(PARENT_MODEL).with_canonical_limits(PROVIDER)
    }

    #[tokio::test]
    #[serial]
    async fn test_resolve_model_config_applies_canonical_limits_to_overridden_model() {
        let _env = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
            ("GOOSE_SUBAGENT_MODEL", None::<&str>),
        ]);

        let parent = parent_config();
        let overridden = goose_providers::model::ModelConfig::new(OVERRIDE_MODEL)
            .with_canonical_limits(PROVIDER);
        assert_ne!(parent.context_limit, overridden.context_limit);
        assert_ne!(parent.reasoning, overridden.reasoning);

        let resolved = resolve_with_override(Some(OVERRIDE_MODEL), parent);

        assert_eq!(resolved.model_name, OVERRIDE_MODEL);
        assert_eq!(resolved.context_limit, overridden.context_limit);
        assert_eq!(resolved.max_tokens, overridden.max_tokens);
        assert_eq!(resolved.reasoning, overridden.reasoning);
    }

    #[tokio::test]
    #[serial]
    async fn test_resolve_model_config_does_not_inherit_provider_specific_request_params() {
        let _env = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
            ("GOOSE_SUBAGENT_MODEL", None::<&str>),
        ]);

        // Parent session is a Claude model with anthropic_beta in request_params.
        // When delegate() overrides to a different model (e.g. Gemini), provider-
        // specific params like anthropic_beta must not bleed through — they would
        // cause a 400 INVALID_ARGUMENT from the target API.
        let mut parent = parent_config();
        parent.request_params = Some(HashMap::from([(
            "anthropic_beta".to_string(),
            serde_json::json!("custom-beta-header"),
        )]));

        let resolved = resolve_with_override(Some(OVERRIDE_MODEL), parent);

        assert_eq!(
            resolved
                .request_params
                .as_ref()
                .and_then(|p| p.get("anthropic_beta")),
            None,
            "anthropic_beta must not be inherited by a child session with a different model"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_resolve_model_config_inherits_thinking_effort_on_override() {
        let _env = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
            ("GOOSE_SUBAGENT_MODEL", None::<&str>),
        ]);

        // Reasoning controls are model-family-agnostic and should be inherited,
        // while provider-specific params like anthropic_beta must not.
        let mut parent = parent_config();
        parent.request_params = Some(HashMap::from([
            ("thinking_effort".to_string(), serde_json::json!("high")),
            ("budget_tokens".to_string(), serde_json::json!(8192)),
            (
                "anthropic_beta".to_string(),
                serde_json::json!("custom-beta-header"),
            ),
        ]));

        let resolved = resolve_with_override(Some(OVERRIDE_MODEL), parent);

        assert_eq!(
            resolved
                .request_params
                .as_ref()
                .and_then(|p| p.get("thinking_effort")),
            Some(&serde_json::json!("high")),
            "thinking_effort should be inherited across model families"
        );
        assert_eq!(
            resolved
                .request_params
                .as_ref()
                .and_then(|p| p.get("budget_tokens")),
            Some(&serde_json::json!(8192)),
            "budget_tokens should be inherited across model families"
        );
        assert_eq!(
            resolved
                .request_params
                .as_ref()
                .and_then(|p| p.get("anthropic_beta")),
            None,
            "anthropic_beta must not be inherited alongside reasoning controls"
        );
    }

    fn extract_text(content: &ContentBlock) -> &str {
        use rmcp::model::ContentBlock;
        match content {
            ContentBlock::Text(t) => t.text.as_str(),
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

        let result = client
            .handle_load_task_result("20260204_999", false, false)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        {
            use crate::agents::subagent_handler::create_tool_notification;
            use crate::conversation::message::MessageContent;
            use rmcp::model::CallToolRequestParams;

            let tool_call = CallToolRequestParams::new("developer__shell").with_arguments(
                serde_json::json!({"command": "ls"})
                    .as_object()
                    .unwrap()
                    .clone(),
            );
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
            .handle_load_task_result("20260204_1", false, false)
            .await
            .expect("load should wait and return result");
        let text = extract_text(&result.content[0]);
        assert!(text.contains("Completed"));
        assert!(text.contains("done"));

        let notif = subscriber
            .try_recv()
            .expect("subscriber should receive buffered notification");
        if let ServerNotification::LoggingMessageNotification(log) = notif {
            let params = serde_json::to_value(&log.params).unwrap();
            let data = params.get("data").and_then(|v| v.as_object()).unwrap();
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
                    completed_at: Instant::now(),
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
                    completed_at: Instant::now(),
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
            .handle_load_task_result("20260204_2", false, false)
            .await
            .unwrap();
        let text = extract_text(&result.content[0]);
        assert!(text.contains("20260204_2"));
        assert!(text.contains("Successful task"));
        assert!(text.contains("✓ Completed"));
        assert!(text.contains("1m"));
        assert!(text.contains("5 turns"));
        assert!(text.contains("Task completed successfully with output"));
        assert_eq!(result.status, "completed");
        assert_eq!(result.turns, Some(5));

        assert!(!client
            .completed_tasks
            .lock()
            .await
            .contains_key("20260204_2"));

        let result = client
            .handle_load_task_result("20260204_3", false, false)
            .await
            .unwrap();
        let text = extract_text(&result.content[0]);
        assert!(text.contains("✗ Failed"));
        assert!(text.contains("Error: Something went wrong"));
        assert_eq!(result.status, "failed");

        let result = client
            .handle_load_task_result("20260204_3", false, false)
            .await;
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
            .handle_load_task_result("20260204_1", true, false)
            .await
            .unwrap();
        let text = extract_text(&result.content[0]);
        assert!(text.contains("Cancelled"));
        assert!(text.contains("20260204_1"));
        assert!(text.contains("Cancellable task"));
        assert_eq!(result.status, "cancelled");
        assert_eq!(result.turns, Some(3));
        assert!(token.is_cancelled());
        assert!(!client
            .background_tasks
            .lock()
            .await
            .contains_key("20260204_1"));
    }

    #[tokio::test]
    async fn test_peek_running_task() {
        let client = SummonClient::new(create_test_context()).unwrap();

        {
            let mut running = client.background_tasks.lock().await;
            running.insert(
                "20260204_1".to_string(),
                BackgroundTask {
                    id: "20260204_1".to_string(),
                    description: "Long running analysis".to_string(),
                    started_at: Instant::now(),
                    turns: Arc::new(AtomicU32::new(7)),
                    last_activity: Arc::new(AtomicU64::new(current_epoch_millis())),
                    handle: tokio::spawn(async {
                        tokio::time::sleep(Duration::from_secs(1000)).await;
                        Ok("eventual result".to_string())
                    }),
                    cancellation_token: CancellationToken::new(),
                    notification_buffer: Arc::new(Mutex::new(Vec::new())),
                },
            );
        }

        // Peek should return status without removing the task
        let result = client
            .handle_load_task_result("20260204_1", false, true)
            .await
            .unwrap();
        let text = extract_text(&result.content[0]);
        assert!(text.contains("Running"));
        assert!(text.contains("Long running analysis"));
        assert!(text.contains("7")); // turns taken

        // Task should still be in background_tasks (not consumed)
        assert!(client
            .background_tasks
            .lock()
            .await
            .contains_key("20260204_1"));
    }

    #[tokio::test]
    async fn test_peek_nonexistent_task() {
        let client = SummonClient::new(create_test_context()).unwrap();

        let result = client
            .handle_load_task_result("20260204_999", false, true)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_peek_completed_task_returns_result() {
        let client = SummonClient::new(create_test_context()).unwrap();

        {
            let mut completed = client.completed_tasks.lock().await;
            completed.insert(
                "20260204_1".to_string(),
                CompletedTask {
                    id: "20260204_1".to_string(),
                    description: "Finished task".to_string(),
                    result: Ok("final output".to_string()),
                    turns_taken: 4,
                    duration: Duration::from_secs(30),
                    completed_at: Instant::now(),
                },
            );
        }

        // Peek on a completed task should return the full result (same as non-peek)
        let result = client
            .handle_load_task_result("20260204_1", false, true)
            .await
            .unwrap();
        let text = extract_text(&result.content[0]);
        assert!(text.contains("Completed"));
        assert!(text.contains("final output"));

        // Peek must be non-destructive: the result is still retrievable afterwards.
        assert!(client
            .completed_tasks
            .lock()
            .await
            .contains_key("20260204_1"));
        let result = client
            .handle_load_task_result("20260204_1", false, false)
            .await
            .unwrap();
        assert!(extract_text(&result.content[0]).contains("final output"));
    }
}
