//! A2A Spec Compliance Tests
//!
//! Verifies our Rust types serialize/deserialize correctly against the A2A v1.0 RC specification.
//! These are integration tests that validate wire-format compatibility.

use a2a::error::A2AError;
use a2a::jsonrpc::{methods, JsonRpcRequest, JsonRpcResponse, JSONRPC_VERSION};
use a2a::types::agent_card::*;
use a2a::types::config::*;
use a2a::types::core::*;
use a2a::types::events::*;
use a2a::types::requests::*;
use a2a::types::security::*;
use serde_json::json;

// ============================================================================
// TaskState: All 8 wire-format states (proto UNSPECIFIED excluded)
// ============================================================================

#[test]
fn spec_all_task_states_serialize() {
    let states = vec![
        (TaskState::Unspecified, "\"TASK_STATE_UNSPECIFIED\""),
        (TaskState::Submitted, "\"TASK_STATE_SUBMITTED\""),
        (TaskState::Working, "\"TASK_STATE_WORKING\""),
        (TaskState::Completed, "\"TASK_STATE_COMPLETED\""),
        (TaskState::Failed, "\"TASK_STATE_FAILED\""),
        (TaskState::Canceled, "\"TASK_STATE_CANCELED\""),
        (TaskState::InputRequired, "\"TASK_STATE_INPUT_REQUIRED\""),
        (TaskState::Rejected, "\"TASK_STATE_REJECTED\""),
        (TaskState::AuthRequired, "\"TASK_STATE_AUTH_REQUIRED\""),
    ];
    for (state, expected) in &states {
        let json = serde_json::to_string(state).unwrap();
        assert_eq!(
            &json, *expected,
            "TaskState serialization mismatch for {:?}",
            state
        );
    }
}

#[test]
fn spec_all_task_states_deserialize() {
    let cases = vec![
        ("\"TASK_STATE_UNSPECIFIED\"", TaskState::Unspecified),
        ("\"TASK_STATE_SUBMITTED\"", TaskState::Submitted),
        ("\"TASK_STATE_WORKING\"", TaskState::Working),
        ("\"TASK_STATE_COMPLETED\"", TaskState::Completed),
        ("\"TASK_STATE_FAILED\"", TaskState::Failed),
        ("\"TASK_STATE_CANCELED\"", TaskState::Canceled),
        ("\"TASK_STATE_INPUT_REQUIRED\"", TaskState::InputRequired),
        ("\"TASK_STATE_REJECTED\"", TaskState::Rejected),
        ("\"TASK_STATE_AUTH_REQUIRED\"", TaskState::AuthRequired),
    ];
    for (json, expected) in &cases {
        let state: TaskState = serde_json::from_str(json).unwrap();
        assert_eq!(
            &state, expected,
            "TaskState deserialization mismatch for {}",
            json
        );
    }
}

#[test]
fn spec_terminal_states() {
    assert!(TaskState::Completed.is_terminal());
    assert!(TaskState::Failed.is_terminal());
    assert!(TaskState::Canceled.is_terminal());
    assert!(TaskState::Rejected.is_terminal());
    assert!(!TaskState::Submitted.is_terminal());
    assert!(!TaskState::Working.is_terminal());
    assert!(!TaskState::InputRequired.is_terminal());
    assert!(!TaskState::AuthRequired.is_terminal());
}

#[test]
fn spec_interrupted_states() {
    assert!(TaskState::InputRequired.is_interrupted());
    assert!(TaskState::AuthRequired.is_interrupted());
    assert!(!TaskState::Working.is_interrupted());
    assert!(!TaskState::Completed.is_interrupted());
}

// ============================================================================
// Role: Two values per spec
// ============================================================================

#[test]
fn spec_role_values() {
    let user_json = serde_json::to_string(&Role::User).unwrap();
    let agent_json = serde_json::to_string(&Role::Agent).unwrap();
    assert_eq!(user_json, "\"ROLE_USER\"");
    assert_eq!(agent_json, "\"ROLE_AGENT\"");
}

// ============================================================================
// Message: spec wire format
// ============================================================================

#[test]
fn spec_message_minimal() {
    let json = r#"{
        "role": "ROLE_USER",
        "parts": [{"type": "text", "text": "Hello"}],
        "messageId": "msg-001"
    }"#;
    let msg: Message = serde_json::from_str(json).unwrap();
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.message_id, "msg-001");
    assert_eq!(msg.parts.len(), 1);
}

#[test]
fn spec_message_with_context_and_task() {
    let json = r#"{
        "role": "ROLE_AGENT",
        "parts": [{"type": "text", "text": "Done"}],
        "messageId": "msg-002",
        "contextId": "ctx-001",
        "taskId": "task-001",
        "extensions": ["ext-1"],
        "referenceTaskIds": ["task-001", "task-002"]
    }"#;
    let msg: Message = serde_json::from_str(json).unwrap();
    assert_eq!(msg.context_id.as_deref(), Some("ctx-001"));
    assert_eq!(msg.task_id.as_deref(), Some("task-001"));
    assert_eq!(msg.extensions.len(), 1);
    assert_eq!(msg.reference_task_ids.len(), 2);
}

// ============================================================================
// PartContent: all three content types per spec
// ============================================================================

#[test]
fn spec_part_text() {
    let json = r#"{"type": "text", "text": "hello world"}"#;
    let part: PartContent = serde_json::from_str(json).unwrap();
    match &part {
        PartContent::Text { text } => assert_eq!(text, "hello world"),
        _ => panic!("Expected Text variant"),
    }
}

#[test]
fn spec_part_file_with_url() {
    let json = r#"{"type": "file", "url": "https://example.com/doc.pdf"}"#;
    let part: PartContent = serde_json::from_str(json).unwrap();
    match &part {
        PartContent::File { url, .. } => {
            assert_eq!(url.as_deref(), Some("https://example.com/doc.pdf"));
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn spec_part_file_with_inline_data() {
    let json = r#"{"type": "file", "raw": "SGVsbG8="}"#;
    let part: PartContent = serde_json::from_str(json).unwrap();
    match &part {
        PartContent::File { raw, .. } => {
            assert_eq!(raw.as_deref(), Some("SGVsbG8="));
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn spec_part_data() {
    let json = r#"{"type": "data", "data": {"key": "value", "nested": {"a": 1}}}"#;
    let part: PartContent = serde_json::from_str(json).unwrap();
    match &part {
        PartContent::Data { data } => {
            assert_eq!(data.get("key").and_then(|v| v.as_str()), Some("value"));
        }
        _ => panic!("Expected Data variant"),
    }
}

#[test]
fn spec_part_with_metadata_and_media_type() {
    let part = Part {
        content: PartContent::Text {
            text: "hello".to_string(),
        },
        metadata: Some(serde_json::json!({"source": "user"})),
        filename: Some("notes.txt".to_string()),
        media_type: Some("text/plain".to_string()),
    };
    let json = serde_json::to_string(&part).unwrap();
    assert!(json.contains("\"filename\""));
    assert!(json.contains("\"mediaType\""));
    let roundtripped: Part = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.filename.as_deref(), Some("notes.txt"));
    assert_eq!(roundtripped.media_type.as_deref(), Some("text/plain"));
}

// ============================================================================
// Task: full wire format
// ============================================================================

#[test]
fn spec_task_full() {
    let json = r#"{
        "id": "task-001",
        "contextId": "ctx-001",
        "status": {
            "state": "TASK_STATE_WORKING",
            "timestamp": "2025-01-01T00:00:00Z"
        },
        "artifacts": [{
            "artifactId": "art-001",
            "parts": [{"type": "text", "text": "result"}],
            "name": "output"
        }],
        "history": [{
            "role": "ROLE_USER",
            "parts": [{"type": "text", "text": "do something"}],
            "messageId": "msg-001"
        }],
        "metadata": {"source": "test"}
    }"#;
    let task: Task = serde_json::from_str(json).unwrap();
    assert_eq!(task.id, "task-001");
    assert_eq!(task.context_id, "ctx-001");
    assert_eq!(task.status.state, TaskState::Working);
    assert_eq!(task.artifacts.len(), 1);
    assert_eq!(task.history.len(), 1);
    assert!(task.metadata.is_some());
}

#[test]
fn spec_task_minimal() {
    let json = r#"{
        "id": "task-002",
        "contextId": "ctx-002",
        "status": {"state": "TASK_STATE_SUBMITTED"}
    }"#;
    let task: Task = serde_json::from_str(json).unwrap();
    assert_eq!(task.id, "task-002");
    assert!(task.artifacts.is_empty());
    assert!(task.history.is_empty());
}

// ============================================================================
// Artifact: spec format
// ============================================================================

#[test]
fn spec_artifact_with_all_fields() {
    let json = r#"{
        "artifactId": "art-001",
        "parts": [{"type": "text", "text": "output data"}],
        "name": "result",
        "description": "The task output",
        "metadata": {"format": "markdown"},
        "extensions": ["ext-1"]
    }"#;
    let art: Artifact = serde_json::from_str(json).unwrap();
    assert_eq!(art.artifact_id, "art-001");
    assert_eq!(art.parts.len(), 1);
    assert_eq!(art.name.as_deref(), Some("result"));
    assert_eq!(art.description.as_deref(), Some("The task output"));
}

// ============================================================================
// SendMessageRequest: spec format
// ============================================================================

#[test]
fn spec_send_message_request() {
    let json = r#"{
        "message": {
            "role": "ROLE_USER",
            "parts": [{"type": "text", "text": "Analyze this code"}],
            "messageId": "msg-001"
        },
        "configuration": {
            "acceptedOutputModes": ["text", "file"],
            "blocking": false
        }
    }"#;
    let req: SendMessageRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.message.role, Role::User);
    assert!(req.configuration.is_some());
}

#[test]
fn spec_send_message_request_minimal() {
    let json = r#"{
        "message": {
            "role": "ROLE_USER",
            "parts": [{"type": "text", "text": "Hello"}],
            "messageId": "msg-003"
        }
    }"#;
    let req: SendMessageRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.message.message_id, "msg-003");
    assert!(req.configuration.is_none());
}

// ============================================================================
// JSON-RPC envelope: spec format
// ============================================================================

#[test]
fn spec_jsonrpc_request_envelope() {
    let json = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "SendMessage",
        "params": {
            "message": {
                "role": "ROLE_USER",
                "parts": [{"type": "text", "text": "test"}],
                "messageId": "msg-001"
            }
        }
    }"#;
    let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.jsonrpc, JSONRPC_VERSION);
    assert_eq!(req.method, methods::SEND_MESSAGE);
}

#[test]
fn spec_jsonrpc_response_with_result() {
    let json = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "id": "task-001",
            "contextId": "ctx-001",
            "status": {"state": "TASK_STATE_COMPLETED"}
        }
    }"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.jsonrpc, JSONRPC_VERSION);
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn spec_jsonrpc_error_response() {
    let json = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    }"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
}

// ============================================================================
// JSON-RPC method constants: match spec
// ============================================================================

#[test]
fn spec_method_constants() {
    assert_eq!(methods::SEND_MESSAGE, "SendMessage");
    assert_eq!(methods::SEND_STREAM, "SendStreamingMessage");
    assert_eq!(methods::GET_TASK, "GetTask");
    assert_eq!(methods::CANCEL_TASK, "CancelTask");
    assert_eq!(methods::LIST_TASKS, "ListTasks");
    assert_eq!(methods::SUBSCRIBE_TASK, "SubscribeToTask");
    assert_eq!(methods::SET_PUSH_CONFIG, "CreateTaskPushNotificationConfig");
    assert_eq!(methods::GET_PUSH_CONFIG, "GetTaskPushNotificationConfig");
    assert_eq!(
        methods::DELETE_PUSH_CONFIG,
        "DeleteTaskPushNotificationConfig"
    );
}

// ============================================================================
// A2AError: JSON-RPC error codes per spec
// ============================================================================

#[test]
fn spec_error_codes_standard_jsonrpc() {
    assert_eq!(A2AError::parse_error("").to_jsonrpc_error().code, -32700);
    assert_eq!(
        A2AError::invalid_request("").to_jsonrpc_error().code,
        -32600
    );
    assert_eq!(
        A2AError::method_not_found("").to_jsonrpc_error().code,
        -32601
    );
    assert_eq!(A2AError::invalid_params("").to_jsonrpc_error().code, -32602);
    assert_eq!(A2AError::internal_error("").to_jsonrpc_error().code, -32603);
}

#[test]
fn spec_error_codes_a2a_specific() {
    assert_eq!(
        A2AError::task_not_found("t1").to_jsonrpc_error().code,
        -32001
    );
    assert_eq!(
        A2AError::task_not_cancelable("t1").to_jsonrpc_error().code,
        -32002
    );
    assert_eq!(
        A2AError::PushNotificationNotSupported
            .to_jsonrpc_error()
            .code,
        -32003
    );
    assert_eq!(
        A2AError::unsupported_operation("").to_jsonrpc_error().code,
        -32004
    );
}

// ============================================================================
// AgentCard: camelCase wire format per spec
// ============================================================================

#[test]
fn spec_agent_card_camel_case_serialization() {
    let card = AgentCard {
        name: "test-agent".to_string(),
        description: "A test agent".to_string(),
        supported_interfaces: vec![AgentInterface {
            url: "https://example.com/a2a".to_string(),
            protocol_binding: Some("JSONRPC".to_string()),
            ..Default::default()
        }],
        version: Some("1.0.0".to_string()),
        protocol_version: Some("1.0".to_string()),
        capabilities: Some(AgentCapabilities {
            streaming: true,
            push_notifications: false,
            extensions: false,
            extended_agent_card: false,
        }),
        skills: vec![AgentSkill {
            id: "code-review".to_string(),
            name: "Code Review".to_string(),
            description: "Reviews code".to_string(),
            ..Default::default()
        }],
        default_input_modes: vec!["text".to_string()],
        default_output_modes: vec!["text".to_string()],
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&card).unwrap();
    assert!(
        json.contains("\"protocolVersion\""),
        "Missing camelCase protocolVersion"
    );
    assert!(
        json.contains("\"defaultInputModes\""),
        "Missing camelCase defaultInputModes"
    );
    assert!(
        json.contains("\"defaultOutputModes\""),
        "Missing camelCase defaultOutputModes"
    );
    assert!(
        json.contains("\"pushNotifications\""),
        "Missing camelCase pushNotifications"
    );
    assert!(
        json.contains("\"supportedInterfaces\""),
        "Missing camelCase supportedInterfaces"
    );
    assert!(
        !json.contains("\"protocol_version\""),
        "Should not have snake_case"
    );
}

#[test]
fn spec_agent_card_deserialize_from_discovery() {
    let json = r#"{
        "name": "Remote Agent",
        "description": "A remote A2A agent",
        "supportedInterfaces": [{
            "url": "https://remote.example.com/a2a",
            "protocolBinding": "JSONRPC"
        }],
        "version": "2.0.0",
        "protocolVersion": "1.0",
        "capabilities": {
            "streaming": true,
            "pushNotifications": true,
            "extensions": false,
            "extendedAgentCard": false
        },
        "skills": [{
            "id": "summarize",
            "name": "Summarize",
            "description": "Summarizes text",
            "tags": ["nlp", "text"]
        }],
        "defaultInputModes": ["text"],
        "defaultOutputModes": ["text", "file"],
        "provider": {
            "organization": "Test Corp",
            "url": "https://test.corp"
        }
    }"#;

    let card: AgentCard = serde_json::from_str(json).unwrap();
    assert_eq!(card.name, "Remote Agent");
    assert_eq!(card.protocol_version.as_deref(), Some("1.0"));
    let caps = card.capabilities.unwrap();
    assert!(caps.streaming);
    assert!(caps.push_notifications);
    assert_eq!(card.skills.len(), 1);
    assert_eq!(card.skills[0].id, "summarize");
    assert!(card.provider.is_some());
}

// ============================================================================
// SSE stream response format per spec
// ============================================================================

#[test]
fn spec_stream_response_status_update() {
    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "task-001".to_string(),
        context_id: "ctx-001".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: Some("2025-01-01T00:00:00Z".to_string()),
        },
        metadata: None,
    });

    let json = serde_json::to_string(&event).unwrap();
    assert!(
        json.contains("\"kind\":\"status-update\""),
        "Missing kind discriminator"
    );
    assert!(json.contains("\"taskId\""), "Missing camelCase taskId");
    assert!(
        json.contains("\"contextId\""),
        "Missing camelCase contextId"
    );
}

#[test]
fn spec_stream_response_artifact_update() {
    let event = StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
        task_id: "task-001".to_string(),
        context_id: "ctx-001".to_string(),
        artifact: Artifact {
            artifact_id: "art-001".to_string(),
            parts: vec![Part {
                content: PartContent::Text {
                    text: "result".to_string(),
                },
                metadata: None,
                filename: None,
                media_type: None,
            }],
            name: None,
            description: None,
            metadata: None,
            extensions: vec![],
        },
        append: false,
        last_chunk: false,
        metadata: None,
    });

    let json = serde_json::to_string(&event).unwrap();
    assert!(
        json.contains("\"kind\":\"artifact-update\""),
        "Missing kind discriminator"
    );
    assert!(
        json.contains("\"artifactId\""),
        "Missing camelCase artifactId"
    );
}

// ============================================================================
// SecurityScheme: key variants per spec
// ============================================================================

#[test]
fn spec_security_scheme_api_key() {
    let json = r#"{"type": "apiKey", "name": "X-API-Key", "location": "header"}"#;
    let scheme: SecurityScheme = serde_json::from_str(json).unwrap();
    match scheme {
        SecurityScheme::ApiKey(ref s) => {
            assert_eq!(s.name, "X-API-Key");
            assert_eq!(s.location, "header");
        }
        _ => panic!("Expected ApiKey"),
    }
}

#[test]
fn spec_security_scheme_http_bearer() {
    let json = r#"{"type": "http", "scheme": "bearer", "bearerFormat": "JWT"}"#;
    let scheme: SecurityScheme = serde_json::from_str(json).unwrap();
    match scheme {
        SecurityScheme::Http(ref s) => {
            assert_eq!(s.scheme, "bearer");
            assert_eq!(s.bearer_format.as_deref(), Some("JWT"));
        }
        _ => panic!("Expected Http"),
    }
}

#[test]
fn spec_security_scheme_oauth2() {
    let json = r#"{
        "type": "oauth2",
        "flows": {
            "clientCredentials": {
                "tokenUrl": "https://auth.example.com/token",
                "scopes": {"read": "Read access", "write": "Write access"}
            }
        }
    }"#;
    let scheme: SecurityScheme = serde_json::from_str(json).unwrap();
    match scheme {
        SecurityScheme::OAuth2(_) => {}
        _ => panic!("Expected OAuth2"),
    }
}

// ============================================================================
// PushNotificationConfig: spec format
// ============================================================================

#[test]
fn spec_push_notification_config() {
    let json = r#"{
        "url": "https://example.com/webhook",
        "token": "secret-token",
        "authentication": {
            "scheme": "bearer",
            "credentials": "my-jwt-token"
        }
    }"#;
    let config: PushNotificationConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.url, "https://example.com/webhook");
    assert!(config.authentication.is_some());
}

// ============================================================================
// CamelCase consistency: request types use "id"
// ============================================================================

#[test]
fn spec_request_types_field_names() {
    let json = r#"{"id": "t1"}"#;
    let req: GetTaskRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.id, "t1");

    let json = r#"{"id": "t1"}"#;
    let req: CancelTaskRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.id, "t1");

    let json = r#"{"contextId": "c1"}"#;
    let req: ListTasksRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.context_id.as_deref(), Some("c1"));
}

// ============================================================================
// Roundtrip: serialize then deserialize preserves values
// ============================================================================

#[test]
fn spec_task_roundtrip() {
    let task = Task {
        id: "task-rt".to_string(),
        context_id: "ctx-rt".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: Some(Box::new(Message {
                message_id: "status-msg".to_string(),
                context_id: None,
                task_id: None,
                role: Role::Agent,
                parts: vec![Part {
                    content: PartContent::Text {
                        text: "Processing...".to_string(),
                    },
                    metadata: None,
                    filename: None,
                    media_type: None,
                }],
                extensions: vec![],
                reference_task_ids: vec![],
                metadata: None,
            })),
            timestamp: Some("2025-01-01T12:00:00Z".to_string()),
        },
        artifacts: vec![Artifact {
            artifact_id: "a1".to_string(),
            parts: vec![Part {
                content: PartContent::Text {
                    text: "partial result".to_string(),
                },
                metadata: None,
                filename: None,
                media_type: None,
            }],
            name: Some("output".to_string()),
            description: None,
            metadata: None,
            extensions: vec![],
        }],
        history: vec![],
        metadata: None,
    };

    let json = serde_json::to_string(&task).unwrap();
    let roundtripped: Task = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.id, task.id);
    assert_eq!(roundtripped.context_id, task.context_id);
    assert_eq!(roundtripped.status.state, task.status.state);
    assert_eq!(roundtripped.artifacts.len(), 1);
    assert_eq!(roundtripped.artifacts[0].artifact_id, "a1");
}

#[test]
fn spec_agent_card_roundtrip() {
    let card = AgentCard {
        name: "roundtrip-agent".to_string(),
        description: "Tests roundtrip".to_string(),
        supported_interfaces: vec![AgentInterface {
            url: "https://example.com".to_string(),
            protocol_binding: Some("JSONRPC".to_string()),
            ..Default::default()
        }],
        version: Some("1.0".to_string()),
        protocol_version: Some("1.0".to_string()),
        capabilities: Some(AgentCapabilities {
            streaming: true,
            push_notifications: true,
            extensions: false,
            extended_agent_card: false,
        }),
        skills: vec![AgentSkill {
            id: "s1".to_string(),
            name: "Skill One".to_string(),
            description: "Does things".to_string(),
            tags: vec!["tag1".to_string()],
            examples: vec!["example1".to_string()],
            ..Default::default()
        }],
        default_input_modes: vec!["text".to_string()],
        default_output_modes: vec!["text".to_string(), "file".to_string()],
        provider: Some(AgentProvider {
            organization: "Test Org".to_string(),
            url: Some("https://test.org".to_string()),
        }),
        ..Default::default()
    };

    let json = serde_json::to_string(&card).unwrap();
    let roundtripped: AgentCard = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.name, card.name);
    let caps = roundtripped.capabilities.unwrap();
    assert!(caps.streaming);
    assert_eq!(roundtripped.skills.len(), 1);
    assert_eq!(roundtripped.default_output_modes.len(), 2);
    assert!(roundtripped.provider.is_some());
}

/// Legacy lowercase enum values must be accepted for backward compatibility
/// with existing clients (e.g., a2a-js v0.3.x which uses non-normative JSON Schema types).
#[test]
fn spec_legacy_task_state_compat() {
    // Legacy lowercase
    let working: TaskState = serde_json::from_value(json!("working")).unwrap();
    assert_eq!(working, TaskState::Working);
    let input_req: TaskState = serde_json::from_value(json!("input-required")).unwrap();
    assert_eq!(input_req, TaskState::InputRequired);
    let completed: TaskState = serde_json::from_value(json!("completed")).unwrap();
    assert_eq!(completed, TaskState::Completed);

    // But serialization always uses ProtoJSON
    assert_eq!(
        serde_json::to_value(working).unwrap(),
        json!("TASK_STATE_WORKING")
    );
}

#[test]
fn spec_legacy_role_compat() {
    let user: Role = serde_json::from_value(json!("user")).unwrap();
    assert_eq!(user, Role::User);
    let agent: Role = serde_json::from_value(json!("agent")).unwrap();
    assert_eq!(agent, Role::Agent);

    // Serialization always ProtoJSON
    assert_eq!(serde_json::to_value(user).unwrap(), json!("ROLE_USER"));
}

#[test]
fn spec_legacy_full_message_compat() {
    // A message in JS SDK format should deserialize correctly
    let js_sdk_msg = json!({
        "messageId": "msg-from-js",
        "role": "user",
        "parts": [{"type": "text", "text": "Hello from JS SDK"}]
    });
    let msg: Message = serde_json::from_value(js_sdk_msg).unwrap();
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.parts.len(), 1);

    // Re-serialized to ProtoJSON format
    let re_serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(re_serialized["role"], "ROLE_USER");
}

#[test]
fn spec_legacy_task_compat() {
    // A task in JS SDK format
    let js_sdk_task = json!({
        "id": "task-js",
        "contextId": "ctx-js",
        "status": {
            "state": "working",
            "timestamp": "2025-10-28T10:30:00.000Z"
        },
        "history": [{
            "messageId": "msg-1",
            "role": "user",
            "parts": [{"type": "text", "text": "do something"}]
        }]
    });
    let task: Task = serde_json::from_value(js_sdk_task).unwrap();
    assert_eq!(task.status.state, TaskState::Working);
    assert_eq!(task.history[0].role, Role::User);

    // Re-serialized to ProtoJSON
    let re_serialized = serde_json::to_value(&task).unwrap();
    assert_eq!(re_serialized["status"]["state"], "TASK_STATE_WORKING");
    assert_eq!(re_serialized["history"][0]["role"], "ROLE_USER");
}
