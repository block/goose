use super::base::{ModelInfo, Provider, ProviderMetadata, ProviderType};
use crate::config::DeclarativeProviderConfig;
use crate::model::ModelConfig;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

type ProviderConstructor = Box<dyn Fn(ModelConfig) -> Result<Arc<dyn Provider>> + Send + Sync>;

struct ProviderEntry {
    metadata: ProviderMetadata,
    constructor: ProviderConstructor,
    provider_type: ProviderType,
}

#[derive(Default)]
pub struct ProviderRegistry {
    entries: HashMap<String, ProviderEntry>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn register<P, F>(&mut self, constructor: F, preferred: bool)
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
                provider_type: if preferred {
                    ProviderType::Preferred
                } else {
                    ProviderType::Builtin
                },
            },
        );
    }

    pub fn register_with_name<P, F>(
        &mut self,
        config: &DeclarativeProviderConfig,
        provider_type: ProviderType,
        constructor: F,
    ) where
        P: Provider + 'static,
        F: Fn(ModelConfig) -> Result<P> + Send + Sync + 'static,
    {
        let base_metadata = P::metadata();
        let description = config
            .description
            .clone()
            .unwrap_or_else(|| format!("Custom {} provider", config.display_name));
        let default_model = config
            .models
            .first()
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let known_models: Vec<ModelInfo> = config
            .models
            .iter()
            .map(|m| ModelInfo {
                name: m.name.clone(),
                context_limit: m.context_limit,
                input_token_cost: m.input_token_cost,
                output_token_cost: m.output_token_cost,
                currency: m.currency.clone(),
                supports_cache_control: Some(m.supports_cache_control.unwrap_or(false)),
            })
            .collect();

        let custom_metadata = ProviderMetadata {
            name: config.name.clone(),
            display_name: config.display_name.clone(),
            description,
            default_model,
            known_models,
            model_doc_link: base_metadata.model_doc_link,
            config_keys: base_metadata.config_keys,
        };

        self.entries.insert(
            config.name.clone(),
            ProviderEntry {
                metadata: custom_metadata,
                constructor: Box::new(move |model| Ok(Arc::new(constructor(model)?))),
                provider_type,
            },
        );
    }

    pub fn with_providers<F>(mut self, setup: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        setup(&mut self);
        self
    }

    pub fn create(&self, name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
        let _available_providers: Vec<_> = self.entries.keys().collect();

        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))?;

        (entry.constructor)(model)
    }

    pub fn all_metadata_with_types(&self) -> Vec<(ProviderMetadata, ProviderType)> {
        self.entries
            .values()
            .map(|e| (e.metadata.clone(), e.provider_type))
            .collect()
    }

    pub fn remove_custom_providers(&mut self) {
        self.entries.retain(|name, _| !name.starts_with("custom_"));
    }
}
