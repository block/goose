use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use crate::message::Message;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use chrono::Local;

// The single app name used for all Goose applications
const APP_NAME: &str = "goose";

pub enum Identifier {
    Name(String),
    Path(PathBuf),
}

pub fn get_path(id: Identifier) -> PathBuf {
    match id {
        Identifier::Name(name) => {
            let session_dir = ensure_session_dir().expect("Failed to create session directory");
            session_dir.join(format!("{}.jsonl", name))
        }
        Identifier::Path(path) => path,
    }
}

/// Ensure the session directory exists and return its path
pub fn ensure_session_dir() -> Result<PathBuf> {
    let app_strategy = AppStrategyArgs {
        top_level_domain: "Block".to_string(),
        author: "Block".to_string(),
        app_name: APP_NAME.to_string(),
    };
    
    let data_dir = choose_app_strategy(app_strategy)
        .expect("goose requires a home dir")
        .data_dir()
        .join("sessions");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)?;
    }

    Ok(data_dir)
}

/// Get the path to the most recently modified session file
pub fn get_most_recent_session() -> Result<PathBuf> {
    let session_dir = ensure_session_dir()?;
    let mut entries = fs::read_dir(&session_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return Err(anyhow::anyhow!("No session files found"));
    }

    // Sort by modification time, most recent first
    entries.sort_by(|a, b| {
        b.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(
                &a.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            )
    });

    Ok(entries[0].path())
}

/// List all available session files
pub fn list_sessions() -> Result<Vec<(String, PathBuf)>> {
    let session_dir = ensure_session_dir()?;
    let entries = fs::read_dir(&session_dir)?
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

/// Generate a session ID using timestamp format (yyyymmdd_hhmmss)
pub fn generate_session_id() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}

/// Read messages from a session file
///
/// Creates the file if it doesn't exist, reads and deserializes all messages if it does.
pub fn read_messages(session_file: &Path) -> Result<Vec<Message>> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(session_file)?;

    let reader = io::BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        messages.push(serde_json::from_str::<Message>(&line?)?);
    }

    Ok(messages)
}

/// Write messages to a session file
///
/// Overwrites the file with all messages in JSONL format.
pub fn persist_messages(session_file: &Path, messages: &[Message]) -> Result<()> {
    let file = File::create(session_file).expect("The path specified does not exist");
    let mut writer = io::BufWriter::new(file);

    for message in messages {
        serde_json::to_writer(&mut writer, &message)?;
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use tempfile::tempdir;

    #[test]
    fn test_read_write_messages() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.jsonl");

        // Create some test messages
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
        ];

        // Write messages
        persist_messages(&file_path, &messages)?;

        // Read them back
        let read_messages = read_messages(&file_path)?;

        // Compare
        assert_eq!(messages.len(), read_messages.len());
        for (orig, read) in messages.iter().zip(read_messages.iter()) {
            assert_eq!(orig.role, read.role);
            assert_eq!(orig.content.len(), read.content.len());

            // Compare first text content
            if let (Some(MessageContent::Text(orig_text)), Some(MessageContent::Text(read_text))) =
                (orig.content.first(), read.content.first())
            {
                assert_eq!(orig_text.text, read_text.text);
            } else {
                panic!("Messages don't match expected structure");
            }
        }

        Ok(())
    }

    #[test]
    fn test_empty_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("empty.jsonl");

        // Reading an empty file should return empty vec
        let messages = read_messages(&file_path)?;
        assert!(messages.is_empty());

        Ok(())
    }

    #[test]
    fn test_generate_session_id() {
        let id = generate_session_id();
        
        // Check that it follows the timestamp format (yyyymmdd_hhmmss)
        assert_eq!(id.len(), 15); // 8 chars for date + 1 for underscore + 6 for time
        assert!(id.contains('_'));
        
        // Split by underscore and check parts
        let parts: Vec<&str> = id.split('_').collect();
        assert_eq!(parts.len(), 2);
        
        // Date part should be 8 digits
        assert_eq!(parts[0].len(), 8);
        // Time part should be 6 digits
        assert_eq!(parts[1].len(), 6);
    }
}