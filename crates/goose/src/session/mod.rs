pub mod storage;

// Re-export common session types and functions
pub use storage::{
    Identifier, 
    SessionMetadata,
    ensure_session_dir, 
    get_path, 
    get_most_recent_session, 
    list_sessions,
    read_messages, 
    read_metadata,
    persist_messages,
    persist_messages_with_metadata,
    update_metadata,
    generate_session_id,
    create_session,
};