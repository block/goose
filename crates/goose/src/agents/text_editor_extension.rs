//! Text Editor Platform Extension
//!
//! Provides file viewing and editing capabilities as a platform extension.

use crate::agents::editor_models::{create_editor_model, EditorModel};
use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use etcetera::AppStrategy;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use indoc::indoc;
use mpatch::{apply_patch, parse_diffs, PatchError};
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "text_editor";

const MAX_VIEW_SIZE: u64 = 400 * 1024; // 400KB limit
const MAX_LINES_THRESHOLD: usize = 1000;
const SIMILARITY_THRESHOLD: f64 = 0.7;
const MAX_DIFF_SIZE: usize = 1024 * 1024; // 1MB max diff size
const MAX_FILES_IN_DIFF: usize = 100;
const MAX_DIR_ITEMS: usize = 50;

const DEFAULT_GOOSEIGNORE_CONTENT: &str = "\
# Default ignore patterns
.git/
.env
.env.*
*.pem
*.key
*_rsa
*_dsa
*_ecdsa
*_ed25519
node_modules/
__pycache__/
.DS_Store
";

#[derive(Debug, Deserialize, JsonSchema)]
struct TextEditorParams {
    /// The command: "view", "write", "str_replace", "insert", "undo_edit", or "apply_diff"
    command: String,
    /// Absolute path to the file (or base directory for apply_diff)
    path: String,
    /// For view: optional start line (1-indexed)
    view_range_start: Option<i32>,
    /// For view: optional end line (1-indexed, -1 for end of file)
    view_range_end: Option<i32>,
    /// For write/insert: the text content
    file_text: Option<String>,
    /// For str_replace: the text to find
    old_str: Option<String>,
    /// For str_replace: the replacement text
    new_str: Option<String>,
    /// For insert: line number after which to insert (0 for beginning)
    insert_line: Option<i32>,
    /// For apply_diff or str_replace: unified diff content
    diff: Option<String>,
}

pub struct TextEditorClient {
    info: InitializeResult,
    #[allow(dead_code)]
    context: PlatformExtensionContext,
    file_history: Arc<std::sync::Mutex<HashMap<PathBuf, Vec<String>>>>,
    editor_model: Option<EditorModel>,
    ignore_patterns: Gitignore,
    working_dir: PathBuf,
}

impl TextEditorClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tasks: None,
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Text Editor".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "File viewing and editing tools. Use text_editor to view, create, and modify files."
                    .to_string(),
            ),
        };

        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let ignore_patterns = Self::build_ignore_patterns(&working_dir);

        Ok(Self {
            info,
            context,
            file_history: Arc::new(std::sync::Mutex::new(HashMap::new())),
            editor_model: create_editor_model(),
            ignore_patterns,
            working_dir,
        })
    }

    fn build_ignore_patterns(cwd: &Path) -> Gitignore {
        let mut builder = GitignoreBuilder::new(cwd);
        let local_ignore_path = cwd.join(".gooseignore");

        let global_ignore_path = etcetera::choose_app_strategy(etcetera::AppStrategyArgs {
            top_level_domain: "block".to_string(),
            author: "Block".to_string(),
            app_name: "goose".to_string(),
        })
        .map(|strategy| strategy.config_dir().join(".gooseignore"))
        .ok();

        let has_local_ignore = local_ignore_path.is_file();
        let has_global_ignore = global_ignore_path
            .as_ref()
            .map(|p: &PathBuf| p.is_file())
            .unwrap_or(false);

        if !has_local_ignore && !has_global_ignore {
            for pattern in DEFAULT_GOOSEIGNORE_CONTENT.lines() {
                let trimmed = pattern.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let _ = builder.add_line(None, trimmed);
            }
        }

        if has_global_ignore {
            let _ = builder.add(global_ignore_path.as_ref().unwrap());
        }

        if has_local_ignore {
            let _ = builder.add(&local_ignore_path);
        }

        builder.build().unwrap_or_else(|_| Gitignore::empty())
    }

    fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_patterns
            .matched(path, path.is_dir())
            .is_ignore()
    }

    fn get_tools() -> Vec<Tool> {
        let schema = schema_for!(TextEditorParams);
        let schema_value =
            serde_json::to_value(schema).expect("Failed to serialize TextEditorParams schema");

        vec![Tool::new(
            "text_editor".to_string(),
            indoc! {r#"
                View, create, and edit files.

                Commands:
                - view: Read file content, optionally with line range [start, end]
                - write: Create or overwrite a file with new content
                - str_replace: Replace exact text (old_str->new_str), or pass diff for fuzzy patching
                - insert: Insert text after a specific line number
                - undo_edit: Revert the last edit to a file
                - apply_diff: Apply a unified diff to one or more files

                For str_replace, old_str must match EXACTLY (including whitespace).
                For apply_diff, provide unified diff format with path as base directory.
            "#}
            .to_string(),
            schema_value.as_object().unwrap().clone(),
        )
        .annotate(ToolAnnotations {
            title: Some("Text Editor".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        })]
    }

    async fn handle_view(
        &self,
        path: &Path,
        start: Option<i32>,
        end: Option<i32>,
    ) -> Result<String, String> {
        if path.is_dir() {
            return self.list_directory_contents(path);
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| format!("Cannot access '{}': {}", path.display(), e))?;

        if metadata.len() > MAX_VIEW_SIZE {
            return Err(format!(
                "File '{}' is too large ({} bytes). Maximum viewable size is {} bytes. Use a line range.",
                path.display(),
                metadata.len(),
                MAX_VIEW_SIZE
            ));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let (start_idx, end_idx) = match (start, end) {
            (Some(s), Some(e)) => {
                let s = (s.max(1) as usize).saturating_sub(1);
                let e = if e < 0 {
                    total_lines
                } else {
                    (e as usize).min(total_lines)
                };
                (s, e)
            }
            (Some(s), None) => {
                let s = (s.max(1) as usize).saturating_sub(1);
                (s, total_lines)
            }
            (None, Some(e)) => {
                let e = if e < 0 {
                    total_lines
                } else {
                    (e as usize).min(total_lines)
                };
                (0, e)
            }
            (None, None) => (0, total_lines),
        };

        if start_idx >= total_lines {
            return Err(format!(
                "Start line {} is beyond end of file ({} lines)",
                start_idx + 1,
                total_lines
            ));
        }

        let selected_lines: Vec<String> = lines[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:6}  {}", start_idx + i + 1, line))
            .collect();

        let mut result = selected_lines.join(
            "
",
        );

        if start.is_none() && end.is_none() && total_lines > MAX_LINES_THRESHOLD {
            result = format!(
                "[Note: Showing {} lines. Use view_range for large files.]
{}",
                total_lines, result
            );
        }

        Ok(result)
    }

    fn list_directory_contents(&self, path: &Path) -> Result<String, String> {
        let entries =
            std::fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))?;

        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let mut total_count = 0;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            total_count += 1;

            if dirs.len() + files.len() < MAX_DIR_ITEMS {
                let metadata = entry
                    .metadata()
                    .map_err(|e| format!("Failed to read metadata: {}", e))?;
                let name = entry.file_name().to_string_lossy().to_string();

                if metadata.is_dir() {
                    dirs.push(format!("{}/", name));
                } else {
                    files.push(name);
                }
            }
        }

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

        if total_count > MAX_DIR_ITEMS {
            output.push_str(&format!(
                "\n... and {} more items (showing first {} items)\n",
                total_count - MAX_DIR_ITEMS,
                MAX_DIR_ITEMS
            ));
        }

        Ok(output)
    }

    async fn handle_write(&self, path: &Path, content: &str) -> Result<String, String> {
        if path.exists() {
            save_file_history(&path.to_path_buf(), &self.file_history)?;
        }

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Cannot create directory '{}': {}", parent.display(), e))?;
        }

        let content = normalize_line_endings(content);
        tokio::fs::write(path, &content)
            .await
            .map_err(|e| format!("Cannot write '{}': {}", path.display(), e))?;

        let line_count = content.lines().count();
        Ok(format!(
            "Successfully wrote {} lines to '{}'",
            line_count,
            path.display()
        ))
    }

    async fn handle_str_replace_with_editor(
        &self,
        path: &Path,
        old_str: &str,
        new_str: &str,
    ) -> Result<String, String> {
        if !path.exists() {
            return Err(format!(
                "File '{}' does not exist, you can write a new file with the `write` command",
                path.display()
            ));
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;

        if let Some(ref editor) = self.editor_model {
            save_file_history(&path.to_path_buf(), &self.file_history)?;

            match editor.edit_code(&content, old_str, new_str).await {
                Ok(updated_content) => {
                    let mut normalized_content = normalize_line_endings(&updated_content);
                    if !normalized_content.ends_with('\n') {
                        normalized_content.push('\n');
                    }

                    tokio::fs::write(path, &normalized_content)
                        .await
                        .map_err(|e| format!("Failed to write file: {}", e))?;

                    return Ok(format!("Successfully edited {}", path.display()));
                }
                Err(e) => {
                    tracing::debug!(
                        "Editor API call failed: {}, falling back to string replacement",
                        e
                    );
                }
            }
        }

        self.handle_str_replace(path, old_str, new_str).await
    }

    async fn handle_str_replace(
        &self,
        path: &Path,
        old_str: &str,
        new_str: &str,
    ) -> Result<String, String> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;

        let matches: Vec<_> = content.match_indices(old_str).collect();

        if matches.is_empty() {
            if let Some(suggestion) = self.find_similar(&content, old_str) {
                return Err(format!(
                    "No exact match found for old_str. Did you mean:\n{}",
                    suggestion
                ));
            }
            return Err(format!(
                "No match found for old_str in '{}'. Ensure it matches exactly including whitespace.",
                path.display()
            ));
        }

        if matches.len() > 1 {
            return Err(format!(
                "Found {} matches for old_str. It must be unique. Add more context.",
                matches.len()
            ));
        }

        save_file_history(&path.to_path_buf(), &self.file_history)?;

        let new_content = content.replacen(old_str, new_str, 1);
        let new_content = normalize_line_endings(&new_content);

        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| format!("Cannot write '{}': {}", path.display(), e))?;

        Ok(format!(
            "Successfully replaced text in '{}'",
            path.display()
        ))
    }

    async fn handle_insert(
        &self,
        path: &Path,
        insert_line: i32,
        text: &str,
    ) -> Result<String, String> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;

        let lines: Vec<&str> = content.lines().collect();
        let insert_idx = insert_line.max(0) as usize;

        if insert_idx > lines.len() {
            return Err(format!(
                "Insert line {} is beyond end of file ({} lines)",
                insert_line,
                lines.len()
            ));
        }

        save_file_history(&path.to_path_buf(), &self.file_history)?;

        let new_lines: Vec<&str> = text.lines().collect();
        let mut result_lines: Vec<&str> = Vec::with_capacity(lines.len() + new_lines.len());
        result_lines.extend_from_slice(&lines[..insert_idx]);
        result_lines.extend_from_slice(&new_lines);
        result_lines.extend_from_slice(&lines[insert_idx..]);

        let new_content = normalize_line_endings(&result_lines.join(
            "
",
        ));
        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| format!("Cannot write '{}': {}", path.display(), e))?;

        Ok(format!(
            "Successfully inserted {} lines after line {} in '{}'",
            new_lines.len(),
            insert_line,
            path.display()
        ))
    }

    async fn handle_undo(&self, path: &Path) -> Result<String, String> {
        let previous = {
            let mut history = self.file_history.lock().unwrap();
            history.get_mut(&path.to_path_buf()).and_then(|h| h.pop())
        };

        match previous {
            Some(content) => {
                tokio::fs::write(path, &content)
                    .await
                    .map_err(|e| format!("Cannot write '{}': {}", path.display(), e))?;
                Ok(format!(
                    "Successfully undid last edit to '{}'",
                    path.display()
                ))
            }
            None => Err(format!("No edit history for '{}'", path.display())),
        }
    }

    fn find_similar(&self, content: &str, needle: &str) -> Option<String> {
        let needle_lines: Vec<&str> = needle.lines().collect();
        if needle_lines.is_empty() {
            return None;
        }

        let content_lines: Vec<&str> = content.lines().collect();
        let window_size = needle_lines.len();

        let mut best_match: Option<(f64, usize)> = None;

        for i in 0..=content_lines.len().saturating_sub(window_size) {
            let window = &content_lines[i..i + window_size];
            let similarity = self.calculate_similarity(&needle_lines, window);

            if similarity >= SIMILARITY_THRESHOLD
                && (best_match.is_none() || similarity > best_match.unwrap().0)
            {
                best_match = Some((similarity, i));
            }
        }

        best_match.map(|(_, start)| {
            content_lines[start..start + window_size].join(
                "
",
            )
        })
    }

    fn calculate_similarity(&self, a: &[&str], b: &[&str]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }

        let total: f64 = a
            .iter()
            .zip(b.iter())
            .map(|(l1, l2)| self.line_similarity(l1, l2))
            .sum();

        total / a.len() as f64
    }

    fn line_similarity(&self, a: &str, b: &str) -> f64 {
        if a == b {
            return 1.0;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        if a_chars.is_empty() && b_chars.is_empty() {
            return 1.0;
        }

        let max_len = a_chars.len().max(b_chars.len());
        if max_len == 0 {
            return 1.0;
        }

        let matches = a_chars
            .iter()
            .zip(b_chars.iter())
            .filter(|(c1, c2)| c1 == c2)
            .count();

        matches as f64 / max_len as f64
    }

    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, String> {
        let expanded: String = shellexpand::tilde(path_str).into();
        let path = Path::new(&expanded);

        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_dir.join(path)
        };

        self.validate_path(&resolved)?;

        Ok(resolved)
    }

    fn validate_path(&self, path: &Path) -> Result<(), String> {
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("Path traversal detected: paths cannot contain '..'".to_string());
        }

        if path.exists() {
            if let (Ok(canonical_path), Ok(canonical_base)) =
                (path.canonicalize(), self.working_dir.canonicalize())
            {
                if !canonical_path.starts_with(&canonical_base) {
                    return Err(format!(
                        "Path '{}' is outside the working directory",
                        path.display()
                    ));
                }
            }

            if let Ok(metadata) = path.symlink_metadata() {
                if metadata.is_symlink() {
                    return Err(format!(
                        "Cannot modify symlink '{}'. Please operate on the actual file.",
                        path.display()
                    ));
                }
            }
        } else if let Some(parent) = path.parent() {
            if let (Ok(canonical_parent), Ok(canonical_base)) =
                (parent.canonicalize(), self.working_dir.canonicalize())
            {
                if !canonical_parent.starts_with(&canonical_base) {
                    return Err(format!(
                        "Path '{}' would be outside the working directory",
                        path.display()
                    ));
                }
            }
        }

        if self.is_ignored(path) {
            return Err(format!(
                "Access to '{}' is restricted by .gooseignore",
                path.display()
            ));
        }

        Ok(())
    }
}

fn normalize_line_endings(content: &str) -> String {
    #[cfg(windows)]
    {
        content.replace("\r\n", "\n").replace('\n', "\r\n")
    }
    #[cfg(not(windows))]
    {
        content.replace("\r\n", "\n")
    }
}

#[derive(Debug, Default)]
struct DiffResults {
    files_created: usize,
    files_modified: usize,
    lines_added: usize,
    lines_removed: usize,
}

fn save_file_history(
    path: &PathBuf,
    file_history: &Arc<std::sync::Mutex<HashMap<PathBuf, Vec<String>>>>,
) -> Result<(), String> {
    let mut history = file_history.lock().unwrap();
    let content = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?
    } else {
        String::new()
    };
    history.entry(path.clone()).or_default().push(content);
    Ok(())
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

fn apply_single_patch(
    patch: &mpatch::Patch,
    base_dir: &Path,
    file_history: &Arc<std::sync::Mutex<HashMap<PathBuf, Vec<String>>>>,
    results: &mut DiffResults,
    failed_hunks: &mut Vec<String>,
) -> Result<(), String> {
    let adjusted_base_dir = adjust_base_dir_for_overlap(base_dir, &patch.file_path);
    let file_path = adjusted_base_dir.join(&patch.file_path);

    let file_existed = file_path.exists();
    if file_existed {
        save_file_history(&file_path, file_history)?;
    }

    let success = apply_patch(patch, &adjusted_base_dir, false, 0.7).map_err(|e| match e {
        PatchError::Io { path, source } => {
            format!("Failed to process '{}': {}", path.display(), source)
        }
        PatchError::PathTraversal(path) => {
            format!(
                "Security: Path '{}' would escape the base directory",
                path.display()
            )
        }
        PatchError::TargetNotFound(path) => {
            format!(
                "File '{}' not found and patch doesn't create it",
                path.display()
            )
        }
        PatchError::MissingFileHeader => "Invalid patch format".to_string(),
    })?;

    if !success {
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

    if file_existed {
        results.files_modified += 1;
    } else {
        results.files_created += 1;
    }

    Ok(())
}

fn parse_diff_content(diff_content: &str) -> Result<Vec<mpatch::Patch>, String> {
    let wrapped_diff = if diff_content.contains("```diff") || diff_content.contains("```patch") {
        diff_content.to_string()
    } else {
        format!("```diff\n{}\n```", diff_content)
    };

    parse_diffs(&wrapped_diff).map_err(|e| match e {
        PatchError::MissingFileHeader => {
            "Invalid diff format: Missing file header (e.g., '--- a/path/to/file')".to_string()
        }
        PatchError::Io { path, source } => {
            format!("I/O error processing {}: {}", path.display(), source)
        }
        PatchError::PathTraversal(path) => {
            format!(
                "Security: Path '{}' would escape the base directory",
                path.display()
            )
        }
        PatchError::TargetNotFound(path) => {
            format!("Target file not found: {}", path.display())
        }
    })
}

fn ensure_trailing_newlines(patches: &[mpatch::Patch], base_dir: &Path) -> Result<(), String> {
    for patch in patches {
        let adjusted_base_dir = adjust_base_dir_for_overlap(base_dir, &patch.file_path);
        let file_path = adjusted_base_dir.join(&patch.file_path);

        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read file for post-processing: {}", e))?;

            if !content.ends_with('\n') {
                let content_with_newline = format!("{}\n", content);
                std::fs::write(&file_path, content_with_newline)
                    .map_err(|e| format!("Failed to add trailing newline: {}", e))?;
            }
        }
    }
    Ok(())
}

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

async fn apply_diff(
    base_path: &Path,
    diff_content: &str,
    file_history: &Arc<std::sync::Mutex<HashMap<PathBuf, Vec<String>>>>,
) -> Result<String, String> {
    if diff_content.len() > MAX_DIFF_SIZE {
        return Err(format!(
            "Diff is too large ({} bytes). Maximum size is {} bytes (1MB).",
            diff_content.len(),
            MAX_DIFF_SIZE
        ));
    }

    let patches = parse_diff_content(diff_content)?;

    if patches.len() > MAX_FILES_IN_DIFF {
        return Err(format!(
            "Too many files in diff ({}). Maximum is {} files.",
            patches.len(),
            MAX_FILES_IN_DIFF
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

    if !failed_hunks.is_empty() {
        tracing::warn!(
            "Some patches were only partially applied: {}",
            failed_hunks.join("\n")
        );
    }

    let (lines_added, lines_removed) = count_line_changes(diff_content);
    results.lines_added = lines_added;
    results.lines_removed = lines_removed;

    let summary = if patches.len() == 1 {
        format!(
            "Successfully applied diff to {}:\n• Lines added: {}\n• Lines removed: {}",
            base_path.display(),
            results.lines_added,
            results.lines_removed
        )
    } else {
        format!(
            "Successfully applied multi-file diff:\n• Files created: {}\n• Files modified: {}\n• Lines added: {}\n• Lines removed: {}",
            results.files_created,
            results.files_modified,
            results.lines_added,
            results.lines_removed
        )
    };

    Ok(summary)
}

#[async_trait]
impl McpClientTrait for TextEditorClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        _session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        _working_dir: Option<&str>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        if name != "text_editor" {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Unknown tool: {}",
                name
            ))]));
        }

        let params: TextEditorParams = match arguments {
            Some(args) => match serde_json::from_value(serde_json::Value::Object(args)) {
                Ok(p) => p,
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Invalid parameters: {}",
                        e
                    ))]));
                }
            },
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Missing parameters",
                )]));
            }
        };

        let path = match self.resolve_path(&params.path) {
            Ok(p) => p,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(e)]));
            }
        };

        let result = match params.command.as_str() {
            "view" => {
                self.handle_view(&path, params.view_range_start, params.view_range_end)
                    .await
            }
            "write" => {
                let text = params.file_text.unwrap_or_default();
                self.handle_write(&path, &text).await
            }
            "str_replace" => {
                if let Some(diff_content) = params.diff {
                    apply_diff(&path, &diff_content, &self.file_history).await
                } else {
                    let old_str = params.old_str.unwrap_or_default();
                    let new_str = params.new_str.unwrap_or_default();
                    self.handle_str_replace_with_editor(&path, &old_str, &new_str)
                        .await
                }
            }
            "insert" => {
                let line = params.insert_line.unwrap_or(0);
                let text = params.file_text.unwrap_or_default();
                self.handle_insert(&path, line, &text).await
            }
            "undo_edit" => self.handle_undo(&path).await,
            "apply_diff" => {
                let diff_content = params.diff.unwrap_or_default();
                apply_diff(&path, &diff_content, &self.file_history).await
            }
            cmd => Err(format!(
                "Unknown command: '{}'. Use: view, write, str_replace, insert, undo_edit, apply_diff",
                cmd
            )),
        };

        match result {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_client() -> TextEditorClient {
        let context = PlatformExtensionContext {
            extension_manager: None,
            session_manager: std::sync::Arc::new(crate::session::SessionManager::new(
                std::env::temp_dir(),
            )),
        };
        TextEditorClient::new(context).unwrap()
    }

    #[tokio::test]
    async fn test_write_and_view() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let client = create_test_client();

        let result = client
            .handle_write(
                &file_path,
                "line 1
line 2
line 3",
            )
            .await;
        assert!(result.is_ok());

        let result = client.handle_view(&file_path, None, None).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("line 1"));
    }

    #[tokio::test]
    async fn test_str_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let client = create_test_client();
        client
            .handle_write(&file_path, "hello world")
            .await
            .unwrap();

        let result = client.handle_str_replace(&file_path, "world", "rust").await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(content.contains("hello rust"));
    }

    #[tokio::test]
    async fn test_undo() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let client = create_test_client();
        client.handle_write(&file_path, "original").await.unwrap();
        client.handle_write(&file_path, "modified").await.unwrap();

        let result = client.handle_undo(&file_path).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content.trim(), "original");
    }

    #[tokio::test]
    async fn test_apply_diff() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        std::fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        let diff = r#"--- a/test.txt
+++ b/test.txt
@@ -1,3 +1,3 @@
 line1
-line2
+modified line2
 line3
"#;

        let file_history = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let result = apply_diff(temp_dir.path(), diff, &file_history).await;
        assert!(result.is_ok(), "apply_diff failed: {:?}", result);

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("modified line2"));
        assert!(!content.contains("\nline2\n"));
    }

    #[tokio::test]
    async fn test_view_directory() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();

        let client = create_test_client();
        let result = client.handle_view(temp_dir.path(), None, None).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("is a directory"));
        assert!(output.contains("subdir/"));
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.txt"));
    }
}
