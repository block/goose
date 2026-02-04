//! Text Editor Platform Extension
//!
//! Provides file viewing and editing capabilities as a platform extension.

use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "text_editor";

const MAX_VIEW_SIZE: u64 = 400 * 1024; // 400KB limit
const MAX_LINES_THRESHOLD: usize = 1000;
const SIMILARITY_THRESHOLD: f64 = 0.7;

#[derive(Debug, Deserialize, JsonSchema)]
struct TextEditorParams {
    /// The command: "view", "write", "str_replace", "insert", or "undo_edit"
    command: String,
    /// Absolute path to the file
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
}

pub struct TextEditorClient {
    info: InitializeResult,
    #[allow(dead_code)]
    context: PlatformExtensionContext,
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
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

        Ok(Self {
            info,
            context,
            file_history: Arc::new(Mutex::new(HashMap::new())),
        })
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
                - str_replace: Replace exact text (old_str->new_str)
                - insert: Insert text after a specific line number
                - undo_edit: Revert the last edit to a file

                For str_replace, old_str must match EXACTLY (including whitespace).
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

    async fn handle_write(&self, path: &Path, content: &str) -> Result<String, String> {
        if path.exists() {
            if let Ok(old_content) = tokio::fs::read_to_string(path).await {
                let mut history = self.file_history.lock().await;
                history
                    .entry(path.to_path_buf())
                    .or_default()
                    .push(old_content);
            }
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
                    "No exact match found for old_str. Did you mean:
{}",
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

        let new_content = content.replacen(old_str, new_str, 1);

        {
            let mut history = self.file_history.lock().await;
            history.entry(path.to_path_buf()).or_default().push(content);
        }

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

        {
            let mut history = self.file_history.lock().await;
            history
                .entry(path.to_path_buf())
                .or_default()
                .push(content.clone());
        }

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
            let mut history = self.file_history.lock().await;
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
        let cwd =
            std::env::current_dir().map_err(|e| format!("Cannot get current directory: {}", e))?;
        let expanded: String = shellexpand::tilde(path_str).into();
        let path = Path::new(&expanded);

        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(cwd.join(path))
        }
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
                let old_str = params.old_str.unwrap_or_default();
                let new_str = params.new_str.unwrap_or_default();
                self.handle_str_replace(&path, &old_str, &new_str).await
            }
            "insert" => {
                let line = params.insert_line.unwrap_or(0);
                let text = params.file_text.unwrap_or_default();
                self.handle_insert(&path, line, &text).await
            }
            "undo_edit" => self.handle_undo(&path).await,
            cmd => Err(format!(
                "Unknown command: '{}'. Use: view, write, str_replace, insert, undo_edit",
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
}
