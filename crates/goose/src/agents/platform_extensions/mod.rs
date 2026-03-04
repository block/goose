pub mod analyze;
pub mod apps;
pub mod autovisualiser;
pub mod chatrecall;
#[cfg(feature = "code-mode")]
pub mod code_execution;
pub mod computercontroller;
pub mod developer;
pub mod ext_manager;
pub mod memory;
#[cfg(target_os = "macos")]
pub mod peekaboo;
pub mod summon;
pub mod todo;
pub mod tom;
pub mod tutorial;

use std::collections::HashMap;

use crate::agents::mcp_client::McpClientTrait;
use crate::session::Session;
use etcetera::AppStrategyArgs;
use once_cell::sync::Lazy;
use rmcp::ServerHandler;

pub use ext_manager::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;

// These are used by integration tests in crates/goose/tests/
#[allow(unused_imports)]
pub use ext_manager::MANAGE_EXTENSIONS_TOOL_NAME;
#[allow(unused_imports)]
pub use ext_manager::SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

/// Type definition for a function that spawns and serves a builtin MCP server in-process
pub type SpawnServerFn = fn(tokio::io::DuplexStream, tokio::io::DuplexStream);

fn spawn_and_serve<S>(
    name: &'static str,
    server: S,
    transport: (tokio::io::DuplexStream, tokio::io::DuplexStream),
) where
    S: ServerHandler + Send + 'static,
{
    use rmcp::ServiceExt;
    tokio::spawn(async move {
        match server.serve(transport).await {
            Ok(running) => {
                let _ = running.waiting().await;
            }
            Err(e) => tracing::error!(builtin = name, error = %e, "server error"),
        }
    });
}

macro_rules! mcp_server {
    ($name:ident, $server_ty:ty) => {{
        fn spawn(r: tokio::io::DuplexStream, w: tokio::io::DuplexStream) {
            spawn_and_serve(stringify!($name), <$server_ty>::new(), (r, w));
        }
        Some(spawn as SpawnServerFn)
    }};
}

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
                default_enabled: false,
                unprefixed_tools: true,
                client_factory: Some(|ctx| Box::new(analyze::AnalyzeClient::new(ctx).unwrap())),
                mcp_server_factory: None,
            },
        );

        map.insert(
            todo::EXTENSION_NAME,
            PlatformExtensionDef {
                name: todo::EXTENSION_NAME,
                display_name: "Todo",
                description:
                    "Enable a todo list for goose so it can keep track of what it is doing",
                default_enabled: false,
                unprefixed_tools: false,
                client_factory: Some(|ctx| Box::new(todo::TodoClient::new(ctx).unwrap())),
                mcp_server_factory: None,
            },
        );

        map.insert(
            apps::EXTENSION_NAME,
            PlatformExtensionDef {
                name: apps::EXTENSION_NAME,
                display_name: "Apps",
                description:
                    "Create and manage custom Goose apps through chat. Apps are HTML/CSS/JavaScript and run in sandboxed windows.",
                default_enabled: false,
                unprefixed_tools: false,
                client_factory: Some(|ctx| Box::new(apps::AppsManagerClient::new(ctx).unwrap())),
                mcp_server_factory: None,
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
                client_factory: Some(|ctx| {
                    Box::new(chatrecall::ChatRecallClient::new(ctx).unwrap())
                }),
                mcp_server_factory: None,
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
                client_factory: Some(|ctx| Box::new(ext_manager::ExtensionManagerClient::new(ctx).unwrap())),
                mcp_server_factory: None,
            },
        );

        map.insert(
            summon::EXTENSION_NAME,
            PlatformExtensionDef {
                name: summon::EXTENSION_NAME,
                display_name: "Summon",
                description: "Load knowledge and delegate tasks to subagents",
                default_enabled: false,
                unprefixed_tools: true,
                client_factory: Some(|ctx| Box::new(summon::SummonClient::new(ctx).unwrap())),
                mcp_server_factory: None,
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
                client_factory: Some(|ctx| {
                    Box::new(code_execution::CodeExecutionClient::new(ctx).unwrap())
                }),
                mcp_server_factory: None,
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
                client_factory: Some(|ctx| Box::new(developer::DeveloperClient::new(ctx).unwrap())),
                mcp_server_factory: None,
            },
        );

        map.insert(
            tom::EXTENSION_NAME,
            PlatformExtensionDef {
                name: tom::EXTENSION_NAME,
                display_name: "Top Of Mind",
                description:
                    "Inject custom context into every turn via GOOSE_MOIM_MESSAGE_TEXT and GOOSE_MOIM_MESSAGE_FILE environment variables",
                default_enabled: false,
                unprefixed_tools: false,
                client_factory: Some(|ctx| Box::new(tom::TomClient::new(ctx).unwrap())),
                mcp_server_factory: None,
            },
        );

        map.insert(
            "autovisualiser",
            PlatformExtensionDef {
                name: "autovisualiser",
                display_name: "Auto Visualiser",
                description: "Automatic data visualization with charts, maps, and diagrams",
                default_enabled: false,
                unprefixed_tools: false,
                client_factory: None,
                mcp_server_factory: mcp_server!(
                    autovisualiser,
                    autovisualiser::AutoVisualiserRouter
                ),
            },
        );

        map.insert(
            "computercontroller",
            PlatformExtensionDef {
                name: "computercontroller",
                display_name: "Computer Controller",
                description: "Control the computer: screenshots, web scraping, file conversion, and system automation",
                default_enabled: true,
                unprefixed_tools: false,
                client_factory: None,
                mcp_server_factory: mcp_server!(computercontroller, computercontroller::ComputerControllerServer),
            },
        );

        map.insert(
            "memory",
            PlatformExtensionDef {
                name: "memory",
                display_name: "Memory",
                description: "Remember and retrieve information across sessions",
                default_enabled: false,
                unprefixed_tools: false,
                client_factory: None,
                mcp_server_factory: mcp_server!(memory, memory::MemoryServer),
            },
        );

        map.insert(
            "tutorial",
            PlatformExtensionDef {
                name: "tutorial",
                display_name: "Tutorial",
                description: "Interactive tutorials for learning to use goose",
                default_enabled: false,
                unprefixed_tools: false,
                client_factory: None,
                mcp_server_factory: mcp_server!(tutorial, tutorial::TutorialServer),
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

/// Definition for a platform extension that runs in-process.
///
/// Extensions come in two flavors:
/// - **Direct**: implement `McpClientTrait` directly via `client_factory`
/// - **MCP server**: implement `rmcp::ServerHandler` and are connected in-process
///   via duplex channels through `mcp_server_factory`
///
/// Exactly one of `client_factory` or `mcp_server_factory` must be `Some`.
#[derive(Debug, Clone)]
pub struct PlatformExtensionDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub default_enabled: bool,
    /// If true, tools are exposed without extension prefix for intuitive first-class use.
    pub unprefixed_tools: bool,
    /// Factory for extensions that implement McpClientTrait directly.
    pub client_factory: Option<fn(PlatformExtensionContext) -> Box<dyn McpClientTrait>>,
    /// Factory for extensions that implement rmcp::ServerHandler and communicate via MCP protocol over duplex channels.
    pub mcp_server_factory: Option<SpawnServerFn>,
}
