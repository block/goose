pub mod apps;
pub mod chatrecall;
pub mod code_execution;
pub mod ext_manager;
pub mod summon;
pub mod todo;
pub mod tom;

use std::collections::HashMap;

use crate::agents::mcp_client::McpClientTrait;
use once_cell::sync::Lazy;

pub use ext_manager::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;

// These are used by integration tests in crates/goose/tests/
#[allow(unused_imports)]
pub use ext_manager::MANAGE_EXTENSIONS_TOOL_NAME;
#[allow(unused_imports)]
pub use ext_manager::SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME;

pub static PLATFORM_EXTENSIONS: Lazy<HashMap<&'static str, PlatformExtensionDef>> = Lazy::new(
    || {
        let mut map = HashMap::new();

        map.insert(
            todo::EXTENSION_NAME,
            PlatformExtensionDef {
                name: todo::EXTENSION_NAME,
                display_name: "Todo",
                description:
                    "Enable a todo list for goose so it can keep track of what it is doing",
                default_enabled: true,
                unprefixed_tools: false,
                client_factory: |ctx| {
                    todo::TodoClient::new(ctx)
                        .ok()
                        .map(|client| Box::new(client) as Box<dyn McpClientTrait>)
                },
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
                client_factory: |ctx| {
                    apps::AppsManagerClient::new(ctx)
                        .ok()
                        .map(|client| Box::new(client) as Box<dyn McpClientTrait>)
                },
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
                client_factory: |ctx| {
                    chatrecall::ChatRecallClient::new(ctx)
                        .ok()
                        .map(|client| Box::new(client) as Box<dyn McpClientTrait>)
                },
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
                client_factory: |ctx| {
                    ext_manager::ExtensionManagerClient::new(ctx)
                        .ok()
                        .map(|client| Box::new(client) as Box<dyn McpClientTrait>)
                },
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
                client_factory: |ctx| {
                    summon::SummonClient::new(ctx)
                        .ok()
                        .map(|client| Box::new(client) as Box<dyn McpClientTrait>)
                },
            },
        );

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
                    code_execution::CodeExecutionClient::new(ctx)
                        .ok()
                        .map(|client| Box::new(client) as Box<dyn McpClientTrait>)
                },
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
                client_factory: |ctx| {
                    tom::TomClient::new(ctx)
                        .ok()
                        .map(|client| Box::new(client) as Box<dyn McpClientTrait>)
                },
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
    pub provider: crate::agents::types::SharedProvider,
}

impl PlatformExtensionContext {
    pub fn get_context_limit(&self) -> Option<usize> {
        if let Ok(provider_guard) = self.provider.try_lock() {
            if let Some(provider) = provider_guard.as_ref() {
                return Some(provider.get_model_config().context_limit());
            }
        }
        None
    }

    pub fn require_min_context(
        &self,
        min_context: usize,
        extension_name: &str,
    ) -> anyhow::Result<()> {
        if let Some(context_limit) = self.get_context_limit() {
            if context_limit < min_context {
                return Err(anyhow::anyhow!(
                    "{} extension requires >= {}K context (current: {})",
                    extension_name,
                    min_context / 1000,
                    context_limit
                ));
            }
        }
        Ok(())
    }

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
    pub client_factory: fn(PlatformExtensionContext) -> Option<Box<dyn McpClientTrait>>,
}
