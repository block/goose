use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::{
    base::{Provider, ProviderMetadata},
    gcpvertexai::GcpVertexAIProvider,
    gemini_oauth::GeminiOAuthProvider,
    provider_registry::ProviderRegistry,
};
use crate::config::ExtensionConfig;
use crate::model::ModelConfig;
use crate::providers::base::ProviderType;
use crate::providers::provider_registry::ProviderEntry;
use anyhow::Result;
use tokio::sync::OnceCell;

static REGISTRY: OnceCell<RwLock<ProviderRegistry>> = OnceCell::const_new();

async fn init_registry() -> RwLock<ProviderRegistry> {
    let registry = ProviderRegistry::new().with_providers(|registry| {
        registry.register::<GcpVertexAIProvider>(false);
        registry.register::<GeminiOAuthProvider>(true);
    });
    RwLock::new(registry)
}

fn load_custom_providers_into_registry(_registry: &mut ProviderRegistry) -> Result<()> {
    Ok(())
}

async fn get_registry() -> &'static RwLock<ProviderRegistry> {
    REGISTRY.get_or_init(init_registry).await
}

pub async fn providers() -> Vec<(ProviderMetadata, ProviderType)> {
    get_registry()
        .await
        .read()
        .unwrap()
        .all_metadata_with_types()
}

pub async fn refresh_custom_providers() -> Result<()> {
    let registry = get_registry().await;
    registry.write().unwrap().remove_custom_providers();

    if let Err(e) = load_custom_providers_into_registry(&mut registry.write().unwrap()) {
        tracing::warn!("Failed to refresh custom providers: {}", e);
        return Err(e);
    }

    tracing::info!("Custom providers refreshed");
    Ok(())
}

pub async fn get_from_registry(name: &str) -> Result<ProviderEntry> {
    let guard = get_registry().await.read().unwrap();
    guard
        .entries
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))
        .cloned()
}

pub async fn inventory_identity(name: &str) -> Result<super::inventory::InventoryIdentityInput> {
    get_from_registry(name).await?.inventory_identity()
}

pub async fn create(
    name: &str,
    model: ModelConfig,
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry.create(model, extensions).await
}

pub async fn create_with_working_dir(
    name: &str,
    model: ModelConfig,
    extensions: Vec<ExtensionConfig>,
    working_dir: PathBuf,
) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry
        .create_with_working_dir(model, extensions, working_dir)
        .await
}

pub async fn create_with_default_model(
    name: impl AsRef<str>,
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    get_from_registry(name.as_ref())
        .await?
        .create_with_default_model(extensions)
        .await
}

pub async fn cleanup_provider(name: &str) -> Result<()> {
    let cleanup_fn = {
        let registry = get_registry().await.read().unwrap();
        registry
            .entries
            .get(name)
            .and_then(|entry| entry.cleanup.clone())
    };
    if let Some(cleanup) = cleanup_fn {
        return cleanup().await;
    }
    Ok(())
}

pub async fn create_with_named_model(
    provider_name: &str,
    model_name: &str,
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    let config = ModelConfig::new(model_name)?;
    create(provider_name, config, extensions).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_contains_only_gemini_providers() {
        let providers_list = providers().await;
        let names: Vec<&str> = providers_list
            .iter()
            .map(|(m, _)| m.name.as_str())
            .collect();

        assert_eq!(
            providers_list.len(),
            2,
            "registry should contain exactly two providers, got: {names:?}"
        );
        assert!(
            names.contains(&"gemini_oauth"),
            "gemini_oauth should be registered"
        );
        assert!(
            names.contains(&"gcp_vertex_ai"),
            "gcp_vertex_ai should be registered"
        );

        for removed in ["openai", "anthropic", "ollama", "google", "gemini_cli"] {
            assert!(
                !names.contains(&removed),
                "{removed} should no longer be registered"
            );
        }
    }

    #[tokio::test]
    async fn test_gemini_oauth_is_preferred_and_vertex_is_builtin() {
        let providers_list = providers().await;

        let (_, gemini_type) = providers_list
            .iter()
            .find(|(m, _)| m.name == "gemini_oauth")
            .expect("gemini_oauth provider should be registered");
        assert_eq!(*gemini_type, ProviderType::Preferred);

        let (_, vertex_type) = providers_list
            .iter()
            .find(|(m, _)| m.name == "gcp_vertex_ai")
            .expect("gcp_vertex_ai provider should be registered");
        assert_eq!(*vertex_type, ProviderType::Builtin);
    }

    #[tokio::test]
    async fn test_gemini_oauth_signs_in_without_api_key() {
        let gemini = get_from_registry("gemini_oauth")
            .await
            .expect("gemini_oauth provider should be registered");
        let meta = gemini.metadata();
        assert_eq!(meta.display_name, "Gemini");
        // OAuth sign-in flow, not an API key: the only config key is the OAuth token.
        assert!(
            meta.config_keys
                .iter()
                .all(|k| k.name == "GEMINI_OAUTH_TOKEN"),
            "gemini_oauth should authenticate via OAuth sign-in, not an API key"
        );
    }
}
