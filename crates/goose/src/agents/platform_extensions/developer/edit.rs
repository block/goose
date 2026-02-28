use std::fs;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

const NO_MATCH_PREVIEW_LINES: usize = 20;
const MAX_READ_FILE_BYTES: u64 = 1024 * 1024;
const DEFAULT_READ_LIMIT_LINES: usize = 500;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteParams {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileReadParams {
    pub path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
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

    pub fn file_write(&self, params: FileWriteParams) -> CallToolResult {
        self.file_write_with_cwd(params, None)
    }

    pub fn file_write_with_cwd(
        &self,
        params: FileWriteParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);

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

    pub fn file_read(&self, params: FileReadParams) -> CallToolResult {
        self.file_read_with_cwd(params, None)
    }

    pub fn file_read_with_cwd(
        &self,
        params: FileReadParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);

        if params.offset.is_some() || params.limit.is_some() {
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(DEFAULT_READ_LIMIT_LINES);
            if limit == 0 {
                return CallToolResult::error(vec![Content::text(
                    "Failed to read: limit must be greater than 0",
                )
                .with_priority(0.0)]);
            }

            return match read_file_chunk_by_lines(&path, offset, limit) {
                Ok(content) => {
                    CallToolResult::success(vec![Content::text(content).with_priority(0.0)])
                }
                Err(error) => CallToolResult::error(vec![Content::text(format!(
                    "Failed to read {}: {}",
                    params.path, error
                ))
                .with_priority(0.0)]),
            };
        }

        match read_file_with_size_limit(&path, &params.path, MAX_READ_FILE_BYTES) {
            Ok(content) => CallToolResult::success(vec![Content::text(content).with_priority(0.0)]),
            Err(error) => {
                let message = if error.contains("exceeds max file size") {
                    format!(
                        "{}. Retry with offset and limit to read the file in chunks.",
                        error
                    )
                } else {
                    error
                };
                CallToolResult::error(vec![Content::text(message).with_priority(0.0)])
            }
        }
    }

    pub fn file_edit(&self, params: FileEditParams) -> CallToolResult {
        self.file_edit_with_cwd(params, None)
    }

    pub fn file_edit_with_cwd(
        &self,
        params: FileEditParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);

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

        let matches: Vec<_> = content.match_indices(&params.before).collect();

        match matches.len() {
            0 => {
                let suggestion = find_similar_context(&content, &params.before);
                let mut msg = "No match found for the specified text.".to_string();
                if let Some(hint) = suggestion {
                    msg.push_str(&format!("\n\nDid you mean:\n```\n{}\n```", hint));
                }
                let preview = build_file_preview(&content, NO_MATCH_PREVIEW_LINES);
                msg.push_str(&format!("\n\nFile preview:\n```\n{}\n```", preview));
                CallToolResult::error(vec![Content::text(msg).with_priority(0.0)])
            }
            1 => {
                let new_content = content.replacen(&params.before, &params.after, 1);

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
            n => {
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

                CallToolResult::error(vec![Content::text(msg).with_priority(0.0)])
            }
        }
    }
}

impl Default for EditTools {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_path(path: &str, working_dir: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        working_dir
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(path)
    }
}

fn read_file_with_size_limit(
    path: &Path,
    display_path: &str,
    max_bytes: u64,
) -> Result<String, String> {
    match fs::metadata(path) {
        Ok(metadata) => {
            let file_size = metadata.len();
            if file_size > max_bytes {
                return Err(format!(
                    "{} exceeds max file size of {} bytes (actual: {} bytes)",
                    display_path, max_bytes, file_size
                ));
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to stat {} before reading: {}",
                display_path, error
            ));
        }
    }

    fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", display_path, e))
}

fn read_file_chunk_by_lines(path: &Path, offset: usize, limit: usize) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let mut output = String::new();
    let mut line = String::new();
    let mut line_index = 0usize;

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }

        if line_index >= offset && line_index < offset.saturating_add(limit) {
            output.push_str(&line);
        }
        if line_index >= offset.saturating_add(limit) {
            break;
        }

        line_index += 1;
    }

    Ok(output)
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
    use crate::agents::platform_extensions::MAX_READ_FILE_BYTES;
    use rmcp::model::RawContent;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn extract_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(text) => &text.text,
            _ => panic!("expected text"),
        }
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
    fn test_file_read_success() {
        let dir = setup();
        let path = dir.path().join("read.txt");
        fs::write(&path, "read me").unwrap();
        let tools = EditTools::new();

        let result = tools.file_read(FileReadParams {
            path: path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
        });

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "read me");
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
        );

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            fs::read_to_string(dir.path().join("relative-edit.txt")).unwrap(),
            "after"
        );
    }

    #[test]
    fn test_file_read_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        fs::write(dir.path().join("relative-read.txt"), "relative read").unwrap();
        let tools = EditTools::new();

        let result = tools.file_read_with_cwd(
            FileReadParams {
                path: "relative-read.txt".to_string(),
                offset: None,
                limit: None,
            },
            Some(dir.path()),
        );

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "relative read");
    }

    #[test]
    fn test_file_read_rejects_oversized_file() {
        let dir = setup();
        let path = dir.path().join("large-read.txt");
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_READ_FILE_BYTES + 1).unwrap();
        let tools = EditTools::new();

        let result = tools.file_read(FileReadParams {
            path: path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
        });

        assert!(result.is_error.unwrap_or(false));
        let text = extract_text(&result);
        assert!(text.contains("exceeds max file size"));
    }

    #[test]
    fn test_file_read_with_offset_and_limit() {
        let dir = setup();
        let path = dir.path().join("chunk-read.txt");
        fs::write(&path, "l1\nl2\nl3\nl4\nl5").unwrap();
        let tools = EditTools::new();

        let result = tools.file_read(FileReadParams {
            path: path.to_string_lossy().to_string(),
            offset: Some(1),
            limit: Some(2),
        });

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "l2\nl3\n");
    }

    #[test]
    fn test_file_read_with_limit_allows_large_file() {
        let dir = setup();
        let path = dir.path().join("large-chunk-read.txt");
        let mut content = String::new();
        for _ in 0..220_000 {
            content.push_str("line\n");
        }
        fs::write(&path, content).unwrap();
        let tools = EditTools::new();

        let result = tools.file_read(FileReadParams {
            path: path.to_string_lossy().to_string(),
            offset: Some(0),
            limit: Some(3),
        });

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "line\nline\nline\n");
    }
}
