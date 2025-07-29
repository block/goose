use crate::project::{Project, ProjectMetadata};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use serde_json;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tracing::{error, info};

const APP_NAME: &str = "goose";

/// Ensure the project directory exists and return its path
pub fn ensure_project_dir() -> Result<PathBuf> {
    let app_strategy = AppStrategyArgs {
        top_level_domain: "Block".to_string(),
        author: "Block".to_string(),
        app_name: APP_NAME.to_string(),
    };

    let data_dir = choose_app_strategy(app_strategy)
        .context("goose requires a home dir")?
        .data_dir()
        .join("projects");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)?;
    }

    Ok(data_dir)
}

/// Generate a unique project ID
fn generate_project_id() -> String {
    use rand::Rng;
    let timestamp = Utc::now().timestamp();
    let random: u32 = rand::thread_rng().gen();
    format!("proj_{}_{}", timestamp, random)
}

/// Get the path for a specific project file
fn get_project_path(project_id: &str) -> Result<PathBuf> {
    let project_dir = ensure_project_dir()?;
    Ok(project_dir.join(format!("{}.json", project_id)))
}

/// Create a new project
pub fn create_project(
    name: String,
    description: Option<String>,
    default_directory: PathBuf,
) -> Result<Project> {
    let project_dir = ensure_project_dir()?;

    // Validate the default directory exists
    if !default_directory.exists() {
        return Err(anyhow!(
            "Default directory does not exist: {:?}",
            default_directory
        ));
    }

    let now = Utc::now();
    let project = Project {
        id: generate_project_id(),
        name,
        description,
        default_directory,
        created_at: now,
        updated_at: now,
        session_ids: Vec::new(),
    };

    // Save the project
    let project_path = project_dir.join(format!("{}.json", project.id));
    let mut file = File::create(&project_path)?;
    let json = serde_json::to_string_pretty(&project)?;
    file.write_all(json.as_bytes())?;

    info!("Created project {} at {:?}", project.id, project_path);
    Ok(project)
}

/// Update an existing project
pub fn update_project(
    project_id: &str,
    name: Option<String>,
    description: Option<Option<String>>,
    default_directory: Option<PathBuf>,
) -> Result<Project> {
    let project_path = get_project_path(project_id)?;

    if !project_path.exists() {
        return Err(anyhow!("Project not found: {}", project_id));
    }

    // Read existing project
    let mut project: Project = serde_json::from_reader(File::open(&project_path)?)?;

    // Update fields
    if let Some(new_name) = name {
        project.name = new_name;
    }

    if let Some(new_description) = description {
        project.description = new_description;
    }

    if let Some(new_directory) = default_directory {
        if !new_directory.exists() {
            return Err(anyhow!(
                "Default directory does not exist: {:?}",
                new_directory
            ));
        }
        project.default_directory = new_directory;
    }

    project.updated_at = Utc::now();

    // Save updated project
    let mut file = File::create(&project_path)?;
    let json = serde_json::to_string_pretty(&project)?;
    file.write_all(json.as_bytes())?;

    info!("Updated project {}", project_id);
    Ok(project)
}

/// Delete a project (does not delete associated sessions)
pub fn delete_project(project_id: &str) -> Result<()> {
    let project_path = get_project_path(project_id)?;

    if !project_path.exists() {
        return Err(anyhow!("Project not found: {}", project_id));
    }

    fs::remove_file(&project_path)?;
    info!("Deleted project {}", project_id);
    Ok(())
}

/// List all projects
pub fn list_projects() -> Result<Vec<ProjectMetadata>> {
    let project_dir = ensure_project_dir()?;
    let mut projects = Vec::new();

    if let Ok(entries) = fs::read_dir(&project_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match serde_json::from_reader::<_, Project>(File::open(&path)?) {
                    Ok(project) => {
                        projects.push(ProjectMetadata::from(&project));
                    }
                    Err(e) => {
                        error!("Failed to read project file {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    // Sort by updated_at descending
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(projects)
}

/// Get a specific project
pub fn get_project(project_id: &str) -> Result<Project> {
    let project_path = get_project_path(project_id)?;

    if !project_path.exists() {
        return Err(anyhow!("Project not found: {}", project_id));
    }

    let project: Project = serde_json::from_reader(File::open(&project_path)?)?;
    Ok(project)
}

/// Add a session to a project
pub fn add_session_to_project(project_id: &str, session_id: &str) -> Result<()> {
    let project_path = get_project_path(project_id)?;

    if !project_path.exists() {
        return Err(anyhow!("Project not found: {}", project_id));
    }

    // Read project
    let mut project: Project = serde_json::from_reader(File::open(&project_path)?)?;

    // Check if session already exists in project
    if project.session_ids.contains(&session_id.to_string()) {
        return Ok(()); // Already added
    }

    // Add session and update timestamp
    project.session_ids.push(session_id.to_string());
    project.updated_at = Utc::now();

    // Save updated project
    let mut file = File::create(&project_path)?;
    let json = serde_json::to_string_pretty(&project)?;
    file.write_all(json.as_bytes())?;

    info!("Added session {} to project {}", session_id, project_id);
    Ok(())
}

/// Remove a session from a project
pub fn remove_session_from_project(project_id: &str, session_id: &str) -> Result<()> {
    let project_path = get_project_path(project_id)?;

    if !project_path.exists() {
        return Err(anyhow!("Project not found: {}", project_id));
    }

    // Read project
    let mut project: Project = serde_json::from_reader(File::open(&project_path)?)?;

    // Remove session
    let original_len = project.session_ids.len();
    project.session_ids.retain(|id| id != session_id);

    if project.session_ids.len() == original_len {
        return Ok(()); // Session wasn't in project
    }

    project.updated_at = Utc::now();

    // Save updated project
    let mut file = File::create(&project_path)?;
    let json = serde_json::to_string_pretty(&project)?;
    file.write_all(json.as_bytes())?;

    info!("Removed session {} from project {}", session_id, project_id);
    Ok(())
}
