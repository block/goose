/// Tool Registry — owns tool caching, filtering, and resolution.
///
/// Extracted from `ExtensionManager` to separate tool lifecycle concerns
/// from extension lifecycle concerns. Phase A of the extension-agent separation.
///
/// # Design
///
/// ToolRegistry does NOT own extensions or MCP connections.
/// It provides a **cache-and-filter layer** on top of a tool source.
/// In Phase A, `ExtensionManager` populates the cache; in Phase B,
/// `ExtensionRegistry` will be the source.
///
/// ```text
/// ExtensionManager (extension lifecycle, fetches tools from MCP)
///   │
///   └─▶ ToolRegistry (caching, filtering, resolution)
///         ├─ tools_cache: Mutex<Option<Arc<Vec<Tool>>>>
///         └─ cache_version: AtomicU64
/// ```
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rmcp::model::Tool;
use tokio::sync::Mutex;

use crate::agents::extension_manager::get_tool_owner;
use crate::agents::tool_filter::filter_tools;
use crate::config::extensions::name_to_key;
use crate::registry::manifest::ToolGroupAccess;

/// Caches, filters, and resolves tools fetched from extensions.
pub struct ToolRegistry {
    tools_cache: Mutex<Option<Arc<Vec<Tool>>>>,
    cache_version: AtomicU64,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools_cache: Mutex::new(None),
            cache_version: AtomicU64::new(0),
        }
    }

    /// Get cached tools, or None if cache is empty/invalidated.
    pub async fn get_cached(&self) -> Option<Arc<Vec<Tool>>> {
        let cache = self.tools_cache.lock().await;
        cache.clone()
    }

    /// Set the cache with freshly fetched tools.
    pub async fn set_cache(&self, tools: Vec<Tool>) {
        let mut cache = self.tools_cache.lock().await;
        *cache = Some(Arc::new(tools));
    }

    /// Invalidate the cache, forcing a re-fetch on next access.
    pub async fn invalidate(&self) {
        self.cache_version.fetch_add(1, Ordering::Release);
        let mut cache = self.tools_cache.lock().await;
        *cache = None;
    }

    /// Get the current cache version (for change detection).
    pub fn version(&self) -> u64 {
        self.cache_version.load(Ordering::Acquire)
    }

    /// Filter tools by tool groups.
    ///
    /// Delegates to the existing `tool_filter::filter_tools` function.
    /// This is a convenience wrapper that applies filtering to the cached tools.
    pub fn filter_by_groups(tools: Vec<Tool>, groups: &[ToolGroupAccess]) -> Vec<Tool> {
        filter_tools(tools, groups)
    }

    /// Filter tools by allowed extension names.
    pub fn filter_by_extensions(tools: &[Tool], allowed: &[String]) -> Vec<Tool> {
        let allowed_keys: Vec<String> = allowed.iter().map(|e| name_to_key(e)).collect();
        tools
            .iter()
            .filter(|tool| {
                let owner_key = tool_owner_key(tool);
                allowed_keys.contains(&owner_key)
            })
            .cloned()
            .collect()
    }

    /// Exclude tools from certain extensions.
    pub fn exclude_extensions(tools: &[Tool], excluded: &[String]) -> Vec<Tool> {
        let excluded_keys: Vec<String> = excluded.iter().map(|e| name_to_key(e)).collect();
        tools
            .iter()
            .filter(|tool| {
                let owner_key = tool_owner_key(tool);
                !excluded_keys.contains(&owner_key)
            })
            .cloned()
            .collect()
    }

    /// Find which extension owns a given tool by name.
    ///
    /// Checks namespace prefix first (e.g., `developer__shell` → "developer"),
    /// then falls back to metadata lookup in the provided tool list.
    pub fn resolve_owner(tool_name: &str, tools: &[Tool]) -> Option<String> {
        // Fast path: namespace prefix
        if let Some((prefix, _)) = tool_name.split_once("__") {
            return Some(name_to_key(prefix));
        }

        // Slow path: search tool metadata
        tools
            .iter()
            .find(|t| t.name.as_ref() == tool_name)
            .and_then(get_tool_owner)
            .map(|o| name_to_key(&o))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the normalized owner key for a tool.
fn tool_owner_key(tool: &Tool) -> String {
    get_tool_owner(tool)
        .map(|o| name_to_key(&o))
        .unwrap_or_else(|| {
            tool.name
                .as_ref()
                .split("__")
                .next()
                .unwrap_or("")
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str, ext_name: &str) -> Tool {
        let schema: std::sync::Arc<serde_json::Map<String, serde_json::Value>> =
            std::sync::Arc::new(serde_json::Map::new());
        let mut tool = Tool::new(name.to_string(), format!("Test tool {name}"), schema);
        let meta_val = serde_json::json!({ "goose_extension": ext_name });
        let meta_map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(meta_val).unwrap();
        tool.meta = Some(rmcp::model::Meta(meta_map));
        tool
    }

    #[test]
    fn test_filter_by_extensions() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("memory__search", "memory"),
            make_tool("fetch__fetch", "fetch"),
        ];
        let allowed = vec!["developer".to_string(), "fetch".to_string()];
        let result = ToolRegistry::filter_by_extensions(&tools, &allowed);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|t| t.name.as_ref() == "developer__shell"));
        assert!(result.iter().any(|t| t.name.as_ref() == "fetch__fetch"));
    }

    #[test]
    fn test_exclude_extensions() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("memory__search", "memory"),
        ];
        let excluded = vec!["memory".to_string()];
        let result = ToolRegistry::exclude_extensions(&tools, &excluded);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name.as_ref(), "developer__shell");
    }

    #[test]
    fn test_resolve_owner_by_prefix() {
        let tools: Vec<Tool> = vec![];
        assert_eq!(
            ToolRegistry::resolve_owner("developer__shell", &tools),
            Some("developer".to_string())
        );
    }

    #[test]
    fn test_resolve_owner_by_metadata() {
        let tools = vec![make_tool("shell", "developer")];
        assert_eq!(
            ToolRegistry::resolve_owner("shell", &tools),
            Some("developer".to_string())
        );
    }

    #[test]
    fn test_resolve_owner_not_found() {
        let tools: Vec<Tool> = vec![];
        assert_eq!(ToolRegistry::resolve_owner("nonexistent", &tools), None);
    }

    #[test]
    fn test_cache_version_increments() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.version(), 0);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { registry.invalidate().await });
        assert_eq!(registry.version(), 1);
    }

    #[tokio::test]
    async fn test_cache_lifecycle() {
        let registry = ToolRegistry::new();

        // Initially empty
        let empty: Option<Arc<Vec<Tool>>> = registry.get_cached().await;
        assert!(empty.is_none());

        // Set cache
        let tools = vec![make_tool("developer__shell", "developer")];
        registry.set_cache(tools).await;

        // Retrieve
        let cached: Option<Arc<Vec<Tool>>> = registry.get_cached().await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);

        // Invalidate
        registry.invalidate().await;
        let after_invalidate: Option<Arc<Vec<Tool>>> = registry.get_cached().await;
        assert!(after_invalidate.is_none());
        assert_eq!(registry.version(), 1);
    }

    #[test]
    fn test_default() {
        let registry = ToolRegistry::default();
        assert_eq!(registry.version(), 0);
    }
}
