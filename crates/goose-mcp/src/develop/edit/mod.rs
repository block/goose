//! File editing: create, overwrite, and modify files.
//!
//! This module provides:
//! - `file_write`: Create a new file or overwrite an existing one
//! - `file_edit`: Find and replace text in a file

use std::fs;
use std::path::Path;

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

// ============================================================================
// Tool Parameter Types
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteParams {
    /// Absolute path to the file to create or overwrite.
    pub path: String,
    /// The content to write to the file.
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileEditParams {
    /// Absolute path to the file to edit.
    pub path: String,
    /// The exact text to find in the file. Must match uniquely.
    pub before: String,
    /// The text to replace it with. Use empty string to delete.
    pub after: String,
}

// ============================================================================
// Tool Implementations
// ============================================================================

pub struct EditTools;

impl EditTools {
    pub fn new() -> Self {
        Self
    }

    /// Create or overwrite a file.
    pub fn file_write(&self, params: FileWriteParams) -> CallToolResult {
        let path = Path::new(&params.path);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))]);
                }
            }
        }

        let is_new = !path.exists();

        match fs::write(path, &params.content) {
            Ok(()) => {
                let line_count = params.content.lines().count();
                let action = if is_new { "Created" } else { "Wrote" };
                CallToolResult::success(vec![Content::text(format!(
                    "{} {} ({} lines)",
                    action, params.path, line_count
                ))])
            }
            Err(e) => CallToolResult::error(vec![Content::text(format!(
                "Failed to write {}: {}",
                params.path, e
            ))]),
        }
    }

    /// Find and replace text in a file.
    pub fn file_edit(&self, params: FileEditParams) -> CallToolResult {
        let path = Path::new(&params.path);

        // Read current content
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to read {}: {}",
                    params.path, e
                ))]);
            }
        };

        // Find all occurrences
        let matches: Vec<_> = content.match_indices(&params.before).collect();

        match matches.len() {
            0 => {
                // No match - provide helpful context
                let suggestion = find_similar_context(&content, &params.before);
                let msg = if let Some(hint) = suggestion {
                    format!(
                        "No match found for the specified text.\n\nDid you mean:\n```\n{}\n```",
                        hint
                    )
                } else {
                    "No match found for the specified text.".to_string()
                };
                CallToolResult::error(vec![Content::text(msg)])
            }
            1 => {
                // Exactly one match - perform replacement
                let new_content = content.replacen(&params.before, &params.after, 1);

                match fs::write(path, &new_content) {
                    Ok(()) => {
                        let old_lines = params.before.lines().count();
                        let new_lines = params.after.lines().count();
                        CallToolResult::success(vec![Content::text(format!(
                            "Edited {} ({} lines -> {} lines)",
                            params.path, old_lines, new_lines
                        ))])
                    }
                    Err(e) => CallToolResult::error(vec![Content::text(format!(
                        "Failed to write {}: {}",
                        params.path, e
                    ))]),
                }
            }
            n => {
                // Multiple matches - show context for first 2
                let mut msg = format!(
                    "Found {} matches. Please provide more context to identify a unique match:\n",
                    n
                );

                for (i, (pos, _)) in matches.iter().enumerate().take(2) {
                    let line_num = count_lines_before(&content, *pos);
                    let context = get_line_context(&content, line_num, 1);
                    msg.push_str(&format!(
                        "\nMatch {} (line {}):\n```\n{}\n```",
                        i + 1,
                        line_num,
                        context
                    ));
                }

                if n > 2 {
                    msg.push_str(&format!("\n\n...and {} more", n - 2));
                }

                CallToolResult::error(vec![Content::text(msg)])
            }
        }
    }
}

impl Default for EditTools {
    fn default() -> Self {
        Self::new()
    }
}

/// Count lines before a byte position (UTF-8 safe).
fn count_lines_before(content: &str, byte_pos: usize) -> usize {
    content
        .char_indices()
        .take_while(|(i, _)| *i < byte_pos)
        .filter(|(_, c)| *c == '\n')
        .count()
        + 1
}

/// Get lines around a target line for context.
fn get_line_context(content: &str, target_line: usize, context: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = target_line.saturating_sub(context + 1);
    let end = (target_line + context).min(lines.len());

    lines[start..end].join("\n")
}

/// Try to find similar text that might be what the user meant.
fn find_similar_context(content: &str, search: &str) -> Option<String> {
    // Get the first line of the search text as a key
    let first_line = search.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }

    // Look for lines containing similar content
    for (i, line) in content.lines().enumerate() {
        if line.contains(first_line) || first_line.contains(line.trim()) {
            return Some(get_line_context(content, i + 1, 2));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_file_write_new() {
        let dir = setup();
        let path = dir.path().join("new_file.txt");
        let tools = EditTools::new();

        let result = tools.file_write(FileWriteParams {
            path: path.to_string_lossy().to_string(),
            content: "Hello, world!\nLine 2".to_string(),
        });

        assert!(!result.is_error.unwrap_or(false));
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello, world!\nLine 2");
    }

    #[test]
    fn test_file_write_overwrite() {
        let dir = setup();
        let path = dir.path().join("existing.txt");
        fs::write(&path, "old content").unwrap();
        let tools = EditTools::new();

        let result = tools.file_write(FileWriteParams {
            path: path.to_string_lossy().to_string(),
            content: "new content".to_string(),
        });

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
    }

    #[test]
    fn test_file_write_creates_dirs() {
        let dir = setup();
        let path = dir.path().join("a/b/c/file.txt");
        let tools = EditTools::new();

        let result = tools.file_write(FileWriteParams {
            path: path.to_string_lossy().to_string(),
            content: "nested".to_string(),
        });

        assert!(!result.is_error.unwrap_or(false));
        assert!(path.exists());
    }

    #[test]
    fn test_file_edit_single_match() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "fn foo() {\n    println!(\"hello\");\n}").unwrap();
        let tools = EditTools::new();

        let result = tools.file_edit(FileEditParams {
            path: path.to_string_lossy().to_string(),
            before: "println!(\"hello\");".to_string(),
            after: "println!(\"world\");".to_string(),
        });

        assert!(!result.is_error.unwrap_or(false));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("println!(\"world\");"));
        assert!(!content.contains("println!(\"hello\");"));
    }

    #[test]
    fn test_file_edit_no_match() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "some content").unwrap();
        let tools = EditTools::new();

        let result = tools.file_edit(FileEditParams {
            path: path.to_string_lossy().to_string(),
            before: "nonexistent".to_string(),
            after: "replacement".to_string(),
        });

        assert!(result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_file_edit_multiple_matches() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "foo\nbar\nfoo\nbaz").unwrap();
        let tools = EditTools::new();

        let result = tools.file_edit(FileEditParams {
            path: path.to_string_lossy().to_string(),
            before: "foo".to_string(),
            after: "qux".to_string(),
        });

        assert!(result.is_error.unwrap_or(false));
        // File should be unchanged
        assert_eq!(fs::read_to_string(&path).unwrap(), "foo\nbar\nfoo\nbaz");
    }

    #[test]
    fn test_file_edit_delete() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "keep\ndelete me\nkeep").unwrap();
        let tools = EditTools::new();

        let result = tools.file_edit(FileEditParams {
            path: path.to_string_lossy().to_string(),
            before: "\ndelete me".to_string(),
            after: "".to_string(),
        });

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep\nkeep");
    }
}
