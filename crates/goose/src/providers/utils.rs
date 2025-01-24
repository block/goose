use super::base::Usage;
use anyhow::{Error, Result};
use regex::Regex;
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tracing::debug;

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

// Maps a non-ok response status to a ProviderError
pub async fn non_ok_response_to_provider_error(
    payload: Value,
    response: Response,
) -> ProviderError {
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ProviderError::Authentication(format!("Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                Status: {}. Response: {:?}", response.status(), response.text().await.unwrap_or_default()))
        }
        StatusCode::TOO_MANY_REQUESTS => {
            ProviderError::RateLimitExceeded(format!("Rate limit exceeded. Please retry after some time. Status: {}", response.status()))
        }
        StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
            ProviderError::ServerError(format!("Server error occurred. Status: {}", response.status()))
        }
        _ => {
            let status = response.status();
            tracing::debug!(
                "{}", format!("Provider request failed with status: {}. Body: {:?}. Payload: {}", status, response.text().await.unwrap_or_default(), payload)
            );
            ProviderError::RequestFailed(format!("Request failed with status: {}.", status))
        }
    }
}

pub async fn handle_response(payload: Value, response: Response) -> Result<Value, Error> {
    match response.status() {
        StatusCode::OK => Ok(response.json().await?),
        _ => {
            let provider_error = non_ok_response_to_provider_error(payload, response).await;
            Err(anyhow::anyhow!(provider_error.to_string()))
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

pub fn check_bedrock_context_length_error(error: &Value) -> Option<ProviderError> {
    let external_message = error
        .get("external_model_message")?
        .get("message")?
        .as_str()?;
    if external_message.to_lowercase().contains("too long") {
        Some(ProviderError::ContextLengthExceeded(
            external_message.to_string(),
        ))
    } else {
        None
    }
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

    debug!(
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
    fn test_check_bedrock_context_length_error() {
        let error = json!({
            "error": "Received error from amazon-bedrock",
            "external_model_message": {
                "message": "Input is too long for requested model."
            }
        });

        let result = check_bedrock_context_length_error(&error);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().to_string(),
            "Context length exceeded: Input is too long for requested model."
        );

        let error = json!({
            "error": "Some other error",
            "external_model_message": {
                "message": "Some other message"
            }
        });

        let result = check_bedrock_context_length_error(&error);
        assert!(result.is_none());
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
}
