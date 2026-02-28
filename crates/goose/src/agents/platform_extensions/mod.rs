pub mod analyze;
pub mod apps;
pub mod chatrecall;
#[cfg(feature = "code-mode")]
pub mod code_execution;
pub mod developer;
pub mod ext_manager;
pub mod summon;
pub mod todo;
pub mod tom;

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use crate::agents::mcp_client::McpClientTrait;
use crate::session::Session;
use once_cell::sync::Lazy;
use tokio::io::AsyncBufReadExt;

pub use ext_manager::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;

pub const MAX_READ_FILE_BYTES: usize = 1024 * 1024;

pub type ReadFileFuture = Pin<Box<dyn Future<Output = io::Result<String>> + Send>>;
pub type ReadFileChunkFuture = Pin<Box<dyn Future<Output = io::Result<String>> + Send>>;
pub type WriteFileFuture = Pin<Box<dyn Future<Output = io::Result<()>> + Send>>;
pub type ReadFileFn = Arc<dyn Fn(PathBuf) -> ReadFileFuture + Send + Sync>;
pub type ReadFileChunkFn = Arc<dyn Fn(PathBuf, usize, usize) -> ReadFileChunkFuture + Send + Sync>;
pub type WriteFileFn = Arc<dyn Fn(PathBuf, String) -> WriteFileFuture + Send + Sync>;

#[derive(Clone)]
pub struct DeveloperFileIo {
    pub read_file: ReadFileFn,
    pub read_file_chunk: Option<ReadFileChunkFn>,
    pub write_file: WriteFileFn,
}

impl DeveloperFileIo {
    pub fn default_local() -> Self {
        let read_file: ReadFileFn = Arc::new(|path: PathBuf| {
            Box::pin(async move {
                let metadata = tokio::fs::metadata(&path).await?;
                let file_size = metadata.len();
                if file_size > MAX_READ_FILE_BYTES as u64 {
                    return Err(io::Error::other(format!(
                        "{} exceeds max file size of {} bytes (actual: {} bytes)",
                        path.display(),
                        MAX_READ_FILE_BYTES,
                        file_size
                    )));
                }
                tokio::fs::read_to_string(path).await
            })
        });
        let read_file_chunk: ReadFileChunkFn =
            Arc::new(|path: PathBuf, offset: usize, limit: usize| {
                Box::pin(async move {
                    let file = tokio::fs::File::open(path).await?;
                    let mut lines = tokio::io::BufReader::new(file).lines();
                    let mut line_index = 0usize;
                    let end = offset.saturating_add(limit);
                    let mut out = Vec::new();

                    while let Some(line) = lines.next_line().await? {
                        if line_index >= offset && line_index < end {
                            out.push(line);
                        }
                        line_index = line_index.saturating_add(1);
                        if line_index >= end {
                            break;
                        }
                    }

                    Ok(out.join("\n"))
                })
            });
        let write_file: WriteFileFn = Arc::new(|path: PathBuf, content: String| {
            Box::pin(async move {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                }
                tokio::fs::write(path, content).await
            })
        });
        Self {
            read_file,
            read_file_chunk: Some(read_file_chunk),
            write_file,
        }
    }
}

// These are used by integration tests in crates/goose/tests/
#[allow(unused_imports)]
pub use ext_manager::MANAGE_EXTENSIONS_TOOL_NAME;
#[allow(unused_imports)]
pub use ext_manager::SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME;

pub static PLATFORM_EXTENSIONS: Lazy<HashMap<&'static str, PlatformExtensionDef>> = Lazy::new(
    || {
        let mut map = HashMap::new();

        map.insert(
            analyze::EXTENSION_NAME,
            PlatformExtensionDef {
                name: analyze::EXTENSION_NAME,
                display_name: "Analyze",
                description:
                    "Analyze code structure with tree-sitter: directory overviews, file details, symbol call graphs",
                default_enabled: true,
                unprefixed_tools: true,
                client_factory: |ctx| Box::new(analyze::AnalyzeClient::new(ctx).unwrap()),
            },
        );

        map.insert(
            todo::EXTENSION_NAME,
            PlatformExtensionDef {
                name: todo::EXTENSION_NAME,
                display_name: "Todo",
                description:
                    "Enable a todo list for goose so it can keep track of what it is doing",
                default_enabled: true,
                unprefixed_tools: false,
                client_factory: |ctx| Box::new(todo::TodoClient::new(ctx).unwrap()),
            },
        );

        map.insert(
            apps::EXTENSION_NAME,
            PlatformExtensionDef {
                name: apps::EXTENSION_NAME,
                display_name: "Apps",
                description:
                    "Create and manage custom Goose apps through chat. Apps are HTML/CSS/JavaScript and run in sandboxed windows.",
                default_enabled: true,
                unprefixed_tools: false,
                client_factory: |ctx| Box::new(apps::AppsManagerClient::new(ctx).unwrap()),
            },
        );

        map.insert(
            chatrecall::EXTENSION_NAME,
            PlatformExtensionDef {
                name: chatrecall::EXTENSION_NAME,
                display_name: "Chat Recall",
                description:
                    "Search past conversations and load session summaries for contextual memory",
                default_enabled: false,
                unprefixed_tools: false,
                client_factory: |ctx| Box::new(chatrecall::ChatRecallClient::new(ctx).unwrap()),
            },
        );

        map.insert(
            "extensionmanager",
            PlatformExtensionDef {
                name: ext_manager::EXTENSION_NAME,
                display_name: "Extension Manager",
                description:
                    "Enable extension management tools for discovering, enabling, and disabling extensions",
                default_enabled: true,
                unprefixed_tools: false,
                client_factory: |ctx| Box::new(ext_manager::ExtensionManagerClient::new(ctx).unwrap()),
            },
        );

        map.insert(
            summon::EXTENSION_NAME,
            PlatformExtensionDef {
                name: summon::EXTENSION_NAME,
                display_name: "Summon",
                description: "Load knowledge and delegate tasks to subagents",
                default_enabled: true,
                unprefixed_tools: true,
                client_factory: |ctx| Box::new(summon::SummonClient::new(ctx).unwrap()),
            },
        );

        #[cfg(feature = "code-mode")]
        map.insert(
            code_execution::EXTENSION_NAME,
            PlatformExtensionDef {
                name: code_execution::EXTENSION_NAME,
                display_name: "Code Mode",
                description:
                    "Goose will make extension calls through code execution, saving tokens",
                default_enabled: false,
                unprefixed_tools: true,
                client_factory: |ctx| {
                    Box::new(code_execution::CodeExecutionClient::new(ctx).unwrap())
                },
            },
        );

        map.insert(
            developer::EXTENSION_NAME,
            PlatformExtensionDef {
                name: developer::EXTENSION_NAME,
                display_name: "Developer",
                description: "Write and edit files, and execute shell commands",
                default_enabled: true,
                unprefixed_tools: true,
                client_factory: |ctx| Box::new(developer::DeveloperClient::new(ctx).unwrap()),
            },
        );

        map.insert(
            tom::EXTENSION_NAME,
            PlatformExtensionDef {
                name: tom::EXTENSION_NAME,
                display_name: "Top Of Mind",
                description:
                    "Inject custom context into every turn via GOOSE_MOIM_MESSAGE_TEXT and GOOSE_MOIM_MESSAGE_FILE environment variables",
                default_enabled: true,
                unprefixed_tools: false,
                client_factory: |ctx| Box::new(tom::TomClient::new(ctx).unwrap()),
            },
        );

        map
    },
);

#[derive(Clone)]
pub struct PlatformExtensionContext {
    pub extension_manager:
        Option<std::sync::Weak<crate::agents::extension_manager::ExtensionManager>>,
    pub session_manager: std::sync::Arc<crate::session::SessionManager>,
    pub session: Option<std::sync::Arc<Session>>,
    pub developer_file_io: Option<DeveloperFileIo>,
}

impl PlatformExtensionContext {
    pub fn result_with_platform_notification(
        &self,
        mut result: rmcp::model::CallToolResult,
        extension_name: impl Into<String>,
        event_type: impl Into<String>,
        mut additional_params: serde_json::Map<String, serde_json::Value>,
    ) -> rmcp::model::CallToolResult {
        additional_params.insert("extension".to_string(), extension_name.into().into());
        additional_params.insert("event_type".to_string(), event_type.into().into());

        let meta_value = serde_json::json!({
            "platform_notification": {
                "method": "platform_event",
                "params": additional_params
            }
        });

        if let Some(ref mut meta) = result.meta {
            if let Some(obj) = meta_value.as_object() {
                for (k, v) in obj {
                    meta.0.insert(k.clone(), v.clone());
                }
            }
        } else {
            result.meta = Some(rmcp::model::Meta(meta_value.as_object().unwrap().clone()));
        }

        result
    }
}

/// Definition for a platform extension that runs in-process with direct agent access.
#[derive(Debug, Clone)]
pub struct PlatformExtensionDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub default_enabled: bool,
    /// If true, tools are exposed without extension prefix for intuitive first-class use.
    pub unprefixed_tools: bool,
    pub client_factory: fn(PlatformExtensionContext) -> Box<dyn McpClientTrait>,
}
