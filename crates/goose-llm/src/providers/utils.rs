use std::{env, io::Read, path::Path, time::Duration};

use anyhow::Result;
use base64::Engine;
use regex::Regex;
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};

use super::base::Usage;
use crate::{
    model::ModelConfig,
    providers::errors::{OpenAIError, ProviderError},
    types::core::ImageContent,
};

#[derive(serde::Deserialize)]
struct OpenAIErrorResponse {
    error: OpenAIError,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
pub enum ImageFormat {
    #[default]
    OpenAi,
    Anthropic,
}

/// Timeout in seconds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Timeout(u32);
impl Default for Timeout {
    fn default() -> Self {
        Timeout(60)
    }
}

/// Configuration for mutual TLS
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MutualTlsConfig {
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub ca_cert_path: Option<String>,
}

/// Build a reqwest client with optional mutual TLS configuration
pub fn build_http_client(
    timeout_secs: u64,
    mtls_config: Option<&MutualTlsConfig>,
) -> Result<Client> {
    let mut client_builder = Client::builder().timeout(Duration::from_secs(timeout_secs));

    if let Some(mtls) = mtls_config {
        // Load client certificate and key if both are provided
        if let (Some(cert_path), Some(key_path)) = (&mtls.client_cert_path, &mtls.client_key_path) {
            let cert_pem = std::fs::read(cert_path)
                .map_err(|e| anyhow::anyhow!("Failed to read client certificate: {}", e))?;
            let key_pem = std::fs::read(key_path)
                .map_err(|e| anyhow::anyhow!("Failed to read client key: {}", e))?;

            let identity = reqwest::Identity::from_pem(&[&cert_pem[..], &key_pem[..]].concat())
                .map_err(|e| anyhow::anyhow!("Failed to create client identity: {}", e))?;

            client_builder = client_builder.identity(identity);
        }

        // Load CA certificate if provided
        if let Some(ca_path) = &mtls.ca_cert_path {
            let ca_cert = std::fs::read(ca_path)
                .map_err(|e| anyhow::anyhow!("Failed to read CA certificate: {}", e))?;

            let ca_cert = reqwest::Certificate::from_pem(&ca_cert)
                .map_err(|e| anyhow::anyhow!("Failed to parse CA certificate: {}", e))?;

            client_builder = client_builder.add_root_certificate(ca_cert);
        }
    }

    client_builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))
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
            Err(ProviderError::Authentication(format!(
                "Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                Status: {}. Response: {:?}",
                status, payload
            )))
        }
        StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND => {
            tracing::debug!(
                "{}",
                format!(
                    "Provider request failed with status: {}. Payload: {:?}",
                    status, payload
                )
            );
            if let Ok(err_resp) = from_value::<OpenAIErrorResponse>(payload) {
                let err = err_resp.error;
                if err.is_context_length_exceeded() {
                    return Err(ProviderError::ContextLengthExceeded(
                        err.message.unwrap_or("Unknown error".to_string()),
                    ));
                }
                return Err(ProviderError::RequestFailed(format!(
                    "{} (status {})",
                    err,
                    status.as_u16()
                )));
            }
            Err(ProviderError::RequestFailed(format!(
                "Unknown error (status {})",
                status
            )))
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
        }
        StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
            Err(ProviderError::ServerError(format!("{:?}", payload)))
        }
        _ => {
            tracing::debug!(
                "{}",
                format!(
                    "Provider request failed with status: {}. Payload: {:?}",
                    status, payload
                )
            );
            Err(ProviderError::RequestFailed(format!(
                "Request failed with status: {}",
                status
            )))
        }
    }
}

/// Get a secret from environment variables. The secret is expected to be in JSON format.
pub fn get_env(key: &str) -> Result<String> {
    // check environment variables (convert to uppercase)
    let env_key = key.to_uppercase();
    if let Ok(val) = env::var(&env_key) {
        let value: Value = serde_json::from_str(&val).unwrap_or(Value::String(val));
        Ok(serde_json::from_value(value)?)
    } else {
        Err(anyhow::anyhow!(
            "Environment variable {} not found",
            env_key
        ))
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
                // GIF: 47 49 46 38
                [0x47, 0x49, 0x46, 0x38] => true,
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
                ));
            }
        },
        None => {
            return Err(ProviderError::RequestFailed(
                "Unknown image format".to_string(),
            ));
        }
    };

    // Convert to base64
    let data = base64::prelude::BASE64_STANDARD.encode(&bytes);

    Ok(ImageContent {
        mime_type: mime_type.to_string(),
        data,
    })
}

pub fn emit_debug_trace(
    model_config: &ModelConfig,
    payload: &Value,
    response: &Value,
    usage: &Usage,
) {
    tracing::debug!(
        model_config = %serde_json::to_string_pretty(model_config).unwrap_or_default(),
        input = %serde_json::to_string_pretty(payload).unwrap_or_default(),
        output = %serde_json::to_string_pretty(response).unwrap_or_default(),
        input_tokens = ?usage.input_tokens.unwrap_or_default(),
        output_tokens = ?usage.output_tokens.unwrap_or_default(),
        total_tokens = ?usage.total_tokens.unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_build_http_client_without_mtls() {
        let client = build_http_client(30, None);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_with_empty_mtls() {
        let mtls_config = MutualTlsConfig::default();
        let client = build_http_client(30, Some(&mtls_config));
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_with_invalid_cert_paths() {
        let mtls_config = MutualTlsConfig {
            client_cert_path: Some("nonexistent_cert.pem".to_string()),
            client_key_path: Some("nonexistent_key.pem".to_string()),
            ca_cert_path: Some("nonexistent_ca.pem".to_string()),
        };
        let client = build_http_client(30, Some(&mtls_config));
        assert!(client.is_err());
    }

    #[test]
    fn test_mutual_tls_config_default() {
        let config = MutualTlsConfig::default();
        assert!(config.client_cert_path.is_none());
        assert!(config.client_key_path.is_none());
        assert!(config.ca_cert_path.is_none());
    }
}
