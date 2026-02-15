use super::types::*;
use super::utils::{generate_secret, process_sse_buffer};
use goose::agents::{AgentEvent, ExtensionConfig};
use tokio::sync::mpsc;

fn sample_message_json() -> &'static str {
    r#"{"role":"assistant","created":0,"content":[{"type":"text","text":"Hello"}],"metadata":{"userVisible":true,"agentVisible":true}}"#
}

fn sample_token_state_json() -> &'static str {
    r#"{"inputTokens":0,"outputTokens":0,"totalTokens":0,"accumulatedInputTokens":0,"accumulatedOutputTokens":0,"accumulatedTotalTokens":0}"#
}

fn make_sse_json(type_name: &str, extra: &str) -> String {
    match type_name {
        "Message" => format!(
            r#"{{"type":"Message","message":{},"token_state":{}}}"#,
            sample_message_json(),
            sample_token_state_json()
        ),
        "Finish" => format!(
            r#"{{"type":"Finish","reason":"stop","token_state":{}}}"#,
            sample_token_state_json()
        ),
        _ => extra.to_string(),
    }
}

#[test]
fn test_parse_sse_message_event() {
    let json = make_sse_json("Message", "");
    let event: SseEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(event, SseEvent::Message { .. }));
}

#[test]
fn test_parse_sse_finish_event() {
    let json = make_sse_json("Finish", "");
    let event: SseEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(event, SseEvent::Finish { .. }));
}

#[test]
fn test_parse_sse_error_event() {
    let json = r#"{"type":"Error","error":"something went wrong"}"#;
    let event: SseEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(event, SseEvent::Error { .. }));
}

#[test]
fn test_parse_sse_ping_event() {
    let json = r#"{"type":"Ping"}"#;
    let event: SseEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(event, SseEvent::Ping));
}

#[test]
fn test_parse_sse_model_change() {
    let json = r#"{"type":"ModelChange","model":"gpt-4","mode":"chat"}"#;
    let event: SseEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(event, SseEvent::ModelChange { .. }));
}

#[test]
fn test_parse_sse_routing_decision() {
    let json = r#"{"type":"RoutingDecision","agent_name":"Goose Agent","mode_slug":"chat","confidence":0.95,"reasoning":"test"}"#;
    let event: SseEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(event, SseEvent::RoutingDecision { .. }));
}

#[test]
fn test_parse_sse_tool_availability_change() {
    let json = r#"{"type":"ToolAvailabilityChange","previous_count":5,"current_count":3}"#;
    let event: SseEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(event, SseEvent::ToolAvailabilityChange { .. }));
}

#[test]
fn test_parse_sse_notification() {
    let json = r#"{"type":"Notification","request_id":"abc","message":{"key":"val"}}"#;
    let event: SseEvent = serde_json::from_str(json).unwrap();
    assert!(matches!(event, SseEvent::Notification { .. }));
}

#[test]
fn test_parse_sse_update_conversation() {
    let json = format!(
        r#"{{"type":"UpdateConversation","conversation":[{}]}}"#,
        sample_message_json()
    );
    let event: SseEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(event, SseEvent::UpdateConversation { .. }));
}

#[test]
fn test_generate_secret() {
    let s = generate_secret();
    assert!(s.starts_with("cli-"));
    assert!(s.len() > 10);
}

#[test]
fn test_sse_event_to_agent_event_message() {
    let json = make_sse_json("Message", "");
    let sse: SseEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        sse.into_agent_event(),
        Some(AgentEvent::Message(_))
    ));
}

#[test]
fn test_sse_event_to_agent_event_finish_is_none() {
    let json = make_sse_json("Finish", "");
    let sse: SseEvent = serde_json::from_str(&json).unwrap();
    assert!(sse.into_agent_event().is_none());
}

#[test]
fn test_serialize_add_extension_request() {
    let req = AddExtensionRequest {
        session_id: "sess-1".to_string(),
        config: ExtensionConfig::Sse {
            name: "test-ext".to_string(),
            description: "Test extension".to_string(),
            uri: Some("http://localhost:3000/sse".to_string()),
        },
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("sess-1"));
    assert!(json.contains("http://localhost:3000/sse"));
}

#[test]
fn test_serialize_remove_extension_request() {
    let req = RemoveExtensionRequest {
        name: "developer".to_string(),
        session_id: "sess-2".to_string(),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("developer"));
    assert!(json.contains("sess-2"));
}

#[test]
fn test_serialize_update_provider_request() {
    let req = UpdateProviderRequest {
        provider: "openai".to_string(),
        model: Some("gpt-4".to_string()),
        session_id: "sess-3".to_string(),
        context_limit: Some(128000),
        request_params: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("openai"));
    assert!(json.contains("gpt-4"));
    assert!(json.contains("128000"));
    assert!(json.contains("sess-3"));
}

#[test]
fn test_serialize_fork_session_request() {
    let req = ForkSessionRequest {
        timestamp: Some(1234567890),
        truncate: true,
        copy: false,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("1234567890"));
    assert!(json.contains("true"));
}

#[test]
fn test_deserialize_tool_info_response() {
    let json = r#"{"name":"bash","description":"Run a command","parameters":["command"],"permission":"AlwaysAllow"}"#;
    let info: ToolInfoResponse = serde_json::from_str(json).unwrap();
    assert_eq!(info.name, "bash");
    assert_eq!(info.description, "Run a command");
    assert_eq!(info.parameters, vec!["command"]);
    assert_eq!(info.permission, Some("AlwaysAllow".to_string()));
}

#[test]
fn test_deserialize_extension_load_result_response() {
    let json = r#"{"name":"developer","success":true,"error":null}"#;
    let result: ExtensionLoadResultResponse = serde_json::from_str(json).unwrap();
    assert_eq!(result.name, "developer");
    assert!(result.success);
    assert!(result.error.is_none());
}

#[test]
fn test_deserialize_get_prompt_result_response() {
    let json = r#"{"description":"A test prompt","messages":[{"role":"user","content":{"type":"text","text":"hello"}}]}"#;
    let result: GetPromptResultResponse = serde_json::from_str(json).unwrap();
    assert_eq!(result.description, Some("A test prompt".to_string()));
    assert_eq!(result.messages.len(), 1);
}

#[test]
fn test_deserialize_get_prompt_result_response_no_description() {
    let json = r#"{"messages":[{"role":"user","content":{"type":"text","text":"hello"}},{"role":"assistant","content":{"type":"text","text":"hi"}}]}"#;
    let result: GetPromptResultResponse = serde_json::from_str(json).unwrap();
    assert!(result.description.is_none());
    assert_eq!(result.messages.len(), 2);
}

// ── process_sse_buffer tests ─────────────────────────────────────────

#[tokio::test]
async fn test_process_sse_buffer_single_event() {
    let (tx, mut rx) = mpsc::channel(10);
    let mut buffer = "data: {\"type\":\"Ping\"}\n\n".to_string();
    process_sse_buffer(&mut buffer, &tx).await;
    drop(tx);

    let event = rx.recv().await.unwrap().unwrap();
    assert!(matches!(event, SseEvent::Ping));
    assert!(rx.recv().await.is_none());
    assert!(buffer.is_empty());
}

#[tokio::test]
async fn test_process_sse_buffer_multiple_events() {
    let (tx, mut rx) = mpsc::channel(10);
    let mut buffer = format!(
        "data: {}\n\ndata: {}\n\n",
        r#"{"type":"Ping"}"#,
        make_sse_json("Finish", "")
    );
    process_sse_buffer(&mut buffer, &tx).await;
    drop(tx);

    let e1 = rx.recv().await.unwrap().unwrap();
    assert!(matches!(e1, SseEvent::Ping));
    let e2 = rx.recv().await.unwrap().unwrap();
    assert!(matches!(e2, SseEvent::Finish { .. }));
}

#[tokio::test]
async fn test_process_sse_buffer_partial_event_stays_in_buffer() {
    let (tx, mut rx) = mpsc::channel(10);
    let mut buffer = "data: {\"type\":\"Ping\"}".to_string(); // no \n\n
    process_sse_buffer(&mut buffer, &tx).await;
    drop(tx);

    assert!(rx.recv().await.is_none(), "No events should be emitted");
    assert!(!buffer.is_empty(), "Partial data should remain in buffer");
}

#[tokio::test]
async fn test_process_sse_buffer_ignores_malformed_json() {
    let (tx, mut rx) = mpsc::channel(10);
    let mut buffer = "data: {not valid json}\n\ndata: {\"type\":\"Ping\"}\n\n".to_string();
    process_sse_buffer(&mut buffer, &tx).await;
    drop(tx);

    // Malformed event is skipped, valid one emitted
    let event = rx.recv().await.unwrap().unwrap();
    assert!(matches!(event, SseEvent::Ping));
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn test_process_sse_buffer_ignores_empty_data_lines() {
    let (tx, mut rx) = mpsc::channel(10);
    let mut buffer = "data: \n\n".to_string();
    process_sse_buffer(&mut buffer, &tx).await;
    drop(tx);

    assert!(
        rx.recv().await.is_none(),
        "Empty data lines should be skipped"
    );
}

#[tokio::test]
async fn test_process_sse_buffer_ignores_non_data_lines() {
    let (tx, mut rx) = mpsc::channel(10);
    let mut buffer = "event: message\nid: 42\ndata: {\"type\":\"Ping\"}\n\n".to_string();
    process_sse_buffer(&mut buffer, &tx).await;
    drop(tx);

    let event = rx.recv().await.unwrap().unwrap();
    assert!(matches!(event, SseEvent::Ping));
}
