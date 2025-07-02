use crate::session::{self, SessionMetadata};
use anyhow::Result;
use serde::Serialize;
use std::cmp::Ordering;
use utoipa::ToSchema;

#[derive(Clone, Serialize, ToSchema)]
pub struct SessionInfo {
    pub id: String,
    pub path: String,
    pub modified: String,
    pub metadata: SessionMetadata,
}

/// Sort order for listing sessions
pub enum SortOrder {
    Ascending,
    Descending,
}

pub fn get_valid_sorted_sessions(sort_order: SortOrder) -> Result<Vec<SessionInfo>> {
    let sessions = match session::list_sessions() {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to list sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to list sessions"));
        }
    };
    let mut session_infos: Vec<SessionInfo> = sessions
        .into_iter()
        .filter_map(|(id, path)| {
            let modified = path
                .metadata()
                .and_then(|m| m.modified())
                .map(|time| {
                    chrono::DateTime::<chrono::Utc>::from(time)
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string()
                })
                .ok()?;

            let metadata = session::read_metadata(&path).ok()?;

            Some(SessionInfo {
                id,
                path: path.to_string_lossy().to_string(),
                modified,
                metadata,
            })
        })
        .collect();

    // Sort sessions by modified date
    // Since all dates are in ISO format (YYYY-MM-DD HH:MM:SS UTC), we can just use string comparison
    // This works because the ISO format ensures lexicographical ordering matches chronological ordering
    session_infos.sort_by(|a, b| {
        if a.modified == "Unknown" && b.modified == "Unknown" {
            return Ordering::Equal;
        } else if a.modified == "Unknown" {
            return Ordering::Greater; // Unknown dates go last
        } else if b.modified == "Unknown" {
            return Ordering::Less;
        }

        match sort_order {
            SortOrder::Ascending => a.modified.cmp(&b.modified),
            SortOrder::Descending => b.modified.cmp(&a.modified),
        }
    });

    Ok(session_infos)
}
