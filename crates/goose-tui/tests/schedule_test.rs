use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use goose_client::ScheduledJob;
use goose_tui::components::popups::schedule::{FormField, SchedulePopup, View};
use goose_tui::state::action::Action;

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn make_test_job(id: &str, paused: bool, currently_running: bool) -> ScheduledJob {
    ScheduledJob {
        id: id.to_string(),
        source: format!("/path/{}.yaml", id),
        cron: "0 9 * * *".to_string(),
        paused,
        currently_running,
        last_run: None,
        current_session_id: None,
        process_start_time: None,
    }
}

#[test]
fn list_navigation_wraps() {
    let mut popup = SchedulePopup::new();
    popup.jobs = vec![
        make_test_job("job1", false, false),
        make_test_job("job2", false, false),
        make_test_job("job3", false, false),
    ];
    popup.list_state.select(Some(2));

    popup.handle_list_key(make_key(KeyCode::Char('j')));
    assert_eq!(popup.list_state.selected(), Some(0));

    popup.handle_list_key(make_key(KeyCode::Char('k')));
    assert_eq!(popup.list_state.selected(), Some(2));
}

#[test]
fn create_validates_empty_fields() {
    let mut popup = SchedulePopup::new();
    popup.view = View::Create;

    let action = popup.handle_create_key(make_key(KeyCode::Enter));
    assert!(action.is_none());
    assert!(popup.error_message.is_some());
}

#[test]
fn pause_toggle_returns_correct_action() {
    let mut popup = SchedulePopup::new();
    popup.jobs = vec![make_test_job("job1", false, false)];
    popup.list_state.select(Some(0));

    let action = popup.handle_list_key(make_key(KeyCode::Char('p')));
    assert!(matches!(action, Some(Action::PauseSchedule(_))));

    popup.jobs = vec![make_test_job("job1", true, false)];
    let action = popup.handle_list_key(make_key(KeyCode::Char('p')));
    assert!(matches!(action, Some(Action::UnpauseSchedule(_))));
}

#[test]
fn kill_only_works_on_running_jobs() {
    let mut popup = SchedulePopup::new();
    popup.jobs = vec![make_test_job("job1", false, false)];
    popup.list_state.select(Some(0));

    let action = popup.handle_list_key(make_key(KeyCode::Char('K')));
    assert!(action.is_none());

    popup.jobs = vec![make_test_job("job1", false, true)];
    let action = popup.handle_list_key(make_key(KeyCode::Char('K')));
    assert!(action.is_some());
}

#[test]
fn cron_preset_applies() {
    let mut popup = SchedulePopup::new();
    popup.view = View::Create;
    popup.form_field = FormField::Cron;
    popup.handle_create_key(make_key(KeyCode::Char('2')));

    assert_eq!(
        SchedulePopup::get_input_text(&popup.cron_input),
        "0 9 * * *"
    );
}

#[test]
fn close_popup_returns_action() {
    let mut popup = SchedulePopup::new();
    let action = popup.handle_list_key(make_key(KeyCode::Esc));
    assert!(matches!(action, Some(Action::ClosePopup)));

    let action = popup.handle_list_key(make_key(KeyCode::Char('q')));
    assert!(matches!(action, Some(Action::ClosePopup)));
}

#[test]
fn run_now_returns_action() {
    let mut popup = SchedulePopup::new();
    popup.jobs = vec![make_test_job("job1", false, false)];
    popup.list_state.select(Some(0));

    let action = popup.handle_list_key(make_key(KeyCode::Char('r')));
    assert!(matches!(action, Some(Action::RunScheduleNow(id)) if id == "job1"));
}

#[test]
fn delete_confirmation_flow() {
    let mut popup = SchedulePopup::new();
    popup.jobs = vec![make_test_job("job1", false, false)];
    popup.list_state.select(Some(0));

    popup.handle_list_key(make_key(KeyCode::Char('d')));
    assert_eq!(popup.view, View::ConfirmDelete);
    assert_eq!(popup.pending_delete_id, Some("job1".to_string()));

    let action = popup.handle_confirm_delete_key(make_key(KeyCode::Char('n')));
    assert!(action.is_none());
    assert_eq!(popup.view, View::List);
    assert!(popup.pending_delete_id.is_none());
}

#[test]
fn delete_confirmed() {
    let mut popup = SchedulePopup::new();
    popup.view = View::ConfirmDelete;
    popup.pending_delete_id = Some("job1".to_string());

    let action = popup.handle_confirm_delete_key(make_key(KeyCode::Char('y')));
    assert!(matches!(action, Some(Action::DeleteSchedule(id)) if id == "job1"));
}
