pub mod storage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utoipa::ToSchema;

/// Main project structure that holds project metadata and associated sessions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Unique identifier for the project
    pub id: String,
    /// Display name of the project
    pub name: String,
    /// Optional description of the project
    pub description: Option<String>,
    /// Default working directory for sessions in this project
    #[schema(value_type = String, example = "/home/user/projects/my-project")]
    pub default_directory: PathBuf,
    /// When the project was created
    pub created_at: DateTime<Utc>,
    /// When the project was last updated
    pub updated_at: DateTime<Utc>,
    /// List of session IDs associated with this project
    pub session_ids: Vec<String>,
}

/// Simplified project metadata for listing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMetadata {
    /// Unique identifier for the project
    pub id: String,
    /// Display name of the project
    pub name: String,
    /// Optional description of the project
    pub description: Option<String>,
    /// Default working directory for sessions in this project
    #[schema(value_type = String)]
    pub default_directory: PathBuf,
    /// Number of sessions in this project
    pub session_count: usize,
    /// When the project was created
    pub created_at: DateTime<Utc>,
    /// When the project was last updated
    pub updated_at: DateTime<Utc>,
}

impl From<&Project> for ProjectMetadata {
    fn from(project: &Project) -> Self {
        ProjectMetadata {
            id: project.id.clone(),
            name: project.name.clone(),
            description: project.description.clone(),
            default_directory: project.default_directory.clone(),
            session_count: project.session_ids.len(),
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

// Re-export storage functions
pub use storage::{
    add_session_to_project, create_project, delete_project, ensure_project_dir, get_project,
    list_projects, remove_session_from_project, update_project,
};
