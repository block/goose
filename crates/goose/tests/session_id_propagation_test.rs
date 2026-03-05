use goose::conversation::message::Message;
use goose::model::ModelConfig;
use goose::providers::api_client::{ApiClient, AuthMethod};
use goose::providers::base::Provider;
use goose::providers::openai::OpenAiProvider;
use goose::session_context::SESSION_ID_HEADER;
use opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest;
use opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue;
use prost::Message as _;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::Instrument;
use tracing_subscriber::prelude::*;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_session_id_propagates_to_log_records() {
    let mock_server = MockServer::start().await;
    let log_bodies: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));

    // Mount /v1/logs (and /v1/traces, /v1/metrics to avoid 404s)
    for signal_path in ["/v1/logs", "/v1/traces", "/v1/metrics"] {
        let bodies = if signal_path == "/v1/logs" {
            log_bodies.clone()
        } else {
            Arc::new(Mutex::new(Vec::new()))
        };
        Mock::given(method("POST"))
            .and(path(signal_path))
            .respond_with(move |req: &Request| {
                bodies.lock().unwrap().push(req.body.clone());
                ResponseTemplate::new(200)
            })
            .expect(0..)
            .mount(&mock_server)
            .await;
    }

    let _otel_guard = goose_test_support::otel::clear_otel_env(&[
        ("OTEL_TRACES_EXPORTER", "none"),
        ("OTEL_METRICS_EXPORTER", "none"),
    ]);
    let endpoint = format!("http://127.0.0.1:{}", mock_server.address().port());
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", &endpoint);

    let layers = goose::otel::otlp::init_otlp_layers(goose::config::Config::global());

    let subscriber = tracing_subscriber::registry().with(layers);
    let _guard = tracing::subscriber::set_default(subscriber);

    let span = tracing::info_span!("test", session.id = "test-session-42");
    async {
        tokio::task::yield_now().await;
        tracing::info!("hello from test");
    }
    .instrument(span)
    .await;

    // Drop subscriber before shutdown to avoid capturing OTel internal logs
    drop(_guard);
    goose::otel::otlp::shutdown_otlp();

    let bodies = log_bodies.lock().unwrap();
    assert!(!bodies.is_empty());
    for body in bodies.iter() {
        let req = ExportLogsServiceRequest::decode(&body[..]).unwrap();
        for rl in &req.resource_logs {
            for sl in &rl.scope_logs {
                for record in &sl.log_records {
                    let has_session_id = record.attributes.iter().any(|kv| {
                        kv.key == "session.id"
                            && kv
                                .value
                                .as_ref()
                                .is_some_and(|v| matches!(&v.value, Some(StringValue(s)) if s == "test-session-42"))
                    });
                    assert!(has_session_id);
                }
            }
        }
    }
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
