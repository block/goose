use crate::config::Config;
use crate::providers::api_client::{ApiClient, AuthMethod};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscribeResponse {
    pub text: String,
}

// Provider definitions
pub struct DictationProviderDef {
    pub config_key: &'static str,
    pub default_url: &'static str,
    pub host_key: Option<&'static str>,
    pub description: &'static str,
    pub uses_provider_config: bool,
    pub settings_path: Option<&'static str>,
}

pub const   PROVIDERS: &[(&str, DictationProviderDef)] = &[
    (
        "openai",
        DictationProviderDef {
            config_key: "OPENAI_API_KEY",
            default_url: "https://api.openai.com/v1/audio/transcriptions",
            host_key: Some("OPENAI_HOST"),
            description: "Uses OpenAI Whisper API for high-quality transcription.",
            uses_provider_config: true,
            settings_path: Some("Settings > Models"),
        },
    ),
    (
        "elevenlabs",
        DictationProviderDef {
            config_key: "ELEVENLABS_API_KEY",
            default_url: "https://api.elevenlabs.io/v1/speech-to-text",
            host_key: None,
            description: "Uses ElevenLabs speech-to-text API for advanced voice processing.",
            uses_provider_config: false,
            settings_path: None,
        },
    ),
    (
        "local",
        DictationProviderDef {
            config_key: "LOCAL_WHISPER_MODEL",
            default_url: "",
            host_key: None,
            description: "Uses local Whisper model for offline transcription. No API key needed.",
            uses_provider_config: false,
            settings_path: None,
        },
    ),
];

pub fn get_provider_def(name: &str) -> Option<&'static DictationProviderDef> {
    PROVIDERS
        .iter()
        .find_map(|(n, def)| if *n == name { Some(def) } else { None })
}

fn build_api_client(provider_name: &str) -> Result<ApiClient> {
    let config = Config::global();
    let def = get_provider_def(provider_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", provider_name))?;

    let api_key = config
        .get_secret(def.config_key)
        .context(format!("{} not configured", def.config_key))?;

    let url = if let Some(host_key) = def.host_key {
        config
            .get(host_key, false)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .map(|custom_host| {
                let path = def
                    .default_url
                    .splitn(4, '/')
                    .nth(3)
                    .map(|p| format!("/{}", p))
                    .unwrap_or_default();
                format!("{}{}", custom_host.trim_end_matches('/'), path)
            })
            .unwrap_or_else(|| def.default_url.to_string())
    } else {
        def.default_url.to_string()
    };

    let auth = match provider_name {
        "openai" => AuthMethod::BearerToken(api_key),
        "elevenlabs" => AuthMethod::ApiKey {
            header_name: "xi-api-key".to_string(),
            key: api_key,
        },
        _ => anyhow::bail!("Unknown provider: {}", provider_name),
    };

    ApiClient::with_timeout(url, auth, REQUEST_TIMEOUT)
        .context("Failed to create API client")
}

pub async fn transcribe_openai(
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
) -> Result<String> {
    let client = build_api_client("openai")?;

    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", extension))
        .mime_str(mime_type)
        .context("Failed to create multipart")?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1");

    let response = client
        .request(None, "")
        .multipart_post(form)
        .await
        .context("Request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        if status == 401 || error_text.contains("Invalid API key") {
            anyhow::bail!("Invalid API key");
        } else if status == 429 || error_text.contains("quota") {
            anyhow::bail!("Rate limit exceeded");
        } else {
            anyhow::bail!("API error: {}", error_text);
        }
    }

    let data: TranscribeResponse = response
        .json()
        .await
        .context("Failed to parse response")?;

    Ok(data.text)
}

pub async fn transcribe_elevenlabs(
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
) -> Result<String> {
    let client = build_api_client("elevenlabs")?;

    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", extension))
        .mime_str(mime_type)
        .context("Failed to create multipart")?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model_id", "scribe_v1");

    let response = client
        .request(None, "")
        .multipart_post(form)
        .await
        .context("Request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        if status == 401 || error_text.contains("Invalid API key") {
            anyhow::bail!("Invalid API key");
        } else if status == 429 || error_text.contains("quota") {
            anyhow::bail!("Rate limit exceeded");
        } else {
            anyhow::bail!("API error: {}", error_text);
        }
    }

    let data: TranscribeResponse = response
        .json()
        .await
        .context("Failed to parse response")?;

    Ok(data.text)
}
