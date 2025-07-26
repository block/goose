use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: &'static str,
    pub factory_name: Option<&'static str>,
    pub model_name: &'static str,
    pub required_env_vars: &'static [&'static str],
    pub env_modifications: Option<HashMap<&'static str, Option<String>>>,
    pub skip: bool,
}

impl ProviderConfig {
    fn simple(name: &'static str, model_name: &'static str) -> Self {
        let key = format!("{}_API_KEY", name.to_uppercase());
        let required_env_vars =
            Box::leak(vec![Box::leak(key.into_boxed_str()) as &str].into_boxed_slice());

        Self {
            name,
            factory_name: None,
            model_name,
            required_env_vars,
            env_modifications: None,
            skip: false,
        }
    }

    fn simple_skip(name: &'static str, model_name: &'static str) -> Self {
        let mut config = Self::simple(name, model_name);
        config.skip = true;
        config
    }

    pub fn name_for_factory(&self) -> String {
        self.factory_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.name.to_lowercase())
    }
}

static PROVIDER_CONFIGS: LazyLock<Vec<ProviderConfig>> = LazyLock::new(|| {
    vec![
        ProviderConfig::simple("OpenAI", "gpt-4o"),
        ProviderConfig::simple("Anthropic", "claude-3-5-sonnet-20241022"),
        ProviderConfig {
            name: "Azure",
            factory_name: Some("azure_openai"),
            model_name: "gpt-4o",
            required_env_vars: &[
                "AZURE_OPENAI_API_KEY",
                "AZURE_OPENAI_ENDPOINT",
                "AZURE_OPENAI_DEPLOYMENT_NAME",
            ],
            env_modifications: None,
            skip: false,
        },
        ProviderConfig {
            name: "Bedrock",
            factory_name: Some("aws_bedrock"),
            model_name: "anthropic.claude-3-5-sonnet-20241022-v2:0",
            required_env_vars: &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
            env_modifications: None,
            skip: false,
        },
        ProviderConfig::simple("Google", "gemini-2.5-flash"),
        ProviderConfig::simple_skip("Groq", "llama-3.1-70b-versatile"),
    ]
});

pub fn get_provider_configs() -> &'static [ProviderConfig] {
    &PROVIDER_CONFIGS
}

pub fn get_active_provider_configs() -> Vec<&'static ProviderConfig> {
    PROVIDER_CONFIGS
        .iter()
        .filter(|config| !config.skip)
        .collect()
}
