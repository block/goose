use anyhow::Result;
use chrono::DateTime;
use console::{style, Term};
use serde_json::json;
use std::path::Path;

use crate::project_tracker::ProjectTracker;

/// Handle the project list command
///
/// # Arguments
/// * `verbose` - Whether to show verbose output
/// * `format` - Output format (text, json)
/// * `ascending` - Sort by date in ascending order
pub fn handle_project_list(verbose: bool, format: &str, ascending: bool) -> Result<()> {
    let tracker = ProjectTracker::load()?;
    let mut projects = tracker.list_projects();

    // Sort projects by last_accessed
    projects.sort_by(|a, b| {
        let ordering = a.last_accessed.cmp(&b.last_accessed);
        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });

    match format.to_lowercase().as_str() {
        "json" => {
            let json_output = json!({
                "projects": projects
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        "text" | _ => {
            let term = Term::stdout();
            term.clear_screen()?;

            if projects.is_empty() {
                println!("No projects found.");
                return Ok(());
            }

            println!(
                "{:<40} {:<30} {:<20}",
                style("PROJECT DIRECTORY").bold(),
                style("LAST ACCESSED").bold(),
                style("SESSION ID").bold()
            );

            for project in projects {
                let path_display = if verbose {
                    project.path.clone()
                } else {
                    // Get the last two directory components for display
                    let path = Path::new(&project.path);
                    let components: Vec<_> = path.components().collect();
                    let len = components.len();
                    
                    if len <= 2 {
                        project.path.clone()
                    } else {
                        let mut short_path = String::new();
                        short_path.push_str("...");
                        for i in (len - 2)..len {
                            short_path.push('/');
                            short_path.push_str(components[i].as_os_str().to_string_lossy().as_ref());
                        }
                        short_path
                    }
                };

                let formatted_date = format_date(project.last_accessed);
                let session_id = project.last_session_id.unwrap_or_else(|| "-".to_string());

                println!(
                    "{:<40} {:<30} {:<20}",
                    path_display,
                    formatted_date,
                    session_id
                );
            }
        }
    }

    Ok(())
}

/// Handle the project resume command
///
/// # Arguments
/// * `project_index` - Index of the project to resume (1-based)
pub fn handle_project_resume(project_index: usize) -> Result<()> {
    let tracker = ProjectTracker::load()?;
    let mut projects = tracker.list_projects();

    // Sort projects by last_accessed (newest first)
    projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

    if projects.is_empty() {
        println!("No projects found to resume.");
        return Ok(());
    }

    if project_index == 0 || project_index > projects.len() {
        println!(
            "Invalid project index. Please specify a number between 1 and {}.",
            projects.len()
        );
        return Ok(());
    }

    let project = &projects[project_index - 1];
    let project_dir = &project.path;
    
    // Check if the directory exists
    if !Path::new(project_dir).exists() {
        println!("Project directory '{}' no longer exists.", project_dir);
        return Ok(());
    }

    // Get the session ID if available
    let session_id = project.last_session_id.clone();

    // Change to the project directory and run Goose with the session ID
    println!("Changing to directory: {}", project_dir);
    std::env::set_current_dir(project_dir)?;

    // Build the command to run Goose
    let mut command = std::process::Command::new("goose");
    command.arg("session");

    if let Some(id) = session_id {
        command.arg("--name").arg(&id).arg("--resume");
        println!("Resuming session: {}", id);
    }

    // Execute the command
    let status = command.status()?;
    
    if !status.success() {
        println!("Failed to run Goose. Exit code: {:?}", status.code());
    }

    Ok(())
}

/// Format a DateTime for display
fn format_date(date: DateTime<chrono::Utc>) -> String {
    // Format: "2025-05-08 18:15:30"
    date.format("%Y-%m-%d %H:%M:%S").to_string()
}