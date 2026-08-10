use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::openai_compatible::{
    handle_response_openai_compat, handle_status, map_http_error_to_provider_error,
    stream_openai_compat,
};
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use goose_providers::errors::ProviderError;
use goose_providers::images::ImageFormat;

use goose_providers::formats::openai::create_request;
use goose_providers::model::ModelConfig;
use goose_providers::request_log::{start_log, LoggerHandleExt};
use rmcp::model::Tool;
use serde_json::Value;

pub const AVOCADO_PROVIDER_NAME: &str = "avocado";
pub const AVOCADO_DOC_URL: &str = "https://dev.avocado.tech/llm-api";
pub const AVOCADO_BILLING_URL: &str = "https://dev.avocado.tech/llm-api/billing";
pub const AVOCADO_DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4.6";
pub const AVOCADO_DEFAULT_HOST: &str = "https://dev.avocado.tech/llm";

const BUDGET_MARKERS: &[&str] = &[
    "exceededbudget",
    "exceededtokenbudget",
    "budget has been exceeded",
    "budget_exceeded",
    "over budget",
];

pub const AVOCADO_KNOWN_MODELS: &[&str] = &[
    "anthropic/claude-sonnet-4.6",
    "anthropic/claude-opus-4.1",
    "openai/gpt-4.1",
    "google/gemini-2.5-pro",
];

#[derive(serde::Serialize)]
pub struct AvocadoProvider {
    #[serde(skip)]
    api_client: ApiClient,
    #[serde(skip)]
    name: String,
}

impl AvocadoProvider {
    pub async fn from_env(
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("AVOCADO_API_KEY")?;
        let host: String = config
            .get_param("AVOCADO_HOST")
            .unwrap_or_else(|_| AVOCADO_DEFAULT_HOST.to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new_with_tls(host, auth, tls_config)?
            .with_request_builder(crate::session_context::session_id_request_builder())
            .with_header("HTTP-Referer", "https://goose-docs.ai")?
            .with_header("X-Title", "goose")?;

        Ok(Self {
            api_client,
            name: AVOCADO_PROVIDER_NAME.to_string(),
        })
    }

    fn enrich_credits_error(err: ProviderError) -> ProviderError {
        match err {
            ProviderError::CreditsExhausted { details, .. } => ProviderError::CreditsExhausted {
                details,
                top_up_url: Some(AVOCADO_BILLING_URL.to_string()),
            },
            other => other,
        }
    }

    fn details_indicate_budget_exceeded(details: &str) -> bool {
        let lower = details.to_ascii_lowercase();
        BUDGET_MARKERS.iter().any(|marker| lower.contains(marker))
    }

    /// Map budget-marker 429s to CreditsExhausted, enrich 402 CreditsExhausted with billing URL.
    fn classify_budget_error(err: ProviderError) -> ProviderError {
        match err {
            ProviderError::RateLimitExceeded { details, .. }
                if Self::details_indicate_budget_exceeded(&details) =>
            {
                ProviderError::CreditsExhausted {
                    details,
                    top_up_url: Some(AVOCADO_BILLING_URL.to_string()),
                }
            }
            other => Self::enrich_credits_error(other),
        }
    }

    fn error_from_avocado_error_payload(payload: Value, url: &str) -> ProviderError {
        let code = payload
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_u64())
            .unwrap_or(500) as u16;
        let status = reqwest::StatusCode::from_u16(code)
            .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        Self::classify_budget_error(map_http_error_to_provider_error(status, Some(payload), url))
    }
}

impl goose_providers::base::ProviderDescriptor for AvocadoProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            AVOCADO_PROVIDER_NAME,
            "Avocado LLM API",
            "AVCD OpenAI-compatible LLM gateway",
            AVOCADO_DEFAULT_MODEL,
            AVOCADO_KNOWN_MODELS.to_vec(),
            AVOCADO_DOC_URL,
            vec![
                ConfigKey::new("AVOCADO_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "AVOCADO_HOST",
                    false,
                    false,
                    Some(AVOCADO_DEFAULT_HOST),
                    false,
                ),
            ],
        )
    }
}

impl ProviderDef for AvocadoProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(tls_config))
    }
}

#[async_trait]
impl Provider for AvocadoProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;

        let mut log = start_log(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .request("v1/chat/completions")
                    .model_headers(model_config)?
                    .streaming(true)
                    .response_post(&payload)
                    .await?;
                let resp = handle_status(resp)
                    .await
                    .map_err(Self::classify_budget_error)?;

                let is_json = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.to_ascii_lowercase())
                    .is_some_and(|v| v.contains("json"));

                if is_json {
                    let body = goose_providers::http_status::read_error_body(resp)
                        .await
                        .unwrap_or_default();
                    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&body) {
                        if payload.get("error").is_some() {
                            return Err(Self::error_from_avocado_error_payload(
                                payload,
                                "v1/chat/completions",
                            ));
                        }
                    }

                    return Err(ProviderError::ExecutionError(format!(
                        "Expected streaming response but received non-streaming payload: {body}"
                    )));
                }

                Ok(resp)
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        stream_openai_compat(response, log)
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .response_get("v1/models")
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
        let json = handle_response_openai_compat(response)
            .await
            .map_err(Self::classify_budget_error)?;

        if json.get("error").is_some() {
            return Err(Self::error_from_avocado_error_payload(json, "v1/models"));
        }

        let arr = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::RequestFailed("Missing 'data' array in models response".to_string())
        })?;
        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose_providers::base::ProviderDescriptor as _;

    #[test]
    fn given_429_exceeded_budget_when_classify_then_credits_exhausted_with_billing_url() {
        let err = ProviderError::RateLimitExceeded {
            details: "ExceededBudget: monthly token allotment used".to_string(),
            retry_delay: None,
        };
        match AvocadoProvider::classify_budget_error(err) {
            ProviderError::CreditsExhausted {
                details,
                top_up_url,
            } => {
                assert!(details.to_ascii_lowercase().contains("exceededbudget"));
                assert_eq!(top_up_url.as_deref(), Some(AVOCADO_BILLING_URL));
            }
            other => panic!("Expected CreditsExhausted, got {other:?}"),
        }
    }

    #[test]
    fn given_429_budget_has_been_exceeded_when_classify_then_credits_exhausted() {
        let err = ProviderError::RateLimitExceeded {
            details: "Budget has been exceeded for this organization".to_string(),
            retry_delay: None,
        };
        match AvocadoProvider::classify_budget_error(err) {
            ProviderError::CreditsExhausted { top_up_url, .. } => {
                assert_eq!(top_up_url.as_deref(), Some(AVOCADO_BILLING_URL));
            }
            other => panic!("Expected CreditsExhausted, got {other:?}"),
        }
    }

    #[test]
    fn given_429_uppercase_budget_exceeded_when_classify_then_credits_exhausted() {
        let err = ProviderError::RateLimitExceeded {
            details: "BUDGET_EXCEEDED".to_string(),
            retry_delay: None,
        };
        match AvocadoProvider::classify_budget_error(err) {
            ProviderError::CreditsExhausted { top_up_url, .. } => {
                assert_eq!(top_up_url.as_deref(), Some(AVOCADO_BILLING_URL));
            }
            other => panic!("Expected CreditsExhausted, got {other:?}"),
        }
    }

    #[test]
    fn given_429_rate_limit_when_classify_then_stays_rate_limit_exceeded() {
        let err = ProviderError::RateLimitExceeded {
            details: "Rate limit exceeded".to_string(),
            retry_delay: None,
        };
        match AvocadoProvider::classify_budget_error(err) {
            ProviderError::RateLimitExceeded { details, .. } => {
                assert!(details.contains("Rate limit exceeded"));
            }
            other => panic!("Expected RateLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn given_402_credits_exhausted_without_url_when_enrich_then_adds_billing_url() {
        let err = ProviderError::CreditsExhausted {
            details: "out of credits".to_string(),
            top_up_url: None,
        };
        match AvocadoProvider::classify_budget_error(err) {
            ProviderError::CreditsExhausted {
                details,
                top_up_url,
            } => {
                assert_eq!(details, "out of credits");
                assert_eq!(top_up_url.as_deref(), Some(AVOCADO_BILLING_URL));
            }
            other => panic!("Expected CreditsExhausted, got {other:?}"),
        }
    }

    #[test]
    fn given_server_error_when_classify_then_passes_through_unchanged() {
        let err = ProviderError::ServerError("boom".to_string());
        assert!(matches!(
            AvocadoProvider::classify_budget_error(err),
            ProviderError::ServerError(msg) if msg == "boom"
        ));
    }

    #[test]
    fn given_metadata_when_read_then_api_key_required_secret_primary_and_host_default() {
        let meta = AvocadoProvider::metadata();
        let api_key = meta
            .config_keys
            .iter()
            .find(|k| k.name == "AVOCADO_API_KEY")
            .expect("AVOCADO_API_KEY config key");
        assert!(api_key.required);
        assert!(api_key.secret);
        assert!(api_key.primary);

        let host = meta
            .config_keys
            .iter()
            .find(|k| k.name == "AVOCADO_HOST")
            .expect("AVOCADO_HOST config key");
        assert!(!host.required);
        assert_eq!(host.default.as_deref(), Some(AVOCADO_DEFAULT_HOST));
    }
}
