use goose::providers::litellm::update_request_for_cache_control;
use serde_json::json;

// This test targets: update_request_for_cache_control
// Goal: ensure it adds ephemeral cache_control to the last two user messages,
// the system message, and to the last tool's function spec.
#[test]
fn test_update_request_for_cache_control_applies_ephemeral_to_user_system_and_last_tool() {
    let payload = json!({
        "messages": [
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there"},
            {"role": "user", "content": "How are you?"},
            {"role": "system", "content": "You are a helpful assistant."}
        ],
        "tools": [
            {"type": "function", "function": {"name": "tool1", "description": "first"}},
            {"type": "function", "function": {"name": "tool2", "description": "second"}}
        ]
    });

    let updated = update_request_for_cache_control(&payload);

    // Verify user messages (last two user messages should be wrapped with cache_control)
    let messages = updated.get("messages").unwrap().as_array().unwrap();

    // message[0] - user("Hello") should have been wrapped
    let m0_content = messages[0].get("content").unwrap().as_array().unwrap();
    assert_eq!(m0_content.len(), 1);
    assert_eq!(m0_content[0]["type"], "text");
    assert_eq!(m0_content[0]["text"], "Hello");
    assert_eq!(m0_content[0]["cache_control"]["type"], "ephemeral");

    // message[1] - assistant should remain as a plain string
    assert_eq!(messages[1]["role"], "assistant");
    assert!(messages[1].get("content").unwrap().is_string());

    // message[2] - user("How are you?") should have been wrapped
    let m2_content = messages[2].get("content").unwrap().as_array().unwrap();
    assert_eq!(m2_content.len(), 1);
    assert_eq!(m2_content[0]["type"], "text");
    assert_eq!(m2_content[0]["text"], "How are you?");
    assert_eq!(m2_content[0]["cache_control"]["type"], "ephemeral");

    // message[3] - system message should be rewritten to the structured format with cache_control
    assert_eq!(messages[3]["role"], "system");
    let sys_content = messages[3].get("content").unwrap().as_array().unwrap();
    assert_eq!(sys_content.len(), 1);
    assert_eq!(sys_content[0]["type"], "text");
    assert_eq!(sys_content[0]["text"], "You are a helpful assistant.");
    assert_eq!(sys_content[0]["cache_control"]["type"], "ephemeral");

    // Verify tools: only the last tool gets cache_control inserted
    let tools = updated.get("tools").unwrap().as_array().unwrap();

    // First tool should NOT have cache_control
    assert!(tools[0]["function"].get("cache_control").is_none());

    // Last tool should have cache_control { type: "ephemeral" }
    assert_eq!(tools[1]["function"]["cache_control"]["type"], "ephemeral");
}
