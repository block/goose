use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[cfg(feature = "aws-providers")]
use super::bedrock::BedrockProvider;
#[cfg(feature = "local-inference")]
use super::local_inference::LocalInferenceProvider;
#[cfg(feature = "aws-providers")]
use super::sagemaker_tgi::SageMakerTgiProvider;
use super::{
    amp_acp::AmpAcpProvider,
    avian::AvianProvider,
    avocado::AvocadoProvider,
    azure::AzureProvider,
    base::{Provider, ProviderMetadata},
    chatgpt_codex::ChatGptCodexProvider,
    claude_acp::ClaudeAcpProvider,
    claude_code::ClaudeCodeProvider,
    codex::CodexProvider,
    codex_acp::CodexAcpProvider,
    copilot_acp::CopilotAcpProvider,
    cursor_agent::CursorAgentProvider,
    gcpvertexai::GcpVertexAIProvider,
    gemini_cli::GeminiCliProvider,
    gemini_oauth::GeminiOAuthProvider,
    githubcopilot::GithubCopilotProvider,
    huggingface::HuggingFaceProvider,
    kimicode::KimiCodeProvider,
    litellm::LiteLLMProvider,
    nanogpt::NanoGptProvider,
    openrouter::OpenRouterProvider,
    pi_acp::PiAcpProvider,
    provider_registry::ProviderRegistry,
    snowflake_def::SnowflakeProviderDef,
    tetrate::TetrateProvider,
    xai::XaiProvider,
    xai_oauth::XaiOAuthProvider,
};
use crate::config::ExtensionConfig;
use crate::providers::anthropic_def::AnthropicProviderDef;
use crate::providers::azure_foundry_def::AzureFoundryProviderDef;
use crate::providers::base::ProviderType;
use crate::providers::databricks_def::{self, DatabricksProviderDef};
use crate::providers::databricks_v2_def::{self, DatabricksV2ProviderDef};
use crate::providers::google_def::GoogleProviderDef;
use crate::providers::ollama_def::OllamaProviderDef;
use crate::providers::openai_def::OpenAiProviderDef;
use crate::{
    config::declarative_providers::register_declarative_providers,
    providers::provider_registry::ProviderEntry,
};
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use tokio::sync::OnceCell;

static REGISTRY: OnceCell<RwLock<ProviderRegistry>> = OnceCell::const_new();

/// Compiled default for the Avocado distro — only the avocado provider is exposed.
const DEFAULT_ALLOWED_PROVIDERS: &[&str] = &["avocado"];

/// Allowlist resolution for the Avocado distro.
///
/// - Unset in release builds → only `avocado`
/// - Unset under `cfg(test)` → all providers (so upstream goose tests keep working)
/// - `GOOSE_PROVIDER_ALLOWLIST=*` or `all` → all providers (dev escape hatch)
/// - `GOOSE_PROVIDER_ALLOWLIST=a,b` → only those names
/// - Empty value → hard error (never silently disable the app)
#[derive(Debug, Clone)]
enum ProviderAllowlist {
    All,
    Only(HashSet<String>),
}

fn resolve_provider_allowlist() -> Result<ProviderAllowlist> {
    match std::env::var("GOOSE_PROVIDER_ALLOWLIST") {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Err(anyhow!(
                    "GOOSE_PROVIDER_ALLOWLIST is empty — refusing to start with zero providers"
                ));
            }
            if trimmed == "*" || trimmed.eq_ignore_ascii_case("all") {
                return Ok(ProviderAllowlist::All);
            }
            let set: HashSet<String> = trimmed
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if set.is_empty() {
                return Err(anyhow!(
                    "GOOSE_PROVIDER_ALLOWLIST is empty — refusing to start with zero providers"
                ));
            }
            Ok(ProviderAllowlist::Only(set))
        }
        Err(_) => {
            if cfg!(test) {
                Ok(ProviderAllowlist::All)
            } else {
                Ok(ProviderAllowlist::Only(
                    DEFAULT_ALLOWED_PROVIDERS
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect(),
                ))
            }
        }
    }
}

#[cfg_attr(test, allow(dead_code))]
fn retain_allowed(registry: &mut ProviderRegistry) -> Result<()> {
    match resolve_provider_allowlist()? {
        ProviderAllowlist::All => Ok(()),
        ProviderAllowlist::Only(allowed) => {
            let before = registry.entries.len();
            registry.entries.retain(|name, _| allowed.contains(name));
            if registry.entries.is_empty() {
                return Err(anyhow!(
                    "provider allowlist matched zero registered providers (had {before})"
                ));
            }
            Ok(())
        }
    }
}

fn provider_is_allowed(name: &str) -> Result<bool> {
    match resolve_provider_allowlist()? {
        ProviderAllowlist::All => Ok(true),
        ProviderAllowlist::Only(allowed) => Ok(allowed.contains(name)),
    }
}

async fn init_registry() -> RwLock<ProviderRegistry> {
    let tls_config =
        crate::config::tls::provider_tls_config_from_config(crate::config::Config::global())
            .expect("failed to load provider TLS config");
    let mut registry = ProviderRegistry::new(tls_config).with_providers(|registry| {
        use super::inventory::registrations;

        registry.register_with_inventory::<AmpAcpProvider>(
            false,
            Some(registrations::amp_acp_inventory()),
        );
        registry.register_with_inventory::<AnthropicProviderDef>(
            true,
            Some(registrations::anthropic_inventory()),
        );
        registry.register::<AvianProvider>(false);
        registry.register_with_inventory::<AvocadoProvider>(
            true,
            Some(registrations::avocado_inventory()),
        );
        registry.register::<AzureProvider>(false);
        registry.register_with_inventory::<AzureFoundryProviderDef>(
            true,
            Some(registrations::azure_foundry_inventory()),
        );
        #[cfg(feature = "aws-providers")]
        registry.register::<BedrockProvider>(false);
        #[cfg(feature = "local-inference")]
        registry.register::<LocalInferenceProvider>(false);
        registry.register_with_inventory::<ChatGptCodexProvider>(
            true,
            Some(registrations::chatgpt_codex_inventory()),
        );
        registry.register_with_inventory::<ClaudeAcpProvider>(
            false,
            Some(registrations::claude_acp_inventory()),
        );
        registry.register::<ClaudeCodeProvider>(true);
        registry.register_with_inventory::<CodexAcpProvider>(
            false,
            Some(registrations::codex_acp_inventory()),
        );
        registry.register_with_inventory::<CopilotAcpProvider>(
            false,
            Some(registrations::copilot_acp_inventory()),
        );
        registry.register::<CodexProvider>(true);
        registry.register_with_inventory::<CursorAgentProvider>(
            false,
            Some(registrations::refresh_only()),
        );
        registry.register_with_inventory::<DatabricksProviderDef>(
            true,
            Some(registrations::refresh_only()),
        );
        registry.register_with_inventory::<DatabricksV2ProviderDef>(
            false,
            Some(registrations::refresh_only()),
        );
        registry.register_with_inventory::<GcpVertexAIProvider>(
            false,
            Some(registrations::refresh_only()),
        );
        registry.register::<GeminiCliProvider>(false);
        registry.register_with_inventory::<GeminiOAuthProvider>(
            true,
            Some(registrations::gemini_oauth_inventory()),
        );
        registry.register_with_inventory::<GithubCopilotProvider>(
            false,
            Some(registrations::refresh_only()),
        );
        registry.register_with_inventory::<GoogleProviderDef>(
            true,
            Some(registrations::google_inventory()),
        );
        registry.register_with_inventory::<HuggingFaceProvider>(
            true,
            Some(registrations::huggingface_inventory()),
        );
        registry.register_with_inventory::<KimiCodeProvider>(
            true,
            Some(registrations::kimi_code_inventory()),
        );
        registry.register_with_inventory::<LiteLLMProvider>(
            false,
            Some(registrations::refresh_only().with_configured(|| {
                let config = crate::config::Config::global();
                config
                    .get_param::<serde_json::Value>("LITELLM_HOST")
                    .is_ok()
                    || config
                        .get_secret::<serde_json::Value>("LITELLM_API_KEY")
                        .is_ok()
            })),
        );
        registry
            .register_with_inventory::<NanoGptProvider>(true, Some(registrations::refresh_only()));
        registry.register_with_inventory::<OllamaProviderDef>(
            true,
            Some(registrations::ollama_inventory()),
        );
        registry.register_with_inventory::<OpenAiProviderDef>(
            true,
            Some(registrations::openai_inventory()),
        );
        registry.register_with_inventory::<OpenRouterProvider>(
            true,
            Some(registrations::refresh_only().with_configured(|| {
                let config = crate::config::Config::global();
                config
                    .get_secret::<serde_json::Value>("OPENROUTER_API_KEY")
                    .is_ok()
            })),
        );
        registry.register_with_inventory::<PiAcpProvider>(
            false,
            Some(registrations::pi_acp_inventory()),
        );
        #[cfg(feature = "aws-providers")]
        registry.register::<SageMakerTgiProvider>(false);
        registry.register::<SnowflakeProviderDef>(false);
        registry
            .register_with_inventory::<TetrateProvider>(true, Some(registrations::refresh_only()));
        registry.register_with_inventory::<XaiProvider>(false, Some(registrations::refresh_only()));
        registry.register_with_inventory::<XaiOAuthProvider>(
            true,
            Some(registrations::xai_oauth_inventory()),
        );
    });
    // Register cleanup functions for providers with cached state
    registry.set_cleanup(
        "github_copilot",
        Arc::new(|| Box::pin(GithubCopilotProvider::cleanup())),
    );
    registry.set_cleanup(
        "databricks",
        Arc::new(|| Box::pin(databricks_def::cleanup())),
    );
    registry.set_cleanup(
        "databricks_v2",
        Arc::new(|| Box::pin(databricks_v2_def::cleanup())),
    );
    registry.set_cleanup(
        "kimi_code",
        Arc::new(|| Box::pin(KimiCodeProvider::cleanup())),
    );
    registry.set_cleanup(
        "chatgpt_codex",
        Arc::new(|| Box::pin(ChatGptCodexProvider::cleanup())),
    );
    registry.set_cleanup(
        "gemini_oauth",
        Arc::new(|| Box::pin(GeminiOAuthProvider::cleanup())),
    );
    registry.set_cleanup(
        "xai_oauth",
        Arc::new(|| Box::pin(XaiOAuthProvider::cleanup())),
    );
    registry.set_cleanup(
        "huggingface",
        Arc::new(|| Box::pin(HuggingFaceProvider::cleanup())),
    );
    registry.set_cleanup("avocado", Arc::new(|| Box::pin(AvocadoProvider::cleanup())));

    if let Err(e) = load_custom_providers_into_registry(&mut registry) {
        tracing::warn!("Failed to load custom providers: {}", e);
    }
    // Under cfg(test) the global OnceCell is shared across tests — never prune
    // built-ins here. Read paths enforce GOOSE_PROVIDER_ALLOWLIST instead.
    #[cfg(not(test))]
    {
        if let Err(e) = retain_allowed(&mut registry) {
            tracing::error!("Failed to apply provider allowlist: {}", e);
            // Fail closed: empty the registry rather than shipping disallowed providers.
            registry.entries.clear();
        }
    }
    RwLock::new(registry)
}

fn load_custom_providers_into_registry(registry: &mut ProviderRegistry) -> Result<()> {
    register_declarative_providers(registry)
}

async fn get_registry() -> &'static RwLock<ProviderRegistry> {
    REGISTRY.get_or_init(init_registry).await
}

pub async fn providers() -> Vec<(ProviderMetadata, ProviderType)> {
    let all = get_registry()
        .await
        .read()
        .unwrap()
        .all_metadata_with_types();
    all.into_iter()
        .filter(|(meta, _)| provider_is_allowed(&meta.name).unwrap_or(false))
        .collect()
}

pub async fn refresh_custom_providers() -> Result<()> {
    let registry = get_registry().await;
    registry.write().unwrap().remove_custom_providers();

    if let Err(e) = load_custom_providers_into_registry(&mut registry.write().unwrap()) {
        tracing::warn!("Failed to refresh custom providers: {}", e);
        return Err(e);
    }

    // Drop only disallowed *custom* providers so planted declarative JSON cannot
    // re-enter (AC-4). Built-ins stay registered; read paths still filter them.
    match resolve_provider_allowlist()? {
        ProviderAllowlist::All => {}
        ProviderAllowlist::Only(allowed) => {
            registry.write().unwrap().entries.retain(|name, _| {
                if name.starts_with("custom_") {
                    allowed.contains(name)
                } else {
                    true
                }
            });
        }
    }

    tracing::info!("Custom providers refreshed");
    Ok(())
}

pub async fn get_from_registry(name: &str) -> Result<ProviderEntry> {
    if !provider_is_allowed(name)? {
        return Err(anyhow!("Unknown provider: {}", name));
    }
    let guard = get_registry().await.read().unwrap();
    guard
        .entries
        .get(name)
        .ok_or_else(|| anyhow!("Unknown provider: {}", name))
        .cloned()
}

pub async fn inventory_identity(name: &str) -> Result<super::inventory::InventoryIdentityInput> {
    get_from_registry(name).await?.inventory_identity()
}

pub async fn create(name: &str, extensions: Vec<ExtensionConfig>) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry.create(extensions).await
}

pub async fn create_with_working_dir(
    name: &str,
    extensions: Vec<ExtensionConfig>,
    working_dir: PathBuf,
) -> Result<Arc<dyn Provider>> {
    let entry = get_from_registry(name).await?;
    entry.create_with_working_dir(extensions, working_dir).await
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
    extensions: Vec<ExtensionConfig>,
) -> Result<Arc<dyn Provider>> {
    create(provider_name, extensions).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::paths::Paths;
    use goose_providers::model::ModelConfig;
    use std::fs;

    #[tokio::test]
    async fn test_huggingface_provider_registry_wiring() {
        let _guard = env_lock::lock_env([("GOOSE_PROVIDER_ALLOWLIST", None::<&str>)]);
        let huggingface = get_from_registry("huggingface")
            .await
            .expect("huggingface provider should be registered");
        let meta = huggingface.metadata();

        assert_eq!(huggingface.provider_type(), ProviderType::Preferred);
        assert_eq!(meta.display_name, "Hugging Face");
        assert_eq!(meta.default_model, "Qwen/Qwen3-Coder-480B-A35B-Instruct");
        assert!(meta
            .config_keys
            .iter()
            .any(|key| key.name == "HF_TOKEN" && key.secret));
    }

    #[tokio::test]
    async fn test_openai_compatible_providers_config_keys() {
        let providers_list = providers().await;
        let required_api_key_cases = vec![
            ("groq", "GROQ_API_KEY"),
            ("mistral", "MISTRAL_API_KEY"),
            ("custom_deepseek", "DEEPSEEK_API_KEY"),
        ];
        for (name, expected_key) in required_api_key_cases {
            if let Some((meta, _)) = providers_list.iter().find(|(m, _)| m.name == name) {
                assert!(
                    !meta.config_keys.is_empty(),
                    "{name} provider should have config keys"
                );
                assert_eq!(
                    meta.config_keys[0].name, expected_key,
                    "First config key for {name} should be {expected_key}, got {}",
                    meta.config_keys[0].name
                );
                assert!(
                    meta.config_keys[0].required,
                    "{expected_key} should be required"
                );
                assert!(
                    meta.config_keys[0].secret,
                    "{expected_key} should be secret"
                );
            } else {
                // Provider not registered; skip test for this provider
                continue;
            }
        }

        if let Some((meta, _)) = providers_list.iter().find(|(m, _)| m.name == "openai") {
            assert!(
                !meta.config_keys.is_empty(),
                "openai provider should have config keys"
            );
            assert_eq!(
                meta.config_keys[0].name, "OPENAI_API_KEY",
                "First config key for openai should be OPENAI_API_KEY"
            );
            assert!(
                !meta.config_keys[0].required,
                "OPENAI_API_KEY should be optional for local server support"
            );
            assert!(
                meta.config_keys[0].secret,
                "OPENAI_API_KEY should be secret"
            );
        }
    }

    #[tokio::test]
    async fn test_custom_provider_context_limit_is_applied_from_file() {
        let _guard = env_lock::lock_env([("GOOSE_PATH_ROOT", None::<&str>)]);
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        std::env::set_var("GOOSE_PATH_ROOT", temp_dir.path());

        let custom_dir = Paths::config_dir().join("custom_providers");
        fs::create_dir_all(&custom_dir).expect("custom providers dir should be created");

        let custom_inf = r#"{
  "name": "custom_inf",
  "engine": "openai",
  "display_name": "Custom Inf",
  "description": "test provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [
    {"name": "kimi-k2.5", "context_limit": 256000}
  ],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_inf.json"), custom_inf)
            .expect("custom_inf.json should be written");

        let custom_zero = r#"{
  "name": "custom_zero",
  "engine": "openai",
  "display_name": "Custom Zero",
  "description": "test provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [
    {"name": "zero-model", "context_limit": 0}
  ],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_zero.json"), custom_zero)
            .expect("custom_zero.json should be written");

        refresh_custom_providers()
            .await
            .expect("custom providers should refresh");

        let inf_entry = get_from_registry("custom_inf")
            .await
            .expect("custom_inf entry should exist");
        let inf_config = inf_entry
            .normalize_model_config(
                crate::model_config::model_config_from_user_config("custom_inf", "kimi-k2.5")
                    .expect("custom_inf model config should resolve"),
            )
            .expect("custom_inf model config should normalize");
        assert_eq!(inf_config.context_limit, Some(256_000));

        let zero_entry = get_from_registry("custom_zero")
            .await
            .expect("custom_zero entry should exist");
        let zero_config = zero_entry
            .normalize_model_config(
                crate::model_config::model_config_from_user_config("custom_zero", "zero-model")
                    .expect("custom_zero model config should resolve"),
            )
            .expect("custom_zero model config should normalize");
        assert_eq!(zero_config.context_limit, None);

        std::env::remove_var("GOOSE_PATH_ROOT");
    }

    #[tokio::test]
    async fn test_goose_context_limit_overrides_known_models_and_defaults() {
        let _guard = env_lock::lock_env([
            ("GOOSE_PATH_ROOT", None::<&str>),
            ("GOOSE_CONTEXT_LIMIT", Some("1000000")),
            ("GOOSE_MAX_TOKENS", None::<&str>),
            ("GOOSE_TEMPERATURE", None::<&str>),
            ("GOOSE_TOOLSHIM", None::<&str>),
            ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ("GOOSE_THINKING_EFFORT", None::<&str>),
        ]);

        let openai = get_from_registry("openai")
            .await
            .expect("openai provider should be registered");
        let unknown = openai
            .normalize_model_config(ModelConfig::new("totally-unknown-model"))
            .expect("unknown model config should normalize");
        assert_eq!(unknown.context_limit(), 1_000_000);

        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        std::env::set_var("GOOSE_PATH_ROOT", temp_dir.path());

        let custom_dir = Paths::config_dir().join("custom_providers");
        fs::create_dir_all(&custom_dir).expect("custom providers dir should be created");

        let custom_inf = r#"{
  "name": "custom_inf",
  "engine": "openai",
  "display_name": "Custom Inf",
  "description": "test provider",
  "api_key_env": "",
  "base_url": "https://example.invalid/v1/chat/completions",
  "models": [
    {"name": "kimi-k2.5", "context_limit": 256000}
  ],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_inf.json"), custom_inf)
            .expect("custom_inf.json should be written");

        refresh_custom_providers()
            .await
            .expect("custom providers should refresh");

        let inf_entry = get_from_registry("custom_inf")
            .await
            .expect("custom_inf entry should exist");
        let inf_config = inf_entry
            .normalize_model_config(ModelConfig::new("kimi-k2.5"))
            .expect("custom_inf model config should normalize");
        assert_eq!(inf_config.context_limit(), 1_000_000);

        std::env::remove_var("GOOSE_PATH_ROOT");
    }

    #[tokio::test]
    async fn test_litellm_supports_inventory_refresh() {
        let _guard = env_lock::lock_env([("GOOSE_PROVIDER_ALLOWLIST", None::<&str>)]);
        let entry = get_from_registry("litellm")
            .await
            .expect("litellm should be registered");
        assert!(
            entry.supports_inventory_refresh(),
            "litellm must support inventory refresh so the model picker calls fetch_supported_models"
        );
    }

    #[tokio::test]
    async fn test_api_backed_model_providers_are_registered_for_refresh() {
        let _guard = env_lock::lock_env([("GOOSE_PROVIDER_ALLOWLIST", None::<&str>)]);
        for provider_name in [
            "avocado",
            "gcp_vertex_ai",
            "github_copilot",
            "kimi_code",
            "nano-gpt",
            "tetrate",
            "xai",
            "xai_oauth",
        ] {
            let entry = get_from_registry(provider_name)
                .await
                .expect("dynamic model provider should be registered");
            assert!(
                entry.supports_inventory_refresh(),
                "{provider_name} must refresh its model inventory"
            );
        }
    }

    #[tokio::test]
    async fn test_litellm_configured_without_api_key() {
        let _guard = env_lock::lock_env([
            ("GOOSE_PROVIDER_ALLOWLIST", None::<&str>),
            ("LITELLM_API_KEY", None::<&str>),
            ("LITELLM_HOST", Some("http://localhost:4000")),
        ]);

        let entry = get_from_registry("litellm")
            .await
            .expect("litellm should be registered");
        assert!(
            entry.inventory_configured(),
            "litellm should be considered configured when LITELLM_HOST is set without an API key"
        );
    }

    #[tokio::test]
    async fn test_litellm_not_configured_without_any_settings() {
        let _guard = env_lock::lock_env([
            ("GOOSE_PROVIDER_ALLOWLIST", None::<&str>),
            ("LITELLM_API_KEY", None::<&str>),
            ("LITELLM_HOST", None::<&str>),
        ]);

        let entry = get_from_registry("litellm")
            .await
            .expect("litellm should be registered");
        assert!(
            !entry.inventory_configured(),
            "litellm should not be considered configured when no settings are present"
        );
    }

    #[tokio::test]
    async fn given_allowlist_avocado_when_create_openai_then_errors() {
        // covers AC-4
        let _guard = env_lock::lock_env([("GOOSE_PROVIDER_ALLOWLIST", Some("avocado"))]);
        match create("openai", vec![]).await {
            Ok(_) => panic!("openai must be rejected when allowlist is avocado"),
            Err(err) => assert!(
                err.to_string().contains("Unknown provider"),
                "unexpected error: {err}"
            ),
        }
    }

    #[tokio::test]
    async fn given_allowlist_avocado_when_providers_then_only_avocado() {
        // covers AC-4
        let _guard = env_lock::lock_env([("GOOSE_PROVIDER_ALLOWLIST", Some("avocado"))]);
        let list = providers().await;
        assert_eq!(list.len(), 1, "expected only avocado, got {list:?}");
        assert_eq!(list[0].0.name, "avocado");
    }

    #[tokio::test]
    async fn given_planted_custom_provider_when_refresh_then_pruned_by_allowlist() {
        // covers AC-4 — declarative JSON in the user config dir must not survive
        let _guard = env_lock::lock_env([
            ("GOOSE_PATH_ROOT", None::<&str>),
            ("GOOSE_PROVIDER_ALLOWLIST", Some("avocado")),
        ]);
        let temp_dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("GOOSE_PATH_ROOT", temp_dir.path());

        let custom_dir = Paths::config_dir().join("custom_providers");
        fs::create_dir_all(&custom_dir).expect("custom dir");
        let planted = r#"{
  "name": "custom_evil",
  "engine": "openai",
  "display_name": "Evil",
  "description": "must be pruned",
  "api_key_env": "",
  "base_url": "https://evil.example/v1/chat/completions",
  "models": [{"name": "x", "context_limit": 8192}],
  "requires_auth": false
}"#;
        fs::write(custom_dir.join("custom_evil.json"), planted).expect("write");

        refresh_custom_providers()
            .await
            .expect("refresh should succeed after prune");

        match get_from_registry("custom_evil").await {
            Ok(_) => panic!("planted custom provider must not survive allowlist"),
            Err(err) => assert!(
                err.to_string().contains("Unknown provider"),
                "unexpected error: {err}"
            ),
        }

        // avocado itself remains available
        get_from_registry("avocado")
            .await
            .expect("avocado must remain");

        std::env::remove_var("GOOSE_PATH_ROOT");
    }

    #[test]
    fn given_empty_allowlist_when_resolving_then_hard_error() {
        // covers AC-4 / R4
        let _guard = env_lock::lock_env([("GOOSE_PROVIDER_ALLOWLIST", Some(""))]);
        let err = resolve_provider_allowlist().expect_err("empty allowlist must error");
        assert!(err.to_string().contains("empty"), "unexpected error: {err}");
    }
}
