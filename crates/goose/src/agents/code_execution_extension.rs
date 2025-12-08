use crate::agents::extension::PlatformExtensionContext;
use crate::agents::extension_manager::get_parameter_names;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use boa_engine::builtins::promise::PromiseState;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction, Source};
use indoc::indoc;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, GetPromptResult, Implementation,
    InitializeResult, JsonObject, ListPromptsResult, ListResourcesResult, ListToolsResult,
    ProtocolVersion, RawContent, ReadResourceResult, ServerCapabilities, ServerNotification, Tool,
    ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "code_execution";

type ToolCallRequest = (
    String,
    String,
    tokio::sync::oneshot::Sender<Result<String, String>>,
);

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ExecuteCodeParams {
    code: String,
}

fn run_js_code(
    code: &str,
    preamble: Option<String>,
    call_tx: tokio::sync::mpsc::UnboundedSender<ToolCallRequest>,
) -> Result<String, String> {
    let mut ctx = Context::default();

    thread_local! {
        static CALL_TX: std::cell::RefCell<Option<tokio::sync::mpsc::UnboundedSender<ToolCallRequest>>> =
            const { std::cell::RefCell::new(None) };
    }

    CALL_TX.with(|tx| *tx.borrow_mut() = Some(call_tx));

    let call_tool_fn = NativeFunction::from_fn_ptr(|_this, args, ctx| {
        let tool_name = args
            .first()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsNativeError::typ().with_message("First argument must be tool name"))?
            .to_std_string_escaped();

        let args_json = args
            .get(1)
            .cloned()
            .unwrap_or(JsValue::undefined())
            .to_json(ctx)
            .map_err(|e| JsNativeError::error().with_message(e.to_string()))?
            .unwrap_or(Value::Null);

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        CALL_TX
            .with(|tx| {
                tx.borrow().as_ref().and_then(|sender| {
                    let args_str = serde_json::to_string(&args_json).unwrap_or_default();
                    sender.send((tool_name, args_str, response_tx)).ok()
                })
            })
            .ok_or_else(|| {
                JsNativeError::error().with_message("Tool call channel not available")
            })?;

        response_rx
            .blocking_recv()
            .map_err(|e| e.to_string())
            .and_then(|r| r)
            .map(|result| JsValue::from(js_string!(result.as_str())))
            .map_err(|e| JsNativeError::error().with_message(e).into())
    });

    ctx.register_global_builtin_callable(js_string!("__call_tool__"), 2, call_tool_fn)
        .expect("Failed to register __call_tool__");

    if let Some(ref p) = preamble {
        if let Err(e) = ctx.eval(Source::from_bytes(p)) {
            return Err(format!("Failed to load preamble: {e}"));
        }
    }

    match ctx.eval(Source::from_bytes(code)) {
        Ok(result) => {
            let _ = ctx.run_jobs();
            if let Some(obj) = result.as_object() {
                if let Ok(promise) = JsPromise::from_object(obj.clone()) {
                    return match promise.state() {
                        PromiseState::Fulfilled(value) => Ok(value.display().to_string()),
                        PromiseState::Rejected(err) => {
                            Err(format!("Promise rejected: {}", err.display()))
                        }
                        PromiseState::Pending => {
                            Err("Promise still pending after execution".to_string())
                        }
                    };
                }
            }
            Ok(result.display().to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub struct CodeExecutionClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
}

impl CodeExecutionClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
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
            instructions: Some(
                indoc! {r#"
                JavaScript code execution environment with live MCP tool bindings.
                
                IMPORTANT: Prefer execute_code over making individual tool calls when:
                - You need to call multiple tools and combine their results
                - You want to process tool outputs with JavaScript logic
                - You need conditional tool execution based on previous results
                
                This reduces round-trips and allows complex workflows in a single call.
                See the execute_code tool for detailed usage instructions.
            "#}
                .to_string(),
            ),
        };

        Ok(Self { info, context })
    }

    async fn build_preamble(&self) -> Option<String> {
        let manager = self.context.extension_manager.as_ref()?.upgrade()?;
        let tools = manager
            .get_prefixed_tools_excluding(EXTENSION_NAME)
            .await
            .ok()
            .filter(|t| !t.is_empty())?;

        Some(Self::generate_preamble_js(&tools))
    }

    fn generate_preamble_js(tools: &[Tool]) -> String {
        let mut by_ext: HashMap<&str, Vec<&Tool>> = HashMap::new();
        for tool in tools {
            if let Some((prefix, _)) = tool.name.as_ref().split_once("__") {
                by_ext.entry(prefix).or_default().push(tool);
            }
        }

        let mut js = String::from(
            "// Auto-generated tool bindings for MCP extensions\n\
             // Each extension is a namespace object containing its tools as methods\n\
             // Tool calls block until completion and return the result directly\n\n",
        );

        for (ext, ext_tools) in &by_ext {
            writeln!(js, "const {ext} = {{").unwrap();
            for (i, tool) in ext_tools.iter().enumerate() {
                let name = tool.name.as_ref().split_once("__").unwrap().1;
                let params = get_parameter_names(tool);
                let trailing = if i < ext_tools.len() - 1 { "," } else { "" };

                js.push_str("  /**\n");
                if let Some(desc) = tool.description.as_ref() {
                    for line in desc.as_ref().lines() {
                        writeln!(js, "   * {line}").unwrap();
                    }
                }
                for param in &params {
                    let (ty, desc) = Self::get_param_info(&tool.input_schema, param);
                    writeln!(js, "   * @param {{{ty}}} {param}{desc}").unwrap();
                }
                js.push_str("   * @returns {string} Tool result\n   */\n");

                let args_obj = if params.is_empty() {
                    "{}".to_string()
                } else {
                    format!(
                        "{{ {} }}",
                        params
                            .iter()
                            .map(|p| format!("{p}: {p}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                writeln!(
                    js,
                    "  {name}: function({}) {{ return __call_tool__(\"{}\", {args_obj}); }}{trailing}",
                    params.join(", "),
                    tool.name.as_ref()
                ).unwrap();
            }
            js.push_str("};\n\n");
        }

        writeln!(
            js,
            "const __extensions__ = [{}];",
            by_ext
                .keys()
                .map(|k| format!("\"{k}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
        js
    }

    fn get_param_info(schema: &JsonObject, param: &str) -> (String, String) {
        let prop = schema
            .get("properties")
            .and_then(|p| p.get(param))
            .and_then(|v| v.as_object());

        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some(param)));

        let base = prop.and_then(|p| p.get("type")?.as_str()).unwrap_or("*");
        let ty = if required {
            base.to_string()
        } else {
            format!("{base}=")
        };
        let desc = prop
            .and_then(|p| p.get("description")?.as_str())
            .map(|d| format!(" - {d}"))
            .unwrap_or_default();

        (ty, desc)
    }

    async fn handle_execute_code(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let code = arguments
            .as_ref()
            .and_then(|a| a.get("code"))
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: code")?
            .to_string();

        let preamble = self.build_preamble().await;

        // Boa's Context is !Send, so we use channels to bridge the blocking JS thread
        // and the async runtime for tool calls.
        let (call_tx, call_rx) = tokio::sync::mpsc::unbounded_channel();
        let tool_handler = tokio::spawn(Self::run_tool_handler(
            call_rx,
            self.context.extension_manager.clone(),
        ));

        let js_result = tokio::task::spawn_blocking(move || run_js_code(&code, preamble, call_tx))
            .await
            .map_err(|e| format!("JS execution task failed: {e}"))?;

        tool_handler.abort();
        js_result.map(|r| vec![Content::text(format!("Result: {r}"))])
    }

    async fn run_tool_handler(
        mut call_rx: tokio::sync::mpsc::UnboundedReceiver<ToolCallRequest>,
        extension_manager: Option<std::sync::Weak<crate::agents::ExtensionManager>>,
    ) {
        while let Some((tool_name, arguments, response_tx)) = call_rx.recv().await {
            let result = match extension_manager.as_ref().and_then(|w| w.upgrade()) {
                Some(manager) => {
                    let tool_call = CallToolRequestParam {
                        name: tool_name.into(),
                        arguments: serde_json::from_str(&arguments).ok(),
                    };
                    match manager
                        .dispatch_tool_call(tool_call, CancellationToken::new())
                        .await
                    {
                        Ok(dispatch_result) => match dispatch_result.result.await {
                            Ok(content) => Ok(content
                                .iter()
                                .filter_map(|c| match &c.raw {
                                    RawContent::Text(t) => Some(t.text.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n")),
                            Err(e) => Err(format!("Tool execution error: {}", e.message)),
                        },
                        Err(e) => Err(format!("Tool dispatch error: {e}")),
                    }
                }
                None => Err("Extension manager not available".to_string()),
            };
            let _ = response_tx.send(result);
        }
    }

    fn get_tools() -> Vec<Tool> {
        let execute_schema = serde_json::to_value(schema_for!(ExecuteCodeParams))
            .expect("schema")
            .as_object()
            .expect("object")
            .clone();

        vec![Tool::new(
            "execute_code".to_string(),
            indoc! {r#"
                Execute JavaScript code with live MCP tool bindings.

                ## Environment
                Sandboxed JavaScript (Boa engine) with synchronous access to all enabled MCP tools.
                Tool calls block until completion, allowing chaining and result passing.

                ## Tool API
                Tools are organized by extension namespace. Each extension becomes a JavaScript
                object with methods for its tools. Parameters are positional, matching the tool schema order.

                Example namespaces (when extensions are enabled):
                  developer.shell(command)              // run shell commands
                  developer.text_editor(path, command, ...) // file operations
                  developer.analyze(path, ...)          // code analysis

                ## Return Value
                The last expression becomes the result. Do NOT use top-level return statements.

                ## Examples

                Simple expression:
                  code: "2 + 2"
                  result: "4"

                Read and process a file:
                  code: `
                    const content = developer.text_editor("/path/to/file.json", "view");
                    JSON.parse(content)
                  `

                Chain multiple tool calls:
                  code: `
                    const files = developer.shell("ls -la");
                    const readme = developer.text_editor("README.md", "view");
                    const analysis = developer.analyze("src/");
                    { files, readme, analysis }
                  `

                Conditional execution:
                  code: `
                    const result = developer.shell("git status --porcelain");
                    if (result.trim()) {
                      developer.shell("git add -A && git commit -m 'auto-commit'");
                    } else {
                      "No changes to commit"
                    }
                  `
                "#}
            .to_string(),
            execute_schema,
        )
        .annotate(ToolAnnotations {
            title: Some("Execute JavaScript code".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(true),
        })]
    }
}

#[async_trait]
impl McpClientTrait for CodeExecutionClient {
    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn read_resource(
        &self,
        _uri: &str,
        _cancellation_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "execute_code" => self.handle_execute_code(arguments).await,
            _ => Err(format!("Unknown tool: {name}")),
        };

        match content {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {error}"
            ))])),
        }
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Value,
        _cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        mpsc::channel(1).1
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn get_moim(&self) -> Option<String> {
        let preamble = self.build_preamble().await?;
        Some(format!(
            "The execute_code tool has the following JavaScript API available:\n\n```javascript\n{}\n```",
            preamble
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_preamble_generation() {
        let tools = vec![
            Tool::new(
                "developer__shell".to_string(),
                "Execute shell commands".to_string(),
                Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "The command to run" }
                        },
                        "required": ["command"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
            Tool::new(
                "developer__text_editor".to_string(),
                "Edit text files".to_string(),
                Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "File path" },
                            "content": { "type": "string" }
                        },
                        "required": ["path"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
        ];

        let js = CodeExecutionClient::generate_preamble_js(&tools);

        assert!(js.contains("const developer = {"));
        assert!(js.contains("shell: function(command)"));
        assert!(js.contains("@param {string} command - The command to run"));
        assert!(js.contains("@param {string} path - File path"));
        assert!(js.contains("@param {string=} content")); // optional param
        assert!(js.contains("__call_tool__"));
        assert!(js.contains("__extensions__"));
        assert!(js.contains("return __call_tool__"));
    }

    #[tokio::test]
    async fn test_call_execute_code() {
        use rmcp::model::RawContent;

        let context = PlatformExtensionContext {
            session_id: None,
            extension_manager: None,
            tool_route_manager: None,
        };
        let client = CodeExecutionClient::new(context).unwrap();

        let mut args = JsonObject::new();
        args.insert("code".to_string(), Value::String("2 + 2".to_string()));

        let result = client
            .call_tool("execute_code", Some(args), CancellationToken::new())
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let content = &result.content[0];
        if let RawContent::Text(text) = &content.raw {
            assert_eq!(text.text, "Result: 4");
        } else {
            panic!("Expected text content");
        }
    }
}
