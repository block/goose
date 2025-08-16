use crate::agents::extension_manager::ExtensionManager;
use crate::agents::router_tool_selector::{
    create_tool_selector, RouterToolSelectionStrategy, RouterToolSelector,
};
use crate::agents::router_tools::{self};
use crate::agents::tool_execution::ToolCallResult;
use crate::agents::tool_router_index_manager::ToolRouterIndexManager;
#[cfg(feature = "tool_vectordb")]
use crate::agents::tool_vectordb::generate_table_id;
use crate::config::Config;
use crate::conversation::message::ToolRequest;
use crate::providers::base::Provider;
use anyhow::{anyhow, Result};
use rmcp::model::{ErrorCode, ErrorData, Tool};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tracing::error;

pub struct ToolRouteManager {
    router_tool_selector: Mutex<Option<Arc<Box<dyn RouterToolSelector>>>>,
    router_disabled_override: Mutex<bool>,
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
        if let Some(selector) = selector {
            for request in requests {
                if let Ok(tool_call) = &request.tool_call {
                    if let Err(e) = selector.record_tool_call(&tool_call.name).await {
                        error!("Failed to record tool call: {}", e);
                    }
                }
            }
        }
    }

    pub async fn dispatch_route_search_tool(
        &self,
        arguments: Value,
    ) -> Result<ToolCallResult, ErrorData> {
        let selector = self.router_tool_selector.lock().await.clone();
        match selector.as_ref() {
            Some(selector) => match selector.select_tools(arguments).await {
                Ok(tools) => Ok(ToolCallResult::from(Ok(tools))),
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

    pub async fn get_router_tool_selection_strategy(&self) -> Option<RouterToolSelectionStrategy> {
        if *self.router_disabled_override.lock().await {
            return None;
        }

        let config = Config::global();
        let router_tool_selection_strategy = config
            .get_param("GOOSE_ROUTER_TOOL_SELECTION_STRATEGY")
            .unwrap_or_else(|_| "default".to_string());

        match router_tool_selection_strategy.to_lowercase().as_str() {
            "vector" => Some(RouterToolSelectionStrategy::Vector),
            "llm" => Some(RouterToolSelectionStrategy::Llm),
            _ => None,
        }
    }

    pub async fn update_router_tool_selector(
        &self,
        provider: Arc<dyn Provider>,
        reindex_all: Option<bool>,
        extension_manager: &Arc<RwLock<ExtensionManager>>,
    ) -> Result<()> {
        let strategy = self.get_router_tool_selection_strategy().await;
        let selector = match strategy {
            Some(RouterToolSelectionStrategy::Vector) => {
                #[cfg(feature = "tool_vectordb")]
                {
                    let table_name = generate_table_id();
                    let selector =
                        create_tool_selector(strategy, provider.clone(), Some(table_name))
                            .await
                            .map_err(|e| anyhow!("Failed to create tool selector: {}", e))?;
                    Arc::new(selector)
                }
                #[cfg(not(feature = "tool_vectordb"))]
                {
                    return Err(anyhow!(
                        "Vector tool selection is not enabled. Enable 'tool_vectordb' feature."
                    ));
                }
            }
            Some(RouterToolSelectionStrategy::Llm) => {
                let selector = create_tool_selector(strategy, provider.clone(), None)
                    .await
                    .map_err(|e| anyhow!("Failed to create tool selector: {}", e))?;
                Arc::new(selector)
            }
            None => return Ok(()),
        };

        // First index platform tools
        let extension_manager = extension_manager.read().await;
        ToolRouterIndexManager::index_platform_tools(&selector, &extension_manager).await?;

        if reindex_all.unwrap_or(false) {
            let enabled_extensions = extension_manager.list_extensions().await?;
            for extension_name in enabled_extensions {
                if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                    &selector,
                    &extension_manager,
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
        *self.router_tool_selector.lock().await = Some(selector.clone());

        Ok(())
    }

    pub async fn get_router_tool_selector(&self) -> Option<Arc<Box<dyn RouterToolSelector>>> {
        self.router_tool_selector.lock().await.clone()
    }

    pub async fn list_tools_for_router(
        &self,
        strategy: Option<RouterToolSelectionStrategy>,
        extension_manager: &Arc<RwLock<ExtensionManager>>,
    ) -> Vec<Tool> {
        if *self.router_disabled_override.lock().await {
            return vec![];
        }

        let mut prefixed_tools = vec![];
        match strategy {
            Some(RouterToolSelectionStrategy::Vector) => {
                prefixed_tools.push(router_tools::vector_search_tool());
            }
            Some(RouterToolSelectionStrategy::Llm) => {
                prefixed_tools.push(router_tools::llm_search_tool());
            }
            None => {}
        }

        // Get recent tool calls from router tool selector if available
        let selector = self.router_tool_selector.lock().await.clone();
        if let Some(selector) = selector {
            if let Ok(recent_calls) = selector.get_recent_tool_calls(20).await {
                let extension_manager = extension_manager.read().await;
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
