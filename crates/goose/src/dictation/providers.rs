use crate::config::Config;
#[cfg(feature = "local-inference")]
use crate::dictation::whisper::LOCAL_WHISPER_MODEL_CONFIG_KEY;
use crate::providers::api_client::{ApiClient, AuthMethod};
use crate::providers::openai::parse_openai_base_url;
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(feature = "local-inference")]
use std::sync::Mutex;
use std::time::Duration;
use utoipa::ToSchema;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const OPENAI_VERSIONLESS_TRANSCRIPTIONS_PATH: &str = "audio/transcriptions";
const GLADIA_UPLOAD_PATH: &str = "v2/upload";
const GLADIA_TRANSCRIPTION_PATH: &str = "v2/pre-recorded";
const GLADIA_POLL_INTERVAL: Duration = Duration::from_millis(1000);
const GLADIA_MAX_POLLS: u32 = 120;
type OpenAiDictationTarget = (String, Vec<(String, String)>, String);

#[cfg(feature = "local-inference")]
static LOCAL_TRANSCRIBER: once_cell::sync::Lazy<
    Mutex<Option<(String, super::whisper::WhisperTranscriber)>>,
> = once_cell::sync::Lazy::new(|| Mutex::new(None));

#[cfg(feature = "local-inference")]
const WHISPER_TOKENIZER_JSON: &str = include_str!("whisper_data/tokens.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DictationProvider {
    OpenAI,
    ElevenLabs,
    Groq,
    Gladia,
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
        provider: DictationProvider::Gladia,
        config_key: "GLADIA_API_KEY",
        default_base_url: "https://api.gladia.io",
        endpoint_path: GLADIA_UPLOAD_PATH,
        host_key: None,
        description: "Uses Gladia's speech-to-text API with an upload-and-poll workflow.",
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
        DictationProvider::Gladia => AuthMethod::ApiKey {
            header_name: "x-gladia-key".to_string(),
            key: api_key,
        },
        #[cfg(feature = "local-inference")]
        DictationProvider::Local => anyhow::bail!("Local provider should not use API client"),
    };

    let mut client = ApiClient::with_timeout(base_url, auth, REQUEST_TIMEOUT).map_err(|e| {
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
        .request(None, &endpoint_path)
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

async fn gladia_response_json(response: reqwest::Response) -> Result<serde_json::Value> {
    if !response.status().is_success() {
        let status = response.status();
        if status == 401 {
            anyhow::bail!("Invalid API key");
        } else if status == 429 {
            anyhow::bail!("Rate limit exceeded");
        } else {
            anyhow::bail!("API error: status {}", status);
        }
    }

    response.json().await.map_err(|e| {
        tracing::error!("Failed to parse Gladia response: {}", e);
        anyhow::anyhow!(e)
    })
}

fn parse_gladia_transcript(result: &serde_json::Value) -> Result<String> {
    result["result"]["transcription"]["full_transcript"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Missing transcript in Gladia response"))
}

enum GladiaPollOutcome {
    Done(String),
    Pending,
}

fn interpret_gladia_poll(result: &serde_json::Value) -> Result<GladiaPollOutcome> {
    match result["status"].as_str() {
        Some("done") => Ok(GladiaPollOutcome::Done(parse_gladia_transcript(result)?)),
        Some("error") => anyhow::bail!("Gladia transcription failed"),
        _ => Ok(GladiaPollOutcome::Pending),
    }
}

fn gladia_pre_recorded_payload(audio_url: &str, model: &str) -> serde_json::Value {
    serde_json::json!({ "audio_url": audio_url, "model": model })
}

pub async fn transcribe_gladia(
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
    model: &str,
) -> Result<String> {
    let (client, upload_path) = build_api_client(DictationProvider::Gladia)?;

    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", extension))
        .mime_str(mime_type)
        .map_err(|e| {
            tracing::error!("Failed to create multipart: {}", e);
            anyhow::anyhow!(e)
        })?;
    let form = reqwest::multipart::Form::new().part("audio", part);

    let upload = client
        .request(None, &upload_path)
        .multipart_post(form)
        .await?;
    let upload = gladia_response_json(upload).await?;
    let audio_url = upload["audio_url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'audio_url' in Gladia upload response"))?;

    let payload = gladia_pre_recorded_payload(audio_url, model);
    let job = client
        .request(None, GLADIA_TRANSCRIPTION_PATH)
        .response_post(&payload)
        .await?;
    let job = gladia_response_json(job).await?;
    let job_id = job["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'id' in Gladia transcription response"))?;

    let poll_path = format!("{}/{}", GLADIA_TRANSCRIPTION_PATH, job_id);
    for _ in 0..GLADIA_MAX_POLLS {
        let response = client.request(None, &poll_path).response_get().await?;
        let result = gladia_response_json(response).await?;
        match interpret_gladia_poll(&result)? {
            GladiaPollOutcome::Done(text) => return Ok(text),
            GladiaPollOutcome::Pending => tokio::time::sleep(GLADIA_POLL_INTERVAL).await,
        }
    }

    anyhow::bail!("Gladia transcription timed out")
}

/// Single entry point for dictation transcription. Hides the per-provider
/// differences (Gladia's async upload-and-poll flow, local Whisper inference)
/// behind one call so dispatch sites stay uniform. `model_param`/`model_value`
/// are the multipart field name and model id used by the shared remote path;
/// providers that ignore them (Gladia, Local) simply don't read them.
pub async fn transcribe(
    provider: DictationProvider,
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
    model_param: &str,
    model_value: &str,
) -> Result<String> {
    match provider {
        #[cfg(feature = "local-inference")]
        DictationProvider::Local => transcribe_local(audio_bytes).await,
        DictationProvider::Gladia => {
            transcribe_gladia(audio_bytes, extension, mime_type, model_value).await
        }
        _ => {
            transcribe_with_provider(
                provider,
                model_param.to_string(),
                model_value.to_string(),
                audio_bytes,
                extension,
                mime_type,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        all_providers, get_provider_def, gladia_pre_recorded_payload, interpret_gladia_poll,
        openai_dictation_target, parse_gladia_transcript, resolve_openai_base_url_target,
        DictationProvider, GladiaPollOutcome, GLADIA_UPLOAD_PATH,
        OPENAI_VERSIONLESS_TRANSCRIPTIONS_PATH,
    };

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

    #[test]
    fn parse_gladia_transcript_reads_nested_full_transcript() {
        let result = serde_json::json!({
            "status": "done",
            "result": { "transcription": { "full_transcript": "hello world" } }
        });
        assert_eq!(parse_gladia_transcript(&result).unwrap(), "hello world");
    }

    #[test]
    fn parse_gladia_transcript_errors_when_transcript_missing() {
        let result = serde_json::json!({ "status": "done", "result": {} });
        assert!(parse_gladia_transcript(&result).is_err());
    }

    #[test]
    fn interpret_gladia_poll_returns_transcript_when_done() {
        let result = serde_json::json!({
            "status": "done",
            "result": { "transcription": { "full_transcript": "hi there" } }
        });
        match interpret_gladia_poll(&result).unwrap() {
            GladiaPollOutcome::Done(text) => assert_eq!(text, "hi there"),
            GladiaPollOutcome::Pending => panic!("expected Done"),
        }
    }

    #[test]
    fn interpret_gladia_poll_is_pending_while_processing() {
        for status in ["queued", "processing"] {
            let result = serde_json::json!({ "status": status });
            assert!(matches!(
                interpret_gladia_poll(&result).unwrap(),
                GladiaPollOutcome::Pending
            ));
        }
    }

    #[test]
    fn interpret_gladia_poll_errors_on_error_status() {
        let result = serde_json::json!({ "status": "error" });
        assert!(interpret_gladia_poll(&result).is_err());
    }

    #[test]
    fn interpret_gladia_poll_errors_when_done_without_transcript() {
        let result = serde_json::json!({ "status": "done", "result": {} });
        assert!(interpret_gladia_poll(&result).is_err());
    }

    #[test]
    fn gladia_pre_recorded_payload_includes_audio_url_and_model() {
        let payload = gladia_pre_recorded_payload("https://api.gladia.io/x.wav", "solaria-1");
        assert_eq!(payload["audio_url"], "https://api.gladia.io/x.wav");
        assert_eq!(payload["model"], "solaria-1");
    }

    #[test]
    fn gladia_provider_def_uses_dedicated_key_and_upload_path() {
        let def = get_provider_def(DictationProvider::Gladia);
        assert_eq!(def.config_key, "GLADIA_API_KEY");
        assert_eq!(def.default_base_url, "https://api.gladia.io");
        assert_eq!(def.endpoint_path, GLADIA_UPLOAD_PATH);
        assert!(!def.uses_provider_config);
        assert!(def.host_key.is_none());
    }

    #[test]
    fn all_providers_includes_gladia() {
        assert!(all_providers()
            .iter()
            .any(|def| def.provider == DictationProvider::Gladia));
    }
}
