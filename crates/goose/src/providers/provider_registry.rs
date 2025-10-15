use super::base::{Provider, ProviderMetadata};
use crate::model::ModelConfig;
use anyhow::Result;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;

type ProviderConstructor =
    Arc<dyn Fn(ModelConfig) -> BoxFuture<'static, Result<Arc<dyn Provider>>> + Send + Sync>;

pub struct ProviderEntry {
    metadata: ProviderMetadata,
    pub(crate) constructor: ProviderConstructor,
}

#[derive(Default)]
pub struct ProviderRegistry {
    pub(crate) entries: HashMap<String, ProviderEntry>,
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
        F: Fn(ModelConfig) -> BoxFuture<'static, Result<P>> + Send + Sync + 'static,
    {
        let metadata = P::metadata();
        let name = metadata.name.clone();

        self.entries.insert(
            name,
            ProviderEntry {
                metadata,
                constructor: Arc::new(move |model| {
                    let fut = constructor(model);
                    Box::pin(async move {
                        let provider = fut.await?;
                        Ok(Arc::new(provider) as Arc<dyn Provider>)
                    })
                }),
            },
        );
    }

    pub fn register_with_name<P, F>(
        &mut self,
        custom_name: String,
        display_name: String,
        description: String,
        default_model: String,
        known_models: Vec<super::base::ModelInfo>,
        constructor: F,
    ) where
        P: Provider + 'static,
        F: Fn(ModelConfig) -> Result<P> + Send + Sync + 'static,
    {
        let base_metadata = P::metadata();
        let custom_metadata = ProviderMetadata {
            name: custom_name.clone(),
            display_name,
            description,
            default_model,
            known_models,
            model_doc_link: base_metadata.model_doc_link,
            config_keys: base_metadata.config_keys,
        };

        self.entries.insert(
            custom_name,
            ProviderEntry {
                metadata: custom_metadata,
                constructor: Arc::new(move |model| {
                    let result = constructor(model);
                    Box::pin(async move {
                        let provider = result?;
                        Ok(Arc::new(provider) as Arc<dyn Provider>)
                    })
                }),
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

    pub async fn create(&self, name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))?;

        (entry.constructor)(model).await
    }

    pub fn all_metadata(&self) -> Vec<ProviderMetadata> {
        self.entries.values().map(|e| e.metadata.clone()).collect()
    }

    pub fn remove_custom_providers(&mut self) {
        self.entries.retain(|name, _| !name.starts_with("custom_"));
    }
}
