use super::base::{ConfigKey, ModelInfo, Provider, ProviderDef, ProviderMetadata, ProviderType};
use super::inventory::{InventoryIdentityInput, InventoryRegistration, InventoryResolvers};
use crate::config::{DeclarativeProviderConfig, ExtensionConfig};
use crate::model::ModelConfig;
use anyhow::Result;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub type ProviderConstructor = Arc<
    dyn Fn(
            ModelConfig,
            Vec<ExtensionConfig>,
            Option<PathBuf>,
        ) -> BoxFuture<'static, Result<Arc<dyn Provider>>>
        + Send
        + Sync,
>;

pub type ProviderCleanup = Arc<dyn Fn() -> BoxFuture<'static, Result<()>> + Send + Sync>;

#[derive(Clone)]
pub struct ProviderEntry {
    metadata: ProviderMetadata,
    pub(crate) constructor: ProviderConstructor,
    pub(crate) inventory_identity: super::inventory::InventoryIdentityResolver,
    pub(crate) inventory_configured: super::inventory::InventoryConfiguredResolver,
    pub(crate) cleanup: Option<ProviderCleanup>,
    provider_type: ProviderType,
    supports_inventory_refresh: bool,
}

impl ProviderEntry {
    pub fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    pub fn provider_type(&self) -> ProviderType {
        self.provider_type
    }

    pub fn supports_inventory_refresh(&self) -> bool {
        self.supports_inventory_refresh
    }

    pub fn inventory_identity(&self) -> Result<InventoryIdentityInput> {
        (self.inventory_identity)()
    }

    pub fn inventory_configured(&self) -> bool {
        (self.inventory_configured)()
    }

    fn normalize_model_config(&self, mut model: ModelConfig) -> ModelConfig {
        model = model.with_canonical_limits(&self.metadata.name);

        if model.context_limit.is_none() {
            if let Some(info) = self
                .metadata
                .known_models
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(&model.model_name) && m.context_limit > 0)
            {
                model.context_limit = Some(info.context_limit);
            }
        }

        model
    }

    pub async fn create_with_default_model(
        &self,
        extensions: Vec<ExtensionConfig>,
    ) -> Result<Arc<dyn Provider>> {
        let default_model = &self.metadata.default_model;
        let model_config = self.normalize_model_config(ModelConfig::new(default_model.as_str())?);
        (self.constructor)(model_config, extensions, None).await
    }

    pub async fn create(
        &self,
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
    ) -> Result<Arc<dyn Provider>> {
        let model = self.normalize_model_config(model);
        (self.constructor)(model, extensions, None).await
    }

    pub async fn create_with_working_dir(
        &self,
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
        working_dir: PathBuf,
    ) -> Result<Arc<dyn Provider>> {
        let model = self.normalize_model_config(model);
        (self.constructor)(model, extensions, Some(working_dir)).await
    }
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

    pub fn register<F>(&mut self, preferred: bool)
    where
        F: ProviderDef + 'static,
    {
        self.register_with_inventory::<F>(preferred, None);
    }

    pub fn register_with_inventory<F>(
        &mut self,
        preferred: bool,
        inventory_registration: Option<InventoryRegistration>,
    ) where
        F: ProviderDef + 'static,
    {
        let metadata = F::metadata();
        let name = metadata.name.clone();

        let inventory = InventoryResolvers::for_metadata(&metadata, inventory_registration);

        self.entries.insert(
            name,
            ProviderEntry {
                metadata,
                constructor: Arc::new(|model, extensions, working_dir| {
                    Box::pin(async move {
                        let provider = match working_dir {
                            Some(working_dir) => {
                                F::from_env_with_working_dir(model, extensions, working_dir).await?
                            }
                            None => F::from_env(model, extensions).await?,
                        };
                        Ok(Arc::new(provider) as Arc<dyn Provider>)
                    })
                }),
                inventory_identity: inventory.identity,
                inventory_configured: inventory.configured,
                cleanup: None,
                provider_type: if preferred {
                    ProviderType::Preferred
                } else {
                    ProviderType::Builtin
                },
                supports_inventory_refresh: inventory.supports_refresh,
            },
        );
    }

    pub fn register_with_name<P, F, G>(
        &mut self,
        config: &DeclarativeProviderConfig,
        provider_type: ProviderType,
        supports_inventory_refresh: bool,
        constructor: F,
        inventory_identity: G,
    ) where
        P: ProviderDef + 'static,
        F: Fn(ModelConfig) -> Result<P::Provider> + Send + Sync + 'static,
        G: Fn() -> Result<InventoryIdentityInput> + Send + Sync + 'static,
    {
        self.register_with_name_impl::<P, F, G>(
            config,
            provider_type,
            supports_inventory_refresh,
            constructor,
            inventory_identity,
            None,
        );
    }

    pub fn register_with_name_and_inventory_configured<P, F, G, H>(
        &mut self,
        config: &DeclarativeProviderConfig,
        provider_type: ProviderType,
        supports_inventory_refresh: bool,
        constructor: F,
        inventory_identity: G,
        inventory_configured: H,
    ) where
        P: ProviderDef + 'static,
        F: Fn(ModelConfig) -> Result<P::Provider> + Send + Sync + 'static,
        G: Fn() -> Result<InventoryIdentityInput> + Send + Sync + 'static,
        H: Fn() -> bool + Send + Sync + 'static,
    {
        self.register_with_name_impl::<P, F, G>(
            config,
            provider_type,
            supports_inventory_refresh,
            constructor,
            inventory_identity,
            Some(Arc::new(inventory_configured)),
        );
    }

    fn register_with_name_impl<P, F, G>(
        &mut self,
        config: &DeclarativeProviderConfig,
        provider_type: ProviderType,
        supports_inventory_refresh: bool,
        constructor: F,
        inventory_identity: G,
        inventory_configured: Option<super::inventory::InventoryConfiguredResolver>,
    ) where
        P: ProviderDef + 'static,
        F: Fn(ModelConfig) -> Result<P::Provider> + Send + Sync + 'static,
        G: Fn() -> Result<InventoryIdentityInput> + Send + Sync + 'static,
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
                resolved_model: None,
                context_limit: m.context_limit,
                input_token_cost: m.input_token_cost,
                output_token_cost: m.output_token_cost,
                currency: m.currency.clone(),
                supports_cache_control: Some(m.supports_cache_control.unwrap_or(false)),
                reasoning: m.reasoning,
            })
            .collect();

        let mut config_keys = if provider_type == ProviderType::Declarative {
            if !config.api_key_env.is_empty() {
                vec![ConfigKey::new(
                    &config.api_key_env,
                    config.requires_auth,
                    true,
                    None,
                    true,
                )]
            } else {
                Vec::new()
            }
        } else {
            let mut config_keys = base_metadata.config_keys.clone();

            if let Some(api_key_index) = config_keys.iter().position(|key| key.secret) {
                if !config.requires_auth {
                    config_keys.remove(api_key_index);
                } else if !config.api_key_env.is_empty() {
                    config_keys[api_key_index] =
                        ConfigKey::new(&config.api_key_env, false, true, None, true);
                }
            }

            config_keys
        };

        if let Some(ref env_vars) = config.env_vars {
            for ev in env_vars {
                // Default primary to `required` so required fields show prominently in the UI
                let primary = ev.primary.unwrap_or(ev.required);
                config_keys.push(ConfigKey::new(
                    &ev.name,
                    ev.required,
                    ev.secret,
                    ev.default.as_deref(),
                    primary,
                ));
            }
        }

        let custom_metadata = ProviderMetadata {
            name: config.name.clone(),
            display_name: config.display_name.clone(),
            description,
            default_model,
            known_models,
            model_doc_link: config
                .model_doc_link
                .clone()
                .unwrap_or(base_metadata.model_doc_link),
            config_keys,
            setup_steps: config.setup_steps.clone(),
            model_selection_hint: None,
        };
        let inventory_config_keys = custom_metadata.config_keys.clone();
        let default_inventory_configured = Arc::new(move || {
            super::inventory::default_inventory_configured(
                &inventory_config_keys,
                crate::config::Config::global(),
            )
        });

        self.entries.insert(
            config.name.clone(),
            ProviderEntry {
                metadata: custom_metadata,
                constructor: Arc::new(move |model, _extensions, _working_dir| {
                    let result = constructor(model);
                    Box::pin(async move {
                        let provider = result?;
                        Ok(Arc::new(provider) as Arc<dyn Provider>)
                    })
                }),
                inventory_identity: Arc::new(inventory_identity),
                inventory_configured: inventory_configured.unwrap_or(default_inventory_configured),
                cleanup: None,
                provider_type,
                supports_inventory_refresh,
            },
        );
    }

    pub fn set_cleanup(&mut self, name: &str, cleanup: ProviderCleanup) {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.cleanup = Some(cleanup);
        }
    }

    pub fn with_providers<F>(mut self, setup: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        setup(&mut self);
        self
    }

    pub async fn create(
        &self,
        name: &str,
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
    ) -> Result<Arc<dyn Provider>> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))?;

        entry.create(model, extensions).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::declarative_providers::ProviderEngine;
    use crate::providers::openai::OpenAiProvider;

    fn test_config() -> DeclarativeProviderConfig {
        DeclarativeProviderConfig {
            name: "custom_hf".to_string(),
            engine: ProviderEngine::OpenAI,
            display_name: "Custom HF".to_string(),
            description: None,
            api_key_env: String::new(),
            base_url: "https://router.huggingface.co/v1".to_string(),
            models: vec![ModelInfo::new("test-model", 128_000)],
            headers: None,
            timeout_seconds: None,
            supports_streaming: Some(true),
            requires_auth: true,
            catalog_provider_id: Some("huggingface".to_string()),
            base_path: None,
            env_vars: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            model_doc_link: None,
            setup_steps: vec![],
            fast_model: None,
            preserves_thinking: false,
        }
    }

    #[test]
    fn register_with_name_can_override_inventory_configured() {
        let mut registry = ProviderRegistry::new();
        registry.register_with_name_and_inventory_configured::<OpenAiProvider, _, _, _>(
            &test_config(),
            ProviderType::Declarative,
            false,
            |_| unreachable!("constructor is not used by this test"),
            || Ok(InventoryIdentityInput::new("custom_hf", "huggingface")),
            || false,
        );

        let entry = registry.entries.get("custom_hf").unwrap();

        assert!(!entry.inventory_configured());
    }
}
