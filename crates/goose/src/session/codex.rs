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
    use std::fs;
    use tempfile::TempDir;

    const SAMPLE_CODEX_JSONL: &str = r#"{"timestamp":"2025-11-20T22:56:16.194Z","type":"session_meta","payload":{"id":"019aa37b-9321-7fa3-9b75-eaf925a29a3c","timestamp":"2025-11-20T22:56:16.161Z","cwd":"/Users/test/project","originator":"codex_cli_rs","cli_version":"0.55.0"}}
{"timestamp":"2025-11-20T22:56:19.969Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"what does this project do"}]}}
{"timestamp":"2025-11-20T22:56:25.665Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"This looks like a Rust project"}]}}
"#;

    #[test]
    fn test_load_codex_session() {
        let temp_dir = TempDir::new().unwrap();
        let codex_dir = temp_dir
            .path()
            .join(".codex")
            .join("sessions")
            .join("2025")
            .join("11")
            .join("20");
        fs::create_dir_all(&codex_dir).unwrap();

        let session_file = codex_dir
            .join("rollout-2025-11-20T22-56-16-019aa37b-9321-7fa3-9b75-eaf925a29a3c.jsonl");
        fs::write(&session_file, SAMPLE_CODEX_JSONL).unwrap();

        std::env::set_var("HOME", temp_dir.path());

        let result = list_codex_sessions();

        if let Ok(sessions) = result {
            assert!(!sessions.is_empty(), "Should find at least one session");
            let (session_id, working_dir, _timestamp) = &sessions[0];
            assert_eq!(session_id, "019aa37b-9321-7fa3-9b75-eaf925a29a3c");
            assert_eq!(working_dir, &PathBuf::from("/Users/test/project"));

            let conversation_result = load_codex_session(session_id);
            assert!(
                conversation_result.is_ok(),
                "Should load conversation successfully"
            );

            if let Ok(conversation) = conversation_result {
                assert_eq!(conversation.messages().len(), 2, "Should have 2 messages");
            }
        }
    }
}
