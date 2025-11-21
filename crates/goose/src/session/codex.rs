use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rmcp::model::Role;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct CodexJsonLine {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    entry_type: String,
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SessionMeta {
    id: String,
    cwd: String,
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseItem {
    #[serde(rename = "type")]
    item_type: String,
    role: Option<String>,
    content: Option<serde_json::Value>,
}

pub fn list_codex_sessions() -> Result<Vec<(String, PathBuf, DateTime<Utc>)>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let sessions_dir = home.join(".codex").join("sessions");

    tracing::debug!("Checking for Codex sessions in: {:?}", sessions_dir);

    if !sessions_dir.exists() {
        tracing::debug!("Codex sessions directory does not exist");
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    // Walk through YYYY/MM/DD structure
    for year_entry in std::fs::read_dir(&sessions_dir)? {
        let year_entry = year_entry?;
        let year_path = year_entry.path();

        if !year_path.is_dir() {
            continue;
        }

        for month_entry in std::fs::read_dir(&year_path)? {
            let month_entry = month_entry?;
            let month_path = month_entry.path();

            if !month_path.is_dir() {
                continue;
            }

            for day_entry in std::fs::read_dir(&month_path)? {
                let day_entry = day_entry?;
                let day_path = day_entry.path();

                if !day_path.is_dir() {
                    continue;
                }

                for file_entry in std::fs::read_dir(&day_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if !file_path.is_file() {
                        continue;
                    }

                    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    if !file_name.starts_with("rollout-") || !file_name.ends_with(".jsonl") {
                        continue;
                    }

                    if let Ok((session_id, working_dir, updated_at)) =
                        parse_session_metadata(&file_path)
                    {
                        tracing::debug!(
                            "Found Codex session: {} updated at {}",
                            session_id,
                            updated_at
                        );
                        sessions.push((session_id, working_dir, updated_at));
                    }
                }
            }
        }
    }

    tracing::debug!("Total Codex sessions found: {}", sessions.len());

    // Sort by timestamp (most recent first) and take top 10
    sessions.sort_by(|a, b| b.2.cmp(&a.2));
    sessions.truncate(10);

    Ok(sessions)
}

fn parse_session_metadata(file_path: &PathBuf) -> Result<(String, PathBuf, DateTime<Utc>)> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut working_dir = None;
    let mut latest_timestamp = None;

    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<CodexJsonLine>(&line) {
            // Parse session metadata from first line
            if entry.entry_type == "session_meta" {
                if let Ok(meta) = serde_json::from_value::<SessionMeta>(entry.payload.clone()) {
                    session_id = Some(meta.id);
                    working_dir = Some(meta.cwd);
                    if let Some(ts) = meta.timestamp {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(&ts) {
                            latest_timestamp = Some(dt.with_timezone(&Utc));
                        }
                    }
                    break;
                }
            }

            // Update latest timestamp from any entry
            if let Some(ts) = entry.timestamp {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&ts) {
                    latest_timestamp = Some(dt.with_timezone(&Utc));
                }
            }
        }
    }

    let session_id = session_id.ok_or_else(|| anyhow::anyhow!("No session ID found"))?;
    let working_dir = working_dir.ok_or_else(|| anyhow::anyhow!("No working dir found"))?;
    let updated_at = latest_timestamp.unwrap_or_else(Utc::now);

    Ok((session_id, PathBuf::from(working_dir), updated_at))
}

pub fn load_codex_session(session_id: &str) -> Result<Conversation> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let sessions_dir = home.join(".codex").join("sessions");

    // Walk through all directories to find the session file
    for year_entry in std::fs::read_dir(&sessions_dir)? {
        let year_entry = year_entry?;
        let year_path = year_entry.path();

        if !year_path.is_dir() {
            continue;
        }

        for month_entry in std::fs::read_dir(&year_path)? {
            let month_entry = month_entry?;
            let month_path = month_entry.path();

            if !month_path.is_dir() {
                continue;
            }

            for day_entry in std::fs::read_dir(&month_path)? {
                let day_entry = day_entry?;
                let day_path = day_entry.path();

                if !day_path.is_dir() {
                    continue;
                }

                for file_entry in std::fs::read_dir(&day_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if !file_path.is_file() {
                        continue;
                    }

                    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    if file_name.contains(session_id) && file_name.ends_with(".jsonl") {
                        return parse_conversation(&file_path);
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("Session not found"))
}

fn parse_conversation(file_path: &PathBuf) -> Result<Conversation> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<CodexJsonLine>(&line) {
            // Only process response_item entries
            if entry.entry_type == "response_item" {
                if let Ok(item) = serde_json::from_value::<ResponseItem>(entry.payload.clone()) {
                    // Only process message types
                    if item.item_type == "message" {
                        if let Some(role_str) = item.role {
                            let role = match role_str.as_str() {
                                "user" => Role::User,
                                "assistant" => Role::Assistant,
                                _ => continue,
                            };

                            if let Some(content_value) = item.content {
                                let content = parse_message_content(&content_value);

                                // Skip environment context messages
                                if !content.is_empty() {
                                    let text = content
                                        .iter()
                                        .filter_map(|c| match c {
                                            MessageContent::Text(text_content) => {
                                                Some(text_content.text.as_str())
                                            }
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>()
                                        .join("");

                                    if !text.starts_with("<environment_context>") {
                                        let timestamp = entry
                                            .timestamp
                                            .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
                                            .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
                                            .unwrap_or_else(|| Utc::now().timestamp_millis());

                                        messages.push(Message::new(role, timestamp, content));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Conversation::new_unvalidated(messages))
}

fn parse_message_content(content_value: &serde_json::Value) -> Vec<MessageContent> {
    let mut result = Vec::new();

    // Handle string content
    if let Some(text) = content_value.as_str() {
        result.push(MessageContent::text(text.to_string()));
        return result;
    }

    // Handle array content
    if let Some(content_array) = content_value.as_array() {
        for item in content_array {
            if let Some(obj) = item.as_object() {
                if let Some("input_text" | "output_text") = obj.get("type").and_then(|v| v.as_str())
                {
                    if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                        result.push(MessageContent::text(text.to_string()));
                    }
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_codex_sessions() {
        let sessions = list_codex_sessions();

        match sessions {
            Ok(session_list) => {
                println!("Found {} Codex sessions", session_list.len());
                for (id, path, timestamp) in session_list.iter().take(3) {
                    println!("Session: {} at {:?} ({})", id, path, timestamp);
                }
            }
            Err(e) => {
                println!("Error listing Codex sessions: {}", e);
            }
        }
    }

    #[test]
    fn test_load_codex_session() {
        let sessions = list_codex_sessions();

        if let Ok(session_list) = sessions {
            if let Some((first_id, _, _)) = session_list.first() {
                let conversation = load_codex_session(first_id);

                match conversation {
                    Ok(conv) => {
                        println!(
                            "Loaded Codex session with {} messages",
                            conv.messages().len()
                        );
                        for (i, msg) in conv.messages().iter().take(2).enumerate() {
                            println!("Message {}: {:?}", i, msg.role);
                        }
                    }
                    Err(e) => {
                        println!("Error loading Codex session: {}", e);
                    }
                }
            }
        }
    }
}
