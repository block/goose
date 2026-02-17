use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory registry tracking which builtin agents are enabled
/// and which extensions are bound to each agent.
#[derive(Clone)]
pub struct AgentSlotRegistry {
    enabled_agents: Arc<RwLock<HashMap<String, bool>>>,
    bound_extensions: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    delegation_strategies: Arc<RwLock<HashMap<String, SlotDelegation>>>,
}

/// How a particular agent slot should be executed.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SlotDelegation {
    /// Execute in-process via the local provider (builtin agents).
    InProcess,
    /// Execute via an external ACP agent process.
    ExternalAcp,
    /// Execute via a remote A2A agent over HTTP.
    RemoteA2A { url: String },
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
        enabled.insert("Developer Agent".to_string(), true);

        let mut strategies = HashMap::new();
        strategies.insert("Goose Agent".to_string(), SlotDelegation::InProcess);
        strategies.insert("Developer Agent".to_string(), SlotDelegation::InProcess);

        Self {
            enabled_agents: Arc::new(RwLock::new(enabled)),
            bound_extensions: Arc::new(RwLock::new(HashMap::new())),
            delegation_strategies: Arc::new(RwLock::new(strategies)),
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

    #[allow(dead_code)]
    pub async fn get_delegation(&self, name: &str) -> SlotDelegation {
        self.delegation_strategies
            .read()
            .await
            .get(name)
            .cloned()
            .unwrap_or(SlotDelegation::InProcess)
    }

    #[allow(dead_code)]
    pub async fn set_delegation(&self, name: &str, delegation: SlotDelegation) {
        self.delegation_strategies
            .write()
            .await
            .insert(name.to_string(), delegation);
    }

    #[allow(dead_code)]
    pub async fn register_a2a_agent(&self, name: &str, url: &str) {
        self.enabled_agents
            .write()
            .await
            .insert(name.to_string(), true);
        self.delegation_strategies.write().await.insert(
            name.to_string(),
            SlotDelegation::RemoteA2A {
                url: url.to_string(),
            },
        );
    }

    pub async fn register_acp_agent(&self, name: &str) {
        self.enabled_agents
            .write()
            .await
            .insert(name.to_string(), true);
        self.delegation_strategies
            .write()
            .await
            .insert(name.to_string(), SlotDelegation::ExternalAcp);
    }

    pub async fn unregister_agent(&self, name: &str) {
        self.enabled_agents.write().await.remove(name);
        self.delegation_strategies.write().await.remove(name);
        self.bound_extensions.write().await.remove(name);
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

    #[tokio::test]
    async fn test_delegation_defaults_to_in_process() {
        let registry = AgentSlotRegistry::new();
        assert_eq!(
            registry.get_delegation("Goose Agent").await,
            SlotDelegation::InProcess
        );
        assert_eq!(
            registry.get_delegation("Unknown Agent").await,
            SlotDelegation::InProcess
        );
    }

    #[tokio::test]
    async fn test_register_a2a_agent() {
        let registry = AgentSlotRegistry::new();
        registry
            .register_a2a_agent("Remote Agent", "https://remote.example.com/a2a")
            .await;
        assert!(registry.is_enabled("Remote Agent").await);
        assert_eq!(
            registry.get_delegation("Remote Agent").await,
            SlotDelegation::RemoteA2A {
                url: "https://remote.example.com/a2a".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_register_acp_agent() {
        let registry = AgentSlotRegistry::new();
        registry.register_acp_agent("ACP Agent").await;
        assert!(registry.is_enabled("ACP Agent").await);
        assert_eq!(
            registry.get_delegation("ACP Agent").await,
            SlotDelegation::ExternalAcp
        );
    }

    #[tokio::test]
    async fn test_unregister_agent() {
        let registry = AgentSlotRegistry::new();
        registry
            .register_a2a_agent("Temp Agent", "https://example.com")
            .await;
        registry.bind_extension("Temp Agent", "developer").await;
        assert_eq!(
            registry.get_delegation("Temp Agent").await,
            SlotDelegation::RemoteA2A {
                url: "https://example.com".to_string()
            }
        );

        registry.unregister_agent("Temp Agent").await;
        // Delegation falls back to InProcess after removal
        assert_eq!(
            registry.get_delegation("Temp Agent").await,
            SlotDelegation::InProcess
        );
        assert!(registry.get_bound_extensions("Temp Agent").await.is_empty());
    }
}
