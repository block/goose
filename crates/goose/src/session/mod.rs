pub mod info;
pub mod storage;

// Re-export common session types and functions
pub use storage::{
    ensure_session_dir, generate_description, generate_session_id, get_most_recent_session,
    get_path, list_sessions, persist_messages, read_messages, read_metadata, update_metadata,
    Identifier, SessionMetadata,
};

pub use info::{get_session_info, SessionInfo};
