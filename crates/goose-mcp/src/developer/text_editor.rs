use anyhow::Result;
use ignore::gitignore::Gitignore;
use indoc::{formatdoc, indoc};
use mcp_core::{handler::ToolError, role::Role, tool::Tool, Content};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use url::Url;

use crate::developer::{lang, shell::normalize_line_endings};

/// Creates the text_editor tool
pub fn create_text_editor_tool() -> Tool {
    Tool::new(
        "text_editor".to_string(),
        indoc! {r#"
            Perform text editing operations on files.

            The `command` parameter specifies the operation to perform. Allowed options are:
            - `view`: View the content of a file.
            - `write`: Create or overwrite a file with the given content
            - `str_replace`: Replace a string in a file with a new string.
            - `undo_edit`: Undo the last edit made to a file.

            To use the write command, you must specify `file_text` which will become the new content of the file. Be careful with
            existing files! This is a full overwrite, so you must include everything - not just sections you are modifying.

            To use the str_replace command, you must specify both `old_str` and `new_str` - the `old_str` needs to exactly match one
            unique section of the original file, including any whitespace. Make sure to include enough context that the match is not
            ambiguous. The entire original string will be replaced with `new_str`.
        "#}.to_string(),
        json!({
            "type": "object",
            "required": ["command", "path"],
            "properties": {
                "path": {
                    "description": "Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`.",
                    "type": "string"
                },
                "command": {
                    "type": "string",
                    "enum": ["view", "write", "str_replace", "undo_edit"],
                    "description": "Allowed options are: `view`, `write`, `str_replace`, undo_edit`."
                },
                "old_str": {"type": "string"},
                "new_str": {"type": "string"},
                "file_text": {"type": "string"}
            }
        }),
        None,
    )
}

/// Execute a text editor command
pub async fn execute_text_editor_command(
    params: Value,
    _ignore_patterns: &Arc<Gitignore>,
    file_history: &Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    resolve_path_fn: impl Fn(&str) -> Result<PathBuf, ToolError>,
    is_ignored_fn: impl Fn(&PathBuf) -> bool,
) -> Result<Vec<Content>, ToolError> {
    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters("Missing 'command' parameter".to_string()))?;

    let path_str = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters("Missing 'path' parameter".into()))?;

    let path = resolve_path_fn(path_str)?;

    // Check if file is ignored before proceeding with any text editor operation
    if is_ignored_fn(&path) {
        return Err(ToolError::ExecutionError(format!(
            "Access to '{}' is restricted by .gooseignore",
            path.display()
        )));
    }

    match command {
        "view" => text_editor_view(&path).await,
        "write" => {
            let file_text = params
                .get("file_text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::InvalidParameters("Missing 'file_text' parameter".into())
                })?;

            text_editor_write(&path, file_text).await
        }
        "str_replace" => {
            let old_str = params
                .get("old_str")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::InvalidParameters("Missing 'old_str' parameter".into())
                })?;
            let new_str = params
                .get("new_str")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::InvalidParameters("Missing 'new_str' parameter".into())
                })?;

            text_editor_replace(&path, old_str, new_str, file_history).await
        }
        "undo_edit" => text_editor_undo(&path, file_history).await,
        _ => Err(ToolError::InvalidParameters(format!(
            "Unknown command '{}'",
            command
        ))),
    }
}

async fn text_editor_view(path: &PathBuf) -> Result<Vec<Content>, ToolError> {
    if path.is_file() {
        // Check file size first (400KB limit)
        const MAX_FILE_SIZE: u64 = 400 * 1024; // 400KB in bytes
        const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB

        let file_size = std::fs::metadata(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to get file metadata: {}", e)))?
            .len();

        if file_size > MAX_FILE_SIZE {
            return Err(ToolError::ExecutionError(format!(
                "File '{}' is too large ({:.2}KB). Maximum size is 400KB to prevent memory issues.",
                path.display(),
                file_size as f64 / 1024.0
            )));
        }

        let uri = Url::from_file_path(path)
            .map_err(|_| ToolError::ExecutionError("Invalid file path".into()))?
            .to_string();

        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

        let char_count = content.chars().count();
        if char_count > MAX_CHAR_COUNT {
            return Err(ToolError::ExecutionError(format!(
                "File '{}' has too many characters ({}). Maximum character count is {}.",
                path.display(),
                char_count,
                MAX_CHAR_COUNT
            )));
        }

        let language = lang::get_language_identifier(path);
        let formatted = formatdoc! {"
            ### {path}
            ```{language}
            {content}
            ```
            ",
            path=path.display(),
            language=language,
            content=content,
        };

        // The LLM gets just a quick update as we expect the file to view in the status
        // but we send a low priority message for the human
        Ok(vec![
            Content::embedded_text(uri, content).with_audience(vec![Role::Assistant]),
            Content::text(formatted)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    } else {
        Err(ToolError::ExecutionError(format!(
            "The path '{}' does not exist or is not a file.",
            path.display()
        )))
    }
}

async fn text_editor_write(path: &PathBuf, file_text: &str) -> Result<Vec<Content>, ToolError> {
    // Normalize line endings based on platform
    let normalized_text = normalize_line_endings(file_text);

    // Write to the file
    std::fs::write(path, normalized_text)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

    // Try to detect the language from the file extension
    let language = lang::get_language_identifier(path);

    // The assistant output does not show the file again because the content is already in the tool request
    // but we do show it to the user here
    Ok(vec![
        Content::text(format!("Successfully wrote to {}", path.display()))
            .with_audience(vec![Role::Assistant]),
        Content::text(formatdoc! {r#"
            ### {path}
            ```{language}
            {content}
            ```
            "#,
            path=path.display(),
            language=language,
            content=file_text,
        })
        .with_audience(vec![Role::User])
        .with_priority(0.2),
    ])
}

async fn text_editor_replace(
    path: &PathBuf,
    old_str: &str,
    new_str: &str,
    file_history: &Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
) -> Result<Vec<Content>, ToolError> {
    // Check if file exists and is active
    if !path.exists() {
        return Err(ToolError::InvalidParameters(format!(
            "File '{}' does not exist, you can write a new file with the `write` command",
            path.display()
        )));
    }

    // Read content
    let content = std::fs::read_to_string(path)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

    // Ensure 'old_str' appears exactly once
    if content.matches(old_str).count() > 1 {
        return Err(ToolError::InvalidParameters(
            "'old_str' must appear exactly once in the file, but it appears multiple times".into(),
        ));
    }
    if content.matches(old_str).count() == 0 {
        return Err(ToolError::InvalidParameters(
            "'old_str' must appear exactly once in the file, but it does not appear in the file. Make sure the string exactly matches existing file content, including whitespace!".into(),
        ));
    }

    // Save history for undo
    save_file_history(path, file_history)?;

    // Replace and write back with platform-specific line endings
    let new_content = content.replace(old_str, new_str);
    let normalized_content = normalize_line_endings(&new_content);
    std::fs::write(path, &normalized_content)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

    // Try to detect the language from the file extension
    let language = lang::get_language_identifier(path);

    // Show a snippet of the changed content with context
    const SNIPPET_LINES: usize = 4;

    // Count newlines before the replacement to find the line number
    let replacement_line = content
        .split(old_str)
        .next()
        .expect("should split on already matched content")
        .matches('\n')
        .count();

    // Calculate start and end lines for the snippet
    let start_line = replacement_line.saturating_sub(SNIPPET_LINES);
    let end_line = replacement_line + SNIPPET_LINES + new_str.matches('\n').count();

    // Get the relevant lines for our snippet
    let lines: Vec<&str> = new_content.lines().collect();
    let snippet = lines
        .iter()
        .skip(start_line)
        .take(end_line - start_line + 1)
        .cloned()
        .collect::<Vec<&str>>()
        .join("\n");

    let output = formatdoc! {r#"
        ```{language}
        {snippet}
        ```
        "#,
        language=language,
        snippet=snippet
    };

    let success_message = formatdoc! {r#"
        The file {} has been edited, and the section now reads:
        {}
        Review the changes above for errors. Undo and edit the file again if necessary!
        "#,
        path.display(),
        output
    };

    Ok(vec![
        Content::text(success_message).with_audience(vec![Role::Assistant]),
        Content::text(output)
            .with_audience(vec![Role::User])
            .with_priority(0.2),
    ])
}

async fn text_editor_undo(
    path: &PathBuf,
    file_history: &Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
) -> Result<Vec<Content>, ToolError> {
    let mut history = file_history.lock().unwrap();
    if let Some(contents) = history.get_mut(path) {
        if let Some(previous_content) = contents.pop() {
            // Write previous content back to file
            std::fs::write(path, previous_content)
                .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;
            Ok(vec![Content::text("Undid the last edit")])
        } else {
            Err(ToolError::InvalidParameters(
                "No edit history available to undo".into(),
            ))
        }
    } else {
        Err(ToolError::InvalidParameters(
            "No edit history available to undo".into(),
        ))
    }
}

fn save_file_history(
    path: &PathBuf,
    file_history: &Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
) -> Result<(), ToolError> {
    let mut history = file_history.lock().unwrap();
    let content = if path.exists() {
        std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?
    } else {
        String::new()
    };
    history.entry(path.clone()).or_default().push(content);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;

    use ignore::gitignore::GitignoreBuilder;
    use std::sync::Arc;

    #[test]
    fn test_create_text_editor_tool() {
        let tool = create_text_editor_tool();
        assert_eq!(tool.name, "text_editor");
        assert!(!tool.description.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_size_limits() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        let ignore_patterns = Arc::new(builder.build().unwrap());
        let file_history = Arc::new(Mutex::new(HashMap::new()));

        let resolve_path_fn =
            |path_str: &str| -> Result<PathBuf, ToolError> { Ok(temp_dir.path().join(path_str)) };
        let is_ignored_fn = |_path: &PathBuf| -> bool { false };

        // Test file size limit
        {
            let large_file_path = temp_dir.path().join("large.txt");

            // Create a file larger than 2MB
            let content = "x".repeat(3 * 1024 * 1024); // 3MB
            std::fs::write(&large_file_path, content).unwrap();

            let result = execute_text_editor_command(
                json!({
                    "command": "view",
                    "path": "large.txt"
                }),
                &ignore_patterns,
                &file_history,
                resolve_path_fn,
                &is_ignored_fn,
            )
            .await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert!(matches!(err, ToolError::ExecutionError(_)));
            assert!(err.to_string().contains("too large"));
        }

        // Test character count limit
        {
            let many_chars_path = temp_dir.path().join("many_chars.txt");

            // Create a file with more than 400K characters but less than 400KB
            let content = "x".repeat(405_000);
            std::fs::write(&many_chars_path, content).unwrap();

            let result = execute_text_editor_command(
                json!({
                    "command": "view",
                    "path": "many_chars.txt"
                }),
                &ignore_patterns,
                &file_history,
                resolve_path_fn,
                &is_ignored_fn,
            )
            .await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert!(matches!(err, ToolError::ExecutionError(_)));
            assert!(err.to_string().contains("too many characters"));
        }

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_write_and_view_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        let ignore_patterns = Arc::new(builder.build().unwrap());
        let file_history = Arc::new(Mutex::new(HashMap::new()));

        let resolve_path_fn =
            |path_str: &str| -> Result<PathBuf, ToolError> { Ok(temp_dir.path().join(path_str)) };
        let is_ignored_fn = |_path: &PathBuf| -> bool { false };

        // Create a new file
        execute_text_editor_command(
            json!({
                "command": "write",
                "path": "test.txt",
                "file_text": "Hello, world!"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        // View the file
        let view_result = execute_text_editor_command(
            json!({
                "command": "view",
                "path": "test.txt"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        assert!(!view_result.is_empty());
        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("Hello, world!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_str_replace() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        let ignore_patterns = Arc::new(builder.build().unwrap());
        let file_history = Arc::new(Mutex::new(HashMap::new()));

        let resolve_path_fn =
            |path_str: &str| -> Result<PathBuf, ToolError> { Ok(temp_dir.path().join(path_str)) };
        let is_ignored_fn = |_path: &PathBuf| -> bool { false };

        // Create a new file
        execute_text_editor_command(
            json!({
                "command": "write",
                "path": "test.txt",
                "file_text": "Hello, world!"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        // Replace string
        let replace_result = execute_text_editor_command(
            json!({
                "command": "str_replace",
                "path": "test.txt",
                "old_str": "world",
                "new_str": "Rust"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        let text = replace_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.contains("has been edited, and the section now reads"));

        // View the file to verify the change
        let view_result = execute_text_editor_command(
            json!({
                "command": "view",
                "path": "test.txt"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("Hello, Rust!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_undo_edit() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        let ignore_patterns = Arc::new(builder.build().unwrap());
        let file_history = Arc::new(Mutex::new(HashMap::new()));

        let resolve_path_fn =
            |path_str: &str| -> Result<PathBuf, ToolError> { Ok(temp_dir.path().join(path_str)) };
        let is_ignored_fn = |_path: &PathBuf| -> bool { false };

        // Create a new file
        execute_text_editor_command(
            json!({
                "command": "write",
                "path": "test.txt",
                "file_text": "First line"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        // Replace string
        execute_text_editor_command(
            json!({
                "command": "str_replace",
                "path": "test.txt",
                "old_str": "First line",
                "new_str": "Second line"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        // Undo the edit
        let undo_result = execute_text_editor_command(
            json!({
                "command": "undo_edit",
                "path": "test.txt"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        let text = undo_result.first().unwrap().as_text().unwrap();
        assert!(text.contains("Undid the last edit"));

        // View the file to verify the undo
        let view_result = execute_text_editor_command(
            json!({
                "command": "view",
                "path": "test.txt"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await
        .unwrap();

        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("First line"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_respects_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        builder.add_line(None, "secret.txt").unwrap();
        let ignore_patterns = Arc::new(builder.build().unwrap());
        let file_history = Arc::new(Mutex::new(HashMap::new()));

        let resolve_path_fn =
            |path_str: &str| -> Result<PathBuf, ToolError> { Ok(temp_dir.path().join(path_str)) };
        let is_ignored_fn =
            |path: &PathBuf| -> bool { ignore_patterns.matched(path, false).is_ignore() };

        // Try to write to an ignored file
        let result = execute_text_editor_command(
            json!({
                "command": "write",
                "path": "secret.txt",
                "file_text": "test content"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await;

        assert!(
            result.is_err(),
            "Should not be able to write to ignored file"
        );
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionError(_)));

        // Try to write to a non-ignored file
        let result = execute_text_editor_command(
            json!({
                "command": "write",
                "path": "allowed.txt",
                "file_text": "test content"
            }),
            &ignore_patterns,
            &file_history,
            &resolve_path_fn,
            &is_ignored_fn,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should be able to write to non-ignored file"
        );

        temp_dir.close().unwrap();
    }
}
