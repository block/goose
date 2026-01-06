use crate::agents::extension::PlatformExtensionContext;
use crate::agents::extension_manager::get_parameter_names;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use boa_engine::builtins::promise::PromiseState;
use boa_engine::module::{MapModuleLoader, Module, SyntheticModuleInitializer};
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsString, JsValue, NativeFunction, Source};
use indoc::indoc;
use regex::Regex;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, GetPromptResult, Implementation,
    InitializeResult, JsonObject, ListPromptsResult, ListResourcesResult, ListToolsResult,
    ProtocolVersion, RawContent, ReadResourceResult, ServerCapabilities, ServerNotification,
    Tool as McpTool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "code_execution";

type ToolCallRequest = (
    String,
    String,
    tokio::sync::oneshot::Sender<Result<String, String>>,
);

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
struct ExecuteCodeParams {
    /// JavaScript code with ES6 imports for MCP tools.
    code: String,
    /// DAG of tool calls showing execution flow. Each node represents a tool call.
    /// Use depends_on to show data flow (e.g., node 1 uses output from node 0).
    #[serde(default)]
    tool_graph: Vec<ToolGraphNode>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ReadModuleParams {
    /// Module path format:
    /// - For entire server: "server_name"
    /// - For specific tool: "server_name/tool_name"
    module_path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SearchModulesParams {
    /// Search terms to find servers/tools (case-insensitive). Can be a single string or array of strings.
    terms: SearchTerms,
    /// If true, treat search terms as regex patterns
    #[serde(default)]
    regex: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
enum SearchTerms {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Default, Deserialize)]
struct InputSchema {
    #[serde(default)]
    properties: BTreeMap<String, Value>,
    #[serde(default)]
    required: Vec<String>,
}

fn quote_join(vals: &[&str]) -> String {
    format!("\"{}\"", vals.join("\" | \""))
}

fn infer_type(schema: &Value) -> Option<String> {
    if schema.get("properties").is_some() {
        Some("object".to_string())
    } else if schema.get("items").is_some() {
        Some("array".to_string())
    } else {
        None
    }
}

fn extract_type_from_schema(schema: &Value) -> Option<String> {
    // enum array (github-mcp style)
    if let Some(arr) = schema.get("enum").and_then(|e| e.as_array()) {
        let vals: Vec<_> = arr.iter().filter_map(|v| v.as_str()).collect();
        if !vals.is_empty() {
            return Some(quote_join(&vals));
        }
    }

    // oneOf with const (schemars enums)
    if let Some(arr) = schema.get("oneOf").and_then(|o| o.as_array()) {
        let vals: Vec<_> = arr
            .iter()
            .filter_map(|v| v.get("const")?.as_str())
            .collect();
        if !vals.is_empty() {
            return Some(quote_join(&vals));
        }
    }

    // anyOf (Option<T> or unions)
    if let Some(arr) = schema.get("anyOf").and_then(|o| o.as_array()) {
        let non_null: Vec<_> = arr
            .iter()
            .filter(|v| v.get("type").and_then(|t| t.as_str()) != Some("null"))
            .collect();
        if non_null.len() == 1 {
            return extract_type_from_schema(non_null[0]).or_else(|| infer_type(non_null[0]));
        }
        if non_null.len() > 1 {
            let types: Vec<_> = non_null
                .iter()
                .filter_map(|v| extract_type_from_schema(v).or_else(|| infer_type(v)))
                .collect();
            if !types.is_empty() {
                return Some(types.join(" | "));
            }
        }
    }

    // type field (string or array)
    match schema.get("type") {
        Some(Value::String(s)) if s == "array" => {
            let item_type = schema
                .get("items")
                .and_then(extract_type_from_schema)
                .unwrap_or_else(|| "any".to_string());
            Some(if item_type == "any" {
                "array".into()
            } else {
                format!("{item_type}[]")
            })
        }
        Some(Value::String(s)) if s == "object" => {
            let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
                return Some("object".to_string());
            };
            let required: Vec<_> = schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let mut fields: Vec<_> = props
                .iter()
                .map(|(name, schema)| {
                    let ty = extract_type_from_schema(schema).unwrap_or_else(|| "any".into());
                    let opt = if required.contains(&name.as_str()) {
                        ""
                    } else {
                        "?"
                    };
                    format!("{name}{opt}: {ty}")
                })
                .collect();
            fields.sort();
            Some(format!("{{ {} }}", fields.join(", ")))
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Array(arr)) => {
            let non_null: Vec<_> = arr
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|s| *s != "null")
                .collect();
            match non_null.len() {
                0 => None,
                1 => Some(non_null[0].to_string()),
                _ => Some(non_null.join(" | ")),
            }
        }
        _ => None,
    }
}

struct ToolInfo {
    server_name: String,
    tool_name: String,
    full_name: String,
    description: String,
    params: Vec<(String, String, bool)>,
    return_type: String,
}

impl ToolInfo {
    fn from_mcp_tool(tool: &McpTool) -> Option<Self> {
        let (server_name, tool_name) = tool.name.as_ref().split_once("__")?;
        let param_names = get_parameter_names(tool);

        let mut schema_value = Value::Object(tool.input_schema.as_ref().clone());
        let _ = unbinder::dereference_schema(&mut schema_value, unbinder::Options::default());
        let schema: InputSchema = serde_json::from_value(schema_value).unwrap_or_default();

        let params = param_names
            .iter()
            .map(|name| {
                let ty = schema
                    .properties
                    .get(name)
                    .and_then(extract_type_from_schema)
                    .unwrap_or_else(|| "any".to_string());
                let required = schema.required.contains(name);
                (name.clone(), ty, required)
            })
            .collect();

        let return_type = tool
            .output_schema
            .as_ref()
            .and_then(|schema| {
                let mut schema_value = Value::Object(schema.as_ref().clone());
                let _ =
                    unbinder::dereference_schema(&mut schema_value, unbinder::Options::default());
                extract_type_from_schema(&schema_value)
            })
            .unwrap_or_else(|| "string".to_string());

        Some(Self {
            server_name: server_name.to_string(),
            tool_name: tool_name.to_string(),
            full_name: tool.name.as_ref().to_string(),
            description: tool
                .description
                .as_ref()
                .map(|d| d.as_ref().to_string())
                .unwrap_or_default(),
            params,
            return_type,
        })
    }

    fn to_signature(&self) -> String {
        let params = self
            .params
            .iter()
            .map(|(name, ty, req)| format!("{name}{}: {ty}", if *req { "" } else { "?" }))
            .collect::<Vec<_>>()
            .join(", ");
        let desc = self.description.lines().next().unwrap_or("");
        format!(
            "{}[\"{}\"]({{{params}}}): {} - {desc}",
            self.server_name, self.tool_name, self.return_type
        )
    }
}

thread_local! {
    static CALL_TX: std::cell::RefCell<Option<mpsc::UnboundedSender<ToolCallRequest>>> =
        const { std::cell::RefCell::new(None) };
}

fn create_server_module(
    server_name: &str,
    server_tools: &[&ToolInfo],
    ctx: &mut Context,
) -> Module {
    let tool_data: Vec<(String, String)> = server_tools
        .iter()
        .map(|t| (t.tool_name.clone(), t.full_name.clone()))
        .collect();

    let mut export_names: Vec<JsString> = server_tools
        .iter()
        .map(|t| js_string!(t.tool_name.as_str()))
        .collect();
    export_names.push(js_string!(server_name));

    let server_name_owned = server_name.to_string();

    Module::synthetic(
        &export_names,
        SyntheticModuleInitializer::from_copy_closure_with_captures(
            |module, (tools, server_name), context| {
                let namespace_obj = boa_engine::JsObject::with_null_proto();

                for (tool_name, full_name) in tools.iter() {
                    let func = create_tool_function(full_name.clone());
                    let js_func = func.to_js_function(context.realm());
                    module.set_export(&js_string!(tool_name.as_str()), js_func.clone().into())?;
                    namespace_obj
                        .set(js_string!(tool_name.as_str()), js_func, false, context)
                        .map_err(|e| {
                            JsNativeError::error().with_message(format!("Failed to set prop: {e}"))
                        })?;
                }
                module.set_export(&js_string!(server_name.as_str()), namespace_obj.into())?;

                Ok(())
            },
            (tool_data, server_name_owned),
        ),
        None,
        None,
        ctx,
    )
}

fn parse_result_to_js(result: &str, ctx: &mut Context) -> JsValue {
    serde_json::from_str::<serde_json::Value>(result)
        .ok()
        .and_then(|v| JsValue::from_json(&v, ctx).ok())
        .unwrap_or_else(|| JsValue::from(js_string!(result)))
}

fn create_tool_function(full_tool_name: String) -> NativeFunction {
    NativeFunction::from_copy_closure_with_captures(
        |_this, args, full_name: &String, ctx| {
            let args_json = args
                .first()
                .cloned()
                .unwrap_or(JsValue::undefined())
                .to_json(ctx)
                .map_err(|e| JsNativeError::error().with_message(e.to_string()))?
                .unwrap_or(Value::Object(serde_json::Map::new()));

            let args_str = serde_json::to_string(&args_json).unwrap_or_else(|_| "{}".to_string());
            let (tx, rx) = tokio::sync::oneshot::channel();

            CALL_TX
                .with(|call_tx| {
                    call_tx
                        .borrow()
                        .as_ref()
                        .and_then(|sender| sender.send((full_name.clone(), args_str, tx)).ok())
                })
                .ok_or_else(|| JsNativeError::error().with_message("Channel unavailable"))?;

            rx.blocking_recv()
                .map_err(|e| e.to_string())
                .and_then(|r| r)
                .map(|result| parse_result_to_js(&result, ctx))
                .map_err(|e| JsNativeError::error().with_message(e).into())
        },
        full_tool_name,
    )
}

fn run_js_module(
    code: &str,
    tools: &[ToolInfo],
    call_tx: mpsc::UnboundedSender<ToolCallRequest>,
) -> Result<String, String> {
    CALL_TX.with(|tx| *tx.borrow_mut() = Some(call_tx));

    let loader = Rc::new(MapModuleLoader::new());
    let mut ctx = Context::builder()
        .module_loader(loader.clone())
        .build()
        .map_err(|e| format!("Failed to create JS context: {e}"))?;

    ctx.register_global_property(
        js_string!("__result__"),
        JsValue::undefined(),
        Attribute::WRITABLE,
    )
    .map_err(|e| format!("Failed to register __result__: {e}"))?;

    let mut by_server: BTreeMap<&str, Vec<&ToolInfo>> = BTreeMap::new();
    for tool in tools {
        by_server.entry(&tool.server_name).or_default().push(tool);
    }

    for (server_name, server_tools) in &by_server {
        let module = create_server_module(server_name, server_tools, &mut ctx);
        loader.insert(*server_name, module);
    }

    let wrapped = {
        let lines: Vec<&str> = code.trim().lines().collect();
        let last_idx = lines
            .iter()
            .rposition(|l| !l.trim().is_empty() && !l.trim().starts_with("//"))
            .unwrap_or(0);
        let last = lines.get(last_idx).map(|s| s.trim()).unwrap_or("");

        const NO_WRAP: &[&str] = &["import ", "export ", "function ", "class "];
        if last.contains("__result__") || NO_WRAP.iter().any(|p| last.starts_with(p)) {
            code.to_string()
        } else {
            let before = lines[..last_idx].join("\n");
            let mut result = None;
            for decl in ["const ", "let ", "var "] {
                if let Some(rest) = last.strip_prefix(decl) {
                    if let Some(name) = rest.split('=').next().map(str::trim) {
                        result = Some(format!("{before}\n{last}\n__result__ = {name};"));
                    }
                    break;
                }
            }
            result.unwrap_or_else(|| {
                format!("{before}\n__result__ = {};", last.trim_end_matches(';'))
            })
        }
    };

    let user_module = Module::parse(Source::from_bytes(&wrapped), None, &mut ctx)
        .map_err(|e| format!("Parse error: {e}"))?;
    loader.insert("__main__", user_module.clone());

    let promise = user_module.load_link_evaluate(&mut ctx);
    ctx.run_jobs()
        .map_err(|e| format!("Job execution error: {e}"))?;

    match promise.state() {
        PromiseState::Fulfilled(_) => {
            let result = ctx
                .global_object()
                .get(js_string!("__result__"), &mut ctx)
                .map_err(|e| format!("Failed to get result: {e}"))?;
            Ok(result.display().to_string())
        }
        PromiseState::Rejected(err) => Err(format!("Module error: {}", err.display())),
        PromiseState::Pending => Err("Module evaluation did not complete".to_string()),
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
            instructions: Some(indoc! {r#"
                BATCH MULTIPLE TOOL CALLS INTO ONE execute_code CALL.

                This extension exists to reduce round-trips. When a task requires multiple tool calls:
                - WRONG: Multiple execute_code calls, each with one tool
                - RIGHT: One execute_code call with a script that calls all needed tools

                Workflow:
                    1. Use the read_module tool to discover tools and signatures
                    2. Write ONE script that imports and calls ALL tools needed for the task
                    3. Chain results: use output from one tool as input to the next
            "#}.to_string()),
        };

        Ok(Self { info, context })
    }

    async fn get_tool_infos(&self) -> Vec<ToolInfo> {
        let Some(manager) = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|w| w.upgrade())
        else {
            return Vec::new();
        };

        match manager.get_prefixed_tools_excluding(EXTENSION_NAME).await {
            Ok(tools) if !tools.is_empty() => {
                tools.iter().filter_map(ToolInfo::from_mcp_tool).collect()
            }
            _ => Vec::new(),
        }
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

        let tools = self.get_tool_infos().await;
        let (call_tx, call_rx) = mpsc::unbounded_channel();
        let tool_handler = tokio::spawn(Self::run_tool_handler(
            call_rx,
            self.context.extension_manager.clone(),
        ));

        let js_result = tokio::task::spawn_blocking(move || run_js_module(&code, &tools, call_tx))
            .await
            .map_err(|e| format!("JS execution task failed: {e}"))?;

        tool_handler.abort();
        js_result.map(|r| vec![Content::text(format!("Result: {r}"))])
    }

    async fn handle_read_module(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let path = arguments
            .as_ref()
            .and_then(|a| a.get("module_path"))
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: module_path")?;

        let tools = self.get_tool_infos().await;
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        match parts.as_slice() {
            [server] => {
                let server_tools: Vec<_> =
                    tools.iter().filter(|t| t.server_name == *server).collect();
                if server_tools.is_empty() {
                    return Err(format!("Module not found: {server}"));
                }
                let sigs: Vec<_> = server_tools.iter().map(|t| t.to_signature()).collect();
                Ok(vec![Content::text(format!(
                    "// import * as {server} from \"{server}\";\n\n{}",
                    sigs.join("\n")
                ))])
            }
            [server, tool] => {
                let t = tools
                    .iter()
                    .find(|t| t.server_name == *server && t.tool_name == *tool)
                    .ok_or_else(|| format!("Tool not found: {server}/{tool}"))?;
                Ok(vec![Content::text(format!(
                    "// import * as {server} from \"{server}\";\n\n{}\n\n{}",
                    t.to_signature(),
                    t.description
                ))])
            }
            _ => Err(format!(
                "Invalid path: {path}. Use 'server' or 'server/tool'"
            )),
        }
    }

    async fn handle_search_modules(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let terms = arguments
            .as_ref()
            .and_then(|a| a.get("terms"))
            .ok_or("Missing required parameter: terms")?;

        let terms_vec = if let Some(s) = terms.as_str() {
            vec![s.to_string()]
        } else if let Some(arr) = terms.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        } else {
            return Err("Parameter 'terms' must be a string or array of strings".to_string());
        };

        if terms_vec.is_empty() {
            return Err("Search terms cannot be empty".to_string());
        }

        let use_regex = arguments
            .as_ref()
            .and_then(|a| a.get("regex"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let tools = self.get_tool_infos().await;
        Self::handle_search(&tools, &terms_vec, use_regex)
    }

    fn handle_search(
        tools: &[ToolInfo],
        terms: &[String],
        use_regex: bool,
    ) -> Result<Vec<Content>, String> {
        enum Matcher {
            Regex(Vec<Regex>),
            Plain(Vec<String>),
        }

        let matcher = if use_regex {
            let patterns: Result<Vec<_>, _> = terms
                .iter()
                .map(|t| {
                    Regex::new(&format!("(?i){t}")).map_err(|e| format!("Invalid regex '{t}': {e}"))
                })
                .collect();
            Matcher::Regex(patterns?)
        } else {
            Matcher::Plain(terms.iter().map(|t| t.to_lowercase()).collect())
        };

        let matches_any = |text: &str| -> bool {
            match &matcher {
                Matcher::Regex(patterns) => patterns.iter().any(|p| p.is_match(text)),
                Matcher::Plain(terms) => {
                    let lower = text.to_lowercase();
                    terms.iter().any(|t| lower.contains(t))
                }
            }
        };

        let mut matching_servers: BTreeSet<&str> = BTreeSet::new();
        let mut matching_tools: Vec<&ToolInfo> = Vec::new();

        for tool in tools {
            if matches_any(&tool.server_name) {
                matching_servers.insert(&tool.server_name);
            }
            if matches_any(&tool.tool_name) || matches_any(&tool.description) {
                matching_tools.push(tool);
            }
        }

        if matching_servers.is_empty() && matching_tools.is_empty() {
            return Err(format!("No matches found for: {}", terms.join(", ")));
        }

        let mut output = String::new();

        if !matching_servers.is_empty() {
            output.push_str("## Matching Servers\n");
            for server in &matching_servers {
                let count = tools.iter().filter(|t| t.server_name == *server).count();
                output.push_str(&format!("- {server} ({count} tools)\n"));
            }
            output.push('\n');
        }

        if !matching_tools.is_empty() {
            output.push_str("## Matching Tools\n");
            output.push_str("Use the read_module tool for full signature and import syntax\n\n");
            for tool in &matching_tools {
                output.push_str(&format!(
                    "- {}/{}: {}\n",
                    tool.server_name,
                    tool.tool_name,
                    tool.description.lines().next().unwrap_or("")
                ));
            }
        }

        Ok(vec![Content::text(output)])
    }

    async fn run_tool_handler(
        mut call_rx: mpsc::UnboundedReceiver<ToolCallRequest>,
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
                            Ok(result) => Ok(if let Some(sc) = &result.structured_content {
                                serde_json::to_string(sc).unwrap_or_default()
                            } else {
                                result
                                    .content
                                    .iter()
                                    .filter_map(|c| match &c.raw {
                                        RawContent::Text(t) => Some(t.text.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }),
                            Err(e) => Err(format!("Tool error: {}", e.message)),
                        },
                        Err(e) => Err(format!("Dispatch error: {e}")),
                    }
                }
                None => Err("Extension manager not available".to_string()),
            };
            let _ = response_tx.send(result);
        }
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

        Ok(ListToolsResult {
            tools: vec![
                McpTool::new(
                    "execute_code".to_string(),
                    indoc! {r#"
                        Batch multiple MCP tool calls into ONE execution. This is the primary purpose of this tool.

                        CRITICAL: Always combine related operations into a single execute_code call.
                        - WRONG: execute_code to read → execute_code to write (2 calls)
                        - RIGHT: execute_code that reads AND writes in one script (1 call)

                        EXAMPLE - Read file and write to another (ONE call):
                        ```javascript
                        import { text_editor } from "developer";
                        const content = text_editor({ path: "/path/to/source.md", command: "view" });
                        text_editor({ path: "/path/to/dest.md", command: "write", file_text: content });
                        ```

                        EXAMPLE - Multiple operations chained:
                        ```javascript
                        import { shell, text_editor } from "developer";
                        const files = shell({ command: "ls -la" });
                        const readme = text_editor({ path: "./README.md", command: "view" });
                        const status = shell({ command: "git status" });
                        { files, readme, status }
                        ```

                        SYNTAX:
                        - Import: import { tool1, tool2 } from "serverName";
                        - Call: toolName({ param1: value, param2: value })
                        - All calls are synchronous, return strings
                        - Last expression is the result
                        - No comments in code

                        TOOL_GRAPH: Always provide tool_graph to describe the execution flow for the UI.
                        Each node has: tool (server/name), description (what it does), depends_on (indices of dependencies).
                        Example for chained operations:
                        [
                          {"tool": "developer/shell", "description": "list files", "depends_on": []},
                          {"tool": "developer/text_editor", "description": "read README.md", "depends_on": []},
                          {"tool": "developer/text_editor", "description": "write output.txt", "depends_on": [0, 1]}
                        ]

                        BEFORE CALLING: Use the read_module tool to check required parameters.
                    "#}
                    .to_string(),
                    schema::<ExecuteCodeParams>(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Execute JavaScript".to_string()),
                    read_only_hint: Some(false),
                    destructive_hint: Some(true),
                    idempotent_hint: Some(false),
                    open_world_hint: Some(true),
                }),
                McpTool::new(
                    "read_module".to_string(),
                    indoc! {r#"
                        Read tool definitions to understand how to call them correctly.

                        PATHS:
                        - "serverName" → lists all tools with signatures (shows required vs optional params)
                        - "serverName/toolName" → full details for one tool including description

                        USE THIS BEFORE execute_code when:
                        - You haven't used a tool before
                        - You're unsure of parameter names or which are required
                        - A previous call failed due to missing/wrong parameters

                        The signature format is: toolName({ param1: type, param2?: type }): string
                        Parameters with ? are optional; others are required.
                    "#}
                    .to_string(),
                    schema::<ReadModuleParams>(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Read module".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                }),
                McpTool::new(
                    "search_modules".to_string(),
                    indoc! {r#"
                        Search for tools by name or description across all available modules.

                        USAGE:
                        - Single term: search_modules with terms="file"
                        - Multiple terms: search_modules with terms=["git", "shell"]
                        - Regex patterns: search_modules with terms="sh.*", regex=true

                        Returns matching servers and tools with descriptions.
                        Use this when you don't know which module contains the tool you need.
                    "#}
                    .to_string(),
                    schema::<SearchModulesParams>(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Search modules".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
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
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "execute_code" => self.handle_execute_code(arguments).await,
            "read_module" => self.handle_read_module(arguments).await,
            "search_modules" => self.handle_search_modules(arguments).await,
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
        let tools = self.get_tool_infos().await;
        if tools.is_empty() {
            return None;
        }

        let mut servers: BTreeSet<&str> = BTreeSet::new();
        for tool in &tools {
            servers.insert(&tool.server_name);
        }

        let server_list: Vec<_> = servers.into_iter().collect();

        Some(format!(
            indoc::indoc! {r#"
                ALWAYS batch multiple tool operations into ONE execute_code call.
                - WRONG: Separate execute_code calls for read file, then write file
                - RIGHT: One execute_code with a script that reads AND writes

                Modules: {}

                Use the read_module tool to see signatures before calling unfamiliar tools.
            "#},
            server_list.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use test_case::test_case;

    #[tokio::test]
    async fn test_execute_code_simple() {
        let context = PlatformExtensionContext {
            session_id: None,
            extension_manager: None,
        };
        let client = CodeExecutionClient::new(context).unwrap();

        let mut args = JsonObject::new();
        args.insert("code".to_string(), Value::String("2 + 2".to_string()));

        let result = client
            .call_tool("execute_code", Some(args), CancellationToken::new())
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        if let RawContent::Text(text) = &result.content[0].raw {
            assert_eq!(text.text, "Result: 4");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_read_module_not_found() {
        let context = PlatformExtensionContext {
            session_id: None,
            extension_manager: None,
        };
        let client = CodeExecutionClient::new(context).unwrap();

        let mut args = JsonObject::new();
        args.insert(
            "module_path".to_string(),
            Value::String("nonexistent".to_string()),
        );

        let result = client.handle_read_module(Some(args)).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_search_plain_text() {
        let tools = vec![
            ToolInfo {
                server_name: "developer".to_string(),
                tool_name: "shell".to_string(),
                full_name: "developer__shell".to_string(),
                description: "Execute shell commands".to_string(),
                params: vec![("command".to_string(), "string".to_string(), true)],
                return_type: "string".to_string(),
            },
            ToolInfo {
                server_name: "developer".to_string(),
                tool_name: "text_editor".to_string(),
                full_name: "developer__text_editor".to_string(),
                description: "Edit text files".to_string(),
                params: vec![("path".to_string(), "string".to_string(), true)],
                return_type: "string".to_string(),
            },
            ToolInfo {
                server_name: "git".to_string(),
                tool_name: "commit".to_string(),
                full_name: "git__commit".to_string(),
                description: "Commit changes to git".to_string(),
                params: vec![("message".to_string(), "string".to_string(), true)],
                return_type: "string".to_string(),
            },
        ];

        // Search for "shell" - should match tool name
        let result =
            CodeExecutionClient::handle_search(&tools, &["shell".to_string()], false).unwrap();
        let text = match &result[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("Expected text"),
        };
        assert!(text.contains("developer/shell"));
        assert!(!text.contains("git/commit"));

        // Search for "developer" - should match server name
        let result =
            CodeExecutionClient::handle_search(&tools, &["developer".to_string()], false).unwrap();
        let text = match &result[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("Expected text"),
        };
        assert!(text.contains("developer (2 tools)"));

        // Search for "edit" - should match description
        let result =
            CodeExecutionClient::handle_search(&tools, &["edit".to_string()], false).unwrap();
        let text = match &result[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("Expected text"),
        };
        assert!(text.contains("developer/text_editor"));

        // Search for multiple terms
        let result = CodeExecutionClient::handle_search(
            &tools,
            &["shell".to_string(), "git".to_string()],
            false,
        )
        .unwrap();
        let text = match &result[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("Expected text"),
        };
        assert!(text.contains("developer/shell"));
        assert!(text.contains("git/commit"));

        // Search with no matches
        let result =
            CodeExecutionClient::handle_search(&tools, &["nonexistent".to_string()], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_regex() {
        let tools = vec![
            ToolInfo {
                server_name: "developer".to_string(),
                tool_name: "shell".to_string(),
                full_name: "developer__shell".to_string(),
                description: "Execute shell commands".to_string(),
                params: vec![],
                return_type: "string".to_string(),
            },
            ToolInfo {
                server_name: "developer".to_string(),
                tool_name: "text_editor".to_string(),
                full_name: "developer__text_editor".to_string(),
                description: "Edit text files".to_string(),
                params: vec![],
                return_type: "string".to_string(),
            },
        ];

        // Regex search for "sh.*" - should match shell
        let result =
            CodeExecutionClient::handle_search(&tools, &["sh.*".to_string()], true).unwrap();
        let text = match &result[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("Expected text"),
        };
        assert!(text.contains("developer/shell"));

        // Regex search for "^text" - should match text_editor
        let result =
            CodeExecutionClient::handle_search(&tools, &["^text".to_string()], true).unwrap();
        let text = match &result[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("Expected text"),
        };
        assert!(text.contains("developer/text_editor"));

        // Invalid regex should error
        let result = CodeExecutionClient::handle_search(&tools, &["[invalid".to_string()], true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid regex"));
    }

    #[test_case(
        "github__get_me",
        serde_json::json!({"type": "object", "properties": {}}),
        None,
        "github[\"get_me\"]({}): string - Get details of the authenticated user";
        "no params, no output schema"
    )]
    #[test_case(
        "filesystem__read_text_file",
        serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}, "tail": {"type": "number"}, "head": {"type": "number"}}, "required": ["path"]}),
        Some(serde_json::json!({"type": "object", "properties": {"content": {"type": "string"}}, "required": ["content"]})),
        "filesystem[\"read_text_file\"]({head?: number, path: string, tail?: number}): { content: string } - Read the complete contents of a file";
        "optional number params, object output"
    )]
    #[test_case(
        "memory__create_entities",
        serde_json::json!({"type": "object", "properties": {"entities": {"type": "array", "items": {"type": "object", "properties": {"name": {"type": "string"}, "entityType": {"type": "string"}, "observations": {"type": "array", "items": {"type": "string"}}}, "required": ["name", "entityType", "observations"]}}}, "required": ["entities"]}),
        Some(serde_json::json!({"type": "object", "properties": {"entities": {"type": "array", "items": {"type": "object", "properties": {"name": {"type": "string"}, "entityType": {"type": "string"}, "observations": {"type": "array", "items": {"type": "string"}}}, "required": ["name", "entityType", "observations"]}}}, "required": ["entities"]})),
        "memory[\"create_entities\"]({entities: { entityType: string, name: string, observations: string[] }[]}): { entities: { entityType: string, name: string, observations: string[] }[] } - Create multiple new entities";
        "nested object array with typed props"
    )]
    #[test_case(
        "github__dismiss_notification",
        serde_json::json!({"type": "object", "properties": {
            "threadID": {"type": "string"},
            "state": {"type": "string", "enum": ["read", "done"]}
        }, "required": ["threadID", "state"]}),
        None,
        "github[\"dismiss_notification\"]({state: \"read\" | \"done\", threadID: string}): string - Dismiss a notification";
        "enum param, no output schema"
    )]
    #[test_case(
        "computercontroller__web_scrape",
        serde_json::json!({"type": "object", "properties": {
            "url": {"type": "string"},
            "save_as": {"oneOf": [{"const": "text"}, {"const": "json"}, {"const": "binary"}]}
        }, "required": ["url"]}),
        None,
        "computercontroller[\"web_scrape\"]({save_as?: \"text\" | \"json\" | \"binary\", url: string}): string - Scrape content from URL";
        "oneOf const param (schemars), no output schema"
    )]
    #[test_case(
        "kiwitravel__search-flight",
        serde_json::json!({"type": "object", "properties": {
            "flyFrom": {"type": "string"},
            "flyTo": {"type": "string"},
            "departureDate": {"type": "string"}
        }, "required": ["flyFrom", "flyTo", "departureDate"]}),
        None,
        "kiwitravel[\"search-flight\"]({departureDate: string, flyFrom: string, flyTo: string}): string - Search for flights";
        "hyphenated tool name uses bracket notation"
    )]
    fn test_mcp_tool_signature(
        name: &str,
        input: serde_json::Value,
        output: Option<serde_json::Value>,
        expected: &str,
    ) {
        let input_schema: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(input).unwrap();
        let output_schema = output.map(|v| {
            Arc::new(
                serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(v).unwrap(),
            )
        });
        let desc = expected.split(" - ").nth(1).unwrap_or("").to_string();
        let tool = McpTool {
            name: name.to_string().into(),
            title: None,
            description: Some(desc.into()),
            input_schema: Arc::new(input_schema),
            output_schema,
            annotations: None,
            icons: None,
            meta: None,
        };
        let info = ToolInfo::from_mcp_tool(&tool).unwrap();
        assert_eq!(info.to_signature(), expected);
    }

    #[test_case(serde_json::json!({"type": "string"}), "string"; "string")]
    #[test_case(serde_json::json!({"type": "number"}), "number"; "number")]
    #[test_case(serde_json::json!({"type": "boolean"}), "boolean"; "boolean")]
    #[test_case(serde_json::json!({"type": "array"}), "array"; "array bare")]
    #[test_case(serde_json::json!({"type": "array", "items": {"type": "string"}}), "string[]"; "array with items")]
    #[test_case(serde_json::json!({"type": "object"}), "object"; "object bare")]
    #[test_case(serde_json::json!({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a"]}), "{ a: string }"; "object with prop")]
    #[test_case(serde_json::json!({"type": "object", "properties": {"a": {"type": "string"}}}), "{ a?: string }"; "object optional prop")]
    #[test_case(serde_json::json!({"type": "object", "properties": {"a": {"type": "array", "items": {"type": "string"}}}, "required": ["a"]}), "{ a: string[] }"; "object with array prop")]
    #[test_case(serde_json::json!({"enum": ["a", "b"]}), "\"a\" | \"b\""; "enum array")]
    #[test_case(serde_json::json!({"oneOf": [{"const": "x"}, {"const": "y"}]}), "\"x\" | \"y\""; "oneOf const")]
    fn test_extract_type_from_schema(schema: serde_json::Value, expected: &str) {
        assert_eq!(
            extract_type_from_schema(&schema),
            Some(expected.to_string())
        );
    }

    fn eval_with_tools(code: &str, tools: &[(&str, &str)]) -> String {
        let mut ctx = Context::default();
        for &(name, response) in tools {
            let resp = response.to_string();
            let func = NativeFunction::from_copy_closure_with_captures(
                |_this, _args, resp: &String, ctx| Ok(parse_result_to_js(resp, ctx)),
                resp,
            );
            ctx.register_global_callable(js_string!(name), 0, func)
                .unwrap();
        }
        ctx.eval(Source::from_bytes(code))
            .unwrap()
            .display()
            .to_string()
    }

    #[test_case("2 + 2", &[], "4"; "pure_js")]
    #[test_case("get_data({}).content", &[("get_data", r#"{"content":"hello"}"#)], "\"hello\""; "structured_property_access")]
    #[test_case("typeof shell({})", &[("shell", "plain text")], "\"string\""; "plain_text_is_string")]
    #[test_case("shell({}).content", &[("shell", "plain text")], "undefined"; "plain_text_no_property")]
    fn test_tool_result(code: &str, tools: &[(&str, &str)], expected: &str) {
        assert_eq!(eval_with_tools(code, tools), expected);
    }

    #[test]
    fn test_namespace_import_with_synthetic_module() {
        let tools = vec![ToolInfo {
            server_name: "testserver".to_string(),
            tool_name: "get_value".to_string(),
            full_name: "testserver__get_value".to_string(),
            description: "Get a value".to_string(),
            params: vec![],
            return_type: "string".to_string(),
        }];

        let (tx, _rx) = mpsc::unbounded_channel();

        let code_named = r#"import { get_value } from "testserver"; typeof get_value"#;
        let result = run_js_module(code_named, &tools, tx.clone());
        assert!(
            result.is_ok(),
            "Named import should work: {:?}",
            result.err()
        );

        let code_namespace =
            r#"import * as testserver from "testserver"; typeof testserver.get_value"#;
        let result = run_js_module(code_namespace, &tools, tx.clone());
        assert!(
            result.is_ok(),
            "Namespace import should work: {:?}",
            result.err()
        );

        let code_server_named =
            r#"import { testserver } from "testserver"; typeof testserver.get_value"#;
        let result = run_js_module(code_server_named, &tools, tx.clone());
        assert!(
            result.is_ok(),
            "Server-named import should work: {:?}",
            result.err()
        );

        let code_bracket =
            r#"import { testserver } from "testserver"; typeof testserver["get_value"]"#;
        let result = run_js_module(code_bracket, &tools, tx);
        assert!(
            result.is_ok(),
            "Bracket notation should work: {:?}",
            result.err()
        );
    }
}
