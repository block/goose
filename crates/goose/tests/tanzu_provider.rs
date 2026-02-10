#[cfg(test)]
mod tanzu_provider_tests {
    use goose::model::ModelConfig;
    use goose::providers::api_client::{ApiClient, AuthMethod};
    use goose::providers::base::{Provider, ProviderDef};
    use goose::providers::openai_compatible::OpenAiCompatibleProvider;
    use goose::providers::tanzu::TanzuAIServicesProvider;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper to create a provider pointed at a mock server.
    fn create_test_provider(mock_url: &str, model_name: &str) -> OpenAiCompatibleProvider {
        let host = format!("{}/openai", mock_url);
        let api_client =
            ApiClient::new(host, AuthMethod::BearerToken("test-jwt-token".to_string())).unwrap();

        OpenAiCompatibleProvider::new(
            "tanzu_ai".to_string(),
            api_client,
            ModelConfig::new_or_fail(model_name),
            String::new(),
        )
    }

    // --- Provider Metadata Tests ---

    #[test]
    fn test_tanzu_provider_registered_metadata() {
        let meta = TanzuAIServicesProvider::metadata();
        assert_eq!(meta.name, "tanzu_ai");
        assert_eq!(meta.display_name, "Tanzu AI Services");
        assert!(meta.allows_unlisted_models);
        assert_eq!(meta.config_keys.len(), 4);
    }

    // --- Non-Streaming Completion Tests ---

    #[tokio::test]
    async fn test_complete_with_model_basic() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .and(header("Authorization", "Bearer test-jwt-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion",
                "model": "openai/gpt-oss-120b",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! I'm running on Tanzu AI Services."
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 8,
                    "total_tokens": 18
                }
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");
        let model_config = provider.get_model_config();

        let result = provider
            .complete_with_model(
                Some("test-session"),
                &model_config,
                "You are a helpful assistant.",
                &[goose::conversation::message::Message::user().with_text("Hello")],
                &[],
            )
            .await;

        assert!(result.is_ok());
        let (message, usage) = result.unwrap();
        assert_eq!(
            message.as_concat_text(),
            "Hello! I'm running on Tanzu AI Services."
        );
        assert_eq!(usage.model, "openai/gpt-oss-120b");
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(8));
    }

    // --- Error Handling Tests ---

    #[tokio::test]
    async fn test_authentication_error_401() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": {
                    "message": "Invalid or expired JWT token",
                    "type": "authentication_error"
                }
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");
        let model_config = provider.get_model_config();

        let result = provider
            .complete_with_model(
                Some("test-session"),
                &model_config,
                "system",
                &[goose::conversation::message::Message::user().with_text("test")],
                &[],
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                goose::providers::errors::ProviderError::Authentication(_)
            ),
            "Expected Authentication error, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_rate_limit_error_429() {
        // Skip backoff to speed up tests; 1 initial + 3 retries = 4 total requests
        std::env::set_var("GOOSE_PROVIDER_SKIP_BACKOFF", "true");
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": {
                    "message": "Rate limit exceeded. Please retry after 30 seconds.",
                    "type": "rate_limit_error"
                }
            })))
            .expect(4) // 1 initial + 3 retries
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");
        let model_config = provider.get_model_config();

        let result = provider
            .complete_with_model(
                Some("test-session"),
                &model_config,
                "system",
                &[goose::conversation::message::Message::user().with_text("test")],
                &[],
            )
            .await;

        std::env::remove_var("GOOSE_PROVIDER_SKIP_BACKOFF");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                goose::providers::errors::ProviderError::RateLimitExceeded { .. }
            ),
            "Expected RateLimitExceeded error, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_server_error_502() {
        // Skip backoff to speed up tests; 1 initial + 3 retries = 4 total requests
        std::env::set_var("GOOSE_PROVIDER_SKIP_BACKOFF", "true");
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .respond_with(ResponseTemplate::new(502).set_body_json(json!({
                "error": {
                    "message": "Bad Gateway: GenAI proxy could not reach upstream AI server",
                    "type": "server_error"
                }
            })))
            .expect(4) // 1 initial + 3 retries
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");
        let model_config = provider.get_model_config();

        let result = provider
            .complete_with_model(
                Some("test-session"),
                &model_config,
                "system",
                &[goose::conversation::message::Message::user().with_text("test")],
                &[],
            )
            .await;

        std::env::remove_var("GOOSE_PROVIDER_SKIP_BACKOFF");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, goose::providers::errors::ProviderError::ServerError(_)),
            "Expected ServerError, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_context_length_exceeded_400() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": {
                    "message": "This model's maximum context length is 4096 tokens. Your input was too long.",
                    "type": "invalid_request_error"
                }
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");
        let model_config = provider.get_model_config();

        let result = provider
            .complete_with_model(
                Some("test-session"),
                &model_config,
                "system",
                &[goose::conversation::message::Message::user().with_text("test")],
                &[],
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                goose::providers::errors::ProviderError::ContextLengthExceeded(_)
            ),
            "Expected ContextLengthExceeded error, got: {:?}",
            err
        );
    }

    // --- Model Discovery Tests ---

    #[tokio::test]
    async fn test_fetch_supported_models() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/openai/models"))
            .and(header("Authorization", "Bearer test-jwt-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [
                    {"id": "openai/gpt-oss-120b", "object": "model"},
                    {"id": "llama3.2:1b", "object": "model"},
                    {"id": "qwen3-30b", "object": "model"},
                    {"id": "nomic-embed-text", "object": "model"}
                ]
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");

        let models = provider.fetch_supported_models().await.unwrap();
        assert_eq!(models.len(), 4);
        assert!(models.contains(&"openai/gpt-oss-120b".to_string()));
        assert!(models.contains(&"llama3.2:1b".to_string()));
        assert!(models.contains(&"qwen3-30b".to_string()));
    }

    // --- Bearer Token Auth Tests ---

    #[tokio::test]
    async fn test_bearer_token_sent_in_requests() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .and(header("Authorization", "Bearer test-jwt-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "test",
                "object": "chat.completion",
                "model": "test-model",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "test-model");
        let model_config = provider.get_model_config();

        let result = provider
            .complete_with_model(
                Some("test-session"),
                &model_config,
                "system",
                &[goose::conversation::message::Message::user().with_text("test")],
                &[],
            )
            .await;

        // If the bearer token wasn't sent, the mock wouldn't match and we'd get an error
        assert!(result.is_ok());
    }

    // --- Streaming Tests ---

    #[tokio::test]
    async fn test_streaming_completion() {
        let mock_server = MockServer::start().await;

        let sse_body = [
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"model\":\"openai/gpt-oss-120b\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"model\":\"openai/gpt-oss-120b\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" from\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"model\":\"openai/gpt-oss-120b\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Tanzu!\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":3,\"total_tokens\":8}}\n\n",
            "data: [DONE]\n\n",
        ]
        .join("");

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");

        let stream_result = provider
            .stream(
                "test-session",
                "You are helpful.",
                &[goose::conversation::message::Message::user().with_text("Hi")],
                &[],
            )
            .await;

        assert!(stream_result.is_ok());

        use futures::StreamExt;
        let mut stream = stream_result.unwrap();
        let mut chunks = Vec::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok((msg, _usage)) => {
                    if let Some(m) = msg {
                        chunks.push(m.as_concat_text());
                    }
                }
                Err(e) => panic!("Stream error: {:?}", e),
            }
        }

        assert!(!chunks.is_empty(), "Should have received streaming chunks");
    }

    // --- Tool Call Tests ---

    #[tokio::test]
    async fn test_completion_with_tool_calls() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/openai/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-tools",
                "object": "chat.completion",
                "model": "openai/gpt-oss-120b",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_abc123",
                            "type": "function",
                            "function": {
                                "name": "get_weather",
                                "arguments": "{\"location\": \"San Francisco\"}"
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }],
                "usage": {
                    "prompt_tokens": 20,
                    "completion_tokens": 15,
                    "total_tokens": 35
                }
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server.uri(), "openai/gpt-oss-120b");
        let model_config = provider.get_model_config();

        let result = provider
            .complete_with_model(
                Some("test-session"),
                &model_config,
                "You are a helpful assistant.",
                &[goose::conversation::message::Message::user()
                    .with_text("What's the weather in SF?")],
                &[],
            )
            .await;

        assert!(result.is_ok());
        let (message, usage) = result.unwrap();
        assert_eq!(usage.usage.total_tokens, Some(35));

        // The message should contain a tool request
        let tool_requests: Vec<_> = message
            .content
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    goose::conversation::message::MessageContent::ToolRequest(_)
                )
            })
            .collect();
        assert_eq!(tool_requests.len(), 1);
    }
}
