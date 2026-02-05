//! Enabled Extensions - Session-scoped active extensions
//!
//! Each session has its own EnabledExtensions instance that tracks
//! which extensions are currently active and owns their runtime state.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use rig::completion::ToolDefinition;
use serde_json::{Map, Value};
use tracing::{info, instrument, warn};

use super::catalog::ExtensionCatalog;
use super::config::ExtensionKind;
use super::{Extension, McpExtension};
use crate::{Error, Result};

/// Index mapping tool names to their owning extension
pub type ToolIndex = HashMap<String, String>;

/// Session-scoped collection of enabled extensions.
///
/// This owns the running extension instances and provides methods
/// to enable/disable extensions and route tool calls.
pub struct EnabledExtensions {
    /// Reference to the global catalog (for looking up extension configs)
    catalog: Arc<RwLock<ExtensionCatalog>>,

    /// Currently enabled extensions, keyed by name
    enabled: HashMap<String, Box<dyn Extension>>,
}

impl EnabledExtensions {
    /// Create a new EnabledExtensions with a reference to the global catalog
    pub fn new(catalog: Arc<RwLock<ExtensionCatalog>>) -> Self {
        Self {
            catalog,
            enabled: HashMap::new(),
        }
    }

    /// Enable an extension by name.
    ///
    /// Looks up the extension in the catalog and instantiates it.
    #[instrument(skip(self), fields(extension = %name))]
    pub async fn enable(&mut self, name: &str) -> Result<()> {
        // Check if already enabled
        if self.enabled.contains_key(name) {
            info!("Extension already enabled");
            return Ok(());
        }

        // Look up config in catalog
        let config = {
            let catalog = self.catalog.read();
            catalog.get(name).cloned().ok_or_else(|| {
                Error::Extension(format!("Extension '{}' not found in catalog", name))
            })?
        };

        // Instantiate based on kind
        let extension: Box<dyn Extension> = match &config.kind {
            ExtensionKind::Native => {
                // TODO: Factory for native extensions
                // For now, return an error - we'll implement this when we add native extensions
                return Err(Error::Extension(format!(
                    "Native extension '{}' not yet implemented",
                    name
                )));
            }
            ExtensionKind::Mcp { command, args, env } => {
                let ext = McpExtension::connect(name, &config.description, command, args, env).await?;
                Box::new(ext)
            }
        };

        info!("Extension enabled");
        self.enabled.insert(name.to_string(), extension);
        Ok(())
    }

    /// Disable an extension by name.
    #[instrument(skip(self), fields(extension = %name))]
    pub fn disable(&mut self, name: &str) -> Result<()> {
        if self.enabled.remove(name).is_some() {
            info!("Extension disabled");
            Ok(())
        } else {
            Err(Error::Extension(format!(
                "Extension '{}' is not enabled",
                name
            )))
        }
    }

    /// Check if an extension is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled.contains_key(name)
    }

    /// Get list of enabled extension names
    pub fn enabled_names(&self) -> Vec<String> {
        self.enabled.keys().cloned().collect()
    }

    /// Gather all tools from enabled extensions.
    ///
    /// Returns the tool definitions and an index mapping tool names to extensions.
    pub async fn gather_tools(&self) -> Result<(Vec<ToolDefinition>, ToolIndex)> {
        let mut all_tools = Vec::new();
        let mut tool_index = HashMap::new();

        for (ext_name, extension) in &self.enabled {
            match extension.list_tools().await {
                Ok(tools) => {
                    for tool in &tools {
                        tool_index.insert(tool.name.clone(), ext_name.clone());
                    }
                    all_tools.extend(tools);
                }
                Err(e) => {
                    warn!(extension = %ext_name, error = %e, "Failed to list tools");
                }
            }
        }

        info!(tool_count = all_tools.len(), "Gathered tools from extensions");
        Ok((all_tools, tool_index))
    }

    /// Call a tool, routing to the appropriate extension.
    pub async fn call_tool(
        &self,
        tool_index: &ToolIndex,
        tool_name: &str,
        arguments: Option<Map<String, Value>>,
    ) -> Result<String> {
        let ext_name = tool_index.get(tool_name).ok_or_else(|| {
            Error::Extension(format!("Tool '{}' not found in any extension", tool_name))
        })?;

        let extension = self.enabled.get(ext_name).ok_or_else(|| {
            Error::Extension(format!(
                "Extension '{}' for tool '{}' is not enabled",
                ext_name, tool_name
            ))
        })?;

        extension.call_tool(tool_name, arguments).await
    }

    /// Get an extension by name (for accessing instructions, etc.)
    pub fn get(&self, name: &str) -> Option<&dyn Extension> {
        self.enabled.get(name).map(|e| e.as_ref())
    }

    /// Iterate over enabled extensions
    pub fn iter(&self) -> impl Iterator<Item = (&str, &dyn Extension)> {
        self.enabled.iter().map(|(k, v)| (k.as_str(), v.as_ref()))
    }

    /// Get reference to the catalog
    pub fn catalog(&self) -> &Arc<RwLock<ExtensionCatalog>> {
        &self.catalog
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_enabled_extensions() {
        let catalog = Arc::new(RwLock::new(ExtensionCatalog::new()));
        let enabled = EnabledExtensions::new(catalog);
        assert!(enabled.enabled_names().is_empty());
    }

    #[tokio::test]
    async fn test_enable_nonexistent_extension() {
        let catalog = Arc::new(RwLock::new(ExtensionCatalog::new()));
        let mut enabled = EnabledExtensions::new(catalog);

        let result = enabled.enable("nonexistent").await;
        assert!(result.is_err());
    }
}
