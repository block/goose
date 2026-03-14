pub mod client;
pub mod tools;
pub mod tree_gen;

use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};

use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "warpgrep";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CodebaseSearchParams {
    /// Natural language search query
    pub search_string: String,
    /// Directory to scope the search, defaults to working directory
    #[serde(default)]
    pub path: Option<String>,
}

pub struct WarpGrepClient {
    info: InitializeResult,
}

impl WarpGrepClient {
    pub fn new(_context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                tasks: None,
                resources: None,
                extensions: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                description: None,
                title: Some("Codebase Search".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                indoc! {"
                Agentic codebase search powered by Morph's WarpGrep. Use codebase_search
                to find relevant code using natural language queries. A search agent will
                autonomously explore the repo structure, run ripgrep, read files across
                multiple turns, and return the most relevant code spans. Prefer this over
                manual grep for broad or conceptual queries like 'how is auth handled' or
                'where are errors propagated'.
            "}
                .to_string(),
            ),
        };

        Ok(Self { info })
    }

    fn schema<T: JsonSchema>() -> JsonObject {
        serde_json::to_value(schema_for!(T))
            .expect("schema serialization should succeed")
            .as_object()
            .expect("schema should serialize to an object")
            .clone()
    }

    fn parse_args<T: serde::de::DeserializeOwned>(
        arguments: Option<JsonObject>,
    ) -> Result<T, String> {
        let value = arguments
            .map(Value::Object)
            .ok_or_else(|| "Missing arguments".to_string())?;
        serde_json::from_value(value).map_err(|err| format!("Failed to parse arguments: {err}"))
    }

    fn resolve_path(path: &str, working_dir: Option<&Path>) -> PathBuf {
        let target = PathBuf::from(path);
        if target.is_absolute() {
            target
        } else if let Some(cwd) = working_dir {
            cwd.join(target)
        } else {
            target
        }
    }
}

#[async_trait]
impl McpClientTrait for WarpGrepClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        let tool = Tool::new(
            "codebase_search".to_string(),
            "Agentic codebase search. An agent autonomously explores the repo structure, runs ripgrep, and reads files across multiple turns to find relevant code. Returns the most relevant code spans with line numbers. Use for broad or conceptual queries like 'where is auth handled' or 'how are errors propagated'.".to_string(),
            Self::schema::<CodebaseSearchParams>(),
        )
        .annotate(ToolAnnotations {
            title: Some("Codebase Search".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(true),
        });

        Ok(ListToolsResult {
            tools: vec![tool],
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        _session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        working_dir: Option<&str>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let working_dir = working_dir.map(Path::new);
        match name {
            "codebase_search" => {
                let api_key = match client::get_api_key() {
                    Ok(key) => key,
                    Err(msg) => {
                        return Ok(CallToolResult::error(vec![
                            Content::text(msg).with_priority(0.0)
                        ]));
                    }
                };

                match Self::parse_args::<CodebaseSearchParams>(arguments) {
                    Ok(params) => {
                        let search_dir = match &params.path {
                            Some(dir) => Self::resolve_path(dir, working_dir),
                            None => working_dir
                                .map(Path::to_path_buf)
                                .unwrap_or_else(|| PathBuf::from(".")),
                        };

                        if !search_dir.exists() {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "Error: path not found: {}",
                                search_dir.display()
                            ))
                            .with_priority(0.0)]));
                        }

                        match client::search(&params.search_string, &search_dir, &api_key).await {
                            Ok(result) => Ok(CallToolResult::success(vec![
                                Content::text(result).with_priority(0.0)
                            ])),
                            Err(err) => Ok(CallToolResult::error(vec![
                                Content::text(err).with_priority(0.0)
                            ])),
                        }
                    }
                    Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                        "Error: {error}"
                    ))
                    .with_priority(0.0)])),
                }
            }
            _ => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: Unknown tool: {name}"
            ))
            .with_priority(0.0)])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;
    use rmcp::model::RawContent;
    use std::sync::Arc;
    fn ctx() -> PlatformExtensionContext {
        PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(SessionManager::new(std::env::temp_dir())),
            session: None,
        }
    }

    fn text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn lists_one_tool() {
        let client = WarpGrepClient::new(ctx()).unwrap();
        let result = client
            .list_tools("test", None, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name.to_string(), "codebase_search");
    }

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let client = WarpGrepClient::new(ctx()).unwrap();
        let result = client
            .call_tool("test", "nonexistent", None, None, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        assert!(text(&result).contains("Unknown tool"));
    }

    #[tokio::test]
    async fn missing_arguments_returns_error() {
        let client = WarpGrepClient::new(ctx()).unwrap();
        let result = client
            .call_tool(
                "test",
                "codebase_search",
                None,
                None,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn nonexistent_path_returns_error() {
        let client = WarpGrepClient::new(ctx()).unwrap();
        let args: JsonObject = serde_json::from_value(serde_json::json!({
            "search_string": "test query",
            "path": "/nonexistent/path/that/does/not/exist"
        }))
        .unwrap();

        let result = client
            .call_tool(
                "test",
                "codebase_search",
                Some(args),
                Some("/tmp"),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        assert!(text(&result).contains("path not found"));
    }

    #[test]
    fn resolve_path_absolute() {
        let abs = "/some/absolute/path";
        let result = WarpGrepClient::resolve_path(abs, Some(Path::new("/working")));
        assert_eq!(result, PathBuf::from(abs));
    }

    #[test]
    fn resolve_path_relative_with_working_dir() {
        let result =
            WarpGrepClient::resolve_path("src/main.rs", Some(Path::new("/home/user/project")));
        assert_eq!(result, PathBuf::from("/home/user/project/src/main.rs"));
    }

    #[test]
    fn resolve_path_relative_without_working_dir() {
        let result = WarpGrepClient::resolve_path("src/main.rs", None);
        assert_eq!(result, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn get_info_returns_some() {
        let client = WarpGrepClient::new(ctx()).unwrap();
        assert!(client.get_info().is_some());
    }

    #[tokio::test]
    async fn tool_annotations_are_correct() {
        let client = WarpGrepClient::new(ctx()).unwrap();
        let result = client
            .list_tools("test", None, CancellationToken::new())
            .await
            .unwrap();
        let tool = &result.tools[0];
        let annotations = tool.annotations.as_ref().unwrap();
        assert_eq!(annotations.read_only_hint, Some(true));
        assert_eq!(annotations.destructive_hint, Some(false));
        assert_eq!(annotations.idempotent_hint, Some(true));
        assert_eq!(annotations.open_world_hint, Some(true));
    }
}
