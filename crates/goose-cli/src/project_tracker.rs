use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use etcetera::{choose_app_strategy, AppStrategy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Structure to track project information
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// The absolute path to the project directory
    pub path: String,
    /// Last time the project was accessed
    pub last_accessed: DateTime<Utc>,
    /// Last instruction sent to goose (if available)
    pub last_instruction: Option<String>,
    /// Last session ID associated with this project
    pub last_session_id: Option<String>,
}

/// Structure to hold all tracked projects
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectTracker {
    projects: HashMap<String, ProjectInfo>,
}

/// Project information with path as a separate field for easier access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfoDisplay {
    /// The absolute path to the project directory
    pub path: String,
    /// Last time the project was accessed
    pub last_accessed: DateTime<Utc>,
    /// Last instruction sent to goose (if available)
    pub last_instruction: Option<String>,
    /// Last session ID associated with this project
    pub last_session_id: Option<String>,
}

impl ProjectTracker {
    /// Get the path to the projects.json file
    fn get_projects_file() -> Result<PathBuf> {
        let projects_file = choose_app_strategy(crate::APP_STRATEGY.clone())
            .context("goose requires a home dir")?
            .in_data_dir("projects.json");

        // Ensure data directory exists
        if let Some(parent) = projects_file.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(projects_file)
    }

    /// Load the project tracker from the projects.json file
    pub fn load() -> Result<Self> {
        let projects_file = Self::get_projects_file()?;

        if projects_file.exists() {
            let file_content = fs::read_to_string(&projects_file)?;
            let tracker: ProjectTracker = serde_json::from_str(&file_content)
                .context("Failed to parse projects.json file")?;
            Ok(tracker)
        } else {
            // If the file doesn't exist, create a new empty tracker
            Ok(ProjectTracker {
                projects: HashMap::new(),
            })
        }
    }

    /// Save the project tracker to the projects.json file
    pub fn save(&self) -> Result<()> {
        let projects_file = Self::get_projects_file()?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(projects_file, json)?;
        Ok(())
    }

    /// Update project information for the current directory
    ///
    /// # Arguments
    /// * `project_dir` - The project directory to update
    /// * `instruction` - Optional instruction that was sent to goose
    /// * `session_id` - Optional session ID associated with this project
    pub fn update_project(
        &mut self,
        project_dir: &Path,
        instruction: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<()> {
        let dir_str = project_dir.to_string_lossy().to_string();

        // Create or update the project entry
        let project_info = self.projects.entry(dir_str.clone()).or_insert(ProjectInfo {
            path: dir_str,
            last_accessed: Utc::now(),
            last_instruction: None,
            last_session_id: None,
        });

        // Update the last accessed time
        project_info.last_accessed = Utc::now();

        // Update the last instruction if provided
        if let Some(instr) = instruction {
            project_info.last_instruction = Some(instr.to_string());
        }

        // Update the session ID if provided
        if let Some(id) = session_id {
            project_info.last_session_id = Some(id.to_string());
        }

        self.save()
    }

    /// List all tracked projects
    ///
    /// Returns a vector of ProjectInfoDisplay objects
    pub fn list_projects(&self) -> Vec<ProjectInfoDisplay> {
        self.projects
            .values()
            .map(|info| ProjectInfoDisplay {
                path: info.path.clone(),
                last_accessed: info.last_accessed,
                last_instruction: info.last_instruction.clone(),
                last_session_id: info.last_session_id.clone(),
            })
            .collect()
    }
}

/// Update the project tracker with the current directory and optional instruction
///
/// # Arguments
/// * `instruction` - Optional instruction that was sent to goose
/// * `session_id` - Optional session ID associated with this project
pub fn update_project_tracker(instruction: Option<&str>, session_id: Option<&str>) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let mut tracker = ProjectTracker::load()?;
    tracker.update_project(&current_dir, instruction, session_id)
}
