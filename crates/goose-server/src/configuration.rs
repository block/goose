use crate::error::{to_env_var, ConfigError};
use config::{Config, Environment};
use goose::providers::{
    configs::{DatabricksProviderConfig, OllamaProviderConfig, OpenAiProviderConfig, ProviderConfig},
    factory::ProviderType,
    ollama,
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
    },
    Databricks {
        #[serde(default = "default_databricks_host")]
        host: String,
        token: String,
        #[serde(default = "default_model")]
        model: String,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        max_tokens: Option<i32>,
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
            } => ProviderConfig::OpenAi(OpenAiProviderConfig {
                host,
                api_key,
                model,
                temperature,
                max_tokens,
            }),
            ProviderSettings::Databricks {
                host,
                token,
                model,
                temperature,
                max_tokens,
            } => ProviderConfig::Databricks(DatabricksProviderConfig {
                host,
                token,
                model,
                temperature,
                max_tokens,
            }),
            ProviderSettings::Ollama {
                host,
                model,
                temperature,
                max_tokens,
            } => ProviderConfig::Ollama(OllamaProviderConfig {
                host,
                model,
                temperature,
                max_tokens,
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

fn default_model() -> String {
    "gpt-4o".to_string()
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
        } = settings.provider
        {
            assert_eq!(host, "https://api.openai.com");
            assert_eq!(api_key, "test-key".to_string());
            assert_eq!(model, "gpt-4o");
            assert_eq!(temperature, None);
            assert_eq!(max_tokens, None);
        } else {
            panic!("Expected OpenAI provider");
        }

        // Clean up
        env::remove_var("GOOSE_PROVIDER__TYPE");
        env::remove_var("GOOSE_PROVIDER__API_KEY");
    }

    #[test]
    #[serial]
    fn test_databricks_settings() {
        clean_env();
        env::set_var("GOOSE_PROVIDER__TYPE", "databricks");
        env::set_var("GOOSE_PROVIDER__TOKEN", "test-token");
        env::set_var("GOOSE_PROVIDER__HOST", "https://custom.databricks.com");
        env::set_var("GOOSE_PROVIDER__MODEL", "llama-2-70b");
        env::set_var("GOOSE_PROVIDER__TEMPERATURE", "0.7");
        env::set_var("GOOSE_PROVIDER__MAX_TOKENS", "2000");

        let settings = Settings::new().unwrap();
        if let ProviderSettings::Databricks {
            host,
            token,
            model,
            temperature,
            max_tokens,
        } = settings.provider
        {
            assert_eq!(host, "https://custom.databricks.com");
            assert_eq!(token, "test-token");
            assert_eq!(model, "llama-2-70b");
            assert_eq!(temperature, Some(0.7));
            assert_eq!(max_tokens, Some(2000));
        } else {
            panic!("Expected Databricks provider");
        }

        // Clean up
        env::remove_var("GOOSE_PROVIDER__TYPE");
        env::remove_var("GOOSE_PROVIDER__TOKEN");
        env::remove_var("GOOSE_PROVIDER__HOST");
        env::remove_var("GOOSE_PROVIDER__MODEL");
        env::remove_var("GOOSE_PROVIDER__TEMPERATURE");
        env::remove_var("GOOSE_PROVIDER__MAX_TOKENS");
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

        let settings = Settings::new().unwrap();
        if let ProviderSettings::Ollama {
            host,
            model,
            temperature,
            max_tokens,
        } = settings.provider
        {
            assert_eq!(host, "http://custom.ollama.host");
            assert_eq!(model, "llama2");
            assert_eq!(temperature, Some(0.7));
            assert_eq!(max_tokens, Some(2000));
        } else {
            panic!("Expected Ollama provider");
        }

        // Clean up
        env::remove_var("GOOSE_PROVIDER__TYPE");
        env::remove_var("GOOSE_PROVIDER__HOST");
        env::remove_var("GOOSE_PROVIDER__MODEL");
        env::remove_var("GOOSE_PROVIDER__TEMPERATURE");
        env::remove_var("GOOSE_PROVIDER__MAX_TOKENS");
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

        let settings = Settings::new().unwrap();
        assert_eq!(settings.server.port, 8080);

        if let ProviderSettings::OpenAi {
            host,
            api_key,
            model,
            temperature,
            ..
        } = settings.provider
        {
            assert_eq!(host, "https://custom.openai.com");
            assert_eq!(api_key, "test-key");
            assert_eq!(model, "gpt-3.5-turbo");
            assert_eq!(temperature, Some(0.8));
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