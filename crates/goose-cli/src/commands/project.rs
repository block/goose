use anyhow::Result;
use chrono::DateTime;
use cliclack::{self, intro, outro};
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
        _ => {
            // Default to text format
            let term = Term::stdout();
            term.clear_screen()?;

            if projects.is_empty() {
                println!("No projects found.");
                return Ok(());
            }

            println!(
                "{:<40} {:<30} {:<20} {:<40}",
                style("PROJECT DIRECTORY").bold(),
                style("LAST ACCESSED").bold(),
                style("SESSION ID").bold(),
                style("LAST INSTRUCTION").bold()
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
                        for component in components.iter().skip(len - 2) {
                            short_path.push('/');
                            short_path.push_str(component.as_os_str().to_string_lossy().as_ref());
                        }
                        short_path
                    }
                };

                let formatted_date = format_date(project.last_accessed);
                let session_id = project.last_session_id.unwrap_or_else(|| "-".to_string());
                
                // Format the last instruction for display (truncated)
                let instruction_display = project.last_instruction.map_or("-".to_string(), |instr| {
                    if instr.len() > 37 {
                        format!("{}...", &instr[0..34])
                    } else {
                        instr
                    }
                });

                println!(
                    "{:<40} {:<30} {:<20} {:<40}",
                    path_display, formatted_date, session_id, instruction_display
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
    let has_previous_session = session_id.is_some();

    // Change to the project directory
    println!("Changing to directory: {}", project_dir);
    std::env::set_current_dir(project_dir)?;

    // Only ask about resuming if there's a previous session
    let resume_session = if has_previous_session {
        let _ = intro("Goose Project Manager");
        let session_choice = cliclack::select("What would you like to do?")
            .item("resume", "Resume previous session", "Continue with the previous session")
            .item("new", "Start new session", "Start a fresh session in this project directory")
            .interact()?;
        
        let _ = outro(if session_choice == "resume" {
            "Resuming previous session"
        } else {
            "Starting new session"
        });
        
        session_choice == "resume"
    } else {
        false
    };

    // Build the command to run Goose
    let mut command = std::process::Command::new("goose");
    command.arg("session");

    if resume_session {
        if let Some(id) = session_id {
            command.arg("--name").arg(&id).arg("--resume");
            println!("Resuming session: {}", id);
        }
    } else {
        println!("Starting new session");
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

/// Handle the default project command (no subcommand)
///
/// Offers options to resume the most recently accessed project
pub fn handle_project_default() -> Result<()> {
    let tracker = ProjectTracker::load()?;
    let mut projects = tracker.list_projects();

    if projects.is_empty() {
        // If no projects exist, just start a new one in the current directory
        println!("No previous projects found. Starting a new session in the current directory.");
        let mut command = std::process::Command::new("goose");
        command.arg("session");
        let status = command.status()?;

        if !status.success() {
            println!("Failed to run Goose. Exit code: {:?}", status.code());
        }
        return Ok(());
    }

    // Sort projects by last_accessed (newest first)
    projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

    // Get the most recent project
    let project = &projects[0];
    let project_dir = &project.path;

    // Check if the directory exists
    if !Path::new(project_dir).exists() {
        println!(
            "Most recent project directory '{}' no longer exists.",
            project_dir
        );
        return Ok(());
    }

    // Format the path for display
    let path = Path::new(project_dir);
    let components: Vec<_> = path.components().collect();
    let len = components.len();
    let short_path = if len <= 2 {
        project_dir.clone()
    } else {
        let mut path_str = String::new();
        path_str.push_str("...");
        for component in components.iter().skip(len - 2) {
            path_str.push('/');
            path_str.push_str(component.as_os_str().to_string_lossy().as_ref());
        }
        path_str
    };

    // Ask the user what they want to do
    let _ = intro("Goose Project Manager");

    let current_dir = std::env::current_dir()?;
    let current_dir_display = current_dir.display();

    let choice = cliclack::select("Choose an option:")
        .item(
            "resume",
            format!("Resume project with session: {}", short_path),
            "Continue with the previous session",
        )
        .item(
            "fresh",
            format!("Resume project with fresh session: {}", short_path),
            "Change to the project directory but start a new session",
        )
        .item(
            "new",
            format!(
                "Start new project in current directory: {}",
                current_dir_display
            ),
            "Stay in the current directory and start a new session",
        )
        .interact()?;

    match choice {
        "resume" => {
            let _ = outro(format!("Changing to directory: {}", project_dir));

            // Get the session ID if available
            let session_id = project.last_session_id.clone();

            // Change to the project directory
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
        }
        "fresh" => {
            let _ = outro(format!(
                "Changing to directory: {} with a fresh session",
                project_dir
            ));

            // Change to the project directory
            std::env::set_current_dir(project_dir)?;

            // Build the command to run Goose with a fresh session
            let mut command = std::process::Command::new("goose");
            command.arg("session");

            // Execute the command
            let status = command.status()?;

            if !status.success() {
                println!("Failed to run Goose. Exit code: {:?}", status.code());
            }
        }
        "new" => {
            let _ = outro("Starting a new session in the current directory");

            // Build the command to run Goose
            let mut command = std::process::Command::new("goose");
            command.arg("session");

            // Execute the command
            let status = command.status()?;

            if !status.success() {
                println!("Failed to run Goose. Exit code: {:?}", status.code());
            }
        }
        _ => {
            let _ = outro("Operation canceled");
        }
    }

    Ok(())
}

/// Handle the interactive projects command
///
/// Shows a list of projects and lets the user select one to resume
pub fn handle_projects_interactive() -> Result<()> {
    let tracker = ProjectTracker::load()?;
    let mut projects = tracker.list_projects();

    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    // Sort projects by last_accessed (newest first)
    projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

    // Format project paths for display
    let project_choices: Vec<(String, String)> = projects
        .iter()
        .enumerate()
        .map(|(i, project)| {
            let path = Path::new(&project.path);
            let components: Vec<_> = path.components().collect();
            let len = components.len();
            let short_path = if len <= 2 {
                project.path.clone()
            } else {
                let mut path_str = String::new();
                path_str.push_str("...");
                for component in components.iter().skip(len - 2) {
                    path_str.push('/');
                    path_str.push_str(component.as_os_str().to_string_lossy().as_ref());
                }
                path_str
            };

            // Include last instruction if available (truncated)
            let instruction_preview = project.last_instruction.as_ref().map_or(String::new(), |instr| {
                let truncated = if instr.len() > 40 {
                    format!("{}...", &instr[0..37])
                } else {
                    instr.clone()
                };
                format!(" [{}]", truncated)
            });

            let formatted_date = format_date(project.last_accessed);
            (
                format!("{}", i + 1),                           // Value to return
                format!("{} ({}){}", short_path, formatted_date, instruction_preview), // Display text with instruction
            )
        })
        .collect();

    // Let the user select a project
    let _ = intro("Goose Project Manager");
    let mut select = cliclack::select("Select a project:");

    // Add each project as an option
    for (value, display) in &project_choices {
        select = select.item(value, display, "");
    }

    // Add a cancel option
    let cancel_value = String::from("cancel");
    select = select.item(&cancel_value, "Cancel", "Don't resume any project");

    let selected = select.interact()?;

    if selected == "cancel" {
        let _ = outro("Project selection canceled.");
        return Ok(());
    }

    // Parse the selected index
    let index = selected.parse::<usize>().unwrap_or(0);
    if index == 0 || index > projects.len() {
        let _ = outro("Invalid selection.");
        return Ok(());
    }

    // Get the selected project
    let project = &projects[index - 1];
    let project_dir = &project.path;

    // Check if the directory exists
    if !Path::new(project_dir).exists() {
        let _ = outro(format!(
            "Project directory '{}' no longer exists.",
            project_dir
        ));
        return Ok(());
    }

    // Ask if the user wants to resume the session or start a new one
    let session_id = project.last_session_id.clone();
    let has_previous_session = session_id.is_some();

    // Change to the project directory first
    std::env::set_current_dir(project_dir)?;
    let _ = outro(format!("Changed to directory: {}", project_dir));

    // Only ask about resuming if there's a previous session
    let resume_session = if has_previous_session {
        let session_choice = cliclack::select("What would you like to do?")
            .item("resume", "Resume previous session", "Continue with the previous session")
            .item("new", "Start new session", "Start a fresh session in this project directory")
            .interact()?;
        
        session_choice == "resume"
    } else {
        false
    };

    // Build the command to run Goose
    let mut command = std::process::Command::new("goose");
    command.arg("session");

    if resume_session {
        if let Some(id) = session_id {
            command.arg("--name").arg(&id).arg("--resume");
            println!("Resuming session: {}", id);
        }
    } else {
        println!("Starting new session");
    }

    // Execute the command
    let status = command.status()?;

    if !status.success() {
        println!("Failed to run Goose. Exit code: {:?}", status.code());
    }

    Ok(())
}
