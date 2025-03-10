use crate::bench_work_dir::BenchmarkWorkDir;
use anyhow::{Context, Result};
use goose::message::Message;
use goose::session::storage;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

/// Write the last agent message to a file
/// Returns the content of the message and an error if writing failed
pub fn write_response_to_file(
    messages: &[Message],
    _work_dir: &mut BenchmarkWorkDir, // Kept for API compatibility
    filename: &str,
) -> Result<String> {
    let last_msg = messages
        .last()
        .ok_or_else(|| anyhow::anyhow!("No messages to write to file"))?;

    let text_content = last_msg.as_concat_text();

    // Create a file in the current directory
    let output_path = PathBuf::from(filename);

    // Create and write to the file
    let mut file = File::create(&output_path)
        .with_context(|| format!("Failed to create file at {}", output_path.display()))?;

    file.write_all(text_content.as_bytes())
        .with_context(|| format!("Failed to write content to {}", output_path.display()))?;

    Ok(text_content)
}

/// Copy the most recent session file to the current working directory
///
/// This function finds the most recent Goose session file (.jsonl) and copies it
/// to the current working directory. Session files are stored by the Goose framework
/// in a platform-specific data directory.
///
/// # Returns
/// - Ok(session_path) if successfully copied, where session_path is the path to the copied file
/// - Err if any errors occurred during the process
pub fn copy_session_to_cwd() -> Result<PathBuf> {
    // Try to get the most recent session file
    let src_path = storage::get_most_recent_session()
        .with_context(|| "Failed to find any recent session files")?;

    // Extract the filename from the path
    let filename = src_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid session filename"))?;

    // Create the destination path in the current directory
    let dest_path = PathBuf::from(".").join(filename);

    // Copy the file
    fs::copy(&src_path, &dest_path).with_context(|| {
        format!(
            "Failed to copy from '{}' to '{}'",
            src_path.display(),
            dest_path.display()
        )
    })?;

    println!("Session file copied to: {}", dest_path.display());

    Ok(dest_path)
}
