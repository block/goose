use super::api_client::TlsConfig;
use super::base::{ConfigKey, ModelInfo, Provider, ProviderDef, ProviderMetadata, ProviderType};
use super::inventory::{InventoryIdentityInput, InventoryRegistration, InventoryResolvers};
use crate::config::{DeclarativeProviderConfig, ExtensionConfig};
use anyhow::Result;
use futures::future::BoxFuture;
use goose_providers::model::ModelConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub type ProviderConstructor = Arc<
    dyn Fn(
            Vec<ExtensionConfig>,
            Option<PathBuf>,
            Option<TlsConfig>,
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
    tls_config: Option<TlsConfig>,
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

    /// Apply provider-specific normalization to a model config: materialize
    /// global defaults, then let the provider's own model declaration outrank
    /// the canonical catalog.
    ///
    /// A declaration in the provider's config describes the endpoint that will
    /// actually serve the request. A canonical entry is matched on model *name*,
    /// which collides whenever a local or proxied deployment reuses a well-known
    /// name — a `deepseek-v4-flash` behind a llama.cpp server with `--ctx 49152`
    /// inherits the hosted model's million-token limit, and auto-compaction is
    /// then calibrated against a ceiling the endpoint will never accept.
    ///
    /// Built-in providers derive `known_models` from the canonical registry
    /// (`model_info_for_provider_model`), so their declared and canonical values
    /// already agree and this changes nothing for them.
    ///
    /// Precedence: caller-supplied value, then `GOOSE_CONTEXT_LIMIT` /
    /// `GOOSE_MAX_TOKENS`, then the provider declaration, then canonical.
    pub fn normalize_model_config(&self, model: ModelConfig) -> Result<ModelConfig> {
        let declared = self
            .metadata
            .known_models
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(&model.model_name));
        let declared_context_limit = declared.map(|m| m.context_limit).filter(|limit| *limit > 0);
        let declared_max_tokens = declared
            .and_then(|m| m.max_tokens)
            .filter(|tokens| *tokens > 0);

        // Callers hand in an already-materialized config, so `is_some()` cannot
        // tell a value the caller chose from one the canonical catalog filled
        // in. Compare against what canonical alone would produce: matching it
        // means nobody chose the value, so the declaration may take over.
        let canonical =
            ModelConfig::new(&model.model_name).with_canonical_limits(&self.metadata.name);
        let context_limit_unchosen =
            model.context_limit.is_none() || model.context_limit == canonical.context_limit;
        let max_tokens_unchosen =
            model.max_tokens.is_none() || model.max_tokens == canonical.max_tokens;

        let mut model = crate::model_config::materialize_model_config(&self.metadata.name, model)?;

        let config = crate::config::Config::global();
        if let Some(limit) = declared_context_limit {
            if context_limit_unchosen && config.get_goose_context_limit()?.is_none() {
                model.context_limit = Some(limit);
            }
        }
        if let Some(max_tokens) = declared_max_tokens {
            if max_tokens_unchosen && config.get_goose_max_tokens()?.is_none() {
                model.max_tokens = Some(max_tokens);
            }
        }

        Ok(model)
    }

    pub async fn create_with_default_model(
        &self,
        extensions: Vec<ExtensionConfig>,
    ) -> Result<Arc<dyn Provider>> {
        self.create(extensions).await
    }

    pub async fn create(&self, extensions: Vec<ExtensionConfig>) -> Result<Arc<dyn Provider>> {
        (self.constructor)(extensions, None, self.tls_config.clone()).await
    }

    pub async fn create_with_working_dir(
        &self,
        extensions: Vec<ExtensionConfig>,
        working_dir: PathBuf,
    ) -> Result<Arc<dyn Provider>> {
        (self.constructor)(extensions, Some(working_dir), self.tls_config.clone()).await
    }
}

#[derive(Default)]
pub struct ProviderRegistry {
    pub(crate) entries: HashMap<String, ProviderEntry>,
    tls_config: Option<TlsConfig>,
}

impl ProviderRegistry {
    pub fn new(tls_config: Option<TlsConfig>) -> Self {
        Self {
            entries: HashMap::new(),
            tls_config,
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
                constructor: Arc::new(|extensions, working_dir, tls_config| {
                    Box::pin(async move {
                        let provider = match working_dir {
                            Some(working_dir) => {
                                F::from_env_with_working_dir(extensions, working_dir, tls_config)
                                    .await?
                            }
                            None => F::from_env(extensions, tls_config).await?,
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
                tls_config: self.tls_config.clone(),
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
        F: Fn(Option<TlsConfig>) -> Result<P::Provider> + Send + Sync + 'static,
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
        F: Fn(Option<TlsConfig>) -> Result<P::Provider> + Send + Sync + 'static,
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
        F: Fn(Option<TlsConfig>) -> Result<P::Provider> + Send + Sync + 'static,
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
                resolved_model: None,
                supports_cache_control: Some(m.supports_cache_control.unwrap_or(false)),
                ..m.clone()
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
            fast_model: config.fast_model.clone(),
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
                constructor: Arc::new(move |_extensions, _working_dir, tls_config| {
                    let result = constructor(tls_config);
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
                tls_config: self.tls_config.clone(),
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
        extensions: Vec<ExtensionConfig>,
    ) -> Result<Arc<dyn Provider>> {
        let entry = self
            .entries
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))?;

        entry.create(extensions).await
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
    use crate::providers::openai_def::OpenAiProviderDef;

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
        let mut registry = ProviderRegistry::new(None);
        registry.register_with_name_and_inventory_configured::<OpenAiProviderDef, _, _, _>(
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

    /// `deepseek-v4-flash` name-matches a hosted catalog entry with a
    /// million-token context. A local server declaring 32_768 must not inherit it.
    fn entry_declaring(models: Vec<ModelInfo>) -> ProviderEntry {
        let mut config = test_config();
        config.models = models;
        let mut registry = ProviderRegistry::new(None);
        registry.register_with_name_and_inventory_configured::<OpenAiProviderDef, _, _, _>(
            &config,
            ProviderType::Declarative,
            false,
            |_| unreachable!("constructor is not used by this test"),
            || Ok(InventoryIdentityInput::new("custom_hf", "huggingface")),
            || false,
        );
        registry.entries.get("custom_hf").unwrap().clone()
    }

    fn declared_flash_model() -> ModelInfo {
        ModelInfo {
            max_tokens: Some(4096),
            ..ModelInfo::new("deepseek-v4-flash", 32_768)
        }
    }

    #[test]
    fn declared_limits_outrank_canonical_catalog() {
        let _guard = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
        ]);

        let normalized = entry_declaring(vec![declared_flash_model()])
            .normalize_model_config(ModelConfig::new("deepseek-v4-flash"))
            .unwrap();

        assert_eq!(normalized.context_limit, Some(32_768));
        assert_eq!(normalized.max_tokens, Some(4096));
    }

    #[test]
    fn undeclared_limit_still_falls_back_to_canonical_catalog() {
        let _guard = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
        ]);

        let normalized = entry_declaring(vec![ModelInfo::new("deepseek-v4-flash", 0)])
            .normalize_model_config(ModelConfig::new("deepseek-v4-flash"))
            .unwrap();

        assert!(
            normalized.context_limit.is_some_and(|limit| limit > 32_768),
            "a model with no declared limit should still be enriched from the catalog, got {:?}",
            normalized.context_limit
        );
    }

    #[test]
    fn goose_context_limit_outranks_declaration() {
        let _guard = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", Some("20000")),
            ("GOOSE_MAX_TOKENS", Some("2048")),
        ]);

        let normalized = entry_declaring(vec![declared_flash_model()])
            .normalize_model_config(ModelConfig::new("deepseek-v4-flash"))
            .unwrap();

        assert_eq!(normalized.context_limit, Some(20_000));
        assert_eq!(normalized.max_tokens, Some(2048));
    }

    /// The limits an operator writes into `custom_providers/<id>.json` must
    /// survive deserialization and reach the resolved config.
    #[test]
    fn limits_declared_in_provider_json_reach_the_resolved_config() {
        let _guard = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
        ]);

        let config: DeclarativeProviderConfig = serde_json::from_str(
            r#"{
                "name": "custom_hf",
                "engine": "openai",
                "display_name": "Local ds4",
                "base_url": "http://localhost:8000/v1",
                "requires_auth": false,
                "dynamic_models": false,
                "skip_canonical_filtering": true,
                "models": [
                    { "name": "deepseek-v4-flash", "context_limit": 32768, "max_tokens": 4096 }
                ]
            }"#,
        )
        .unwrap();

        let mut registry = ProviderRegistry::new(None);
        registry.register_with_name_and_inventory_configured::<OpenAiProviderDef, _, _, _>(
            &config,
            ProviderType::Declarative,
            false,
            |_| unreachable!("constructor is not used by this test"),
            || Ok(InventoryIdentityInput::new("custom_hf", "huggingface")),
            || false,
        );

        let normalized = registry
            .entries
            .get("custom_hf")
            .unwrap()
            .normalize_model_config(ModelConfig::new("deepseek-v4-flash"))
            .unwrap();

        assert_eq!(normalized.context_limit, Some(32_768));
        assert_eq!(normalized.max_tokens, Some(4096));
    }

    /// Every production caller (`session/builder.rs`, `acp/server.rs`) passes a
    /// config that already went through `model_config_from_user_config`, so the
    /// canonical limits are populated before this runs. Treating a populated
    /// field as "the caller chose it" would silently skip the declaration —
    /// which is the original bug.
    #[test]
    fn declaration_still_applies_to_an_already_materialized_config() {
        let _guard = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
        ]);

        let already_resolved =
            crate::model_config::model_config_from_user_config("custom_hf", "deepseek-v4-flash")
                .unwrap();
        assert!(
            already_resolved.context_limit.is_some(),
            "precondition: the caller's config arrives with canonical limits already applied"
        );

        let normalized = entry_declaring(vec![declared_flash_model()])
            .normalize_model_config(already_resolved)
            .unwrap();

        assert_eq!(normalized.context_limit, Some(32_768));
        assert_eq!(normalized.max_tokens, Some(4096));
    }

    #[test]
    fn caller_supplied_limit_outranks_declaration() {
        let _guard = env_lock::lock_env([
            ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ("GOOSE_MAX_TOKENS", None::<&str>),
        ]);

        let normalized = entry_declaring(vec![declared_flash_model()])
            .normalize_model_config(
                ModelConfig::new("deepseek-v4-flash").with_context_limit(Some(8_000)),
            )
            .unwrap();

        assert_eq!(normalized.context_limit, Some(8_000));
    }
}
