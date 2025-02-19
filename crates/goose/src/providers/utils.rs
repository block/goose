use super::base::Usage;
use super::errors::GoogleErrorCode;
use anyhow::Result;
use base64::Engine;
use regex::Regex;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::io::Read;
use std::path::Path;

use crate::providers::errors::ProviderError;
use mcp_core::content::ImageContent;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    OpenAi,
    Anthropic,
}

/// Convert an image content into an image json based on format
pub fn convert_image(image: &ImageContent, image_format: &ImageFormat) -> Value {
    match image_format {
        ImageFormat::OpenAi => json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{}", image.mime_type, image.data)
            }
        }),
        ImageFormat::Anthropic => json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": image.mime_type,
                "data": image.data,
            }
        }),
    }
}

/// Handle response from OpenAI compatible endpoints
/// Error codes: https://platform.openai.com/docs/guides/error-codes
/// Context window exceeded: https://community.openai.com/t/help-needed-tackling-context-length-limits-in-openai-models/617543
pub async fn handle_response_openai_compat(response: Response) -> Result<Value, ProviderError> {
    let status = response.status();
    // Try to parse the response body as JSON (if applicable)
    let payload = match response.json::<Value>().await {
        Ok(json) => json,
        Err(e) => return Err(ProviderError::RequestFailed(e.to_string())),
    };

    match status {
        StatusCode::OK => Ok(payload),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            Err(ProviderError::Authentication(format!("Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                Status: {}. Response: {:?}", status, payload)))
        }
        StatusCode::BAD_REQUEST => {
            let mut message = "Unknown error".to_string();
            if let Some(error) = payload.get("error") {
                tracing::debug!("Bad Request Error: {error:?}");
                message = error
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error")
                            .to_string();

                if let Some(code) = error.get("code").and_then(|c| c.as_str()) {
                    if code == "context_length_exceeded" || code == "string_above_max_length" {
                        return Err(ProviderError::ContextLengthExceeded(message));
                    }
                }
            }
            tracing::debug!(
                "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
            );
            Err(ProviderError::RequestFailed(format!("Request failed with status: {}. Message: {}", status, message)))
        }
        StatusCode::NOT_FOUND => {
            Err(ProviderError::RequestFailed(format!("{:?}", payload)))
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
        }
        StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
            Err(ProviderError::ServerError(format!("{:?}", payload)))
        }
        _ => {
            tracing::debug!(
                "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
            );
            Err(ProviderError::RequestFailed(format!("Request failed with status: {}", status)))
        }
    }
}

/// Check if the model is a Google model based on the "model" field in the payload.
///
/// ### Arguments
/// - `payload`: The JSON payload as a `serde_json::Value`.
///
/// ### Returns
/// - `bool`: Returns `true` if the model is a Google model, otherwise `false`.
pub fn is_google_model(payload: &Value) -> bool {
    if let Some(model) = payload.get("model").and_then(|m| m.as_str()) {
        // Check if the model name contains "google"
        return model.to_lowercase().contains("google");
    }
    false
}

/// Extracts `StatusCode` from response status or payload error code.
/// This function first checks the status code of the response. If the status is successful (2xx),
/// it then checks the payload for any error codes and maps them to appropriate `StatusCode`.
/// If the status is not successful (e.g., 4xx or 5xx), the original status code is returned.
fn get_google_final_status(status: StatusCode, payload: Option<&Value>) -> StatusCode {
    // If the status is successful, check for an error in the payload
    if status.is_success() {
        if let Some(payload) = payload {
            if let Some(error) = payload.get("error") {
                if let Some(code) = error.get("code").and_then(|c| c.as_u64()) {
                    if let Some(google_error) = GoogleErrorCode::from_code(code) {
                        return google_error.to_status_code();
                    }
                }
            }
        }
    }
    status
}

/// Handle response from Google Gemini API-compatible endpoints.
///
/// Processes HTTP responses, handling specific statuses and parsing the payload
/// for error messages. Logs the response payload for debugging purposes.
///
/// ### References
/// - Error Codes: https://ai.google.dev/gemini-api/docs/troubleshooting?lang=python
///
/// ### Arguments
/// - `response`: The HTTP response to process.
///
/// ### Returns
/// - `Ok(Value)`: Parsed JSON on success.
/// - `Err(ProviderError)`: Describes the failure reason.
pub async fn handle_response_google_compat(response: Response) -> Result<Value, ProviderError> {
    let status = response.status();
    let payload: Option<Value> = response.json().await.ok();
    let final_status = get_google_final_status(status, payload.as_ref());

    match final_status {
        StatusCode::OK =>  payload.ok_or_else( || ProviderError::RequestFailed("Response body is not valid JSON".to_string()) ),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            Err(ProviderError::Authentication(format!("Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                Status: {}. Response: {:?}", final_status, payload )))
        }
        StatusCode::BAD_REQUEST => {
            let mut error_msg = "Unknown error".to_string();
            if let Some(payload) = &payload {
                if let Some(error) = payload.get("error") {
                    error_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error").to_string();
                    let error_status = error.get("status").and_then(|s| s.as_str()).unwrap_or("Unknown status");
                    if error_status == "INVALID_ARGUMENT" && error_msg.to_lowercase().contains("exceeds") {
                        return Err(ProviderError::ContextLengthExceeded(error_msg.to_string()));
                    }
                }
            }
            tracing::debug!(
                "{}", format!("Provider request failed with status: {}. Payload: {:?}", final_status, payload)
            );
            Err(ProviderError::RequestFailed(format!("Request failed with status: {}. Message: {}", final_status, error_msg)))
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
        }
        StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
            Err(ProviderError::ServerError(format!("{:?}", payload)))
        }
        _ => {
            tracing::debug!(
                "{}", format!("Provider request failed with status: {}. Payload: {:?}", final_status, payload)
            );
            Err(ProviderError::RequestFailed(format!("Request failed with status: {}", final_status)))
        }
    }
}

pub fn sanitize_function_name(name: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z0-9_-]").unwrap();
    re.replace_all(name, "_").to_string()
}

pub fn is_valid_function_name(name: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    re.is_match(name)
}

/// Extract the model name from a JSON object. Common with most providers to have this top level attribute.
pub fn get_model(data: &Value) -> String {
    if let Some(model) = data.get("model") {
        if let Some(model_str) = model.as_str() {
            model_str.to_string()
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    }
}

/// Check if a file is actually an image by examining its magic bytes
fn is_image_file(path: &Path) -> bool {
    if let Ok(mut file) = std::fs::File::open(path) {
        let mut buffer = [0u8; 8]; // Large enough for most image magic numbers
        if file.read(&mut buffer).is_ok() {
            // Check magic numbers for common image formats
            return match &buffer[0..4] {
                // PNG: 89 50 4E 47
                [0x89, 0x50, 0x4E, 0x47] => true,
                // JPEG: FF D8 FF
                [0xFF, 0xD8, 0xFF, _] => true,
                _ => false,
            };
        }
    }
    false
}

/// Detect if a string contains a path to an image file
pub fn detect_image_path(text: &str) -> Option<&str> {
    // Basic image file extension check
    let extensions = [".png", ".jpg", ".jpeg"];

    // Find any word that ends with an image extension
    for word in text.split_whitespace() {
        if extensions
            .iter()
            .any(|ext| word.to_lowercase().ends_with(ext))
        {
            let path = Path::new(word);
            // Check if it's an absolute path and file exists
            if path.is_absolute() && path.is_file() {
                // Verify it's actually an image file
                if is_image_file(path) {
                    return Some(word);
                }
            }
        }
    }
    None
}

/// Convert a local image file to base64 encoded ImageContent
pub fn load_image_file(path: &str) -> Result<ImageContent, ProviderError> {
    let path = Path::new(path);

    // Verify it's an image before proceeding
    if !is_image_file(path) {
        return Err(ProviderError::RequestFailed(
            "File is not a valid image".to_string(),
        ));
    }

    // Read the file
    let bytes = std::fs::read(path)
        .map_err(|e| ProviderError::RequestFailed(format!("Failed to read image file: {}", e)))?;

    // Detect mime type from extension
    let mime_type = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => match ext.to_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            _ => {
                return Err(ProviderError::RequestFailed(
                    "Unsupported image format".to_string(),
                ))
            }
        },
        None => {
            return Err(ProviderError::RequestFailed(
                "Unknown image format".to_string(),
            ))
        }
    };

    // Convert to base64
    let data = base64::prelude::BASE64_STANDARD.encode(&bytes);

    Ok(ImageContent {
        mime_type: mime_type.to_string(),
        data,
        annotations: None,
    })
}

pub fn unescape_json_values(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let new_map: Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), unescape_json_values(v))) // Process each value
                .collect();
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            let new_array: Vec<Value> = arr.iter().map(unescape_json_values).collect();
            Value::Array(new_array)
        }
        Value::String(s) => {
            let unescaped = s
                .replace("\\\\n", "\n")
                .replace("\\\\t", "\t")
                .replace("\\\\r", "\r")
                .replace("\\\\\"", "\"")
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r")
                .replace("\\\"", "\"");
            Value::String(unescaped)
        }
        _ => value.clone(),
    }
}

pub fn emit_debug_trace<T: serde::Serialize>(
    model_config: &T,
    payload: &impl serde::Serialize,
    response: &Value,
    usage: &Usage,
) {
    // Handle both Map<String, Value> and Value payload types
    let payload_str = match serde_json::to_value(payload) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(_) => serde_json::to_string_pretty(&payload).unwrap_or_default(),
    };

    tracing::debug!(
        model_config = %serde_json::to_string_pretty(model_config).unwrap_or_default(),
        input = %payload_str,
        output = %serde_json::to_string_pretty(response).unwrap_or_default(),
        input_tokens = ?usage.input_tokens.unwrap_or_default(),
        output_tokens = ?usage.output_tokens.unwrap_or_default(),
        total_tokens = ?usage.total_tokens.unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_image_path() {
        // Create a temporary PNG file with valid PNG magic numbers
        let temp_dir = tempfile::tempdir().unwrap();
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, &png_data).unwrap();
        let png_path_str = png_path.to_str().unwrap();

        // Create a fake PNG (wrong magic numbers)
        let fake_png_path = temp_dir.path().join("fake.png");
        std::fs::write(&fake_png_path, b"not a real png").unwrap();

        // Test with valid PNG file using absolute path
        let text = format!("Here is an image {}", png_path_str);
        assert_eq!(detect_image_path(&text), Some(png_path_str));

        // Test with non-image file that has .png extension
        let text = format!("Here is a fake image {}", fake_png_path.to_str().unwrap());
        assert_eq!(detect_image_path(&text), None);

        // Test with non-existent file
        let text = "Here is a fake.png that doesn't exist";
        assert_eq!(detect_image_path(text), None);

        // Test with non-image file
        let text = "Here is a file.txt";
        assert_eq!(detect_image_path(text), None);

        // Test with relative path (should not match)
        let text = "Here is a relative/path/image.png";
        assert_eq!(detect_image_path(text), None);
    }

    #[test]
    fn test_load_image_file() {
        // Create a temporary PNG file with valid PNG magic numbers
        let temp_dir = tempfile::tempdir().unwrap();
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, &png_data).unwrap();
        let png_path_str = png_path.to_str().unwrap();

        // Create a fake PNG (wrong magic numbers)
        let fake_png_path = temp_dir.path().join("fake.png");
        std::fs::write(&fake_png_path, b"not a real png").unwrap();
        let fake_png_path_str = fake_png_path.to_str().unwrap();

        // Test loading valid PNG file
        let result = load_image_file(png_path_str);
        assert!(result.is_ok());
        let image = result.unwrap();
        assert_eq!(image.mime_type, "image/png");

        // Test loading fake PNG file
        let result = load_image_file(fake_png_path_str);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a valid image"));

        // Test non-existent file
        let result = load_image_file("nonexistent.png");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_function_name() {
        assert_eq!(sanitize_function_name("hello-world"), "hello-world");
        assert_eq!(sanitize_function_name("hello world"), "hello_world");
        assert_eq!(sanitize_function_name("hello@world"), "hello_world");
    }

    #[test]
    fn test_is_valid_function_name() {
        assert!(is_valid_function_name("hello-world"));
        assert!(is_valid_function_name("hello_world"));
        assert!(!is_valid_function_name("hello world"));
        assert!(!is_valid_function_name("hello@world"));
    }

    #[test]
    fn unescape_json_values_with_object() {
        let value = json!({"text": "Hello\\nWorld"});
        let unescaped_value = unescape_json_values(&value);
        assert_eq!(unescaped_value, json!({"text": "Hello\nWorld"}));
    }

    #[test]
    fn unescape_json_values_with_array() {
        let value = json!(["Hello\\nWorld", "Goodbye\\tWorld"]);
        let unescaped_value = unescape_json_values(&value);
        assert_eq!(unescaped_value, json!(["Hello\nWorld", "Goodbye\tWorld"]));
    }

    #[test]
    fn unescape_json_values_with_string() {
        let value = json!("Hello\\nWorld");
        let unescaped_value = unescape_json_values(&value);
        assert_eq!(unescaped_value, json!("Hello\nWorld"));
    }

    #[test]
    fn unescape_json_values_with_mixed_content() {
        let value = json!({
            "text": "Hello\\nWorld\\\\n!",
            "array": ["Goodbye\\tWorld", "See you\\rlater"],
            "nested": {
                "inner_text": "Inner\\\"Quote\\\""
            }
        });
        let unescaped_value = unescape_json_values(&value);
        assert_eq!(
            unescaped_value,
            json!({
                "text": "Hello\nWorld\n!",
                "array": ["Goodbye\tWorld", "See you\rlater"],
                "nested": {
                    "inner_text": "Inner\"Quote\""
                }
            })
        );
    }

    #[test]
    fn unescape_json_values_with_no_escapes() {
        let value = json!({"text": "Hello World"});
        let unescaped_value = unescape_json_values(&value);
        assert_eq!(unescaped_value, json!({"text": "Hello World"}));
    }

    #[test]
    fn test_is_google_model() {
        // Define the test cases as a vector of tuples
        let test_cases = vec![
            // (input, expected_result)
            (json!({ "model": "google_gemini" }), true),
            (json!({ "model": "microsoft_bing" }), false),
            (json!({ "model": "" }), false),
            (json!({}), false),
            (json!({ "model": "Google_XYZ" }), true),
            (json!({ "model": "google_abc" }), true),
        ];

        // Iterate through each test case and assert the result
        for (payload, expected_result) in test_cases {
            assert_eq!(is_google_model(&payload), expected_result);
        }
    }

    #[test]
    fn test_get_google_final_status_success() {
        let status = StatusCode::OK;
        let payload = json!({});
        let result = get_google_final_status(status, Some(&payload));
        assert_eq!(result, StatusCode::OK);
    }

    #[test]
    fn test_get_google_final_status_with_error_code() {
        // Test error code mappings for different payload error codes
        let test_cases = vec![
            // (error code, status, expected status code)
            (200, None, StatusCode::OK),
            (429, Some(StatusCode::OK), StatusCode::TOO_MANY_REQUESTS),
            (400, Some(StatusCode::OK), StatusCode::BAD_REQUEST),
            (401, Some(StatusCode::OK), StatusCode::UNAUTHORIZED),
            (403, Some(StatusCode::OK), StatusCode::FORBIDDEN),
            (404, Some(StatusCode::OK), StatusCode::NOT_FOUND),
            (500, Some(StatusCode::OK), StatusCode::INTERNAL_SERVER_ERROR),
            (503, Some(StatusCode::OK), StatusCode::SERVICE_UNAVAILABLE),
            (999, Some(StatusCode::OK), StatusCode::INTERNAL_SERVER_ERROR),
            (500, Some(StatusCode::BAD_REQUEST), StatusCode::BAD_REQUEST),
            (
                404,
                Some(StatusCode::INTERNAL_SERVER_ERROR),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (error_code, status, expected_status) in test_cases {
            let payload = if let Some(_status) = status {
                json!({
                    "error": {
                        "code": error_code,
                        "message": "Error message"
                    }
                })
            } else {
                json!({})
            };

            let result = get_google_final_status(status.unwrap_or(StatusCode::OK), Some(&payload));
            assert_eq!(result, expected_status);
        }
    }
}
