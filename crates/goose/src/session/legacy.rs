use crate::conversation::Conversation;
use crate::session::Session;
use anyhow::Result;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
const MAX_MESSAGE_COUNT: usize = 10000;
const MAX_LINE_LENGTH: usize = 5 * 1024 * 1024;

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

pub fn read_messages(session_file: &Path) -> Result<Conversation> {
    // Basic security check: file size limit
    if session_file.exists() {
        let metadata = fs::metadata(session_file)?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("Session file too large"));
        }
    }

    let file = fs::OpenOptions::new()
        .read(true)
        .create(true)
        .truncate(false)
        .open(session_file)?;

    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();
    let mut messages = Vec::new();
    let mut message_count = 0;

    // Skip first line (metadata)
    if let Some(line_result) = lines.next() {
        match line_result {
            Ok(line) => {
                // Try to parse as metadata, if it fails, treat as message
                if serde_json::from_str::<Session>(&line).is_err() {
                    // First line is a message, not metadata
                    if let Ok(message) = serde_json::from_str(&line) {
                        messages.push(message);
                        message_count += 1;
                    }
                }
            }
            Err(_) => {} // Skip unreadable first line
        }
    }

    // Read the rest as messages
    for line_result in lines {
        if message_count >= MAX_MESSAGE_COUNT {
            break;
        }

        match line_result {
            Ok(line) => {
                if line.len() > MAX_LINE_LENGTH {
                    continue;
                }

                if let Ok(message) = serde_json::from_str(&line) {
                    messages.push(message);
                    message_count += 1;
                }
            }
            Err(_) => {} // Skip unreadable lines
        }
    }

    Ok(Conversation::new_unvalidated(messages))
}

pub async fn read_metadata(session_file: &Path) -> Result<Session> {
    let mut file = match tokio::fs::File::open(session_file).await {
        Ok(file) => file,
        Err(_) => return Ok(Session::default()),
    };

    let mut buffer = vec![0u8; MAX_LINE_LENGTH.min(1024)];
    let bytes_read = file.read(&mut buffer).await?;

    if bytes_read == 0 {
        return Ok(Session::default());
    }

    let content = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = content.lines().next().unwrap_or("");

    if first_line.len() > MAX_LINE_LENGTH {
        return Err(anyhow::anyhow!("Metadata line too long"));
    }

    Ok(serde_json::from_str(first_line).unwrap_or_default())
}
