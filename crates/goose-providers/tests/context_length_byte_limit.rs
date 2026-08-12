//! Regression tests for byte-size request-limit errors (see goose#11171).
//!
//! A gateway/proxy that caps the request body in *bytes* returns HTTP 400
//! (not 413) when the limit is exceeded. `is_context_length_exceeded_message`
//! must recognize those byte-size phrasings so the error is classified as
//! `ContextLengthExceeded` (which triggers compaction) rather than a generic
//! `RequestFailed` that leaves an image-heavy session permanently stuck.

use goose_providers::http_status::is_context_length_exceeded_message;

#[test]
fn byte_size_limit_messages_classify_as_context_length_exceeded() {
    let messages = [
        // Exact error from a gateway enforcing a 32 MiB request-body limit.
        "Server received a request which exceeds maximum allowed content length. RequestSize(bytes): 34021227, Limit(bytes): 33554432.",
        "Request body size exceeds the maximum allowed limit",
        "content length exceeds the maximum allowed limit",
    ];

    for message in messages {
        assert!(
            is_context_length_exceeded_message(message),
            "expected context-length match for: {message}"
        );
    }
}

#[test]
fn generic_length_errors_are_not_context_length_exceeded() {
    let messages = [
        "metadata length exceeds maximum allowed",
        "temperature exceeds maximum allowed value",
        "max_tokens must be less than or equal to 4096",
    ];

    for message in messages {
        assert!(
            !is_context_length_exceeded_message(message),
            "expected generic bad request for: {message}"
        );
    }
}
