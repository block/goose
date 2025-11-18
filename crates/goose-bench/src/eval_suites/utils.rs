use crate::bench_work_dir::BenchmarkWorkDir;
use anyhow::{Context, Result};
use goose::conversation::message::Message;
use std::fs::File;
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
