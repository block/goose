use std::sync::{Arc, RwLock};

use super::{
    anthropic::AnthropicProvider,
    azure::AzureProvider,
    base::{Provider, ProviderMetadata},
    bedrock::BedrockProvider,
    claude_code::ClaudeCodeProvider,
    codex::CodexProvider,
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
        registry.register::<CodexProvider, _>(|m| Box::pin(CodexProvider::from_env(m)), true);
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

    if let Ok(limit) = global_config.get_param::<usize>("GOOSE_WORKER_CONTEXT_LIMIT") {
        worker_config = worker_config.with_context_limit(Some(limit));
    } else if let Ok(limit) = global_config.get_param::<usize>("GOOSE_CONTEXT_LIMIT") {
        worker_config = worker_config.with_context_limit(Some(limit));
    }

    Ok(worker_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case::test_case(None, None, None, DEFAULT_LEAD_TURNS, DEFAULT_FAILURE_THRESHOLD, DEFAULT_FALLBACK_TURNS ; "defaults")]
    #[test_case::test_case(Some("7"), Some("4"), Some("3"), 7, 4, 3 ; "custom")]
    #[tokio::test]
    async fn test_create_lead_worker_provider(
        lead_turns: Option<&str>,
        failure_threshold: Option<&str>,
        fallback_turns: Option<&str>,
        expected_turns: usize,
        expected_failure: usize,
        expected_fallback: usize,
    ) {
        let _guard = env_lock::lock_env([
            ("GOOSE_LEAD_MODEL", Some("gpt-4o")),
            ("GOOSE_LEAD_PROVIDER", None),
            ("GOOSE_LEAD_TURNS", lead_turns),
            ("GOOSE_LEAD_FAILURE_THRESHOLD", failure_threshold),
            ("GOOSE_LEAD_FALLBACK_TURNS", fallback_turns),
            ("OPENAI_API_KEY", Some("fake-openai-no-keyring")),
        ]);

        let provider = create("openai", ModelConfig::new_or_fail("gpt-4o-mini"))
            .await
            .unwrap();
        let lw = provider.as_lead_worker().unwrap();
        let (lead, worker) = lw.get_model_info();
        assert_eq!(lead, "gpt-4o");
        assert_eq!(worker, "gpt-4o-mini");
        assert_eq!(
            lw.get_settings(),
            (expected_turns, expected_failure, expected_fallback)
        );
    }

    #[tokio::test]
    async fn test_create_regular_provider_without_lead_config() {
        let _guard = env_lock::lock_env([
            ("GOOSE_LEAD_MODEL", None),
            ("GOOSE_LEAD_PROVIDER", None),
            ("GOOSE_LEAD_TURNS", None),
            ("GOOSE_LEAD_FAILURE_THRESHOLD", None),
            ("GOOSE_LEAD_FALLBACK_TURNS", None),
            ("OPENAI_API_KEY", Some("fake-openai-no-keyring")),
        ]);

        let provider = create("openai", ModelConfig::new_or_fail("gpt-4o-mini"))
            .await
            .unwrap();
        assert!(provider.as_lead_worker().is_none());
        assert_eq!(provider.get_model_config().model_name, "gpt-4o-mini");
    }

    #[test_case::test_case(None, None, 16_000 ; "no overrides uses default")]
    #[test_case::test_case(Some("32000"), None, 32_000 ; "worker limit overrides default")]
    #[test_case::test_case(Some("32000"), Some("64000"), 32_000 ; "worker limit takes priority over global")]
    fn test_worker_model_context_limit(
        worker_limit: Option<&str>,
        global_limit: Option<&str>,
        expected_limit: usize,
    ) {
        let _guard = env_lock::lock_env([
            ("GOOSE_WORKER_CONTEXT_LIMIT", worker_limit),
            ("GOOSE_CONTEXT_LIMIT", global_limit),
        ]);

        let default_model =
            ModelConfig::new_or_fail("gpt-3.5-turbo").with_context_limit(Some(16_000));

        let result = create_worker_model_config(&default_model).unwrap();
        assert_eq!(result.context_limit, Some(expected_limit));
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
