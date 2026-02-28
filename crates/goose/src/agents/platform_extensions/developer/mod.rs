pub mod edit;
pub mod shell;
pub mod tree;

use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use edit::{EditTools, FileEditParams, FileReadParams, FileWriteParams};
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde_json::Value;
use shell::{ShellParams, ShellTool};
use std::path::Path;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tree::{TreeParams, TreeTool};

pub static EXTENSION_NAME: &str = "developer";

pub struct DeveloperClient {
    info: InitializeResult,
    shell_tool: Arc<ShellTool>,
    edit_tools: Arc<EditTools>,
    tree_tool: Arc<TreeTool>,
}

impl DeveloperClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
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
                title: Some("Developer".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(indoc! {"
                Use the developer extension to build software and operate a terminal.

                Make sure to use the tools *efficiently* - reading all the content you need in as few
                iterations as possible and then making the requested edits or running commands. You are
                responsible for managing your context window, and to minimize unnecessary turns which
                cost the user money.

                For editing software, prefer the flow of using tree to understand the codebase structure
                and file sizes. When you need to search, prefer rg which correctly respects gitignored
                content. Use read to gather file context (and use offset/limit for large files), always
                reading before editing. Use write and edit to efficiently make changes. Use shell for
                command execution and workflows where terminal tools are the right fit. Test and verify
                as appropriate.
            "}.to_string()),
        };

        let edit_tools = context
            .developer_file_io
            .clone()
            .map(|file_io| {
                EditTools::with_file_io(
                    file_io.read_file,
                    file_io.read_file_chunk,
                    file_io.write_file,
                )
            })
            .unwrap_or_default();

        Ok(Self {
            info,
            shell_tool: Arc::new(ShellTool::new()),
            edit_tools: Arc::new(edit_tools),
            tree_tool: Arc::new(TreeTool::new()),
        })
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
        serde_json::from_value(value).map_err(|e| format!("Failed to parse arguments: {e}"))
    }

    fn get_tools() -> Vec<Tool> {
        vec![
            Tool::new(
                "read".to_string(),
                "Read a UTF-8 text file and return its contents. Files over 1MB require offset/limit for chunked reads."
                    .to_string(),
                Self::schema::<FileReadParams>(),
            )
            .annotate(ToolAnnotations {
                title: Some("Read".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
            Tool::new(
                "write".to_string(),
                "Create a new file or overwrite an existing file. Creates parent directories if needed.".to_string(),
                Self::schema::<FileWriteParams>(),
            )
            .annotate(ToolAnnotations {
                title: Some("Write".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(true),
                idempotent_hint: Some(false),
                open_world_hint: Some(false),
            }),
            Tool::new(
                "edit".to_string(),
                "Edit a file by finding and replacing text. The before text must match exactly and uniquely. Use empty after text to delete.".to_string(),
                Self::schema::<FileEditParams>(),
            )
            .annotate(ToolAnnotations {
                title: Some("Edit".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(true),
                idempotent_hint: Some(false),
                open_world_hint: Some(false),
            }),
            Tool::new(
                "shell".to_string(),
                "Execute a shell command in the user's default shell in the current dir and return both stdout/stderr. The output is limited to up to 2000 lines, and longer outputs will be saved to a temporary file.".to_string(),
                Self::schema::<ShellParams>(),
            )
            .annotate(ToolAnnotations {
                title: Some("Shell".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(true),
                idempotent_hint: Some(false),
                open_world_hint: Some(true),
            }),
            Tool::new(
                "tree".to_string(),
                "List a directory tree with line counts. Traversal respects .gitignore rules.".to_string(),
                Self::schema::<TreeParams>(),
            )
            .annotate(ToolAnnotations {
                title: Some("Tree".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
        ]
    }
}

#[async_trait]
impl McpClientTrait for DeveloperClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
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
            "shell" => match Self::parse_args::<ShellParams>(arguments) {
                Ok(params) => Ok(self.shell_tool.shell_with_cwd(params, working_dir).await),
                Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error: {error}"
                ))
                .with_priority(0.0)])),
            },
            "read" => match Self::parse_args::<FileReadParams>(arguments) {
                Ok(params) => Ok(self
                    .edit_tools
                    .file_read_with_cwd(params, working_dir)
                    .await),
                Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error: {error}"
                ))
                .with_priority(0.0)])),
            },
            "write" => match Self::parse_args::<FileWriteParams>(arguments) {
                Ok(params) => Ok(self
                    .edit_tools
                    .file_write_with_cwd(params, working_dir)
                    .await),
                Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error: {error}"
                ))
                .with_priority(0.0)])),
            },
            "edit" => match Self::parse_args::<FileEditParams>(arguments) {
                Ok(params) => Ok(self
                    .edit_tools
                    .file_edit_with_cwd(params, working_dir)
                    .await),
                Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error: {error}"
                ))
                .with_priority(0.0)])),
            },
            "tree" => match Self::parse_args::<TreeParams>(arguments) {
                Ok(params) => Ok(self.tree_tool.tree_with_cwd(params, working_dir)),
                Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error: {error}"
                ))
                .with_priority(0.0)])),
            },
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
    use rmcp::object;
    use std::fs;

    #[test]
    fn developer_tools_are_flat() {
        let names: Vec<String> = DeveloperClient::get_tools()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();

        assert_eq!(names, vec!["read", "write", "edit", "shell", "tree"]);
    }

    fn test_context(data_dir: std::path::PathBuf) -> PlatformExtensionContext {
        PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(SessionManager::new(data_dir)),
            session: None,
            developer_file_io: None,
        }
    }

    fn first_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(text) => &text.text,
            _ => panic!("expected text content"),
        }
    }

    #[tokio::test]
    async fn developer_client_uses_working_dir_for_file_tools() {
        let temp = tempfile::tempdir().unwrap();
        let client = DeveloperClient::new(test_context(temp.path().join("sessions"))).unwrap();
        let cwd = temp.path().join("workspace");
        fs::create_dir_all(&cwd).unwrap();

        let write = client
            .call_tool(
                "session",
                "write",
                Some(object!({
                    "path": "notes.txt",
                    "content": "first line"
                })),
                Some(cwd.to_str().unwrap()),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(write.is_error, Some(false));
        assert_eq!(
            fs::read_to_string(cwd.join("notes.txt")).unwrap(),
            "first line"
        );

        let edit = client
            .call_tool(
                "session",
                "edit",
                Some(object!({
                    "path": "notes.txt",
                    "before": "first",
                    "after": "updated"
                })),
                Some(cwd.to_str().unwrap()),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(edit.is_error, Some(false));
        assert_eq!(
            fs::read_to_string(cwd.join("notes.txt")).unwrap(),
            "updated line"
        );

        let read = client
            .call_tool(
                "session",
                "read",
                Some(object!({
                    "path": "notes.txt"
                })),
                Some(cwd.to_str().unwrap()),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(read.is_error, Some(false));
        assert_eq!(first_text(&read), "updated line");
    }

    #[tokio::test]
    async fn developer_client_delegates_read_to_context_io() {
        use crate::agents::platform_extensions::{DeveloperFileIo, ReadFileFn, WriteFileFn};
        use std::path::PathBuf;
        use std::sync::Arc;

        let temp = tempfile::tempdir().unwrap();
        let mut context = test_context(temp.path().join("sessions"));

        let read_file: ReadFileFn = Arc::new(|path: PathBuf| {
            Box::pin(async move {
                if path.to_string_lossy().ends_with("delegated.txt") {
                    Ok("delegated content".to_string())
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("not found: {}", path.display()),
                    ))
                }
            })
        });

        let write_file: WriteFileFn =
            Arc::new(|_path: PathBuf, _content: String| Box::pin(async move { Ok(()) }));

        context.developer_file_io = Some(DeveloperFileIo {
            read_file,
            read_file_chunk: None,
            write_file,
        });

        let client = DeveloperClient::new(context).unwrap();

        let result = client
            .call_tool(
                "session",
                "read",
                Some(object!({
                    "path": "delegated.txt"
                })),
                None,
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(result.is_error, Some(false));
        assert_eq!(first_text(&result), "delegated content");
    }

    #[tokio::test]
    async fn developer_client_delegates_write_to_context_io() {
        use crate::agents::platform_extensions::{DeveloperFileIo, ReadFileFn, WriteFileFn};
        use std::path::PathBuf;
        use std::sync::{Arc, Mutex};

        let temp = tempfile::tempdir().unwrap();
        let mut context = test_context(temp.path().join("sessions"));

        let captured_writes = Arc::new(Mutex::new(Vec::new()));
        let writes_clone = captured_writes.clone();

        let read_file: ReadFileFn = Arc::new(|_path: PathBuf| {
            Box::pin(async move {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                ))
            })
        });

        let write_file: WriteFileFn = Arc::new(move |path: PathBuf, content: String| {
            let writes = writes_clone.clone();
            Box::pin(async move {
                writes.lock().unwrap().push((path, content));
                Ok(())
            })
        });

        context.developer_file_io = Some(DeveloperFileIo {
            read_file,
            read_file_chunk: None,
            write_file,
        });

        let client = DeveloperClient::new(context).unwrap();

        let result = client
            .call_tool(
                "session",
                "write",
                Some(object!({
                    "path": "output.txt",
                    "content": "delegated write"
                })),
                None,
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(result.is_error, Some(false));

        let writes = captured_writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert!(writes[0].0.to_string_lossy().ends_with("output.txt"));
        assert_eq!(writes[0].1, "delegated write");
    }

    #[tokio::test]
    async fn developer_client_delegates_read_slice_fallback() {
        use crate::agents::platform_extensions::{DeveloperFileIo, ReadFileFn, WriteFileFn};
        use std::path::PathBuf;
        use std::sync::Arc;

        let temp = tempfile::tempdir().unwrap();
        let mut context = test_context(temp.path().join("sessions"));

        let read_file: ReadFileFn = Arc::new(|_path: PathBuf| {
            Box::pin(async move {
                // Return 5 lines
                Ok("line1\nline2\nline3\nline4\nline5".to_string())
            })
        });

        let write_file: WriteFileFn =
            Arc::new(|_path: PathBuf, _content: String| Box::pin(async move { Ok(()) }));

        // Explicitly set read_file_chunk to None to force fallback
        context.developer_file_io = Some(DeveloperFileIo {
            read_file,
            read_file_chunk: None,
            write_file,
        });

        let client = DeveloperClient::new(context).unwrap();

        // Request offset=1, limit=2 (expect "line2\nline3")
        let result = client
            .call_tool(
                "session",
                "read",
                Some(object!({
                    "path": "test.txt",
                    "offset": 1,
                    "limit": 2
                })),
                None,
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(result.is_error, Some(false));
        assert_eq!(first_text(&result), "line2\nline3");
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn developer_client_uses_working_dir_for_shell_tool() {
        let temp = tempfile::tempdir().unwrap();
        let client = DeveloperClient::new(test_context(temp.path().join("sessions"))).unwrap();
        let cwd = temp.path().join("workspace");
        fs::create_dir_all(&cwd).unwrap();

        let result = client
            .call_tool(
                "session",
                "shell",
                Some(object!({
                    "command": "pwd"
                })),
                Some(cwd.to_str().unwrap()),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(false));
        let observed = std::fs::canonicalize(first_text(&result)).unwrap();
        let expected = std::fs::canonicalize(&cwd).unwrap();
        assert_eq!(observed, expected);
    }
}
