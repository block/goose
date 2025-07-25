//! Developer tool handlers implementation
//!
//! This module contains the actual implementation of developer tools handlers

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use base64::Engine;
use glob::glob;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use mcp_core::{ToolError, ToolResult};
use rmcp::model::Content;
use tokio::process::Command;
use xcap::{Monitor, Window};

use super::developer_tools::*;
use super::Agent;

/// Shell command configuration helpers
mod shell_config {
    use std::env;
    use std::path::PathBuf;

    pub fn get_shell_config() -> (String, Vec<String>) {
        if env::consts::OS == "windows" {
            // On Windows, use cmd.exe for better compatibility
            ("cmd".to_string(), vec!["/C".to_string()])
        } else {
            // On Unix-like systems, use the user's shell
            let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            (shell, vec!["-c".to_string()])
        }
    }

    pub fn expand_path(path: &str) -> PathBuf {
        PathBuf::from(shellexpand::tilde(path).to_string())
    }

    pub fn is_absolute_path(path: &str) -> bool {
        PathBuf::from(path).is_absolute()
    }

    #[allow(dead_code)]
    pub fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }
}

use shell_config::*;

impl Agent {
    /// Initialize developer tools state
    #[allow(dead_code)]
    pub(crate) fn init_developer_tools_state(&mut self) {
        // This would be called during agent initialization
        // For now, we'll handle state inline
    }

    /// Handle developer tool calls
    pub async fn handle_developer_tool_call(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        match tool_name {
            DEVELOPER_SHELL_TOOL_NAME => self.handle_shell_command(arguments).await,
            DEVELOPER_GLOB_TOOL_NAME => self.handle_glob_search(arguments).await,
            DEVELOPER_GREP_TOOL_NAME => self.handle_grep_search(arguments).await,
            DEVELOPER_TEXT_EDITOR_TOOL_NAME => self.handle_text_editor(arguments).await,
            DEVELOPER_LIST_WINDOWS_TOOL_NAME => self.handle_list_windows().await,
            DEVELOPER_SCREEN_CAPTURE_TOOL_NAME => self.handle_screen_capture(arguments).await,
            DEVELOPER_IMAGE_PROCESSOR_TOOL_NAME => self.handle_image_processor(arguments).await,
            _ => Err(ToolError::ExecutionError(format!(
                "Unknown developer tool: {}",
                tool_name
            ))),
        }
    }

    /// Execute a shell command
    async fn handle_shell_command(&self, arguments: serde_json::Value) -> ToolResult<Vec<Content>> {
        let command = arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'command' parameter".to_string()))?;

        let (shell, shell_args) = get_shell_config();

        let mut cmd = Command::new(&shell);
        for arg in shell_args {
            cmd.arg(arg);
        }
        cmd.arg(command);
        cmd.current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = if output.status.success() {
            format!("Command succeeded.\n\nOutput:\n{}", stdout)
        } else {
            format!(
                "Command failed with exit code: {}\n\nStdout:\n{}\n\nStderr:\n{}",
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
            )
        };

        Ok(vec![Content::text(result)])
    }

    /// Search for files using glob patterns
    async fn handle_glob_search(&self, arguments: serde_json::Value) -> ToolResult<Vec<Content>> {
        let pattern = arguments
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'pattern' parameter".to_string()))?;

        let search_path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let base_path = expand_path(search_path);
        let full_pattern = if is_absolute_path(pattern) {
            pattern.to_string()
        } else {
            base_path.join(pattern).to_string_lossy().to_string()
        };

        // Load .gooseignore patterns
        let ignore_patterns = load_gooseignore_patterns(&base_path)?;

        let mut matches = Vec::new();
        for entry in glob(&full_pattern)
            .map_err(|e| ToolError::ExecutionError(format!("Invalid glob pattern: {}", e)))?
        {
            match entry {
                Ok(path) => {
                    if !should_ignore_path(&path, &ignore_patterns) {
                        matches.push(path.to_string_lossy().to_string());
                    }
                }
                Err(e) => {
                    // Log but don't fail on individual path errors
                    eprintln!("Error accessing path: {}", e);
                }
            }
        }

        // Sort by modification time (newest first)
        matches.sort_by(|a, b| {
            let a_meta = std::fs::metadata(a).ok();
            let b_meta = std::fs::metadata(b).ok();

            match (a_meta, b_meta) {
                (Some(a), Some(b)) => {
                    let a_time = a.modified().ok();
                    let b_time = b.modified().ok();
                    b_time.cmp(&a_time)
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        let result = if matches.is_empty() {
            "No files found matching the pattern.".to_string()
        } else {
            format!("Found {} files:\n{}", matches.len(), matches.join("\n"))
        };

        Ok(vec![Content::text(result)])
    }

    /// Execute file content search
    async fn handle_grep_search(&self, arguments: serde_json::Value) -> ToolResult<Vec<Content>> {
        let command = arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'command' parameter".to_string()))?;

        // Execute the search command
        let (shell, shell_args) = get_shell_config();

        let mut cmd = Command::new(&shell);
        for arg in shell_args {
            cmd.arg(arg);
        }
        cmd.arg(command);
        cmd.current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to execute search command: {}", e))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Filter results based on .gooseignore
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let ignore_patterns = load_gooseignore_patterns(&cwd)?;

        let filtered_output = stdout
            .lines()
            .filter(|line| {
                // Try to extract file path from the line
                // This is a simple heuristic that works for most grep/rg output
                if let Some(path_end) = line.find(':') {
                    let path_str = &line[..path_end];
                    let path = PathBuf::from(path_str);
                    !should_ignore_path(&path, &ignore_patterns)
                } else {
                    true // Keep lines that don't look like file paths
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let result = if output.status.success() {
            if filtered_output.is_empty() {
                "No matches found.".to_string()
            } else {
                filtered_output
            }
        } else {
            format!("Search command failed:\nStderr:\n{}", stderr)
        };

        Ok(vec![Content::text(result)])
    }

    /// Handle text editor operations
    async fn handle_text_editor(&self, arguments: serde_json::Value) -> ToolResult<Vec<Content>> {
        let command = arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'command' parameter".to_string()))?;

        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'path' parameter".to_string()))?;

        let path = expand_path(path);

        match command {
            "view" => self.handle_view_file(&path, &arguments).await,
            "write" => self.handle_write_file(&path, &arguments).await,
            "str_replace" => self.handle_str_replace(&path, &arguments).await,
            "insert" => self.handle_insert_text(&path, &arguments).await,
            "undo_edit" => self.handle_undo_edit(&path).await,
            _ => Err(ToolError::ExecutionError(format!(
                "Unknown text editor command: {}",
                command
            ))),
        }
    }

    /// View file contents
    async fn handle_view_file(
        &self,
        path: &Path,
        arguments: &serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        if path.is_dir() {
            // List directory contents
            let mut entries = Vec::new();
            let read_dir = tokio::fs::read_dir(path).await.map_err(|e| {
                ToolError::ExecutionError(format!("Failed to read directory: {}", e))
            })?;

            let mut read_dir = read_dir;
            while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
                ToolError::ExecutionError(format!("Failed to read directory entry: {}", e))
            })? {
                let file_type = if entry
                    .file_type()
                    .await
                    .map_err(|e| {
                        ToolError::ExecutionError(format!("Failed to get file type: {}", e))
                    })?
                    .is_dir()
                {
                    "[DIR]"
                } else {
                    "[FILE]"
                };
                entries.push(format!(
                    "{} {}",
                    file_type,
                    entry.file_name().to_string_lossy()
                ));
            }

            entries.sort();
            Ok(vec![Content::text(entries.join("\n"))])
        } else {
            // Read file contents
            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

            // Handle view_range if specified
            if let Some(range) = arguments.get("view_range") {
                if let Some(range_array) = range.as_array() {
                    if range_array.len() == 2 {
                        let start = range_array[0].as_i64().unwrap_or(1) as usize;
                        let end = range_array[1].as_i64().unwrap_or(-1) as isize;

                        let lines: Vec<&str> = content.lines().collect();
                        let total_lines = lines.len();

                        let start_idx = if start > 0 { start - 1 } else { 0 };
                        let end_idx = if end < 0 {
                            total_lines
                        } else {
                            (end as usize).min(total_lines)
                        };

                        let selected_lines = lines[start_idx..end_idx].join("\n");
                        return Ok(vec![Content::text(selected_lines)]);
                    }
                }
            }

            Ok(vec![Content::text(content)])
        }
    }

    /// Write content to file
    async fn handle_write_file(
        &self,
        path: &Path,
        arguments: &serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let content = arguments
            .get("file_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::ExecutionError("Missing 'file_text' parameter".to_string())
            })?;

        // Save current content to history (for undo)
        self.save_file_history(path).await?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::ExecutionError(format!("Failed to create directories: {}", e))
            })?;
        }

        // Write the file
        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        Ok(vec![Content::text(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path.display()
        ))])
    }

    /// Replace string in file
    async fn handle_str_replace(
        &self,
        path: &Path,
        arguments: &serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let old_str = arguments
            .get("old_str")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'old_str' parameter".to_string()))?;

        let new_str = arguments
            .get("new_str")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'new_str' parameter".to_string()))?;

        // Read current content
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

        // Check if old_str exists in the file
        if !content.contains(old_str) {
            return Err(ToolError::ExecutionError(
                "The specified 'old_str' was not found in the file".to_string(),
            ));
        }

        // Check if old_str appears multiple times
        let occurrences = content.matches(old_str).count();
        if occurrences > 1 {
            return Err(ToolError::ExecutionError(format!(
                "The specified 'old_str' appears {} times in the file. Please make it more specific.",
                occurrences
            )));
        }

        // Save current content to history
        self.save_file_history(path).await?;

        // Perform replacement
        let new_content = content.replace(old_str, new_str);

        // Write the file
        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        Ok(vec![Content::text(format!(
            "Successfully replaced string in {}",
            path.display()
        ))])
    }

    /// Insert text at specific line
    async fn handle_insert_text(
        &self,
        path: &Path,
        arguments: &serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let insert_line = arguments
            .get("insert_line")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                ToolError::ExecutionError("Missing 'insert_line' parameter".to_string())
            })? as usize;

        let new_str = arguments
            .get("new_str")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'new_str' parameter".to_string()))?;

        // Read current content
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

        // Save current content to history
        self.save_file_history(path).await?;

        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        // Insert at the specified line
        if insert_line == 0 {
            lines.insert(0, new_str.to_string());
        } else if insert_line <= lines.len() {
            lines.insert(insert_line, new_str.to_string());
        } else {
            return Err(ToolError::ExecutionError(format!(
                "Insert line {} is beyond the file length ({})",
                insert_line,
                lines.len()
            )));
        }

        let new_content = lines.join("\n");

        // Write the file
        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        Ok(vec![Content::text(format!(
            "Successfully inserted text at line {} in {}",
            insert_line,
            path.display()
        ))])
    }

    /// Undo last edit
    async fn handle_undo_edit(&self, path: &Path) -> ToolResult<Vec<Content>> {
        // Get file history from developer tools state
        let content_to_restore = {
            let file_history = self.developer_tools_state.lock().await;
            let mut history = file_history.file_history.lock().unwrap();

            if let Some(versions) = history.get_mut(path) {
                versions.pop()
            } else {
                None
            }
        }; // All locks are dropped here

        match content_to_restore {
            Some(content) => {
                tokio::fs::write(path, &content).await.map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to restore file: {}", e))
                })?;

                Ok(vec![Content::text(format!(
                    "Successfully restored previous version of {}",
                    path.display()
                ))])
            }
            None => Err(ToolError::ExecutionError(
                "No previous version available for undo".to_string(),
            )),
        }
    }

    /// Save file content to history for undo functionality
    async fn save_file_history(&self, path: &Path) -> Result<(), ToolError> {
        if path.exists() {
            let content = tokio::fs::read_to_string(path).await.map_err(|e| {
                ToolError::ExecutionError(format!("Failed to read file for history: {}", e))
            })?;

            let file_history = self.developer_tools_state.lock().await;
            let mut history = file_history.file_history.lock().unwrap();

            let versions = history.entry(path.to_path_buf()).or_insert_with(Vec::new);
            versions.push(content);

            // Keep only last 10 versions
            if versions.len() > 10 {
                versions.remove(0);
            }
        }
        Ok(())
    }

    /// List available windows
    async fn handle_list_windows(&self) -> ToolResult<Vec<Content>> {
        let windows = Window::all()
            .map_err(|e| ToolError::ExecutionError(format!("Failed to list windows: {}", e)))?;

        let window_list: Vec<String> = windows
            .into_iter()
            .map(|w| w.title().to_string())
            .filter(|title| !title.is_empty())
            .collect();

        let result = if window_list.is_empty() {
            "No windows found".to_string()
        } else {
            format!("Available windows:\n{}", window_list.join("\n"))
        };

        Ok(vec![Content::text(result)])
    }

    /// Capture screenshot
    async fn handle_screen_capture(
        &self,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let display = arguments.get("display").and_then(|v| v.as_i64());
        let window_title = arguments.get("window_title").and_then(|v| v.as_str());

        let image = if let Some(title) = window_title {
            // Capture specific window
            let windows = Window::all()
                .map_err(|e| ToolError::ExecutionError(format!("Failed to list windows: {}", e)))?;

            let window = windows
                .into_iter()
                .find(|w| w.title() == title)
                .ok_or_else(|| {
                    ToolError::ExecutionError(format!("Window '{}' not found", title))
                })?;

            window.capture_image().map_err(|e| {
                ToolError::ExecutionError(format!("Failed to capture window: {}", e))
            })?
        } else {
            // Capture display
            let display_num = display.unwrap_or(0) as usize;
            let monitors = Monitor::all().map_err(|e| {
                ToolError::ExecutionError(format!("Failed to list monitors: {}", e))
            })?;

            let monitor = monitors.get(display_num).ok_or_else(|| {
                ToolError::ExecutionError(format!("Display {} not found", display_num))
            })?;

            monitor.capture_image().map_err(|e| {
                ToolError::ExecutionError(format!("Failed to capture display: {}", e))
            })?
        };

        // Convert to PNG and encode as base64
        let mut buffer = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buffer), xcap::image::ImageFormat::Png)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to encode image: {}", e)))?;

        let base64_image = base64::engine::general_purpose::STANDARD.encode(&buffer);
        let data_uri = format!("data:image/png;base64,{}", base64_image);

        Ok(vec![Content::text(format!(
            "Screenshot captured. Image data URI:\n{}",
            data_uri
        ))])
    }

    /// Process image file
    async fn handle_image_processor(
        &self,
        arguments: serde_json::Value,
    ) -> ToolResult<Vec<Content>> {
        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing 'path' parameter".to_string()))?;

        let path = expand_path(path);

        // Load the image
        let img = image::open(&path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to load image: {}", e)))?;

        // Resize if needed (max width 1920px)
        const MAX_WIDTH: u32 = 1920;
        let img = if img.width() > MAX_WIDTH {
            let scale = MAX_WIDTH as f32 / img.width() as f32;
            let new_height = (img.height() as f32 * scale) as u32;
            img.resize(MAX_WIDTH, new_height, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        // Convert to PNG and encode as base64
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to encode image: {}", e)))?;

        let base64_image = base64::engine::general_purpose::STANDARD.encode(&buffer);
        let data_uri = format!("data:image/png;base64,{}", base64_image);

        Ok(vec![Content::text(format!(
            "Image processed. Dimensions: {}x{}. Data URI:\n{}",
            img.width(),
            img.height(),
            data_uri
        ))])
    }
}

/// Load .gooseignore patterns
fn load_gooseignore_patterns(base_path: &Path) -> Result<Gitignore, ToolError> {
    let ignore_path = base_path.join(".gooseignore");
    let mut builder = GitignoreBuilder::new(base_path);

    if ignore_path.exists() {
        builder.add(&ignore_path);
    }

    builder
        .build()
        .map_err(|e| ToolError::ExecutionError(format!("Failed to parse .gooseignore: {}", e)))
}

/// Check if a path should be ignored
fn should_ignore_path(path: &Path, ignore: &Gitignore) -> bool {
    ignore.matched(path, path.is_dir()).is_ignore()
}
