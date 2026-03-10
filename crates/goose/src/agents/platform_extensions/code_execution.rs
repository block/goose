use crate::agents::extension::PlatformExtensionContext;
use crate::agents::extension_manager::get_tool_owner;
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
use std::collections::HashMap;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Thread-safe store for non-text content captured during code execution.
/// Maps token strings (e.g. "cref_1") to the original rich Content items.
type ContentStore = Arc<std::sync::Mutex<HashMap<String, Vec<Content>>>>;

/// Global counter for generating unique content reference tokens.
static CONTENT_REF_COUNTER: AtomicU64 = AtomicU64::new(0);

pub static EXTENSION_NAME: &str = "code_execution";

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
            let (name, namespace) = if let Some((prefix, tool_name)) = tool.name.split_once("__") {
                (tool_name.to_string(), Some(prefix.to_string()))
            } else if let Some(owner) = get_tool_owner(&tool) {
                (tool.name.to_string(), Some(owner))
            } else {
                (tool.name.to_string(), None)
            };

            cfgs.push(CallbackConfig {
                name,
                namespace,
                description: tool.description.as_ref().map(|d| d.to_string()),
                input_schema: Some(json!(tool.input_schema)),
                output_schema: tool.output_schema.as_ref().map(|s| json!(s)),
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

    /// Build a PctxRegistry with all tool callbacks registered.
    /// Returns the registry and a ContentStore that will accumulate non-text content
    /// produced by tool callbacks during execution.
    fn build_callback_registry(
        &self,
        session_id: &str,
        code_mode: &CodeMode,
    ) -> Result<(PctxRegistry, ContentStore), String> {
        let manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())
            .ok_or("Extension manager not available")?;

        let content_store: ContentStore = Arc::new(std::sync::Mutex::new(HashMap::new()));
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
                session_id.to_string(),
                full_name,
                manager.clone(),
                content_store.clone(),
            );
            registry
                .add_callback(&cfg.id(), callback)
                .map_err(|e| format!("Failed to register callback: {e}"))?;
        }

        Ok((registry, content_store))
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
    ) -> Result<Vec<Content>, String> {
        let input: ExecuteBashInput = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for execute_bash")?;
        let command = input.command;
        let code_mode = self.get_code_mode(session_id).await?;

        // Deno runtime is not Send, so we need to run it in a blocking task
        // with its own tokio runtime
        let output = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to create runtime: {e}"))?;

            rt.block_on(async move {
                code_mode
                    .execute_bash(&command)
                    .await
                    .map_err(|e| format!("Typescript execution error: {e}"))
            })
        })
        .await
        .map_err(|e| format!("Typescript execution task failed: {e}"))??;

        Ok(vec![Content::text(output.markdown())])
    }

    /// Handle the execute typescript tool call
    async fn handle_execute_typescript(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let args: ExecuteWithToolGraph = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for execute_typescript")?;

        let code_mode = self.get_code_mode(session_id).await?;
        let (registry, content_store) =
            self.build_callback_registry(session_id, &code_mode)?;
        let code = args.input.code.clone();
        let disclosure = self.disclosure;

        // Deno runtime is not Send, so we need to run it in a blocking task
        // with its own tokio runtime
        let output = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to create runtime: {e}"))?;

            rt.block_on(async move {
                code_mode
                    .execute_typescript(&code, disclosure, Some(registry))
                    .await
                    .map_err(|e| format!("Typescript execution error: {e}"))
            })
        })
        .await
        .map_err(|e| format!("Typescript execution task failed: {e}"))??;

        // Collect any rich content referenced by tokens in the output
        let mut rich_contents = Vec::new();
        if let Some(ref val) = output.output {
            let store = content_store.lock().unwrap();
            collect_rich_content(val, &store, &mut rich_contents);
        }

        // If the entire return value was just a content ref, return only
        // the resolved rich content. Otherwise return the text output
        // (with refs intact) plus the rich content alongside it.
        // Only treat as a pure content ref if the object is exactly the shape
        // we created (just _goose_content_ref + text_result, nothing extra).
        let is_pure_ref = output
            .output
            .as_ref()
            .and_then(|v| v.as_object())
            .is_some_and(|m| {
                m.contains_key("_goose_content_ref")
                    && m.len() <= 2
                    && m.keys()
                        .all(|k| k == "_goose_content_ref" || k == "text_result")
            });

        if is_pure_ref && !rich_contents.is_empty() {
            // Always include a text fallback so providers that only serialize
            // text content (OpenAI, Codex, Anthropic) still produce a tool result
            // for the model. Preserve the original text_result if the callback
            // returned one, so the model still sees meaningful textual data.
            let text_fallback = output
                .output
                .as_ref()
                .and_then(|v| v.get("text_result"))
                .and_then(|v| match v {
                    Value::String(s) if !s.is_empty() => Some(s.clone()),
                    Value::Null | Value::String(_) => None,
                    other => Some(other.to_string()),
                })
                .unwrap_or_else(|| "Tool returned rich content.".to_string());
            let mut contents = vec![Content::text(&text_fallback)];
            contents.extend(rich_contents);
            Ok(contents)
        } else {
            let return_val = serde_json::to_string_pretty(&output.output)
                .unwrap_or_else(|_| json!(&output.output).to_string());
            let mut contents = vec![Content::text(format_output(
                output.success,
                &return_val,
                &output.stderr,
            ))];
            contents.extend(rich_contents);
            Ok(contents)
        }
    }
}

fn create_tool_callback(
    session_id: String,
    full_name: String,
    manager: Arc<crate::agents::ExtensionManager>,
    content_store: ContentStore,
) -> CallbackFn {
    Arc::new(move |args: Option<Value>| {
        let session_id = session_id.clone();
        let full_name = full_name.clone();
        let manager = manager.clone();
        let content_store = content_store.clone();
        Box::pin(async move {
            let tool_call = {
                let mut params = CallToolRequestParams::new(full_name);
                if let Some(args) = args.and_then(|v| v.as_object().cloned()) {
                    params = params.with_arguments(args);
                }
                params
            };
            let ctx = crate::agents::ToolCallContext::new(session_id, None, None);
            match manager
                .dispatch_tool_call(&ctx, tool_call, CancellationToken::new())
                .await
            {
                Ok(dispatch_result) => match dispatch_result.result.await {
                    Ok(result) => {
                        if let Some(sc) = &result.structured_content {
                            Ok(serde_json::to_value(sc).unwrap_or(Value::Null))
                        } else {
                            // Separate text content from non-text (resources, images, etc.)
                            let mut text_parts = Vec::new();
                            let mut rich_contents = Vec::new();

                            for content in &result.content {
                                match &content.raw {
                                    RawContent::Text(t) => {
                                        // Only include text meant for assistant
                                        if content.audience().is_none_or(|audiences| {
                                            audiences.is_empty()
                                                || audiences.contains(&Role::Assistant)
                                        }) {
                                            text_parts.push(t.text.clone());
                                        }
                                    }
                                    _ => {
                                        // Non-text content (resources, images, blobs) — store for later
                                        rich_contents.push(content.clone());
                                    }
                                }
                            }

                            let text = text_parts.join("\n");
                            let text_value: Value =
                                serde_json::from_str(&text).unwrap_or(Value::String(text));

                            // If there's non-text content, store it and return a token reference.
                            // When text_value is an object, inject the ref key into it so that
                            // the original shape is preserved (e.g. r.items still works in JS).
                            if !rich_contents.is_empty() {
                                let token = format!(
                                    "cref_{}",
                                    CONTENT_REF_COUNTER.fetch_add(1, Ordering::Relaxed)
                                );
                                content_store
                                    .lock()
                                    .unwrap()
                                    .insert(token.clone(), rich_contents);
                                match text_value {
                                    Value::Object(mut map) => {
                                        map.insert(
                                            "_goose_content_ref".to_string(),
                                            Value::String(token),
                                        );
                                        Ok(Value::Object(map))
                                    }
                                    _ => Ok(json!({
                                        "_goose_content_ref": token,
                                        "text_result": text_value,
                                    })),
                                }
                            } else {
                                Ok(text_value)
                            }
                        }
                    }
                    Err(e) => Err(format!("Tool error: {}", e.message)),
                },
                Err(e) => Err(format!("Dispatch error: {e}")),
            }
        }) as Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>
    })
}

fn format_output(success: bool, return_val: &str, stderr: &str) -> String {
    if success {
        format!("Code Executed Successfully: true\n\n# Return Value\n```json\n{return_val}\n```\n")
    } else {
        format!(
            "Code Executed Successfully: false\n\n# Return Value\n```json\n{return_val}\n```\n\n# STDERR\n{stderr}\n"
        )
    }
}

/// Recursively walk a JSON value, collecting rich content for any
/// `_goose_content_ref` tokens found.
fn collect_rich_content(
    val: &Value,
    store: &HashMap<String, Vec<Content>>,
    rich: &mut Vec<Content>,
) {
    match val {
        Value::Object(map) => {
            if let Some(Value::String(token)) = map.get("_goose_content_ref") {
                if let Some(stored) = store.get(token) {
                    rich.extend(stored.iter().cloned());
                }
            }
            for v in map.values() {
                collect_rich_content(v, store, rich);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_rich_content(v, store, rich);
            }
        }
        _ => {}
    }
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
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let session_id = &ctx.session_id;
        let result = match name {
            "list_functions" => self.handle_list_functions(session_id).await,
            "get_function_details" => {
                self.handle_get_function_details(session_id, arguments)
                    .await
            }
            "execute_bash" => self.handle_execute_bash(session_id, arguments).await,
            "execute_typescript" => self.handle_execute_typescript(session_id, arguments).await,
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
                let functions = code_mode.list_functions().functions;
                let sandbox_only: Vec<_> = functions
                    .iter()
                    .filter(|f| !crate::agents::extension_manager::is_first_class_extension(&f.namespace))
                    .map(|f| format!("{}.{}", &f.namespace, &f.name))
                    .collect();
                let mut msg = String::new();
                if !sandbox_only.is_empty() {
                    msg.push_str(&format!(
                        "Additional functions available ONLY via execute_typescript (do NOT call these as direct tool calls): {}",
                        sandbox_only.join(", ")
                    ));
                }
                msg.push_str("\n\n                Use the list_functions & get_function_details tools to see tool signatures and input/output types before calling execute_typescript.");
                msg
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
