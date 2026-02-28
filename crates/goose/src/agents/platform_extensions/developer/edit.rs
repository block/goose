use std::path::{Path, PathBuf};

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::agents::platform_extensions::{
    DeveloperFileIo, ReadFileChunkFn, ReadFileFn, WriteFileFn,
};

const NO_MATCH_PREVIEW_LINES: usize = 20;
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

pub struct EditTools {
    read_file: ReadFileFn,
    write_file: WriteFileFn,
    read_file_chunk: Option<ReadFileChunkFn>,
}

impl EditTools {
    pub fn with_file_io(
        read_file: ReadFileFn,
        read_file_chunk: Option<ReadFileChunkFn>,
        write_file: WriteFileFn,
    ) -> Self {
        Self {
            read_file,
            write_file,
            read_file_chunk,
        }
    }

    pub async fn file_write(&self, params: FileWriteParams) -> CallToolResult {
        self.file_write_with_cwd(params, None).await
    }

    pub async fn file_read(&self, params: FileReadParams) -> CallToolResult {
        self.file_read_with_cwd(params, None).await
    }

    pub async fn file_read_with_cwd(
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
            if let Some(read_file_chunk) = &self.read_file_chunk {
                match read_file_chunk(path, offset, limit).await {
                    Ok(chunk) => {
                        return CallToolResult::success(vec![
                            Content::text(chunk).with_priority(0.0)
                        ]);
                    }
                    Err(error) => {
                        return CallToolResult::error(vec![Content::text(format!(
                            "Failed to read {}: {}",
                            params.path, error
                        ))
                        .with_priority(0.0)]);
                    }
                }
            }
            // Fallback for delegated transports (e.g. ACP) that don't support
            // offset/limit chunk reads yet: read once, then slice locally.
            return match (self.read_file)(path).await {
                Ok(content) => CallToolResult::success(vec![Content::text(slice_lines(
                    &content, offset, limit,
                ))
                .with_priority(0.0)]),
                Err(error) => CallToolResult::error(vec![Content::text(format!(
                    "Failed to read {}: {}",
                    params.path, error
                ))
                .with_priority(0.0)]),
            };
        }

        match (self.read_file)(path).await {
            Ok(content) => CallToolResult::success(vec![Content::text(content).with_priority(0.0)]),
            Err(error) => {
                let error_text = error.to_string();
                let message = if self.read_file_chunk.is_some()
                    && error_text.contains("exceeds max file size")
                {
                    format!(
                        "Failed to read {}: {}. Retry with offset and limit to read the file in chunks.",
                        params.path, error_text
                    )
                } else {
                    format!("Failed to read {}: {}", params.path, error_text)
                };
                CallToolResult::error(vec![Content::text(message).with_priority(0.0)])
            }
        }
    }

    pub async fn file_write_with_cwd(
        &self,
        params: FileWriteParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let FileWriteParams {
            path: display_path,
            content,
        } = params;
        let path = resolve_path(&display_path, working_dir);
        let line_count = content.lines().count();

        match (self.write_file)(path, content).await {
            Ok(()) => CallToolResult::success(vec![Content::text(format!(
                "Wrote {} ({} lines)",
                display_path, line_count
            ))
            .with_priority(0.0)]),
            Err(error) => CallToolResult::error(vec![Content::text(format!(
                "Failed to write {}: {}",
                display_path, error
            ))
            .with_priority(0.0)]),
        }
    }

    pub async fn file_edit(&self, params: FileEditParams) -> CallToolResult {
        self.file_edit_with_cwd(params, None).await
    }

    pub async fn file_edit_with_cwd(
        &self,
        params: FileEditParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);

        let content = match (self.read_file)(path.clone()).await {
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

                match (self.write_file)(path, new_content).await {
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
        let local_io = DeveloperFileIo::default_local();
        Self {
            read_file: local_io.read_file,
            write_file: local_io.write_file,
            read_file_chunk: local_io.read_file_chunk,
        }
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

fn slice_lines(content: &str, offset: usize, limit: usize) -> String {
    content
        .lines()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>()
        .join("\n")
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
    use std::sync::Arc;
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

    #[tokio::test]
    async fn test_file_write_new() {
        let dir = setup();
        let path = dir.path().join("new_file.txt");
        let tools = EditTools::default();

        let result = tools
            .file_write(FileWriteParams {
                path: path.to_string_lossy().to_string(),
                content: "Hello, world!\nLine 2".to_string(),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello, world!\nLine 2");
    }

    #[tokio::test]
    async fn test_file_read_success() {
        let dir = setup();
        let path = dir.path().join("read.txt");
        fs::write(&path, "read me").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read(FileReadParams {
                path: path.to_string_lossy().to_string(),
                offset: None,
                limit: None,
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "read me");
    }

    #[tokio::test]
    async fn test_file_read_with_offset_and_limit() {
        let dir = setup();
        let path = dir.path().join("read-chunked.txt");
        fs::write(&path, "line 1\nline 2\nline 3\nline 4\nline 5").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read(FileReadParams {
                path: path.to_string_lossy().to_string(),
                offset: Some(1),
                limit: Some(2),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "line 2\nline 3");
    }

    #[tokio::test]
    async fn test_file_read_with_offset_and_limit_falls_back_without_chunk_delegate() {
        let read_file: ReadFileFn =
            Arc::new(|_path: PathBuf| Box::pin(async { Ok("l1\nl2\nl3\nl4".to_string()) }));
        let write_file: WriteFileFn =
            Arc::new(|_path: PathBuf, _content: String| Box::pin(async { Ok(()) }));
        let tools = EditTools::with_file_io(read_file, None, write_file);

        let result = tools
            .file_read(FileReadParams {
                path: "ignored.txt".to_string(),
                offset: Some(1),
                limit: Some(2),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "l2\nl3");
    }

    #[tokio::test]
    async fn test_file_read_with_zero_limit_errors() {
        let dir = setup();
        let path = dir.path().join("read-invalid-limit.txt");
        fs::write(&path, "line 1\nline 2").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read(FileReadParams {
                path: path.to_string_lossy().to_string(),
                offset: Some(0),
                limit: Some(0),
            })
            .await;

        assert!(result.is_error.unwrap_or(false));
        assert!(extract_text(&result).contains("limit must be greater than 0"));
    }

    #[tokio::test]
    async fn test_file_read_large_requires_chunking() {
        let dir = setup();
        let path = dir.path().join("large-read.txt");
        let large = "a".repeat(MAX_READ_FILE_BYTES + 1);
        fs::write(&path, &large).unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read(FileReadParams {
                path: path.to_string_lossy().to_string(),
                offset: None,
                limit: None,
            })
            .await;

        assert!(result.is_error.unwrap_or(false));
        assert!(extract_text(&result).contains("Retry with offset and limit"));
    }

    #[tokio::test]
    async fn test_file_read_rejects_oversized_file() {
        let dir = setup();
        let path = dir.path().join("large-read.txt");
        let large = "a".repeat(MAX_READ_FILE_BYTES + 1);
        fs::write(&path, &large).unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read(FileReadParams {
                path: path.to_string_lossy().to_string(),
                offset: None,
                limit: None,
            })
            .await;

        assert!(result.is_error.unwrap_or(false));
        assert!(extract_text(&result).contains("exceeds max file size"));
    }

    #[tokio::test]
    async fn test_file_read_chunking_allows_large_file() {
        let dir = setup();
        let path = dir.path().join("large-read-chunked.txt");
        let lines = vec!["line".repeat(3000); 400];
        fs::write(&path, lines.join("\n")).unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read(FileReadParams {
                path: path.to_string_lossy().to_string(),
                offset: Some(0),
                limit: Some(3),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result).lines().count(), 3);
    }

    #[tokio::test]
    async fn test_file_read_with_limit_allows_large_file() {
        let dir = setup();
        let path = dir.path().join("large-chunk-read.txt");
        let mut content = String::new();
        for _ in 0..220_000 {
            content.push_str("line\n");
        }
        fs::write(&path, content).unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read(FileReadParams {
                path: path.to_string_lossy().to_string(),
                offset: Some(0),
                limit: Some(3),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result).lines().count(), 3);
    }

    #[tokio::test]
    async fn test_file_write_overwrite() {
        let dir = setup();
        let path = dir.path().join("existing.txt");
        fs::write(&path, "old content").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_write(FileWriteParams {
                path: path.to_string_lossy().to_string(),
                content: "new content".to_string(),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
    }

    #[tokio::test]
    async fn test_file_write_creates_dirs() {
        let dir = setup();
        let path = dir.path().join("a/b/c/file.txt");
        let tools = EditTools::default();

        let result = tools
            .file_write(FileWriteParams {
                path: path.to_string_lossy().to_string(),
                content: "nested".to_string(),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_file_edit_single_match() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "fn foo() {\n    println!(\"hello\");\n}").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_edit(FileEditParams {
                path: path.to_string_lossy().to_string(),
                before: "println!(\"hello\");".to_string(),
                after: "println!(\"world\");".to_string(),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("println!(\"world\");"));
        assert!(!content.contains("println!(\"hello\");"));
    }

    #[tokio::test]
    async fn test_file_edit_no_match() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "some content").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_edit(FileEditParams {
                path: path.to_string_lossy().to_string(),
                before: "nonexistent".to_string(),
                after: "replacement".to_string(),
            })
            .await;

        assert!(result.is_error.unwrap_or(false));
        let text = extract_text(&result);
        assert!(text.contains("No match found"));
        assert!(text.contains("File preview:"));
        assert!(text.contains("some content"));
    }

    #[tokio::test]
    async fn test_file_edit_multiple_matches() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "foo\nbar\nfoo\nbaz").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_edit(FileEditParams {
                path: path.to_string_lossy().to_string(),
                before: "foo".to_string(),
                after: "qux".to_string(),
            })
            .await;

        assert!(result.is_error.unwrap_or(false));
        assert_eq!(fs::read_to_string(&path).unwrap(), "foo\nbar\nfoo\nbaz");
    }

    #[tokio::test]
    async fn test_file_edit_delete() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        fs::write(&path, "keep\ndelete me\nkeep").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_edit(FileEditParams {
                path: path.to_string_lossy().to_string(),
                before: "\ndelete me".to_string(),
                after: "".to_string(),
            })
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep\nkeep");
    }

    #[tokio::test]
    async fn test_file_write_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        let tools = EditTools::default();

        let result = tools
            .file_write_with_cwd(
                FileWriteParams {
                    path: "relative.txt".to_string(),
                    content: "relative write".to_string(),
                },
                Some(dir.path()),
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            fs::read_to_string(dir.path().join("relative.txt")).unwrap(),
            "relative write"
        );
    }

    #[tokio::test]
    async fn test_file_edit_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        fs::write(dir.path().join("relative-edit.txt"), "before").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_edit_with_cwd(
                FileEditParams {
                    path: "relative-edit.txt".to_string(),
                    before: "before".to_string(),
                    after: "after".to_string(),
                },
                Some(dir.path()),
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            fs::read_to_string(dir.path().join("relative-edit.txt")).unwrap(),
            "after"
        );
    }

    #[tokio::test]
    async fn test_file_read_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        fs::write(dir.path().join("relative-read.txt"), "relative read").unwrap();
        let tools = EditTools::default();

        let result = tools
            .file_read_with_cwd(
                FileReadParams {
                    path: "relative-read.txt".to_string(),
                    offset: None,
                    limit: None,
                },
                Some(dir.path()),
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "relative read");
    }
}
