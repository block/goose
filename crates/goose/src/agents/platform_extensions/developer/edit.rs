use fs_err as fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::debug;

const NO_MATCH_PREVIEW_LINES: usize = 20;

#[async_trait]
pub trait Fs: Send + Sync {
    async fn read_text_file(
        &self,
        path: &Path,
        line: Option<u32>,
        limit: Option<u32>,
    ) -> io::Result<String>;
    async fn write_text_file(&self, path: &Path, content: &str) -> io::Result<()>;
}

pub struct LocalFs;

#[async_trait]
impl Fs for LocalFs {
    async fn read_text_file(
        &self,
        path: &Path,
        line: Option<u32>,
        limit: Option<u32>,
    ) -> io::Result<String> {
        debug!(path = %path.display(), ?line, ?limit, "read_text_file");
        let content = fs::read_to_string(path)?;
        Ok(apply_line_limit(&content, line, limit))
    }

    async fn write_text_file(&self, path: &Path, content: &str) -> io::Result<()> {
        debug!(path = %path.display(), content_len = content.len(), "write_text_file");
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, content)
    }
}

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

pub struct EditTools {
    fs: Arc<dyn Fs>,
}

impl EditTools {
    pub fn new(fs: Arc<dyn Fs>) -> Self {
        Self { fs }
    }

    pub async fn file_read(
        &self,
        params: FileReadParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);

        match self
            .fs
            .read_text_file(&path, params.line, params.limit)
            .await
        {
            Ok(content) => CallToolResult::success(vec![Content::text(content).with_priority(0.0)]),
            Err(error) => CallToolResult::error(vec![Content::text(format!(
                "Failed to read {}: {}",
                params.path, error
            ))
            .with_priority(0.0)]),
        }
    }

    pub async fn file_write(
        &self,
        params: FileWriteParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);
        let is_new = !path.exists();

        match self.fs.write_text_file(&path, &params.content).await {
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

    pub async fn file_edit(
        &self,
        params: FileEditParams,
        working_dir: Option<&Path>,
    ) -> CallToolResult {
        let path = resolve_path(&params.path, working_dir);

        let content = match self.fs.read_text_file(&path, None, None).await {
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

                match self.fs.write_text_file(&path, &new_content).await {
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

pub fn apply_line_limit(content: &str, line: Option<u32>, limit: Option<u32>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = line
        .map(|l| (l as usize).saturating_sub(1))
        .unwrap_or(0)
        .min(lines.len());
    let end = limit
        .map(|l| start + l as usize)
        .unwrap_or(lines.len())
        .min(lines.len());
    lines[start..end].join("\n")
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
    use std::sync::Mutex;
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

    #[derive(Debug, Default)]
    #[allow(clippy::type_complexity)]
    struct RecordingFs {
        reads: Mutex<Vec<(PathBuf, Option<u32>, Option<u32>)>>,
        writes: Mutex<Vec<(PathBuf, String)>>,
    }

    #[async_trait]
    impl Fs for RecordingFs {
        async fn read_text_file(
            &self,
            path: &Path,
            line: Option<u32>,
            limit: Option<u32>,
        ) -> io::Result<String> {
            self.reads
                .lock()
                .unwrap()
                .push((path.to_path_buf(), line, limit));
            Ok("recorded-content".to_string())
        }

        async fn write_text_file(&self, path: &Path, content: &str) -> io::Result<()> {
            self.writes
                .lock()
                .unwrap()
                .push((path.to_path_buf(), content.to_string()));
            Ok(())
        }
    }

    #[test_case(None, None, "line1\nline2\nline3" ; "full content")]
    #[test_case(Some(2), None, "line2\nline3" ; "from line 2")]
    #[test_case(None, Some(2), "line1\nline2" ; "limit 2")]
    #[test_case(Some(2), Some(1), "line2" ; "line 2 limit 1")]
    #[test_case(Some(99), None, "" ; "beyond eof")]
    fn test_apply_line_limit(line: Option<u32>, limit: Option<u32>, expected: &str) {
        assert_eq!(
            apply_line_limit("line1\nline2\nline3", line, limit),
            expected
        );
    }

    #[tokio::test]
    async fn test_file_read() {
        let dir = setup();
        let path = dir.path().join("read.txt");
        std::fs::write(&path, "line1\nline2\nline3").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_read(
                FileReadParams {
                    path: path.to_string_lossy().to_string(),
                    line: None,
                    limit: None,
                },
                None,
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "line1\nline2\nline3");
    }

    #[tokio::test]
    async fn test_file_read_partial() {
        let dir = setup();
        let path = dir.path().join("read.txt");
        std::fs::write(&path, "line1\nline2\nline3").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_read(
                FileReadParams {
                    path: path.to_string_lossy().to_string(),
                    line: Some(2),
                    limit: Some(1),
                },
                None,
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(extract_text(&result), "line2");
    }

    #[tokio::test]
    async fn test_file_write_new() {
        let dir = setup();
        let path = dir.path().join("new_file.txt");
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_write(
                FileWriteParams {
                    path: path.to_string_lossy().to_string(),
                    content: "Hello, world!\nLine 2".to_string(),
                },
                None,
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert!(path.exists());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "Hello, world!\nLine 2"
        );
    }

    #[tokio::test]
    async fn test_file_write_overwrite() {
        let dir = setup();
        let path = dir.path().join("existing.txt");
        std::fs::write(&path, "old content").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_write(
                FileWriteParams {
                    path: path.to_string_lossy().to_string(),
                    content: "new content".to_string(),
                },
                None,
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
    }

    #[tokio::test]
    async fn test_file_write_creates_dirs() {
        let dir = setup();
        let path = dir.path().join("a/b/c/file.txt");
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_write(
                FileWriteParams {
                    path: path.to_string_lossy().to_string(),
                    content: "nested".to_string(),
                },
                None,
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_file_edit_single_match() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        std::fs::write(&path, "fn foo() {\n    println!(\"hello\");\n}").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_edit(
                FileEditParams {
                    path: path.to_string_lossy().to_string(),
                    before: "println!(\"hello\");".to_string(),
                    after: "println!(\"world\");".to_string(),
                },
                None,
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("println!(\"world\");"));
        assert!(!content.contains("println!(\"hello\");"));
    }

    #[tokio::test]
    async fn test_file_edit_no_match() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        std::fs::write(&path, "some content").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_edit(
                FileEditParams {
                    path: path.to_string_lossy().to_string(),
                    before: "nonexistent".to_string(),
                    after: "replacement".to_string(),
                },
                None,
            )
            .await;

        assert!(result.is_error.unwrap_or(false));
        let text = extract_text(&result);
        assert!(text.contains("No match found"));
    }

    #[tokio::test]
    async fn test_file_edit_multiple_matches() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        std::fs::write(&path, "foo\nbar\nfoo\nbaz").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_edit(
                FileEditParams {
                    path: path.to_string_lossy().to_string(),
                    before: "foo".to_string(),
                    after: "qux".to_string(),
                },
                None,
            )
            .await;

        assert!(result.is_error.unwrap_or(false));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "foo\nbar\nfoo\nbaz"
        );
    }

    #[tokio::test]
    async fn test_file_edit_delete() {
        let dir = setup();
        let path = dir.path().join("edit.txt");
        std::fs::write(&path, "keep\ndelete me\nkeep").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_edit(
                FileEditParams {
                    path: path.to_string_lossy().to_string(),
                    before: "\ndelete me".to_string(),
                    after: "".to_string(),
                },
                None,
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep\nkeep");
    }

    #[tokio::test]
    async fn test_file_write_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_write(
                FileWriteParams {
                    path: "relative.txt".to_string(),
                    content: "relative write".to_string(),
                },
                Some(dir.path()),
            )
            .await;

        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("relative.txt")).unwrap(),
            "relative write"
        );
    }

    #[tokio::test]
    async fn test_file_edit_resolves_relative_paths_from_working_dir() {
        let dir = setup();
        std::fs::write(dir.path().join("relative-edit.txt"), "before").unwrap();
        let tools = EditTools::new(Arc::new(LocalFs));

        let result = tools
            .file_edit(
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
            std::fs::read_to_string(dir.path().join("relative-edit.txt")).unwrap(),
            "after"
        );
    }

    #[tokio::test]
    async fn test_fs_read_delegation() {
        let service = Arc::new(RecordingFs::default());
        let tools = EditTools::new(service.clone());

        tools
            .file_read(
                FileReadParams {
                    path: "/test/file.txt".to_string(),
                    line: Some(5),
                    limit: Some(10),
                },
                None,
            )
            .await;

        let reads = service.reads.lock().unwrap();
        assert_eq!(reads.len(), 1);
        assert_eq!(reads[0].0, PathBuf::from("/test/file.txt"));
        assert_eq!(reads[0].1, Some(5));
        assert_eq!(reads[0].2, Some(10));
    }

    #[tokio::test]
    async fn test_fs_write_delegation() {
        let service = Arc::new(RecordingFs::default());
        let tools = EditTools::new(service.clone());

        tools
            .file_write(
                FileWriteParams {
                    path: "/test/out.txt".to_string(),
                    content: "hello".to_string(),
                },
                None,
            )
            .await;

        let writes = service.writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, PathBuf::from("/test/out.txt"));
        assert_eq!(writes[0].1, "hello");
    }
}
