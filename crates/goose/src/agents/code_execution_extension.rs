use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait, McpMeta};
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use pctx_code_mode::model::{CallbackConfig, ExecuteInput, GetFunctionDetailsInput};
use pctx_code_mode::{CallbackRegistry, CodeMode};
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, Implementation, InitializeResult, JsonObject,
    ListToolsResult, ProtocolVersion, RawContent, ServerCapabilities, Tool as McpTool,
    ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "code_execution";

pub struct CodeExecutionClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
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
                title: Some("Code Execution".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(indoc! {r#"
                BATCH MULTIPLE TOOL CALLS INTO ONE execute_code CALL.

                This extension exists to reduce round-trips. When a task requires multiple tool calls:
                - WRONG: Multiple execute_code calls, each with one tool
                - RIGHT: One execute_code call with a script that calls all needed tools

                IMPORTANT: All tool calls are ASYNC. Use await for each call.

                Workflow:
                    1. Use the list_functions and get_function_details tools to discover tools and signatures
                    2. Write ONE script that imports and calls ALL tools needed for the task
                    3. Chain results: use output from one tool as input to the next
                    4. Only return and console.log data you need, tools could have very large responses.
            "#}.to_string()),
        };

        Ok(Self { info, context })
    }

    async fn load_callbacks_configs(&self) -> Option<Vec<CallbackConfig>> {
        let manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())?;

        // generate callback configurations
        let mut callback_cfgs = vec![];
        if let Ok(tools) = manager.get_prefixed_tools_excluding(EXTENSION_NAME).await {
            for tool in tools {
                let (server_name, tool_name) = tool.name.as_ref().split_once("__")?;
                callback_cfgs.push(CallbackConfig {
                    name: tool_name.into(),
                    namespace: server_name.into(),
                    description: tool.description.map(String::from),
                    input_schema: Some(json!(tool.input_schema)),
                    output_schema: tool.output_schema.map(|s| json!(s)),
                })
            }
        }

        Some(callback_cfgs)
    }

    /// Build a CodeMode instance with all available callbacks configured
    async fn build_code_mode(&self) -> Result<CodeMode, String> {
        let callback_cfgs = self
            .load_callbacks_configs()
            .await
            .ok_or("Failed to load callback configurations")?;

        let mut code_mode = CodeMode::default();
        for cfg in &callback_cfgs {
            code_mode
                .add_callback(cfg)
                .map_err(|e| format!("Failed to add callback: {e}"))?;
        }

        Ok(code_mode)
    }

    /// Build a CallbackRegistry with all tool callbacks registered
    fn build_callback_registry(
        &self,
        session_id: &str,
        callback_cfgs: &[CallbackConfig],
    ) -> Result<CallbackRegistry, String> {
        let manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())
            .ok_or("Extension manager not available")?;

        let registry = CallbackRegistry::default();
        for cfg in callback_cfgs {
            let full_name = format!("{}__{}", cfg.namespace, cfg.name);
            let callback = create_tool_callback(session_id.to_string(), full_name, manager.clone());
            registry
                .add(&cfg.id(), callback)
                .map_err(|e| format!("Failed to register callback: {e}"))?;
        }

        Ok(registry)
    }

    /// Handle the list_functions tool call
    async fn handle_list_functions(&self) -> Result<Vec<Content>, String> {
        let code_mode = self.build_code_mode().await?;
        let output = code_mode.list_functions();

        Ok(vec![Content::text(output.code)])
    }

    /// Handle the get_function_details tool call
    async fn handle_get_function_details(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let input: GetFunctionDetailsInput = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for get_function_details")?;

        let code_mode = self.build_code_mode().await?;
        let output = code_mode.get_function_details(input);

        Ok(vec![Content::text(output.code)])
    }

    /// Handle the execute tool call
    async fn handle_execute(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let input: ExecuteInput = arguments
            .map(|args| serde_json::from_value(Value::Object(args)))
            .transpose()
            .map_err(|e| format!("Failed to parse arguments: {e}"))?
            .ok_or("Missing arguments for execute")?;

        let callback_cfgs = self
            .load_callbacks_configs()
            .await
            .ok_or("Failed to load callback configurations")?;

        let mut code_mode = CodeMode::default();
        for cfg in &callback_cfgs {
            code_mode
                .add_callback(cfg)
                .map_err(|e| format!("Failed to add callback: {e}"))?;
        }

        let registry = self.build_callback_registry(session_id, &callback_cfgs)?;
        let code = input.code.clone();

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

        if output.success {
            let result_text = if let Some(output_value) = output.output {
                serde_json::to_string_pretty(&output_value).unwrap_or_else(|_| output.stdout)
            } else {
                output.stdout
            };
            Ok(vec![Content::text(result_text)])
        } else {
            Err(format!("Execution failed:\n{}", output.stderr))
        }
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
            let tool_call = CallToolRequestParam {
                task: None,
                name: full_name.into(),
                arguments: args.and_then(|v| v.as_object().cloned()),
            };
            match manager
                .dispatch_tool_call(&session_id, tool_call, CancellationToken::new())
                .await
            {
                Ok(dispatch_result) => match dispatch_result.result.await {
                    Ok(result) => {
                        if let Some(sc) = &result.structured_content {
                            Ok(serde_json::to_value(sc).unwrap_or(Value::Null))
                        } else {
                            let text: String = result
                                .content
                                .iter()
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
                        (e.g., "Developer.shell", "Github.create_issue").

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
                            // Access functions via Namespace.functionName({ params })
                            const files = await Developer.shell({ command: "ls -la" });
                            const readme = await Developer.text_editor({ path: "./README.md", command: "view" });
                            return { files, readme };
                        }
                        ```

                        KEY RULES:
                        - Code MUST define an async function named `run()`
                        - All function calls are async - use `await`
                        - Access functions as Namespace.functionName() (e.g., Developer.shell, Github.create_issue)
                        - Return value from `run()` is the result, all `console.log()` output will be returned as well.
                        - Only functions from `list_functions()` are available - no `fetch()`, fs, or other Node/Deno APIs
                        - Variables don't persist between `execute()` calls - return or log anything you need later
                        - Add console.log() statements between API calls to track progress if errors occur
                        - Code runs in an isolated Deno sandbox with restricted network access

                        TOKEN USAGE WARNING: This tool could return LARGE responses if your code returns big objects.
                        To minimize tokens:
                        - Filter/map/reduce data IN YOUR CODE before returning
                        - Only return specific fields you need (e.g., return {id: result.id, count: items.length})
                        - Use console.log() for intermediate results instead of returning everything
                        - Avoid returning full API responses - extract just what you need

                        BEFORE CALLING: Use list_functions or get_function_details to check available functions and their parameters.
                    "#}
                    .to_string(),
                    schema::<ExecuteInput>(),
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
        name: &str,
        arguments: Option<JsonObject>,
        meta: McpMeta,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let result = match name {
            "list_functions" => self.handle_list_functions().await,
            "get_function_details" => self.handle_get_function_details(arguments).await,
            "execute" => self.handle_execute(&meta.session_id, arguments).await,
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

    async fn get_moim(&self, _session_id: &str) -> Option<String> {
        let code_mode = self.build_code_mode().await.ok()?;
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
