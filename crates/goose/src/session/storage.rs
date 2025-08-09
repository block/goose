// IMPORTANT: This file includes session recovery functionality to handle corrupted session files.
// Only essential logging is included with the [SESSION] prefix to track:
// - Total message counts
// - Corruption detection and recovery
// - Backup creation
// Additional debug logging can be added if needed for troubleshooting.

use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::utils::safe_truncate;
use anyhow::Result;
use chrono::Local;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, Write};
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use utoipa::ToSchema;

// Security limits
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const MAX_MESSAGE_COUNT: usize = 5000;
const MAX_LINE_LENGTH: usize = 1024 * 1024; // 1MB per line

fn get_home_dir() -> PathBuf {
    choose_app_strategy(crate::config::APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .home_dir()
        .to_path_buf()
}

fn get_current_working_dir() -> PathBuf {
    std::env::current_dir()
        .or_else(|_| Ok::<PathBuf, io::Error>(get_home_dir()))
        .expect("could not determine the current working directory")
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
    /// ID of the project this session belongs to, if any
    pub project_id: Option<String>,
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
            project_id: Option<String>,  // For backward compatibility
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
            .unwrap_or_else(get_current_working_dir);

        Ok(SessionMetadata {
            description: helper.description,
            message_count: helper.message_count,
            schedule_id: helper.schedule_id,
            project_id: helper.project_id,
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
            project_id: None,
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
        Self::new(get_current_working_dir())
    }
}

// The single app name used for all Goose applications
const APP_NAME: &str = "goose";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Identifier {
    Name(String),
    Path(PathBuf),
}

pub fn get_path(id: Identifier) -> Result<PathBuf> {
    let path = match id {
        Identifier::Name(name) => {
            // Validate session name for security
            if name.is_empty() || name.len() > 255 {
                return Err(anyhow::anyhow!("Invalid session name length"));
            }

            // Check for path traversal attempts
            if name.contains("..") || name.contains('/') || name.contains('\\') {
                return Err(anyhow::anyhow!("Invalid characters in session name"));
            }

            let session_dir = ensure_session_dir().map_err(|e| {
                tracing::error!("Failed to create session directory: {}", e);
                anyhow::anyhow!("Failed to access session directory")
            })?;
            session_dir.join(format!("{}.jsonl", name))
        }
        Identifier::Path(path) => {
            // In test mode, allow temporary directory paths
            #[cfg(test)]
            {
                if let Some(path_str) = path.to_str() {
                    if path_str.contains("/tmp") || path_str.contains("/.tmp") {
                        // Allow test temporary directories
                        return Ok(path);
                    }
                }
            }

            // Validate that the path is within allowed directories
            let session_dir = ensure_session_dir().map_err(|e| {
                tracing::error!("Failed to create session directory: {}", e);
                anyhow::anyhow!("Failed to access session directory")
            })?;

            // Handle path validation with Windows-compatible logic
            let is_path_allowed = validate_path_within_session_dir(&path, &session_dir)?;
            if !is_path_allowed {
                tracing::warn!(
                    "Attempted access outside session directory: {:?} not within {:?}",
                    path,
                    session_dir
                );
                return Err(anyhow::anyhow!("Path not allowed"));
            }

            path
        }
    };

    // Additional security check for file extension (skip for special no-session paths)
    if let Some(ext) = path.extension() {
        if ext != "jsonl" {
            return Err(anyhow::anyhow!("Invalid file extension"));
        }
    }

    Ok(path)
}

/// Validate that a path is within the session directory, with Windows-compatible logic
///
/// This function handles Windows-specific path issues like:
/// - UNC path conversion during canonicalization
/// - Case sensitivity differences
/// - Path separator normalization
/// - Drive letter casing inconsistencies
fn validate_path_within_session_dir(path: &Path, session_dir: &Path) -> Result<bool> {
    // First, try the simple case - if canonicalization works cleanly
    if let (Ok(canonical_path), Ok(canonical_session_dir)) =
        (path.canonicalize(), session_dir.canonicalize())
    {
        if canonical_path.starts_with(&canonical_session_dir) {
            return Ok(true);
        }
    }

    // Fallback approach for Windows: normalize paths manually
    let normalized_path = normalize_path_for_comparison(path);
    let normalized_session_dir = normalize_path_for_comparison(session_dir);

    // Check if the normalized path starts with the normalized session directory
    if normalized_path.starts_with(&normalized_session_dir) {
        return Ok(true);
    }

    // Additional check: if the path doesn't exist yet, check its parent directory
    if !path.exists() {
        if let Some(parent) = path.parent() {
            return validate_path_within_session_dir(parent, session_dir);
        }
    }

    Ok(false)
}

/// Normalize a path for cross-platform comparison
///
/// This handles Windows-specific issues like:
/// - Converting to absolute paths
/// - Normalizing path separators
/// - Handling case sensitivity
fn normalize_path_for_comparison(path: &Path) -> PathBuf {
    // Try to canonicalize first, but fall back to absolute path if that fails
    let absolute_path = if let Ok(canonical) = path.canonicalize() {
        canonical
    } else if let Ok(absolute) = path.to_path_buf().canonicalize() {
        absolute
    } else {
        // Last resort: try to make it absolute manually
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            // If we can't make it absolute, use the current directory
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    };

    // On Windows, normalize the path representation
    #[cfg(windows)]
    {
        // Convert the path to components and rebuild it normalized
        let components: Vec<_> = absolute_path.components().collect();
        let mut normalized = PathBuf::new();

        for component in components {
            match component {
                std::path::Component::Prefix(prefix) => {
                    // Handle drive letters and UNC paths
                    let prefix_str = prefix.as_os_str().to_string_lossy();
                    if prefix_str.starts_with("\\\\?\\") {
                        // Remove UNC prefix and add the drive letter normally
                        let clean_prefix = &prefix_str[4..];
                        normalized.push(clean_prefix);
                    } else {
                        normalized.push(component);
                    }
                }
                std::path::Component::RootDir => {
                    normalized.push(component);
                }
                std::path::Component::CurDir | std::path::Component::ParentDir => {
                    // Skip these as they should be resolved by canonicalization
                    continue;
                }
                std::path::Component::Normal(name) => {
                    // Normalize case for Windows
                    let name_str = name.to_string_lossy().to_lowercase();
                    normalized.push(name_str);
                }
            }
        }

        normalized
    }

    #[cfg(not(windows))]
    {
        absolute_path
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

/// Read messages from a session file with corruption recovery
///
/// Creates the file if it doesn't exist, reads and deserializes all messages if it does.
/// The first line of the file is expected to be metadata, and the rest are messages.
/// Large messages are automatically truncated to prevent memory issues.
/// Includes recovery mechanisms for corrupted files.
///
/// Security features:
/// - Validates file paths to prevent directory traversal
/// - Includes all security limits from read_messages_with_truncation
pub fn read_messages(session_file: &Path) -> Result<Conversation> {
    // Validate the path for security
    let secure_path = get_path(Identifier::Path(session_file.to_path_buf()))?;

    let result = read_messages_with_truncation(&secure_path, Some(50000)); // 50KB limit per message content
    match &result {
        Ok(_messages) => {}
        Err(e) => println!(
            "[SESSION] Failed to read messages from {:?}: {}",
            secure_path, e
        ),
    }
    result
}

/// Read messages from a session file with optional content truncation and corruption recovery
///
/// Creates the file if it doesn't exist, reads and deserializes all messages if it does.
/// The first line of the file is expected to be metadata, and the rest are messages.
/// If max_content_size is Some, large message content will be truncated during loading.
/// Includes robust error handling and corruption recovery mechanisms.
///
/// Security features:
/// - File size limits to prevent resource exhaustion
/// - Message count limits to prevent DoS attacks
/// - Line length restrictions to prevent memory issues
pub fn read_messages_with_truncation(
    session_file: &Path,
    max_content_size: Option<usize>,
) -> Result<Conversation> {
    // Security check: file size limit
    if session_file.exists() {
        let metadata = fs::metadata(session_file)?;
        if metadata.len() > MAX_FILE_SIZE {
            tracing::warn!("Session file exceeds size limit: {} bytes", metadata.len());
            return Err(anyhow::anyhow!("Session file too large"));
        }
    }

    // Check if there's a backup file we should restore from
    let backup_file = session_file.with_extension("backup");
    if !session_file.exists() && backup_file.exists() {
        println!(
            "[SESSION] Session file missing but backup exists, restoring from backup: {:?}",
            backup_file
        );
        tracing::warn!(
            "Session file missing but backup exists, restoring from backup: {:?}",
            backup_file
        );
        if let Err(e) = fs::copy(&backup_file, session_file) {
            println!("[SESSION] Failed to restore from backup: {}", e);
            tracing::error!("Failed to restore from backup: {}", e);
        }
    }

    // Open the file with appropriate options
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(session_file)?;

    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();
    let mut messages = Vec::new();
    let mut corrupted_lines = Vec::new();
    let mut line_number = 1;
    let mut message_count = 0;

    // Read the first line as metadata or create default if empty/missing
    if let Some(line_result) = lines.next() {
        match line_result {
            Ok(line) => {
                // Security check: line length
                if line.len() > MAX_LINE_LENGTH {
                    tracing::warn!("Line {} exceeds length limit", line_number);
                    return Err(anyhow::anyhow!("Line too long"));
                }

                // Try to parse as metadata, but if it fails, treat it as a message
                if let Ok(_metadata) = serde_json::from_str::<SessionMetadata>(&line) {
                    // Metadata successfully parsed, continue with the rest of the lines as messages
                } else {
                    // This is not metadata, it's a message
                    match parse_message_with_truncation(&line, max_content_size) {
                        Ok(message) => {
                            messages.push(message);
                            message_count += 1;
                        }
                        Err(e) => {
                            println!("[SESSION] Failed to parse first line as message: {}", e);
                            println!("[SESSION] Attempting to recover corrupted first line...");
                            tracing::warn!("Failed to parse first line as message: {}", e);

                            // Try to recover the corrupted line
                            match attempt_corruption_recovery(&line, max_content_size) {
                                Ok(recovered) => {
                                    println!(
                                        "[SESSION] Successfully recovered corrupted first line!"
                                    );
                                    messages.push(recovered);
                                    message_count += 1;
                                }
                                Err(recovery_err) => {
                                    println!(
                                        "[SESSION] Failed to recover corrupted first line: {}",
                                        recovery_err
                                    );
                                    corrupted_lines.push((line_number, line));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("[SESSION] Failed to read first line: {}", e);
                tracing::error!("Failed to read first line: {}", e);
                corrupted_lines.push((line_number, "[Unreadable line]".to_string()));
            }
        }
        line_number += 1;
    }

    // Read the rest of the lines as messages
    for line_result in lines {
        // Security check: message count limit
        if message_count >= MAX_MESSAGE_COUNT {
            tracing::warn!("Message count limit reached: {}", MAX_MESSAGE_COUNT);
            println!(
                "[SESSION] Message count limit reached, stopping at {}",
                MAX_MESSAGE_COUNT
            );
            break;
        }

        match line_result {
            Ok(line) => {
                // Security check: line length
                if line.len() > MAX_LINE_LENGTH {
                    tracing::warn!("Line {} exceeds length limit", line_number);
                    corrupted_lines.push((
                        line_number,
                        "[Line too long - truncated for security]".to_string(),
                    ));
                    line_number += 1;
                    continue;
                }

                match parse_message_with_truncation(&line, max_content_size) {
                    Ok(message) => {
                        messages.push(message);
                        message_count += 1;
                    }
                    Err(e) => {
                        println!("[SESSION] Failed to parse line {}: {}", line_number, e);
                        println!(
                            "[SESSION] Attempting to recover corrupted line {}...",
                            line_number
                        );
                        tracing::warn!("Failed to parse line {}: {}", line_number, e);

                        // Try to recover the corrupted line
                        match attempt_corruption_recovery(&line, max_content_size) {
                            Ok(recovered) => {
                                println!(
                                    "[SESSION] Successfully recovered corrupted line {}!",
                                    line_number
                                );
                                messages.push(recovered);
                                message_count += 1;
                            }
                            Err(recovery_err) => {
                                println!(
                                    "[SESSION] Failed to recover corrupted line {}: {}",
                                    line_number, recovery_err
                                );
                                corrupted_lines.push((line_number, line));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("[SESSION] Failed to read line {}: {}", line_number, e);
                tracing::error!("Failed to read line {}: {}", line_number, e);
                corrupted_lines.push((line_number, "[Unreadable line]".to_string()));
            }
        }
        line_number += 1;
    }

    // If we found corrupted lines, create a backup and log the issues
    if !corrupted_lines.is_empty() {
        println!(
            "[SESSION] Found {} corrupted lines, creating backup",
            corrupted_lines.len()
        );
        tracing::warn!(
            "Found {} corrupted lines in session file, creating backup",
            corrupted_lines.len()
        );

        // Create a backup of the original file
        if !backup_file.exists() {
            if let Err(e) = fs::copy(session_file, &backup_file) {
                println!("[SESSION] Failed to create backup file: {}", e);
                tracing::error!("Failed to create backup file: {}", e);
            } else {
                println!("[SESSION] Created backup file: {:?}", backup_file);
                tracing::info!("Created backup file: {:?}", backup_file);
            }
        }

        // Log details about corrupted lines (with limited detail for security)
        for (num, line) in &corrupted_lines {
            let preview = if line.len() > 50 {
                format!("{}... (truncated)", safe_truncate(line, 50))
            } else {
                line.clone()
            };
            tracing::debug!("Corrupted line {}: {}", num, preview);
        }
    }

    Ok(Conversation::new_unvalidated(messages))
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
        Err(_e) => {
            // If parsing fails and the string is very long, it might be due to size
            if json_str.len() > 100000 {
                println!(
                    "[SESSION] Very large message detected ({}KB), attempting truncation",
                    json_str.len() / 1024
                );
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
                        println!(
                            "[SESSION] Failed to parse even after truncation, attempting recovery"
                        );
                        tracing::error!("Failed to parse message even after truncation");
                        attempt_corruption_recovery(json_str, max_content_size)
                    }
                }
            } else {
                // Try intelligent corruption recovery
                attempt_corruption_recovery(json_str, max_content_size)
            }
        }
    }
}

/// Truncate content within a message in place
fn truncate_message_content_in_place(message: &mut Message, max_content_size: usize) {
    use crate::conversation::message::MessageContent;
    use rmcp::model::{RawContent, ResourceContents};

    for content in &mut message.content {
        match content {
            MessageContent::Text(text_content) => {
                if text_content.text.chars().count() > max_content_size {
                    let truncated = format!(
                        "{}\n\n[... content truncated during session loading from {} to {} characters ...]",
                        safe_truncate(&text_content.text, max_content_size),
                        text_content.text.chars().count(),
                        max_content_size
                    );
                    text_content.text = truncated;
                }
            }
            MessageContent::ToolResponse(tool_response) => {
                if let Ok(ref mut result) = tool_response.tool_result {
                    for content_item in result {
                        match content_item.deref_mut() {
                            RawContent::Text(ref mut text_content) => {
                                if text_content.text.chars().count() > max_content_size {
                                    let truncated = format!(
                                        "{}\n\n[... tool response truncated during session loading from {} to {} characters ...]",
                                        safe_truncate(&text_content.text, max_content_size),
                                        text_content.text.chars().count(),
                                        max_content_size
                                    );
                                    text_content.text = truncated;
                                }
                            }
                            RawContent::Resource(ref mut resource_content) => {
                                if let ResourceContents::TextResourceContents { text, .. } =
                                    &mut resource_content.resource
                                {
                                    if text.chars().count() > max_content_size {
                                        let truncated = format!(
                                            "{}\n\n[... resource content truncated during session loading from {} to {} characters ...]",
                                            safe_truncate(text, max_content_size),
                                            text.chars().count(),
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

/// Attempt to recover corrupted JSON lines using various strategies
fn attempt_corruption_recovery(json_str: &str, max_content_size: Option<usize>) -> Result<Message> {
    // Strategy 1: Try to fix common JSON corruption issues
    if let Ok(message) = try_fix_json_corruption(json_str, max_content_size) {
        println!("[SESSION] Recovered using JSON corruption fix");
        return Ok(message);
    }

    // Strategy 2: Try to extract partial content if it looks like a message
    if let Ok(message) = try_extract_partial_message(json_str) {
        println!("[SESSION] Recovered using partial message extraction");
        return Ok(message);
    }

    // Strategy 3: Try to fix truncated JSON
    if let Ok(message) = try_fix_truncated_json(json_str, max_content_size) {
        println!("[SESSION] Recovered using truncated JSON fix");
        return Ok(message);
    }

    // Strategy 4: Create a placeholder message with the raw content
    println!("[SESSION] All recovery strategies failed, creating placeholder message");
    let preview = if json_str.len() > 200 {
        format!("{}...", safe_truncate(json_str, 200))
    } else {
        json_str.to_string()
    };

    Ok(Message::user().with_text(format!(
        "[RECOVERED FROM CORRUPTED LINE]\nOriginal content preview: {}\n\n[This message was recovered from a corrupted session file line. The original data may be incomplete.]",
        preview
    )))
}

/// Try to fix common JSON corruption patterns
fn try_fix_json_corruption(json_str: &str, max_content_size: Option<usize>) -> Result<Message> {
    let mut fixed_json = json_str.to_string();
    let mut fixes_applied = Vec::new();

    // Fix 1: Remove trailing commas before closing braces/brackets
    if fixed_json.contains(",}") || fixed_json.contains(",]") {
        fixed_json = fixed_json.replace(",}", "}").replace(",]", "]");
        fixes_applied.push("trailing commas");
    }

    // Fix 2: Try to close unclosed quotes in text fields
    if let Some(text_start) = fixed_json.find("\"text\":\"") {
        let content_start = text_start + 8;
        if let Some(remaining) = fixed_json.get(content_start..) {
            // Count quotes to see if we have an odd number (unclosed quote)
            let quote_count = remaining.matches('"').count();
            if quote_count % 2 == 1 {
                // Find the last quote and see if we need to close it
                if let Some(last_quote_pos) = remaining.rfind('"') {
                    let after_last_quote = &remaining[last_quote_pos + 1..];
                    if !after_last_quote.trim_start().starts_with(',')
                        && !after_last_quote.trim_start().starts_with('}')
                    {
                        // Insert a closing quote before the next field or end
                        if let Some(next_field) = after_last_quote.find(',') {
                            fixed_json.insert(content_start + last_quote_pos + 1 + next_field, '"');
                            fixes_applied.push("unclosed quotes");
                        } else if after_last_quote.contains('}') {
                            if let Some(brace_pos) = after_last_quote.find('}') {
                                fixed_json
                                    .insert(content_start + last_quote_pos + 1 + brace_pos, '"');
                                fixes_applied.push("unclosed quotes");
                            }
                        }
                    }
                }
            }
        }
    }

    // Fix 3: Try to close unclosed JSON objects/arrays
    let open_braces = fixed_json.matches('{').count();
    let close_braces = fixed_json.matches('}').count();
    let open_brackets = fixed_json.matches('[').count();
    let close_brackets = fixed_json.matches(']').count();

    if open_braces > close_braces {
        for _ in 0..(open_braces - close_braces) {
            fixed_json.push('}');
        }
        fixes_applied.push("unclosed braces");
    }

    if open_brackets > close_brackets {
        for _ in 0..(open_brackets - close_brackets) {
            fixed_json.push(']');
        }
        fixes_applied.push("unclosed brackets");
    }

    // Fix 4: Remove control characters that might break JSON parsing
    let original_len = fixed_json.len();
    fixed_json = fixed_json
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect();
    if fixed_json.len() != original_len {
        fixes_applied.push("control characters");
    }

    if !fixes_applied.is_empty() {
        match serde_json::from_str::<Message>(&fixed_json) {
            Ok(mut message) => {
                if let Some(max_size) = max_content_size {
                    truncate_message_content_in_place(&mut message, max_size);
                }
                return Ok(message);
            }
            Err(e) => {
                println!("[SESSION] JSON fixes didn't work: {}", e);
            }
        }
    }

    Err(anyhow::anyhow!("JSON corruption fixes failed"))
}

/// Try to extract a partial message from corrupted JSON
fn try_extract_partial_message(json_str: &str) -> Result<Message> {
    // Look for recognizable patterns that indicate this was a message

    // Try to extract role
    let role = if json_str.contains("\"role\":\"user\"") {
        rmcp::model::Role::User
    } else if json_str.contains("\"role\":\"assistant\"") {
        rmcp::model::Role::Assistant
    } else {
        rmcp::model::Role::User // Default fallback
    };

    // Try to extract text content
    let mut extracted_text = String::new();

    // Look for text field content
    if let Some(text_start) = json_str.find("\"text\":\"") {
        let content_start = text_start + 8;
        if let Some(content_end) = json_str[content_start..].find("\",") {
            extracted_text = json_str[content_start..content_start + content_end].to_string();
        } else if let Some(content_end) = json_str[content_start..].find("\"") {
            extracted_text = json_str[content_start..content_start + content_end].to_string();
        } else {
            // Take everything after "text":" until we hit a likely end
            let remaining = &json_str[content_start..];
            if let Some(end_pos) = remaining.find('}') {
                extracted_text = remaining[..end_pos].trim_end_matches('"').to_string();
            } else {
                extracted_text = remaining.to_string();
            }
        }
    }

    // If we couldn't extract text, try to find any readable content
    if extracted_text.is_empty() {
        // Look for any quoted strings that might be content
        let quote_pattern = Regex::new(r#""([^"]{10,})""#).unwrap();
        if let Some(captures) = quote_pattern.find(json_str) {
            extracted_text = captures.as_str().trim_matches('"').to_string();
        }
    }

    if !extracted_text.is_empty() {
        let message = match role {
            rmcp::model::Role::User => Message::user(),
            rmcp::model::Role::Assistant => Message::assistant(),
        };

        return Ok(message.with_text(format!("[PARTIALLY RECOVERED] {}", extracted_text)));
    }

    Err(anyhow::anyhow!("Could not extract partial message"))
}

/// Try to fix truncated JSON by completing it
fn try_fix_truncated_json(json_str: &str, max_content_size: Option<usize>) -> Result<Message> {
    let mut completed_json = json_str.to_string();

    // If the JSON appears to be cut off mid-field, try to complete it
    if !completed_json.trim().ends_with('}') && !completed_json.trim().ends_with(']') {
        // Try to find where it was likely cut off
        if let Some(last_quote) = completed_json.rfind('"') {
            let after_quote = &completed_json[last_quote + 1..];
            if !after_quote.contains('"') && !after_quote.contains('}') {
                // Looks like it was cut off in the middle of a string value
                completed_json.push('"');

                // Try to close the JSON structure
                let open_braces = completed_json.matches('{').count();
                let close_braces = completed_json.matches('}').count();

                for _ in 0..(open_braces - close_braces) {
                    completed_json.push('}');
                }

                match serde_json::from_str::<Message>(&completed_json) {
                    Ok(mut message) => {
                        if let Some(max_size) = max_content_size {
                            truncate_message_content_in_place(&mut message, max_size);
                        }
                        return Ok(message);
                    }
                    Err(e) => {
                        println!("[SESSION] Truncation fix didn't work: {}", e);
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("Truncation fix failed"))
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
                    safe_truncate(text_content, max_content_size),
                    text_content.len(),
                    max_content_size
                );
                result.replace_range(text_start..text_end, &truncated_text);
            }
        }
    }

    result
}

/// Read session metadata from a session file with security validation
///
/// Returns default empty metadata if the file doesn't exist or has no metadata.
/// Includes security checks for file access and content validation.
pub fn read_metadata(session_file: &Path) -> Result<SessionMetadata> {
    // Validate the path for security
    let secure_path = get_path(Identifier::Path(session_file.to_path_buf()))?;

    if !secure_path.exists() {
        return Ok(SessionMetadata::default());
    }

    // Security check: file size
    let file_metadata = fs::metadata(&secure_path)?;
    if file_metadata.len() > MAX_FILE_SIZE {
        tracing::warn!("Session file exceeds size limit during metadata read");
        return Err(anyhow::anyhow!("Session file too large"));
    }

    let file = fs::File::open(&secure_path).map_err(|e| {
        tracing::error!("Failed to open session file for metadata read: {}", e);
        anyhow::anyhow!("Failed to access session file")
    })?;
    let mut reader = io::BufReader::new(file);
    let mut first_line = String::new();

    // Read just the first line
    if reader.read_line(&mut first_line)? > 0 {
        // Security check: line length
        if first_line.len() > MAX_LINE_LENGTH {
            tracing::warn!("Metadata line exceeds length limit");
            return Err(anyhow::anyhow!("Metadata line too long"));
        }

        // Try to parse as metadata
        match serde_json::from_str::<SessionMetadata>(&first_line) {
            Ok(metadata) => Ok(metadata),
            Err(e) => {
                // If the first line isn't metadata, return default
                tracing::debug!("Metadata parse error: {}", e);
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
///
/// Security features:
/// - Validates file paths to prevent directory traversal
pub async fn persist_messages(
    session_file: &Path,
    messages: &Conversation,
    provider: Option<Arc<dyn Provider>>,
    working_dir: Option<PathBuf>,
) -> Result<()> {
    persist_messages_with_schedule_id(session_file, messages, provider, None, working_dir).await
}

/// Write messages to a session file with metadata, including an optional scheduled job ID
///
/// Overwrites the file with metadata as the first line, followed by all messages in JSONL format.
/// If a provider is supplied, it will automatically generate a description when appropriate.
///
/// Security features:
/// - Validates file paths to prevent directory traversal
/// - Limits error message details in logs
/// - Uses atomic file operations via save_messages_with_metadata
pub async fn persist_messages_with_schedule_id(
    session_file: &Path,
    messages: &Conversation,
    provider: Option<Arc<dyn Provider>>,
    schedule_id: Option<String>,
    working_dir: Option<PathBuf>,
) -> Result<()> {
    // Validate the session file path for security
    let secure_path = get_path(Identifier::Path(session_file.to_path_buf()))?;

    // Security check: message count limit
    if messages.len() > MAX_MESSAGE_COUNT {
        tracing::warn!("Message count exceeds limit: {}", messages.len());
        return Err(anyhow::anyhow!("Too many messages"));
    }

    // Count user messages
    let user_message_count = messages
        .iter()
        .filter(|m| m.role == rmcp::model::Role::User && !m.as_concat_text().trim().is_empty())
        .count();

    // Check if we need to update the description (after 1st or 3rd user message)
    match provider {
        Some(provider) if user_message_count < 4 => {
            //generate_description is responsible for writing the messages
            generate_description_with_schedule_id(
                &secure_path,
                messages,
                provider,
                schedule_id,
                working_dir,
            )
            .await
        }
        _ => {
            // Read existing metadata or create new with proper working_dir
            let mut metadata = if secure_path.exists() {
                read_metadata(&secure_path)?
            } else {
                // Create new metadata with the provided working_dir or fall back to home
                let work_dir = working_dir.clone().unwrap_or_else(get_home_dir);
                SessionMetadata::new(work_dir)
            };

            // Update the working_dir if provided (even for existing files)
            if let Some(work_dir) = working_dir {
                metadata.working_dir = work_dir;
            }

            // Update the schedule_id if provided
            if schedule_id.is_some() {
                metadata.schedule_id = schedule_id;
            }

            // Write the file with metadata and messages
            save_messages_with_metadata(&secure_path, &metadata, messages)
        }
    }
}

/// Write messages to a session file with the provided metadata using secure atomic operations
///
/// This function uses atomic file operations to prevent corruption:
/// 1. Writes to a temporary file first with secure permissions
/// 2. Uses fs2 file locking to prevent concurrent writes
/// 3. Atomically moves the temp file to the final location
/// 4. Includes comprehensive error handling and recovery
///
/// Security features:
/// - Secure temporary file creation with restricted permissions
/// - Path validation to prevent directory traversal
/// - File size and message count limits
/// - Sanitized error messages to prevent information leakage
pub fn save_messages_with_metadata(
    session_file: &Path,
    metadata: &SessionMetadata,
    messages: &Conversation,
) -> Result<()> {
    use fs2::FileExt;

    // Validate the path for security
    let secure_path = get_path(Identifier::Path(session_file.to_path_buf()))?;

    // Security check: message count limit
    if messages.len() > MAX_MESSAGE_COUNT {
        tracing::warn!(
            "Message count exceeds limit during save: {}",
            messages.len()
        );
        return Err(anyhow::anyhow!("Too many messages to save"));
    }

    // Create a temporary file in the same directory to ensure atomic move
    let temp_file = secure_path.with_extension("tmp");

    // Ensure the parent directory exists
    if let Some(parent) = secure_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            tracing::error!("Failed to create parent directory: {}", e);
            anyhow::anyhow!("Failed to create session directory")
        })?;
    }

    // Create and lock the temporary file with secure permissions
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_file)
        .map_err(|e| {
            tracing::error!("Failed to create temporary file: {}", e);
            anyhow::anyhow!("Failed to create temporary session file")
        })?;

    // Set secure file permissions (Unix only - read/write for owner only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600); // rw-------
        fs::set_permissions(&temp_file, perms).map_err(|e| {
            tracing::error!("Failed to set secure file permissions: {}", e);
            anyhow::anyhow!("Failed to secure temporary file")
        })?;
    }

    // Get an exclusive lock on the file
    file.try_lock_exclusive().map_err(|e| {
        tracing::error!("Failed to lock file: {}", e);
        anyhow::anyhow!("Failed to lock session file")
    })?;

    // Write to temporary file
    {
        let mut writer = io::BufWriter::new(&file);

        // Write metadata as the first line
        serde_json::to_writer(&mut writer, &metadata).map_err(|e| {
            tracing::error!("Failed to serialize metadata: {}", e);
            anyhow::anyhow!("Failed to write session metadata")
        })?;
        writeln!(writer)?;

        // Write all messages with progress tracking
        for (i, message) in messages.iter().enumerate() {
            serde_json::to_writer(&mut writer, &message).map_err(|e| {
                tracing::error!("Failed to serialize message {}: {}", i, e);
                anyhow::anyhow!("Failed to write session message")
            })?;
            writeln!(writer)?;
        }

        // Ensure all data is written to disk
        writer.flush().map_err(|e| {
            tracing::error!("Failed to flush writer: {}", e);
            anyhow::anyhow!("Failed to flush session data")
        })?;
    }

    // Sync to ensure data is persisted
    file.sync_all().map_err(|e| {
        tracing::error!("Failed to sync data: {}", e);
        anyhow::anyhow!("Failed to sync session data")
    })?;

    // Release the lock
    fs2::FileExt::unlock(&file).map_err(|e| {
        tracing::error!("Failed to unlock file: {}", e);
        anyhow::anyhow!("Failed to unlock session file")
    })?;

    // Atomically move the temporary file to the final location
    fs::rename(&temp_file, &secure_path).map_err(|e| {
        // Clean up temp file on failure
        tracing::error!("Failed to move temporary file: {}", e);
        let _ = fs::remove_file(&temp_file);
        anyhow::anyhow!("Failed to finalize session file")
    })?;

    tracing::debug!("Successfully saved session file: {:?}", secure_path);
    Ok(())
}

/// Generate a description for the session using the provider
///
/// This function is called when appropriate to generate a short description
/// of the session based on the conversation history.
pub async fn generate_description(
    session_file: &Path,
    messages: &Conversation,
    provider: Arc<dyn Provider>,
    working_dir: Option<PathBuf>,
) -> Result<()> {
    generate_description_with_schedule_id(session_file, messages, provider, None, working_dir).await
}

/// Generate a description for the session using the provider, including an optional scheduled job ID and working directory
///
/// This function is called when appropriate to generate a short description
/// of the session based on the conversation history.
///
/// Security features:
/// - Validates file paths to prevent directory traversal
/// - Limits context size to prevent resource exhaustion
/// - Uses secure file operations for saving
pub async fn generate_description_with_schedule_id(
    session_file: &Path,
    messages: &Conversation,
    provider: Arc<dyn Provider>,
    schedule_id: Option<String>,
    working_dir: Option<PathBuf>,
) -> Result<()> {
    // Validate the path for security
    let secure_path = get_path(Identifier::Path(session_file.to_path_buf()))?;

    // Security check: message count limit
    if messages.len() > MAX_MESSAGE_COUNT {
        tracing::warn!(
            "Message count exceeds limit during description generation: {}",
            messages.len()
        );
        return Err(anyhow::anyhow!(
            "Too many messages for description generation"
        ));
    }

    // Use the provider's session naming capability
    let sanitized_description = provider
        .generate_session_name(messages)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate session description: {}", e);
            anyhow::anyhow!("Failed to generate session description")
        })?;

    // Create metadata with proper working_dir or read existing and update
    let mut metadata = if secure_path.exists() {
        read_metadata(&secure_path)?
    } else {
        // Create new metadata with the provided working_dir or fall back to home
        let work_dir = working_dir.clone().unwrap_or_else(get_home_dir);
        SessionMetadata::new(work_dir)
    };

    // Update description and schedule_id
    metadata.description = sanitized_description;
    if schedule_id.is_some() {
        metadata.schedule_id = schedule_id;
    }

    // Update the working_dir if provided (even for existing files)
    if let Some(work_dir) = working_dir {
        metadata.working_dir = work_dir;
    }

    // Update the file with the new metadata and existing messages
    save_messages_with_metadata(&secure_path, &metadata, messages)
}

/// Update only the metadata in a session file, preserving all messages
///
/// Security features:
/// - Validates file paths to prevent directory traversal
/// - Uses secure file operations for reading and writing
pub async fn update_metadata(session_file: &Path, metadata: &SessionMetadata) -> Result<()> {
    // Validate the path for security
    let secure_path = get_path(Identifier::Path(session_file.to_path_buf()))?;

    // Read all messages from the file
    let messages = read_messages(&secure_path)?;

    // Rewrite the file with the new metadata and existing messages
    save_messages_with_metadata(&secure_path, metadata, &messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent};
    use tempfile::tempdir;

    #[test]
    fn test_corruption_recovery() -> Result<()> {
        let test_cases = vec![
            // Case 1: Unclosed quotes
            (
                r#"{"role":"user","content":[{"type":"text","text":"Hello there}]"#,
                "Unclosed JSON with truncated content",
            ),
            // Case 2: Trailing comma
            (
                r#"{"role":"user","content":[{"type":"text","text":"Test"},]}"#,
                "JSON with trailing comma",
            ),
            // Case 3: Missing closing brace
            (
                r#"{"role":"user","content":[{"type":"text","text":"Test""#,
                "Incomplete JSON structure",
            ),
            // Case 4: Control characters in text
            (
                r#"{"role":"user","content":[{"type":"text","text":"Test\u{0000}with\u{0001}control\u{0002}chars"}]}"#,
                "JSON with control characters",
            ),
            // Case 5: Partial message with role and text
            (
                r#"broken{"role": "assistant", "text": "This is recoverable content"more broken"#,
                "Partial message with recoverable content",
            ),
        ];

        println!("[TEST] Starting corruption recovery tests...");
        for (i, (corrupt_json, desc)) in test_cases.iter().enumerate() {
            println!("\n[TEST] Case {}: {}", i + 1, desc);
            println!(
                "[TEST] Input: {}",
                if corrupt_json.len() > 100 {
                    safe_truncate(corrupt_json, 100)
                } else {
                    corrupt_json.to_string()
                }
            );

            // Try to parse the corrupted JSON
            match attempt_corruption_recovery(corrupt_json, Some(50000)) {
                Ok(message) => {
                    println!("[TEST] Successfully recovered message");
                    // Verify we got some content
                    if let Some(MessageContent::Text(text_content)) = message.content.first() {
                        assert!(
                            !text_content.text.is_empty(),
                            "Recovered message should have content"
                        );
                        println!(
                            "[TEST] Recovered content: {}",
                            if text_content.text.len() > 50 {
                                format!("{}...", &text_content.text[..50])
                            } else {
                                text_content.text.clone()
                            }
                        );
                    }
                }
                Err(e) => {
                    println!("[TEST] Failed to recover: {}", e);
                    panic!("Failed to recover from case {}: {}", i + 1, desc);
                }
            }
        }

        println!("\n[TEST] All corruption recovery tests passed!");
        Ok(())
    }

    #[tokio::test]
    async fn test_read_write_messages() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.jsonl");

        // Create some test messages
        let messages = Conversation::new_unvalidated(vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
        ]);

        // Write messages
        persist_messages(&file_path, &messages, None, None).await?;

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

        let mut messages = Conversation::empty();
        for text in special_chars {
            messages.push(Message::user().with_text(text));
            messages.push(Message::assistant().with_text(text));
        }

        // Write messages with special characters
        persist_messages(&file_path, &messages, None, None).await?;

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
        let messages = Conversation::new_unvalidated(vec![
            Message::user().with_text(&very_large_text),
            Message::assistant().with_text("Small response"),
        ]);

        // Write messages
        persist_messages(&file_path, &messages, None, None).await?;

        // Read them back - should be truncated
        let read_messages = read_messages(&file_path)?;

        assert_eq!(messages.len(), read_messages.len());

        // First message should be truncated
        if let Some(MessageContent::Text(read_text)) =
            read_messages.first().unwrap().content.first()
        {
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
        if let Some(MessageContent::Text(read_text)) = read_messages.messages()[1].content.first() {
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

        let messages = Conversation::new_unvalidated(vec![Message::user().with_text("test")]);

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
        let messages = Conversation::new_unvalidated(vec![Message::user().with_text("test")]);
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
        assert_eq!(read_metadata.working_dir, get_current_working_dir());

        Ok(())
    }

    #[tokio::test]
    async fn test_working_dir_preservation() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.jsonl");

        // Create a temporary working directory
        let working_dir = tempdir()?;
        let working_dir_path = working_dir.path().to_path_buf();

        // Create messages
        let messages =
            Conversation::new_unvalidated(vec![Message::user().with_text("test message")]);

        // Use persist_messages_with_schedule_id to set working dir
        persist_messages_with_schedule_id(
            &file_path,
            &messages,
            None,
            None,
            Some(working_dir_path.clone()),
        )
        .await?;

        // Read back the metadata and verify working_dir is preserved
        let metadata = read_metadata(&file_path)?;
        assert_eq!(metadata.working_dir, working_dir_path);

        // Verify the messages are also preserved
        let read_messages = read_messages(&file_path)?;
        assert_eq!(read_messages.len(), 1);
        assert_eq!(
            read_messages.first().unwrap().role,
            messages.messages()[0].role
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_working_dir_issue_fixed() -> Result<()> {
        // This test demonstrates that the working_dir issue in jsonl files is fixed
        let dir = tempdir()?;
        let file_path = dir.path().join("test.jsonl");

        // Create a temporary working directory (this simulates the actual working directory)
        let working_dir = tempdir()?;
        let working_dir_path = working_dir.path().to_path_buf();

        // Create messages
        let messages =
            Conversation::new_unvalidated(vec![Message::user().with_text("test message")]);

        // Get the home directory for comparison
        let home_dir = get_home_dir();

        // Test 1: Using the old persist_messages function (without working_dir)
        // This will fall back to home directory since no working_dir is provided
        persist_messages(&file_path, &messages, None, None).await?;

        // Read back the metadata - this should now have the home directory as working_dir
        let metadata_old = read_metadata(&file_path)?;
        assert_eq!(
            metadata_old.working_dir, home_dir,
            "persist_messages should use home directory when no working_dir is provided"
        );

        // Test 2: Using persist_messages_with_schedule_id function
        // This should properly set the working_dir (this is the main fix)
        persist_messages_with_schedule_id(
            &file_path,
            &messages,
            None,
            None,
            Some(working_dir_path.clone()),
        )
        .await?;

        // Read back the metadata - this should now have the correct working_dir
        let metadata_new = read_metadata(&file_path)?;
        assert_eq!(
            metadata_new.working_dir, working_dir_path,
            "persist_messages_with_schedule_id should use provided working_dir"
        );
        assert_ne!(
            metadata_new.working_dir, home_dir,
            "working_dir should be different from home directory"
        );

        // Test 3: Create a new session file without working_dir (should fall back to home)
        let file_path_2 = dir.path().join("test2.jsonl");
        persist_messages_with_schedule_id(
            &file_path_2,
            &messages,
            None,
            None,
            None, // No working_dir provided
        )
        .await?;

        let metadata_fallback = read_metadata(&file_path_2)?;
        assert_eq!(metadata_fallback.working_dir, home_dir, "persist_messages_with_schedule_id should fall back to home directory when no working_dir is provided");

        // Test 4: Test that the fix works for existing files
        // Create a session file and then add to it with different working_dir
        let file_path_3 = dir.path().join("test3.jsonl");

        // First, create with home directory
        persist_messages(&file_path_3, &messages, None, None).await?;
        let metadata_initial = read_metadata(&file_path_3)?;
        assert_eq!(
            metadata_initial.working_dir, home_dir,
            "Initial session should use home directory"
        );

        // Then update with a specific working_dir
        persist_messages_with_schedule_id(
            &file_path_3,
            &messages,
            None,
            None,
            Some(working_dir_path.clone()),
        )
        .await?;

        let metadata_updated = read_metadata(&file_path_3)?;
        assert_eq!(
            metadata_updated.working_dir, working_dir_path,
            "Updated session should use new working_dir"
        );

        // Test 5: Most important test - simulate the real-world scenario where
        // CLI and web interfaces pass the current directory instead of None
        let file_path_4 = dir.path().join("test4.jsonl");
        let current_dir = std::env::current_dir()?;

        // This is what web.rs and session/mod.rs do now after the fix
        persist_messages_with_schedule_id(
            &file_path_4,
            &messages,
            None,
            None,
            Some(current_dir.clone()),
        )
        .await?;

        let metadata_current = read_metadata(&file_path_4)?;
        assert_eq!(
            metadata_current.working_dir, current_dir,
            "Session should use current directory when explicitly provided"
        );
        // This should NOT be the home directory anymore (unless current_dir == home_dir)
        if current_dir != home_dir {
            assert_ne!(
                metadata_current.working_dir, home_dir,
                "working_dir should be different from home directory when current_dir is different"
            );
        }

        Ok(())
    }

    #[test]
    fn test_windows_path_validation() -> Result<()> {
        // Test the Windows path validation logic
        let temp_dir = tempfile::tempdir()?;
        let session_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&session_dir)?;

        // Test case 1: Valid path within session directory
        let valid_path = session_dir.join("test.jsonl");
        assert!(validate_path_within_session_dir(&valid_path, &session_dir)?);

        // Test case 2: Invalid path outside session directory
        let invalid_path = temp_dir.path().join("outside.jsonl");
        assert!(!validate_path_within_session_dir(
            &invalid_path,
            &session_dir
        )?);

        // Test case 3: Path with different separators (simulate Windows issue)
        let mixed_sep_path = session_dir.join("subdir").join("test.jsonl");
        fs::create_dir_all(mixed_sep_path.parent().unwrap())?;
        assert!(validate_path_within_session_dir(
            &mixed_sep_path,
            &session_dir
        )?);

        // Test case 4: Non-existent path within session directory
        let nonexistent_path = session_dir.join("nonexistent").join("test.jsonl");
        assert!(validate_path_within_session_dir(
            &nonexistent_path,
            &session_dir
        )?);

        Ok(())
    }

    #[test]
    fn test_path_normalization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_path = temp_dir.path().join("test");

        // Test that normalization doesn't crash and returns a path
        let normalized = normalize_path_for_comparison(&test_path);
        assert!(!normalized.as_os_str().is_empty());

        // Test with existing path
        fs::create_dir_all(&test_path).unwrap();
        let normalized_existing = normalize_path_for_comparison(&test_path);
        assert!(!normalized_existing.as_os_str().is_empty());
    }

    #[tokio::test]
    async fn test_save_session_parameter() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_save_session.jsonl");

        let messages = Conversation::new_unvalidated(vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
        ]);

        let metadata = SessionMetadata::default();

        // Test with save_session = true - should create file
        save_messages_with_metadata(&file_path, &metadata, &messages)?;
        assert!(
            file_path.exists(),
            "File should be created when save_session=true"
        );

        // Verify content is correct
        let read_messages = read_messages(&file_path)?;
        assert_eq!(messages.len(), read_messages.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_persist_messages_with_save_session_false() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_persist_no_save.jsonl");

        let messages = Conversation::new_unvalidated(vec![
            Message::user().with_text("Test message"),
            Message::assistant().with_text("Test response"),
        ]);

        // Test persist_messages_with_schedule_id with working_dir parameter
        persist_messages_with_schedule_id(
            &file_path,
            &messages,
            None,
            Some("test_schedule".to_string()),
            None,
        )
        .await?;

        assert!(
            file_path.exists(),
            "File should be created when save_session=true"
        );

        // Verify the schedule_id was set correctly
        let metadata = read_metadata(&file_path)?;
        assert_eq!(metadata.schedule_id, Some("test_schedule".to_string()));

        Ok(())
    }
}
