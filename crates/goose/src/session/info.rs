use anyhow::Result;
use serde::Serialize;

use crate::session::{self, SessionMetadata};

#[derive(Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub path: String,
    pub modified: String,
    pub metadata: SessionMetadata,
}

pub fn get_session_info() -> Result<Vec<SessionInfo>> {
    let sessions = match session::list_sessions() {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to list sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to list sessions"));
        }
    };
    let session_infos = sessions
        .into_iter()
        .map(|(id, path)| {
            // Get last modified time as string
            let modified = path
                .metadata()
                .and_then(|m| m.modified())
                .map(|time| {
                    chrono::DateTime::<chrono::Utc>::from(time)
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string()
                })
                .unwrap_or_else(|_| "Unknown".to_string());

            // Get session description
            let metadata = session::read_metadata(&path).expect("Failed to read session metadata");

            SessionInfo {
                id,
                path: path.to_string_lossy().to_string(),
                modified,
                metadata,
            }
        })
        .collect();

    Ok(session_infos)
}
