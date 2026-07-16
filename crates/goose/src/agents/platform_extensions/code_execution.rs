use crate::agents::extension::PlatformExtensionContext;
use crate::agents::extension_manager::{get_tool_owner, get_tool_resource_uri};
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::tool_execution::ToolCallContext;
use anyhow::Result;
use async_trait::async_trait;
use pctx_code_mode::{
    config::ToolDisclosure,
    descriptions::{tools as tool_descriptions, workflow::get_workflow_description},
    model::{CallbackConfig, ExecuteBashInput, ExecuteInput, GetFunctionDetailsInput},
    registry::{CallbackFn, PctxRegistry},
    CodeMode,
};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, InitializeResult, JsonObject,
    ListToolsResult, RawContent, Role, ServerCapabilities, Tool as McpTool, ToolAnnotations,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "code_execution";

fn sanitize_schema_for_code_mode(schema: &mut Value) {
    let Some(obj) = schema.as_object_mut() else {
        return;
    };

    let Some(defs_key) = ["$defs", "definitions"]
        .into_iter()
        .find(|key| obj.get(*key).is_some_and(Value::is_object))
    else {
        return;
    };

    let names: Vec<String> = obj[defs_key]
        .as_object()
        .map(|defs| defs.keys().cloned().collect())
        .unwrap_or_default();

    let mut edges: HashMap<String, HashSet<String>> = HashMap::new();
    if let Some(defs) = obj.get(defs_key).and_then(Value::as_object) {
        for name in &names {
            let mut refs = HashSet::new();
            if let Some(def_value) = defs.get(name) {
                collect_ref_targets(def_value, &mut refs);
            }
            edges.insert(name.clone(), refs);
        }
    }

    let cuts = find_cycle_edges(&names, &edges);
    if cuts.is_empty() {
        return;
    }

    if let Some(defs) = obj.get_mut(defs_key).and_then(Value::as_object_mut) {
        for (from, to) in &cuts {
            if let Some(def_value) = defs.get_mut(from) {
                neutralize_refs_to(def_value, to);
            }
        }
    }
}

fn collect_ref_targets(value: &Value, out: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get("$ref") {
                if let Some(name) = r.rsplit('/').next() {
                    out.insert(name.to_string());
                }
            }
            map.values().for_each(|v| collect_ref_targets(v, out));
        }
        Value::Array(items) => items.iter().for_each(|v| collect_ref_targets(v, out)),
        _ => {}
    }
}

fn neutralize_refs_to(value: &mut Value, target: &str) {
    let is_target_ref = matches!(
        value.as_object().and_then(|map| map.get("$ref")),
        Some(Value::String(r)) if r.rsplit('/').next() == Some(target)
    );
    if is_target_ref {
        *value = json!({});
        return;
    }
    match value {
        Value::Object(map) => map.values_mut().for_each(|v| neutralize_refs_to(v, target)),
        Value::Array(items) => items.iter_mut().for_each(|v| neutralize_refs_to(v, target)),
        _ => {}
    }
}

fn find_cycle_edges(
    names: &[String],
    edges: &HashMap<String, HashSet<String>>,
) -> Vec<(String, String)> {
    enum State {
        InProgress,
        Done,
    }

    fn visit<'a>(
        node: &'a str,
        edges: &'a HashMap<String, HashSet<String>>,
        state: &mut HashMap<&'a str, State>,
        cuts: &mut Vec<(String, String)>,
    ) {
        state.insert(node, State::InProgress);
        if let Some(targets) = edges.get(node) {
            for target in targets {
                match state.get(target.as_str()) {
                    Some(State::InProgress) => cuts.push((node.to_string(), target.clone())),
                    Some(State::Done) => {}
                    None => visit(target, edges, state, cuts),
                }
            }
        }
        state.insert(node, State::Done);
    }

    let mut state: HashMap<&str, State> = HashMap::new();
    let mut cuts = Vec::new();
    for name in names {
        if !state.contains_key(name.as_str()) {
            visit(name, edges, &mut state, &mut cuts);
        }
    }
    cuts
}

pub struct CodeExecutionClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    disclosure: ToolDisclosure,
    state: RwLock<Option<CodeModeState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ToolGraphNode {
    /// Tool name in format "server/tool" (e.g., "developer/shell")
    tool: String,
    /// Brief description of what this call does (e.g., "list files in /src")
    description: String,
    /// Indices of nodes this depends on (empty if no dependencies)
    #[serde(default)]
    depends_on: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteWithToolGraph {
    #[serde(flatten)]
    input: ExecuteInput,
    /// DAG of tool calls showing execution flow. Each node represents a tool call.
    /// Use depends_on to show data flow (e.g., node 1 uses output from node 0).
    #[serde(default)]
    tool_graph: Vec<ToolGraphNode>,
}

impl CodeExecutionClient {
    pub fn new(context: PlatformExtensionContext, disclosure: ToolDisclosure) -> Result<Self> {
        let info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new(EXTENSION_NAME.to_string(), "1.0.0".to_string())
                    .with_title("Code Mode"),
            )
            .with_instructions(get_workflow_description(disclosure));

        Ok(Self {
            info,
            context,
            disclosure,
            state: RwLock::new(None),
        })
    }

    async fn load_callback_configs(&self, session_id: &str) -> Option<Vec<CallbackConfig>> {
        let manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())?;

        let tools = manager
            .get_prefixed_tools_excluding(session_id, EXTENSION_NAME)
            .await
            .ok()?;

        let mut cfgs = vec![];
        for tool in tools {
            if get_tool_resource_uri(&tool).is_some() {
                continue;
            }

            let (name, namespace) = if let Some((prefix, tool_name)) = tool.name.split_once("__") {
                (tool_name.to_string(), Some(prefix.to_string()))
            } else if let Some(owner) = get_tool_owner(&tool) {
                (tool.name.to_string(), Some(owner))
            } else {
                (tool.name.to_string(), None)
            };

            let mut input_schema = json!(tool.input_schema);
            sanitize_schema_for_code_mode(&mut input_schema);

            let mut output_schema = tool.output_schema.as_ref().map(|s| json!(s));
            if let Some(schema) = output_schema.as_mut() {
                sanitize_schema_for_code_mode(schema);
            }

            cfgs.push(CallbackConfig {
                name,
                namespace,
                description: tool.description.as_ref().map(|d| d.to_string()),
                input_schema: Some(input_schema),
                output_schema,
            })
        }
        Some(cfgs)
    }

    /// Get the cached CodeMode, rebuilding if callback configs have changed
    async fn get_code_mode(&self, session_id: &str) -> Result<CodeMode, String> {
        let cfgs = self
            .load_callback_configs(session_id)
            .await
            .ok_or("Failed to load callback configs")?;
        let current_hash = CodeModeState::hash(&cfgs);

        // Use cache if no state change
        {
            let guard = self.state.read().await;
            if let Some(state) = guard.as_ref() {
                if state.hash == current_hash {
                    return Ok(state.code_mode.clone());
                }
            }
        }

        // Rebuild CodeMode & cache
        let mut guard = self.state.write().await;
        // Double-check after acquiring write lock
        if let Some(state) = guard.as_ref() {
            if state.hash == current_hash {
                return Ok(state.code_mode.clone());
            }
        }

        let state = CodeModeState::new(cfgs)?;
        let code_mode = state.code_mode.clone();
        *guard = Some(state);

        Ok(code_mode)
    }

    /// Build a PctxRegistry with all tool callbacks registered
    fn build_callback_registry(
        &self,
        ctx: &ToolCallContext,
        code_mode: &CodeMode,
        cancellation_token: CancellationToken,
    ) -> Result<PctxRegistry, String> {
        let manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())
            .ok_or("Extension manager not available")?;

        let registry = PctxRegistry::default();
        for cfg in code_mode.callbacks() {
            let full_name = format!(
                "{}{}",
                cfg.namespace
                    .clone()
                    .map(|n| format!("{n}__"))
                    .unwrap_or_default(),
                &cfg.name
            );
            let callback = create_tool_callback(
                ctx.clone(),
                full_name,
                manager.clone(),
                cancellation_token.clone(),
            );
            registry
                .add_callback(&cfg.id(), callback)
                .map_err(|e| format!("Failed to register callback: {e}"))?;
        }

        Ok(registry)
    }

    /// Handle the list_functions tool call
    async fn handle_list_functions(&self, session_id: &str) -> Result<Vec<Content>, String> {
        let code_mode = self.get_code_mode(session_id).await?;
        let output = code_mode.list_functions();

        Ok(vec![Content::text(output.code)])
    }

    /// Handle the get_function_details tool call
    async fn handle_get_function_details(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let input: GetFunctionDetailsInput = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for get_function_details")?;

        let code_mode = self.get_code_mode(session_id).await?;
        let output = code_mode.get_function_details(input);

        Ok(vec![Content::text(output.code)])
    }

    /// Handle the execute bash tool call
    async fn handle_execute_bash(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, String> {
        let input: ExecuteBashInput = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for execute_bash")?;
        let command = input.command;
        let code_mode = self.get_code_mode(session_id).await?;

        let dispatch_token = cancellation_token.child_token();
        let output = run_in_deno_runtime(
            execution_timeout(),
            cancellation_token,
            dispatch_token,
            move || async move {
                code_mode
                    .execute_bash(&command)
                    .await
                    .map_err(|e| format!("Bash execution error: {e}"))
            },
        )
        .await?;

        Ok(vec![Content::text(output.markdown())])
    }

    /// Handle the execute typescript tool call
    async fn handle_execute_typescript(
        &self,
        ctx: &ToolCallContext,
        arguments: Option<JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, String> {
        let args: ExecuteWithToolGraph = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for execute_typescript")?;

        let session_id = &ctx.session_id;
        let code_mode = self.get_code_mode(session_id).await?;
        let dispatch_token = cancellation_token.child_token();
        let registry = self.build_callback_registry(ctx, &code_mode, dispatch_token.clone())?;
        let code = args.input.code.clone();
        let disclosure = self.disclosure;

        let output = run_in_deno_runtime(
            execution_timeout(),
            cancellation_token,
            dispatch_token,
            move || async move {
                code_mode
                    .execute_typescript(&code, disclosure, Some(registry))
                    .await
                    .map_err(|e| format!("Typescript execution error: {e}"))
            },
        )
        .await?;

        Ok(vec![Content::text(output.markdown())])
    }
}

fn execution_timeout() -> Duration {
    let secs = crate::config::Config::global()
        .get_goose_default_extension_timeout()
        .unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT);
    Duration::from_secs(secs)
}

/// Deno runtime is not Send, so execution runs in a blocking task with its
/// own tokio runtime. pctx serializes all executions behind a process-wide
/// V8 mutex, so a hung script would wedge code execution for every session:
/// bound the wait with the extension timeout and honor cancellation.
///
/// `dispatch_token` is the child token shared with the callback dispatches that
/// a script makes back into Goose tools. When execution is abandoned (timeout
/// or cancellation), the token is cancelled so an in-flight nested tool call
/// (e.g. a long `developer.shell` command) is told to stop instead of running
/// on in the background.
/// Grace period for nested tool calls to observe a dispatched cancellation
/// signal and clean up (e.g. kill child processes) before the task future is
/// abandoned.
const DISPATCH_DRAIN_TIMEOUT: Duration = Duration::from_millis(500);

async fn run_in_deno_runtime<T, F, Fut>(
    timeout: Duration,
    cancellation_token: CancellationToken,
    dispatch_token: CancellationToken,
    task: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, String>>,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {e}"))?;

        rt.block_on(async move {
            let task_future = task();
            tokio::pin!(task_future);

            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    dispatch_token.cancel();
                    let _ = tokio::time::timeout(
                        DISPATCH_DRAIN_TIMEOUT,
                        &mut task_future,
                    ).await;
                    Err("Execution cancelled".to_string())
                }
                _ = tokio::time::sleep(timeout) => {
                    dispatch_token.cancel();
                    let _ = tokio::time::timeout(
                        DISPATCH_DRAIN_TIMEOUT,
                        &mut task_future,
                    ).await;
                    Err(format!(
                        "Execution timed out after {} seconds",
                        timeout.as_secs()
                    ))
                }
                result = &mut task_future => result,
            }
        })
    })
    .await
    .map_err(|e| format!("Execution task failed: {e}"))?
}

fn create_tool_callback(
    ctx: ToolCallContext,
    full_name: String,
    manager: Arc<crate::agents::ExtensionManager>,
    cancellation_token: CancellationToken,
) -> CallbackFn {
    Arc::new(move |args: Option<Value>| {
        let ctx = ctx.clone();
        let full_name = full_name.clone();
        let manager = manager.clone();
        let cancellation_token = cancellation_token.clone();
        Box::pin(async move {
            let tool_call = {
                let mut params = CallToolRequestParams::new(full_name);
                if let Some(args) = args.and_then(|v| v.as_object().cloned()) {
                    params = params.with_arguments(args);
                }
                params
            };
            match manager
                .dispatch_tool_call(&ctx, tool_call, cancellation_token)
                .await
            {
                Ok(dispatch_result) => match dispatch_result.result.await {
                    Ok(result) => {
                        if let Some(sc) = &result.structured_content {
                            Ok(serde_json::to_value(sc).unwrap_or(Value::Null))
                        } else {
                            // Filter to assistant-audience or no-audience content,
                            // skipping user-only content to avoid duplicated output
                            let text: String = result
                                .content
                                .iter()
                                .filter(|c| {
                                    c.audience().is_none_or(|audiences| {
                                        audiences.is_empty() || audiences.contains(&Role::Assistant)
                                    })
                                })
                                .filter_map(|c| match &c.raw {
                                    RawContent::Text(t) => Some(t.text.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            // Try to parse as JSON, otherwise return as string
                            Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
                        }
                    }
                    Err(e) => Err(format!("Tool error: {}", e.message)),
                },
                Err(e) => Err(format!("Dispatch error: {e}")),
            }
        }) as Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
    })
}

#[async_trait]
impl McpClientTrait for CodeExecutionClient {
    #[allow(clippy::too_many_lines)]
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        fn schema<T: JsonSchema>() -> JsonObject {
            serde_json::to_value(schema_for!(T))
                .map(|v| v.as_object().unwrap().clone())
                .expect("valid schema")
        }

        // Empty schema for list_functions (no parameters)
        let empty_schema: JsonObject = serde_json::from_value(json!({
            "type": "object",
            "properties": {},
            "required": []
        }))
        .expect("valid schema");

        let tools = match self.disclosure {
            ToolDisclosure::Catalog => {
                vec![
                    McpTool::new(
                        "list_functions".to_string(),
                        tool_descriptions::LIST_FUNCTIONS.to_string(),
                        empty_schema,
                    )
                    .annotate(ToolAnnotations::from_raw(
                        Some("List functions".to_string()),
                        Some(true),
                        Some(false),
                        Some(true),
                        Some(false),
                    )),
                    McpTool::new(
                        "get_function_details".to_string(),
                        tool_descriptions::GET_FUNCTION_DETAILS.to_string(),
                        schema::<GetFunctionDetailsInput>(),
                    )
                    .annotate(ToolAnnotations::from_raw(
                        Some("Get function details".to_string()),
                        Some(true),
                        Some(false),
                        Some(true),
                        Some(false),
                    )),
                    McpTool::new(
                        "execute_typescript".to_string(),
                        tool_descriptions::EXECUTE_TYPESCRIPT_CATALOG.to_string(),
                        schema::<ExecuteWithToolGraph>(),
                    )
                    .annotate(ToolAnnotations::from_raw(
                        Some("Execute TypeScript".to_string()),
                        Some(false),
                        Some(true),
                        Some(false),
                        Some(true),
                    )),
                ]
            }
            ToolDisclosure::Filesystem => {
                vec![
                    McpTool::new(
                        "execute_bash".to_string(),
                        tool_descriptions::EXECUTE_BASH.to_string(),
                        schema::<ExecuteBashInput>(),
                    )
                    .annotate(ToolAnnotations::from_raw(
                        Some("Get function details".to_string()),
                        Some(true),
                        Some(false),
                        Some(true),
                        Some(false),
                    )),
                    McpTool::new(
                        "execute_typescript".to_string(),
                        tool_descriptions::EXECUTE_TYPESCRIPT_FILESYSTEM.to_string(),
                        schema::<ExecuteWithToolGraph>(),
                    )
                    .annotate(ToolAnnotations::from_raw(
                        Some("Execute TypeScript".to_string()),
                        Some(false),
                        Some(true),
                        Some(false),
                        Some(true),
                    )),
                ]
            }
            ToolDisclosure::Sidecar => {
                vec![McpTool::new(
                    "execute_typescript".to_string(),
                    tool_descriptions::EXECUTE_TYPESCRIPT_SIDECAR.to_string(),
                    schema::<ExecuteWithToolGraph>(),
                )
                .annotate(ToolAnnotations::from_raw(
                    Some("Execute TypeScript".to_string()),
                    Some(false),
                    Some(true),
                    Some(false),
                    Some(true),
                ))]
            }
        };

        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools,
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
        let result = match name {
            "list_functions" => self.handle_list_functions(session_id).await,
            "get_function_details" => {
                self.handle_get_function_details(session_id, arguments)
                    .await
            }
            "execute_bash" => {
                self.handle_execute_bash(session_id, arguments, cancellation_token)
                    .await
            }
            "execute_typescript" => {
                self.handle_execute_typescript(ctx, arguments, cancellation_token)
                    .await
            }
            _ => Err(format!("Unknown tool: {name}")),
        };

        match result {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {error}"
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn get_moim(&self, session_id: &str) -> Option<String> {
        let code_mode = self.get_code_mode(session_id).await.ok()?;

        let disclosure_style_moim = match self.disclosure {
            ToolDisclosure::Catalog => {
                let function_count = code_mode.list_functions().functions.len();
                catalog_disclosure_moim(function_count)
            }
            ToolDisclosure::Filesystem => {
                let available_filepaths: Vec<_> = code_mode
                    .virtual_fs().keys().map(String::from).collect();
                format!("Use execute_bash to search and read the tool signatures and input/output types before calling execute_typescript. The available files are: {}", available_filepaths.join(", "))
            },
            ToolDisclosure::Sidecar => "Prioritize calling tools with the execute_typescript tool, especially when multiple tools can be called in one script.".into(),
        };

        Some(format!(
            indoc::indoc! {r#"
                ALWAYS batch multiple tool operations into ONE execute_typescript call.
                - WRONG: Separate execute_typescript calls for read file, then write file
                - RIGHT: One execute_typescript with an async run() function that reads AND writes AND logs/returns as little information as needed for the next step.

                {}
            "#},
            disclosure_style_moim
        ))
    }
}

fn catalog_disclosure_moim(function_count: usize) -> String {
    if function_count == 0 {
        "No execute_typescript callback functions are currently registered.".to_string()
    } else {
        format!(
            "{function_count} callback functions are available only from inside execute_typescript. Do not call callback function names directly as tools. Use list_functions and get_function_details to inspect signatures before writing one execute_typescript call."
        )
    }
}

pub fn get_tool_disclosure() -> ToolDisclosure {
    let config = crate::config::Config::global();
    let tool_disclosure_str: String = config
        .get_param("CODE_MODE_TOOL_DISCLOSURE")
        .unwrap_or_else(|_| "catalog".to_string());
    serde_json::from_value(serde_json::json!(tool_disclosure_str)).unwrap_or_default()
}

struct CodeModeState {
    code_mode: CodeMode,
    hash: u64,
}

impl CodeModeState {
    fn new(cfgs: Vec<CallbackConfig>) -> Result<Self, String> {
        let hash = Self::hash(&cfgs);

        let code_mode = CodeMode::default()
            .with_callbacks(&cfgs)
            .map_err(|e| format!("failed adding callback configs to CodeMode: {e}"))?;

        Ok(Self { code_mode, hash })
    }

    /// Compute order-independent hash of callback configs
    fn hash(cfgs: &[CallbackConfig]) -> u64 {
        let mut cfg_strings: Vec<_> = cfgs
            .iter()
            .filter_map(|c| serde_json::to_string(c).ok())
            .collect();
        cfg_strings.sort();

        let mut hasher = DefaultHasher::new();
        for s in cfg_strings {
            s.hash(&mut hasher);
        }
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_in_deno_runtime_times_out_on_hung_execution() {
        let result: Result<(), String> = run_in_deno_runtime(
            Duration::from_millis(50),
            CancellationToken::new(),
            CancellationToken::new(),
            std::future::pending,
        )
        .await;

        assert!(result.unwrap_err().contains("timed out"));
    }

    #[tokio::test]
    async fn run_in_deno_runtime_honors_cancellation() {
        let token = CancellationToken::new();
        token.cancel();

        let result: Result<(), String> = run_in_deno_runtime(
            Duration::from_secs(60),
            token,
            CancellationToken::new(),
            std::future::pending,
        )
        .await;

        assert_eq!(result.unwrap_err(), "Execution cancelled");
    }

    #[tokio::test]
    async fn run_in_deno_runtime_cancels_dispatch_token_when_abandoned() {
        // On timeout, an in-flight nested tool call (via the dispatch token)
        // must be told to stop rather than left running in the background.
        let dispatch_token = CancellationToken::new();
        let result: Result<(), String> = run_in_deno_runtime(
            Duration::from_millis(50),
            CancellationToken::new(),
            dispatch_token.clone(),
            std::future::pending,
        )
        .await;
        assert!(result.unwrap_err().contains("timed out"));
        assert!(dispatch_token.is_cancelled());

        // On cancellation, the child dispatch token is likewise cancelled.
        let outer = CancellationToken::new();
        outer.cancel();
        let dispatch_token = outer.child_token();
        let result: Result<(), String> = run_in_deno_runtime(
            Duration::from_secs(60),
            outer,
            dispatch_token.clone(),
            std::future::pending,
        )
        .await;
        assert_eq!(result.unwrap_err(), "Execution cancelled");
        assert!(dispatch_token.is_cancelled());
    }

    #[tokio::test]
    async fn run_in_deno_runtime_drains_task_on_timeout() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let dispatch_token = CancellationToken::new();
        let observed = Arc::new(AtomicBool::new(false));
        let task_token = dispatch_token.clone();
        let task_observed = observed.clone();

        let result: Result<(), String> = run_in_deno_runtime(
            Duration::from_millis(50),
            CancellationToken::new(),
            dispatch_token.clone(),
            move || async move {
                task_token.cancelled().await;
                task_observed.store(true, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;

        assert!(
            result.unwrap_err().contains("timed out"),
            "should report timeout"
        );
        assert!(
            observed.load(Ordering::SeqCst),
            "task should observe dispatch token cancellation before being dropped"
        );
    }

    /// Exercises the real Deno/V8 stack: a script whose event loop never
    /// resolves must time out instead of wedging forever, and a normal
    /// script must run right after, proving pctx's process-wide V8 mutex
    /// was released (i.e. one hung execution no longer blocks other sessions).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_v8_hung_script_times_out_and_frees_the_runtime() {
        let hung = CodeMode::default();
        let hung_result = run_in_deno_runtime(
            Duration::from_secs(2),
            CancellationToken::new(),
            CancellationToken::new(),
            move || async move {
                hung.execute_typescript(
                    "async function run() { await new Promise(() => {}); }",
                    ToolDisclosure::default(),
                    None,
                )
                .await
                .map_err(|e| format!("execution error: {e}"))
            },
        )
        .await;
        assert!(
            hung_result.unwrap_err().contains("timed out"),
            "hung script should time out"
        );

        let normal = CodeMode::default();
        let normal_result = run_in_deno_runtime(
            Duration::from_secs(60),
            CancellationToken::new(),
            CancellationToken::new(),
            move || async move {
                normal
                    .execute_typescript(
                        "async function run() { return 1 + 1; }",
                        ToolDisclosure::default(),
                        None,
                    )
                    .await
                    .map_err(|e| format!("execution error: {e}"))
            },
        )
        .await
        .expect("normal script should run after a prior timeout");
        assert!(
            normal_result.success,
            "normal script should succeed once the V8 mutex is released: {}",
            normal_result.stderr
        );
    }

    #[test]
    fn catalog_moim_mentions_inspection_tools_without_function_names() {
        let moim = catalog_disclosure_moim(3);

        assert!(moim.contains("3 callback functions"));
        assert!(moim.contains("list_functions"));
        assert!(moim.contains("get_function_details"));
        assert!(!moim.contains("extract_relations"));
        assert!(!moim.contains("ask_heimdall"));
    }

    fn self_referential_any_schema() -> Value {
        json!({
            "$ref": "#/$defs/Any",
            "$defs": {
                "Any": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "number"},
                        {
                            "type": "object",
                            "additionalProperties": {"$ref": "#/$defs/Any"}
                        }
                    ]
                }
            }
        })
    }

    #[test]
    fn collect_ref_targets_finds_nested_refs() {
        let schema = self_referential_any_schema();
        let mut refs = HashSet::new();
        collect_ref_targets(&schema["$defs"]["Any"], &mut refs);

        assert_eq!(refs, HashSet::from(["Any".to_string()]));
    }

    #[test]
    fn find_cycle_edges_detects_self_loop() {
        let mut edges = HashMap::new();
        edges.insert("Any".to_string(), HashSet::from(["Any".to_string()]));
        let names = vec!["Any".to_string()];

        let cuts = find_cycle_edges(&names, &edges);

        assert_eq!(cuts, vec![("Any".to_string(), "Any".to_string())]);
    }

    #[test]
    fn find_cycle_edges_detects_longer_cycle_without_flagging_acyclic_refs() {
        let mut edges = HashMap::new();
        edges.insert("A".to_string(), HashSet::from(["B".to_string()]));
        edges.insert("B".to_string(), HashSet::from(["C".to_string()]));
        edges.insert("C".to_string(), HashSet::from(["A".to_string()]));
        edges.insert("D".to_string(), HashSet::from(["A".to_string()]));
        let names = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
        ];

        let cuts = find_cycle_edges(&names, &edges);

        assert_eq!(cuts, vec![("C".to_string(), "A".to_string())]);
    }

    #[test]
    fn neutralize_refs_to_replaces_matching_refs_only() {
        let mut value = json!({
            "anyOf": [
                {"$ref": "#/$defs/Any"},
                {"$ref": "#/$defs/Other"}
            ]
        });

        neutralize_refs_to(&mut value, "Any");

        assert_eq!(value["anyOf"][0], json!({}));
        assert_eq!(value["anyOf"][1], json!({"$ref": "#/$defs/Other"}));
    }

    #[test]
    fn sanitize_schema_for_code_mode_breaks_self_referential_defs() {
        let mut schema = self_referential_any_schema();

        sanitize_schema_for_code_mode(&mut schema);

        let mut refs = HashSet::new();
        collect_ref_targets(&schema["$defs"]["Any"], &mut refs);
        assert!(
            !refs.contains("Any"),
            "cycle should be broken, got: {schema}"
        );
    }

    #[test]
    fn sanitize_schema_for_code_mode_leaves_acyclic_schemas_untouched() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "content": {"$ref": "#/$defs/Content"}
            },
            "$defs": {
                "Content": {"type": "string"}
            }
        });
        let original = schema.clone();

        sanitize_schema_for_code_mode(&mut schema);

        assert_eq!(schema, original);
    }

    #[test]
    fn code_mode_accepts_previously_crashing_self_referential_schema() {
        let mut output_schema = self_referential_any_schema();
        sanitize_schema_for_code_mode(&mut output_schema);

        let cfg = CallbackConfig {
            name: "retain".to_string(),
            namespace: Some("hindsight".to_string()),
            description: Some("Store a memory".to_string()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {"content": {"type": "string"}},
                "required": ["content"]
            })),
            output_schema: Some(output_schema),
        };

        let result = CodeMode::default().with_callback(&cfg);
        assert!(result.is_ok(), "{:?}", result.err());
    }
}
