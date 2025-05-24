use anyhow::{Context, Result};
use cliclack::{confirm, multiselect};
use goose::session::info::{get_session_info, SessionInfo, SortOrder};
use regex::Regex;
use std::fs;

const TRUNCATED_DESC_LENGTH: usize = 60;

pub fn remove_sessions(sessions: Vec<SessionInfo>) -> Result<()> {
    println!("The following sessions will be removed:");
    for session in &sessions {
        println!("- {}", session.id);
    }

    let should_delete = confirm("Are you sure you want to delete these sessions?")
        .initial_value(false)
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

fn prompt_interactive_session_selection(sessions: &[SessionInfo]) -> Result<Vec<SessionInfo>> {
    if sessions.is_empty() {
        println!("No sessions to delete.");
        return Ok(vec![]);
    }

    let mut selector = multiselect(
        "Select sessions to delete (use spacebar, Enter to confirm, Ctrl+C to cancel):",
    );

    let display_map: std::collections::HashMap<String, SessionInfo> = sessions
        .iter()
        .map(|s| {
            let desc = if s.metadata.description.is_empty() {
                "(no description)"
            } else {
                &s.metadata.description
            };
            let truncated_desc = if desc.len() > TRUNCATED_DESC_LENGTH {
                format!("{}...", &desc[..TRUNCATED_DESC_LENGTH - 3])
            } else {
                desc.to_string()
            };
            let display_text = format!("{} - {} ({})", s.modified, truncated_desc, s.id);
            (display_text, s.clone())
        })
        .collect();

    for display_text in display_map.keys() {
        selector = selector.item(display_text.clone(), display_text.clone(), "");
    }

    let selected_display_texts: Vec<String> = selector.interact()?;

    let selected_sessions: Vec<SessionInfo> = selected_display_texts
        .into_iter()
        .filter_map(|text| display_map.get(&text).cloned())
        .collect();

    Ok(selected_sessions)
}

pub fn handle_session_remove(id: Option<String>, regex_string: Option<String>) -> Result<()> {
    let all_sessions = match get_session_info(SortOrder::Descending) {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to retrieve sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to retrieve sessions"));
        }
    };

    let matched_sessions: Vec<SessionInfo>;

    if let Some(id_val) = id {
        if let Some(session) = all_sessions.iter().find(|s| s.id == id_val) {
            matched_sessions = vec![session.clone()];
        } else {
            return Err(anyhow::anyhow!("Session '{}' not found.", id_val));
        }
    } else if let Some(regex_val) = regex_string {
        let session_regex = Regex::new(&regex_val)
            .with_context(|| format!("Invalid regex pattern '{}'", regex_val))?;

        matched_sessions = all_sessions
            .into_iter()
            .filter(|session| session_regex.is_match(&session.id))
            .collect();

        if matched_sessions.is_empty() {
            println!("Regex string '{}' does not match any sessions", regex_val);
            return Ok(());
        }
    } else {
        if all_sessions.is_empty() {
            return Err(anyhow::anyhow!("No sessions found."));
        }
        matched_sessions = prompt_interactive_session_selection(&all_sessions)?;
    }

    if matched_sessions.is_empty() {
        return Ok(());
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
