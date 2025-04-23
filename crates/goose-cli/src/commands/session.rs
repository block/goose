use anyhow::{Context, Result};
use goose::session::info::{get_session_info, SessionInfo, SortOrder};
use regex::Regex;
use std::fs;

pub fn remove_session(session: &SessionInfo) -> Result<()> {
    let should_delete = cliclack::confirm(format!(
        "Are you sure you want to delete session `{}`? (yes/no):?",
        session.id
    ))
    .initial_value(true)
    .interact()?;
    if should_delete {
        fs::remove_file(session.path.clone())
            .with_context(|| format!("Failed to remove session file '{}'", session.path))?;
        println!("Session `{}` removed.", session.id);
    } else {
        println!("Skipping deletion of '{}'.", session.id);
    }
    Ok(())
}

pub fn handle_session_remove(id: String, regex_string: String) -> Result<()> {
    let sessions = match get_session_info(SortOrder::Descending) {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to remove sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to remove sessions"));
        }
    };
    if !id.is_empty() {
        if let Some(session_to_remove) = sessions.iter().find(|s| s.id == id) {
            remove_session(session_to_remove)?;
        } else {
            return Err(anyhow::anyhow!("Session '{}' not found.", id));
        }
    } else if !regex_string.is_empty() {
        let session_regex: Regex = Regex::new(regex_string.as_str())?;
        let mut removed_count = 0;
        for session_info in sessions {
            if session_regex.is_match(session_info.id.as_str()) {
                remove_session(&session_info)?;
                removed_count += 1;
            }
        }
        if removed_count == 0 {
            println!(
                "Regex string '{}' does not match any sessions",
                regex_string
            );
        }
    } else {
        return Err(anyhow::anyhow!(
            "Neither --regex nor --session-name flags provided."
        ));
    }
    Ok(())
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
