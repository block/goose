pub mod extension_data;
mod legacy;
pub mod session_manager;
use crate::recipe::Recipe;
use crate::session::extension_data::ExtensionData;
use serde::{Deserialize, Serialize};
pub use session_manager::SessionManager;
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(Clone, Serialize, ToSchema)]
pub struct SessionInfo {
    pub id: String,
    pub path: String,
    pub modified: String,
    pub metadata: SessionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionMetadata {
    /// Working directory for the session
    #[schema(value_type = String, example = "/home/user/sessions/session1")]
    pub working_dir: PathBuf,
    /// A short description of the session, typically 3 words or less
    pub description: String,
    /// ID of the schedule that triggered this session, if any
    pub schedule_id: Option<String>,

    /// Number of messages in the session
    pub message_count: usize,
    /// The total number of tokens used in the session. Retrieved from the provider's last usage.
    pub total_tokens: Option<i32>,
    /// The number of input tokens used in the session. Retrieved from the provider's last usage.
    pub input_tokens: Option<i32>,
    /// The number of output tokens used in the session. Retrieved from the provider's last usage.
    pub output_tokens: Option<i32>,
    /// The total number of tokens used in the session. Accumulated across all messages (useful for tracking cost over an entire session).
    pub accumulated_total_tokens: Option<i32>,
    /// The number of input tokens used in the session. Accumulated across all messages.
    pub accumulated_input_tokens: Option<i32>,
    /// The number of output tokens used in the session. Accumulated across all messages.
    pub accumulated_output_tokens: Option<i32>,

    /// Extension data containing extension states
    #[serde(default)]
    pub extension_data: extension_data::ExtensionData,

    pub recipe: Option<Recipe>,
}

impl SessionMetadata {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            description: String::new(),
            schedule_id: None,
            message_count: 0,
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
            extension_data: ExtensionData::new(),
            recipe: None,
        }
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self::new(working_dir)
    }
}
