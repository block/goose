use crate::conversation::Conversation;
use crate::session::Session;
use anyhow::Result;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;

pub fn list_sessions(session_dir: &PathBuf) -> Result<Vec<(String, PathBuf)>> {
    let entries = fs::read_dir(session_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "jsonl") {
                let name = path.file_stem()?.to_string_lossy().to_string();
                Some((name, path))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(entries)
}

pub fn load_session(session_name: &str, session_path: &Path) -> Result<Session> {
    let file = fs::File::open(session_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to open session file {}: {}",
            session_path.display(),
            e
        )
    })?;

    let file_metadata = file.metadata()?;

    if file_metadata.len() > MAX_FILE_SIZE {
        return Err(anyhow::anyhow!("Session file too large"));
    }
    if file_metadata.len() == 0 {
        return Err(anyhow::anyhow!("Empty session file"));
    }

    let modified_time = file_metadata.modified().unwrap_or(SystemTime::now());
    let created_time = file_metadata.created().unwrap_or(modified_time);

    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();
    let mut messages = Vec::new();
    let mut session = Session::default();
    session.id = session_name.to_string();

    if let Some(Ok(line)) = lines.next() {
        let mut metadata_json: serde_json::Value = serde_json::from_str(&line)
            .map_err(|_| anyhow::anyhow!("Invalid session metadata JSON"))?;

        if let Some(obj) = metadata_json.as_object_mut() {
            obj.entry("id").or_insert(serde_json::json!(session_name));
            obj.entry("created_at")
                .or_insert(serde_json::json!(format_timestamp(created_time)?));
            obj.entry("updated_at")
                .or_insert(serde_json::json!(format_timestamp(modified_time)?));
            obj.entry("extension_data").or_insert(serde_json::json!({}));
            obj.entry("message_count").or_insert(serde_json::json!(0));
        }

        session = serde_json::from_value(metadata_json)?;
        session.id = session_name.to_string();
    }

    for line_result in lines {
        if let Ok(line) = line_result {
            if let Ok(message) = serde_json::from_str(&line) {
                messages.push(message);
            }
        }
    }

    if !messages.is_empty() {
        session.conversation = Some(Conversation::new_unvalidated(messages));
    }

    Ok(session)
}

fn format_timestamp(time: SystemTime) -> Result<String> {
    let duration = time.duration_since(std::time::UNIX_EPOCH)?;
    let timestamp = chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)
        .unwrap_or_default()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    Ok(timestamp)
}
