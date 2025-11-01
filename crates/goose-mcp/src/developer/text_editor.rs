use anyhow::Result;
use indoc::formatdoc;
use mpatch::{apply_patch, parse_diffs, PatchError};
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
    // Check for .. components
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

    // Try to canonicalize and check if within base
    if let (Ok(canonical_target), Ok(canonical_base)) =
        (target_path.canonicalize(), base_dir.canonicalize())
    {
        if !canonical_target.starts_with(&canonical_base) {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "Path '{}' is outside the base directory",
                    target_path.display()
                ),
                None,
            ));
        }
    } else if !target_path.exists() {
        // For new files, check parent directory
        if let Some(parent) = target_path.parent() {
            if let (Ok(canonical_parent), Ok(canonical_base)) =
                (parent.canonicalize(), base_dir.canonicalize())
            {
                if !canonical_parent.starts_with(&canonical_base) {
                    return Err(ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!(
                            "Path '{}' would be outside the base directory",
                            target_path.display()
                        ),
                        None,
                    ));
                }
            }
        }
    }

    // Check for symlinks
    if target_path.exists() {
        let metadata = target_path.symlink_metadata().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to check symlink status: {}", e),
                None,
            )
        })?;

        if metadata.is_symlink() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "Cannot modify symlink '{}'. Please operate on the actual file.",
                    target_path.display()
                ),
                None,
            ));
        }
    }

    Ok(())
}

/// Results from applying a diff
#[derive(Debug, Default)]
pub struct DiffResults {
    files_created: usize,
    files_modified: usize,
    files_deleted: usize,
    lines_added: usize,
    lines_removed: usize,
}

/// Validates the size of the diff content
fn validate_diff_size(diff_content: &str) -> Result<(), ErrorData> {
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
    Ok(())
}

/// Counts line changes from the diff content
fn count_line_changes(diff_content: &str) -> (usize, usize) {
    let lines_added = diff_content
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count();
    let lines_removed = diff_content
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .count();
    (lines_added, lines_removed)
}

/// Generates the summary for the diff application
fn generate_summary(results: &DiffResults, is_single_file: bool, base_path: &Path) -> Vec<Content> {
    let summary = if is_single_file {
        format!(
            "Successfully applied diff to {}:\n• Lines added: {}\n• Lines removed: {}",
            base_path.display(),
            results.lines_added,
            results.lines_removed
        )
    } else if results.files_created + results.files_modified + results.files_deleted > 1 {
        format!(
            "Successfully applied multi-file diff:\n\
            • Files created: {}\n\
            • Files modified: {}\n\
            • Files deleted: {}\n\
            • Lines added: {}\n\
            • Lines removed: {}",
            results.files_created,
            results.files_modified,
            results.files_deleted,
            results.lines_added,
            results.lines_removed
        )
    } else {
        format!(
            "Successfully applied diff:\n\
            • Files created: {}\n\
            • Files modified: {}\n\
            • Files deleted: {}\n\
            • Lines added: {}\n\
            • Lines removed: {}",
            results.files_created,
            results.files_modified,
            results.files_deleted,
            results.lines_added,
            results.lines_removed
        )
    };

    let user_message = if is_single_file {
        format!("{}\n\nUse 'undo_edit' to revert if needed.\n\n", summary)
    } else {
        format!(
            "{}\n\nUse 'undo_edit' on individual files to revert if needed.\n\n",
            summary
        )
    };

    vec![
        Content::text(summary.clone()).with_audience(vec![Role::Assistant]),
        Content::text(user_message)
            .with_audience(vec![Role::User])
            .with_priority(0.2),
    ]
}

fn adjust_base_dir_for_overlap(base_dir: &Path, file_path: &Path) -> PathBuf {
    let base_components: Vec<_> = base_dir.components().collect();
    let file_components: Vec<_> = file_path.components().collect();

    let min_len = base_components.len().min(file_components.len());
    let max_k = (1..=min_len)
        .rfind(|&k| file_components[0..k] == base_components[base_components.len() - k..])
        .unwrap_or(0);

    if max_k > 0 {
        let adjusted_components = base_components[..base_components.len() - max_k].to_vec();
        PathBuf::from_iter(adjusted_components)
    } else {
        base_dir.to_path_buf()
    }
}

/// Applies a single patch and updates results
fn apply_single_patch(
    patch: &mpatch::Patch,
    base_dir: &Path,
    file_history: &std::sync::Arc<std::sync::Mutex<HashMap<PathBuf, Vec<String>>>>,
    results: &mut DiffResults,
    failed_hunks: &mut Vec<String>,
) -> Result<(), ErrorData> {
    let adjusted_base_dir = adjust_base_dir_for_overlap(base_dir, &patch.file_path);

    let file_path = adjusted_base_dir.join(&patch.file_path);

    // Validate path safety
    validate_path_safety(&adjusted_base_dir, &file_path)?;

    // Save history before modifying
    let file_existed = file_path.exists();
    if file_existed {
        save_file_history(&file_path, file_history)?;
    }

    // Apply patch with fuzzy matching (70% similarity threshold)
    let success = apply_patch(patch, &adjusted_base_dir, false, 0.7).map_err(|e| match e {
        PatchError::Io { path, source } => ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to process '{}': {}", path.display(), source),
            None,
        ),
        PatchError::PathTraversal(path) => ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "Security: Path '{}' would escape the base directory",
                path.display()
            ),
            None,
        ),
        PatchError::TargetNotFound(path) => ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!(
                "File '{}' not found and patch doesn't create it",
                path.display()
            ),
            None,
        ),
        PatchError::MissingFileHeader => ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "Invalid patch format".to_string(),
            None,
        ),
    })?;

    if !success {
        // Collect information about failed hunks for better error reporting
        let hunk_count = patch.hunks.len();
        let context_preview = patch
            .hunks
            .first()
            .and_then(|h| {
                let match_block = h.get_match_block();
                match_block.first().map(|s| s.to_string())
            })
            .unwrap_or_else(|| "(empty context)".to_string());

        failed_hunks.push(format!(
            "Failed to apply some hunks to '{}' ({} hunks total). First expected line: '{}'",
            patch.file_path.display(),
            hunk_count,
            context_preview
        ));
    }

    // Update statistics
    if file_existed {
        results.files_modified += 1;
    } else {
        results.files_created += 1;
    }

    Ok(())
}

/// Parses diff content into patches with proper error handling
fn parse_diff_content(diff_content: &str) -> Result<Vec<mpatch::Patch>, ErrorData> {
    let wrapped_diff = if diff_content.contains("```diff") || diff_content.contains("```patch") {
        diff_content.to_string()
    } else {
        format!("```diff\n{}\n```", diff_content)
    };

    parse_diffs(&wrapped_diff).map_err(|e| match e {
        PatchError::MissingFileHeader => ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "Invalid diff format: Missing file header (e.g., '--- a/path/to/file')".to_string(),
            None,
        ),
        PatchError::Io { path, source } => ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("I/O error processing {}: {}", path.display(), source),
            None,
        ),
        PatchError::PathTraversal(path) => ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "Security: Path '{}' would escape the base directory",
                path.display()
            ),
            None,
        ),
        PatchError::TargetNotFound(path) => ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Target file not found: {}", path.display()),
            None,
        ),
    })
}

/// Ensures all patched files end with a newline
fn ensure_trailing_newlines(patches: &[mpatch::Patch], base_dir: &Path) -> Result<(), ErrorData> {
    for patch in patches {
        let adjusted_base_dir = adjust_base_dir_for_overlap(base_dir, &patch.file_path);
        let file_path = adjusted_base_dir.join(&patch.file_path);

        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path).map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to read file for post-processing: {}", e),
                    None,
                )
            })?;

            if !content.ends_with('\n') {
                let content_with_newline = format!("{}\n", content);
                std::fs::write(&file_path, content_with_newline).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to add trailing newline: {}", e),
                        None,
                    )
                })?;
            }
        }
    }
    Ok(())
}

/// Reports partial failures from patch application
fn report_partial_failures(failed_hunks: &[String]) {
    if !failed_hunks.is_empty() {
        let error_msg = format!(
            "Some patches were only partially applied (fuzzy matching at 70% similarity):\n\n{}\n\n\
            The files have been modified but some hunks couldn't find their context.\n\
            This usually happens when:\n\
            • The file has changed significantly from when the diff was created\n\
            • Line numbers in the diff are incorrect\n\
            • The context lines don't match exactly\n\n\
            Review the changes and use 'undo_edit' if needed.",
            failed_hunks.join("\n")
        );

        tracing::warn!("{}", error_msg);
    }
}

/// Applies any diff (single or multi-file) using mpatch for fuzzy matching
pub async fn apply_diff(
    base_path: &Path,
    diff_content: &str,
    file_history: &std::sync::Arc<std::sync::Mutex<HashMap<PathBuf, Vec<String>>>>,
) -> Result<Vec<Content>, ErrorData> {
    validate_diff_size(diff_content)?;
    let patches = parse_diff_content(diff_content)?;

    if patches.len() > MAX_FILES_IN_DIFF {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "Too many files in diff ({}). Maximum is {} files.",
                patches.len(),
                MAX_FILES_IN_DIFF
            ),
            None,
        ));
    }

    let base_dir = if base_path.is_file() {
        base_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        base_path.to_path_buf()
    };

    let mut results = DiffResults::default();
    let mut failed_hunks = Vec::new();

    for patch in &patches {
        apply_single_patch(
            patch,
            &base_dir,
            file_history,
            &mut results,
            &mut failed_hunks,
        )?;
    }

    ensure_trailing_newlines(&patches, &base_dir)?;
    report_partial_failures(&failed_hunks);

    let (lines_added, lines_removed) = count_line_changes(diff_content);
    results.lines_added = lines_added;
    results.lines_removed = lines_removed;

    let is_single_file = patches.len() == 1;
    Ok(generate_summary(&results, is_single_file, base_path))
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

/// Lists the contents of a directory with a maximum number of items
fn list_directory_contents(path: &Path) -> Result<Vec<Content>, ErrorData> {
    const MAX_ITEMS: usize = 50; // Maximum number of items to display

    // List files in the directory (similar to ls output)
    let entries = std::fs::read_dir(path).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read directory: {}", e),
            None,
        )
    })?;

    let mut files = Vec::new();
    let mut dirs = Vec::new();
    let mut total_count = 0;

    for entry in entries {
        let entry = entry.map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read directory entry: {}", e),
                None,
            )
        })?;

        total_count += 1;

        // Only process up to MAX_ITEMS entries
        if dirs.len() + files.len() < MAX_ITEMS {
            let metadata = entry.metadata().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to read metadata: {}", e),
                    None,
                )
            })?;

            let name = entry.file_name().to_string_lossy().to_string();

            if metadata.is_dir() {
                dirs.push(format!("{}/", name));
            } else {
                files.push(name);
            }
        }
    }

    // Sort for consistent output
    dirs.sort();
    files.sort();

    let mut output = format!("'{}' is a directory. Contents:\n\n", path.display());

    if !dirs.is_empty() {
        output.push_str("Directories:\n");
        for dir in &dirs {
            output.push_str(&format!("  {}\n", dir));
        }
        output.push('\n');
    }

    if !files.is_empty() {
        output.push_str("Files:\n");
        for file in &files {
            output.push_str(&format!("  {}\n", file));
        }
    }

    if dirs.is_empty() && files.is_empty() {
        output.push_str("  (empty directory)\n");
    }

    // If we hit the limit, indicate there are more items
    if total_count > MAX_ITEMS {
        output.push_str(&format!(
            "\n... and {} more items (showing first {} items)\n",
            total_count - MAX_ITEMS,
            MAX_ITEMS
        ));
    }

    Ok(vec![Content::text(output)])
}

pub async fn text_editor_view(
    path: &PathBuf,
    view_range: Option<(usize, i64)>,
) -> Result<Vec<Content>, ErrorData> {
    // Check if path is a directory
    if path.is_dir() {
        return list_directory_contents(path);
    }

    if !path.is_file() {
        return Err(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!(
                "The path '{}' does not exist or is not accessible.",
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
                let mut normalized_content = normalize_line_endings(&updated_content);

                if !normalized_content.ends_with('\n') {
                    normalized_content.push('\n');
                }

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
                tracing::debug!(
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
    let mut normalized_content = normalize_line_endings(&new_content);

    if !normalized_content.ends_with('\n') {
        normalized_content.push('\n');
    }

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
