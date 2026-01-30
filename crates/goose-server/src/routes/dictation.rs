use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use utoipa::ToSchema;

const MAX_AUDIO_SIZE_BYTES: usize = 25 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// DictationProvider definitions
struct DictationProviderDef {
    config_key: &'static str,
    default_url: &'static str,
    host_key: Option<&'static str>,
    description: &'static str,
    uses_provider_config: bool,
    settings_path: Option<&'static str>,
}

const PROVIDERS: &[(&str, DictationProviderDef)] = &[
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
];

fn get_provider_def(name: &str) -> Option<&'static DictationProviderDef> {
    PROVIDERS
        .iter()
        .find_map(|(n, def)| if *n == name { Some(def) } else { None })
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DictationProvider {
    OpenAI,
    ElevenLabs,
}

impl DictationProvider {
    fn as_str(&self) -> &'static str {
        match self {
            DictationProvider::OpenAI => "openai",
            DictationProvider::ElevenLabs => "elevenlabs",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TranscribeRequest {
    /// Base64 encoded audio data
    pub audio: String,
    /// MIME type of the audio (e.g., "audio/webm", "audio/wav")
    pub mime_type: String,
    /// Transcription provider to use
    pub provider: DictationProvider,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TranscribeResponse {
    /// Transcribed text from the audio
    pub text: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DictationProviderStatus {
    /// Whether the provider is fully configured and ready to use
    pub configured: bool,
    /// Custom host URL if configured (only for providers that support it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Description of what this provider does
    pub description: String,
    /// Whether this provider uses the main provider config (true) or has its own key (false)
    pub uses_provider_config: bool,
    /// Path to settings if uses_provider_config is true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings_path: Option<String>,
    /// Config key name if uses_provider_config is false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_key: Option<String>,
}

fn validate_audio(audio: &str, mime_type: &str) -> Result<(Vec<u8>, &'static str), ErrorResponse> {
    let audio_bytes = BASE64
        .decode(audio)
        .map_err(|_| ErrorResponse::bad_request("Invalid base64 audio data"))?;

    if audio_bytes.len() > MAX_AUDIO_SIZE_BYTES {
        return Err(ErrorResponse {
            message: format!(
                "Audio file too large: {} bytes (max: {} bytes)",
                audio_bytes.len(),
                MAX_AUDIO_SIZE_BYTES
            ),
            status: StatusCode::PAYLOAD_TOO_LARGE,
        });
    }

    let extension = match mime_type {
        "audio/webm" | "audio/webm;codecs=opus" => "webm",
        "audio/mp4" => "mp4",
        "audio/mpeg" | "audio/mpga" => "mp3",
        "audio/m4a" => "m4a",
        "audio/wav" | "audio/x-wav" => "wav",
        _ => {
            return Err(ErrorResponse {
                message: format!("Unsupported audio format: {}", mime_type),
                status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
            })
        }
    };

    Ok((audio_bytes, extension))
}

fn get_provider_config(provider: &str) -> Result<(String, String), ErrorResponse> {
    let config = goose::config::Config::global();
    let def = get_provider_def(provider)
        .ok_or_else(|| ErrorResponse::bad_request(format!("Unknown provider: {}", provider)))?;

    let api_key = config
        .get_secret(def.config_key)
        .map_err(|_| ErrorResponse {
            message: format!("{} not configured", def.config_key),
            status: StatusCode::PRECONDITION_FAILED,
        })?;

    let url = if let Some(host_key) = def.host_key {
        // If host_key is configured, replace the host part of the default URL
        if let Some(custom_host) = config
            .get(host_key, false)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            // Extract the path from default_url (everything after the third slash)
            // e.g., "https://api.openai.com/v1/audio/transcriptions" -> "/v1/audio/transcriptions"
            let path = def
                .default_url
                .splitn(4, '/')
                .nth(3)
                .map(|p| format!("/{}", p))
                .unwrap_or_else(|| "".to_string());

            // Remove trailing slash from custom host if present
            let custom_host = custom_host.trim_end_matches('/');

            format!("{}{}", custom_host, path)
        } else {
            def.default_url.to_string()
        }
    } else {
        def.default_url.to_string()
    };

    Ok((api_key, url))
}

async fn transcribe_openai(
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
    api_key: &str,
    url: &str,
) -> Result<String, ErrorResponse> {
    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", extension))
        .mime_str(mime_type)
        .map_err(|e| ErrorResponse::internal(format!("Failed to create multipart: {}", e)))?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1");

    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| ErrorResponse::internal(format!("Failed to create client: {}", e)))?;

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                ErrorResponse {
                    message: "Request timed out".to_string(),
                    status: StatusCode::GATEWAY_TIMEOUT,
                }
            } else {
                ErrorResponse {
                    message: format!("Request failed: {}", e),
                    status: StatusCode::SERVICE_UNAVAILABLE,
                }
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        return Err(ErrorResponse {
            message: if status == 401 {
                "Invalid API key".to_string()
            } else if status == 429 {
                "Rate limit exceeded".to_string()
            } else {
                format!("API error: {}", error_text)
            },
            status: if status.is_client_error() {
                status
            } else {
                StatusCode::BAD_GATEWAY
            },
        });
    }

    let data: TranscribeResponse = response
        .json()
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to parse response: {}", e)))?;

    Ok(data.text)
}

async fn transcribe_elevenlabs(
    audio_bytes: Vec<u8>,
    extension: &str,
    mime_type: &str,
    api_key: &str,
    url: &str,
) -> Result<String, ErrorResponse> {
    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", extension))
        .mime_str(mime_type)
        .map_err(|_| ErrorResponse::internal("Failed to create multipart"))?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model_id", "scribe_v1");

    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| ErrorResponse::internal(format!("Failed to create client: {}", e)))?;

    let response = client
        .post(url)
        .header("xi-api-key", api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                ErrorResponse {
                    message: "Request timed out".to_string(),
                    status: StatusCode::GATEWAY_TIMEOUT,
                }
            } else {
                ErrorResponse {
                    message: format!("Request failed: {}", e),
                    status: StatusCode::SERVICE_UNAVAILABLE,
                }
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        return Err(ErrorResponse {
            message: if error_text.contains("Unauthorized")
                || error_text.contains("Invalid API key")
            {
                "Invalid API key".to_string()
            } else if error_text.contains("quota") || error_text.contains("limit") {
                "Rate limit exceeded".to_string()
            } else {
                format!("API error: {}", error_text)
            },
            status: if status.is_client_error() {
                status
            } else {
                StatusCode::BAD_GATEWAY
            },
        });
    }

    let data: TranscribeResponse = response
        .json()
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to parse response: {}", e)))?;

    Ok(data.text)
}

#[utoipa::path(
    post,
    path = "/dictation/transcribe",
    request_body = TranscribeRequest,
    responses(
        (status = 200, description = "Audio transcribed successfully", body = TranscribeResponse),
        (status = 400, description = "Invalid request (bad base64 or unsupported format)"),
        (status = 401, description = "Invalid API key"),
        (status = 412, description = "DictationProvider not configured"),
        (status = 413, description = "Audio file too large (max 25MB)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "DictationProvider API error"),
        (status = 503, description = "Service unavailable"),
        (status = 504, description = "Request timeout")
    )
)]
pub async fn transcribe_dictation(
    Json(request): Json<TranscribeRequest>,
) -> Result<Json<TranscribeResponse>, ErrorResponse> {
    let (audio_bytes, extension) = validate_audio(&request.audio, &request.mime_type)?;
    let provider_name = request.provider.as_str();
    let (api_key, url) = get_provider_config(provider_name)?;

    let text = match request.provider {
        DictationProvider::OpenAI => {
            transcribe_openai(audio_bytes, extension, &request.mime_type, &api_key, &url).await?
        }
        DictationProvider::ElevenLabs => {
            transcribe_elevenlabs(audio_bytes, extension, &request.mime_type, &api_key, &url)
                .await?
        }
    };

    Ok(Json(TranscribeResponse { text }))
}

#[utoipa::path(
    get,
    path = "/dictation/config",
    responses(
        (status = 200, description = "Audio transcription provider configurations", body = HashMap<String, DictationProviderStatus>)
    )
)]
pub async fn get_dictation_config(
) -> Result<Json<HashMap<String, DictationProviderStatus>>, ErrorResponse> {
    let config = goose::config::Config::global();
    let mut providers = HashMap::new();

    for (name, def) in PROVIDERS.iter() {
        let configured = config.get_secret::<String>(def.config_key).is_ok();

        let host = if let Some(host_key) = def.host_key {
            config
                .get(host_key, false)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
        } else {
            None
        };

        providers.insert(
            name.to_string(),
            DictationProviderStatus {
                configured,
                host,
                description: def.description.to_string(),
                uses_provider_config: def.uses_provider_config,
                settings_path: def.settings_path.map(|s| s.to_string()),
                config_key: if !def.uses_provider_config {
                    Some(def.config_key.to_string())
                } else {
                    None
                },
            },
        );
    }

    Ok(Json(providers))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/dictation/transcribe", post(transcribe_dictation))
        .route("/dictation/config", get(get_dictation_config))
        .with_state(state)
}
