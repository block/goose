use goose::conversation::message::Message;
use goose_tui::services::config::TuiConfig;
use goose_tui::state::action::Action;
use goose_tui::state::reducer::{extract_todos_from_message, update};
use goose_tui::state::{
    ActivePopup, AppState, CwdAnalysisState, InputMode, PendingToolConfirmation,
};
use rmcp::model::CallToolRequestParam;
use serde_json::json;

fn test_state() -> AppState {
    let config = TuiConfig::load().unwrap_or_else(|_| TuiConfig {
        theme: goose_tui::utils::styles::Theme::default(),
        custom_commands: Vec::new(),
        smart_context: true,
    });
    AppState::new("test-session".to_string(), config, None, None)
}

fn make_todo_tool_request(content: &str) -> Message {
    Message::assistant().with_tool_request(
        "tool_123",
        Ok(CallToolRequestParam {
            name: "todo__todo_write".into(),
            arguments: Some(json!({"content": content}).as_object().unwrap().clone()),
        }),
    )
}

fn make_shell_tool_request(command: &str) -> Message {
    Message::assistant().with_tool_request(
        "tool_456",
        Ok(CallToolRequestParam {
            name: "developer__shell".into(),
            arguments: Some(json!({"command": command}).as_object().unwrap().clone()),
        }),
    )
}

#[test]
fn toggle_input_mode() {
    let mut state = test_state();
    assert_eq!(state.input_mode, InputMode::Editing);

    update(&mut state, Action::ToggleInputMode);
    assert_eq!(state.input_mode, InputMode::Normal);

    update(&mut state, Action::ToggleInputMode);
    assert_eq!(state.input_mode, InputMode::Editing);
}

#[test]
fn toggle_help_popup() {
    let mut state = test_state();
    assert_eq!(state.active_popup, ActivePopup::None);

    update(&mut state, Action::ToggleHelp);
    assert_eq!(state.active_popup, ActivePopup::Help);

    update(&mut state, Action::ToggleHelp);
    assert_eq!(state.active_popup, ActivePopup::None);
}

#[test]
fn toggle_todo_popup() {
    let mut state = test_state();

    update(&mut state, Action::ToggleTodo);
    assert_eq!(state.active_popup, ActivePopup::Todo);

    update(&mut state, Action::ToggleTodo);
    assert_eq!(state.active_popup, ActivePopup::None);
}

#[test]
fn close_popup_clears_any_popup() {
    let mut state = test_state();

    update(&mut state, Action::OpenConfig);
    assert_eq!(state.active_popup, ActivePopup::Config(0));

    update(&mut state, Action::ClosePopup);
    assert_eq!(state.active_popup, ActivePopup::None);
}

#[test]
fn open_message_info_stores_index() {
    let mut state = test_state();

    update(&mut state, Action::OpenMessageInfo(5));
    assert_eq!(state.active_popup, ActivePopup::MessageInfo(5));
}

#[test]
fn clear_chat_resets_state() {
    let mut state = test_state();
    state.has_worked = true;

    update(&mut state, Action::ClearChat);

    assert!(state.messages.is_empty());
    assert!(state.todos.is_empty());
    assert!(!state.has_worked);
}

#[test]
fn interrupt_stops_working() {
    let mut state = test_state();
    state.is_working = true;

    update(&mut state, Action::Interrupt);

    assert!(!state.is_working);
    assert!(state.flash_message.is_some());
}

#[test]
fn toggle_copy_mode() {
    let mut state = test_state();
    assert!(!state.copy_mode);

    update(&mut state, Action::ToggleCopyMode);
    assert!(state.copy_mode);

    update(&mut state, Action::ToggleCopyMode);
    assert!(!state.copy_mode);
}

#[test]
fn extract_todos_parses_done_and_undone() {
    let content = "- [ ] Task 1\n- [x] Task 2\n- [ ] Task 3";
    let message = make_todo_tool_request(content);

    let todos = extract_todos_from_message(&message).expect("should extract todos");

    assert_eq!(todos.len(), 3);
    assert_eq!(todos[0].text, "Task 1");
    assert!(!todos[0].done);
    assert_eq!(todos[1].text, "Task 2");
    assert!(todos[1].done);
    assert_eq!(todos[2].text, "Task 3");
    assert!(!todos[2].done);
}

#[test]
fn extract_todos_ignores_non_todo_tool() {
    let message = make_shell_tool_request("ls -la");
    assert!(extract_todos_from_message(&message).is_none());
}

#[test]
fn extract_todos_handles_empty_content() {
    let message = make_todo_tool_request("");
    assert!(extract_todos_from_message(&message).is_none());
}

#[test]
fn extract_todos_rejects_malformed_checkboxes() {
    let content = "- [] Task without space\n-[ ] Task without dash space\n- [X] Uppercase X";
    let message = make_todo_tool_request(content);
    assert!(extract_todos_from_message(&message).is_none());
}

#[test]
fn extract_todos_handles_indented_items() {
    let content = "  - [ ] Indented task\n    - [x] Deeply indented";
    let message = make_todo_tool_request(content);

    let todos = extract_todos_from_message(&message).expect("should extract todos");

    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].text, "Indented task");
    assert_eq!(todos[1].text, "Deeply indented");
}

#[test]
fn confirm_tool_call_clears_pending_on_match() {
    let mut state = test_state();
    state.pending_confirmation = Some(PendingToolConfirmation {
        id: "tool_abc".to_string(),
        tool_name: "dangerous_tool".to_string(),
        arguments: serde_json::Map::new(),
        security_warning: Some("This is dangerous".to_string()),
        message_index: 0,
    });
    state.is_working = false;

    update(
        &mut state,
        Action::ConfirmToolCall {
            id: "tool_abc".to_string(),
            approved: true,
        },
    );

    assert!(state.pending_confirmation.is_none());
    assert!(state.is_working);
    assert!(state.flash_message.unwrap().0.contains("allowed"));
}

#[test]
fn confirm_tool_call_ignores_mismatched_id() {
    let mut state = test_state();
    state.pending_confirmation = Some(PendingToolConfirmation {
        id: "tool_abc".to_string(),
        tool_name: "dangerous_tool".to_string(),
        arguments: serde_json::Map::new(),
        security_warning: None,
        message_index: 0,
    });

    update(
        &mut state,
        Action::ConfirmToolCall {
            id: "wrong_id".to_string(),
            approved: true,
        },
    );

    assert!(state.pending_confirmation.is_some());
}

#[test]
fn confirm_tool_call_noop_when_no_pending() {
    let mut state = test_state();

    update(
        &mut state,
        Action::ConfirmToolCall {
            id: "any_id".to_string(),
            approved: false,
        },
    );

    assert!(state.pending_confirmation.is_none());
}

#[test]
fn confirm_tool_call_denied_sets_flash() {
    let mut state = test_state();
    state.pending_confirmation = Some(PendingToolConfirmation {
        id: "tool_xyz".to_string(),
        tool_name: "risky_tool".to_string(),
        arguments: serde_json::Map::new(),
        security_warning: None,
        message_index: 0,
    });

    update(
        &mut state,
        Action::ConfirmToolCall {
            id: "tool_xyz".to_string(),
            approved: false,
        },
    );

    assert!(state.pending_confirmation.is_none());
    assert!(state.flash_message.unwrap().0.contains("denied"));
}

#[test]
fn cwd_analysis_state_take_result_returns_complete_data() {
    let mut state = CwdAnalysisState::Complete("analysis data".to_string());
    assert_eq!(state.take_result(), Some("analysis data".to_string()));
    assert!(matches!(state, CwdAnalysisState::NotStarted));
}

#[test]
fn cwd_analysis_state_take_result_preserves_other_states() {
    let mut pending = CwdAnalysisState::Pending;
    assert!(pending.take_result().is_none());
    assert!(matches!(pending, CwdAnalysisState::Pending));

    let mut failed = CwdAnalysisState::Failed;
    assert!(failed.take_result().is_none());
    assert!(matches!(failed, CwdAnalysisState::Failed));
}

#[test]
fn cwd_analysis_state_is_pending() {
    assert!(CwdAnalysisState::Pending.is_pending());
    assert!(!CwdAnalysisState::NotStarted.is_pending());
    assert!(!CwdAnalysisState::Complete("x".to_string()).is_pending());
    assert!(!CwdAnalysisState::Failed.is_pending());
}

fn make_session_with_messages(id: &str, messages: Vec<Message>) -> Box<goose::session::Session> {
    use goose::conversation::Conversation;
    use goose::session::{extension_data::ExtensionData, Session, SessionType};

    let conversation = Conversation::new_unvalidated(messages.clone());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Box::new(Session {
        id: id.to_string(),
        working_dir: std::path::PathBuf::from("/tmp/test"),
        name: "test".to_string(),
        user_set_name: false,
        session_type: SessionType::User,
        created_at: chrono::DateTime::from_timestamp(now, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(now, 0).unwrap(),
        extension_data: ExtensionData::default(),
        total_tokens: Some(0),
        input_tokens: Some(0),
        output_tokens: Some(0),
        accumulated_total_tokens: Some(0),
        accumulated_input_tokens: Some(0),
        accumulated_output_tokens: Some(0),
        schedule_id: None,
        recipe: None,
        user_recipe_values: None,
        conversation: Some(conversation),
        message_count: messages.len(),
        provider_name: None,
        model_config: None,
    })
}

fn make_text_message_event(
    id: &str,
    text: &str,
) -> std::sync::Arc<goose_server::routes::reply::MessageEvent> {
    let message = Message::assistant().with_id(id).with_text(text);

    std::sync::Arc::new(goose_server::routes::reply::MessageEvent::Message {
        message,
        token_state: goose::conversation::message::TokenState::default(),
    })
}



#[test]
fn session_resumed_new_session_with_messages_fails_cwd_analysis() {
    let mut state = test_state();
    state.session_id = "old-session".to_string();
    state.cwd_analysis = CwdAnalysisState::Complete("some context".to_string());

    let messages = vec![Message::user().with_text("hello")];
    let session = make_session_with_messages("new-session", messages);

    update(&mut state, Action::SessionResumed(session));

    assert_eq!(state.session_id, "new-session");
    assert!(matches!(state.cwd_analysis, CwdAnalysisState::Failed));
}

#[test]
fn session_resumed_same_session_preserves_cwd_analysis() {
    let mut state = test_state();
    state.session_id = "same-session".to_string();
    state.cwd_analysis = CwdAnalysisState::Complete("context".to_string());

    let messages = vec![Message::user().with_text("hello")];
    let session = make_session_with_messages("same-session", messages);

    update(&mut state, Action::SessionResumed(session));

    assert!(matches!(state.cwd_analysis, CwdAnalysisState::Complete(_)));
}

#[test]
fn session_resumed_clears_pending_confirmation() {
    let mut state = test_state();
    state.session_id = "old-session".to_string();
    state.pending_confirmation = Some(PendingToolConfirmation {
        id: "old_tool".to_string(),
        tool_name: "shell".to_string(),
        arguments: serde_json::Map::new(),
        security_warning: None,
        message_index: 0,
    });

    let session = make_session_with_messages("new-session", vec![]);

    update(&mut state, Action::SessionResumed(session));

    assert!(state.pending_confirmation.is_none());
}

#[test]
fn server_message_streaming_concatenates_text() {
    let mut state = test_state();

    let event1 = make_text_message_event("msg_1", "Hello ");
    update(&mut state, Action::ServerMessage(event1));

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].as_concat_text(), "Hello ");

    let event2 = make_text_message_event("msg_1", "world");
    update(&mut state, Action::ServerMessage(event2));

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].as_concat_text(), "Hello world");
}

#[test]
fn server_message_new_id_creates_new_message() {
    let mut state = test_state();

    let event1 = make_text_message_event("msg_1", "First");
    update(&mut state, Action::ServerMessage(event1));

    let event2 = make_text_message_event("msg_2", "Second");
    update(&mut state, Action::ServerMessage(event2));

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].as_concat_text(), "First");
    assert_eq!(state.messages[1].as_concat_text(), "Second");
}

#[test]
fn server_message_mixed_content_appends_correctly() {
    let mut state = test_state();

    let event1 = make_text_message_event("msg_1", "Some text");
    update(&mut state, Action::ServerMessage(event1));

    let tool_msg = Message::assistant().with_id("msg_1").with_tool_request(
        "req_1",
        Ok(CallToolRequestParam {
            name: "shell".into(),
            arguments: Some(json!({"command": "ls"}).as_object().unwrap().clone()),
        }),
    );

    let event2 = std::sync::Arc::new(goose_server::routes::reply::MessageEvent::Message {
        message: tool_msg,
        token_state: goose::conversation::message::TokenState::default(),
    });
    update(&mut state, Action::ServerMessage(event2));

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].content.len(), 2);
}

#[test]
fn server_message_empty_state_creates_first_message() {
    let mut state = test_state();
    assert!(state.messages.is_empty());

    let event = make_text_message_event("msg_1", "First message");
    update(&mut state, Action::ServerMessage(event));

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].as_concat_text(), "First message");
}
