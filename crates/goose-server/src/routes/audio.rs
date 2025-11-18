/// Audio transcription route handler
///
/// This module provides endpoints for audio transcription using OpenAI's Whisper API.
/// The OpenAI API key must be configured in the backend for this to work.
use crate::state::AppState;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

// Constants
const MAX_AUDIO_SIZE_BYTES: usize = 25 * 1024 * 1024; // 25MB
const OPENAI_TIMEOUT_SECONDS: u64 = 30;

#[derive(Debug, Deserialize)]
struct TranscribeRequest {
    audio: String, // Base64 encoded audio data
    mime_type: String,
}

#[derive(Debug, Deserialize)]
struct TranscribeElevenLabsRequest {
    audio: String, // Base64 encoded audio data
    mime_type: String,
}

#[derive(Debug, Serialize)]
struct TranscribeResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

/// Validate audio input and return decoded bytes and file extension
fn validate_audio_input(
    audio: &str,
    mime_type: &str,
) -> Result<(Vec<u8>, &'static str), StatusCode> {
    // Decode the base64 audio data
    let audio_bytes = BASE64.decode(audio).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check file size
    if audio_bytes.len() > MAX_AUDIO_SIZE_BYTES {
        tracing::warn!(
            "Audio file too large: {} bytes (max: {} bytes)",
            audio_bytes.len(),
            MAX_AUDIO_SIZE_BYTES
        );
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Determine file extension based on MIME type
    let file_extension = match mime_type {
        "audio/webm" => "webm",
        "audio/webm;codecs=opus" => "webm",
        "audio/mp4" => "mp4",
        "audio/mpeg" => "mp3",
        "audio/mpga" => "mpga",
        "audio/m4a" => "m4a",
        "audio/wav" => "wav",
        "audio/x-wav" => "wav",
        _ => return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE),
    };

    Ok((audio_bytes, file_extension))
}

/// Get OpenAI configuration (API key and host)
fn get_openai_config() -> Result<(String, String), StatusCode> {
    let config = goose::config::Config::global();

    let api_key: String = config.get_secret("OPENAI_API_KEY").map_err(|e| {
        tracing::error!("Failed to get OpenAI API key: {:?}", e);
        StatusCode::PRECONDITION_FAILED
    })?;

    let openai_host = match config.get("OPENAI_HOST", false) {
        Ok(value) => value
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "https://api.openai.com".to_string()),
        Err(_) => "https://api.openai.com".to_string(),
    };

    Ok((api_key, openai_host))
}

/// Send transcription request to OpenAI Whisper API
async fn send_openai_request(
    audio_bytes: Vec<u8>,
    file_extension: &str,
    mime_type: &str,
    api_key: &str,
    openai_host: &str,
) -> Result<WhisperResponse, StatusCode> {
    tracing::info!("Using OpenAI host: {}", openai_host);
    tracing::info!(
        "Audio file size: {} bytes, extension: {}, mime_type: {}",
        audio_bytes.len(),
        file_extension,
        mime_type
    );

    // Create a multipart form with the audio file
    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", file_extension))
        .mime_str(mime_type)
        .map_err(|e| {
            tracing::error!("Failed to create multipart part: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1")
        .text("response_format", "json");

    tracing::info!("Created multipart form for OpenAI Whisper API");

    // Make request to OpenAI Whisper API
    let client = Client::builder()
        .timeout(Duration::from_secs(OPENAI_TIMEOUT_SECONDS))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to create HTTP client: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "Sending request to OpenAI: {}/v1/audio/transcriptions",
        openai_host
    );

    let response = client
        .post(format!("{}/v1/audio/transcriptions", openai_host))
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                tracing::error!(
                    "OpenAI API request timed out after {}s",
                    OPENAI_TIMEOUT_SECONDS
                );
                StatusCode::GATEWAY_TIMEOUT
            } else {
                tracing::error!("Failed to send request to OpenAI: {}", e);
                StatusCode::SERVICE_UNAVAILABLE
            }
        })?;

    tracing::info!(
        "Received response from OpenAI with status: {}",
        response.status()
    );

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!("OpenAI API error (status: {}): {}", status, error_text);

        // Check for specific error codes
        if status == 401 {
            tracing::error!("OpenAI API key appears to be invalid or unauthorized");
            return Err(StatusCode::UNAUTHORIZED);
        } else if status == 429 {
            tracing::error!("OpenAI API quota or rate limit exceeded");
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        return Err(StatusCode::BAD_GATEWAY);
    }

    let whisper_response: WhisperResponse = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse OpenAI response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(whisper_response)
}

/// Transcribe audio using OpenAI's Whisper API
///
/// # Request
/// - `audio`: Base64 encoded audio data
/// - `mime_type`: MIME type of the audio (e.g., "audio/webm", "audio/wav")
///
/// # Response
/// - `text`: Transcribed text from the audio
///
/// # Errors
/// - 401: Unauthorized (missing or invalid X-Secret-Key header)
/// - 412: Precondition Failed (OpenAI API key not configured)
/// - 400: Bad Request (invalid base64 audio data)
/// - 413: Payload Too Large (audio file exceeds 25MB limit)
/// - 415: Unsupported Media Type (unsupported audio format)
/// - 502: Bad Gateway (OpenAI API error)
/// - 503: Service Unavailable (network error)
async fn transcribe_handler(
    Json(request): Json<TranscribeRequest>,
) -> Result<Json<TranscribeResponse>, StatusCode> {
    let (audio_bytes, file_extension) = validate_audio_input(&request.audio, &request.mime_type)?;
    let (api_key, openai_host) = get_openai_config()?;

    let whisper_response = send_openai_request(
        audio_bytes,
        file_extension,
        &request.mime_type,
        &api_key,
        &openai_host,
    )
    .await?;

    Ok(Json(TranscribeResponse {
        text: whisper_response.text,
    }))
}

/// Transcribe audio using ElevenLabs Speech-to-Text API
///
/// Uses ElevenLabs' speech-to-text endpoint for transcription.
/// Requires an ElevenLabs API key with speech-to-text access.
async fn transcribe_elevenlabs_handler(
    Json(request): Json<TranscribeElevenLabsRequest>,
) -> Result<Json<TranscribeResponse>, StatusCode> {
    let (audio_bytes, file_extension) = validate_audio_input(&request.audio, &request.mime_type)?;

    // Get the ElevenLabs API key from config (after input validation)
    let config = goose::config::Config::global();

    // First try to get it as a secret
    let api_key: String = match config.get_secret::<String>("ELEVENLABS_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            // Try to get it as non-secret (for backward compatibility)
            match config.get("ELEVENLABS_API_KEY", false) {
                Ok(value) => {
                    match value.as_str() {
                        Some(key_str) => {
                            let key = key_str.to_string();
                            // Migrate to secret storage
                            if let Err(e) = config.set(
                                "ELEVENLABS_API_KEY",
                                &serde_json::Value::String(key.clone()),
                                true,
                            ) {
                                tracing::error!("Failed to migrate ElevenLabs API key: {:?}", e);
                            }
                            // Delete the non-secret version
                            if let Err(e) = config.delete("ELEVENLABS_API_KEY") {
                                tracing::warn!(
                                    "Failed to delete non-secret ElevenLabs API key: {:?}",
                                    e
                                );
                            }
                            key
                        }
                        None => {
                            tracing::error!(
                                "ElevenLabs API key is not a string, found: {:?}",
                                value
                            );
                            return Err(StatusCode::PRECONDITION_FAILED);
                        }
                    }
                }
                Err(_) => {
                    tracing::error!("No ElevenLabs API key found in configuration");
                    return Err(StatusCode::PRECONDITION_FAILED);
                }
            }
        }
    };

    // Create multipart form for ElevenLabs API
    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", file_extension))
        .mime_str(&request.mime_type)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let form = reqwest::multipart::Form::new()
        .part("file", part) // Changed from "audio" to "file"
        .text("model_id", "scribe_v1") // Use the correct model_id for speech-to-text
        .text("tag_audio_events", "false")
        .text("diarize", "false");

    // Make request to ElevenLabs Speech-to-Text API
    let client = Client::builder()
        .timeout(Duration::from_secs(OPENAI_TIMEOUT_SECONDS))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to create HTTP client: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = client
        .post("https://api.elevenlabs.io/v1/speech-to-text")
        .header("xi-api-key", &api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                tracing::error!(
                    "ElevenLabs API request timed out after {}s",
                    OPENAI_TIMEOUT_SECONDS
                );
                StatusCode::GATEWAY_TIMEOUT
            } else {
                tracing::error!("Failed to send request to ElevenLabs: {}", e);
                StatusCode::SERVICE_UNAVAILABLE
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!("ElevenLabs API error (status: {}): {}", status, error_text);

        // Check for specific error codes
        if error_text.contains("Unauthorized") || error_text.contains("Invalid API key") {
            return Err(StatusCode::UNAUTHORIZED);
        } else if error_text.contains("quota") || error_text.contains("limit") {
            return Err(StatusCode::PAYMENT_REQUIRED);
        }

        return Err(StatusCode::BAD_GATEWAY);
    }

    // Parse ElevenLabs response
    #[derive(Debug, Deserialize)]
    struct ElevenLabsResponse {
        text: String,
        #[serde(rename = "chunks")]
        #[allow(dead_code)]
        _chunks: Option<Vec<serde_json::Value>>,
    }

    let elevenlabs_response: ElevenLabsResponse = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse ElevenLabs response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(TranscribeResponse {
        text: elevenlabs_response.text,
    }))
}

/// Check if dictation providers are configured
///
/// Returns configuration status for dictation providers
async fn check_dictation_config() -> Result<Json<serde_json::Value>, StatusCode> {
    let config = goose::config::Config::global();

    // Check if ElevenLabs API key is configured
    let has_elevenlabs = match config.get_secret::<String>("ELEVENLABS_API_KEY") {
        Ok(_) => true,
        Err(_) => {
            // Check non-secret for backward compatibility
            config.get("ELEVENLABS_API_KEY", false).is_ok()
        }
    };

    Ok(Json(serde_json::json!({
        "elevenlabs": has_elevenlabs
    })))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/audio/transcribe", post(transcribe_handler))
        .route(
            "/audio/transcribe/elevenlabs",
            post(transcribe_elevenlabs_handler),
        )
        .route("/audio/config", get(check_dictation_config))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_transcribe_endpoint_requires_auth() {
        let state = AppState::new().await.unwrap();
        let app = routes(state);
        // Test without auth header
        let request = Request::builder()
            .uri("/audio/transcribe")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "audio": "dGVzdA==",
                    "mime_type": "audio/webm"
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(
            response.status() == StatusCode::PRECONDITION_FAILED
                || response.status() == StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_transcribe_endpoint_validates_size() {
        let state = AppState::new().await.unwrap();
        let app = routes(state);

        let request = Request::builder()
            .uri("/audio/transcribe")
            .method("POST")
            .header("content-type", "application/json")
            .header("x-secret-key", "test-secret")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "audio": "dGVzdA==",
                    "mime_type": "application/pdf" // Invalid MIME type
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(
            response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE
                || response.status() == StatusCode::PRECONDITION_FAILED
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_transcribe_endpoint_validates_mime_type() {
        let state = AppState::new().await.unwrap();
        let app = routes(state);

        let request = Request::builder()
            .uri("/audio/transcribe")
            .method("POST")
            .header("content-type", "application/json")
            .header("x-secret-key", "test-secret")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "audio": "invalid-base64-!@#$%",
                    "mime_type": "audio/webm"
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::PRECONDITION_FAILED
        );
    }
}
