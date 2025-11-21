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
struct ClaudeJsonLine {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
    message: Option<ClaudeMessageWrapper>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessageWrapper {
    role: Option<String>,
    content: serde_json::Value,
}

pub fn list_claude_sessions() -> Result<Vec<(String, PathBuf, DateTime<Utc>)>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let projects_dir = home.join(".claude").join("projects");

    tracing::debug!("Checking for Claude sessions in: {:?}", projects_dir);

    if !projects_dir.exists() {
        tracing::debug!("Claude projects directory does not exist");
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(projects_dir)? {
        let entry = entry?;
        let project_path = entry.path();

        if !project_path.is_dir() {
            continue;
        }

        for file_entry in std::fs::read_dir(&project_path)? {
            let file_entry = file_entry?;
            let file_path = file_entry.path();

            if !file_path.is_file() {
                continue;
            }

            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if file_name.starts_with("agent-") || !file_name.ends_with(".jsonl") {
                continue;
            }

            if let Ok((session_id, working_dir, updated_at)) = parse_session_metadata(&file_path) {
                tracing::debug!(
                    "Found Claude session: {} updated at {}",
                    session_id,
                    updated_at
                );
                sessions.push((session_id, working_dir, updated_at));
            }
        }
    }

    tracing::debug!("Total Claude sessions found: {}", sessions.len());

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
        if let Ok(entry) = serde_json::from_str::<ClaudeJsonLine>(&line) {
            if session_id.is_none() {
                session_id = entry.session_id;
            }
            if working_dir.is_none() {
                working_dir = entry.cwd;
            }
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

pub fn load_claude_session(session_id: &str) -> Result<Conversation> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let projects_dir = home.join(".claude").join("projects");

    for entry in std::fs::read_dir(projects_dir)? {
        let entry = entry?;
        let project_path = entry.path();

        if !project_path.is_dir() {
            continue;
        }

        for file_entry in std::fs::read_dir(&project_path)? {
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

    Err(anyhow::anyhow!("Session not found"))
}

fn parse_conversation(file_path: &PathBuf) -> Result<Conversation> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<ClaudeJsonLine>(&line) {
            if entry.entry_type.as_deref() == Some("user") {
                if let Some(msg) = entry.message {
                    if let Some(role_str) = msg.role {
                        if role_str == "user" {
                            let content = parse_message_content(&msg.content);
                            if !content.is_empty() {
                                let timestamp = entry
                                    .timestamp
                                    .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
                                    .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
                                    .unwrap_or_else(|| Utc::now().timestamp_millis());

                                messages.push(Message::new(Role::User, timestamp, content));
                            }
                        }
                    }
                }
            } else if entry.entry_type.as_deref() == Some("assistant") {
                if let Some(msg) = entry.message {
                    if let Some(role_str) = msg.role {
                        if role_str == "assistant" {
                            let content = parse_message_content(&msg.content);
                            if !content.is_empty() {
                                let timestamp = entry
                                    .timestamp
                                    .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
                                    .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
                                    .unwrap_or_else(|| Utc::now().timestamp_millis());

                                messages.push(Message::new(Role::Assistant, timestamp, content));
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

    if let Some(text) = content_value.as_str() {
        result.push(MessageContent::text(text.to_string()));
        return result;
    }

    if let Some(content_array) = content_value.as_array() {
        for item in content_array {
            if let Some(obj) = item.as_object() {
                if let Some(content_type) = obj.get("type").and_then(|v| v.as_str()) {
                    match content_type {
                        "text" => {
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                result.push(MessageContent::text(text.to_string()));
                            }
                        }
                        "tool_result" => {
                            if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                                result.push(MessageContent::text(format!(
                                    "[Tool Result]\n{}",
                                    content
                                )));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    result
}
