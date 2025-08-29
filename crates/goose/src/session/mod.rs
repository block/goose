pub mod info;
pub mod storage;

// Re-export common session types and functions
pub use storage::{
    ensure_session_dir, generate_description, generate_description_with_schedule_id,
    generate_session_id, get_most_recent_session, get_path, list_sessions, persist_messages,
    persist_messages_with_schedule_id, read_messages, read_metadata, update_metadata, Identifier,
    SessionMetadata,
};

pub use info::{get_valid_sorted_sessions, SessionInfo};
