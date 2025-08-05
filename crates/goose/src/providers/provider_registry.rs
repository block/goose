use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use super::base::{Provider, ProviderMetadata};
use crate::model::ModelConfig;

type ProviderConstructor = Box<dyn Fn(ModelConfig) -> Result<Arc<dyn Provider>> + Send + Sync>;

struct ProviderEntry {
    metadata: ProviderMetadata,
    constructor: ProviderConstructor,
}

pub struct ProviderRegistry {
    entries: HashMap<String, ProviderEntry>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn register<P, F>(&mut self, constructor: F)
    where
        P: Provider + 'static,
        F: Fn(ModelConfig) -> Result<P> + Send + Sync + 'static,
    {
        let metadata = P::metadata();
        let name = metadata.name.clone();

        self.entries.insert(
            name,
            ProviderEntry {
                metadata,
                constructor: Box::new(move |model| Ok(Arc::new(constructor(model)?))),
            },
        );
    }

    pub fn create(&self, name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))?;

        (entry.constructor)(model)
    }

    pub fn all_metadata(&self) -> Vec<ProviderMetadata> {
        self.entries.values().map(|e| e.metadata.clone()).collect()
    }
}
