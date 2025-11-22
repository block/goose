#[cfg(test)]
mod tests {
    use goose_tui::services::config::TuiConfig;
    use goose_tui::state::state::AppState;

    #[test]
    fn test_app_state_creation() {
        let config = TuiConfig::load().unwrap_or_else(|_| TuiConfig {
            theme: goose_tui::utils::styles::Theme::default(),
            custom_commands: vec![],
        });

        let state = AppState::new("test-session".to_string(), config);
        assert_eq!(state.session_id, "test-session");
        assert_eq!(state.messages.len(), 0);
    }
}
