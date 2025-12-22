use std::sync::{Arc, RwLock};

use super::{
    anthropic::AnthropicProvider,
    azure::AzureProvider,
    base::{Provider, ProviderMetadata},
    bedrock::BedrockProvider,
    claude_code::ClaudeCodeProvider,
    cursor_agent::CursorAgentProvider,
    databricks::DatabricksProvider,
    gcpvertexai::GcpVertexAIProvider,
    gemini_cli::GeminiCliProvider,
    githubcopilot::GithubCopilotProvider,
    google::GoogleProvider,
    lead_worker::LeadWorkerProvider,
    litellm::LiteLLMProvider,
    ollama::OllamaProvider,
    openai::OpenAiProvider,
    openrouter::OpenRouterProvider,
    provider_registry::ProviderRegistry,
    sagemaker_tgi::SageMakerTgiProvider,
    snowflake::SnowflakeProvider,
    tetrate::TetrateProvider,
    venice::VeniceProvider,
    xai::XaiProvider,
};
use crate::model::ModelConfig;
use crate::providers::base::ProviderType;
use crate::{
    config::declarative_providers::register_declarative_providers,
    providers::provider_registry::ProviderEntry,
};
use anyhow::Result;
use tokio::sync::OnceCell;

const DEFAULT_LEAD_TURNS: usize = 3;
const DEFAULT_FAILURE_THRESHOLD: usize = 2;
const DEFAULT_FALLBACK_TURNS: usize = 2;

static REGISTRY: OnceCell<RwLock<ProviderRegistry>> = OnceCell::const_new();

async fn init_registry() -> RwLock<ProviderRegistry> {
    let mut registry = ProviderRegistry::new().with_providers(|registry| {
        registry
            .register::<AnthropicProvider, _>(|m| Box::pin(AnthropicProvider::from_env(m)), true);
        registry.register::<AzureProvider, _>(|m| Box::pin(AzureProvider::from_env(m)), false);
        registry.register::<BedrockProvider, _>(|m| Box::pin(BedrockProvider::from_env(m)), false);
        registry
            .register::<ClaudeCodeProvider, _>(|m| Box::pin(ClaudeCodeProvider::from_env(m)), true);
        registry.register::<CursorAgentProvider, _>(
            |m| Box::pin(CursorAgentProvider::from_env(m)),
            false,
        );
        registry
            .register::<DatabricksProvider, _>(|m| Box::pin(DatabricksProvider::from_env(m)), true);
        registry.register::<GcpVertexAIProvider, _>(
            |m| Box::pin(GcpVertexAIProvider::from_env(m)),
            false,
        );
        registry
            .register::<GeminiCliProvider, _>(|m| Box::pin(GeminiCliProvider::from_env(m)), false);
        registry.register::<GithubCopilotProvider, _>(
            |m| Box::pin(GithubCopilotProvider::from_env(m)),
            false,
        );
        registry.register::<GoogleProvider, _>(|m| Box::pin(GoogleProvider::from_env(m)), true);
        registry.register::<LiteLLMProvider, _>(|m| Box::pin(LiteLLMProvider::from_env(m)), false);
        registry.register::<OllamaProvider, _>(|m| Box::pin(OllamaProvider::from_env(m)), true);
        registry.register::<OpenAiProvider, _>(|m| Box::pin(OpenAiProvider::from_env(m)), true);
        registry
            .register::<OpenRouterProvider, _>(|m| Box::pin(OpenRouterProvider::from_env(m)), true);
        registry.register::<SageMakerTgiProvider, _>(
            |m| Box::pin(SageMakerTgiProvider::from_env(m)),
            false,
        );
        registry
            .register::<SnowflakeProvider, _>(|m| Box::pin(SnowflakeProvider::from_env(m)), false);
        registry.register::<TetrateProvider, _>(|m| Box::pin(TetrateProvider::from_env(m)), true);
        registry.register::<VeniceProvider, _>(|m| Box::pin(VeniceProvider::from_env(m)), false);
        registry.register::<XaiProvider, _>(|m| Box::pin(XaiProvider::from_env(m)), false);
    });
    if let Err(e) = load_custom_providers_into_registry(&mut registry) {
        tracing::warn!("Failed to load custom providers: {}", e);
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

async fn get_from_registry(name: &str) -> Result<ProviderEntry> {
    let guard = get_registry().await.read().unwrap();
    guard
        .entries
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", name))
        .cloned()
}

pub async fn create(name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
    let config = crate::config::Config::global();

    if let Ok(lead_model_name) = config.get_param::<String>("GOOSE_LEAD_MODEL") {
        tracing::info!("Creating lead/worker provider from environment variables");
        return create_lead_worker_from_env(name, &model, &lead_model_name).await;
    }

    let constructor = get_from_registry(name).await?.constructor.clone();
    constructor(model).await
}

pub async fn create_with_default_model(name: impl AsRef<str>) -> Result<Arc<dyn Provider>> {
    get_from_registry(name.as_ref())
        .await?
        .create_with_default_model()
        .await
}

pub async fn create_with_named_model(
    provider_name: &str,
    model_name: &str,
) -> Result<Arc<dyn Provider>> {
    let config = ModelConfig::new(model_name)?;
    create(provider_name, config).await
}

async fn create_lead_worker_from_env(
    default_provider_name: &str,
    default_model: &ModelConfig,
    lead_model_name: &str,
) -> Result<Arc<dyn Provider>> {
    let config = crate::config::Config::global();

    let lead_provider_name = config
        .get_param::<String>("GOOSE_LEAD_PROVIDER")
        .unwrap_or_else(|_| default_provider_name.to_string());

    let lead_turns = config
        .get_param::<usize>("GOOSE_LEAD_TURNS")
        .unwrap_or(DEFAULT_LEAD_TURNS);
    let failure_threshold = config
        .get_param::<usize>("GOOSE_LEAD_FAILURE_THRESHOLD")
        .unwrap_or(DEFAULT_FAILURE_THRESHOLD);
    let fallback_turns = config
        .get_param::<usize>("GOOSE_LEAD_FALLBACK_TURNS")
        .unwrap_or(DEFAULT_FALLBACK_TURNS);

    let lead_model_config = ModelConfig::new_with_context_env(
        lead_model_name.to_string(),
        Some("GOOSE_LEAD_CONTEXT_LIMIT"),
    )?;

    let worker_model_config = create_worker_model_config(default_model)?;

    let registry = get_registry().await;

    let lead_constructor = {
        let guard = registry.read().unwrap();
        guard
            .entries
            .get(&lead_provider_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", lead_provider_name))?
            .constructor
            .clone()
    };

    let worker_constructor = {
        let guard = registry.read().unwrap();
        guard
            .entries
            .get(default_provider_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", default_provider_name))?
            .constructor
            .clone()
    };

    let lead_provider = lead_constructor(lead_model_config).await?;
    let worker_provider = worker_constructor(worker_model_config).await?;

    Ok(Arc::new(LeadWorkerProvider::new_with_settings(
        lead_provider,
        worker_provider,
        lead_turns,
        failure_threshold,
        fallback_turns,
    )))
}

fn create_worker_model_config(default_model: &ModelConfig) -> Result<ModelConfig> {
    let mut worker_config = ModelConfig::new_or_fail(&default_model.model_name)
        .with_context_limit(default_model.context_limit)
        .with_temperature(default_model.temperature)
        .with_max_tokens(default_model.max_tokens)
        .with_toolshim(default_model.toolshim)
        .with_toolshim_model(default_model.toolshim_model.clone());

    let global_config = crate::config::Config::global();

    if let Ok(limit_str) = global_config.get_param::<String>("GOOSE_WORKER_CONTEXT_LIMIT") {
        if let Ok(limit) = limit_str.parse::<usize>() {
            worker_config = worker_config.with_context_limit(Some(limit));
        }
    } else if let Ok(limit_str) = global_config.get_param::<String>("GOOSE_CONTEXT_LIMIT") {
        if let Ok(limit) = limit_str.parse::<usize>() {
            worker_config = worker_config.with_context_limit(Some(limit));
        }
    }

    Ok(worker_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    struct EnvVarGuard {
        vars: Vec<(String, Option<String>)>,
    }

    impl EnvVarGuard {
        fn new(vars: &[&str]) -> Self {
            let saved_vars = vars
                .iter()
                .map(|&var| (var.to_string(), env::var(var).ok()))
                .collect();

            for &var in vars {
                env::remove_var(var);
            }

            Self { vars: saved_vars }
        }

        fn set(&self, key: &str, value: &str) {
            env::set_var(key, value);
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            for (key, value) in &self.vars {
                match value {
                    Some(val) => env::set_var(key, val),
                    None => env::remove_var(key),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_create_lead_worker_provider() {
        // Both API keys needed: openai for worker, anthropic for lead (GOOSE_LEAD_PROVIDER=anthropic)
        let _guard = EnvVarGuard::new(&[
            "GOOSE_LEAD_MODEL",
            "GOOSE_LEAD_PROVIDER",
            "GOOSE_LEAD_TURNS",
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
        ]);

        _guard.set("OPENAI_API_KEY", "fake-openai-no-keyring");
        _guard.set("ANTHROPIC_API_KEY", "fake-anthropic-no-keyring");
        _guard.set("GOOSE_LEAD_MODEL", "gpt-4o");

        let gpt4mini_config = ModelConfig::new_or_fail("gpt-4o-mini");
        let result = create("openai", gpt4mini_config.clone()).await;

        match result {
            Ok(_) => {}
            Err(error) => {
                let error_msg = error.to_string();
                assert!(error_msg.contains("OPENAI_API_KEY") || error_msg.contains("secret"));
            }
        }

        _guard.set("GOOSE_LEAD_PROVIDER", "anthropic");
        _guard.set("GOOSE_LEAD_TURNS", "5");

        let _result = create("openai", gpt4mini_config).await;
    }

    #[tokio::test]
    async fn test_lead_model_env_vars_with_defaults() {
        let _guard = EnvVarGuard::new(&[
            "GOOSE_LEAD_MODEL",
            "GOOSE_LEAD_PROVIDER",
            "GOOSE_LEAD_TURNS",
            "GOOSE_LEAD_FAILURE_THRESHOLD",
            "GOOSE_LEAD_FALLBACK_TURNS",
            "OPENAI_API_KEY",
        ]);

        _guard.set("OPENAI_API_KEY", "fake-openai-no-keyring");
        _guard.set("GOOSE_LEAD_MODEL", "grok-3");

        let result = create("openai", ModelConfig::new_or_fail("gpt-4o-mini")).await;

        match result {
            Ok(_) => {}
            Err(error) => {
                let error_msg = error.to_string();
                assert!(error_msg.contains("OPENAI_API_KEY") || error_msg.contains("secret"));
            }
        }

        _guard.set("GOOSE_LEAD_TURNS", "7");
        _guard.set("GOOSE_LEAD_FAILURE_THRESHOLD", "4");
        _guard.set("GOOSE_LEAD_FALLBACK_TURNS", "3");

        let _result = create("openai", ModelConfig::new_or_fail("gpt-4o-mini"));
    }

    #[tokio::test]
    async fn test_create_regular_provider_without_lead_config() {
        let _guard = EnvVarGuard::new(&[
            "GOOSE_LEAD_MODEL",
            "GOOSE_LEAD_PROVIDER",
            "GOOSE_LEAD_TURNS",
            "GOOSE_LEAD_FAILURE_THRESHOLD",
            "GOOSE_LEAD_FALLBACK_TURNS",
            "OPENAI_API_KEY",
        ]);

        _guard.set("OPENAI_API_KEY", "fake-openai-no-keyring");
        let result = create("openai", ModelConfig::new_or_fail("gpt-4o-mini")).await;

        match result {
            Ok(_) => {}
            Err(error) => {
                let error_msg = error.to_string();
                assert!(error_msg.contains("OPENAI_API_KEY") || error_msg.contains("secret"));
            }
        }
    }

    #[test]
    fn test_worker_model_preserves_original_context_limit() {
        let _guard = EnvVarGuard::new(&[
            "GOOSE_LEAD_MODEL",
            "GOOSE_WORKER_CONTEXT_LIMIT",
            "GOOSE_CONTEXT_LIMIT",
        ]);

        _guard.set("GOOSE_LEAD_MODEL", "gpt-4o");

        let default_model =
            ModelConfig::new_or_fail("gpt-3.5-turbo").with_context_limit(Some(16_000));

        let _result = create_lead_worker_from_env("openai", &default_model, "gpt-4o");

        _guard.set("GOOSE_WORKER_CONTEXT_LIMIT", "32000");
        let _result = create_lead_worker_from_env("openai", &default_model, "gpt-4o");

        _guard.set("GOOSE_CONTEXT_LIMIT", "64000");
        let _result = create_lead_worker_from_env("openai", &default_model, "gpt-4o");
    }

    #[tokio::test]
    async fn test_openai_compatible_providers_config_keys() {
        let providers_list = providers().await;
        let cases = vec![
            ("openai", "OPENAI_API_KEY"),
            ("groq", "GROQ_API_KEY"),
            ("mistral", "MISTRAL_API_KEY"),
            ("custom_deepseek", "DEEPSEEK_API_KEY"),
        ];
        for (name, expected_key) in cases {
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
    }
}
