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
use utoipa::ToSchema;

fn get_home_dir() -> PathBuf {
    choose_app_strategy(crate::config::APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .home_dir()
        .to_path_buf()
}

/// Metadata for a session, stored as the first line in the session file
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionMetadata {
    /// Working directory for the session
    #[schema(value_type = String, example = "/home/user/sessions/session1")]
    pub working_dir: PathBuf,
    /// A short description of the session, typically 3 words or less
    pub description: String,
    /// ID of the schedule that triggered this session, if any
    pub schedule_id: Option<String>,
    /// Number of messages in the session
    pub message_count: usize,
    /// The total number of tokens used in the session. Retrieved from the provider's last usage.
    pub total_tokens: Option<i32>,
    /// The number of input tokens used in the session. Retrieved from the provider's last usage.
    pub input_tokens: Option<i32>,
    /// The number of output tokens used in the session. Retrieved from the provider's last usage.
    pub output_tokens: Option<i32>,
    /// The total number of tokens used in the session. Accumulated across all messages (useful for tracking cost over an entire session).
    pub accumulated_total_tokens: Option<i32>,
    /// The number of input tokens used in the session. Accumulated across all messages.
    pub accumulated_input_tokens: Option<i32>,
    /// The number of output tokens used in the session. Accumulated across all messages.
    pub accumulated_output_tokens: Option<i32>,
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
            schedule_id: Option<String>, // For backward compatibility
            total_tokens: Option<i32>,
            input_tokens: Option<i32>,
            output_tokens: Option<i32>,
            accumulated_total_tokens: Option<i32>,
            accumulated_input_tokens: Option<i32>,
            accumulated_output_tokens: Option<i32>,
            working_dir: Option<PathBuf>,
        }

        let helper = Helper::deserialize(deserializer)?;

        // Get working dir, falling back to home if not specified or if specified dir doesn't exist
        let working_dir = helper
            .working_dir
            .filter(|path| path.exists())
            .unwrap_or_else(get_home_dir);

        Ok(SessionMetadata {
            description: helper.description,
            message_count: helper.message_count,
            schedule_id: helper.schedule_id,
            total_tokens: helper.total_tokens,
            input_tokens: helper.input_tokens,
            output_tokens: helper.output_tokens,
            accumulated_total_tokens: helper.accumulated_total_tokens,
            accumulated_input_tokens: helper.accumulated_input_tokens,
            accumulated_output_tokens: helper.accumulated_output_tokens,
            working_dir,
        })
    }
}

impl SessionMetadata {
    pub fn new(working_dir: PathBuf) -> Self {
        // If working_dir doesn't exist, fall back to home directory
        let working_dir = if !working_dir.exists() {
            get_home_dir()
        } else {
            working_dir
        };

        Self {
            working_dir,
            description: String::new(),
            schedule_id: None,
            message_count: 0,
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
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
/// Large messages are automatically truncated to prevent memory issues.
pub fn read_messages(session_file: &Path) -> Result<Vec<Message>> {
    read_messages_with_truncation(session_file, Some(50000)) // 50KB limit per message content
}

/// Read messages from a session file with optional content truncation
///
/// Creates the file if it doesn't exist, reads and deserializes all messages if it does.
/// The first line of the file is expected to be metadata, and the rest are messages.
/// If max_content_size is Some, large message content will be truncated during loading.
pub fn read_messages_with_truncation(
    session_file: &Path,
    max_content_size: Option<usize>,
) -> Result<Vec<Message>> {
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
            let message = parse_message_with_truncation(&line, max_content_size)?;
            messages.push(message);
        }
    }

    // Read the rest of the lines as messages
    for line in lines {
        let line = line?;
        let message = parse_message_with_truncation(&line, max_content_size)?;
        messages.push(message);
    }

    Ok(messages)
}

/// Parse a message from JSON string with optional content truncation
fn parse_message_with_truncation(
    json_str: &str,
    max_content_size: Option<usize>,
) -> Result<Message> {
    // First try to parse normally
    match serde_json::from_str::<Message>(json_str) {
        Ok(mut message) => {
            // If we have a size limit, check and truncate if needed
            if let Some(max_size) = max_content_size {
                truncate_message_content_in_place(&mut message, max_size);
            }
            Ok(message)
        }
        Err(e) => {
            // If parsing fails and the string is very long, it might be due to size
            if json_str.len() > 100000 {
                tracing::warn!(
                    "Failed to parse very large message ({}KB), attempting truncation",
                    json_str.len() / 1024
                );

                // Try to truncate the JSON string itself before parsing
                let truncated_json = if let Some(max_size) = max_content_size {
                    truncate_json_string(json_str, max_size)
                } else {
                    json_str.to_string()
                };

                match serde_json::from_str::<Message>(&truncated_json) {
                    Ok(message) => {
                        tracing::info!("Successfully parsed message after JSON truncation");
                        Ok(message)
                    }
                    Err(_) => {
                        tracing::error!("Failed to parse message even after truncation, skipping");
                        // Return a placeholder message indicating the issue
                        Ok(Message::user()
                            .with_text("[Message too large to load - content truncated]"))
                    }
                }
            } else {
                Err(e.into())
            }
        }
    }
}

/// Truncate content within a message in place
fn truncate_message_content_in_place(message: &mut Message, max_content_size: usize) {
    use crate::message::MessageContent;
    use mcp_core::{Content, ResourceContents};

    for content in &mut message.content {
        match content {
            MessageContent::Text(text_content) => {
                if text_content.text.len() > max_content_size {
                    let truncated = format!(
                        "{}\n\n[... content truncated during session loading from {} to {} characters ...]",
                        &text_content.text[..max_content_size.min(text_content.text.len())],
                        text_content.text.len(),
                        max_content_size
                    );
                    text_content.text = truncated;
                }
            }
            MessageContent::ToolResponse(tool_response) => {
                if let Ok(ref mut result) = tool_response.tool_result {
                    for content_item in result {
                        match content_item {
                            Content::Text(ref mut text_content) => {
                                if text_content.text.len() > max_content_size {
                                    let truncated = format!(
                                        "{}\n\n[... tool response truncated during session loading from {} to {} characters ...]",
                                        &text_content.text[..max_content_size.min(text_content.text.len())],
                                        text_content.text.len(),
                                        max_content_size
                                    );
                                    text_content.text = truncated;
                                }
                            }
                            Content::Resource(ref mut resource_content) => {
                                if let ResourceContents::TextResourceContents { text, .. } =
                                    &mut resource_content.resource
                                {
                                    if text.len() > max_content_size {
                                        let truncated = format!(
                                            "{}\n\n[... resource content truncated during session loading from {} to {} characters ...]",
                                            &text[..max_content_size.min(text.len())],
                                            text.len(),
                                            max_content_size
                                        );
                                        *text = truncated;
                                    }
                                }
                            }
                            _ => {} // Other content types are typically smaller
                        }
                    }
                }
            }
            _ => {} // Other content types are typically smaller
        }
    }
}

/// Attempt to truncate a JSON string by finding and truncating large text values
fn truncate_json_string(json_str: &str, max_content_size: usize) -> String {
    // This is a heuristic approach - look for large text values in the JSON
    // and truncate them. This is not perfect but should handle the common case
    // of large tool responses.

    if json_str.len() <= max_content_size * 2 {
        return json_str.to_string();
    }

    // Try to find patterns that look like large text content
    // Look for "text":"..." patterns and truncate the content
    let mut result = json_str.to_string();

    // Simple regex-like approach to find and truncate large text values
    if let Some(start) = result.find("\"text\":\"") {
        let text_start = start + 8; // Length of "text":"
        if let Some(end) = result[text_start..].find("\",") {
            let text_end = text_start + end;
            let text_content = &result[text_start..text_end];

            if text_content.len() > max_content_size {
                let truncated_text = format!(
                    "{}\n\n[... content truncated during JSON parsing from {} to {} characters ...]",
                    &text_content[..max_content_size.min(text_content.len())],
                    text_content.len(),
                    max_content_size
                );
                result.replace_range(text_start..text_end, &truncated_text);
            }
        }
    }

    result
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
    provider: Option<Arc<dyn Provider>>,
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
            generate_description(session_file, messages, provider).await
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
    provider: Arc<dyn Provider>,
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

    #[tokio::test]
    async fn test_special_characters_and_long_text() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("special.jsonl");

        // Insert some problematic JSON-like content between moderately long text
        // (keeping under truncation limit to test serialization/deserialization)
        let long_text = format!(
            "Start_of_message\n{}{}SOME_MIDDLE_TEXT{}End_of_message",
            "A".repeat(10_000), // Reduced from 100_000 to stay under 50KB limit
            "\"}]\n",
            "A".repeat(10_000) // Reduced from 100_000 to stay under 50KB limit
        );

        let special_chars = vec![
            // Long text
            long_text.as_str(),
            // Newlines in different positions
            "Line 1\nLine 2",
            "Line 1\r\nLine 2",
            "\nStart with newline",
            "End with newline\n",
            "\n\nMultiple\n\nNewlines\n\n",
            // JSON special characters
            "Quote\"in middle",
            "\"Quote at start",
            "Quote at end\"",
            "Multiple\"\"Quotes",
            "{\"json\": \"looking text\"}",
            // Unicode and special characters
            "Unicode: 🦆🤖👾",
            "Special: \\n \\r \\t",
            "Mixed: \n\"🦆\"\r\n\\n",
            // Control characters
            "Tab\there",
            "Bell\u{0007}char",
            "Null\u{0000}char",
            // Long text with mixed content
            "A very long message with multiple lines\nand \"quotes\"\nand emojis 🦆\nand \\escaped chars",
            // Potentially problematic JSON content
            "}{[]\",\\",
            "]}}\"\\n\\\"{[",
            "Edge case: } ] some text",
            "{\"foo\": \"} ]\"}",
            "}]",   
        ];

        let mut messages = Vec::new();
        for text in special_chars {
            messages.push(Message::user().with_text(text));
            messages.push(Message::assistant().with_text(text));
        }

        // Write messages with special characters
        persist_messages(&file_path, &messages, None).await?;

        // Read them back
        let read_messages = read_messages(&file_path)?;

        // Compare all messages
        assert_eq!(messages.len(), read_messages.len());
        for (i, (orig, read)) in messages.iter().zip(read_messages.iter()).enumerate() {
            assert_eq!(orig.role, read.role, "Role mismatch at message {}", i);
            assert_eq!(
                orig.content.len(),
                read.content.len(),
                "Content length mismatch at message {}",
                i
            );

            if let (Some(MessageContent::Text(orig_text)), Some(MessageContent::Text(read_text))) =
                (orig.content.first(), read.content.first())
            {
                assert_eq!(
                    orig_text.text, read_text.text,
                    "Text mismatch at message {}\nExpected: {}\nGot: {}",
                    i, orig_text.text, read_text.text
                );
            } else {
                panic!("Messages don't match expected structure at index {}", i);
            }
        }

        // Verify file format
        let contents = fs::read_to_string(&file_path)?;
        let lines: Vec<&str> = contents.lines().collect();

        // First line should be metadata
        assert!(
            lines[0].contains("\"description\""),
            "First line should be metadata"
        );

        // Each subsequent line should be valid JSON
        for (i, line) in lines.iter().enumerate().skip(1) {
            assert!(
                serde_json::from_str::<Message>(line).is_ok(),
                "Invalid JSON at line {}: {}",
                i + 1,
                line
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_large_content_truncation() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("large_content.jsonl");

        // Create a message with content larger than the 50KB truncation limit
        let very_large_text = "A".repeat(100_000); // 100KB of text
        let messages = vec![
            Message::user().with_text(&very_large_text),
            Message::assistant().with_text("Small response"),
        ];

        // Write messages
        persist_messages(&file_path, &messages, None).await?;

        // Read them back - should be truncated
        let read_messages = read_messages(&file_path)?;

        assert_eq!(messages.len(), read_messages.len());

        // First message should be truncated
        if let Some(MessageContent::Text(read_text)) = read_messages[0].content.first() {
            assert!(
                read_text.text.len() < very_large_text.len(),
                "Content should be truncated"
            );
            assert!(
                read_text
                    .text
                    .contains("content truncated during session loading"),
                "Should contain truncation notice"
            );
            assert!(
                read_text.text.starts_with("AAAA"),
                "Should start with original content"
            );
        } else {
            panic!("Expected text content in first message");
        }

        // Second message should be unchanged
        if let Some(MessageContent::Text(read_text)) = read_messages[1].content.first() {
            assert_eq!(read_text.text, "Small response");
        } else {
            panic!("Expected text content in second message");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_metadata_special_chars() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("metadata.jsonl");

        let mut metadata = SessionMetadata::default();
        metadata.description = "Description with\nnewline and \"quotes\" and 🦆".to_string();

        let messages = vec![Message::user().with_text("test")];

        // Write with special metadata
        save_messages_with_metadata(&file_path, &metadata, &messages)?;

        // Read back metadata
        let read_metadata = read_metadata(&file_path)?;
        assert_eq!(metadata.description, read_metadata.description);

        Ok(())
    }

    #[test]
    fn test_invalid_working_dir() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.jsonl");

        // Create metadata with non-existent directory
        let invalid_dir = PathBuf::from("/path/that/does/not/exist");
        let metadata = SessionMetadata::new(invalid_dir.clone());

        // Should fall back to home directory
        assert_ne!(metadata.working_dir, invalid_dir);
        assert_eq!(metadata.working_dir, get_home_dir());

        // Test deserialization of invalid directory
        let messages = vec![Message::user().with_text("test")];
        save_messages_with_metadata(&file_path, &metadata, &messages)?;

        // Modify the file to include invalid directory
        let contents = fs::read_to_string(&file_path)?;
        let mut lines: Vec<String> = contents.lines().map(String::from).collect();
        lines[0] = lines[0].replace(
            &get_home_dir().to_string_lossy().into_owned(),
            &invalid_dir.to_string_lossy().into_owned(),
        );
        fs::write(&file_path, lines.join("\n"))?;

        // Read back - should fall back to home dir
        let read_metadata = read_metadata(&file_path)?;
        assert_ne!(read_metadata.working_dir, invalid_dir);
        assert_eq!(read_metadata.working_dir, get_home_dir());

        Ok(())
    }
}
