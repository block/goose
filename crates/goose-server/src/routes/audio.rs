/// Audio transcription route handler
///
/// This module provides endpoints for audio transcription using OpenAI's Whisper API.
/// The OpenAI API key must be configured in the backend for this to work.
use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
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

#[derive(Debug, Serialize)]
struct TranscribeResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
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
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<TranscribeRequest>,
) -> Result<Json<TranscribeResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Get the OpenAI API key from config
    let config = goose::config::Config::global();
    let api_key: String = config
        .get_secret("OPENAI_API_KEY")
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    // Decode the base64 audio data
    let audio_bytes = BASE64
        .decode(&request.audio)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

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
    let file_extension = match request.mime_type.as_str() {
        "audio/webm" => "webm",
        "audio/mp4" => "mp4",
        "audio/mpeg" => "mp3",
        "audio/mpga" => "mpga",
        "audio/m4a" => "m4a",
        "audio/wav" => "wav",
        "audio/x-wav" => "wav",
        _ => return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE),
    };

    // Create a multipart form with the audio file
    let part = reqwest::multipart::Part::bytes(audio_bytes)
        .file_name(format!("audio.{}", file_extension))
        .mime_str(&request.mime_type)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1")
        .text("response_format", "json");

    // Make request to OpenAI Whisper API
    let client = Client::builder()
        .timeout(Duration::from_secs(OPENAI_TIMEOUT_SECONDS))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to create HTTP client: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
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

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!("OpenAI API error: {}", error_text);
        return Err(StatusCode::BAD_GATEWAY);
    }

    let whisper_response: WhisperResponse = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse OpenAI response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(TranscribeResponse {
        text: whisper_response.text,
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/audio/transcribe", post(transcribe_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_transcribe_endpoint_requires_auth() {
        let state = AppState::new(
            Arc::new(goose::agents::Agent::new()),
            "test-secret".to_string(),
        )
        .await;
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
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_transcribe_endpoint_validates_size() {
        let state = AppState::new(
            Arc::new(goose::agents::Agent::new()),
            "test-secret".to_string(),
        )
        .await;
        let app = routes(state);

        // Create a large base64 string (simulating > 25MB audio)
        let large_audio = BASE64.encode(vec![0u8; MAX_AUDIO_SIZE_BYTES + 1]);

        let request = Request::builder()
            .uri("/audio/transcribe")
            .method("POST")
            .header("content-type", "application/json")
            .header("x-secret-key", "test-secret")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "audio": large_audio,
                    "mime_type": "audio/webm"
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_transcribe_endpoint_validates_mime_type() {
        let state = AppState::new(
            Arc::new(goose::agents::Agent::new()),
            "test-secret".to_string(),
        )
        .await;
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
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn test_transcribe_endpoint_handles_invalid_base64() {
        let state = AppState::new(
            Arc::new(goose::agents::Agent::new()),
            "test-secret".to_string(),
        )
        .await;
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
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
