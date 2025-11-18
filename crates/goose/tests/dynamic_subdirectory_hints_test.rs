use goose::session::extension_data::{
    get_or_create_conversation_turn_state, save_conversation_turn_state, ConversationTurnState,
    ExtensionData,
};

#[test]
fn test_conversation_turn_state_increment() {
    let mut state = ConversationTurnState::new();
    assert_eq!(state.get(), 0);

    assert_eq!(state.increment(), 1);
    assert_eq!(state.get(), 1);

    assert_eq!(state.increment(), 2);
    assert_eq!(state.increment(), 3);
    assert_eq!(state.get(), 3);
}

#[test]
fn test_conversation_turn_state_serialization() {
    let mut state = ConversationTurnState::new();
    state.increment();
    state.increment();
    state.increment();

    let mut extension_data = ExtensionData::default();
    save_conversation_turn_state(&mut extension_data, &state).unwrap();

    let restored = get_or_create_conversation_turn_state(&extension_data);
    assert_eq!(restored.get(), 3);
}

#[test]
fn test_conversation_turn_state_default() {
    let extension_data = ExtensionData::default();
    let state = get_or_create_conversation_turn_state(&extension_data);
    assert_eq!(state.get(), 0, "Should default to turn 0");
}

#[test]
fn test_hierarchical_hint_loading() {
    temp_env::with_var(
        "GOOSE_DYNAMIC_SUBDIRECTORY_HINT_LOADING",
        Some("true"),
        || {
            // Setup: Create nested directory structure
            let temp_dir = tempfile::TempDir::new().unwrap();
            let repo_root = temp_dir.path();
            std::fs::create_dir(repo_root.join(".git")).unwrap();

            // Create hints at different levels
            std::fs::write(repo_root.join(".goosehints"), "Root hints").unwrap();

            let features_dir = repo_root.join("features");
            std::fs::create_dir(&features_dir).unwrap();
            std::fs::write(features_dir.join("AGENTS.md"), "Features hints").unwrap();

            let auth_dir = features_dir.join("auth");
            std::fs::create_dir(&auth_dir).unwrap();
            std::fs::write(auth_dir.join("AGENTS.md"), "Auth hints").unwrap();
            std::fs::write(auth_dir.join("file.py"), "pass").unwrap();

            // Load hints from auth directory
            let gitignore = ignore::gitignore::GitignoreBuilder::new(repo_root)
                .build()
                .unwrap();

            let result = goose::hints::load_hints_from_directory(
                &auth_dir,
                repo_root,
                &["AGENTS.md".to_string(), ".goosehints".to_string()],
                &gitignore,
            );

            assert!(result.is_some(), "Should load hints");
            let content = result.unwrap();

            // Should contain hints from root, features, and auth (hierarchical loading)
            assert!(content.contains("Root hints"), "Should load from root");
            assert!(
                content.contains("Features hints"),
                "Should load from features/"
            );
            assert!(content.contains("Auth hints"), "Should load from auth/");
            assert!(
                content.contains("Directory-Specific Hints:"),
                "Should have header"
            );
        },
    );
}
