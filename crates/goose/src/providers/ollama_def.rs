use std::time::Duration;

use anyhow::Result;
use futures::future::BoxFuture;
use url::Url;

use crate::{
    config::declarative_providers::DeclarativeProviderConfig, providers::base::ProviderDef,
};
use goose_providers::{
    api_client::{ApiClient, AuthMethod},
    base::ProviderDescriptor,
    ollama::{
        OllamaOptions, OllamaProvider, OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS, OLLAMA_DEFAULT_PORT,
        OLLAMA_HOST, OLLAMA_PROVIDER_NAME, OLLAMA_TIMEOUT,
    },
};

pub struct OllamaProviderDef;

impl ProviderDescriptor for OllamaProviderDef {
    fn metadata() -> goose_providers::base::ProviderMetadata {
        OllamaProvider::metadata()
    }
}

impl ProviderDef for OllamaProviderDef {
    type Provider = OllamaProvider;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(from_env(tls_config))
    }
}

pub async fn from_env(
    tls_config: Option<crate::providers::api_client::TlsConfig>,
) -> Result<OllamaProvider> {
    let config = crate::config::Config::global();
    let host: String = config
        .get_param("OLLAMA_HOST")
        .unwrap_or_else(|_| OLLAMA_HOST.to_string());

    let timeout: Duration =
        Duration::from_secs(config.get_param("OLLAMA_TIMEOUT").unwrap_or(OLLAMA_TIMEOUT));

    let base = if host.starts_with("http://") || host.starts_with("https://") {
        host.clone()
    } else {
        format!("http://{}", host)
    };

    let mut base_url = Url::parse(&base).map_err(|e| anyhow::anyhow!("Invalid base URL: {e}"))?;

    let explicit_port = host.contains(':');
    let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";

    if base_url.port().is_none() && !explicit_port && !host.starts_with("http") && is_localhost {
        base_url
            .set_port(Some(OLLAMA_DEFAULT_PORT))
            .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
    }

    let api_client = ApiClient::with_timeout_and_tls(
        base_url.to_string(),
        AuthMethod::NoAuth,
        timeout,
        tls_config,
    )?;

    Ok(OllamaProvider::new(
        api_client,
        OLLAMA_PROVIDER_NAME.to_string(),
        false,
        options_from_config(),
    ))
}

pub fn from_custom_config(
    config: DeclarativeProviderConfig,
    tls_config: Option<crate::providers::api_client::TlsConfig>,
) -> Result<OllamaProvider> {
    let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(OLLAMA_TIMEOUT));

    let base = if config.base_url.starts_with("http://") || config.base_url.starts_with("https://")
    {
        config.base_url.clone()
    } else {
        format!("http://{}", config.base_url)
    };

    let mut base_url = Url::parse(&base)
        .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

    let explicit_default_port =
        config.base_url.ends_with(":80") || config.base_url.ends_with(":443");
    let is_https = base_url.scheme() == "https";

    if base_url.port().is_none() && !explicit_default_port && !is_https {
        base_url
            .set_port(Some(OLLAMA_DEFAULT_PORT))
            .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
    }

    let mut api_client = ApiClient::with_timeout_and_tls(
        base_url.to_string(),
        AuthMethod::NoAuth,
        timeout,
        tls_config,
    )?;

    if let Some(headers) = &config.headers {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
            let header_value = reqwest::header::HeaderValue::from_str(value)?;
            header_map.insert(header_name, header_value);
        }
        api_client = api_client.with_headers(header_map)?;
    }

    let supports_streaming = config.supports_streaming.unwrap_or(true);

    if !supports_streaming {
        return Err(anyhow::anyhow!(
            "Ollama provider does not support non-streaming mode. All Ollama models support streaming. \
            Please remove 'supports_streaming: false' from your provider configuration."
        ));
    }

    Ok(OllamaProvider::new(
        api_client,
        config.name.clone(),
        config.skip_canonical_filtering,
        options_from_config(),
    ))
}

pub fn options_from_config() -> OllamaOptions {
    let config = crate::config::Config::global();

    let input_limit = match config.get_param::<usize>("GOOSE_INPUT_LIMIT") {
        Ok(limit) if limit > 0 => Some(limit),
        Ok(_) => None,
        Err(crate::config::ConfigError::NotFound(_)) => None,
        Err(e) => {
            tracing::warn!("Invalid GOOSE_INPUT_LIMIT value: {}", e);
            None
        }
    };

    let stream_usage = match config.get_param::<bool>("OLLAMA_STREAM_USAGE") {
        Ok(val) => val,
        // Key not set: default to true. Ollama supports stream_options since
        // mid-2025 and most installs benefit from token usage tracking.
        Err(crate::config::ConfigError::NotFound(_)) => true,
        // Invalid value (e.g. "0", "yes", typo): warn and disable stream_options
        // so users who intended to opt out aren't silently left hanging.
        Err(e) => {
            tracing::warn!(
                "Invalid OLLAMA_STREAM_USAGE value ({}); disabling stream_options. \
                     Use true or false.",
                e
            );
            false
        }
    };

    OllamaOptions {
        input_limit,
        stream_usage,
        chunk_timeout_secs: resolve_ollama_chunk_timeout(config),
    }
}

/// Resolve the per-chunk stream timeout from config.
/// Priority: OLLAMA_STREAM_TIMEOUT > GOOSE_STREAM_TIMEOUT > OLLAMA_TIMEOUT > default (120s).
/// Zero values are treated as invalid and skipped, since a zero timeout would
/// cause every chunk after the first to be treated as a stall.
fn resolve_ollama_chunk_timeout(config: &crate::config::Config) -> u64 {
    if let Ok(val) = config.get_param::<u64>("OLLAMA_STREAM_TIMEOUT") {
        if val > 0 {
            return val;
        }
    }
    if let Ok(val) = config.get_param::<u64>("GOOSE_STREAM_TIMEOUT") {
        if val > 0 {
            return val;
        }
    }
    match config.get_param::<u64>("OLLAMA_TIMEOUT") {
        Ok(val) if val > 0 => val,
        _ => OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_ollama_chunk_timeout_defaults_to_ollama_timeout() {
        let _guard = env_lock::lock_env([
            ("OLLAMA_STREAM_TIMEOUT", None::<&str>),
            ("GOOSE_STREAM_TIMEOUT", None::<&str>),
            ("OLLAMA_TIMEOUT", Some("300")),
        ]);
        let config = crate::config::Config::global();
        assert_eq!(resolve_ollama_chunk_timeout(config), 300);
    }

    #[test]
    fn test_resolve_ollama_chunk_timeout_prefers_stream_override() {
        let _guard = env_lock::lock_env([
            ("OLLAMA_STREAM_TIMEOUT", Some("60")),
            ("GOOSE_STREAM_TIMEOUT", Some("90")),
            ("OLLAMA_TIMEOUT", Some("300")),
        ]);
        let config = crate::config::Config::global();
        assert_eq!(resolve_ollama_chunk_timeout(config), 60);
    }

    #[test]
    fn test_resolve_ollama_chunk_timeout_uses_goose_stream_fallback() {
        let _guard = env_lock::lock_env([
            ("OLLAMA_STREAM_TIMEOUT", None::<&str>),
            ("GOOSE_STREAM_TIMEOUT", Some("90")),
            ("OLLAMA_TIMEOUT", Some("300")),
        ]);
        let config = crate::config::Config::global();
        assert_eq!(resolve_ollama_chunk_timeout(config), 90);
    }

    #[test]
    fn test_resolve_ollama_chunk_timeout_uses_default_when_unset() {
        let _guard = env_lock::lock_env([
            ("OLLAMA_STREAM_TIMEOUT", None::<&str>),
            ("GOOSE_STREAM_TIMEOUT", None::<&str>),
            ("OLLAMA_TIMEOUT", None::<&str>),
        ]);
        let config = crate::config::Config::global();
        assert_eq!(
            resolve_ollama_chunk_timeout(config),
            OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS
        );
    }

    #[test]
    fn test_resolve_ollama_chunk_timeout_skips_zero_values() {
        let _guard = env_lock::lock_env([
            ("OLLAMA_STREAM_TIMEOUT", Some("0")),
            ("GOOSE_STREAM_TIMEOUT", Some("0")),
            ("OLLAMA_TIMEOUT", Some("300")),
        ]);
        let config = crate::config::Config::global();
        assert_eq!(resolve_ollama_chunk_timeout(config), 300);
    }

    #[test]
    fn test_resolve_ollama_chunk_timeout_skips_all_zero_to_default() {
        let _guard = env_lock::lock_env([
            ("OLLAMA_STREAM_TIMEOUT", Some("0")),
            ("GOOSE_STREAM_TIMEOUT", Some("0")),
            ("OLLAMA_TIMEOUT", Some("0")),
        ]);
        let config = crate::config::Config::global();
        assert_eq!(
            resolve_ollama_chunk_timeout(config),
            OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS
        );
    }
}
