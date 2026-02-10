use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use crate::model::ModelConfig;
use anyhow::Result;
use futures::future::BoxFuture;
use serde::Deserialize;
use serde_json::Value;

const TANZU_PROVIDER_NAME: &str = "tanzu_ai";
const TANZU_DEFAULT_MODEL: &str = "openai/gpt-oss-120b";
const TANZU_DOC_URL: &str =
    "https://techdocs.broadcom.com/us/en/vmware-tanzu/platform/ai-services/10-3/ai/index.html";

/// Credentials parsed from Tanzu AI Services binding
#[derive(Debug, Clone)]
struct TanzuCredentials {
    /// The base endpoint URL (without /openai suffix)
    endpoint_base: String,
    /// JWT API key for Bearer auth
    api_key: String,
    /// Config URL for model discovery
    config_url: Option<String>,
    /// Model name (for single-model bindings; used in model discovery)
    #[allow(dead_code)]
    model_name: Option<String>,
}

/// Response from the config URL endpoint
#[derive(Debug, Deserialize)]
struct ConfigResponse {
    #[serde(default)]
    #[serde(rename = "advertisedModels")]
    advertised_models: Vec<AdvertisedModel>,
}

/// A model advertised by the config endpoint
#[derive(Debug, Deserialize)]
struct AdvertisedModel {
    name: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

pub struct TanzuAIServicesProvider;

impl ProviderDef for TanzuAIServicesProvider {
    type Provider = OpenAiCompatibleProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            TANZU_PROVIDER_NAME,
            "Tanzu AI Services",
            "LLM access via VMware Tanzu Platform AI Services (OpenAI-compatible)",
            TANZU_DEFAULT_MODEL,
            vec![TANZU_DEFAULT_MODEL],
            TANZU_DOC_URL,
            vec![
                ConfigKey::new("TANZU_AI_API_KEY", true, true, None),
                ConfigKey::new("TANZU_AI_ENDPOINT", true, false, None),
                ConfigKey::new("TANZU_AI_CONFIG_URL", false, false, None),
                ConfigKey::new("TANZU_AI_MODEL_NAME", false, false, None),
            ],
        )
        .with_unlisted_models()
    }

    fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<OpenAiCompatibleProvider>> {
        Box::pin(async move {
            let creds = resolve_credentials()?;

            // The OpenAI-compatible base URL is {endpoint_base}/openai
            let host = format!("{}/openai", creds.endpoint_base.trim_end_matches('/'));

            let api_client = ApiClient::new(host, AuthMethod::BearerToken(creds.api_key))?;

            Ok(OpenAiCompatibleProvider::new(
                TANZU_PROVIDER_NAME.to_string(),
                api_client,
                model,
                String::new(), // no extra prefix; paths are relative to host
            ))
        })
    }
}

/// Resolve credentials from environment variables or VCAP_SERVICES.
///
/// Priority:
/// 1. Explicit env vars (TANZU_AI_ENDPOINT + TANZU_AI_API_KEY)
/// 2. VCAP_SERVICES auto-detection
fn resolve_credentials() -> Result<TanzuCredentials> {
    let config = crate::config::Config::global();

    // Try explicit configuration first
    let endpoint: Result<String, _> = config.get_param("TANZU_AI_ENDPOINT");
    let api_key: Result<String, _> = config.get_secret("TANZU_AI_API_KEY");

    if let (Ok(endpoint), Ok(api_key)) = (endpoint, api_key) {
        let config_url: Option<String> = config.get_param("TANZU_AI_CONFIG_URL").ok();
        let model_name: Option<String> = config.get_param("TANZU_AI_MODEL_NAME").ok();

        return Ok(TanzuCredentials {
            endpoint_base: endpoint,
            api_key,
            config_url,
            model_name,
        });
    }

    // Try VCAP_SERVICES
    if let Ok(vcap) = std::env::var("VCAP_SERVICES") {
        if let Some(creds) = parse_vcap_services(&vcap) {
            return Ok(creds);
        }
    }

    anyhow::bail!(
        "Tanzu AI Services credentials not found. Set TANZU_AI_ENDPOINT and TANZU_AI_API_KEY, \
         or run on Cloud Foundry with a bound genai service instance."
    )
}

/// Parse credentials from the VCAP_SERVICES environment variable.
///
/// Looks for `genai` service bindings and supports both single-model
/// and multi-model credential formats.
fn parse_vcap_services(vcap_json: &str) -> Option<TanzuCredentials> {
    let vcap: Value = serde_json::from_str(vcap_json).ok()?;
    let genai_bindings = vcap.get("genai")?.as_array()?;

    // Check for a specific binding name override
    let binding_name = std::env::var("TANZU_AI_BINDING_NAME").ok();

    let binding = if let Some(ref name) = binding_name {
        genai_bindings.iter().find(|b| {
            b.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == name.as_str())
                .unwrap_or(false)
        })?
    } else {
        genai_bindings.first()?
    };

    let creds = binding.get("credentials")?;
    parse_binding_credentials(creds)
}

/// Parse credentials from a single binding's credentials object.
///
/// Handles both formats:
/// - Multi-model: only `endpoint` block present
/// - Single-model: top-level `api_base`, `model_name`, and optionally `endpoint`
fn parse_binding_credentials(creds: &Value) -> Option<TanzuCredentials> {
    // Try multi-model format first (recommended): only endpoint block
    if let Some(endpoint) = creds.get("endpoint") {
        let endpoint_base = endpoint.get("api_base")?.as_str()?.to_string();
        let api_key = endpoint.get("api_key")?.as_str()?.to_string();
        let config_url = endpoint
            .get("config_url")
            .and_then(|v| v.as_str())
            .map(String::from);

        // If model_name exists at top level, this is single-model format with endpoint block
        let model_name = creds
            .get("model_name")
            .and_then(|v| v.as_str())
            .map(String::from);

        return Some(TanzuCredentials {
            endpoint_base,
            api_key,
            config_url,
            model_name,
        });
    }

    // Fall back to single-model format (deprecated): top-level api_base with /openai suffix
    let api_base = creds.get("api_base")?.as_str()?;
    let api_key = creds.get("api_key")?.as_str()?.to_string();
    let model_name = creds
        .get("model_name")
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(TanzuCredentials {
        endpoint_base: strip_openai_suffix(api_base),
        api_key,
        config_url: None,
        model_name,
    })
}

/// Strip the `/openai` suffix from a single-model format `api_base`.
fn strip_openai_suffix(api_base: &str) -> String {
    api_base
        .trim_end_matches('/')
        .trim_end_matches("/openai")
        .to_string()
}

/// Discover available models from the config URL endpoint.
///
/// The config URL returns metadata including advertised models with their capabilities.
/// Falls back to the OpenAI `/v1/models` endpoint if the config URL is unavailable.
#[allow(dead_code)]
async fn discover_models(creds: &TanzuCredentials) -> Result<Vec<AdvertisedModel>> {
    let client = reqwest::Client::new();

    // Try config URL first for rich metadata
    if let Some(config_url) = &creds.config_url {
        let response = client
            .get(config_url)
            .bearer_auth(&creds.api_key)
            .send()
            .await;

        if let Ok(resp) = response {
            if resp.status().is_success() {
                if let Ok(config) = resp.json::<ConfigResponse>().await {
                    if !config.advertised_models.is_empty() {
                        return Ok(config.advertised_models);
                    }
                }
            }
        }
    }

    // Fall back to OpenAI /v1/models endpoint
    let models_url = format!(
        "{}/openai/v1/models",
        creds.endpoint_base.trim_end_matches('/')
    );
    let response = client
        .get(&models_url)
        .bearer_auth(&creds.api_key)
        .send()
        .await?;

    let json: Value = response.json().await?;
    let models = json
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    Some(AdvertisedModel {
                        name: m.get("id")?.as_str()?.to_string(),
                        capabilities: vec!["CHAT".to_string()],
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}

/// Filter models to only those with chat or tool capabilities.
#[allow(dead_code)]
fn filter_chat_models(models: &[AdvertisedModel]) -> Vec<String> {
    models
        .iter()
        .filter(|m| {
            m.capabilities.iter().any(|c| {
                c.eq_ignore_ascii_case("chat")
                    || c.eq_ignore_ascii_case("tools")
                    || c.eq_ignore_ascii_case("completion")
            })
        })
        .map(|m| m.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Credential Parsing Tests ---

    #[test]
    fn test_parse_single_model_credentials() {
        let json = serde_json::json!({
            "api_base": "https://genai-proxy.sys.example.com/tanzu-gpt-oss-120b-v1025-eaf66e7/openai",
            "api_key": "eyJhbGciOiJIUzI1NiJ9.test",
            "endpoint": {
                "api_base": "https://genai-proxy.sys.example.com/tanzu-gpt-oss-120b-v1025-eaf66e7",
                "api_key": "eyJhbGciOiJIUzI1NiJ9.test",
                "config_url": "https://genai-proxy.sys.example.com/tanzu-gpt-oss-120b-v1025-eaf66e7/config/v1/endpoint",
                "name": "tanzu-gpt-oss-120b-v1025-eaf66e7"
            },
            "model_aliases": null,
            "model_capabilities": ["chat", "tools"],
            "model_name": "openai/gpt-oss-120b",
            "wire_format": "openai"
        });

        let creds = parse_binding_credentials(&json).unwrap();
        assert_eq!(
            creds.endpoint_base,
            "https://genai-proxy.sys.example.com/tanzu-gpt-oss-120b-v1025-eaf66e7"
        );
        assert_eq!(creds.api_key, "eyJhbGciOiJIUzI1NiJ9.test");
        assert_eq!(creds.model_name, Some("openai/gpt-oss-120b".to_string()));
        assert!(creds.config_url.is_some());
        assert_eq!(
            creds.config_url.unwrap(),
            "https://genai-proxy.sys.example.com/tanzu-gpt-oss-120b-v1025-eaf66e7/config/v1/endpoint"
        );
    }

    #[test]
    fn test_parse_multi_model_credentials() {
        let json = serde_json::json!({
            "endpoint": {
                "api_base": "https://genai-proxy.sys.example.com/tanzu-all-models-1a56b7a",
                "api_key": "eyJhbGciOiJIUzI1NiJ9.multi",
                "config_url": "https://genai-proxy.sys.example.com/tanzu-all-models-1a56b7a/config/v1/endpoint",
                "name": "tanzu-all-models-1a56b7a"
            }
        });

        let creds = parse_binding_credentials(&json).unwrap();
        assert_eq!(
            creds.endpoint_base,
            "https://genai-proxy.sys.example.com/tanzu-all-models-1a56b7a"
        );
        assert_eq!(creds.api_key, "eyJhbGciOiJIUzI1NiJ9.multi");
        assert_eq!(creds.model_name, None);
        assert!(creds.config_url.is_some());
    }

    #[test]
    fn test_parse_deprecated_single_model_no_endpoint() {
        let json = serde_json::json!({
            "api_base": "https://genai-proxy.sys.example.com/some-guid/openai",
            "api_key": "eyJhbGciOiJIUzI1NiJ9.deprecated",
            "model_name": "llama3:8b",
            "model_capabilities": ["chat"],
            "wire_format": "openai"
        });

        let creds = parse_binding_credentials(&json).unwrap();
        assert_eq!(
            creds.endpoint_base,
            "https://genai-proxy.sys.example.com/some-guid"
        );
        assert_eq!(creds.api_key, "eyJhbGciOiJIUzI1NiJ9.deprecated");
        assert_eq!(creds.model_name, Some("llama3:8b".to_string()));
        assert!(creds.config_url.is_none());
    }

    // --- URL Construction Tests ---

    #[test]
    fn test_strip_openai_suffix() {
        assert_eq!(
            strip_openai_suffix("https://proxy.example.com/guid/openai"),
            "https://proxy.example.com/guid"
        );
        assert_eq!(
            strip_openai_suffix("https://proxy.example.com/guid/openai/"),
            "https://proxy.example.com/guid"
        );
        assert_eq!(
            strip_openai_suffix("https://proxy.example.com/guid"),
            "https://proxy.example.com/guid"
        );
    }

    #[test]
    fn test_openai_base_url_construction() {
        let endpoint_base = "https://genai-proxy.sys.example.com/tanzu-all-models-1a56b7a";
        let host = format!("{}/openai", endpoint_base.trim_end_matches('/'));
        assert_eq!(
            host,
            "https://genai-proxy.sys.example.com/tanzu-all-models-1a56b7a/openai"
        );
    }

    // --- VCAP_SERVICES Parsing Tests ---

    #[test]
    fn test_parse_vcap_services_multi_model() {
        let vcap = serde_json::json!({
            "genai": [{
                "binding_guid": "162e78b4-408b-4bdd-8df3-0ae1e4d6d13b",
                "binding_name": null,
                "credentials": {
                    "endpoint": {
                        "api_base": "https://genai-proxy.sys.example.com/all-models-9afff1f",
                        "api_key": "eyJhbGciOiJIUzI1NiJ9.vcap",
                        "config_url": "https://genai-proxy.sys.example.com/all-models-9afff1f/config/v1/endpoint",
                        "name": "all-models-9afff1f"
                    }
                },
                "instance_guid": "5008a1ec-c406-4ee8-8f9d-56c723af2f1f",
                "instance_name": "all-models",
                "label": "genai",
                "name": "all-models",
                "plan": "all-models",
                "tags": ["genai", "llm"]
            }]
        });

        let creds = parse_vcap_services(&vcap.to_string()).unwrap();
        assert_eq!(
            creds.endpoint_base,
            "https://genai-proxy.sys.example.com/all-models-9afff1f"
        );
        assert_eq!(creds.api_key, "eyJhbGciOiJIUzI1NiJ9.vcap");
        assert!(creds.config_url.is_some());
        assert_eq!(creds.model_name, None);
    }

    #[test]
    fn test_parse_vcap_services_no_genai() {
        let vcap = serde_json::json!({
            "mysql": [{
                "credentials": {"uri": "mysql://localhost"}
            }]
        });

        assert!(parse_vcap_services(&vcap.to_string()).is_none());
    }

    #[test]
    fn test_parse_vcap_services_empty_genai() {
        let vcap = serde_json::json!({
            "genai": []
        });

        assert!(parse_vcap_services(&vcap.to_string()).is_none());
    }

    #[test]
    fn test_parse_vcap_services_invalid_json() {
        assert!(parse_vcap_services("not json").is_none());
    }

    // --- Model Discovery Tests ---

    #[test]
    fn test_filter_chat_models() {
        let models = vec![
            AdvertisedModel {
                name: "llama3.2:1b".to_string(),
                capabilities: vec!["CHAT".to_string(), "TOOLS".to_string()],
            },
            AdvertisedModel {
                name: "mxbai-embed-large".to_string(),
                capabilities: vec!["EMBEDDING".to_string()],
            },
            AdvertisedModel {
                name: "qwen3-30b".to_string(),
                capabilities: vec!["chat".to_string()],
            },
        ];

        let chat_models = filter_chat_models(&models);
        assert_eq!(chat_models.len(), 2);
        assert!(chat_models.contains(&"llama3.2:1b".to_string()));
        assert!(chat_models.contains(&"qwen3-30b".to_string()));
        assert!(!chat_models.contains(&"mxbai-embed-large".to_string()));
    }

    #[test]
    fn test_parse_config_response() {
        let json = r#"{
            "name": "all-models-9afff1f",
            "advertisedModels": [
                {"name": "llama3.2:1b", "capabilities": ["CHAT", "TOOLS"]},
                {"name": "mxbai-embed-large", "capabilities": ["EMBEDDING"]}
            ]
        }"#;

        let config: ConfigResponse = serde_json::from_str(json).unwrap();
        assert_eq!(config.advertised_models.len(), 2);
        assert_eq!(config.advertised_models[0].name, "llama3.2:1b");
        assert_eq!(
            config.advertised_models[0].capabilities,
            vec!["CHAT", "TOOLS"]
        );
    }

    // --- Format Detection Tests ---

    #[test]
    fn test_format_detection_single_model_with_endpoint() {
        // v10.3+ format: has both model_name and endpoint
        let json = serde_json::json!({
            "api_base": "https://proxy.example.com/guid/openai",
            "api_key": "key",
            "endpoint": {
                "api_base": "https://proxy.example.com/guid",
                "api_key": "key",
                "config_url": "https://proxy.example.com/guid/config/v1/endpoint",
                "name": "guid"
            },
            "model_name": "openai/gpt-oss-120b",
            "model_capabilities": ["chat", "tools"],
            "wire_format": "openai"
        });

        let creds = parse_binding_credentials(&json).unwrap();
        // Should prefer endpoint.api_base and have model_name
        assert_eq!(creds.endpoint_base, "https://proxy.example.com/guid");
        assert_eq!(creds.model_name, Some("openai/gpt-oss-120b".to_string()));
    }

    #[test]
    fn test_format_detection_multi_model_only() {
        let json = serde_json::json!({
            "endpoint": {
                "api_base": "https://proxy.example.com/plan",
                "api_key": "key",
                "config_url": "https://proxy.example.com/plan/config/v1/endpoint",
                "name": "plan"
            }
        });

        let creds = parse_binding_credentials(&json).unwrap();
        assert_eq!(creds.endpoint_base, "https://proxy.example.com/plan");
        assert_eq!(creds.model_name, None);
    }

    // --- Provider Metadata Tests ---

    #[test]
    fn test_provider_metadata() {
        let meta = TanzuAIServicesProvider::metadata();
        assert_eq!(meta.name, "tanzu_ai");
        assert_eq!(meta.display_name, "Tanzu AI Services");
        assert!(meta.allows_unlisted_models);

        // Check required config keys
        let api_key = meta
            .config_keys
            .iter()
            .find(|k| k.name == "TANZU_AI_API_KEY")
            .unwrap();
        assert!(api_key.required);
        assert!(api_key.secret);

        let endpoint = meta
            .config_keys
            .iter()
            .find(|k| k.name == "TANZU_AI_ENDPOINT")
            .unwrap();
        assert!(endpoint.required);
        assert!(!endpoint.secret);

        let config_url = meta
            .config_keys
            .iter()
            .find(|k| k.name == "TANZU_AI_CONFIG_URL")
            .unwrap();
        assert!(!config_url.required);
    }
}
