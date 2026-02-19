use goose::conversation::message::Message;
use goose::model::ModelConfig;
use goose::providers::api_client::{ApiClient, AuthMethod};
use goose::providers::base::Provider;
use goose::providers::openai::OpenAiProvider;
use goose::session_context::SESSION_ID_HEADER;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

#[derive(Clone, Default)]
struct HeaderCapture {
    captured_headers: Arc<Mutex<Vec<Option<String>>>>,
}

impl HeaderCapture {
    fn new() -> Self {
        Self {
            captured_headers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn capture_session_header(&self, req: &Request) {
        let session_id = req
            .headers
            .get(SESSION_ID_HEADER)
            .map(|v| v.to_str().unwrap().to_string());
        self.captured_headers.lock().unwrap().push(session_id);
    }

    fn get_captured(&self) -> Vec<Option<String>> {
        self.captured_headers.lock().unwrap().clone()
    }
}

fn create_test_provider(mock_server_url: &str) -> Box<dyn Provider> {
    let api_client = ApiClient::new(
        mock_server_url.to_string(),
        AuthMethod::BearerToken("test-key".to_string()),
    )
    .unwrap();
    let model = ModelConfig::new_or_fail("gpt-5-nano");
    Box::new(OpenAiProvider::new(api_client, model))
}

async fn setup_mock_server() -> (MockServer, HeaderCapture, Box<dyn Provider>) {
    let mock_server = MockServer::start().await;
    let capture = HeaderCapture::new();
    let capture_clone = capture.clone();

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |req: &Request| {
            capture_clone.capture_session_header(req);
            // Return SSE streaming format
            let sse_response = format!(
                "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                json!({
                    "choices": [{
                        "delta": {
                            "content": "Hi there! How can I help you today?",
                            "role": "assistant"
                        },
                        "index": 0
                    }],
                    "created": 1755133833,
                    "id": "chatcmpl-test",
                    "model": "gpt-5-nano"
                }),
                json!({
                    "choices": [],
                    "usage": {
                        "completion_tokens": 10,
                        "prompt_tokens": 8,
                        "total_tokens": 18
                    }
                })
            );
            ResponseTemplate::new(200)
                .set_body_string(sse_response)
                .insert_header("content-type", "text/event-stream")
        })
        .mount(&mock_server)
        .await;

    let provider = create_test_provider(&mock_server.uri());
    (mock_server, capture, provider)
}

async fn make_request(provider: &dyn Provider, session_id: &str) {
    let message = Message::user().with_text("test message");
    let model_config = provider.get_model_config();
    let _ = provider
        .complete(
            &model_config,
            session_id,
            "You are a helpful assistant.",
            &[message],
            &[],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn test_session_id_propagation_to_llm() {
    let (_, capture, provider) = setup_mock_server().await;

    make_request(provider.as_ref(), "integration-test-session-123").await;

    assert_eq!(
        capture.get_captured(),
        vec![Some("integration-test-session-123".to_string())]
    );
}

#[tokio::test]
async fn test_session_id_always_present() {
    let (_, capture, provider) = setup_mock_server().await;

    make_request(provider.as_ref(), "test-session-id").await;

    assert_eq!(
        capture.get_captured(),
        vec![Some("test-session-id".to_string())]
    );
}

#[tokio::test]
async fn test_session_id_matches_across_calls() {
    let (_, capture, provider) = setup_mock_server().await;

    let session_id = "consistent-session-456";
    make_request(provider.as_ref(), session_id).await;
    make_request(provider.as_ref(), session_id).await;
    make_request(provider.as_ref(), session_id).await;

    assert_eq!(
        capture.get_captured(),
        vec![Some(session_id.to_string()); 3]
    );
}

#[tokio::test]
async fn test_different_sessions_have_different_ids() {
    let (_, capture, provider) = setup_mock_server().await;

    let session_id_1 = "session-one";
    let session_id_2 = "session-two";
    make_request(provider.as_ref(), session_id_1).await;
    make_request(provider.as_ref(), session_id_2).await;

    assert_eq!(
        capture.get_captured(),
        vec![
            Some(session_id_1.to_string()),
            Some(session_id_2.to_string())
        ]
    );
}
