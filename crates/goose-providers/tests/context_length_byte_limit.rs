use goose_providers::errors::ProviderError;
use goose_providers::http_status::{
    is_context_length_exceeded_message, map_http_error_to_provider_error,
};
use reqwest::StatusCode;
use serde_json::json;

#[test]
fn byte_size_limit_messages_classify_as_context_length_exceeded() {
    let messages = [
        "Server received a request which exceeds maximum allowed content length. RequestSize(bytes): 34021227, Limit(bytes): 33554432.",
        "Request body size exceeds the maximum allowed limit",
        "Request body is too large",
        "Request payload too large",
        "Content-Length exceeds the maximum allowed request size",
    ];

    for message in messages {
        assert!(
            is_context_length_exceeded_message(message),
            "expected context-length match for: {message}"
        );
    }
}

#[test]
fn byte_size_limit_bad_request_maps_to_context_length_exceeded() {
    let message = "Server received a request which exceeds maximum allowed content length. RequestSize(bytes): 34021227, Limit(bytes): 33554432.";
    let error = map_http_error_to_provider_error(
        StatusCode::BAD_REQUEST,
        Some(json!({ "error": { "message": message } })),
        "https://example.com/v1/messages",
    );

    assert_eq!(
        error,
        ProviderError::ContextLengthExceeded(message.to_string())
    );
}

#[test]
fn generic_length_errors_are_not_context_length_exceeded() {
    let messages = [
        "metadata length exceeds maximum allowed",
        "temperature exceeds maximum allowed value",
        "Invalid request body: temperature exceeds maximum allowed value",
        "tools[0].description content length exceeds maximum allowed",
        "response content length exceeds maximum allowed",
        "max_tokens must be less than or equal to 4096",
    ];

    for message in messages {
        assert!(
            !is_context_length_exceeded_message(message),
            "expected generic bad request for: {message}"
        );
    }
}
