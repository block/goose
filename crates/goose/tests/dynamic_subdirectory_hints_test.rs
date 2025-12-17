use goose::agents::types::SessionConfig;
use goose::agents::Agent;
use goose::session::extension_data::{
    get_or_create_conversation_turn_state, get_or_create_loaded_agents_state,
    save_conversation_turn_state, save_loaded_agents_state, ConversationTurnState, ExtensionData,
};
use goose::session::{SessionManager, SessionType};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_hint_loading_and_pruning_integration() -> anyhow::Result<()> {
    // This test directly calls the pub(crate) methods to verify integration
    // Tests: loading, state tracking, access time updates, and pruning

    // Setup: Create test directory with hints
    let temp_dir = tempfile::TempDir::new()?;
    let repo_root = temp_dir.path();
    std::fs::create_dir(repo_root.join(".git"))?;

    let auth_dir = repo_root.join("auth");
    std::fs::create_dir(&auth_dir)?;
    std::fs::write(auth_dir.join("AGENTS.md"), "# Auth Hints\nUse JWT tokens")?;
    std::fs::write(auth_dir.join("login.py"), "def login(): pass")?;

    // Create agent and session
    let agent = Agent::new();

    let session = SessionManager::create_session(
        repo_root.to_path_buf(),
        "hint-integration-test".to_string(),
        SessionType::Hidden,
    )
    .await?;

    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: None,
        max_turns: Some(10),
        retry_config: None,
    };

    let file_path = auth_dir.join("login.py");

    // Turn 1: Load hints
    let loaded = agent
        .maybe_load_directory_hints(&file_path, &session_config, 1)
        .await?;
    assert!(loaded, "Should load hints on first access");

    {
        let session = SessionManager::get_session(&session.id, false).await?;
        let loaded_state = get_or_create_loaded_agents_state(&session.extension_data);
        assert!(loaded_state.is_loaded(&auth_dir));

        let context = loaded_state
            .loaded_directories
            .get(&auth_dir.to_string_lossy().to_string())
            .unwrap();
        assert_eq!(context.access_turn, 1);
    }

    // Turn 2: Access again (should update access time)
    let loaded = agent
        .maybe_load_directory_hints(&file_path, &session_config, 2)
        .await?;
    assert!(!loaded, "Should not reload (already loaded)");

    {
        let session = SessionManager::get_session(&session.id, false).await?;
        let loaded_state = get_or_create_loaded_agents_state(&session.extension_data);

        let context = loaded_state
            .loaded_directories
            .get(&auth_dir.to_string_lossy().to_string())
            .unwrap();
        assert_eq!(context.access_turn, 2, "Access time should update");
    }

    // Turn 5: Prune (last access was turn 2, so 3 turns idle)
    // Test pruning via LoadedAgentsState.prune_stale directly
    {
        let mut session = SessionManager::get_session(&session.id, false).await?;
        let mut loaded_state = get_or_create_loaded_agents_state(&session.extension_data);

        let pruned = loaded_state.prune_stale(5, 3);
        assert_eq!(pruned.len(), 1, "Should prune 1 directory");

        save_loaded_agents_state(&mut session.extension_data, &loaded_state)?;
        SessionManager::update_session(&session.id)
            .extension_data(session.extension_data)
            .apply()
            .await?;
    }

    {
        let session = SessionManager::get_session(&session.id, false).await?;
        let loaded_state = get_or_create_loaded_agents_state(&session.extension_data);
        assert!(!loaded_state.is_loaded(&auth_dir), "Should be pruned");
    }

    SessionManager::delete_session(&session.id).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_hint_loading_security_guards() -> anyhow::Result<()> {
    // Setup
    let temp_dir = tempfile::TempDir::new()?;
    let repo_root = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_root)?;
    std::fs::create_dir(repo_root.join(".git"))?;

    let outside_dir = temp_dir.path().join("outside");
    std::fs::create_dir(&outside_dir)?;
    std::fs::write(outside_dir.join("AGENTS.md"), "Forbidden hints")?;
    std::fs::write(outside_dir.join("file.py"), "pass")?;

    let agent = Agent::new();
    let session = SessionManager::create_session(
        repo_root.clone(),
        "hint-security-test".to_string(),
        SessionType::Hidden,
    )
    .await?;
    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: None,
        max_turns: Some(10),
        retry_config: None,
    };

    // Test 1: Path outside working directory
    let loaded = agent
        .maybe_load_directory_hints(&outside_dir.join("file.py"), &session_config, 1)
        .await?;
    assert!(
        !loaded,
        "Should not load hints from outside working directory"
    );

    let session_data = SessionManager::get_session(&session.id, false).await?;
    let loaded_state = get_or_create_loaded_agents_state(&session_data.extension_data);
    assert!(loaded_state.loaded_directories.is_empty());

    SessionManager::delete_session(&session.id).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_hint_loading_filesystem_updates() -> anyhow::Result<()> {
    let temp_dir = tempfile::TempDir::new()?;
    let repo_root = temp_dir.path();
    std::fs::create_dir(repo_root.join(".git"))?;

    let src_dir = repo_root.join("src");
    std::fs::create_dir(&src_dir)?;
    let file_path = src_dir.join("main.rs");
    std::fs::write(&file_path, "fn main() {}")?;

    let agent = Agent::new();
    let session = SessionManager::create_session(
        repo_root.to_path_buf(),
        "hint-fs-update-test".to_string(),
        SessionType::Hidden,
    )
    .await?;
    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: None,
        max_turns: Some(10),
        retry_config: None,
    };

    // Turn 1: Access file, no hints exist yet
    let loaded = agent
        .maybe_load_directory_hints(&file_path, &session_config, 1)
        .await?;
    assert!(!loaded, "Should not load anything (no hints file)");

    // Verify state: NOT marked as loaded (so we retry later)
    {
        let session_data = SessionManager::get_session(&session.id, false).await?;
        let loaded_state = get_or_create_loaded_agents_state(&session_data.extension_data);
        assert!(!loaded_state.is_loaded(&src_dir));
    }

    // Add hints file
    std::fs::write(src_dir.join("AGENTS.md"), "New hints")?;

    // Turn 2: Access again, should load now
    let loaded = agent
        .maybe_load_directory_hints(&file_path, &session_config, 2)
        .await?;
    assert!(loaded, "Should load newly created hints file");

    {
        let session_data = SessionManager::get_session(&session.id, false).await?;
        let loaded_state = get_or_create_loaded_agents_state(&session_data.extension_data);
        assert!(loaded_state.is_loaded(&src_dir));
    }

    SessionManager::delete_session(&session.id).await?;
    Ok(())
}

#[test]
fn test_conversation_turn_state_increment() {
    let mut state = ConversationTurnState::new();
    assert_eq!(state.turn, 0);

    assert_eq!(state.increment(), 1);
    assert_eq!(state.turn, 1);

    assert_eq!(state.increment(), 2);
    assert_eq!(state.increment(), 3);
    assert_eq!(state.turn, 3);
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
    assert_eq!(restored.turn, 3);
}

#[test]
fn test_conversation_turn_state_default() {
    let extension_data = ExtensionData::default();
    let state = get_or_create_conversation_turn_state(&extension_data);
    assert_eq!(state.turn, 0, "Should default to turn 0");
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
            let result = goose::hints::load_hints_from_directory(
                &auth_dir,
                repo_root,
                &["AGENTS.md".to_string(), ".goosehints".to_string()],
            );

            assert!(result.is_some(), "Should load hints");
            let content = result.unwrap();

            // Should contain hints from features and auth (hierarchical loading)
            // Should NOT contain root hints (they are loaded at startup)
            assert!(
                !content.contains("Root hints"),
                "Should NOT load from root (avoid dupes)"
            );
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
