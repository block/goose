use crate::agents::extension_manager::ExtensionManager;
use crate::agents::router_tool_selector::{create_tool_selector, RouterToolSelector};
use crate::agents::router_tools::{self};
use crate::agents::tool_execution::ToolCallResult;
use crate::agents::tool_router_index_manager::ToolRouterIndexManager;
use crate::config::Config;
use crate::conversation::message::ToolRequest;
use crate::providers::base::Provider;
use anyhow::{anyhow, Result};
use rmcp::model::{ErrorCode, ErrorData, JsonObject, Tool};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

pub struct ToolRouteManager {
    router_tool_selector: Mutex<Option<Arc<Box<dyn RouterToolSelector>>>>,
    router_disabled_override: Mutex<bool>,
}

impl Default for ToolRouteManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRouteManager {
    pub fn new() -> Self {
        Self {
            router_tool_selector: Mutex::new(None),
            router_disabled_override: Mutex::new(false),
        }
    }

    pub async fn disable_router_for_recipe(&self) {
        *self.router_disabled_override.lock().await = true;
        *self.router_tool_selector.lock().await = None;
    }

    pub async fn record_tool_requests(&self, requests: &[ToolRequest]) {
        let selector = self.router_tool_selector.lock().await.clone();
        for request in requests {
            if let Ok(tool_call) = &request.tool_call {
                if let Some(ref selector) = selector {
                    if let Err(e) = selector.record_tool_call(&tool_call.name).await {
                        error!("Failed to record tool call: {}", e);
                    }
                }
            }
        }
    }

    pub async fn dispatch_route_search_tool(
        &self,
        arguments: JsonObject,
    ) -> Result<ToolCallResult, ErrorData> {
        let selector = self.router_tool_selector.lock().await.clone();
        match selector.as_ref() {
            Some(selector) => match selector.select_tools(arguments).await {
                Ok(content) => Ok(ToolCallResult::from(Ok(rmcp::model::CallToolResult {
                    content,
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }))),
                Err(e) => Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to select tools: {}", e),
                    None,
                )),
            },
            None => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "No tool selector available".to_string(),
                None,
            )),
        }
    }

    pub async fn is_router_enabled(&self) -> bool {
        if *self.router_disabled_override.lock().await {
            return false;
        }

        let config = Config::global();
        if let Ok(config_value) = config.get_param::<String>("GOOSE_ENABLE_ROUTER") {
            return config_value.to_lowercase() == "true";
        }

        // Default to false if neither is set
        false
    }

    pub async fn update_router_tool_selector(
        &self,
        provider: Arc<dyn Provider>,
        reindex_all: Option<bool>,
        extension_manager: &ExtensionManager,
    ) -> Result<()> {
        let enabled = self.is_router_enabled().await;
        if !enabled {
            return Ok(());
        }

        let selector = create_tool_selector(provider.clone())
            .await
            .map_err(|e| anyhow!("Failed to create tool selector: {}", e))?;

        // Wrap selector in Arc for the index manager methods
        let selector_arc = Arc::new(selector);

        if reindex_all.unwrap_or(false) {
            let enabled_extensions = extension_manager.list_extensions().await?;
            for extension_name in enabled_extensions {
                if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                    &selector_arc,
                    extension_manager,
                    &extension_name,
                    "add",
                )
                .await
                {
                    error!(
                        "Failed to index tools for extension {}: {}",
                        extension_name, e
                    );
                }
            }
        }

        // Update the selector
        *self.router_tool_selector.lock().await = Some(selector_arc);

        Ok(())
    }

    pub async fn get_router_tool_selector(&self) -> Option<Arc<Box<dyn RouterToolSelector>>> {
        self.router_tool_selector.lock().await.clone()
    }

    /// Check if the router is actually functional (enabled in config AND initialized)
    pub async fn is_router_functional(&self) -> bool {
        if !self.is_router_enabled().await {
            return false;
        }

        // Check if the selector actually exists (meaning it was successfully initialized)
        self.router_tool_selector.lock().await.is_some()
    }

    pub async fn list_tools_for_router(&self, extension_manager: &ExtensionManager) -> Vec<Tool> {
        // If router is disabled or overridden, return empty
        if *self.router_disabled_override.lock().await {
            return vec![];
        }

        let mut prefixed_tools = vec![];

        // If router is enabled but not functional (no provider), just return the search tool
        if !self.is_router_functional().await {
            return prefixed_tools;
        }
        prefixed_tools.push(router_tools::llm_search_tool());

        // Get recent tool calls from router tool selector
        let selector = self.router_tool_selector.lock().await.clone();
        if let Some(selector) = selector {
            if let Ok(recent_calls) = selector.get_recent_tool_calls(20).await {
                // Add recent tool calls to the list, avoiding duplicates
                for tool_name in recent_calls {
                    // Find the tool in the extension manager's tools
                    if let Ok(extension_tools) = extension_manager.get_prefixed_tools(None).await {
                        if let Some(tool) = extension_tools.iter().find(|t| t.name == tool_name) {
                            // Only add if not already in prefixed_tools
                            if !prefixed_tools.iter().any(|t| t.name == tool.name) {
                                prefixed_tools.push(tool.clone());
                            }
                        }
                    }
                }
            }
        }

        prefixed_tools
    }
}
