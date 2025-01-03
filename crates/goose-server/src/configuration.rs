use crate::error::{to_env_var, ConfigError};
use config::{Config, Environment};
use goose::providers::configs::{
    AnthropicProviderConfig, GoogleProviderConfig, GroqProviderConfig,
};
use goose::providers::openai::OPEN_AI_DEFAULT_MODEL;
use goose::providers::{
    anthropic,
    configs::{
        DatabricksAuth, DatabricksProviderConfig, ModelConfig, OllamaProviderConfig,
        OpenAiProviderConfig, ProviderConfig,
    },
    factory::ProviderType,
    google, groq, ollama,
    utils::ImageFormat,
};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Default, Deserialize)]
pub struct ServerSettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl ServerSettings {
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Failed to parse socket address")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum ProviderSettings {
    OpenAi {
        #[serde(default = "default_openai_host")]
        host: String,
        api_key: String,
        #[serde(default = "default_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
        #[serde(default)]
        context_limit: Option<usize>,
        #[serde(default)]
        estimate_factor: Option<f32>,
    },
    OpenRouter {
        #[serde(default = "default_openrouter_host")]
        host: String,
        api_key: String,
        #[serde(default = "default_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
        #[serde(default)]
        context_limit: Option<usize>,
        #[serde(default)]
        estimate_factor: Option<f32>,
    },
    Databricks {
        #[serde(default = "default_databricks_host")]
        host: String,
        #[serde(default = "default_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
        #[serde(default)]
        context_limit: Option<usize>,
        #[serde(default)]
        estimate_factor: Option<f32>,
        #[serde(default = "default_image_format")]
        image_format: ImageFormat,
    },
    Ollama {
        #[serde(default = "default_ollama_host")]
        host: String,
        #[serde(default = "default_ollama_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
        #[serde(default)]
        context_limit: Option<usize>,
        #[serde(default)]
        estimate_factor: Option<f32>,
    },
    Google {
        #[serde(default = "default_google_host")]
        host: String,
        api_key: String,
        #[serde(default = "default_google_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
        #[serde(default)]
        context_limit: Option<usize>,
        #[serde(default)]
        estimate_factor: Option<f32>,
    },
    Groq {
        #[serde(default = "default_groq_host")]
        host: String,
        api_key: String,
        #[serde(default = "default_groq_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
        #[serde(default)]
        context_limit: Option<usize>,
        #[serde(default)]
        estimate_factor: Option<f32>,
    },
    Anthropic {
        #[serde(default = "default_anthropic_host")]
        host: String,
        api_key: String,
        #[serde(default = "default_anthropic_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
        #[serde(default)]
        context_limit: Option<usize>,
        #[serde(default)]
        estimate_factor: Option<f32>,
    },
}

impl ProviderSettings {
    // Get the provider type
    #[allow(dead_code)]
    pub fn provider_type(&self) -> ProviderType {
        match self {
            ProviderSettings::OpenAi { .. } => ProviderType::OpenAi,
            ProviderSettings::Databricks { .. } => ProviderType::Databricks,
            ProviderSettings::Ollama { .. } => ProviderType::Ollama,
            ProviderSettings::Google { .. } => ProviderType::Google,
            ProviderSettings::Groq { .. } => ProviderType::Groq,
            ProviderSettings::Anthropic { .. } => ProviderType::Anthropic,
            ProviderSettings::OpenRouter { .. } => ProviderType::OpenRouter,
        }
    }

    // Convert to the goose ProviderConfig
    pub fn into_config(self) -> ProviderConfig {
        match self {
            ProviderSettings::OpenAi {
                host,
                api_key,
                model,
                temperature,
                max_tokens,
                context_limit,
                estimate_factor,
            } => ProviderConfig::OpenAi(OpenAiProviderConfig {
                host,
                api_key,
                model: ModelConfig::new(model)
                    .with_temperature(temperature)
                    .with_max_tokens(max_tokens)
                    .with_context_limit(context_limit)
                    .with_estimate_factor(estimate_factor),
            }),
            ProviderSettings::OpenRouter {
                host,
                api_key,
                model,
                temperature,
                max_tokens,
                context_limit,
                estimate_factor,
            } => ProviderConfig::OpenRouter(OpenAiProviderConfig {
                host,
                api_key,
                model: ModelConfig::new(model)
                    .with_temperature(temperature)
                    .with_max_tokens(max_tokens)
                    .with_context_limit(context_limit)
                    .with_estimate_factor(estimate_factor),
            }),
            ProviderSettings::Databricks {
                host,
                model,
                temperature,
                max_tokens,
                context_limit,
                image_format,
                estimate_factor,
            } => ProviderConfig::Databricks(DatabricksProviderConfig {
                host: host.clone(),
                auth: DatabricksAuth::oauth(host),
                model: ModelConfig::new(model)
                    .with_temperature(temperature)
                    .with_max_tokens(max_tokens)
                    .with_context_limit(context_limit)
                    .with_estimate_factor(estimate_factor),
                image_format,
            }),
            ProviderSettings::Ollama {
                host,
                model,
                temperature,
                max_tokens,
                context_limit,
                estimate_factor,
            } => ProviderConfig::Ollama(OllamaProviderConfig {
                host,
                model: ModelConfig::new(model)
                    .with_temperature(temperature)
                    .with_max_tokens(max_tokens)
                    .with_context_limit(context_limit)
                    .with_estimate_factor(estimate_factor),
            }),
            ProviderSettings::Google {
                host,
                api_key,
                model,
                temperature,
                max_tokens,
                context_limit,
                estimate_factor,
            } => ProviderConfig::Google(GoogleProviderConfig {
                host,
                api_key,
                model: ModelConfig::new(model)
                    .with_temperature(temperature)
                    .with_max_tokens(max_tokens)
                    .with_context_limit(context_limit)
                    .with_estimate_factor(estimate_factor),
            }),
            ProviderSettings::Groq {
                host,
                api_key,
                model,
                temperature,
                max_tokens,
                context_limit,
                estimate_factor,
            } => ProviderConfig::Groq(GroqProviderConfig {
                host,
                api_key,
                model: ModelConfig::new(model)
                    .with_temperature(temperature)
                    .with_max_tokens(max_tokens)
                    .with_context_limit(context_limit)
                    .with_estimate_factor(estimate_factor),
            }),
            ProviderSettings::Anthropic {
                host,
                api_key,
                model,
                temperature,
                max_tokens,
                context_limit,
                estimate_factor,
            } => ProviderConfig::Anthropic(AnthropicProviderConfig {
                host,
                api_key,
                model: ModelConfig::new(model)
                    .with_temperature(temperature)
                    .with_max_tokens(max_tokens)
                    .with_context_limit(context_limit)
                    .with_estimate_factor(estimate_factor),
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub server: ServerSettings,
    pub provider: ProviderSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        Self::load_and_validate()
    }

    fn load_and_validate() -> Result<Self, ConfigError> {
        // Start with default configuration
        let config = Config::builder()
            // Server defaults
            .set_default("server.host", default_host())?
            .set_default("server.port", default_port())?
            // Provider defaults
            .set_default("provider.host", default_openai_host())?
            .set_default("provider.model", default_model())?
            // Layer on the environment variables
            .add_source(
                Environment::with_prefix("GOOSE")
                    .prefix_separator("_")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        // Try to deserialize the configuration
        let result: Result<Self, config::ConfigError> = config.try_deserialize();

        // Handle missing field errors specially
        match result {
            Ok(settings) => Ok(settings),
            Err(err) => {
                tracing::debug!("Configuration error: {:?}", &err);

                // Handle both NotFound and missing field message variants
                let error_str = err.to_string();
                if error_str.starts_with("missing field") {
                    // Extract field name from error message "missing field `type`"
                    let field = error_str
                        .trim_start_matches("missing field `")
                        .trim_end_matches("`");
                    let env_var = to_env_var(field);
                    Err(ConfigError::MissingEnvVar { env_var })
                } else if let config::ConfigError::NotFound(field) = &err {
                    let env_var = to_env_var(field);
                    Err(ConfigError::MissingEnvVar { env_var })
                } else {
                    Err(ConfigError::Other(err))
                }
            }
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

pub fn default_openrouter_host() -> String {
    "https://openrouter.ai".to_string()
}

fn default_model() -> String {
    OPEN_AI_DEFAULT_MODEL.to_string()
}

fn default_openai_host() -> String {
    "https://api.openai.com".to_string()
}

fn default_databricks_host() -> String {
    "https://api.databricks.com".to_string()
}

fn default_ollama_host() -> String {
    ollama::OLLAMA_HOST.to_string()
}

fn default_ollama_model() -> String {
    ollama::OLLAMA_MODEL.to_string()
}

fn default_google_host() -> String {
    google::GOOGLE_API_HOST.to_string()
}

fn default_google_model() -> String {
    google::GOOGLE_DEFAULT_MODEL.to_string()
}

fn default_groq_host() -> String {
    groq::GROQ_API_HOST.to_string()
}

fn default_groq_model() -> String {
    groq::GROQ_DEFAULT_MODEL.to_string()
}

fn default_anthropic_host() -> String {
    "https://api.anthropic.com".to_string()
}

fn default_anthropic_model() -> String {
    anthropic::ANTHROPIC_DEFAULT_MODEL.to_string()
}

fn default_image_format() -> ImageFormat {
    ImageFormat::Anthropic
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    fn clean_env() {
        for (key, _) in env::vars() {
            if key.starts_with("GOOSE_") {
                env::remove_var(&key);
            }
        }
    }

    #[test]
    #[serial]
    fn test_default_settings() {
        clean_env();

        // Set required provider settings for test
        env::set_var("GOOSE_PROVIDER__TYPE", "openai");
        env::set_var("GOOSE_PROVIDER__API_KEY", "test-key");

        let settings = Settings::new().unwrap();
        assert_eq!(settings.server.host, "127.0.0.1");
        assert_eq!(settings.server.port, 3000);

        if let ProviderSettings::OpenAi {
            host,
            api_key,
            model,
            temperature,
            max_tokens,
            context_limit,
            estimate_factor,
        } = settings.provider
        {
            assert_eq!(host, "https://api.openai.com");
            assert_eq!(api_key, "test-key");
            assert_eq!(model, "gpt-4o");
            assert_eq!(temperature, None);
            assert_eq!(max_tokens, None);
            assert_eq!(context_limit, None);
            assert_eq!(estimate_factor, None);
        } else {
            panic!("Expected OpenAI provider");
        }

        // Clean up
        env::remove_var("GOOSE_PROVIDER__TYPE");
        env::remove_var("GOOSE_PROVIDER__API_KEY");
    }

    #[test]
    #[serial]
    fn test_into_config_conversion() {
        // Test OpenAI conversion
        let settings = ProviderSettings::OpenAi {
            host: "https://api.openai.com".to_string(),
            api_key: "test-key".to_string(),
            model: "gpt-4o".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(1000),
            context_limit: Some(150_000),
            estimate_factor: Some(0.8),
        };

        if let ProviderConfig::OpenAi(config) = settings.into_config() {
            assert_eq!(config.host, "https://api.openai.com");
            assert_eq!(config.api_key, "test-key");
            assert_eq!(config.model.model_name, "gpt-4o");
            assert_eq!(config.model.temperature, Some(0.7));
            assert_eq!(config.model.max_tokens, Some(1000));
            assert_eq!(config.model.context_limit, Some(150_000));
            assert_eq!(config.model.estimate_factor, Some(0.8));
        } else {
            panic!("Expected OpenAI config");
        }
    }

    #[test]
    #[serial]
    fn test_databricks_settings() {
        clean_env();
        env::set_var("GOOSE_PROVIDER__TYPE", "databricks");
        env::set_var("GOOSE_PROVIDER__HOST", "https://custom.databricks.com");
        env::set_var("GOOSE_PROVIDER__MODEL", "llama-2-70b");
        env::set_var("GOOSE_PROVIDER__TEMPERATURE", "0.7");
        env::set_var("GOOSE_PROVIDER__MAX_TOKENS", "2000");
        env::set_var("GOOSE_PROVIDER__CONTEXT_LIMIT", "150000");

        let settings = Settings::new().unwrap();
        if let ProviderSettings::Databricks {
            host,
            model,
            temperature,
            max_tokens,
            context_limit,
            estimate_factor,
            image_format: _,
        } = settings.provider
        {
            assert_eq!(host, "https://custom.databricks.com");
            assert_eq!(model, "llama-2-70b");
            assert_eq!(temperature, Some(0.7));
            assert_eq!(max_tokens, Some(2000));
            assert_eq!(context_limit, Some(150000));
            assert_eq!(estimate_factor, None);
        } else {
            panic!("Expected Databricks provider");
        }

        // Clean up
        env::remove_var("GOOSE_PROVIDER__TYPE");
        env::remove_var("GOOSE_PROVIDER__HOST");
        env::remove_var("GOOSE_PROVIDER__MODEL");
        env::remove_var("GOOSE_PROVIDER__TEMPERATURE");
        env::remove_var("GOOSE_PROVIDER__MAX_TOKENS");
        env::remove_var("GOOSE_PROVIDER__CONTEXT_LIMIT");
    }

    #[test]
    #[serial]
    fn test_ollama_settings() {
        clean_env();
        env::set_var("GOOSE_PROVIDER__TYPE", "ollama");
        env::set_var("GOOSE_PROVIDER__HOST", "http://custom.ollama.host");
        env::set_var("GOOSE_PROVIDER__MODEL", "llama2");
        env::set_var("GOOSE_PROVIDER__TEMPERATURE", "0.7");
        env::set_var("GOOSE_PROVIDER__MAX_TOKENS", "2000");
        env::set_var("GOOSE_PROVIDER__CONTEXT_LIMIT", "150000");
        env::set_var("GOOSE_PROVIDER__ESTIMATE_FACTOR", "0.7");

        let settings = Settings::new().unwrap();
        if let ProviderSettings::Ollama {
            host,
            model,
            temperature,
            max_tokens,
            context_limit,
            estimate_factor,
        } = settings.provider
        {
            assert_eq!(host, "http://custom.ollama.host");
            assert_eq!(model, "llama2");
            assert_eq!(temperature, Some(0.7));
            assert_eq!(max_tokens, Some(2000));
            assert_eq!(context_limit, Some(150000));
            assert_eq!(estimate_factor, Some(0.7));
        } else {
            panic!("Expected Ollama provider");
        }

        // Clean up
        env::remove_var("GOOSE_PROVIDER__TYPE");
        env::remove_var("GOOSE_PROVIDER__HOST");
        env::remove_var("GOOSE_PROVIDER__MODEL");
        env::remove_var("GOOSE_PROVIDER__TEMPERATURE");
        env::remove_var("GOOSE_PROVIDER__MAX_TOKENS");
        env::remove_var("GOOSE_PROVIDER__CONTEXT_LIMIT");
        env::remove_var("GOOSE_PROVIDER__ESTIMATE_FACTOR");
    }

    #[test]
    #[serial]
    fn test_environment_override() {
        clean_env();
        env::set_var("GOOSE_SERVER__PORT", "8080");
        env::set_var("GOOSE_PROVIDER__TYPE", "openai");
        env::set_var("GOOSE_PROVIDER__API_KEY", "test-key");
        env::set_var("GOOSE_PROVIDER__HOST", "https://custom.openai.com");
        env::set_var("GOOSE_PROVIDER__MODEL", "gpt-3.5-turbo");
        env::set_var("GOOSE_PROVIDER__TEMPERATURE", "0.8");
        env::set_var("GOOSE_PROVIDER__CONTEXT_LIMIT", "150000");

        let settings = Settings::new().unwrap();
        assert_eq!(settings.server.port, 8080);

        if let ProviderSettings::OpenAi {
            host,
            api_key,
            model,
            temperature,
            context_limit,
            ..
        } = settings.provider
        {
            assert_eq!(host, "https://custom.openai.com");
            assert_eq!(api_key, "test-key");
            assert_eq!(model, "gpt-3.5-turbo");
            assert_eq!(temperature, Some(0.8));
            assert_eq!(context_limit, Some(150000));
        } else {
            panic!("Expected OpenAI provider");
        }

        // Clean up
        env::remove_var("GOOSE_SERVER__PORT");
        env::remove_var("GOOSE_PROVIDER__TYPE");
        env::remove_var("GOOSE_PROVIDER__API_KEY");
        env::remove_var("GOOSE_PROVIDER__HOST");
        env::remove_var("GOOSE_PROVIDER__MODEL");
        env::remove_var("GOOSE_PROVIDER__TEMPERATURE");
        env::remove_var("GOOSE_PROVIDER__CONTEXT_LIMIT");
    }

    #[test]
    fn test_socket_addr_conversion() {
        let server_settings = ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 3000,
        };
        let addr = server_settings.socket_addr();
        assert_eq!(addr.to_string(), "127.0.0.1:3000");
    }
}
