mod manager;
mod migrations;
pub mod session_storage;

pub use manager::StorageManager;
pub use migrations::CURRENT_SCHEMA_VERSION;
pub use session_storage::{SessionStorage, ensure_session_dir};

pub const SESSIONS_FOLDER: &str = "sessions";
pub const DB_NAME: &str = "sessions.db";
