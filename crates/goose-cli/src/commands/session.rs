use crate::session::message_to_markdown;
use anyhow::{Context, Result};
use cliclack::{confirm, multiselect, select};
use goose::session::info::{get_valid_sorted_sessions, SessionInfo, SortOrder};
use goose::session::{self, Identifier};
use goose::utils::safe_truncate;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

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

fn prompt_interactive_session_removal(sessions: &[SessionInfo]) -> Result<Vec<SessionInfo>> {
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
            let truncated_desc = safe_truncate(desc, TRUNCATED_DESC_LENGTH);
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
    let all_sessions = match get_valid_sorted_sessions(SortOrder::Descending) {
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
        matched_sessions = prompt_interactive_session_removal(&all_sessions)?;
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

    let sessions = match get_valid_sorted_sessions(sort_order) {
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

/// Export a session to Markdown without creating a full Session object
///
/// This function directly reads messages from the session file and converts them to Markdown
/// without creating an Agent or prompting about working directories.
pub fn handle_session_export(identifier: Identifier, output_path: Option<PathBuf>) -> Result<()> {
    // Get the session file path
    let session_file_path = match goose::session::get_path(identifier.clone()) {
        Ok(path) => path,
        Err(e) => {
            return Err(anyhow::anyhow!("Invalid session identifier: {}", e));
        }
    };

    if !session_file_path.exists() {
        return Err(anyhow::anyhow!(
            "Session file not found (expected path: {})",
            session_file_path.display()
        ));
    }

    // Read messages directly without using Session
    let messages = match goose::session::read_messages(&session_file_path) {
        Ok(msgs) => msgs,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to read session messages: {}", e));
        }
    };

    // Generate the markdown content using the export functionality
    let markdown =
        export_session_to_markdown(messages.messages().clone(), &session_file_path, None);

    // Output the markdown
    if let Some(output) = output_path {
        fs::write(&output, markdown)
            .with_context(|| format!("Failed to write to output file: {}", output.display()))?;
        println!("Session exported to {}", output.display());
    } else {
        println!("{}", markdown);
    }

    Ok(())
}

/// Convert a list of messages to markdown format for session export
///
/// This function handles the formatting of a complete session including headers,
/// message organization, and proper tool request/response pairing.
fn export_session_to_markdown(
    messages: Vec<goose::conversation::message::Message>,
    session_file: &Path,
    session_name_override: Option<&str>,
) -> String {
    let mut markdown_output = String::new();

    let session_name = session_name_override.unwrap_or_else(|| {
        session_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unnamed Session")
    });

    markdown_output.push_str(&format!("# Session Export: {}\n\n", session_name));

    if messages.is_empty() {
        markdown_output.push_str("*(This session has no messages)*\n");
        return markdown_output;
    }

    markdown_output.push_str(&format!("*Total messages: {}*\n\n---\n\n", messages.len()));

    // Track if the last message had tool requests to properly handle tool responses
    let mut skip_next_if_tool_response = false;

    for message in &messages {
        // Check if this is a User message containing only ToolResponses
        let is_only_tool_response = message.role == rmcp::model::Role::User
            && message.content.iter().all(|content| {
                matches!(
                    content,
                    goose::conversation::message::MessageContent::ToolResponse(_)
                )
            });

        // If the previous message had tool requests and this one is just tool responses,
        // don't create a new User section - we'll attach the responses to the tool calls
        if skip_next_if_tool_response && is_only_tool_response {
            // Export the tool responses without a User heading
            markdown_output.push_str(&message_to_markdown(message, false));
            markdown_output.push_str("\n\n---\n\n");
            skip_next_if_tool_response = false;
            continue;
        }

        // Reset the skip flag - we'll update it below if needed
        skip_next_if_tool_response = false;

        // Output the role prefix except for tool response-only messages
        if !is_only_tool_response {
            let role_prefix = match message.role {
                rmcp::model::Role::User => "### User:\n",
                rmcp::model::Role::Assistant => "### Assistant:\n",
            };
            markdown_output.push_str(role_prefix);
        }

        // Add the message content
        markdown_output.push_str(&message_to_markdown(message, false));
        markdown_output.push_str("\n\n---\n\n");

        // Check if this message has any tool requests, to handle the next message differently
        if message.content.iter().any(|content| {
            matches!(
                content,
                goose::conversation::message::MessageContent::ToolRequest(_)
            )
        }) {
            skip_next_if_tool_response = true;
        }
    }

    markdown_output
}

/// Prompt the user to interactively select a session
///
/// Shows a list of available sessions and lets the user select one
pub fn prompt_interactive_session_selection() -> Result<session::Identifier> {
    // Get sessions sorted by modification date (newest first)
    let sessions = match get_valid_sorted_sessions(SortOrder::Descending) {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to list sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to list sessions"));
        }
    };

    if sessions.is_empty() {
        return Err(anyhow::anyhow!("No sessions found"));
    }

    // Build the selection prompt
    let mut selector = select("Select a session to export:");

    // Map to display text
    let display_map: std::collections::HashMap<String, SessionInfo> = sessions
        .iter()
        .map(|s| {
            let desc = if s.metadata.description.is_empty() {
                "(no description)"
            } else {
                &s.metadata.description
            };

            // Truncate description if too long
            let truncated_desc = safe_truncate(desc, 40);

            let display_text = format!("{} - {} ({})", s.modified, truncated_desc, s.id);
            (display_text, s.clone())
        })
        .collect();

    // Add each session as an option
    for display_text in display_map.keys() {
        selector = selector.item(display_text.clone(), display_text.clone(), "");
    }

    // Add a cancel option
    let cancel_value = String::from("cancel");
    selector = selector.item(cancel_value, "Cancel", "Cancel export");

    // Get user selection
    let selected_display_text: String = selector.interact()?;

    if selected_display_text == "cancel" {
        return Err(anyhow::anyhow!("Export canceled"));
    }

    // Retrieve the selected session
    if let Some(session) = display_map.get(&selected_display_text) {
        Ok(goose::session::Identifier::Name(session.id.clone()))
    } else {
        Err(anyhow::anyhow!("Invalid selection"))
    }
}
