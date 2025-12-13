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

#[derive(Debug, Clone)]
pub struct CodexSession {
    pub id: String,
    pub working_dir: PathBuf,
    pub updated_at: DateTime<Utc>,
    pub file_path: PathBuf,
}

fn find_all_sessions() -> Result<Vec<CodexSession>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let sessions_dir = home.join(".codex").join("sessions");

    if !sessions_dir.exists() {
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

                    if let Ok(session) = parse_session_metadata(&file_path) {
                        sessions.push(session);
                    }
                }
            }
        }
    }

    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sessions.truncate(10);

    Ok(sessions)
}

pub fn list_codex_sessions() -> Result<Vec<CodexSession>> {
    find_all_sessions()
}

fn parse_session_metadata(file_path: &PathBuf) -> Result<CodexSession> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut session_id = None;
    let mut working_dir = None;
    let mut latest_timestamp = None;

    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<CodexJsonLine>(&line) {
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

            if let Some(ts) = entry.timestamp {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&ts) {
                    latest_timestamp = Some(dt.with_timezone(&Utc));
                }
            }
        }
    }

    Ok(CodexSession {
        id: session_id.ok_or_else(|| anyhow::anyhow!("No session ID found"))?,
        working_dir: PathBuf::from(
            working_dir.ok_or_else(|| anyhow::anyhow!("No working dir found"))?,
        ),
        updated_at: latest_timestamp.unwrap_or_else(Utc::now),
        file_path: file_path.clone(),
    })
}

pub fn load_codex_session_from_path(file_path: &PathBuf) -> Result<Conversation> {
    parse_conversation(file_path)
}

fn parse_conversation(file_path: &PathBuf) -> Result<Conversation> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<CodexJsonLine>(&line) {
            if entry.entry_type == "response_item" {
                if let Ok(item) = serde_json::from_value::<ResponseItem>(entry.payload.clone()) {
                    if item.item_type == "message" {
                        if let Some(role_str) = item.role {
                            let role = match role_str.as_str() {
                                "user" => Role::User,
                                "assistant" => Role::Assistant,
                                _ => continue,
                            };

                            if let Some(content_value) = item.content {
                                let content = parse_message_content(&content_value);

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

    if let Some(text) = content_value.as_str() {
        result.push(MessageContent::text(text.to_string()));
        return result;
    }

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
    use std::io::Write;
    use tempfile::NamedTempFile;

    const SAMPLE_CODEX_JSONL: &str = r#"{"timestamp":"2025-11-20T22:56:16.194Z","type":"session_meta","payload":{"id":"019aa37b-9321-7fa3-9b75-eaf925a29a3c","timestamp":"2025-11-20T22:56:16.161Z","cwd":"/Users/test/project","originator":"codex_cli_rs","cli_version":"0.55.0"}}
{"timestamp":"2025-11-20T22:56:19.969Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"what does this project do"}]}}
{"timestamp":"2025-11-20T22:56:25.665Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"This looks like a Rust project"}]}}
"#;

    #[test]
    fn test_parse_codex_session() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(SAMPLE_CODEX_JSONL.as_bytes()).unwrap();

        let session = parse_session_metadata(&temp_file.path().to_path_buf()).unwrap();
        assert_eq!(session.id, "019aa37b-9321-7fa3-9b75-eaf925a29a3c");
        assert_eq!(session.working_dir, PathBuf::from("/Users/test/project"));

        let conversation = parse_conversation(&temp_file.path().to_path_buf()).unwrap();
        assert_eq!(conversation.messages().len(), 2);
        assert_eq!(conversation.messages()[0].role, Role::User);
        assert_eq!(conversation.messages()[1].role, Role::Assistant);
    }
}
