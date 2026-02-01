use crate::config::paths::Paths;
use crate::config::Config;
use super::whisper::WhisperTranscriber;
use anyhow::{Context, Result};
use std::sync::Mutex;

// Global lazy-initialized transcriber to reuse the loaded model
// Stores (model_path, transcriber) to detect when model changes
static LOCAL_TRANSCRIBER: once_cell::sync::Lazy<Mutex<Option<(String, WhisperTranscriber)>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

// Bundled tokenizer JSON (2.4MB)
const WHISPER_TOKENIZER_JSON: &str = include_str!("whisper_data/tokens.json");

pub async fn transcribe_local(audio_bytes: Vec<u8>) -> Result<String> {
    // Run transcription in a blocking task to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || {
        // Get model ID from config or use default
        let config = Config::global();
        let model_id = config
            .get("LOCAL_WHISPER_MODEL", false)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "tiny".to_string());

        // Convert model ID to full path
        let model_path = if model_id == "tiny" {
            Paths::in_data_dir("models").join("model-tiny-q80.gguf")
        } else {
            Paths::in_data_dir("models").join(format!("whisper-{}-q8_0.gguf", model_id))
        };

        // Get or initialize the transcriber
        let mut transcriber_lock = LOCAL_TRANSCRIBER
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock transcriber: {}", e))?;

        // Check if we need to load/reload the transcriber
        let model_path_str = model_path.to_string_lossy().to_string();
        let needs_reload = match transcriber_lock.as_ref() {
            None => true,
            Some((cached_path, _)) => cached_path != &model_path_str,
        };

        if needs_reload {
            tracing::info!("Loading Whisper model from: {}", model_path.display());
            tracing::debug!("Model path details: exists={}, is_absolute={}",
                model_path.exists(),
                model_path.is_absolute()
            );

            let transcriber = WhisperTranscriber::new_with_tokenizer(
                &model_path,
                Some(WHISPER_TOKENIZER_JSON),
            )?;

            *transcriber_lock = Some((model_path_str, transcriber));
        }

        // Transcribe the audio
        let (_, transcriber) = transcriber_lock.as_mut().unwrap();
        let text = transcriber
            .transcribe(&audio_bytes)
            .context("Transcription failed")?;

        Ok(text)
    })
    .await
    .context("Transcription task failed")?
}

pub fn is_local_configured() -> bool {
    let config = Config::global();
    // Get model ID from config or use default
    let model_id = config
        .get("LOCAL_WHISPER_MODEL", false)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "tiny".to_string());

    // Convert model ID to full path
    let filename = if model_id == "tiny" {
        "model-tiny-q80.gguf".to_string()
    } else {
        format!("whisper-{}-q8_0.gguf", model_id)
    };
    let model_path = Paths::in_data_dir("models").join(filename);

    let exists = model_path.exists();
    tracing::debug!(
        "is_local_configured: model_id={}, path={:?}, exists={}",
        model_id,
        model_path,
        exists
    );

    exists
}
