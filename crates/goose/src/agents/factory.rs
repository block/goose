use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

pub use super::Agent;
use crate::providers::base::Provider;

type AgentConstructor = Box<dyn Fn(Box<dyn Provider>) -> Box<dyn Agent> + Send + Sync>;

// Use std::sync::RwLock for interior mutability
static AGENT_REGISTRY: OnceLock<RwLock<HashMap<&'static str, AgentConstructor>>> = OnceLock::new();

/// Initialize the registry if it hasn't been initialized
fn registry() -> &'static RwLock<HashMap<&'static str, AgentConstructor>> {
    AGENT_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a new agent version
pub fn register_agent(
    version: &'static str,
    constructor: impl Fn(Box<dyn Provider>) -> Box<dyn Agent> + Send + Sync + 'static,
) {
    let registry = registry();
    if let Ok(mut map) = registry.write() {
        map.insert(version, Box::new(constructor));
    }
}

pub struct AgentFactory;

impl AgentFactory {
    /// Create a new agent instance of the specified version
    pub fn create(version: &str, provider: Box<dyn Provider>) -> Option<Box<dyn Agent>> {
        let registry = registry();
        let map = registry
            .read()
            .expect("should be able to read the registry");
        let constructor = map.get(version)?;
        Some(constructor(provider))
    }

    /// Get a list of all available agent versions
    pub fn available_versions() -> Vec<&'static str> {
        registry()
            .read()
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Get the default version name
    pub fn default_version() -> &'static str {
        "truncate"
    }
}

/// Macro to help with agent registration
#[macro_export]
macro_rules! register_agent {
    ($version:expr, $agent_type:ty) => {
        paste::paste! {
            #[ctor::ctor]
            #[allow(non_snake_case)]
            fn [<__register_agent_ $version>]() {
                $crate::agents::factory::register_agent($version, |provider| {
                    Box::new(<$agent_type>::new(provider))
                });
            }
        }
    };
}
