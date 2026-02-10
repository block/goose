use crate::agents::extension::PlatformExtensionContext;
use crate::agents::extension_manager::get_tool_owner;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use pctx_code_mode::model::{CallbackConfig, ExecuteInput, GetFunctionDetailsInput};
use pctx_code_mode::{CallbackRegistry, CodeMode};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, InitializeResult, JsonObject,
    ListToolsResult, ProtocolVersion, RawContent, Role, ServerCapabilities, Tool as McpTool,
    ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "code_execution";

pub struct CodeExecutionClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
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
                title: Some("Code Mode".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(indoc! {r#"
                BATCH MULTIPLE TOOL CALLS INTO ONE execute CALL.

                This extension exists to reduce round-trips. When a task requires multiple tool calls:
                - WRONG: Multiple execute calls, each with one tool
                - RIGHT: One execute call with a script that calls all needed tools

                IMPORTANT: All tool calls are ASYNC. Use await for each call.

                Workflow:
                    1. Use the list_functions and get_function_details tools to discover tools and signatures
                    2. Write ONE script that calls ALL tools needed for the task, no need to import anything,
                       all the namespaces returned by list_functions and get_function_details will be available
                    3. Chain results: use output from one tool as input to the next
                    4. Only return and console.log data you need, tools could have very large responses.
            "#}.to_string()),
        };

        Ok(Self {
            info,
            context,
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
            let full_name = tool.name.to_string();
            let (namespace, name) = if let Some((server, tool_name)) = full_name.split_once("__") {
                (server.to_string(), tool_name.to_string())
            } else if let Some(owner) = get_tool_owner(&tool) {
                (owner, full_name)
            } else {
                continue;
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

    /// Build a CallbackRegistry with all tool callbacks registered
    fn build_callback_registry(
        &self,
        session_id: &str,
        code_mode: &CodeMode,
    ) -> Result<CallbackRegistry, String> {
        let manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())
            .ok_or("Extension manager not available")?;

        let registry = CallbackRegistry::default();
        for cfg in code_mode.callbacks() {
            let full_name = format!("{}__{}", &cfg.namespace, &cfg.name);
            let callback = create_tool_callback(session_id.to_string(), full_name, manager.clone());
            registry
                .add(&cfg.id(), callback)
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

    /// Handle the execute tool call
    async fn handle_execute(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let args: ExecuteWithToolGraph = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for execute")?;

        let code_mode = self.get_code_mode(session_id).await?;
        let registry = self.build_callback_registry(session_id, &code_mode)?;
        let code = args.input.code.clone();

        // Deno runtime is not Send, so we need to run it in a blocking task
        // with its own tokio runtime
        let output = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to create runtime: {e}"))?;

            rt.block_on(async move {
                code_mode
                    .execute(&code, Some(registry))
                    .await
                    .map_err(|e| format!("Execution error: {e}"))
            })
        })
        .await
        .map_err(|e| format!("Execution task failed: {e}"))??;

        Ok(vec![Content::text(output.markdown())])
    }
}

fn create_tool_callback(
    session_id: String,
    full_name: String,
    manager: Arc<crate::agents::ExtensionManager>,
) -> pctx_code_mode::CallbackFn {
    Arc::new(move |args: Option<Value>| {
        let session_id = session_id.clone();
        let full_name = full_name.clone();
        let manager = manager.clone();
        Box::pin(async move {
            let tool_call = CallToolRequestParams {
                task: None,
                meta: None,
                name: full_name.into(),
                arguments: args.and_then(|v| v.as_object().cloned()),
            };
            match manager
                .dispatch_tool_call(&session_id, tool_call, None, CancellationToken::new())
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

        Ok(ListToolsResult {
            tools: vec![
                McpTool::new(
                    "list_functions".to_string(),
                    indoc! {r#"
                        List all available functions across all namespaces.
                        
                        This will not return function input and output types.
                        After determining which functions are needed use
                        get_function_details to get input and output type 
                        information about specific functions.
                    "#}
                    .to_string(),
                    empty_schema,
                )
                .annotate(ToolAnnotations {
                    title: Some("List functions".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                }),
                McpTool::new(
                    "get_function_details".to_string(),
                    indoc! {r#"
                        Get detailed type information for specific functions.

                        Provide a list of function identifiers in the format "Namespace.functionName"
                        (e.g., "Developer.shell", "Github.createIssue").

                        Returns full TypeScript interface definitions with parameter types,
                        return types, and descriptions for the requested functions.
                    "#}
                    .to_string(),
                    schema::<GetFunctionDetailsInput>(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Get function details".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                }),
                McpTool::new(
                    "execute".to_string(),
                    indoc! {r#"
                        Execute TypeScript code that calls available functions.

                        SYNTAX - TypeScript with async run() function:
                        ```typescript
                        async function run() {
                            // Access functions via Namespace.functionName({ params }) — always camelCase
                            const files = await Developer.shell({ command: "ls -la" });
                            const readme = await Developer.textEditor({ path: "./README.md", command: "view" });
                            return { files, readme };
                        }
                        ```

                        TOOL_GRAPH: Always provide tool_graph to describe the execution flow for the UI.
                        Each node has: tool (Namespace.functionName), description (what it does), depends_on (indices of dependencies).
                        Example for chained operations:
                        [
                          {"tool": "Developer.shell", "description": "list files", "depends_on": []},
                          {"tool": "Developer.textEditor", "description": "read README.md", "depends_on": []},
                          {"tool": "Developer.textEditor", "description": "write output.txt", "depends_on": [0, 1]}
                        ]

                        KEY RULES:
                        - Code MUST define an async function named `run()`
                        - All function calls are async - use `await`
                        - Function names are always camelCase (e.g., Developer.textEditor, Github.listIssues, Github.createIssue)
                        - Return value from `run()` is the result, all `console.log()` output will be returned as well.
                        - Only functions from `list_functions()` and `console` methods are available — no `fetch()`, `fs`, or other Node/Deno APIs
                        - Variables don't persist between `execute()` calls - return or log anything you need later
                        - Code runs in an isolated sandbox with restricted network access

                        HANDLING RETURN VALUES:
                        - If a function returns `any`, do NOT assume its shape - log it first: `console.log(JSON.stringify(result))`
                        - Many functions return wrapper objects, not raw arrays - check the response structure before calling .filter(), .map(), etc.
                        - Always inspect unfamiliar return values with console.log() before processing them

                        TOKEN USAGE WARNING: This tool could return LARGE responses if your code returns big objects.
                        To minimize tokens:
                        - Filter/map/reduce data IN YOUR CODE before returning
                        - Only return specific fields you need (e.g., return {id: result.id, count: items.length})
                        - Use console.log() for intermediate results instead of returning everything
                        - Avoid returning full API responses - extract just what you need

                        BEFORE CALLING: Use list_functions or get_function_details to check available functions and their parameters.
                    "#}
                    .to_string(),
                    schema::<ExecuteWithToolGraph>(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Execute TypeScript".to_string()),
                    read_only_hint: Some(false),
                    destructive_hint: Some(true),
                    idempotent_hint: Some(false),
                    open_world_hint: Some(true),
                }),
            ],
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
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let result = match name {
            "list_functions" => self.handle_list_functions(session_id).await,
            "get_function_details" => {
                self.handle_get_function_details(session_id, arguments)
                    .await
            }
            "execute" => self.handle_execute(session_id, arguments).await,
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
        let available: Vec<_> = code_mode
            .list_functions()
            .functions
            .iter()
            .map(|f| format!("{}.{}", &f.namespace, &f.name))
            .collect();

        Some(format!(
            indoc::indoc! {r#"
                ALWAYS batch multiple tool operations into ONE execute call.
                - WRONG: Separate execute calls for read file, then write file
                - RIGHT: One execute with an async run() function that reads AND writes

                Available namespaces: {}

                Use the list_functions & get_function_details tools to see tool signatures and input/output types before calling unfamiliar tools.
            "#},
            available.join(", ")
        ))
    }
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
