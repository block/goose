#[allow(dead_code)]
mod common_tests;

use common_tests::fixtures::server::ClientToAgentConnection;
use common_tests::fixtures::{run_test, Connection, Session, SessionResult, TestConnectionConfig};
use goose_test_support::EnforceSessionId;
use std::sync::Arc;

use common_tests::fixtures::OpenAiFixture;

/// Send an untyped custom request and return the result or error.
async fn send_custom(
    cx: &sacp::JrConnectionCx<sacp::ClientToAgent>,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, sacp::Error> {
    let msg = sacp::UntypedMessage::new(method, params).unwrap();
    cx.send_request(msg).block_task().await
}

#[test]
fn test_custom_session_list() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let SessionResult { session, .. } = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        // Verify the session exists via _session/get
        let get_result = send_custom(
            conn.cx(),
            "_goose/session/get",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(
            get_result.is_ok(),
            "session should exist via get: {:?}",
            get_result
        );
        let get_response = get_result.unwrap();
        assert_eq!(
            get_response
                .get("session")
                .and_then(|s| s.get("id"))
                .and_then(|v| v.as_str()),
            Some(session_id.as_ref()),
        );

        // Verify _session/list returns a valid response
        // Note: list_sessions uses INNER JOIN on messages, so a fresh session
        // with no messages won't appear. We just verify the call succeeds.
        let result = send_custom(conn.cx(), "_goose/session/list", serde_json::json!({})).await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result);
        let response = result.unwrap();
        let sessions = response.get("sessions").expect("missing 'sessions' field");
        assert!(sessions.is_array(), "sessions should be array");
    });
}

#[test]
fn test_custom_session_get() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let SessionResult { session, .. } = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/session/get",
            serde_json::json!({
                "session_id": session_id,
            }),
        )
        .await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result);

        let response = result.unwrap();
        let returned_session = response.get("session").expect("missing 'session' field");
        assert_eq!(
            returned_session.get("id").and_then(|v| v.as_str()),
            Some(session_id.as_ref())
        );
    });
}

#[test]
fn test_custom_session_delete() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let SessionResult { session, .. } = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/session/delete",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_ok(), "delete failed: {:?}", result);

        let result = send_custom(
            conn.cx(),
            "_goose/session/get",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_err(), "expected error for deleted session");
    });
}

#[test]
fn test_custom_get_tools() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let SessionResult { session, .. } = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/tools",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result);

        let response = result.unwrap();
        let tools = response.get("tools").expect("missing 'tools' field");
        assert!(tools.is_array(), "tools should be array");
    });
}

#[test]
fn test_custom_get_extensions() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let result =
            send_custom(conn.cx(), "_goose/config/extensions", serde_json::json!({})).await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result);

        let response = result.unwrap();
        assert!(
            response.get("extensions").is_some(),
            "missing 'extensions' field"
        );
        assert!(
            response.get("warnings").is_some(),
            "missing 'warnings' field"
        );
    });
}

#[test]
fn test_custom_unknown_method() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let result = send_custom(conn.cx(), "_unknown/method", serde_json::json!({})).await;
        assert!(result.is_err(), "expected method_not_found error");
    });
}

#[test]
fn test_custom_health() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let result = send_custom(conn.cx(), "_goose/health", serde_json::json!({})).await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result);

        let response = result.unwrap();
        assert_eq!(
            response.get("status").and_then(|v| v.as_str()),
            Some("ok"),
            "expected status 'ok'"
        );
    });
}

#[test]
fn test_custom_set_session_instructions() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/session/set_instructions",
            serde_json::json!({
                "session_id": session_id,
                "instructions": "You are a helpful assistant for the #eng-platform Slack channel.",
            }),
        )
        .await;
        assert!(result.is_ok(), "set_instructions failed: {:?}", result);
    });
}

#[test]
fn test_custom_set_session_instructions_unknown_session() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let result = send_custom(
            conn.cx(),
            "_goose/session/set_instructions",
            serde_json::json!({
                "session_id": "nonexistent-session-id",
                "instructions": "some instructions",
            }),
        )
        .await;
        assert!(result.is_err(), "expected error for unknown session");
    });
}

#[test]
fn test_custom_session_get_includes_token_fields() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/session/get",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result);

        let response = result.unwrap();
        let returned_session = response.get("session").expect("missing 'session' field");

        // Verify token metric fields are present (may be null for a fresh session).
        assert!(
            returned_session.get("input_tokens").is_some(),
            "missing 'input_tokens' field"
        );
        assert!(
            returned_session.get("output_tokens").is_some(),
            "missing 'output_tokens' field"
        );
        assert!(
            returned_session.get("accumulated_total_tokens").is_some(),
            "missing 'accumulated_total_tokens' field"
        );
        assert!(
            returned_session.get("accumulated_input_tokens").is_some(),
            "missing 'accumulated_input_tokens' field"
        );
        assert!(
            returned_session.get("accumulated_output_tokens").is_some(),
            "missing 'accumulated_output_tokens' field"
        );

        // model_config contains model_name and context_limit (the slackbot reads context_limit
        // via provider_config.context_limit in the HTTP API equivalent).
        // For a fresh session, model_config is populated from the configured provider.
        assert!(
            returned_session.get("model_config").is_some(),
            "missing 'model_config' field"
        );
    });
}

#[test]
fn test_session_set_model_with_provider() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        // Switch model without specifying provider (reads from config — same as before).
        // Uses camelCase per sacp's SetSessionModelRequest field naming.
        let result = send_custom(
            conn.cx(),
            "session/set_model",
            serde_json::json!({
                "sessionId": session_id,
                "modelId": "gpt-4o",
            }),
        )
        .await;
        assert!(result.is_ok(), "set_model failed: {:?}", result);
    });
}

#[test]
fn test_session_set_model_with_explicit_provider() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        // Switch provider + model explicitly — the extra `provider` field is extracted from raw
        // params before the sacp-typed parse.
        let result = send_custom(
            conn.cx(),
            "session/set_model",
            serde_json::json!({
                "sessionId": session_id,
                "modelId": "gpt-4o",
                "provider": "openai",
            }),
        )
        .await;
        assert!(
            result.is_ok(),
            "set_model with provider failed: {:?}",
            result
        );
    });
}
