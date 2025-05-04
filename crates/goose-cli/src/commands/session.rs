use anyhow::{Context, Result};
use goose::session::info::{get_session_info, SessionInfo, SortOrder};
use regex::Regex;
use std::fs;

pub fn remove_sessions(sessions: Vec<SessionInfo>) -> Result<()> {
    println!("The following sessions will be removed:");
    for session in &sessions {
        println!("- {}", session.id);
    }

    let should_delete =
        cliclack::confirm("Are you sure you want to delete all these sessions? (yes/no):")
            .initial_value(true)
            .interact()?;

    if should_delete {
        for session in sessions {
            fs::remove_file(session.path.clone())
                .with_context(|| format!("Failed to remove session file '{}'", session.path))?;
            println!("Session `{}` removed.", session.id);
        }
    } else {
        println!("Skipping deletion of the sessions.");
    }

    Ok(())
}

pub fn handle_session_remove(id: String, regex_string: String) -> Result<()> {
    let sessions = match get_session_info(SortOrder::Descending) {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to retrieve sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to retrieve sessions"));
        }
    };

    let matched_sessions: Vec<SessionInfo>;
    if !id.is_empty() {
        if let Some(session) = sessions.iter().find(|s| s.id == id) {
            matched_sessions = vec![session.clone()];
        } else {
            return Err(anyhow::anyhow!("Session '{}' not found.", id));
        }
    } else if !regex_string.is_empty() {
        let session_regex = Regex::new(&regex_string)
            .with_context(|| format!("Invalid regex pattern '{}'", regex_string))?;
        matched_sessions = sessions
            .into_iter()
            .filter(|session| session_regex.is_match(&session.id))
            .collect();

        if matched_sessions.is_empty() {
            println!(
                "Regex string '{}' does not match any sessions",
                regex_string
            );
            return Ok(());
        }
    } else {
        return Err(anyhow::anyhow!("Neither --regex nor --id flags provided."));
    }

    remove_sessions(matched_sessions)
}

pub fn handle_session_list(verbose: bool, format: String, ascending: bool) -> Result<()> {
    let sort_order = if ascending {
        SortOrder::Ascending
    } else {
        SortOrder::Descending
    };

    let sessions = match get_session_info(sort_order) {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to list sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to list sessions"));
        }
    };

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string(&sessions)?);
        }
        _ => {
            if sessions.is_empty() {
                println!("No sessions found");
                return Ok(());
            } else {
                println!("Available sessions:");
                for SessionInfo {
                    id,
                    path,
                    metadata,
                    modified,
                } in sessions
                {
                    let description = if metadata.description.is_empty() {
                        "(none)"
                    } else {
                        &metadata.description
                    };
                    let output = format!("{} - {} - {}", id, description, modified);
                    if verbose {
                        println!("  {}", output);
                        println!("    Path: {}", path);
                    } else {
                        println!("{}", output);
                    }
                }
            }
        }
    }
    Ok(())
}
