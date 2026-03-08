#[allow(dead_code)]
mod common_tests;

use common_tests::fixtures::server::ClientToAgentConnection;
use common_tests::fixtures::{run_test, Connection, Session, TestConnectionConfig};
use goose_test_support::ExpectedSessionId;

use common_tests::fixtures::OpenAiFixture;

async fn send_custom(
    cx: &sacp::JrConnectionCx<sacp::ClientToAgent>,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, sacp::Error> {
    let msg = sacp::UntypedMessage::new(method, params).unwrap();
    cx.send_request(msg).block_task().await
}

#[test]
fn test_custom_config_prompts() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/config/prompts",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_ok(), "config/prompts failed: {:?}", result);

        let response = result.unwrap();
        assert!(
            response.get("prompts").is_some(),
            "missing 'prompts' field in response"
        );
        let prompts = response.get("prompts").unwrap();
        assert!(prompts.is_object(), "prompts should be an object (map)");
    });
}

#[test]
fn test_custom_config_prompt_info_not_found() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/config/prompt_info",
            serde_json::json!({
                "session_id": session_id,
                "name": "nonexistent_prompt"
            }),
        )
        .await;
        assert!(result.is_ok(), "config/prompt_info failed: {:?}", result);

        let response = result.unwrap();
        assert_eq!(
            response.get("found").and_then(|v| v.as_bool()),
            Some(false),
            "expected found=false for nonexistent prompt"
        );
    });
}

#[test]
fn test_custom_agent_provider_info() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/agent/provider_info",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_ok(), "agent/provider_info failed: {:?}", result);

        let response = result.unwrap();
        assert!(
            response.get("provider_name").is_some(),
            "missing 'provider_name'"
        );
        assert!(response.get("model_name").is_some(), "missing 'model_name'");
        assert!(
            response.get("context_limit").is_some(),
            "missing 'context_limit'"
        );
        let context_limit = response
            .get("context_limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(context_limit > 0, "context_limit should be positive");
    });
}

#[test]
fn test_custom_agent_provider_info_invalid_session() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let result = send_custom(
            conn.cx(),
            "_goose/agent/provider_info",
            serde_json::json!({ "session_id": "nonexistent-session-id" }),
        )
        .await;
        assert!(
            result.is_err(),
            "expected error for nonexistent session, got: {:?}",
            result
        );
    });
}

#[test]
fn test_custom_agent_plan_prompt() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/agent/plan_prompt",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_ok(), "agent/plan_prompt failed: {:?}", result);

        let response = result.unwrap();
        assert!(
            response.get("plan_prompt").is_some(),
            "missing 'plan_prompt' field"
        );
        let plan_prompt = response
            .get("plan_prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(!plan_prompt.is_empty(), "plan_prompt should not be empty");
    });
}

#[test]
fn test_custom_session_clear() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let mut conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let (session, _models) = conn.new_session().await;
        let session_id = session.session_id().0.clone();

        let result = send_custom(
            conn.cx(),
            "_goose/session/clear",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(result.is_ok(), "session/clear failed: {:?}", result);

        // Verify the session still exists after clear
        let get_result = send_custom(
            conn.cx(),
            "_goose/session/get",
            serde_json::json!({ "session_id": session_id }),
        )
        .await;
        assert!(get_result.is_ok(), "session should still exist after clear");

        // Verify token counts are reset
        let session_data = get_result.unwrap();
        let session_obj = session_data.get("session").unwrap();
        let total_tokens = session_obj
            .get("total_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        assert_eq!(total_tokens, 0, "total_tokens should be 0 after clear");
    });
}

#[test]
fn test_custom_session_clear_invalid_session() {
    run_test(async {
        let openai = OpenAiFixture::new(vec![], ExpectedSessionId::default()).await;
        let conn = ClientToAgentConnection::new(TestConnectionConfig::default(), openai).await;

        let result = send_custom(
            conn.cx(),
            "_goose/session/clear",
            serde_json::json!({ "session_id": "nonexistent-session-id" }),
        )
        .await;
        assert!(
            result.is_err(),
            "expected error for clearing nonexistent session"
        );
    });
}
