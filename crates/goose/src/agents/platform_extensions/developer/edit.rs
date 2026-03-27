use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

const NO_MATCH_PREVIEW_LINES: usize = 20;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReadParams {
    /// Absolute path to the file to read.
    pub path: String,
    /// Line number to start reading from (1-based).
    #[schemars(range(min = 1))]
    pub line: Option<u32>,
    /// Maximum number of lines to read.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteParams {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileEditParams {
    pub path: String,
    pub before: String,
    pub after: String,
}

pub struct EditTools;

impl EditTools {
    pub fn new() -> Self {
        Self
    }

    pub fn file_read_with_cwd(
        &self,
        params: FileReadParams,
        working_dir: Option<&Path>,
        allowed_paths: Option<&HashSet<PathBuf>>,
    ) -> CallToolResult {
        let path = match validate_and_resolve_path(&params.path, working_dir, allowed_paths) {
            Ok(p) => p,
            Err(msg) => return CallToolResult::error(vec![Content::text(msg).with_priority(0.0)]),
        };

        match fs::read_to_string(&path) {
            Ok(content) => {
                let content = apply_line_limit(&content, params.line, params.limit);
                CallToolResult::success(vec![Content::text(content).with_priority(0.0)])
            }
            Err(error) => CallToolResult::error(vec![Content::text(format!(
                "Failed to read {}: {}",
                params.path, error
            ))
            .with_priority(0.0)]),
        }
    }

    pub fn file_write(&self, params: FileWriteParams) -> CallToolResult {
        self.file_write_with_cwd(params, None, None)
    }

    pub fn file_write_with_cwd(
        &self,
        params: FileWriteParams,
        working_dir: Option<&Path>,
        allowed_paths: Option<&HashSet<PathBuf>>,
    ) -> CallToolResult {
        let path = match validate_and_resolve_path(&params.path, working_dir, allowed_paths) {
            Ok(p) => p,
            Err(msg) => return CallToolResult::error(vec![Content::text(msg).with_priority(0.0)]),
        };

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                if let Err(error) = fs::create_dir_all(parent) {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        error
                    ))
                    .with_priority(0.0)]);
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
                ))
                .with_priority(0.0)])
            }
            Err(error) => CallToolResult::error(vec![Content::text(format!(
                "Failed to write {}: {}",
                params.path, error
            ))
            .with_priority(0.0)]),
        }
    }

    pub fn file_edit(&self, params: FileEditParams) -> CallToolResult {
        self.file_edit_with_cwd(params, None, None)
    }

    pub fn file_edit_with_cwd(
        &self,
        params: FileEditParams,
        working_dir: Option<&Path>,
        allowed_paths: Option<&HashSet<PathBuf>>,
    ) -> CallToolResult {
        let path = match validate_and_resolve_path(&params.path, working_dir, allowed_paths) {
            Ok(p) => p,
            Err(msg) => return CallToolResult::error(vec![Content::text(msg).with_priority(0.0)]),
        };

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(error) => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Failed to read {}: {}",
                    params.path, error
                ))
                .with_priority(0.0)]);
            }
        };

        let new_content = match string_replace(&content, &params.before, &params.after) {
            Ok(c) => c,
            Err(msg) => {
                return CallToolResult::error(vec![Content::text(msg).with_priority(0.0)]);
            }
        };
        match fs::write(&path, &new_content) {
            Ok(()) => {
                let old_lines = params.before.lines().count();
                let new_lines = params.after.lines().count();
                CallToolResult::success(vec![Content::text(format!(
                    "Edited {} ({} lines -> {} lines)",
                    params.path, old_lines, new_lines
                ))
                .with_priority(0.0)])
            }
            Err(error) => CallToolResult::error(vec![Content::text(format!(
                "Failed to write {}: {}",
                params.path, error
            ))
            .with_priority(0.0)]),
        }
    }
}

impl Default for EditTools {
    fn default() -> Self {
        Self::new()
    }
}

pub fn string_replace(content: &str, before: &str, after: &str) -> Result<String, String> {
    let matches: Vec<_> = content.match_indices(before).collect();

    match matches.len() {
        0 => {
            let suggestion = find_similar_context(content, before);
            let mut msg = "No match found for the specified text.".to_string();
            if let Some(hint) = suggestion {
                msg.push_str(&format!("\n\nDid you mean:\n```\n{}\n```", hint));
            }
            let preview = build_file_preview(content, NO_MATCH_PREVIEW_LINES);
            msg.push_str(&format!("\n\nFile preview:\n```\n{}\n```", preview));
            Err(msg)
        }
        1 => Ok(content.replacen(before, after, 1)),
        n => {
            let mut msg = format!(
                "Found {} matches. Please provide more context to identify a unique match:\n",
                n
            );

            for (i, (pos, _)) in matches.iter().enumerate().take(2) {
                let line_num = count_lines_before(content, *pos);
                let context = get_line_context(content, line_num, 1);
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

            Err(msg)
        }
    }
}

fn apply_line_limit(content: &str, line: Option<u32>, limit: Option<u32>) -> String {
    if line.is_none() && limit.is_none() {
        return content.to_string();
    }
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let start = line
        .map(|l| (l as usize).saturating_sub(1))
        .unwrap_or(0)
        .min(lines.len());
    let end = limit
        .map(|l| start + l as usize)
        .unwrap_or(lines.len())
        .min(lines.len());
    lines[start..end].concat()
}

/// Resolve a user-supplied path, optionally enforcing confinement to a set of allowed base paths.
///
/// * `working_dir` — used to resolve relative paths. Falls back to `cwd` when `None`.
/// * `allowed_paths` — when `Some`, the resolved path must be inside at least one of the
///   listed base directories. When `None`, no confinement is enforced.
pub fn validate_and_resolve_path(
    path: &str,
    working_dir: Option<&Path>,
    allowed_paths: Option<&HashSet<PathBuf>>,
) -> Result<PathBuf, String> {
    let path_buf = PathBuf::from(path);

    // Resolve relative paths against working_dir (or cwd).
    let cwd_fallback;
    let base = match working_dir {
        Some(dir) => dir,
        None => {
            cwd_fallback = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            cwd_fallback.as_path()
        }
    };
    let joined = if path_buf.is_absolute() {
        path_buf
    } else {
        base.join(&path_buf)
    };

    // If no confinement, just resolve and return.
    let Some(bases) = allowed_paths else {
        return Ok(joined);
    };

    // Build the set of canonical base paths for confinement checks.
    let canonical_bases: Vec<PathBuf> =
        bases.iter().filter_map(|b| b.canonicalize().ok()).collect();
    if canonical_bases.is_empty() && !bases.is_empty() {
        return Err("Failed to resolve allowed base directories for confinement".to_string());
    }

    let is_within_bases = |p: &Path| canonical_bases.iter().any(|cb| p.starts_with(cb));

    // Walk the path component-by-component through the filesystem,
    // then validate the result against allowed base directories.
    let resolved = resolve_confined(&joined, path)?;

    if !is_within_bases(&resolved) {
        return Err(format!("Path escapes allowed directories: {path}"));
    }
    Ok(resolved)
}

/// Walk path components, resolving each through the filesystem where possible.
///
/// Existing components are canonicalized (following symlinks). Once a non-existent
/// component is encountered, subsequent `..` and `.` are rejected outright — they
/// are ambiguous without the filesystem and create path-confusion escapes. Dangling
/// symlinks (exist but can't be canonicalized) are also rejected since their targets
/// cannot be verified.
fn resolve_confined(joined: &Path, original_path: &str) -> Result<PathBuf, String> {
    let mut resolved = PathBuf::new();
    let mut past_fs = false;

    for component in joined.components() {
        if past_fs {
            // Beyond the filesystem boundary: only plain names are safe.
            match component {
                std::path::Component::ParentDir | std::path::Component::CurDir => {
                    return Err(format!(
                        "Path contains traversal after non-existent segment: {original_path}"
                    ));
                }
                _ => resolved.push(component),
            }
            continue;
        }

        let candidate = resolved.join(component.as_os_str());
        if candidate.symlink_metadata().is_ok() {
            // Component exists — canonicalize to follow symlinks.
            match candidate.canonicalize() {
                Ok(canonical) => resolved = canonical,
                Err(_) => {
                    // Exists but can't canonicalize (dangling symlink). We cannot
                    // verify where it points, so reject under confinement.
                    return Err(format!(
                        "Path contains an unresolvable symlink: {original_path}"
                    ));
                }
            }
        } else {
            // Component doesn't exist on the filesystem.
            resolved.push(component);
            past_fs = true;
        }
    }

    Ok(resolved)
}

fn count_lines_before(content: &str, byte_pos: usize) -> usize {
    content
        .char_indices()
        .take_while(|(i, _)| *i < byte_pos)
        .filter(|(_, c)| *c == '\n')
        .count()
        + 1
}

fn get_line_context(content: &str, target_line: usize, context: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = target_line.saturating_sub(context + 1);
    let end = (target_line + context).min(lines.len());

    lines[start..end].join("\n")
}

fn find_similar_context(content: &str, search: &str) -> Option<String> {
    let first_line = search.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }

    for (i, line) in content.lines().enumerate() {
        if line.contains(first_line) || first_line.contains(line.trim()) {
            return Some(get_line_context(content, i + 1, 2));
        }
    }

    None
}

fn build_file_preview(content: &str, max_lines: usize) -> String {
    if content.is_empty() {
        return "(file is empty)".to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    let preview_end = lines.len().min(max_lines);
    let mut preview = lines[..preview_end]
        .iter()
        .enumerate()
        .map(|(index, line)| format!("{:>4}: {}", index + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    if lines.len() > preview_end {
        preview.push_str(&format!("\n... ({} more lines)", lines.len() - preview_end));
    }

    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::RawContent;
    use std::fs;
    use tempfile::TempDir;
    use test_case::test_case;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn extract_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(text) => &text.text,
            _ => panic!("expected text"),
        }
    }

    #[test_case(None, None, "line1\nline2\nline3" ; "full content")]
    #[test_case(Some(2), None, "line2\nline3" ; "from line 2")]
    #[test_case(None, Some(2), "line1\nline2\n" ; "limit 2")]
    #[test_case(Some(2), Some(1), "line2\n" ; "line 2 limit 1")]
    #[test_case(Some(99), None, "" ; "beyond eof")]
    fn test_apply_line_limit(line: Option<u32>, limit: Option<u32>, expected: &str) {
        assert_eq!(
            apply_line_limit("line1\nline2\nline3", line, limit),
            expected
        );
    }

    #[test]
    fn test_file_read() {
        let dir = setup();
        let path = dir.path().join("read.txt");
        fs::write(&path, "line1\nline2\nline3").unwrap();
        let tools = EditTools::new();

        let result = tools.file_read_with_cwd(
            FileReadParams {
                path: path.to_string_lossy().to_string(),
                line: None,
                limit: None,
            },
            None,
            None,
        );

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "line1\nline2\nline3");
    }

    #[test]
    fn test_file_read_partial() {
        let dir = setup();
        let path = dir.path().join("read.txt");
        fs::write(&path, "line1\nline2\nline3").unwrap();
        let tools = EditTools::new();

        let result = tools.file_read_with_cwd(
            FileReadParams {
                path: path.to_string_lossy().to_string(),
                line: Some(2),
                limit: Some(1),
            },
            None,
            None,
        );

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "line2\n");
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
        let text = extract_text(&result);
        assert!(text.contains("No match found"));
        assert!(text.contains("File preview:"));
        assert!(text.contains("some content"));
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

    #[test]
    fn test_file_write_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        let tools = EditTools::new();

        let result = tools.file_write_with_cwd(
            FileWriteParams {
                path: "relative.txt".to_string(),
                content: "relative write".to_string(),
            },
            Some(dir.path()),
            None,
        );

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            fs::read_to_string(dir.path().join("relative.txt")).unwrap(),
            "relative write"
        );
    }

    #[test]
    fn test_file_edit_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        fs::write(dir.path().join("relative-edit.txt"), "before").unwrap();
        let tools = EditTools::new();

        let result = tools.file_edit_with_cwd(
            FileEditParams {
                path: "relative-edit.txt".to_string(),
                before: "before".to_string(),
                after: "after".to_string(),
            },
            Some(dir.path()),
            None,
        );

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            fs::read_to_string(dir.path().join("relative-edit.txt")).unwrap(),
            "after"
        );
    }

    // --- Path confinement tests ---

    /// Helper: create a confined set containing just the given directory.
    fn confined(dir: &Path) -> HashSet<PathBuf> {
        HashSet::from([dir.to_path_buf()])
    }

    #[test]
    fn test_absolute_path_outside_allowed_dirs_rejected() {
        let dir = setup();
        let allowed = confined(dir.path());
        let result = validate_and_resolve_path("/etc/passwd", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escapes allowed directories"));
    }

    #[test]
    fn test_absolute_path_inside_allowed_dir_allowed() {
        let dir = setup();
        let file = dir.path().join("allowed.txt");
        fs::write(&file, "ok").unwrap();
        let abs_path = file.to_string_lossy().to_string();
        let allowed = confined(dir.path());
        let result = validate_and_resolve_path(&abs_path, Some(dir.path()), Some(&allowed));
        assert!(result.is_ok());
    }

    #[test]
    fn test_dotdot_traversal_rejected() {
        let dir = setup();
        let allowed = confined(dir.path());
        let result =
            validate_and_resolve_path("../../etc/passwd", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escapes allowed directories"));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_outside_allowed_dir_rejected() {
        let dir = setup();
        let outside = setup();
        let target = outside.path().join("outside.txt");
        fs::write(&target, "secret").unwrap();

        std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();

        let allowed = confined(dir.path());
        let result = validate_and_resolve_path("link.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escapes allowed directories"));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_inside_allowed_dir_allowed() {
        let dir = setup();
        let target = dir.path().join("real.txt");
        fs::write(&target, "ok").unwrap();

        std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();

        let allowed = confined(dir.path());
        let result = validate_and_resolve_path("link.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_new_file_within_allowed_dir() {
        let dir = setup();
        fs::create_dir_all(dir.path().join("subdir")).unwrap();
        let allowed = confined(dir.path());
        let result =
            validate_and_resolve_path("subdir/new_file.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_ok());
        assert!(result
            .unwrap()
            .starts_with(dir.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_write_new_file_dotdot_escape_rejected() {
        let dir = setup();
        let allowed = confined(dir.path());
        let result = validate_and_resolve_path("../escape.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escapes allowed directories"));
    }

    #[test]
    fn test_no_confinement_allows_absolute_paths() {
        let result = validate_and_resolve_path("/tmp/some_file.txt", None, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/tmp/some_file.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_ancestor_escape_on_new_file_rejected() {
        // A symlink dir inside the workspace pointing outside: link -> /outside
        // Writing to link/newdir/file.txt should be rejected because the ancestor
        // "link" resolves outside the allowed directory.
        let dir = setup();
        let outside = setup();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();

        let allowed = confined(dir.path());
        let result =
            validate_and_resolve_path("link/newdir/file.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escapes allowed directories"));
    }

    #[cfg(unix)]
    #[test]
    fn test_dangling_symlink_outside_allowed_dir_rejected() {
        // A dangling symlink (target doesn't exist) pointing outside the workspace
        // should be rejected even though the parent dir is inside.
        let dir = setup();
        std::os::unix::fs::symlink("/tmp/nonexistent_target.txt", dir.path().join("link.txt"))
            .unwrap();

        let allowed = confined(dir.path());
        let result = validate_and_resolve_path("link.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("symlink"));
    }

    #[test]
    fn test_dotdot_traversal_with_nonexistent_prefix_rejected() {
        // a/../../escape.txt should be rejected even when "a" doesn't exist,
        // because .. after a non-existent component is always rejected.
        let dir = setup();
        let allowed = confined(dir.path());
        let result =
            validate_and_resolve_path("a/../../escape.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("traversal after non-existent"));
    }

    #[test]
    fn test_multiple_allowed_paths() {
        let dir1 = setup();
        let dir2 = setup();
        let file1 = dir1.path().join("file1.txt");
        let file2 = dir2.path().join("file2.txt");
        fs::write(&file1, "ok").unwrap();
        fs::write(&file2, "ok").unwrap();

        let allowed = HashSet::from([dir1.path().to_path_buf(), dir2.path().to_path_buf()]);

        // File in dir1 is allowed
        let result =
            validate_and_resolve_path(&file1.to_string_lossy(), Some(dir1.path()), Some(&allowed));
        assert!(result.is_ok());

        // File in dir2 is also allowed
        let result =
            validate_and_resolve_path(&file2.to_string_lossy(), Some(dir1.path()), Some(&allowed));
        assert!(result.is_ok());

        // File outside both is rejected
        let result = validate_and_resolve_path("/etc/passwd", Some(dir1.path()), Some(&allowed));
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_dotdot_across_symlink_rejected() {
        // link -> /tmp/outside, then "link/../newdir/file.txt" should be rejected
        // because `link/..` resolves through the symlink to /tmp, escaping the workspace.
        let dir = setup();
        let outside = setup();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();

        let allowed = confined(dir.path());
        let result =
            validate_and_resolve_path("link/../newdir/file.txt", Some(dir.path()), Some(&allowed));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("escapes allowed directories"));
    }

    #[cfg(unix)]
    #[test]
    fn test_dotdot_rewind_into_symlink_after_missing_rejected() {
        // missing/../link/new/file.txt — "missing" doesn't exist so `..` after it
        // is rejected outright (traversal after non-existent segment).
        let dir = setup();
        let outside = setup();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();

        let allowed = confined(dir.path());
        let result = validate_and_resolve_path(
            "missing/../link/new/file.txt",
            Some(dir.path()),
            Some(&allowed),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("traversal after non-existent"));
    }
}
