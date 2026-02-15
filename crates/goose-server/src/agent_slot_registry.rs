use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory registry tracking which builtin agents are enabled
/// and which extensions are bound to each agent.
#[derive(Clone)]
pub struct AgentSlotRegistry {
    enabled_agents: Arc<RwLock<HashMap<String, bool>>>,
    bound_extensions: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl Default for AgentSlotRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentSlotRegistry {
    pub fn new() -> Self {
        let mut enabled = HashMap::new();
        enabled.insert("Goose Agent".to_string(), true);
        enabled.insert("Coding Agent".to_string(), true);

        Self {
            enabled_agents: Arc::new(RwLock::new(enabled)),
            bound_extensions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn is_enabled(&self, name: &str) -> bool {
        self.enabled_agents
            .read()
            .await
            .get(name)
            .copied()
            .unwrap_or(true)
    }

    pub async fn toggle(&self, name: &str) -> bool {
        let mut agents = self.enabled_agents.write().await;
        let current = agents.get(name).copied().unwrap_or(true);
        let new_state = !current;
        agents.insert(name.to_string(), new_state);
        new_state
    }

    pub async fn get_bound_extensions(&self, name: &str) -> HashSet<String> {
        self.bound_extensions
            .read()
            .await
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn bind_extension(&self, agent_name: &str, extension_name: &str) {
        self.bound_extensions
            .write()
            .await
            .entry(agent_name.to_string())
            .or_default()
            .insert(extension_name.to_string());
    }

    pub async fn unbind_extension(&self, agent_name: &str, extension_name: &str) {
        let mut bindings = self.bound_extensions.write().await;
        if let Some(exts) = bindings.get_mut(agent_name) {
            exts.remove(extension_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_toggle_agent() {
        let registry = AgentSlotRegistry::new();
        assert!(registry.is_enabled("Goose Agent").await);
        let new_state = registry.toggle("Goose Agent").await;
        assert!(!new_state);
        assert!(!registry.is_enabled("Goose Agent").await);
        let new_state = registry.toggle("Goose Agent").await;
        assert!(new_state);
    }

    #[tokio::test]
    async fn test_bind_unbind_extension() {
        let registry = AgentSlotRegistry::new();
        assert!(registry
            .get_bound_extensions("Goose Agent")
            .await
            .is_empty());
        registry.bind_extension("Goose Agent", "developer").await;
        registry.bind_extension("Goose Agent", "memory").await;
        let exts = registry.get_bound_extensions("Goose Agent").await;
        assert_eq!(exts.len(), 2);
        assert!(exts.contains("developer"));
        registry.unbind_extension("Goose Agent", "developer").await;
        let exts = registry.get_bound_extensions("Goose Agent").await;
        assert_eq!(exts.len(), 1);
    }
}
