use axum::{extract::Multipart, routing::post, Json, Router};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;
use tempfile::Builder;
use tokio::fs;
use tower_http::cors::{Any, CorsLayer};

static INIT: Once = Once::new();

/// Ensures whisper is built and the model is downloaded
fn ensure_whisper() {
    INIT.call_once(|| {
        // Get the project root directory
        let mut project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        project_dir.pop(); // go up from goose-server
        project_dir.pop(); // go up from crates

        // Check if whisper executable exists
        let whisper_path = project_dir.join("whisper.cpp/build/bin/main");
        if !whisper_path.exists() {
            println!("Building whisper...");
            let whisper_dir = project_dir.join("whisper.cpp");

            // Build whisper
            let status = Command::new("make")
                .current_dir(&whisper_dir)
                .status()
                .expect("Failed to build whisper");

            if !status.success() {
                panic!("Failed to build whisper");
            }
        }

        // Check if model exists
        let model_path = project_dir.join("whisper.cpp/models/ggml-base.en.bin");
        if !model_path.exists() {
            println!("Downloading whisper model...");
            let whisper_dir = project_dir.join("whisper.cpp");

            // Download model
            let status = Command::new("bash")
                .current_dir(&whisper_dir)
                .arg("./models/download-ggml-model.sh")
                .arg("base.en")
                .status()
                .expect("Failed to download whisper model");

            if !status.success() {
                panic!("Failed to download whisper model");
            }
        }

        println!("Whisper is ready!");
    });
}

pub fn routes() -> Router {
    // Ensure whisper is installed when creating routes
    ensure_whisper();

    Router::new().route("/transcribe", post(transcribe)).layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
}

async fn transcribe(mut multipart: Multipart) -> Json<serde_json::Value> {
    eprintln!("Starting transcription process...");

    while let Some(field) = multipart.next_field().await.unwrap() {
        eprintln!("Processing multipart field: {:?}", field.name());

        if let Ok(data) = field.bytes().await {
            eprintln!("Received audio data of size: {} bytes", data.len());
            if data.len() == 0 {
                eprintln!("Error: Received empty audio data");
                return Json(json!({
                    "success": false,
                    "error": "Received empty audio data"
                }));
            }

            // Create temporary files with proper extensions
            let webm_file = match Builder::new().suffix(".webm").tempfile() {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Error creating WebM tempfile: {}", e);
                    return Json(json!({
                        "success": false,
                        "error": format!("Failed to create temporary WebM file: {}", e)
                    }));
                }
            };
            let webm_path = webm_file.path().to_str().unwrap().to_string();

            let wav_file = match Builder::new().suffix(".wav").tempfile() {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Error creating WAV tempfile: {}", e);
                    return Json(json!({
                        "success": false,
                        "error": format!("Failed to create temporary WAV file: {}", e)
                    }));
                }
            };
            let wav_path = wav_file.path().to_str().unwrap().to_string();

            // Write the WebM data
            match webm_file.as_file().write_all(&data) {
                Ok(_) => eprintln!("Successfully wrote WebM data to temporary file"),
                Err(e) => {
                    eprintln!("Error writing WebM data: {}", e);
                    return Json(json!({
                        "success": false,
                        "error": format!("Failed to write WebM data: {}", e)
                    }));
                }
            }

            // Get the path to the whisper executable in the project directory
            let mut whisper_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            whisper_path.pop(); // go up from goose-server
            whisper_path.pop(); // go up from crates
            whisper_path.push("whisper.cpp");
            whisper_path.push("build");
            whisper_path.push("bin");
            whisper_path.push("main");

            // Get the path to the model file
            let mut model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            model_path.pop(); // go up from goose-server
            model_path.pop(); // go up from crates
            model_path.push("whisper.cpp");
            model_path.push("models");
            model_path.push("ggml-base.en.bin");

            eprintln!("Paths configuration:");
            eprintln!("Whisper path: {:?}", whisper_path);
            eprintln!("Model path: {:?}", model_path);
            eprintln!("WebM path: {:?}", webm_path);
            eprintln!("WAV path: {:?}", wav_path);

            // Verify whisper executable exists
            if !whisper_path.exists() {
                eprintln!("Error: Whisper executable not found at {:?}", whisper_path);
                return Json(json!({
                    "success": false,
                    "error": "Whisper executable not found"
                }));
            }

            // Verify model exists
            if !model_path.exists() {
                eprintln!("Error: Whisper model not found at {:?}", model_path);
                return Json(json!({
                    "success": false,
                    "error": "Whisper model not found"
                }));
            }

            // Check WebM file size
            let webm_size = match std::fs::metadata(&webm_path) {
                Ok(metadata) => metadata.len(),
                Err(e) => {
                    eprintln!("Error getting WebM file metadata: {}", e);
                    return Json(json!({
                        "success": false,
                        "error": format!("Failed to verify WebM file: {}", e)
                    }));
                }
            };
            eprintln!("WebM file size: {} bytes", webm_size);

            // Check WebM file content
            eprintln!("Analyzing WebM file with FFprobe...");
            let ffprobe_webm = Command::new("ffprobe")
                .arg("-v")
                .arg("error") // Only show errors
                .arg("-show_format")
                .arg("-show_streams")
                .arg(&webm_path)
                .output()
                .unwrap();

            let webm_probe_output = String::from_utf8_lossy(&ffprobe_webm.stdout);
            eprintln!("WebM FFprobe analysis:");
            eprintln!("{}", webm_probe_output);

            if !ffprobe_webm.status.success() {
                eprintln!(
                    "WebM FFprobe error: {}",
                    String::from_utf8_lossy(&ffprobe_webm.stderr)
                );
                return Json(json!({
                    "success": false,
                    "error": format!("Invalid WebM file: {}", String::from_utf8_lossy(&ffprobe_webm.stderr))
                }));
            }

            // Run ffmpeg to convert WebM to WAV
            eprintln!("Converting WebM to WAV...");
            let ffmpeg_output = Command::new("ffmpeg")
                .arg("-hide_banner")
                .arg("-loglevel")
                .arg("debug") // Increased logging level
                .arg("-i")
                .arg(&webm_path)
                .arg("-vn") // Ignore video stream if present
                .arg("-acodec")
                .arg("pcm_s16le") // Force audio codec
                .arg("-ar")
                .arg("16000") // Sample rate that whisper expects
                .arg("-ac")
                .arg("1") // Mono audio
                .arg("-f")
                .arg("wav") // Force WAV format
                .arg("-y") // Overwrite output file
                .arg(&wav_path)
                .output()
                .unwrap();

            eprintln!("FFmpeg conversion details:");
            eprintln!("stdout: {}", String::from_utf8_lossy(&ffmpeg_output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&ffmpeg_output.stderr));

            if !ffmpeg_output.status.success() {
                eprintln!("FFmpeg conversion failed!");
                return Json(json!({
                    "success": false,
                    "error": format!("FFmpeg conversion failed: {}", String::from_utf8_lossy(&ffmpeg_output.stderr))
                }));
            }

            // Check WAV file size
            let wav_size = match std::fs::metadata(&wav_path) {
                Ok(metadata) => metadata.len(),
                Err(e) => {
                    eprintln!("Error getting WAV file metadata: {}", e);
                    return Json(json!({
                        "success": false,
                        "error": format!("Failed to verify WAV file: {}", e)
                    }));
                }
            };
            eprintln!("WAV file size: {} bytes", wav_size);

            // Check if WAV file exists and has content
            if wav_size == 0 {
                eprintln!("Error: WAV file is empty!");
                return Json(json!({
                    "success": false,
                    "error": "WAV conversion failed - output file is empty"
                }));
            }

            // Analyze WAV file
            eprintln!("Analyzing WAV file with FFprobe...");
            let ffprobe_wav = Command::new("ffprobe")
                .arg("-v")
                .arg("error") // Only show errors
                .arg("-show_format")
                .arg("-show_streams")
                .arg(&wav_path)
                .output()
                .unwrap();

            let wav_probe_output = String::from_utf8_lossy(&ffprobe_wav.stdout);
            eprintln!("WAV FFprobe analysis:");
            eprintln!("{}", wav_probe_output);

            if !ffprobe_wav.status.success() {
                eprintln!(
                    "WAV FFprobe error: {}",
                    String::from_utf8_lossy(&ffprobe_wav.stderr)
                );
                return Json(json!({
                    "success": false,
                    "error": format!("Invalid WAV file: {}", String::from_utf8_lossy(&ffprobe_wav.stderr))
                }));
            }

            // Run whisper transcription
            eprintln!("Running whisper on WAV file...");
            let output = Command::new(&whisper_path)
                .arg("-m")
                .arg(&model_path)
                .arg("-f")
                .arg(&wav_path)
                .arg("-l")
                .arg("en")
                .arg("-t")
                .arg("4")
                .arg("-pp")
                .arg("0")
                .arg("-otxt")
                .output()
                .unwrap();

            eprintln!("Whisper process completed");
            eprintln!(
                "Whisper stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            eprintln!(
                "Whisper stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            if output.status.success() {
                // Read the output text file
                let txt_path = format!("{}.txt", wav_path);
                match fs::read_to_string(&txt_path).await {
                    Ok(text) => {
                        // Clean up temporary files
                        eprintln!("Cleaning up temporary files...");
                        let _ = fs::remove_file(&txt_path).await;

                        eprintln!("Transcription successful: {}", text.trim());
                        return Json(json!({
                            "success": true,
                            "text": text.trim()
                        }));
                    }
                    Err(e) => {
                        eprintln!("Error reading transcription output: {}", e);
                        return Json(json!({
                            "success": false,
                            "error": format!("Failed to read transcription output: {}", e)
                        }));
                    }
                }
            } else {
                eprintln!("Whisper process failed");
                eprintln!("Error output: {}", String::from_utf8_lossy(&output.stderr));
                eprintln!(
                    "Standard output: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                return Json(json!({
                    "success": false,
                    "error": format!("Whisper failed: {}", String::from_utf8_lossy(&output.stderr))
                }));
            }
        } else {
            eprintln!("Error: Failed to read audio data from multipart field");
        }
    }

    eprintln!("Error: No valid audio data found in request");
    Json(json!({
        "success": false,
        "error": "Failed to process audio"
    }))
}
