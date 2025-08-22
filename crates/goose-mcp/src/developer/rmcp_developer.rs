use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{Content, Role, CallToolResult, ErrorData, ErrorCode, ServerInfo, ServerCapabilities},
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, io::Cursor, path::{Path, PathBuf}, fs::File, io::Read, collections::HashMap, sync::{Arc, Mutex}, process::Stdio};
use base64::Engine;
use xcap::{Monitor, Window};
use indoc::formatdoc;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tokio_stream::{wrappers::SplitStream, StreamExt as _};

use super::shell::{expand_path, is_absolute_path, normalize_line_endings, get_shell_config};
use super::lang::get_language_identifier;
use super::editor_models::{create_editor_model, EditorModel};

/// Parameters for the screen_capture tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScreenCaptureParams {
    /// The display number to capture (0 is main display)
    #[serde(default)]
    pub display: Option<u64>,
    
    /// Optional: the exact title of the window to capture. 
    /// Use the list_windows tool to find the available windows.
    pub window_title: Option<String>,
}

/// Parameters for the text_editor tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TextEditorParams {
    /// Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`.
    pub path: String,
    
    /// The operation to perform. Allowed options are: `view`, `write`, `str_replace`, `insert`, `undo_edit`.
    pub command: String,
    
    /// Optional array of two integers specifying the start and end line numbers to view. 
    /// Line numbers are 1-indexed, and -1 for the end line means read to the end of the file. 
    /// This parameter only applies when viewing files, not directories.
    pub view_range: Option<Vec<i64>>,
    
    /// The content to write to the file. Required for `write` command.
    pub file_text: Option<String>,
    
    /// The old string to replace. Required for `str_replace` command.
    pub old_str: Option<String>,
    
    /// The new string to replace with. Required for `str_replace` and `insert` commands.
    pub new_str: Option<String>,
    
    /// The line number after which to insert text (0 for beginning). Required for `insert` command.
    pub insert_line: Option<i64>,
}

/// Parameters for the shell tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShellParams {
    /// The command string to execute in the shell
    pub command: String,
}

/// Parameters for the image_processor tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ImageProcessorParams {
    /// Absolute path to the image file to process
    pub path: String,
}

/// Developer MCP Server using official RMCP SDK
#[derive(Debug)]
pub struct DeveloperServer {
    tool_router: ToolRouter<Self>,
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    ignore_patterns: Gitignore,
    editor_model: Option<EditorModel>,
}

impl Default for DeveloperServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DeveloperServer {
    fn get_info(&self) -> ServerInfo {
        // Get base instructions and working directory
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let os = std::env::consts::OS;

        let base_instructions = match os {
            "windows" => formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

                You can use the shell tool to run Windows commands (PowerShell or CMD).
                When using paths, you can use either backslashes or forward slashes.

                Use the shell tool as needed to locate files or interact with the project.

                Your windows/screen tools can be used for visual debugging. You should not use these tools unless
                prompted to, but you can mention they are available if they are relevant.

                operating system: {os}
                current directory: {cwd}

                "#,
                os=os,
                cwd=cwd.to_string_lossy(),
            },
            _ => formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

            You can use the shell tool to run any command that would work on the relevant operating system.
            Use the shell tool as needed to locate files or interact with the project.

            Your windows/screen tools can be used for visual debugging. You should not use these tools unless
            prompted to, but you can mention they are available if they are relevant.

            operating system: {os}
            current directory: {cwd}

                "#,
                os=os,
                cwd=cwd.to_string_lossy(),
            },
        };

        let hints_filenames: Vec<String> = std::env::var("CONTEXT_FILE_NAMES")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| vec!["AGENTS.md".to_string(), ".goosehints".to_string()]);

        let mut hints = String::new();
        for filename in hints_filenames {
            let hints_path = cwd.join(&filename);
            if hints_path.exists() && hints_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&hints_path) {
                    if !content.trim().is_empty() {
                        hints.push_str(&format!("\n{filename}:\n{content}\n"));
                    }
                }
            }
        }

        // Return base instructions directly when no hints are found
        let instructions = if hints.is_empty() {
            base_instructions
        } else {
            format!("{base_instructions}\n{hints}")
        };

        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(instructions),
            ..Default::default()
        }
    }
}

#[tool_router(router = tool_router)]
impl DeveloperServer {
    pub fn new() -> Self {
        // Build ignore patterns (simplified version for this tool)
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut builder = GitignoreBuilder::new(&cwd);
        
        // Check for local .gooseignore
        let local_ignore_path = cwd.join(".gooseignore");
        let mut has_ignore_file = false;
        
        if local_ignore_path.is_file() {
            let _ = builder.add(local_ignore_path);
            has_ignore_file = true;
        } else {
            // Fallback to .gitignore
            let gitignore_path = cwd.join(".gitignore");
            if gitignore_path.is_file() {
                let _ = builder.add(gitignore_path);
                has_ignore_file = true;
            }
        }
        
        // Add default patterns if no ignore files found
        if !has_ignore_file {
            let _ = builder.add_line(None, "**/.env");
            let _ = builder.add_line(None, "**/.env.*");
            let _ = builder.add_line(None, "**/secrets.*");
        }
        
        let ignore_patterns = builder.build().expect("Failed to build ignore patterns");
        
        // Initialize editor model for AI-powered code editing
        let editor_model = create_editor_model();
        
        Self {
            tool_router: Self::tool_router(),
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns,
            editor_model,
        }
    }

    /// List all available windows that can be used with screen_capture.
    /// Returns a list of window titles that can be used with the window_title parameter
    /// of the screen_capture tool.
    #[tool(
        name = "list_windows", 
        description = "List all available window titles that can be used with screen_capture. Returns a list of window titles that can be used with the window_title parameter of the screen_capture tool."
    )]
    pub async fn list_windows(&self) -> Result<CallToolResult, ErrorData> {
        let windows = Window::all().map_err(|_| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to list windows".to_string(),
                None,
            )
        })?;

        let window_titles: Vec<String> = windows
            .into_iter()
            .map(|w| w.title().to_string())
            .collect();

        let content_text = format!("Available windows:\n{}", window_titles.join("\n"));

        Ok(CallToolResult::success(vec![
            Content::text(content_text.clone())
                .with_audience(vec![Role::Assistant]),
            Content::text(content_text)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    /// Capture a screenshot of a specified display or window.
    /// You can capture either:
    /// 1. A full display (monitor) using the display parameter
    /// 2. A specific window by its title using the window_title parameter
    ///
    /// Only one of display or window_title should be specified.
    #[tool(
        name = "screen_capture",
        description = "Capture a screenshot of a specified display or window. You can capture either: 1. A full display (monitor) using the display parameter 2. A specific window by its title using the window_title parameter. Only one of display or window_title should be specified."
    )]
    pub async fn screen_capture(
        &self,
        params: Parameters<ScreenCaptureParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        let mut image = if let Some(window_title) = &params.window_title {
            // Try to find and capture the specified window
            let windows = Window::all().map_err(|_| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to list windows".to_string(),
                    None,
                )
            })?;

            let window = windows
                .into_iter()
                .find(|w| w.title() == window_title)
                .ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("No window found with title '{}'", window_title),
                        None,
                    )
                })?;

            window.capture_image().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to capture window '{}': {}", window_title, e),
                    None,
                )
            })?
        } else {
            // Default to display capture if no window title is specified
            let display = params.display.unwrap_or(0) as usize;

            let monitors = Monitor::all().map_err(|_| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to access monitors".to_string(),
                    None,
                )
            })?;
            
            let monitor = monitors.get(display).ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!(
                        "{} was not an available monitor, {} found.",
                        display,
                        monitors.len()
                    ),
                    None,
                )
            })?;

            monitor.capture_image().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to capture display {}: {}", display, e),
                    None,
                )
            })?
        };

        // Resize the image to a reasonable width while maintaining aspect ratio
        let max_width = 768;
        if image.width() > max_width {
            let scale = max_width as f32 / image.width() as f32;
            let new_height = (image.height() as f32 * scale) as u32;
            image = xcap::image::imageops::resize(
                &image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            );
        }

        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write image buffer {}", e),
                    None,
                )
            })?;

        // Convert to base64
        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        // Return two Content objects like the old implementation:
        // one text for Assistant, one image with priority 0.0
        Ok(CallToolResult::success(vec![
            Content::text("Screenshot captured")
                .with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png")
                .with_priority(0.0),
        ]))
    }

    /// Perform text editing operations on files.
    /// 
    /// The `command` parameter specifies the operation to perform. Allowed options are:
    /// - `view`: View the content of a file.
    /// - `write`: Create or overwrite a file with the given content
    /// - `str_replace`: Replace old_str with new_str in the file.
    /// - `insert`: Insert text at a specific line location in the file.
    /// - `undo_edit`: Undo the last edit made to a file.
    #[tool(
        name = "text_editor",
        description = "Perform text editing operations on files. Commands: view (show file content), write (create/overwrite file), str_replace (AI-enhanced replace text when configured, fallback to literal replacement), insert (insert at line), undo_edit (undo last change)."
    )]
    pub async fn text_editor(
        &self,
        params: Parameters<TextEditorParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = self.resolve_path(&params.path)?;

        match params.command.as_str() {
            "view" => {
                let view_range = params.view_range.as_ref().and_then(|vr| {
                    if vr.len() == 2 {
                        Some((vr[0] as usize, vr[1]))
                    } else {
                        None
                    }
                });
                self.text_editor_view(&path, view_range).await
            }
            "write" => {
                let file_text = params.file_text.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'file_text' parameter for write command".to_string(),
                        None,
                    )
                })?;
                self.text_editor_write(&path, &file_text).await
            }
            "str_replace" => {
                let old_str = params.old_str.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'old_str' parameter for str_replace command".to_string(),
                        None,
                    )
                })?;
                let new_str = params.new_str.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'new_str' parameter for str_replace command".to_string(),
                        None,
                    )
                })?;
                self.text_editor_replace(&path, &old_str, &new_str).await
            }
            "insert" => {
                let insert_line = params.insert_line.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'insert_line' parameter for insert command".to_string(),
                        None,
                    )
                })? as usize;
                let new_str = params.new_str.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'new_str' parameter for insert command".to_string(),
                        None,
                    )
                })?;
                self.text_editor_insert(&path, insert_line, &new_str).await
            }
            "undo_edit" => self.text_editor_undo(&path).await,
            _ => Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Unknown command '{}'", params.command),
                None,
            )),
        }
    }

    /// Execute a command in the shell.
    /// 
    /// This will return the output and error concatenated into a single string, as
    /// you would see from running on the command line. There will also be an indication
    /// of if the command succeeded or failed.
    /// 
    /// Avoid commands that produce a large amount of output, and consider piping those outputs to files.
    /// If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
    /// this tool does not run indefinitely.
    #[tool(
        name = "shell",
        description = "Execute a command in the shell. Returns output and error concatenated. Avoid commands with large output, use background commands for long-running processes."
    )]
    pub async fn shell(
        &self,
        params: Parameters<ShellParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let command = &params.command;

        // Check if command might access ignored files and return early if it does
        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
        for arg in &cmd_parts[1..] {
            // Skip command flags
            if arg.starts_with('-') {
                continue;
            }
            // Skip invalid paths
            let path = Path::new(arg);
            if !path.exists() {
                continue;
            }

            if self.is_ignored(path) {
                return Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!(
                        "The command attempts to access '{}' which is restricted by .gooseignore",
                        arg
                    ),
                    None,
                ));
            }
        }

        // Get platform-specific shell configuration
        let shell_config = get_shell_config();

        // Execute the command using platform-specific shell
        let mut child = Command::new(&shell_config.executable)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .env("GOOSE_TERMINAL", "1")
            .args(&shell_config.args)
            .arg(command)
            .spawn()
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());

        let output_task = tokio::spawn(async move {
            let mut combined_output = String::new();

            // We have the individual two streams above, now merge them into one unified stream of
            // an enum. ref https://blog.yoshuawuyts.com/futures-concurrency-3
            let stdout = SplitStream::new(stdout.split(b'\n')).map(|v| ("stdout", v));
            let stderr = SplitStream::new(stderr.split(b'\n')).map(|v| ("stderr", v));
            let mut merged = stdout.merge(stderr);

            while let Some((_, line)) = merged.next().await {
                let mut line = line?;
                // Re-add this as clients expect it
                line.push(b'\n');
                // Here we always convert to UTF-8 so agents don't have to deal with corrupted output
                let line = String::from_utf8_lossy(&line);

                combined_output.push_str(&line);
            }
            Ok::<_, std::io::Error>(combined_output)
        });

        // Wait for the command to complete and get output
        child
            .wait()
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        let output_str = match output_task.await {
            Ok(result) => result
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?,
            Err(e) => {
                return Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    e.to_string(),
                    None,
                ))
            }
        };

        // Check the character count of the output
        const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB
        let char_count = output_str.chars().count();
        if char_count > MAX_CHAR_COUNT {
            return Err(ErrorData::new(ErrorCode::INTERNAL_ERROR, format!(
                    "Shell output from command '{}' has too many characters ({}). Maximum character count is {}.",
                    command,
                    char_count,
                    MAX_CHAR_COUNT
                ), None));
        }

        let (final_output, user_output) = self.process_shell_output(&output_str)?;

        Ok(CallToolResult::success(vec![
            Content::text(final_output).with_audience(vec![Role::Assistant]),
            Content::text(user_output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    /// Process an image file from disk. 
    /// 
    /// The image will be:
    /// 1. Resized if larger than max width while maintaining aspect ratio
    /// 2. Converted to PNG format
    /// 3. Returned as base64 encoded data
    /// 
    /// This allows processing image files for use in the conversation.
    #[tool(
        name = "image_processor",
        description = "Process an image file from disk. Resizes if needed, converts to PNG, and returns as base64 data."
    )]
    pub async fn image_processor(
        &self,
        params: Parameters<ImageProcessorParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path_str = &params.path;

        let path = {
            let p = self.resolve_path(path_str)?;
            if cfg!(target_os = "macos") {
                self.normalize_mac_screenshot_path(&p)
            } else {
                p
            }
        };

        // Check if file is ignored before proceeding
        if self.is_ignored(&path) {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Access to '{}' is restricted by .gooseignore",
                    path.display()
                ),
                None,
            ));
        }

        // Check if file exists
        if !path.exists() {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("File '{}' does not exist", path.display()),
                None,
            ));
        }

        // Check file size (10MB limit for image files)
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB in bytes
        let file_size = std::fs::metadata(&path)
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
                    "File '{}' is too large ({:.2}MB). Maximum size is 10MB.",
                    path.display(),
                    file_size as f64 / (1024.0 * 1024.0)
                ),
                None,
            ));
        }

        // Open and decode the image
        let image = xcap::image::open(&path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to open image file: {}", e),
                None,
            )
        })?;

        // Resize if necessary (same logic as screen_capture)
        let mut processed_image = image;
        let max_width = 768;
        if processed_image.width() > max_width {
            let scale = max_width as f32 / processed_image.width() as f32;
            let new_height = (processed_image.height() as f32 * scale) as u32;
            processed_image = xcap::image::DynamicImage::ImageRgba8(xcap::image::imageops::resize(
                &processed_image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            ));
        }

        // Convert to PNG and encode as base64
        let mut bytes: Vec<u8> = Vec::new();
        processed_image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write image buffer: {}", e),
                    None,
                )
            })?;

        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(CallToolResult::success(vec![
            Content::text(format!(
                "Successfully processed image from {}",
                path.display()
            ))
            .with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png").with_priority(0.0),
        ]))
    }

    // Helper method to resolve and validate file paths
    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, ErrorData> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = expand_path(path_str);
        let path = Path::new(&expanded);

        let suggestion = cwd.join(path);

        match is_absolute_path(&expanded) {
            true => Ok(path.to_path_buf()),
            false => Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "The path {} is not an absolute path, did you possibly mean {}?",
                    path_str,
                    suggestion.to_string_lossy(),
                ),
                None,
            )),
        }
    }

    // Helper method to validate and calculate view range indices
    fn calculate_view_range(
        &self,
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

            // Validate range
            if start_idx > total_lines {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!(
                        "Start line {} is beyond the end of the file (total lines: {})",
                        start_line, total_lines
                    ),
                    None,
                ));
            }

            if start_idx >= end_idx && end_idx != 0 {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Start line {} must be less than end line {}", start_line, end_line),
                    None,
                ));
            }

            Ok((start_idx, end_idx))
        } else {
            Ok((0, total_lines))
        }
    }

    async fn text_editor_view(
        &self,
        path: &PathBuf,
        view_range: Option<(usize, i64)>,
    ) -> Result<CallToolResult, ErrorData> {
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

        let (start_idx, end_idx) = self.calculate_view_range(view_range, total_lines)?;

        let selected_content = if start_idx == 0 && end_idx >= total_lines {
            // Show entire file
            content.clone()
        } else {
            // Show selected lines
            let selected_lines: Vec<String> = lines
                .iter()
                .skip(start_idx)
                .take(end_idx - start_idx)
                .enumerate()
                .map(|(i, line)| format!("{:6}|{}", start_idx + i + 1, line))
                .collect();

            selected_lines.join("\n")
        };

        let language = get_language_identifier(path);
        let display_content = if view_range.is_some() {
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
                content=selected_content,
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
                content=selected_content,
            }
        };

        Ok(CallToolResult::success(vec![
            Content::text(format!("Viewing {}", path.display()))
                .with_audience(vec![Role::Assistant]),
            Content::text(display_content)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    async fn text_editor_write(
        &self,
        path: &PathBuf,
        file_text: &str,
    ) -> Result<CallToolResult, ErrorData> {
        // Normalize line endings based on platform
        let mut normalized_text = normalize_line_endings(file_text);

        // Ensure the text ends with a newline
        if !normalized_text.ends_with('\n') {
            normalized_text.push('\n');
        }

        // Write to the file
        std::fs::write(path, &normalized_text).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to write file: {}", e),
                None,
            )
        })?;

        // Try to detect the language from the file extension
        let language = get_language_identifier(path);

        // The assistant output does not show the file again because the content is already in the tool request
        // but we do show it to the user here, using the final written content
        Ok(CallToolResult::success(vec![
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
        ]))
    }

    async fn text_editor_replace(
        &self,
        path: &PathBuf,
        old_str: &str,
        new_str: &str,
    ) -> Result<CallToolResult, ErrorData> {
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

        // Check if Editor API is configured and use it as the primary path
        if let Some(ref editor) = self.editor_model {
            // Editor API path - save history then call API directly
            self.save_file_history(path)?;

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
                    return Ok(CallToolResult::success(vec![
                        Content::text(format!("Successfully edited {}", path.display()))
                            .with_audience(vec![Role::Assistant]),
                        Content::text(format!("File {} has been edited", path.display()))
                            .with_audience(vec![Role::User])
                            .with_priority(0.2),
                    ]));
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

        // Traditional string replacement path (fallback)
        // Check if old_str exists in the file
        if !content.contains(old_str) {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("The old_str '{}' was not found in the file.", old_str),
                None,
            ));
        }

        // Save history for undo
        self.save_file_history(path)?;

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
        let language = get_language_identifier(path);

        // Show a snippet of the changed content with context
        const SNIPPET_LINES: usize = 4;

        // Count newlines before the replacement to find the line number
        let replacement_line = content
            .split(old_str)
            .next()
            .expect("should split on already matched content")
            .matches('\n')
            .count()
            + 1;

        // Get lines around the replacement for context
        let lines: Vec<&str> = normalized_content.lines().collect();
        let start_line = replacement_line.saturating_sub(SNIPPET_LINES);
        let end_line = std::cmp::min(replacement_line + SNIPPET_LINES, lines.len());

        let snippet_lines: Vec<String> = lines
            .iter()
            .skip(start_line.saturating_sub(1))
            .take(end_line - start_line.saturating_sub(1))
            .enumerate()
            .map(|(i, line)| format!("{:6}|{}", start_line + i, line))
            .collect();

        let snippet = snippet_lines.join("\n");

        Ok(CallToolResult::success(vec![
            Content::text(format!("Successfully edited {}", path.display()))
                .with_audience(vec![Role::Assistant]),
            Content::text(formatdoc! {
                r#"
                ### {path} (around line {line})
                ```{language}
                {snippet}
                ```
                "#,
                path=path.display(),
                line=replacement_line,
                language=language,
                snippet=snippet,
            })
            .with_audience(vec![Role::User])
            .with_priority(0.2),
        ]))
    }

    async fn text_editor_insert(
        &self,
        path: &PathBuf,
        insert_line: usize,
        new_str: &str,
    ) -> Result<CallToolResult, ErrorData> {
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
        self.save_file_history(path)?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

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

        // If inserting at the end, add the new text at the end
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
        let language = get_language_identifier(path);

        // Show a snippet of the inserted content with context
        const SNIPPET_LINES: usize = 4;
        let insertion_line = insert_line + 1; // Convert to 1-indexed for display

        // Calculate start and end lines for the snippet
        let start_line = insertion_line.saturating_sub(SNIPPET_LINES);
        let end_line = std::cmp::min(insertion_line + SNIPPET_LINES, new_lines.len());

        // Get the relevant lines for our snippet with line numbers
        let snippet_lines: Vec<String> = new_lines
            .iter()
            .skip(start_line.saturating_sub(1))
            .take(end_line - start_line.saturating_sub(1))
            .enumerate()
            .map(|(i, line)| format!("{:6}|{}", start_line + i, line))
            .collect();

        let snippet = snippet_lines.join("\n");

        Ok(CallToolResult::success(vec![
            Content::text(format!("Successfully inserted text at line {} in {}", insertion_line, path.display()))
                .with_audience(vec![Role::Assistant]),
            Content::text(formatdoc! {
                r#"
                ### {path} (around line {line})
                ```{language}
                {snippet}
                ```
                "#,
                path=path.display(),
                line=insertion_line,
                language=language,
                snippet=snippet,
            })
            .with_audience(vec![Role::User])
            .with_priority(0.2),
        ]))
    }

    async fn text_editor_undo(&self, path: &PathBuf) -> Result<CallToolResult, ErrorData> {
        let mut history = self.file_history.lock().unwrap();
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
                Ok(CallToolResult::success(vec![Content::text("Undid the last edit")]))
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

    fn save_file_history(&self, path: &PathBuf) -> Result<(), ErrorData> {
        let mut history = self.file_history.lock().unwrap();
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

        // Keep only the last 10 versions to prevent memory issues
        let entries = history.entry(path.clone()).or_insert_with(Vec::new);
        entries.push(content);
        if entries.len() > 10 {
            entries.remove(0);
        }

        Ok(())
    }

    // Helper method to check if a path should be ignored
    fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_patterns.matched(path, false).is_ignore()
    }

    // Helper function to handle Mac screenshot filenames that contain U+202F (narrow no-break space)
    fn normalize_mac_screenshot_path(&self, path: &Path) -> PathBuf {
        // Only process if the path has a filename
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            // Check if this matches Mac screenshot pattern:
            // "Screenshot YYYY-MM-DD at H.MM.SS AM/PM.png"
            if let Some(captures) = regex::Regex::new(r"^Screenshot \d{4}-\d{2}-\d{2} at \d{1,2}\.\d{2}\.\d{2} (AM|PM|am|pm)(?: \(\d+\))?\.png$")
                .ok()
                .and_then(|re| re.captures(filename))
            {
                // Get the AM/PM part
                let meridian = captures.get(1).unwrap().as_str();

                // Find the last space before AM/PM and replace it with U+202F
                let space_pos = filename.rfind(meridian)
                    .map(|pos| filename[..pos].trim_end().len())
                    .unwrap_or(0);

                if space_pos > 0 {
                    let parent = path.parent().unwrap_or(Path::new(""));
                    let new_filename = format!(
                        "{}{}{}",
                        &filename[..space_pos],
                        '\u{202F}',
                        &filename[space_pos+1..]
                    );
                    let new_path = parent.join(new_filename);

                    return new_path;
                }
            }
        }

        // Return the original path if it doesn't match or couldn't be processed
        path.to_path_buf()
    }

    // shell output can be large, this will help manage that
    fn process_shell_output(&self, output_str: &str) -> Result<(String, String), ErrorData> {
        let lines: Vec<&str> = output_str.lines().collect();
        let line_count = lines.len();

        let start = lines.len().saturating_sub(100);
        let last_100_lines_str = lines[start..].join("\n");

        let final_output = if line_count > 100 {
            let tmp_file = tempfile::NamedTempFile::new().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to create temporary file: {}", e),
                    None,
                )
            })?;

            std::fs::write(tmp_file.path(), output_str).map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write to temporary file: {}", e),
                    None,
                )
            })?;

            let (_, path) = tmp_file.keep().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to persist temporary file: {}", e),
                    None,
                )
            })?;

            format!(
                "private note: output was {} lines and we are only showing the most recent lines, remainder of lines in {} do not show tmp file to user, that file can be searched if extra context needed to fulfill request. truncated output: \n{}",
                line_count,
                path.display(),
                last_100_lines_str
            )
        } else {
            output_str.to_string()
        };

        let user_output = if line_count > 100 {
            format!(
                "NOTE: Output was {} lines, showing only the last 100 lines.\n\n{}",
                line_count,
                last_100_lines_str
            )
        } else {
            output_str.to_string()
        };

        Ok((final_output, user_output))
    }
}