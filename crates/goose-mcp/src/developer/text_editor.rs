use anyhow::Result;
use indoc::formatdoc;
use mpatch::{apply_patch, parse_diffs, PatchError};
use rmcp::model::{Content, ErrorCode, ErrorData, Role};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use similar::ChangeTag;

use super::editor_models::EditorModel;
use super::lang;

// --- Line ending detection and preservation ---

/// Detect the dominant line ending style in a file's content.
fn detect_line_ending(content: &str) -> &'static str {
    let crlf_idx = content.find("\r\n");
    let lf_idx = content.find('\n');
    match (crlf_idx, lf_idx) {
        (Some(c), Some(l)) if c < l => "\r\n",
        _ => "\n",
    }
}

/// Normalize all line endings to LF for consistent internal processing.
fn normalize_to_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Restore line endings to the original style detected from the file.
fn restore_line_endings(text: &str, ending: &str) -> String {
    if ending == "\r\n" {
        text.replace('\n', "\r\n")
    } else {
        text.to_string()
    }
}

// --- BOM handling ---

/// Strip UTF-8 BOM if present. Returns the BOM prefix (empty string if none) and the text without it.
fn strip_bom(content: &str) -> (&str, &str) {
    if let Some(stripped) = content.strip_prefix('\u{FEFF}') {
        ("\u{FEFF}", stripped)
    } else {
        ("", content)
    }
}

// --- Fuzzy text matching ---

/// Normalize text for fuzzy matching:
/// - Strip trailing whitespace from each line
/// - Normalize smart quotes to ASCII
/// - Normalize Unicode dashes/hyphens to ASCII hyphen
/// - Normalize special Unicode spaces to regular space
fn normalize_for_fuzzy_match(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            result.push('\n');
        }
        result.push_str(line.trim_end());
    }
    // Smart single quotes → '
    let result = result.replace(['\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}'], "'");
    // Smart double quotes → "
    let result = result.replace(['\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}'], "\"");
    // Various dashes → -
    let result = result.replace(
        [
            '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2015}', '\u{2212}',
        ],
        "-",
    );
    // Special spaces → regular space
    result.replace(
        [
            '\u{00A0}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}',
            '\u{2008}', '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}',
        ],
        " ",
    )
}

struct FuzzyMatchResult {
    found: bool,
    /// The text that was actually matched (to use with `replacen`).
    matched_text: String,
    /// The content string to perform the replacement on.
    /// When exact match: the original content. When fuzzy: the normalized content.
    content_for_replacement: String,
}

/// Find `old_text` in `content`, trying exact match first, then fuzzy.
fn fuzzy_find_text(content: &str, old_text: &str) -> FuzzyMatchResult {
    // Try exact match first
    if content.contains(old_text) {
        return FuzzyMatchResult {
            found: true,
            matched_text: old_text.to_string(),
            content_for_replacement: content.to_string(),
        };
    }

    // Try fuzzy match
    let fuzzy_content = normalize_for_fuzzy_match(content);
    let fuzzy_old = normalize_for_fuzzy_match(old_text);
    if fuzzy_content.contains(&fuzzy_old) {
        return FuzzyMatchResult {
            found: true,
            matched_text: fuzzy_old,
            content_for_replacement: fuzzy_content,
        };
    }

    FuzzyMatchResult {
        found: false,
        matched_text: String::new(),
        content_for_replacement: content.to_string(),
    }
}

// --- Unified diff generation for user-facing output ---

struct DiffOutput {
    diff: String,
}

/// Generate a compact unified diff with line numbers and context.
/// Shows added/removed lines with `+`/`-` prefixes and surrounding context.
fn generate_diff_string(old_content: &str, new_content: &str, context_lines: usize) -> DiffOutput {
    let diff = similar::TextDiff::from_lines(old_content, new_content);
    let changes: Vec<_> = diff.iter_all_changes().collect();

    let max_line = old_content.lines().count().max(new_content.lines().count());
    let width = max_line.to_string().len().max(1);

    let mut output = Vec::new();
    let mut old_line: usize = 1;
    let mut new_line: usize = 1;
    // Group changes into regions: each region is a run of changes plus context
    // We need to decide for each line whether to show it.
    // Strategy: collect line info, then select which to display.
    struct LineInfo {
        tag: ChangeTag,
        text: String,
        old_num: usize,
        new_num: usize,
    }

    let mut all_lines = Vec::new();
    for change in &changes {
        let text = change.value().trim_end_matches('\n').to_string();
        all_lines.push(LineInfo {
            tag: change.tag(),
            text,
            old_num: old_line,
            new_num: new_line,
        });
        match change.tag() {
            ChangeTag::Equal => {
                old_line += 1;
                new_line += 1;
            }
            ChangeTag::Delete => {
                old_line += 1;
            }
            ChangeTag::Insert => {
                new_line += 1;
            }
        }
    }

    // Find indices of changed lines
    let change_indices: Vec<usize> = all_lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.tag != ChangeTag::Equal)
        .map(|(i, _)| i)
        .collect();

    if change_indices.is_empty() {
        return DiffOutput {
            diff: String::new(),
        };
    }

    // Build set of lines to display (changes + context)
    let mut visible = vec![false; all_lines.len()];
    for &ci in &change_indices {
        let start = ci.saturating_sub(context_lines);
        let end = (ci + context_lines + 1).min(all_lines.len());
        for v in &mut visible[start..end] {
            *v = true;
        }
    }

    let mut last_visible = false;
    for (i, line) in all_lines.iter().enumerate() {
        if !visible[i] {
            if last_visible {
                output.push(format!(" {:>width$} ...", "", width = width));
            }
            last_visible = false;
            continue;
        }
        last_visible = true;

        match line.tag {
            ChangeTag::Equal => {
                output.push(format!(
                    " {:>width$} {}",
                    line.old_num,
                    line.text,
                    width = width
                ));
            }
            ChangeTag::Delete => {
                output.push(format!(
                    "-{:>width$} {}",
                    line.old_num,
                    line.text,
                    width = width
                ));
            }
            ChangeTag::Insert => {
                output.push(format!(
                    "+{:>width$} {}",
                    line.new_num,
                    line.text,
                    width = width
                ));
            }
        }
    }

    DiffOutput {
        diff: output.join("\n"),
    }
}

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

    let user_message = format!("{}\n\n", summary);

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
    results: &mut DiffResults,
    failed_hunks: &mut Vec<String>,
) -> Result<(), ErrorData> {
    let adjusted_base_dir = adjust_base_dir_for_overlap(base_dir, &patch.file_path);

    let file_path = adjusted_base_dir.join(&patch.file_path);

    // Validate path safety
    validate_path_safety(&adjusted_base_dir, &file_path)?;

    let file_existed = file_path.exists();

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
            Review the changes carefully.",
            failed_hunks.join("\n")
        );

        tracing::warn!("{}", error_msg);
    }
}

/// Applies any diff (single or multi-file) using mpatch for fuzzy matching
pub async fn apply_diff(base_path: &Path, diff_content: &str) -> Result<Vec<Content>, ErrorData> {
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
        apply_single_patch(patch, &base_dir, &mut results, &mut failed_hunks)?;
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
    if let Some((start, end)) = view_range {
        formatdoc! {"
            ### {path} (lines {start}-{end})
            ```{language}
            {content}
            ```
            ",
            path=path.display(),
            start=start,
            end=if end == -1 { "end".to_string() } else { end.to_string() },
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

    Ok(vec![
        Content::text(formatted.clone()).with_audience(vec![Role::Assistant]),
        Content::text(formatted)
            .with_audience(vec![Role::User])
            .with_priority(0.0),
    ])
}

pub async fn text_editor_write(path: &PathBuf, file_text: &str) -> Result<Vec<Content>, ErrorData> {
    // Detect existing file's line ending style, or use platform default
    let original_ending = if path.exists() {
        std::fs::read_to_string(path)
            .map(|c| detect_line_ending(&c).to_string())
            .unwrap_or_else(|_| "\n".to_string())
    } else if cfg!(windows) {
        "\r\n".to_string()
    } else {
        "\n".to_string()
    };

    let mut normalized_text = normalize_to_lf(file_text);
    if !normalized_text.ends_with('\n') {
        normalized_text.push('\n');
    }
    let final_text = restore_line_endings(&normalized_text, &original_ending);

    std::fs::write(path, &final_text).map_err(|e| {
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
            content=&normalized_text
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

        return apply_diff(path, diff_content).await;
    }
    if !path.exists() {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "File '{}' does not exist, you can write a new file with the `write_file` command",
                path.display()
            ),
            None,
        ));
    }

    // Read raw content and preserve original encoding details
    let raw_content = std::fs::read_to_string(path).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read file: {}", e),
            None,
        )
    })?;

    let (bom, content_without_bom) = strip_bom(&raw_content);
    let original_ending = detect_line_ending(content_without_bom);
    let content = normalize_to_lf(content_without_bom);
    let normalized_old = normalize_to_lf(old_str);
    let normalized_new = normalize_to_lf(new_str);

    // Check if Editor API is configured and use it as the primary path
    if let Some(ref editor) = editor_model {
        match editor.edit_code(&content, old_str, new_str).await {
            Ok(updated_content) => {
                let mut result = normalize_to_lf(&updated_content);
                if !result.ends_with('\n') {
                    result.push('\n');
                }
                let final_content =
                    format!("{}{}", bom, restore_line_endings(&result, original_ending));

                std::fs::write(path, &final_content).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write file: {}", e),
                        None,
                    )
                })?;

                return Ok(vec![
                    Content::text(format!("Successfully replaced text in {}.", path.display()))
                        .with_audience(vec![Role::Assistant]),
                    Content::text(format!("Successfully replaced text in {}.", path.display()))
                        .with_audience(vec![Role::User])
                        .with_priority(0.2),
                ]);
            }
            Err(e) => {
                tracing::debug!(
                    "Editor API call failed: {}, falling back to string replacement",
                    e
                );
            }
        }
    }

    // Fuzzy find: tries exact match first, then normalizes whitespace/unicode
    let match_result = fuzzy_find_text(&content, &normalized_old);

    if !match_result.found {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "'old_str' was not found in the file. Make sure it matches existing file content, including whitespace.".to_string(),
            None,
        ));
    }

    // Check uniqueness using fuzzy-normalized content for consistency
    let fuzzy_content = normalize_for_fuzzy_match(&content);
    let fuzzy_old = normalize_for_fuzzy_match(&normalized_old);
    let occurrences = fuzzy_content.matches(&fuzzy_old).count();
    if occurrences > 1 {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!(
                "'old_str' matches {} locations in the file. Provide more context to make it unique.",
                occurrences
            ),
            None,
        ));
    }

    // Perform replacement (exactly once)
    let base = &match_result.content_for_replacement;
    let new_content = base.replacen(&match_result.matched_text, &normalized_new, 1);

    if *base == new_content {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "No changes made — the replacement produced identical content.".to_string(),
            None,
        ));
    }

    let mut final_lf = new_content;
    if !final_lf.ends_with('\n') {
        final_lf.push('\n');
    }
    let final_content = format!(
        "{}{}",
        bom,
        restore_line_endings(&final_lf, original_ending)
    );

    std::fs::write(path, &final_content).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to write file: {}", e),
            None,
        )
    })?;

    let diff_output = generate_diff_string(base, &final_lf, 4);
    let summary = format!("Successfully replaced text in {}.", path.display());

    let user_output = formatdoc! {r#"
        {summary}
        ```diff
        {diff}
        ```
        "#,
        summary=summary,
        diff=diff_output.diff,
    };

    Ok(vec![
        Content::text(summary).with_audience(vec![Role::Assistant]),
        Content::text(user_output)
            .with_audience(vec![Role::User])
            .with_priority(0.2),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_line_ending_lf() {
        assert_eq!(detect_line_ending("hello\nworld\n"), "\n");
    }

    #[test]
    fn test_detect_line_ending_crlf() {
        assert_eq!(detect_line_ending("hello\r\nworld\r\n"), "\r\n");
    }

    #[test]
    fn test_detect_line_ending_empty() {
        assert_eq!(detect_line_ending("no newlines"), "\n");
    }

    #[test]
    fn test_normalize_to_lf() {
        assert_eq!(normalize_to_lf("a\r\nb\r\n"), "a\nb\n");
        assert_eq!(normalize_to_lf("a\rb\r"), "a\nb\n");
        assert_eq!(normalize_to_lf("a\nb\n"), "a\nb\n");
    }

    #[test]
    fn test_restore_line_endings() {
        assert_eq!(restore_line_endings("a\nb\n", "\r\n"), "a\r\nb\r\n");
        assert_eq!(restore_line_endings("a\nb\n", "\n"), "a\nb\n");
    }

    #[test]
    fn test_strip_bom() {
        assert_eq!(strip_bom("\u{FEFF}hello"), ("\u{FEFF}", "hello"));
        assert_eq!(strip_bom("hello"), ("", "hello"));
    }

    #[test]
    fn test_normalize_for_fuzzy_match_trailing_whitespace() {
        assert_eq!(
            normalize_for_fuzzy_match("hello   \nworld  \n"),
            "hello\nworld\n"
        );
    }

    #[test]
    fn test_normalize_for_fuzzy_match_smart_quotes() {
        assert_eq!(
            normalize_for_fuzzy_match("\u{201C}hello\u{201D}"),
            "\"hello\""
        );
        assert_eq!(
            normalize_for_fuzzy_match("\u{2018}it\u{2019}s\u{2019}"),
            "'it's'"
        );
    }

    #[test]
    fn test_normalize_for_fuzzy_match_dashes() {
        // em-dash and en-dash normalize to hyphen
        assert_eq!(normalize_for_fuzzy_match("a\u{2014}b"), "a-b");
        assert_eq!(normalize_for_fuzzy_match("a\u{2013}b"), "a-b");
    }

    #[test]
    fn test_normalize_for_fuzzy_match_special_spaces() {
        assert_eq!(normalize_for_fuzzy_match("a\u{00A0}b"), "a b");
    }

    #[test]
    fn test_fuzzy_find_exact_match() {
        let result = fuzzy_find_text("hello world", "world");
        assert!(result.found);
        assert_eq!(result.matched_text, "world");
        assert_eq!(result.content_for_replacement, "hello world");
    }

    #[test]
    fn test_fuzzy_find_trailing_whitespace() {
        // LLM sends without trailing spaces, file has them
        let result = fuzzy_find_text("hello   \nworld  \n", "hello\nworld\n");
        assert!(result.found);
    }

    #[test]
    fn test_fuzzy_find_smart_quotes() {
        let result = fuzzy_find_text("say \u{201C}hello\u{201D}", "say \"hello\"");
        assert!(result.found);
    }

    #[test]
    fn test_fuzzy_find_not_found() {
        let result = fuzzy_find_text("hello world", "xyz");
        assert!(!result.found);
    }

    #[tokio::test]
    async fn test_replace_preserves_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello\r\nworld\r\n").unwrap();

        let result = text_editor_replace(&path, "world", "rust", None, &None)
            .await
            .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\r\n"), "CRLF should be preserved");
        assert!(content.contains("rust"));
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_replace_strips_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "\u{FEFF}hello world\n").unwrap();

        text_editor_replace(&path, "world", "rust", None, &None)
            .await
            .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with('\u{FEFF}'), "BOM should be preserved");
        assert!(content.contains("rust"));
    }

    #[tokio::test]
    async fn test_replace_fuzzy_smart_quotes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        // File has smart quotes
        std::fs::write(&path, "say \u{201C}hello\u{201D}\n").unwrap();

        // LLM sends ASCII quotes
        let result = text_editor_replace(&path, "say \"hello\"", "say \"hi\"", None, &None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replace_fuzzy_trailing_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello   \nworld  \n").unwrap();

        // LLM sends without trailing whitespace
        let result = text_editor_replace(&path, "hello\nworld", "hi\nearth", None, &None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_diff_string_simple() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nmodified\nline3\n";
        let result = generate_diff_string(old, new, 4);
        assert!(result.diff.contains("-"), "should have removed lines");
        assert!(result.diff.contains("+"), "should have added lines");
        assert!(result.diff.contains("line2"), "should show old line");
        assert!(result.diff.contains("modified"), "should show new line");
        assert!(result.diff.contains("line1"), "should show context");
        assert!(result.diff.contains("line3"), "should show context");
    }

    #[test]
    fn test_generate_diff_string_context_limit() {
        // 10 unchanged lines, then a change, then 10 more unchanged
        let lines: Vec<String> = (1..=21).map(|i| format!("line{}", i)).collect();
        let old = lines.join("\n") + "\n";
        let mut new_lines = lines.clone();
        new_lines[10] = "CHANGED".to_string();
        let new = new_lines.join("\n") + "\n";

        let result = generate_diff_string(&old, &new, 2);
        // Should have ellipsis for skipped context
        assert!(result.diff.contains("..."), "should elide distant context");
        // Should NOT show line2 (too far from change at line 11)
        assert!(
            !result.diff.contains("line2"),
            "should not show distant lines"
        );
        // Should show nearby context
        assert!(result.diff.contains("line10"), "should show nearby context");
        assert!(result.diff.contains("line12"), "should show nearby context");
    }

    #[test]
    fn test_generate_diff_string_no_changes() {
        let content = "same\n";
        let result = generate_diff_string(content, content, 4);
        assert!(result.diff.is_empty());
    }

    #[tokio::test]
    async fn test_replace_shows_diff_in_user_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello\nworld\nfoo\n").unwrap();

        let result = text_editor_replace(&path, "world", "rust", None, &None)
            .await
            .unwrap();

        let user_content = result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(
            user_content.text.contains("```diff"),
            "should use diff format"
        );
        assert!(user_content.text.contains("-"), "should show removed");
        assert!(user_content.text.contains("+"), "should show added");
    }
}
