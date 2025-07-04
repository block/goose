use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryProvider {
    Datadog,
    Otlp,
    Console,
    File,
}

impl Default for TelemetryProvider {
    fn default() -> Self {
        TelemetryProvider::Console
    }
}

impl std::str::FromStr for TelemetryProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "datadog" => Ok(TelemetryProvider::Datadog),
            "otlp" => Ok(TelemetryProvider::Otlp),
            "console" => Ok(TelemetryProvider::Console),
            "file" => Ok(TelemetryProvider::File),
            _ => Err(format!("Unknown telemetry provider: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageType {
    Human,
    Automation,
    Ci,
}

impl Default for UsageType {
    fn default() -> Self {
        UsageType::Human
    }
}

impl std::str::FromStr for UsageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" => Ok(UsageType::Human),
            "automation" => Ok(UsageType::Automation),
            "ci" => Ok(UsageType::Ci),
            _ => Err(format!("Unknown usage type: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub provider: TelemetryProvider,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub usage_type: Option<UsageType>,
    pub environment: Option<String>,
    pub service_name: String,
    pub service_version: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: TelemetryProvider::default(),
            endpoint: None,
            api_key: None,
            usage_type: None,
            environment: None,
            service_name: "goose".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl TelemetryConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        config.enabled = env::var("GOOSE_TELEMETRY_ENABLED")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        config.provider = env::var("GOOSE_TELEMETRY_PROVIDER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_default();

        config.endpoint = env::var("GOOSE_TELEMETRY_ENDPOINT").ok();
        config.api_key = env::var("GOOSE_TELEMETRY_API_KEY").ok();

        config.usage_type = env::var("GOOSE_USAGE_TYPE")
            .ok()
            .and_then(|v| v.parse().ok());

        config.environment = env::var("GOOSE_ENVIRONMENT").ok();

        if let Ok(service_name) = env::var("OTEL_SERVICE_NAME") {
            config.service_name = service_name;
        }

        config
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        match self.provider {
            TelemetryProvider::Datadog => {
                if self.api_key.is_none() {
                    return Err(
                        "Datadog provider requires GOOSE_TELEMETRY_API_KEY or DD_API_KEY"
                            .to_string(),
                    );
                }
            }
            TelemetryProvider::Otlp => {
                if self.endpoint.is_none() {
                    return Err("OTLP provider requires GOOSE_TELEMETRY_ENDPOINT or OTEL_EXPORTER_OTLP_ENDPOINT".to_string());
                }
            }
            TelemetryProvider::File => {
                if self.endpoint.is_none() {
                    return Err("File provider requires GOOSE_TELEMETRY_ENDPOINT (file path)".to_string());
                }
            }
            TelemetryProvider::Console => {}
        }

        Ok(())
    }

    pub fn get_endpoint(&self) -> Option<String> {
        match self.provider {
            TelemetryProvider::Datadog => self
                .endpoint
                .clone()
                .or_else(|| Some("https://api.datadoghq.com".to_string())),
            TelemetryProvider::Otlp => self.endpoint.clone(),
            TelemetryProvider::File => self.endpoint.clone(),
            TelemetryProvider::Console => None,
        }
    }
}

fn extract_api_key_from_headers(headers: &str) -> Option<String> {
    for header in headers.split(',') {
        let parts: Vec<&str> = header.split('=').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().to_lowercase();
            if key.contains("api") && key.contains("key") {
                return Some(parts[1].trim().to_string());
            }
            if key == "authorization" {
                return Some(parts[1].trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_telemetry_provider_parsing() {
        assert_eq!(
            "datadog".parse::<TelemetryProvider>().unwrap(),
            TelemetryProvider::Datadog
        );
        assert_eq!(
            "otlp".parse::<TelemetryProvider>().unwrap(),
            TelemetryProvider::Otlp
        );
        assert_eq!(
            "console".parse::<TelemetryProvider>().unwrap(),
            TelemetryProvider::Console
        );
        assert_eq!(
            "file".parse::<TelemetryProvider>().unwrap(),
            TelemetryProvider::File
        );
        assert!("invalid".parse::<TelemetryProvider>().is_err());
    }

    #[test]
    fn test_usage_type_parsing() {
        assert_eq!("human".parse::<UsageType>().unwrap(), UsageType::Human);
        assert_eq!(
            "automation".parse::<UsageType>().unwrap(),
            UsageType::Automation
        );
        assert_eq!("ci".parse::<UsageType>().unwrap(), UsageType::Ci);
        assert!("invalid".parse::<UsageType>().is_err());
    }

    #[test]
    fn test_config_from_env() {
        let config = TelemetryConfig::from_env();
        assert!(!config.enabled);

        env::set_var("GOOSE_TELEMETRY_ENABLED", "true");
        env::set_var("GOOSE_TELEMETRY_PROVIDER", "datadog");
        env::set_var("GOOSE_TELEMETRY_API_KEY", "test-key");

        let config = TelemetryConfig::from_env();
        assert!(config.enabled);
        assert_eq!(config.provider, TelemetryProvider::Datadog);
        assert_eq!(config.api_key, Some("test-key".to_string()));

        env::remove_var("GOOSE_TELEMETRY_ENABLED");
        env::remove_var("GOOSE_TELEMETRY_PROVIDER");
        env::remove_var("GOOSE_TELEMETRY_API_KEY");
    }

    #[test]
    fn test_config_validation() {
        let mut config = TelemetryConfig::default();

        assert!(config.validate().is_ok());

        config.enabled = true;
        config.provider = TelemetryProvider::Datadog;
        assert!(config.validate().is_err());

        config.api_key = Some("test-key".to_string());
        assert!(config.validate().is_ok());

        config.provider = TelemetryProvider::Otlp;
        config.api_key = None;
        assert!(config.validate().is_err());

        config.endpoint = Some("http://localhost:4317".to_string());
        assert!(config.validate().is_ok());

        config.provider = TelemetryProvider::File;
        config.endpoint = None;
        assert!(config.validate().is_err());

        config.endpoint = Some("/tmp/telemetry.log".to_string());
        assert!(config.validate().is_ok());

        config.provider = TelemetryProvider::Console;
        config.endpoint = None;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_extract_api_key_from_headers() {
        assert_eq!(
            extract_api_key_from_headers("api-key=test123,other=value"),
            Some("test123".to_string())
        );
        assert_eq!(
            extract_api_key_from_headers("authorization=Bearer token123"),
            Some("Bearer token123".to_string())
        );
        assert_eq!(extract_api_key_from_headers("other=value"), None);
    }
}
