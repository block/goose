use futures::StreamExt;
use goose_client::{GooseClient, GooseClientConfig, GooseClientError, MessageEvent};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer, secret: &str) -> GooseClient {
    GooseClient::new(GooseClientConfig::new(server.uri(), secret)).unwrap()
}

#[tokio::test]
async fn test_auth_header_is_sent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/status"))
        .and(header("X-Secret-Key", "my-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!("ok")))
        .mount(&server)
        .await;

    let client = client_for(&server, "my-secret");
    let result: String = client.status().await.unwrap();
    assert_eq!(result, "ok");
}

#[tokio::test]
async fn test_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/status"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = client_for(&server, "wrong-key");
    let err = client.status().await.unwrap_err();
    assert!(matches!(err, GooseClientError::Unauthorized));
}

#[tokio::test]
async fn test_server_error_extracts_json_message() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/status"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({"message": "internal error detail"})),
        )
        .mount(&server)
        .await;

    let client = client_for(&server, "key");
    let err = client.status().await.unwrap_err();
    match err {
        GooseClientError::Server { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "internal error detail");
        }
        other => panic!("expected Server error, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_sse_streaming_round_trip() {
    let server = MockServer::start().await;

    let sse_body = [
        "data: {\"type\":\"Ping\"}\n\n",
        "data: {\"type\":\"Finish\",\"reason\":\"stop\",\"token_state\":{\"inputTokens\":0,\"outputTokens\":0,\"totalTokens\":0,\"accumulatedInputTokens\":0,\"accumulatedOutputTokens\":0,\"accumulatedTotalTokens\":0}}\n\n",
    ]
    .join("");

    Mock::given(method("POST"))
        .and(path("/reply"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let client = client_for(&server, "key");

    let message = goose::conversation::message::Message::user().with_text("hello");
    let request = goose_client::ChatRequest::new("session-1", message);
    let mut stream = client.reply(request).await.unwrap();

    let first = stream.next().await.unwrap().unwrap();
    assert!(matches!(first, MessageEvent::Ping));

    let second = stream.next().await.unwrap().unwrap();
    assert!(
        matches!(second, MessageEvent::Finish { ref reason, .. } if reason == "stop"),
        "expected Finish, got: {second:?}"
    );
}

#[tokio::test]
async fn test_list_tools_sends_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent/tools"))
        .and(query_param("session_id", "s1"))
        .and(query_param("extension_name", "my-ext"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let client = client_for(&server, "key");
    let tools = client.list_tools("s1", Some("my-ext")).await.unwrap();
    assert!(tools.is_empty());
}
