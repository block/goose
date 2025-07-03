use super::Session;
use anyhow::Result;

use crate::session::{utils::extract_session_id, utils::update_project_tracker};

impl Session {
    /// Update the project tracker with the current message
    pub(crate) fn update_project_tracker(&self, message: &str) -> Result<()> {
        let session_id = extract_session_id(&self.session_file);
        update_project_tracker(message, session_id.as_deref())
    }

    /// Update the completion cache with fresh data
    /// This should be called before the interactive session starts
    pub async fn update_completion_cache(&mut self) -> Result<()> {
        self.completion_cache_manager
            .update_cache(&self.agent)
            .await
    }

    /// Invalidate the completion cache
    /// This should be called when extensions are added or removed
    pub async fn invalidate_completion_cache(&self) {
        self.completion_cache_manager.invalidate_cache();
    }
}
