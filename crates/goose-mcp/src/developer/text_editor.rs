use anyhow::Result;
use indoc::formatdoc;
use patcher::{
    ApplyResult, MultifilePatch, MultifilePatcher, Patch as PatcherPatch, PatchAlgorithm, Patcher,
};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use url::Url;

use rmcp::model::{Content, ErrorCode, ErrorData, Role};

use super::editor_models::EditorModel;
use super::lang;
use super::shell::normalize_line_endings;

// Constants
pub const LINE_READ_LIMIT: usize = 2000;
pub const MAX_DIFF_SIZE: usize = 1024 * 1024; // 1MB max diff size
pub const MAX_FILES_IN_DIFF: usize = 100; // Maximum files in a multi-file diff

/// Validates paths to prevent directory traversal attacks
fn validate_path_safety(base_dir: &Path, target_path: &Path) -> Result<(), ErrorData> {
    // Resolve to absolute paths
    let base = base_dir.canonicalize().map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to resolve base directory: {}", e),
            None,
        )
    })?;

    // For new files, we need to check the parent directory
    let target_abs = if target_path.exists() {
        target_path.canonicalize().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to resolve target path: {}", e),
                None,
            )
        })?
    } else {
        // For non-existent files, resolve the parent and append the filename
        let parent = target_path.parent().ok_or_else(|| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Invalid target path".to_string(),
                None,
            )
        })?;

        if parent.exists() {
            let parent_abs = parent.canonicalize().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to resolve parent directory: {}", e),
                    None,
                )
            })?;
            parent_abs.join(target_path.file_name().unwrap())
        } else {
            // If parent doesn't exist, just check the path doesn't contain ..
            if target_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "Path traversal detected: paths cannot contain '..'".to_string(),
                    None,
                ));
            }
            base_dir.join(target_path)
        }
    };

    // Check that target is within base directory
    if !target_abs.starts_with(&base) {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "Path '{}' is outside the base directory. This could be a security risk.",
                target_path.display()
            ),
            None,
        ));
    }

    // Check for symlinks
    if target_path.exists()
        && target_path
            .symlink_metadata()
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to check symlink status: {}", e),
                    None,
                )
            })?
            .is_symlink()
    {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "Cannot modify symlink '{}'. Please operate on the actual file.",
                target_path.display()
            ),
            None,
        ));
    }

    Ok(())
}

/// Clean path string by removing a/ or b/ prefixes commonly found in git diffs
fn clean_diff_path(path: &str) -> String {
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
        .to_string()
}

/// Applies any diff (single or multi-file) atomically with rollback on failure
pub async fn apply_diff(
    base_path: &Path,
    diff_content: &str,
    file_history: &std::sync::Arc<std::sync::Mutex<HashMap<PathBuf, Vec<String>>>>,
) -> Result<Vec<Content>, ErrorData> {
    // Size validation
    if diff_content.len() > MAX_DIFF_SIZE {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "Diff is too large ({} bytes). Maximum size is {} bytes (1MB).",
                diff_content.len(),
                MAX_DIFF_SIZE
            ),
            None,
        ));
    }

    // Parse the diff using patcher
    let (multipatch, is_single_file) = if diff_content.contains("diff --git") {
        // Multi-file git diff
        let mp = MultifilePatch::parse(diff_content).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid diff format: {}", e),
                None,
            )
        })?;
        (mp, false)
    } else {
        // Single file unified diff
        let patch = PatcherPatch::parse(diff_content).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid diff format: {}", e),
                None,
            )
        })?;
        (MultifilePatch::new(vec![patch]), true)
    };

    // Validate number of files
    if multipatch.patches.len() > MAX_FILES_IN_DIFF {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "Too many files in diff ({}). Maximum is {} files.",
                multipatch.patches.len(),
                MAX_FILES_IN_DIFF
            ),
            None,
        ));
    }

    // Determine base directory
    let base_dir = if is_single_file && !base_path.is_dir() {
        // Single file case: use the file's parent directory as base
        base_path.parent().unwrap_or(Path::new("."))
    } else {
        // Multi-file case or directory: use provided path as base
        base_path
    };

    // Pre-validate all paths and save history
    for patch in &multipatch.patches {
        // Clean the paths (remove a/ b/ prefixes)
        let old_path_str = clean_diff_path(&patch.old_file);
        let new_path_str = clean_diff_path(&patch.new_file);

        // Handle special case for single file
        let (old_path, new_path) = if is_single_file && !base_path.is_dir() {
            (base_path.to_path_buf(), base_path.to_path_buf())
        } else {
            (base_dir.join(&old_path_str), base_dir.join(&new_path_str))
        };

        // Skip /dev/null paths
        if old_path_str != "/dev/null" && new_path_str != "/dev/null" {
            // Validate path safety
            validate_path_safety(base_dir, &new_path)?;

            // Save history for existing files
            if old_path.exists() {
                save_file_history(&old_path, file_history)?;
            }
        } else if new_path_str != "/dev/null" {
            // New file case
            validate_path_safety(base_dir, &new_path)?;
        }
    }

    // Apply the patches using patcher's built-in functionality
    let patcher = if is_single_file && !base_path.is_dir() {
        // For single file, we need to handle it specially
        // Read the file content and apply the patch directly
        let content = if base_path.exists() {
            std::fs::read_to_string(base_path).map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to read '{}': {}", base_path.display(), e),
                    None,
                )
            })?
        } else {
            String::new()
        };

        let patch = &multipatch.patches[0];
        let patcher_obj = Patcher::new(patch.clone());
        let new_content = patcher_obj.apply(&content, false).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Failed to apply diff to '{}': {}. The diff may be for a different version of the file.",
                    base_path.display(),
                    e
                ),
                None,
            )
        })?;

        // Write the result
        std::fs::write(base_path, new_content).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to write '{}': {}", base_path.display(), e),
                None,
            )
        })?;

        // Generate summary for single file
        let lines_added = diff_content
            .lines()
            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
            .count();
        let lines_removed = diff_content
            .lines()
            .filter(|l| l.starts_with('-') && !l.starts_with("---"))
            .count();

        let summary = format!(
            "Successfully applied diff to {}:\n• Lines added: {}\n• Lines removed: {}",
            base_path.display(),
            lines_added,
            lines_removed
        );

        return Ok(vec![
            Content::text(summary.clone()).with_audience(vec![Role::Assistant]),
            Content::text(format!(
                "{}\n\nUse 'undo_edit' to revert if needed.",
                summary
            ))
            .with_audience(vec![Role::User])
            .with_priority(0.2),
        ]);
    } else {
        MultifilePatcher::with_root(multipatch, base_dir)
    };

    // Apply patches and write to files
    let results = patcher.apply_and_write(false).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to apply patches: {}", e),
            None,
        )
    })?;

    // Process results and generate summary
    let mut files_created = 0;
    let mut files_modified = 0;
    let mut files_deleted = 0;
    let mut errors = Vec::new();

    for result in results {
        match result {
            ApplyResult::Applied(file) => {
                if file.is_new {
                    files_created += 1;
                } else {
                    files_modified += 1;
                }
            }
            ApplyResult::Deleted(_path) => {
                files_deleted += 1;
            }
            ApplyResult::Failed(path, err) => {
                errors.push(format!("Failed to process '{}': {}", path, err));
            }
            ApplyResult::Skipped(reason) => {
                // Log skipped files if needed
                eprintln!("Skipped: {}", reason);
            }
        }
    }

    // Count line changes from the diff itself (more accurate)
    let total_lines_added = diff_content
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count();
    let total_lines_removed = diff_content
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .count();

    // Check for errors
    if !errors.is_empty() {
        return Err(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to apply some patches:\n{}", errors.join("\n")),
            None,
        ));
    }

    let summary = if files_created + files_modified + files_deleted > 1 {
        format!(
            "Successfully applied multi-file diff:\n\
            • Files created: {}\n\
            • Files modified: {}\n\
            • Files deleted: {}\n\
            • Lines added: {}\n\
            • Lines removed: {}",
            files_created, files_modified, files_deleted, total_lines_added, total_lines_removed
        )
    } else {
        format!(
            "Successfully applied diff:\n\
            • Files created: {}\n\
            • Files modified: {}\n\
            • Files deleted: {}\n\
            • Lines added: {}\n\
            • Lines removed: {}",
            files_created, files_modified, files_deleted, total_lines_added, total_lines_removed
        )
    };

    Ok(vec![
        Content::text(summary.clone()).with_audience(vec![Role::Assistant]),
        Content::text(format!(
            "{}\n\nUse 'undo_edit' on individual files to revert if needed.",
            summary
        ))
        .with_audience(vec![Role::User])
        .with_priority(0.2),
    ])
}

// Helper method to validate and calculate view range indices
pub fn calculate_view_range(
    view_range: Option<(usize, i64)>,
    total_lines: usize,
) -> Result<(usize, usize), ErrorData> {
    if let Some((start_line, end_line)) = view_range {
        // Convert 1-indexed line numbers to 0-indexed
        let start_idx = if start_line > 0 { start_line - 1 } else { 0 };
        let end_idx = if end_line == -1 {
            total_lines
        } else {
            std::cmp::min(end_line as usize, total_lines)
        };

        if start_idx >= total_lines {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "Start line {} is beyond the end of the file (total lines: {})",
                    start_line, total_lines
                ),
                None,
            ));
        }

        if start_idx >= end_idx {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "Start line {} must be less than end line {}",
                    start_line, end_line
                ),
                None,
            ));
        }

        Ok((start_idx, end_idx))
    } else {
        Ok((0, total_lines))
    }
}

// Helper method to format file content with line numbers
pub fn format_file_content(
    path: &Path,
    lines: &[&str],
    start_idx: usize,
    end_idx: usize,
    view_range: Option<(usize, i64)>,
) -> String {
    let display_content = if lines.is_empty() {
        String::new()
    } else {
        let selected_lines: Vec<String> = lines[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}: {}", start_idx + i + 1, line))
            .collect();

        selected_lines.join("\n")
    };

    let language = lang::get_language_identifier(path);
    if view_range.is_some() {
        formatdoc! {"
            ### {path} (lines {start}-{end})
            ```{language}
            {content}
            ```
            ",
            path=path.display(),
            start=view_range.unwrap().0,
            end=if view_range.unwrap().1 == -1 { "end".to_string() } else { view_range.unwrap().1.to_string() },
            language=language,
            content=display_content,
        }
    } else {
        formatdoc! {"
            ### {path}
            ```{language}
            {content}
            ```
            ",
            path=path.display(),
            language=language,
            content=display_content,
        }
    }
}

pub fn recommend_read_range(path: &Path, total_lines: usize) -> Result<Vec<Content>, ErrorData> {
    Err(ErrorData::new(ErrorCode::INTERNAL_ERROR, format!(
        "File '{}' is {} lines long, recommended to read in with view_range (or searching) to get bite size content. If you do wish to read all the file, please pass in view_range with [1, {}] to read it all at once",
        path.display(),
        total_lines,
        total_lines
    ), None))
}

pub async fn text_editor_view(
    path: &PathBuf,
    view_range: Option<(usize, i64)>,
) -> Result<Vec<Content>, ErrorData> {
    if !path.is_file() {
        return Err(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!(
                "The path '{}' does not exist or is not a file.",
                path.display()
            ),
            None,
        ));
    }

    const MAX_FILE_SIZE: u64 = 400 * 1024; // 400KB

    let f = File::open(path).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to open file: {}", e),
            None,
        )
    })?;

    let file_size = f
        .metadata()
        .map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to get file metadata: {}", e),
                None,
            )
        })?
        .len();

    if file_size > MAX_FILE_SIZE {
        return Err(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!(
                "File '{}' is too large ({:.2}KB). Maximum size is 400KB to prevent memory issues.",
                path.display(),
                file_size as f64 / 1024.0
            ),
            None,
        ));
    }

    // Ensure we never read over that limit even if the file is being concurrently mutated
    let mut f = f.take(MAX_FILE_SIZE);

    let uri = Url::from_file_path(path)
        .map_err(|_| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Invalid file path".to_string(),
                None,
            )
        })?
        .to_string();

    let mut content = String::new();
    f.read_to_string(&mut content).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read file: {}", e),
            None,
        )
    })?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // We will gently encourage the LLM to specify a range for large line count files
    // it can of course specify exact range to read any size file
    if view_range.is_none() && total_lines > LINE_READ_LIMIT {
        return recommend_read_range(path, total_lines);
    }

    let (start_idx, end_idx) = calculate_view_range(view_range, total_lines)?;
    let formatted = format_file_content(path, &lines, start_idx, end_idx, view_range);

    // The LLM gets just a quick update as we expect the file to view in the status
    // but we send a low priority message for the human
    Ok(vec![
        Content::embedded_text(uri, content).with_audience(vec![Role::Assistant]),
        Content::text(formatted)
            .with_audience(vec![Role::User])
            .with_priority(0.0),
    ])
}

pub async fn text_editor_write(path: &PathBuf, file_text: &str) -> Result<Vec<Content>, ErrorData> {
    // Normalize line endings based on platform
    let mut normalized_text = normalize_line_endings(file_text); // Make mutable

    // Ensure the text ends with a newline
    if !normalized_text.ends_with('\n') {
        normalized_text.push('\n');
    }

    // Write to the file
    std::fs::write(path, &normalized_text) // Write the potentially modified text
        .map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to write file: {}", e),
                None,
            )
        })?;

    // Try to detect the language from the file extension
    let language = lang::get_language_identifier(path);

    // The assistant output does not show the file again because the content is already in the tool request
    // but we do show it to the user here, using the final written content
    Ok(vec![
        Content::text(format!("Successfully wrote to {}", path.display()))
            .with_audience(vec![Role::Assistant]),
        Content::text(formatdoc! {
            r#"
            ### {path}
            ```{language}
            {content}
            ```
            "#,
            path=path.display(),
            language=language,
            content=&normalized_text // Use the final normalized_text for user feedback
        })
        .with_audience(vec![Role::User])
        .with_priority(0.2),
    ])
}

#[allow(clippy::too_many_lines)]
pub async fn text_editor_replace(
    path: &PathBuf,
    old_str: &str,
    new_str: &str,
    diff: Option<&str>,
    editor_model: &Option<EditorModel>,
    file_history: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<PathBuf, Vec<String>>>,
    >,
) -> Result<Vec<Content>, ErrorData> {
    // Check if diff is provided
    if let Some(diff_content) = diff {
        // Validate it's a proper diff
        if !diff_content.contains("---") || !diff_content.contains("+++") {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "The 'diff' parameter must be in unified diff format".to_string(),
                None,
            ));
        }

        return apply_diff(path, diff_content, file_history).await;
    }
    // Check if file exists and is active
    if !path.exists() {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "File '{}' does not exist, you can write a new file with the `write` command",
                path.display()
            ),
            None,
        ));
    }

    // Read content
    let content = std::fs::read_to_string(path).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read file: {}", e),
            None,
        )
    })?;

    // Check if Editor API is configured and use it as the primary path
    if let Some(ref editor) = editor_model {
        // Editor API path - save history then call API directly
        save_file_history(path, file_history)?;

        match editor.edit_code(&content, old_str, new_str).await {
            Ok(updated_content) => {
                // Write the updated content directly
                let normalized_content = normalize_line_endings(&updated_content);
                std::fs::write(path, &normalized_content).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write file: {}", e),
                        None,
                    )
                })?;

                // Simple success message for Editor API
                return Ok(vec![
                    Content::text(format!("Successfully edited {}", path.display()))
                        .with_audience(vec![Role::Assistant]),
                    Content::text(format!("File {} has been edited", path.display()))
                        .with_audience(vec![Role::User])
                        .with_priority(0.2),
                ]);
            }
            Err(e) => {
                eprintln!(
                    "Editor API call failed: {}, falling back to string replacement",
                    e
                );
                // Fall through to traditional path below
            }
        }
    }

    // Traditional string replacement path (original logic)
    // Ensure 'old_str' appears exactly once
    if content.matches(old_str).count() > 1 {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "'old_str' must appear exactly once in the file, but it appears multiple times"
                .to_string(),
            None,
        ));
    }
    if content.matches(old_str).count() == 0 {
        return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, "'old_str' must appear exactly once in the file, but it does not appear in the file. Make sure the string exactly matches existing file content, including whitespace!".to_string(), None));
    }

    // Save history for undo (original behavior - after validation)
    save_file_history(path, file_history)?;

    let new_content = content.replace(old_str, new_str);
    let normalized_content = normalize_line_endings(&new_content);
    std::fs::write(path, &normalized_content).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to write file: {}", e),
            None,
        )
    })?;

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
    let end_line = replacement_line + SNIPPET_LINES + new_content.matches('\n').count();

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

pub async fn text_editor_insert(
    path: &PathBuf,
    insert_line_spec: i64,
    new_str: &str,
    file_history: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<PathBuf, Vec<String>>>,
    >,
) -> Result<Vec<Content>, ErrorData> {
    // Check if file exists
    if !path.exists() {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "File '{}' does not exist, you can write a new file with the `write` command",
                path.display()
            ),
            None,
        ));
    }

    // Read content
    let content = std::fs::read_to_string(path).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read file: {}", e),
            None,
        )
    })?;

    // Save history for undo
    save_file_history(path, file_history)?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Allow insert_line to be negative
    let insert_line = if insert_line_spec < 0 {
        // -1 == end of file, -2 == before the last line, etc.
        (total_lines as i64 + 1 + insert_line_spec) as usize
    } else {
        insert_line_spec as usize
    };

    // Validate insert_line parameter
    if insert_line > total_lines {
        return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, format!(
            "Insert line {} is beyond the end of the file (total lines: {}). Use 0 to insert at the beginning or {} to insert at the end.",
            insert_line, total_lines, total_lines
        ), None));
    }

    // Create new content with inserted text
    let mut new_lines = Vec::new();

    // Add lines before the insertion point
    for (i, line) in lines.iter().enumerate() {
        if i == insert_line {
            // Insert the new text at this position
            new_lines.push(new_str.to_string());
        }
        new_lines.push(line.to_string());
    }

    // If inserting at the end (after all existing lines)
    if insert_line == total_lines {
        new_lines.push(new_str.to_string());
    }

    let new_content = new_lines.join("\n");
    let normalized_content = normalize_line_endings(&new_content);

    // Ensure the file ends with a newline
    let final_content = if !normalized_content.ends_with('\n') {
        format!("{}\n", normalized_content)
    } else {
        normalized_content
    };

    std::fs::write(path, &final_content).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to write file: {}", e),
            None,
        )
    })?;

    // Try to detect the language from the file extension
    let language = lang::get_language_identifier(path);

    // Show a snippet of the inserted content with context
    const SNIPPET_LINES: usize = 4;
    let insertion_line = insert_line + 1; // Convert to 1-indexed for display

    // Calculate start and end lines for the snippet
    let start_line = insertion_line.saturating_sub(SNIPPET_LINES);
    let end_line = std::cmp::min(insertion_line + SNIPPET_LINES, new_lines.len());

    // Get the relevant lines for our snippet with line numbers
    let snippet_lines: Vec<String> = new_lines[start_line.saturating_sub(1)..end_line]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}: {}", start_line + i, line))
        .collect();

    let snippet = snippet_lines.join("\n");

    let output = formatdoc! {r#"
        ```{language}
        {snippet}
        ```
        "#,
        language=language,
        snippet=snippet
    };

    let success_message = formatdoc! {r#"
        Text has been inserted at line {} in {}. The section now reads:
        {}
        Review the changes above for errors. Undo and edit the file again if necessary!
        "#,
        insertion_line,
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

pub async fn text_editor_undo(
    path: &PathBuf,
    file_history: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<PathBuf, Vec<String>>>,
    >,
) -> Result<Vec<Content>, ErrorData> {
    let mut history = file_history.lock().unwrap();
    if let Some(contents) = history.get_mut(path) {
        if let Some(previous_content) = contents.pop() {
            // Write previous content back to file
            std::fs::write(path, previous_content).map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write file: {}", e),
                    None,
                )
            })?;
            Ok(vec![Content::text("Undid the last edit")])
        } else {
            Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "No edit history available to undo".to_string(),
                None,
            ))
        }
    } else {
        Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "No edit history available to undo".to_string(),
            None,
        ))
    }
}

pub fn save_file_history(
    path: &PathBuf,
    file_history: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<PathBuf, Vec<String>>>,
    >,
) -> Result<(), ErrorData> {
    let mut history = file_history.lock().unwrap();
    let content = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read file: {}", e),
                None,
            )
        })?
    } else {
        String::new()
    };
    history.entry(path.clone()).or_default().push(content);
    Ok(())
}
