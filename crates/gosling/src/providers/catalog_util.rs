pub use gosling_providers::canonical::catalog::{
    ModelCapabilities, ModelTemplate, ProviderCatalogEntry, ProviderFormat,
    ProviderSetupCapabilities, ProviderSetupCatalogEntry, ProviderSetupCategory,
    ProviderSetupConfigKey, ProviderSetupField, ProviderSetupGroup, ProviderSetupMetadata,
    ProviderSetupMethod, ProviderTemplate,
};
use std::collections::{HashMap, HashSet};

use super::base::{ConfigKey, ProviderMetadata};

fn setup_config_key(config_key: ConfigKey) -> ProviderSetupConfigKey {
    ProviderSetupConfigKey {
        name: config_key.name,
        required: config_key.required,
        secret: config_key.secret,
        default: config_key.default,
        primary: config_key.primary,
    }
}

fn setup_metadata(metadata: ProviderMetadata) -> ProviderSetupMetadata {
    ProviderSetupMetadata {
        name: metadata.name,
        display_name: metadata.display_name,
        description: metadata.description,
        model_doc_link: metadata.model_doc_link,
        config_keys: metadata
            .config_keys
            .into_iter()
            .map(setup_config_key)
            .collect(),
    }
}

pub async fn get_providers_by_format(format: ProviderFormat) -> Vec<ProviderCatalogEntry> {
    let native_provider_ids = super::init::providers()
        .await
        .into_iter()
        .map(|(metadata, _)| metadata.name)
        .collect::<HashSet<_>>();

    gosling_providers::canonical::catalog::get_providers_by_format(format, &native_provider_ids)
}

pub async fn get_setup_catalog_entries() -> Vec<ProviderSetupCatalogEntry> {
    let registry_metadata = super::providers()
        .await
        .into_iter()
        .map(|(metadata, _)| {
            let name = metadata.name.clone();
            (name, setup_metadata(metadata))
        })
        .collect::<HashMap<_, _>>();

    gosling_providers::canonical::catalog::get_setup_catalog_entries(&registry_metadata)
}

pub fn get_provider_setup_category(provider_id: &str) -> Option<ProviderSetupCategory> {
    gosling_providers::canonical::catalog::get_provider_setup_category(provider_id)
}

pub fn get_provider_template(provider_id: &str) -> Option<ProviderTemplate> {
    gosling_providers::canonical::catalog::get_provider_template(provider_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::base::ProviderType;

    #[tokio::test]
    async fn test_featherless_provider() {
        let featherless = crate::providers::get_from_registry("featherless")
            .await
            .expect("featherless should be registered as a declarative provider");
        assert_eq!(featherless.provider_type(), ProviderType::Declarative);

        let metadata = featherless.metadata();
        assert_eq!(metadata.display_name, "Featherless AI");
        assert!(
            !metadata.known_models.is_empty(),
            "featherless should have starter models"
        );
        assert!(
            metadata
                .known_models
                .iter()
                .any(|model| model.name == "deepseek-ai/DeepSeek-V4-Flash"),
            "featherless should expose a starter DeepSeek model"
        );

        let setup_entries = get_setup_catalog_entries().await;
        let setup_entry = setup_entries
            .iter()
            .find(|entry| entry.provider_id == "featherless")
            .expect("featherless should be in the setup catalog");
        assert_eq!(setup_entry.setup_method, ProviderSetupMethod::SingleApiKey);
    }

    #[tokio::test]
    async fn setup_catalog_includes_gosling_and_curated_fields() {
        let entries = get_setup_catalog_entries().await;

        let gosling = entries
            .iter()
            .find(|entry| entry.provider_id == "gosling")
            .expect("setup catalog should include synthetic gosling");
        assert_eq!(gosling.category, ProviderSetupCategory::Agent);
        assert_eq!(gosling.setup_method, ProviderSetupMethod::None);
        assert!(gosling.fields.is_empty());

        let ollama = entries
            .iter()
            .find(|entry| entry.provider_id == "ollama")
            .expect("setup catalog should include ollama");
        assert_eq!(ollama.setup_method, ProviderSetupMethod::ConfigFields);
        assert_eq!(ollama.fields.len(), 1);
        assert_eq!(ollama.fields[0].key, "OLLAMA_HOST");
        assert_eq!(ollama.fields[0].label, "Host");
        assert_eq!(
            ollama.fields[0].default_value.as_deref(),
            Some("http://localhost:11434")
        );

        let databricks = entries
            .iter()
            .find(|entry| entry.provider_id == "databricks")
            .expect("setup catalog should include databricks");
        assert_eq!(
            databricks.setup_method,
            ProviderSetupMethod::HostWithOauthFallback
        );
        assert_eq!(
            databricks
                .fields
                .iter()
                .map(|field| field.key.as_str())
                .collect::<Vec<_>>(),
            ["DATABRICKS_HOST", "DATABRICKS_TOKEN"]
        );

        let featherless = entries
            .iter()
            .find(|entry| entry.provider_id == "featherless")
            .expect("setup catalog should include featherless");
        assert_eq!(featherless.setup_method, ProviderSetupMethod::SingleApiKey);
        assert_eq!(featherless.fields.len(), 1);
        assert_eq!(featherless.fields[0].key, "FEATHERLESS_API_KEY");
    }

    #[tokio::test]
    async fn setup_catalog_excludes_uncurated_deprecated_providers() {
        let provider_ids = get_setup_catalog_entries()
            .await
            .into_iter()
            .map(|entry| entry.provider_id)
            .collect::<std::collections::HashSet<_>>();

        assert!(provider_ids.contains("claude-acp"));
        assert!(provider_ids.contains("codex-acp"));
        assert!(!provider_ids.contains("claude_code"));
        assert!(!provider_ids.contains("codex"));
        assert!(!provider_ids.contains("gemini_cli"));
    }
}
