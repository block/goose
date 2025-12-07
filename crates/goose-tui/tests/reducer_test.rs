use goose_tui::services::config::TuiConfig;
use goose_tui::state::action::Action;
use goose_tui::state::reducer::update;
use goose_tui::state::{ActivePopup, AppState, InputMode};

fn test_state() -> AppState {
    let config = TuiConfig::load().unwrap_or_else(|_| TuiConfig {
        theme: goose_tui::utils::styles::Theme::default(),
        custom_commands: Vec::new(),
        smart_context: true,
    });
    AppState::new("test-session".to_string(), config, None, None)
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
    assert_eq!(state.active_popup, ActivePopup::Config);

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
