use crate::message::Message;
use crate::providers::base::Provider;
use anyhow::Result;
use chrono::Local;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn get_home_dir() -> PathBuf {
    choose_app_strategy(crate::config::APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .home_dir()
        .to_path_buf()
}

/// Metadata for a session, stored as the first line in the session file
#[derive(Debug, Clone, Serialize)]
pub struct SessionMetadata {
    /// Working directory for the session
    pub working_dir: PathBuf,
    /// A short description of the session, typically 3 words or less
    pub description: String,
    /// Number of messages in the session
    pub message_count: usize,
    /// The total number of tokens used in the session. Retrieved from the provider's last usage.
    pub total_tokens: Option<i32>,
}

// Custom deserializer to handle old sessions without working_dir
impl<'de> Deserialize<'de> for SessionMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            description: String,
            message_count: usize,
            total_tokens: Option<i32>,
            working_dir: Option<PathBuf>,
        }

        let helper = Helper::deserialize(deserializer)?;

        Ok(SessionMetadata {
            description: helper.description,
            message_count: helper.message_count,
            total_tokens: helper.total_tokens,
            working_dir: helper.working_dir.unwrap_or_else(get_home_dir),
        })
    }
}

impl SessionMetadata {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            description: String::new(),
            message_count: 0,
            total_tokens: None,
        }
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self::new(get_home_dir())
    }
}

// The single app name used for all Goose applications
const APP_NAME: &str = "goose";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// The first line of the file is expected to be metadata, and the rest are messages.
pub fn read_messages(session_file: &Path) -> Result<Vec<Message>> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(session_file)?;

    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();
    let mut messages = Vec::new();

    // Read the first line as metadata or create default if empty/missing
    if let Some(line) = lines.next() {
        let line = line?;
        // Try to parse as metadata, but if it fails, treat it as a message
        if let Ok(_metadata) = serde_json::from_str::<SessionMetadata>(&line) {
            // Metadata successfully parsed, continue with the rest of the lines as messages
        } else {
            // This is not metadata, it's a message
            messages.push(serde_json::from_str::<Message>(&line)?);
        }
    }

    // Read the rest of the lines as messages
    for line in lines {
        messages.push(serde_json::from_str::<Message>(&line?)?);
    }

    Ok(messages)
}

/// Read session metadata from a session file
///
/// Returns default empty metadata if the file doesn't exist or has no metadata.
pub fn read_metadata(session_file: &Path) -> Result<SessionMetadata> {
    if !session_file.exists() {
        return Ok(SessionMetadata::default());
    }

    let file = fs::File::open(session_file)?;
    let mut reader = io::BufReader::new(file);
    let mut first_line = String::new();

    // Read just the first line
    if reader.read_line(&mut first_line)? > 0 {
        // Try to parse as metadata
        match serde_json::from_str::<SessionMetadata>(&first_line) {
            Ok(metadata) => Ok(metadata),
            Err(_) => {
                // If the first line isn't metadata, return default
                Ok(SessionMetadata::default())
            }
        }
    } else {
        // Empty file, return default
        Ok(SessionMetadata::default())
    }
}

/// Write messages to a session file with metadata
///
/// Overwrites the file with metadata as the first line, followed by all messages in JSONL format.
/// If a provider is supplied, it will automatically generate a description when appropriate.
pub async fn persist_messages(
    session_file: &Path,
    messages: &[Message],
    provider: Option<Arc<Box<dyn Provider>>>,
) -> Result<()> {
    // Count user messages
    let user_message_count = messages
        .iter()
        .filter(|m| m.role == mcp_core::role::Role::User && !m.as_concat_text().trim().is_empty())
        .count();

    // Check if we need to update the description (after 1st or 3rd user message)
    match provider {
        Some(provider) if user_message_count < 4 => {
            //generate_description is responsible for writing the messages
            generate_description(session_file, messages, provider.as_ref().as_ref()).await
        }
        _ => {
            // Read existing metadata
            let metadata = read_metadata(session_file)?;
            // Write the file with metadata and messages
            save_messages_with_metadata(session_file, &metadata, messages)
        }
    }
}

/// Write messages to a session file with the provided metadata
///
/// Overwrites the file with metadata as the first line, followed by all messages in JSONL format.
pub fn save_messages_with_metadata(
    session_file: &Path,
    metadata: &SessionMetadata,
    messages: &[Message],
) -> Result<()> {
    let file = File::create(session_file).expect("The path specified does not exist");
    let mut writer = io::BufWriter::new(file);

    // Write metadata as the first line
    serde_json::to_writer(&mut writer, &metadata)?;
    writeln!(writer)?;

    // Write all messages
    for message in messages {
        serde_json::to_writer(&mut writer, &message)?;
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

/// Generate a description for the session using the provider
///
/// This function is called when appropriate to generate a short description
/// of the session based on the conversation history.
pub async fn generate_description(
    session_file: &Path,
    messages: &[Message],
    provider: &dyn Provider,
) -> Result<()> {
    // Create a special message asking for a 3-word description
    let mut description_prompt = "Based on the conversation so far, provide a concise description of this session in 4 words or less. This will be used for finding the session later in a UI with limited space - reply *ONLY* with the description".to_string();

    // get context from messages so far, limiting each message to 300 chars
    let context: Vec<String> = messages
        .iter()
        .filter(|m| m.role == mcp_core::role::Role::User)
        .take(3) // Use up to first 3 user messages for context
        .map(|m| m.as_concat_text())
        .collect();

    if !context.is_empty() {
        description_prompt = format!(
            "Here are the first few user messages:\n{}\n\n{}",
            context.join("\n"),
            description_prompt
        );
    }

    // Generate the description
    let message = Message::user().with_text(&description_prompt);
    let result = provider
        .complete(
            "Reply with only a description in four words or less",
            &[message],
            &[],
        )
        .await?;

    let description = result.0.as_concat_text();

    // Read current metadata
    let mut metadata = read_metadata(session_file)?;

    // Update description
    metadata.description = description;

    // Update the file with the new metadata and existing messages
    save_messages_with_metadata(session_file, &metadata, messages)
}

/// Update only the metadata in a session file, preserving all messages
pub async fn update_metadata(session_file: &Path, metadata: &SessionMetadata) -> Result<()> {
    // Read all messages from the file
    let messages = read_messages(session_file)?;

    // Rewrite the file with the new metadata and existing messages
    save_messages_with_metadata(session_file, metadata, &messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_write_messages() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.jsonl");

        // Create some test messages
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
        ];

        // Write messages
        persist_messages(&file_path, &messages, None).await?;

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
