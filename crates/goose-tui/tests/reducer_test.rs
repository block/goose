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

// ============================================================================
// extract_todos_from_message tests
// ============================================================================

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

    let todos = extract_todos_from_message(&message);

    assert!(todos.is_none());
}

#[test]
fn extract_todos_handles_empty_content() {
    let message = make_todo_tool_request("");

    let todos = extract_todos_from_message(&message);

    assert!(todos.is_none());
}

#[test]
fn extract_todos_handles_malformed_checkboxes() {
    // These should NOT be parsed as todos
    let content = "- [] Task without space\n-[ ] Task without dash space\n- [X] Uppercase X";
    let message = make_todo_tool_request(content);

    let todos = extract_todos_from_message(&message);

    // None of these match the expected format "- [ ] " or "- [x] "
    assert!(todos.is_none());
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

// ============================================================================
// ConfirmToolCall tests
// ============================================================================

#[test]
fn confirm_tool_call_matches_pending_id() {
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
    assert!(state.flash_message.is_some());
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

    // Should not clear pending since ID doesn't match
    assert!(state.pending_confirmation.is_some());
}

#[test]
fn confirm_tool_call_noop_when_no_pending() {
    let mut state = test_state();
    assert!(state.pending_confirmation.is_none());

    // Should not panic
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
    assert!(state.flash_message.is_some());
    assert!(state.flash_message.unwrap().0.contains("denied"));
}

// ============================================================================
// CwdAnalysisState tests
// ============================================================================

#[test]
fn cwd_analysis_state_take_result_complete() {
    let mut state = CwdAnalysisState::Complete("analysis data".to_string());

    let result = state.take_result();

    assert_eq!(result, Some("analysis data".to_string()));
    assert!(matches!(state, CwdAnalysisState::NotStarted));
}

#[test]
fn cwd_analysis_state_take_result_pending() {
    let mut state = CwdAnalysisState::Pending;

    let result = state.take_result();

    assert!(result.is_none());
    assert!(matches!(state, CwdAnalysisState::Pending));
}

#[test]
fn cwd_analysis_state_take_result_failed() {
    let mut state = CwdAnalysisState::Failed;

    let result = state.take_result();

    assert!(result.is_none());
    assert!(matches!(state, CwdAnalysisState::Failed));
}

#[test]
fn cwd_analysis_state_is_pending() {
    assert!(CwdAnalysisState::Pending.is_pending());
    assert!(!CwdAnalysisState::NotStarted.is_pending());
    assert!(!CwdAnalysisState::Complete("x".to_string()).is_pending());
    assert!(!CwdAnalysisState::Failed.is_pending());
}
