use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing;

use crate::agents::extension_manager::ExtensionManager;
use crate::agents::router_tool_selector::RouterToolSelector;

/// Manages tool indexing operations for the router when LLM routing is enabled
pub struct ToolRouterIndexManager;

impl ToolRouterIndexManager {
    /// Updates the LLM index for tools when extensions are added or removed
    pub async fn update_extension_tools(
        selector: &Arc<Box<dyn RouterToolSelector>>,
        extension_manager: &ExtensionManager,
        extension_name: &str,
        action: &str,
    ) -> Result<()> {
        match action {
            "add" => {
                // Get tools for specific extension
                let tools = extension_manager
                    .get_prefixed_tools(Some(extension_name.to_string()))
                    .await?;

                if !tools.is_empty() {
                    // Index all tools at once
                    selector
                        .index_tools(&tools, extension_name)
                        .await
                        .map_err(|e| {
                            anyhow!(
                                "Failed to index tools for extension {}: {}",
                                extension_name,
                                e
                            )
                        })?;

                    tracing::info!(
                        "Indexed {} tools for extension {}",
                        tools.len(),
                        extension_name
                    );
                }
            }
            "remove" => {
                // Remove all tools for this extension
                let tools = extension_manager
                    .get_prefixed_tools(Some(extension_name.to_string()))
                    .await?;

                for tool in &tools {
                    selector.remove_tool(&tool.name).await.map_err(|e| {
                        anyhow!(
                            "Failed to remove tool {} for extension {}: {}",
                            tool.name,
                            extension_name,
                            e
                        )
                    })?;
                }

                tracing::info!(
                    "Removed {} tools for extension {}",
                    tools.len(),
                    extension_name
                );
            }
            _ => {
                return Err(anyhow!("Invalid action: {}", action));
            }
        }

        Ok(())
    }
}
