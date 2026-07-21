use crate::config::tls::provider_tls_config_from_config;
use crate::config::Config;
#[cfg(feature = "local-inference")]
use crate::dictation::whisper::LOCAL_WHISPER_MODEL_CONFIG_KEY;
use crate::providers::api_client::{ApiClient, AuthMethod};
use crate::providers::azureauth::AzureAuth;
use crate::providers::openai::parse_openai_base_url;
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(feature = "local-inference")]
use std::sync::Mutex;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const OPENAI_VERSIONLESS_TRANSCRIPTIONS_PATH: &str = "audio/transcriptions";
type OpenAiDictationTarget = (String, Vec<(String, String)>, String);

#[cfg(feature = "local-inference")]
static LOCAL_TRANSCRIBER: once_cell::sync::Lazy<
    Mutex<Option<(String, super::whisper::WhisperTranscriber)>>,
> = once_cell::sync::Lazy::new(|| Mutex::new(None));

#[cfg(feature = "local-inference")]
const WHISPER_TOKENIZER_JSON: &str = include_str!("whisper_data/tokens.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DictationProvider {
    OpenAI,
    ElevenLabs,
    Groq,
    #[serde(rename = "azure_foundry")]
    AzureFoundry,
    #[cfg(feature = "local-inference")]
    Local,
}

pub struct DictationProviderDef {
    pub provider: DictationProvider,
    pub config_key: &'static str,
    pub default_base_url: &'static str,
    pub endpoint_path: &'static str,
    pub host_key: Option<&'static str>,
    pub description: &'static str,
    pub uses_provider_config: bool,
    pub settings_path: Option<&'static str>,
}

pub const PROVIDERS: &[DictationProviderDef] = &[
    DictationProviderDef {
        provider: DictationProvider::OpenAI,
        config_key: "OPENAI_API_KEY",
        default_base_url: "https://api.openai.com",
        endpoint_path: "v1/audio/transcriptions",
        host_key: Some("OPENAI_HOST"),
        description: "Uses OpenAI Whisper API for high-quality transcription.",
        uses_provider_config: true,
        settings_path: Some("Settings > Models"),
    },
    DictationProviderDef {
        provider: DictationProvider::Groq,
        config_key: "GROQ_API_KEY",
        default_base_url: "https://api.groq.com/openai/v1",
        endpoint_path: "audio/transcriptions",
        host_key: None,
        description: "Uses Groq's ultra-fast Whisper implementation with LPU acceleration.",
        uses_provider_config: false,
        settings_path: None,
    },
    DictationProviderDef {
        provider: DictationProvider::ElevenLabs,
        config_key: "ELEVENLABS_API_KEY",
        default_base_url: "https://api.elevenlabs.io",
        endpoint_path: "v1/speech-to-text",
        host_key: None,
        description: "Uses ElevenLabs speech-to-text API for advanced voice processing.",
        uses_provider_config: false,
        settings_path: None,
    },
    DictationProviderDef {
        provider: DictationProvider::AzureFoundry,
        config_key: "AZURE_SPEECH_KEY",
        default_base_url: "",
        endpoint_path: "speechtotext/transcriptions:transcribe",
        host_key: Some("AZURE_SPEECH_ENDPOINT"),
        description: "Uses Azure AI Foundry speech-to-text via the Fast Transcription API. \
                      Set AZURE_SPEECH_ENDPOINT to the cognitiveservices URL (Azure portal → \
                      AI Foundry Hub → AI Services resource → Keys and Endpoint). \
                      The API key is the same as AZURE_FOUNDRY_API_KEY — no extra key needed.",
        uses_provider_config: false,
        settings_path: None,
    },
];

#[cfg(feature = "local-inference")]
pub const LOCAL_PROVIDER_DEF: DictationProviderDef = DictationProviderDef {
    provider: DictationProvider::Local,
    config_key: LOCAL_WHISPER_MODEL_CONFIG_KEY,
    default_base_url: "",
    endpoint_path: "",
    host_key: None,
    description: "Uses local Whisper model for transcription. No API key needed.",
    uses_provider_config: false,
    settings_path: None,
};

/// Returns all provider definitions, including Local when the `local-inference` feature is enabled.
pub fn all_providers() -> Vec<&'static DictationProviderDef> {
    #[cfg(not(feature = "local-inference"))]
    {
        PROVIDERS.iter().collect()
    }
    #[cfg(feature = "local-inference")]
    {
        let mut all: Vec<&DictationProviderDef> = PROVIDERS.iter().collect();
        all.push(&LOCAL_PROVIDER_DEF);
        all
    }
}

pub fn get_provider_def(provider: DictationProvider) -> &'static DictationProviderDef {
    #[cfg(feature = "local-inference")]
    if provider == DictationProvider::Local {
        return &LOCAL_PROVIDER_DEF;
    }
    PROVIDERS
        .iter()
        .find(|def| def.provider == provider)
        .unwrap()
}

pub fn is_configured(provider: DictationProvider) -> bool {
    let config = Config::global();

    match provider {
        #[cfg(feature = "local-inference")]
        DictationProvider::Local => config
            .get(LOCAL_WHISPER_MODEL_CONFIG_KEY, false)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .and_then(|id| super::whisper::get_model(&id))
            .is_some_and(|m| m.is_downloaded()),
        DictationProvider::AzureFoundry => azure_speech_endpoint(config).is_ok(),
        _ => {
            let def = get_provider_def(provider);
            config.get_secret::<String>(def.config_key).is_ok()
        }
    }
}

#[cfg(feature = "local-inference")]
pub async fn transcribe_local(audio_bytes: Vec<u8>) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let config = Config::global();
        let model_id = config
            .get(LOCAL_WHISPER_MODEL_CONFIG_KEY, false)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Local Whisper model not configured"))?;

        let model = super::whisper::get_model(&model_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", model_id))?;
        let model_path = model.local_path();

        let mut transcriber_lock = LOCAL_TRANSCRIBER
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock transcriber: {}", e))?;

        let model_path_str = model_path.to_string_lossy().to_string();
        let needs_reload = match transcriber_lock.as_ref() {
            None => true,
            Some((cached_path, _)) => cached_path != &model_path_str,
        };

        if needs_reload {
            tracing::info!("Loading Whisper model from: {}", model_path.display());

            let transcriber = super::whisper::WhisperTranscriber::new_with_tokenizer(
                &model_id,
                &model_path,
                WHISPER_TOKENIZER_JSON,
            )?;

            *transcriber_lock = Some((model_path_str, transcriber));
        }

        let (_, transcriber) = transcriber_lock.as_mut().unwrap();
        let text = transcriber.transcribe(&audio_bytes).map_err(|e| {
            tracing::error!("Transcription failed: {}", e);
            e
        })?;

        Ok(text)
    })
    .await
    .map_err(|e| {
        tracing::error!("Transcription task failed: {}", e);
        anyhow::anyhow!(e)
    })?
}

fn openai_dictation_target(raw_url: &str) -> Result<OpenAiDictationTarget> {
    let (host, query_params, has_v1) = parse_openai_base_url(raw_url)?;
    let endpoint_path = if has_v1 {
        "v1/audio/transcriptions".to_string()
    } else {
        OPENAI_VERSIONLESS_TRANSCRIPTIONS_PATH.to_string()
    };
    Ok((host, query_params, endpoint_path))
}

fn resolve_openai_base_url_target(raw_url: Option<&str>) -> Result<Option<OpenAiDictationTarget>> {
    raw_url
        .map(str::trim)
        .filter(|raw_url| !raw_url.is_empty())
        .map(openai_dictation_target)
        .transpose()
}

const AZURE_SPEECH_API_VERSION: &str = "2024-11-15";
const AZURE_SPEECH_PATH: &str = "speechtotext/transcriptions:transcribe";

fn derive_cognitive_services_from_foundry(foundry_endpoint: &str) -> Option<String> {
    let url = url::Url::parse(foundry_endpoint).ok()?;
    let name = url.host_str()?.strip_suffix(".services.ai.azure.com")?;
    Some(format!("https://{name}.cognitiveservices.azure.com"))
}

fn azure_speech_url(endpoint: &str) -> String {
    format!(
        "{}/{AZURE_SPEECH_PATH}?api-version={AZURE_SPEECH_API_VERSION}",
        endpoint.trim_end_matches('/')
    )
}

pub(crate) fn azure_speech_endpoint(config: &Config) -> Result<String> {
    if let Ok(endpoint) = config.get_param::<String>("AZURE_SPEECH_ENDPOINT") {
        let endpoint = endpoint.trim();
        if !endpoint.is_empty() {
            return Ok(endpoint.to_string());
        }
    }

    let foundry_endpoint: String = config
        .get_param("AZURE_FOUNDRY_ENDPOINT")
        .map_err(|_| anyhow::anyhow!(
            "Configure AZURE_SPEECH_ENDPOINT or an AZURE_FOUNDRY_ENDPOINT whose host ends in .services.ai.azure.com"
        ))?;
    derive_cognitive_services_from_foundry(&foundry_endpoint).ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot derive Azure Speech endpoint from AZURE_FOUNDRY_ENDPOINT; configure AZURE_SPEECH_ENDPOINT"
        )
    })
}

fn azure_speech_key(config: &Config) -> Option<String> {
    config
        .get_secret("AZURE_SPEECH_KEY")
        .ok()
        .filter(|key: &String| !key.is_empty())
        .or_else(|| {
            config
                .get_secret("AZURE_FOUNDRY_API_KEY")
                .ok()
                .filter(|key: &String| !key.is_empty())
        })
}

async fn azure_auth_header(auth: &AzureAuth) -> Result<(String, String)> {
    let token = auth.get_token().await.map_err(anyhow::Error::from)?;
    Ok(match auth.credential_type() {
        crate::providers::azureauth::AzureCredentials::ApiKey(_) => {
            ("Ocp-Apim-Subscription-Key".to_string(), token.token_value)
        }
        crate::providers::azureauth::AzureCredentials::BearerToken(_)
        | crate::providers::azureauth::AzureCredentials::DefaultCredential => (
            "Authorization".to_string(),
            format!("Bearer {}", token.token_value),
        ),
    })
}

async fn transcribe_with_azure_foundry(
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
) -> Result<String> {
    let config = Config::global();
    let endpoint = azure_speech_endpoint(config)?;
    let ad_token = config
        .get_secret::<String>("AZURE_SPEECH_AD_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
        .or_else(|| {
            config
                .get_secret::<String>("AZURE_FOUNDRY_AD_TOKEN")
                .ok()
                .filter(|token| !token.is_empty())
        });
    let auth = AzureAuth::new(azure_speech_key(config), ad_token).map_err(anyhow::Error::from)?;
    let (auth_header_name, auth_header_value) = azure_auth_header(&auth).await?;
    let locale = config
        .get_param::<String>("AZURE_SPEECH_LOCALE")
        .ok()
        .filter(|locale| !locale.trim().is_empty());

    transcribe_speech_request(
        &azure_speech_url(&endpoint),
        &auth_header_name,
        &auth_header_value,
        audio_bytes,
        extension,
        mime_type,
        locale.as_deref(),
    )
    .await
}

async fn transcribe_speech_request(
    speech_url: &str,
    auth_header_name: &str,
    auth_header_value: &str,
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
    locale: Option<&str>,
) -> Result<String> {
    let audio_part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{extension}"))
        .mime_str(mime_type)?;
    let definition = match locale {
        Some(locale) => serde_json::json!({ "locales": [locale] }).to_string(),
        None => "{}".to_string(),
    };
    let form = reqwest::multipart::Form::new()
        .part("audio", audio_part)
        .text("definition", definition);
    let response = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?
        .post(speech_url)
        .header(auth_header_name, auth_header_value)
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if status == 401 || status == 403 {
            anyhow::bail!("Azure speech authentication failed ({status}): {body}");
        }
        if status == 429 {
            anyhow::bail!("Azure speech rate limit exceeded");
        }
        if body.contains("too short") {
            return Ok(String::new());
        }
        anyhow::bail!("Azure speech API error ({status}): {body}");
    }

    let data: serde_json::Value = response.json().await?;
    Ok(data["combinedPhrases"]
        .as_array()
        .and_then(|phrases| phrases.first())
        .and_then(|phrase| phrase["text"].as_str())
        .unwrap_or_default()
        .to_string())
}

fn build_api_client(provider: DictationProvider) -> Result<(ApiClient, String)> {
    let config = Config::global();
    let def = get_provider_def(provider);

    let api_key = config.get_secret(def.config_key).map_err(|e| {
        tracing::error!("{} not configured: {}", def.config_key, e);
        anyhow::anyhow!("{} not configured", def.config_key)
    })?;

    let (base_url, query_params, endpoint_path) = if provider == DictationProvider::OpenAI {
        let openai_base_url = config.get_param::<String>("OPENAI_BASE_URL").ok();

        if let Ok(host) = std::env::var("OPENAI_HOST") {
            (host, vec![], def.endpoint_path.to_string())
        } else if let Some(target) = resolve_openai_base_url_target(openai_base_url.as_deref())? {
            target
        } else if let Ok(host) = config.get_param::<String>("OPENAI_HOST") {
            (host, vec![], def.endpoint_path.to_string())
        } else {
            (
                def.default_base_url.to_string(),
                vec![],
                def.endpoint_path.to_string(),
            )
        }
    } else if let Some(host_key) = def.host_key {
        let base_url = config
            .get(host_key, false)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| def.default_base_url.to_string());
        (base_url, vec![], def.endpoint_path.to_string())
    } else {
        (
            def.default_base_url.to_string(),
            vec![],
            def.endpoint_path.to_string(),
        )
    };

    let auth = match provider {
        DictationProvider::OpenAI => AuthMethod::BearerToken(api_key),
        DictationProvider::Groq => AuthMethod::BearerToken(api_key),
        DictationProvider::ElevenLabs => AuthMethod::ApiKey {
            header_name: "xi-api-key".to_string(),
            key: api_key,
        },
        DictationProvider::AzureFoundry => {
            anyhow::bail!("Azure Foundry uses a dedicated transcription path")
        }
        #[cfg(feature = "local-inference")]
        DictationProvider::Local => anyhow::bail!("Local provider should not use API client"),
    };

    let tls = provider_tls_config_from_config(config)?;
    let mut client = ApiClient::with_timeout_and_tls(base_url, auth, REQUEST_TIMEOUT, tls)
        .map_err(|e| {
            tracing::error!("Failed to create API client: {}", e);
            e
        })?;
    if !query_params.is_empty() {
        client = client.with_query(query_params);
    }
    Ok((client, endpoint_path))
}

pub async fn transcribe_with_provider(
    provider: DictationProvider,
    model_param: String,
    model_value: String,
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
) -> Result<String> {
    if provider == DictationProvider::AzureFoundry {
        return transcribe_with_azure_foundry(audio_bytes, extension, mime_type).await;
    }

    let (client, endpoint_path) = build_api_client(provider)?;

    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", extension))
        .mime_str(mime_type)
        .map_err(|e| {
            tracing::error!("Failed to create multipart: {}", e);
            anyhow::anyhow!(e)
        })?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text(model_param, model_value);

    let response = client
        .request(&endpoint_path)
        .multipart_post(form)
        .await
        .map_err(|e| {
            tracing::error!("Request failed: {}", e);
            e
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        if status == 401 || error_text.contains("Invalid API key") {
            anyhow::bail!("Invalid API key");
        } else if status == 429 || error_text.contains("quota") {
            anyhow::bail!("Rate limit exceeded");
        } else if error_text.contains("too short") {
            return Ok(String::new());
        } else {
            anyhow::bail!("API error: {}", error_text);
        }
    }

    let data: serde_json::Value = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse response: {}", e);
        anyhow::anyhow!(e)
    })?;

    let text = data["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'text' field in response"))?
        .to_string();

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::{
        azure_auth_header, azure_speech_url, derive_cognitive_services_from_foundry,
        openai_dictation_target, resolve_openai_base_url_target, transcribe_speech_request,
        OPENAI_VERSIONLESS_TRANSCRIPTIONS_PATH,
    };
    use crate::providers::azureauth::AzureAuth;
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn openai_dictation_target_preserves_prefix_and_query_params() {
        let (host, query_params, endpoint_path) = openai_dictation_target(
            "https://user:pass@gateway.example.com/openai/v1?api-version=2024-02-01",
        )
        .unwrap();
        assert_eq!(host, "https://user:pass@gateway.example.com/openai");
        assert_eq!(
            query_params,
            vec![("api-version".to_string(), "2024-02-01".to_string())]
        );
        assert_eq!(endpoint_path, "v1/audio/transcriptions");
    }

    #[test]
    fn openai_dictation_target_uses_versionless_endpoint_without_v1() {
        let (host, query_params, endpoint_path) =
            openai_dictation_target("https://gateway.example.com/custom/api").unwrap();
        assert_eq!(host, "https://gateway.example.com/custom/api");
        assert!(query_params.is_empty());
        assert_eq!(endpoint_path, OPENAI_VERSIONLESS_TRANSCRIPTIONS_PATH);
    }

    #[test]
    fn openai_dictation_target_keeps_v1_endpoint_for_bare_host() {
        let (host, query_params, endpoint_path) =
            openai_dictation_target("https://api.openai.com").unwrap();
        assert_eq!(host, "https://api.openai.com");
        assert!(query_params.is_empty());
        assert_eq!(endpoint_path, "v1/audio/transcriptions");
    }

    #[test]
    fn resolve_openai_base_url_target_ignores_blank_values() {
        assert!(resolve_openai_base_url_target(Some("   "))
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn azure_api_key_uses_subscription_key_header() {
        let auth = AzureAuth::new(Some("key".to_string()), None).unwrap();
        assert_eq!(
            azure_auth_header(&auth).await.unwrap(),
            ("Ocp-Apim-Subscription-Key".to_string(), "key".to_string())
        );
    }

    #[tokio::test]
    async fn azure_entra_token_uses_bearer_header() {
        let auth = AzureAuth::new(None, Some("token".to_string())).unwrap();
        assert_eq!(
            azure_auth_header(&auth).await.unwrap(),
            ("Authorization".to_string(), "Bearer token".to_string())
        );
    }

    #[tokio::test]
    async fn parses_fast_transcription_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/speechtotext/transcriptions:transcribe"))
            .and(query_param("api-version", "2024-11-15"))
            .and(header("Ocp-Apim-Subscription-Key", "key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "combinedPhrases": [{"text": "Bonjour goose"}]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let text = transcribe_speech_request(
            &azure_speech_url(&server.uri()),
            "Ocp-Apim-Subscription-Key",
            "key",
            vec![0, 1, 2],
            "wav",
            "audio/wav",
            Some("fr-FR"),
        )
        .await
        .unwrap();
        assert_eq!(text, "Bonjour goose");
    }

    #[test]
    fn derives_cognitive_services_endpoint_from_project_url() {
        assert_eq!(
            derive_cognitive_services_from_foundry(
                "https://my-hub.services.ai.azure.com/api/projects/project"
            ),
            Some("https://my-hub.cognitiveservices.azure.com".to_string())
        );
    }

    #[test]
    fn does_not_derive_endpoint_from_maas_url() {
        assert_eq!(
            derive_cognitive_services_from_foundry("https://deployment.models.ai.azure.com/models"),
            None
        );
    }

    #[test]
    fn builds_fast_transcription_url() {
        assert_eq!(
            azure_speech_url("https://my-hub.cognitiveservices.azure.com/"),
            "https://my-hub.cognitiveservices.azure.com/speechtotext/transcriptions:transcribe?api-version=2024-11-15"
        );
    }
}
