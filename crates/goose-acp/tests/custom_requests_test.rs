#[allow(dead_code)]
mod common_tests;

use common_tests::fixtures::server::AcpServerConnection;
use common_tests::fixtures::{
    run_test, send_custom, Connection, Session, SessionData, TestConnectionConfig,
};
use goose_test_support::EnforceSessionId;
use std::sync::Arc;

use common_tests::fixtures::OpenAiFixture;

#[test]
fn test_custom_session_get() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let mut conn = AcpServerConnection::new(TestConnectionConfig::default(), openai).await;

        let SessionData { session, .. } = conn.new_session().await.unwrap();
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "session/get",
            serde_json::json!({
                "sessionId": session_id,
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
fn test_custom_get_tools() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], Arc::new(EnforceSessionId::default())).await;
        let mut conn = AcpServerConnection::new(TestConnectionConfig::default(), openai).await;

        let SessionData { session, .. } = conn.new_session().await.unwrap();
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/tools",
            serde_json::json!({ "sessionId": session_id }),
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
        let conn = AcpServerConnection::new(TestConnectionConfig::default(), openai).await;

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
        let conn = AcpServerConnection::new(TestConnectionConfig::default(), openai).await;

        let result = send_custom(conn.cx(), "_unknown/method", serde_json::json!({})).await;
        assert!(result.is_err(), "expected method_not_found error");
    });
}
