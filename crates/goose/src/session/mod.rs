pub mod storage;

// Re-export common session types and functions
pub use storage::{
    Identifier, 
    ensure_session_dir, 
    get_path, 
    get_most_recent_session, 
    list_sessions,
    read_messages, 
    persist_messages,
    generate_session_id,
};