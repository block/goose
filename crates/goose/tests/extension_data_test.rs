use goose::session::extension_data::{
    get_or_create_loaded_agents_state, save_loaded_agents_state, ExtensionData, ExtensionState,
    LoadedAgentsState,
};
use std::path::Path;

#[test]
fn test_loaded_agents_state_creation() {
    let state = LoadedAgentsState::new();
    assert!(state.loaded_directories.is_empty());
}

#[test]
fn test_loaded_agents_state_mark_loaded() {
    let mut state = LoadedAgentsState::new();
    let path = Path::new("/repo/features/auth");

    assert!(!state.is_loaded(path));

    let tag = state.mark_loaded(path, 1);
    assert_eq!(tag, "agents_md:/repo/features/auth");
    assert!(state.is_loaded(path));

    let context = state.loaded_directories.get("/repo/features/auth").unwrap();
    assert_eq!(context.access_turn, 1);
}

#[test]
fn test_loaded_agents_state_mark_accessed() {
    let mut state = LoadedAgentsState::new();
    let path = Path::new("/repo/features/auth");

    state.mark_loaded(path, 1);

    // Same turn - should return false
    assert!(!state.mark_accessed(path, 1));

    // New turn - should return true and update
    assert!(state.mark_accessed(path, 5));

    let context = state.loaded_directories.get("/repo/features/auth").unwrap();
    assert_eq!(context.access_turn, 5);

    // Same turn again - should return false
    assert!(!state.mark_accessed(path, 5));
}

#[test]
fn test_prune_stale() {
    let mut state = LoadedAgentsState::new();

    state.mark_loaded(Path::new("/repo/auth"), 1);
    state.mark_loaded(Path::new("/repo/payments"), 2);
    state.mark_loaded(Path::new("/repo/api"), 10);

    state.mark_accessed(Path::new("/repo/auth"), 8);

    // At turn 20, with max_idle_turns=10:
    // Clone state for independent test case
    let mut state_1 = state.clone();
    let pruned = state_1.prune_stale(20, 10);
    assert_eq!(pruned.len(), 3); // All are stale or at threshold
    assert!(!state_1.is_loaded(Path::new("/repo/auth")));

    // With max_idle_turns=11:
    let pruned = state.prune_stale(20, 11);
    assert_eq!(pruned.len(), 2); // auth is stale (idle 12), payments is stale (idle 18), api is not stale (idle 10)

    assert!(!state.is_loaded(Path::new("/repo/auth")));
    assert!(!state.is_loaded(Path::new("/repo/payments")));
    assert!(state.is_loaded(Path::new("/repo/api")));
}

#[test]
fn test_loaded_agents_state_serialization() {
    let mut state = LoadedAgentsState::new();
    state.mark_loaded(Path::new("/repo/features/auth"), 1);
    state.mark_loaded(Path::new("/repo/features/payments"), 2);

    let mut extension_data = ExtensionData::default();
    state.to_extension_data(&mut extension_data).unwrap();

    let restored = LoadedAgentsState::from_extension_data(&extension_data).unwrap();
    assert_eq!(state, restored);
    assert_eq!(restored.loaded_directories.len(), 2);
}

#[test]
fn test_get_or_create_loaded_agents_state() {
    let extension_data = ExtensionData::default();
    let state = get_or_create_loaded_agents_state(&extension_data);
    assert!(state.loaded_directories.is_empty());

    let mut extension_data = ExtensionData::default();
    let mut state = LoadedAgentsState::new();
    state.mark_loaded(Path::new("/test"), 1);
    save_loaded_agents_state(&mut extension_data, &state).unwrap();

    let restored = get_or_create_loaded_agents_state(&extension_data);
    assert!(restored.is_loaded(Path::new("/test")));
}
