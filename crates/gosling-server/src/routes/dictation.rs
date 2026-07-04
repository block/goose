use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use gosling::dictation::providers::{
    all_providers, is_configured, transcribe_with_provider, DictationProvider,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

const MAX_AUDIO_SIZE_BYTES: usize = 50 * 1024 * 1024;

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
            });
        }
    };

    Ok((audio_bytes, extension))
}

fn convert_error(e: anyhow::Error) -> ErrorResponse {
    let error_msg = e.to_string();

    if error_msg.contains("Invalid API key") {
        ErrorResponse {
            message: error_msg,
            status: StatusCode::UNAUTHORIZED,
        }
    } else if error_msg.contains("Rate limit exceeded") || error_msg.contains("quota") {
        ErrorResponse {
            message: error_msg,
            status: StatusCode::TOO_MANY_REQUESTS,
        }
    } else if error_msg.contains("not configured") {
        ErrorResponse {
            message: error_msg,
            status: StatusCode::PRECONDITION_FAILED,
        }
    } else if error_msg.contains("timeout") {
        ErrorResponse {
            message: error_msg,
            status: StatusCode::GATEWAY_TIMEOUT,
        }
    } else if error_msg.contains("API error") {
        ErrorResponse {
            message: error_msg,
            status: StatusCode::BAD_GATEWAY,
        }
    } else {
        ErrorResponse::internal(error_msg)
    }
}

#[utoipa::path(
    post,
    path = "/dictation/transcribe",
    request_body = TranscribeRequest,
    responses(
        (status = 200, description = "Audio transcribed successfully", body = TranscribeResponse),
        (status = 400, description = "Invalid request (bad base64 or unsupported format)"),
        (status = 401, description = "Invalid API key"),
        (status = 412, description = "Provider not configured"),
        (status = 413, description = "Audio file too large (max 50MB)"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error"),
        (status = 502, description = "Provider API error"),
        (status = 503, description = "Service unavailable"),
        (status = 504, description = "Request timeout")
    )
)]
pub async fn transcribe_dictation(
    Json(request): Json<TranscribeRequest>,
) -> Result<Json<TranscribeResponse>, ErrorResponse> {
    let (audio_bytes, extension) = validate_audio(&request.audio, &request.mime_type)?;

    let text = match request.provider {
        DictationProvider::OpenAI => transcribe_with_provider(
            DictationProvider::OpenAI,
            "model".to_string(),
            "whisper-1".to_string(),
            audio_bytes,
            extension,
            &request.mime_type,
        )
        .await
        .map_err(convert_error)?,
        DictationProvider::Groq => transcribe_with_provider(
            DictationProvider::Groq,
            "model".to_string(),
            "whisper-large-v3-turbo".to_string(),
            audio_bytes,
            extension,
            &request.mime_type,
        )
        .await
        .map_err(convert_error)?,
        DictationProvider::ElevenLabs => transcribe_with_provider(
            DictationProvider::ElevenLabs,
            "model_id".to_string(),
            "scribe_v1".to_string(),
            audio_bytes,
            extension,
            &request.mime_type,
        )
        .await
        .map_err(convert_error)?,
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
) -> Result<Json<HashMap<DictationProvider, DictationProviderStatus>>, ErrorResponse> {
    let config = gosling::config::Config::global();
    let mut providers = HashMap::new();

    for def in all_providers() {
        let provider = def.provider;
        let configured = is_configured(provider);

        let host = if let Some(host_key) = def.host_key {
            config
                .get(host_key, false)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
        } else {
            None
        };

        providers.insert(
            provider,
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
        .layer(DefaultBodyLimit::max(MAX_AUDIO_SIZE_BYTES))
        .with_state(state)
}
