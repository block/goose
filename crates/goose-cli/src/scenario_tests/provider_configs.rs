//! Providers for the scenario tests. Keep in sync with
//!

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
        ProviderConfig::simple("OpenAI", "gpt-4o"),
        ProviderConfig::simple("Anthropic", "claude-3-5-sonnet-20241022"),
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
            model_name: "anthropic.claude-3-5-sonnet-20241022-v2:0",
            required_env_vars: &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
            env_modifications: None,
            skip_reason: Some("No valid keys around"),
        },
        ProviderConfig::simple("Google", "gemini-2.5-flash"),
        ProviderConfig::simple("Groq", "llama-3.3-70b-versatile"),
        ProviderConfig::simple_skip(
            "OpenRouter",
            "anthropic/claude-3.5-sonnet",
            Some("Key is no longer valid"),
        ),
    ]
});

pub fn get_provider_configs() -> Vec<&'static ProviderConfig> {
    PROVIDER_CONFIGS
        .iter()
        .filter(|config| !config.is_skipped())
        .collect()
}
