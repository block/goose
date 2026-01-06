//! Providers for the scenario tests. Keep in sync with
//! goose/crates/goose/src/providers/factory.rs

use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: &'static str,
    pub model_name: &'static str,
    pub required_env_vars: &'static [&'static str],
    pub env_modifications: Option<HashMap<&'static str, Option<String>>>,
    pub skip_reason: Option<&'static str>,
}

impl ProviderConfig {
    fn simple_skip(
        name: &'static str,
        model_name: &'static str,
        skip_reason: Option<&'static str>,
    ) -> Self {
        let key = format!("{}_API_KEY", name.to_uppercase());
        let required_env_vars =
            Box::leak(vec![Box::leak(key.into_boxed_str()) as &str].into_boxed_slice());

        Self {
            name,
            model_name,
            required_env_vars,
            env_modifications: None,
            skip_reason,
        }
    }

    pub fn simple(name: &'static str, model_name: &'static str) -> Self {
        Self::simple_skip(name, model_name, None)
    }

    pub fn is_skipped(&self) -> bool {
        self.skip_reason.is_some()
    }
}

static PROVIDER_CONFIGS: LazyLock<Vec<ProviderConfig>> = LazyLock::new(|| {
    vec![
        ProviderConfig::simple("openai", "gpt-4o"),
        ProviderConfig::simple("anthropic", "claude-sonnet-4-20250514"),
        ProviderConfig {
            name: "azure_openai",
            model_name: "gpt-4o",
            required_env_vars: &[
                "AZURE_OPENAI_API_KEY",
                "AZURE_OPENAI_ENDPOINT",
                "AZURE_OPENAI_DEPLOYMENT_NAME",
            ],
            env_modifications: None,
            skip_reason: None,
        },
        ProviderConfig {
            name: "aws_bedrock",
            model_name: "anthropic.claude-sonnet-4-20250514:0",
            required_env_vars: &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
            env_modifications: None,
            skip_reason: Some("No valid keys around"),
        },
        ProviderConfig::simple("google", "gemini-2.5-flash"),
        ProviderConfig::simple("groq", "llama-3.3-70b-versatile"),
        ProviderConfig::simple_skip(
            "openrouter",
            "anthropic/claude-sonnet-4",
            Some("Key is no longer valid"),
        ),
        ProviderConfig::simple_skip(
            "claude-code",
            "claude-sonnet-4-20250514",
            Some("No keys available"),
        ),
        ProviderConfig::simple_skip("cursor-agent", "gpt-5", Some("No keys available")),
        ProviderConfig::simple_skip(
            "databricks",
            "databricks-dbrx-instruct",
            Some("No keys available"),
        ),
        ProviderConfig::simple_skip(
            "gcp_vertex_ai",
            "gemini-2.5-flash",
            Some("No keys available"),
        ),
        ProviderConfig::simple_skip("gemini-cli", "gemini-2.5-flash", Some("No keys available")),
        ProviderConfig::simple_skip("litellm", "gpt-4o", Some("No keys available")),
        ProviderConfig::simple_skip("ollama", "qwen3", Some("Ollama not supported")),
        ProviderConfig::simple_skip(
            "sagemaker_tgi",
            "meta-llama/Llama-2-7b-chat-hf",
            Some("No keys available"),
        ),
        ProviderConfig::simple_skip("snowflake", "claude-3-7-sonnet", Some("No keys available")),
        ProviderConfig::simple_skip("venice", "llama-3.3-70b", Some("No keys available")),
        ProviderConfig::simple_skip("xai", "grok-3", Some("No keys available")),
    ]
});

pub fn get_provider_configs() -> Vec<&'static ProviderConfig> {
    PROVIDER_CONFIGS
        .iter()
        .filter(|config| !config.is_skipped())
        .collect()
}
