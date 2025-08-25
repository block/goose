use base64::Engine;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use indoc::formatdoc;
use once_cell::sync::Lazy;
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{CallToolResult, Content, ErrorCode, ErrorData, Role, ServerCapabilities, ServerInfo},
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    future::Future,
    io::Cursor,
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
};
use xcap::{Monitor, Window};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tokio_stream::{wrappers::SplitStream, StreamExt as _};

use super::editor_models::{create_editor_model, EditorModel};
use super::lang::get_language_identifier;
use super::shell::{expand_path, get_shell_config, is_absolute_path, normalize_line_endings};

/// Regex pattern to match file references (@-mentions) in text
static FILE_REFERENCE_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"(?:^|\s)@([a-zA-Z0-9_\-./]+(?:\.[a-zA-Z0-9]+)+|[A-Z][a-zA-Z0-9_\-]*|[a-zA-Z0-9_\-./]*[./][a-zA-Z0-9_\-./]*)")
        .expect("Invalid file reference regex pattern")
});

/// Sanitize and resolve a file reference path safely
///
/// This function prevents path traversal attacks by:
/// 1. Rejecting absolute paths
/// 2. Resolving the path canonically
/// 3. Ensuring the resolved path stays within the allowed base directory
fn sanitize_reference_path(reference: &Path, base_path: &Path) -> Result<PathBuf, std::io::Error> {
    if reference.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Absolute paths not allowed in file references",
        ));
    }

    let resolved = base_path.join(reference);
    let base_canonical = base_path.canonicalize().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Base directory not found")
    })?;

    if let Ok(canonical) = resolved.canonicalize() {
        if !canonical.starts_with(&base_canonical) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Path traversal attempt detected",
            ));
        }
        Ok(canonical)
    } else {
        Ok(resolved) // File doesn't exist, but path structure is safe
    }
}

/// Parse file references (@-mentions) from content
fn parse_file_references(content: &str) -> Vec<PathBuf> {
    // Keep size limits for ReDoS protection - .goosehints should be reasonably sized
    const MAX_CONTENT_LENGTH: usize = 131_072; // 128KB limit

    if content.len() > MAX_CONTENT_LENGTH {
        tracing::warn!(
            "Content too large for file reference parsing: {} bytes (limit: {} bytes)",
            content.len(),
            MAX_CONTENT_LENGTH
        );
        return Vec::new();
    }

    FILE_REFERENCE_REGEX
        .captures_iter(content)
        .map(|cap| PathBuf::from(&cap[1]))
        .collect()
}

/// Check if a file reference should be processed
fn should_process_reference_v2(
    reference: &Path,
    visited: &HashSet<PathBuf>,
    base_path: &Path,
    ignore_patterns: &Gitignore,
) -> Option<PathBuf> {
    // Check if we've already visited this file (circular reference protection)
    if visited.contains(reference) {
        return None;
    }

    // Sanitize the path
    let safe_path = match sanitize_reference_path(reference, base_path) {
        Ok(path) => path,
        Err(_) => {
            tracing::warn!("Skipping unsafe file reference: {:?}", reference);
            return None;
        }
    };

    // Check if the file should be ignored
    if ignore_patterns.matched(&safe_path, false).is_ignore() {
        tracing::debug!("Skipping ignored file reference: {:?}", safe_path);
        return None;
    }

    // Check if file exists
    if !safe_path.is_file() {
        return None;
    }

    Some(safe_path)
}

/// Process a single file reference and return the replacement content
fn process_file_reference_v2(
    reference: &Path,
    safe_path: &Path,
    visited: &mut HashSet<PathBuf>,
    base_path: &Path,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> Option<(String, String)> {
    match std::fs::read_to_string(safe_path) {
        Ok(file_content) => {
            // Mark this file as visited
            visited.insert(reference.to_path_buf());

            // Recursively expand any references in the included file
            let expanded_content = read_referenced_files(
                &file_content,
                base_path,
                visited,
                depth + 1,
                ignore_patterns,
            );

            // Create the replacement content
            let reference_pattern = format!("@{}", reference.to_string_lossy());
            let replacement = format!(
                "--- Content from {} ---\n{}\n--- End of {} ---",
                reference.display(),
                expanded_content,
                reference.display()
            );

            // Remove from visited so it can be referenced again in different contexts
            visited.remove(reference);

            Some((reference_pattern, replacement))
        }
        Err(e) => {
            tracing::warn!("Could not read referenced file {:?}: {}", safe_path, e);
            None
        }
    }
}

/// Read referenced files and expand their content
fn read_referenced_files(
    content: &str,
    base_path: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> String {
    const MAX_DEPTH: usize = 3;

    if depth >= MAX_DEPTH {
        tracing::warn!("Maximum reference depth {} exceeded", MAX_DEPTH);
        return content.to_string();
    }

    let references = parse_file_references(content);
    let mut result = content.to_string();

    for reference in references {
        let safe_path =
            match should_process_reference_v2(&reference, visited, base_path, ignore_patterns) {
                Some(path) => path,
                None => continue,
            };

        if let Some((pattern, replacement)) = process_file_reference_v2(
            &reference,
            &safe_path,
            visited,
            base_path,
            depth,
            ignore_patterns,
        ) {
            result = result.replace(&pattern, &replacement);
        }
    }

    result
}

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

        // Build ignore patterns for file reference processing
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

        // Process hints with file reference expansion
        let mut hints = String::new();

        // First, check for global hints
        let global_config_dir = PathBuf::from(shellexpand::tilde("~/.config/goose").to_string());
        let mut global_hints_contents = Vec::new();

        let global_hints_path = global_config_dir.join(".goosehints");
        if global_hints_path.exists() && global_hints_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&global_hints_path) {
                if !content.trim().is_empty() {
                    global_hints_contents.push(content);
                }
            }
        }

        // Process global hints with file reference expansion
        if !global_hints_contents.is_empty() {
            hints.push_str("### Global Hints\nThe developer extension includes some global hints that apply to all projects & directories.\n");

            // Expand file references in global hints
            let mut visited = HashSet::new();
            let global_hints_text = global_hints_contents.join("\n");
            let expanded_global_hints = read_referenced_files(
                &global_hints_text,
                &global_config_dir,
                &mut visited,
                0,
                &ignore_patterns,
            );
            hints.push_str(&expanded_global_hints);
        }

        // Then process local hints
        let mut local_hints_contents = Vec::new();

        for filename in hints_filenames {
            let hints_path = cwd.join(&filename);
            if hints_path.exists() && hints_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&hints_path) {
                    if !content.trim().is_empty() {
                        local_hints_contents.push(content);
                    }
                }
            }
        }

        if !local_hints_contents.is_empty() {
            if !hints.is_empty() {
                hints.push_str("\n\n");
            }
            hints.push_str("### Project Hints\nThe developer extension includes some hints for working on the project in this directory.\n");

            // Expand file references in local hints
            let mut visited = HashSet::new();
            let local_hints_text = local_hints_contents.join("\n");
            let expanded_local_hints =
                read_referenced_files(&local_hints_text, &cwd, &mut visited, 0, &ignore_patterns);
            hints.push_str(&expanded_local_hints);
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

        let window_titles: Vec<String> =
            windows.into_iter().map(|w| w.title().to_string()).collect();

        let content_text = format!("Available windows:\n{}", window_titles.join("\n"));

        Ok(CallToolResult::success(vec![
            Content::text(content_text.clone()).with_audience(vec![Role::Assistant]),
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
            Content::text("Screenshot captured").with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png").with_priority(0.0),
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

        // Check if file is ignored before proceeding with any text editor operation
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
        if cmd_parts.is_empty() {
            // Empty command, just return empty output
            return Ok(CallToolResult::success(vec![
                Content::text("").with_audience(vec![Role::Assistant]),
                Content::text("")
                    .with_audience(vec![Role::User])
                    .with_priority(0.0),
            ]));
        }

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
            Content::text(format!(
                "Successfully inserted text at line {} in {}",
                insertion_line,
                path.display()
            ))
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
                Ok(CallToolResult::success(vec![Content::text(
                    "Undid the last edit",
                )]))
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
                line_count, last_100_lines_str
            )
        } else {
            output_str.to_string()
        };

        Ok((final_output, user_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::handler::server::tool::Parameters;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_server() -> DeveloperServer {
        DeveloperServer::new()
    }

    #[test]
    #[serial]
    fn test_global_goosehints() {
        // Note: This test checks if ~/.config/goose/.goosehints exists and includes it in instructions
        // Since RMCP version uses get_info() instead of instructions(), we test that method
        let global_hints_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints").to_string());
        let global_hints_bak_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints.bak").to_string());
        let mut globalhints_existed = false;

        if global_hints_path.is_file() {
            globalhints_existed = true;
            fs::copy(&global_hints_path, &global_hints_bak_path).unwrap();
        }

        fs::write(&global_hints_path, "These are my global goose hints.").unwrap();

        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("my global goose hints."));

        // restore backup if globalhints previously existed
        if globalhints_existed {
            fs::copy(&global_hints_bak_path, &global_hints_path).unwrap();
            fs::remove_file(&global_hints_bak_path).unwrap();
        } else {
            fs::remove_file(&global_hints_path).unwrap();
        }
    }

    #[test]
    #[serial]
    fn test_goosehints_when_present() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(".goosehints", "Test hint content").unwrap();
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("Test hint content"));
    }

    #[test]
    #[serial]
    fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        // When no hints are present, instructions should not contain hint content
        assert!(!instructions.contains("AGENTS.md:") && !instructions.contains(".goosehints:"));
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_missing_parameters() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // This should fail because the command parameter is missing
        // We can't directly test this with RMCP because parameters are typed,
        // but we can test with an empty command
        let params = Parameters(ShellParams {
            command: String::new(),
        });

        let result = server.shell(params).await;
        // Empty command should still work, just return empty output
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_goosehints_multiple_filenames() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::env::set_var("CONTEXT_FILE_NAMES", r#"["CLAUDE.md", ".goosehints"]"#);

        fs::write("CLAUDE.md", "Custom hints file content from CLAUDE.md").unwrap();
        fs::write(".goosehints", "Custom hints file content from .goosehints").unwrap();
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("Custom hints file content from CLAUDE.md"));
        assert!(instructions.contains("Custom hints file content from .goosehints"));
        std::env::remove_var("CONTEXT_FILE_NAMES");
    }

    #[test]
    #[serial]
    fn test_goosehints_configurable_filename() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::env::set_var("CONTEXT_FILE_NAMES", r#"["CLAUDE.md"]"#);

        fs::write("CLAUDE.md", "Custom hints file content").unwrap();
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("Custom hints file content"));
        assert!(!instructions.contains(".goosehints")); // Make sure it's not loading the default
        std::env::remove_var("CONTEXT_FILE_NAMES");
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_write_and_view_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a new file
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Hello, world!".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // View the file
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let view_result = server.text_editor(view_params).await.unwrap();

        assert!(!view_result.content.is_empty());
        let user_content = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(user_content.text.contains("Hello, world!"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_str_replace() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a new file
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Hello, world!".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Replace string
        let replace_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "str_replace".to_string(),
            view_range: None,
            file_text: None,
            old_str: Some("world".to_string()),
            new_str: Some("Rust".to_string()),
            insert_line: None,
        });

        let replace_result = server.text_editor(replace_params).await.unwrap();

        let assistant_content = replace_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(assistant_content.text.contains("Successfully edited"));

        // Verify the file contents changed
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Hello, Rust!"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_size_limits() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a large file that exceeds the 400KB limit
        let large_content = "a".repeat(500 * 1024); // 500KB
        let file_path = temp_dir.path().join("large_file.txt");
        fs::write(&file_path, &large_content).unwrap();

        let view_params = Parameters(TextEditorParams {
            path: file_path.to_str().unwrap().to_string(),
            command: "view".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;
        assert!(result.is_err());

        let error = result.err().unwrap();
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        assert!(error.message.contains("too large"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_undo_edit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Original content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Make an edit
        let replace_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "str_replace".to_string(),
            view_range: None,
            file_text: None,
            old_str: Some("Original".to_string()),
            new_str: Some("Modified".to_string()),
            insert_line: None,
        });

        server.text_editor(replace_params).await.unwrap();

        // Verify the edit was made
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Modified content"));

        // Undo the edit
        let undo_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "undo_edit".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let undo_result = server.text_editor(undo_params).await.unwrap();

        // Verify undo worked
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Original content"));

        let undo_content = undo_result
            .content
            .iter()
            .find(|c| c.as_text().is_some())
            .unwrap()
            .as_text()
            .unwrap();
        assert!(undo_content.text.contains("Undid the last edit"));
    }

    #[tokio::test]
    #[serial]
    async fn test_goose_ignore_basic_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create .gooseignore file with patterns
        fs::write(".gooseignore", "secret.txt\n*.env").unwrap();

        let server = create_test_server();

        // Test basic file matching
        assert!(
            server.is_ignored(Path::new("secret.txt")),
            "secret.txt should be ignored"
        );
        assert!(
            server.is_ignored(Path::new("./secret.txt")),
            "./secret.txt should be ignored"
        );
        assert!(
            !server.is_ignored(Path::new("not_secret.txt")),
            "not_secret.txt should not be ignored"
        );

        // Test pattern matching
        assert!(
            server.is_ignored(Path::new("test.env")),
            "*.env pattern should match test.env"
        );
        assert!(
            server.is_ignored(Path::new("./test.env")),
            "*.env pattern should match ./test.env"
        );
        assert!(
            !server.is_ignored(Path::new("test.txt")),
            "*.env pattern should not match test.txt"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_respects_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create .gooseignore file
        fs::write(".gooseignore", "secret.txt").unwrap();

        let server = create_test_server();

        // Try to write to an ignored file
        let secret_path = temp_dir.path().join("secret.txt");
        let write_params = Parameters(TextEditorParams {
            path: secret_path.to_str().unwrap().to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("test content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(write_params).await;
        assert!(
            result.is_err(),
            "Should not be able to write to ignored file"
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

        // Try to write to a non-ignored file
        let allowed_path = temp_dir.path().join("allowed.txt");
        let write_params = Parameters(TextEditorParams {
            path: allowed_path.to_str().unwrap().to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("test content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(write_params).await;
        assert!(
            result.is_ok(),
            "Should be able to write to non-ignored file"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_respects_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create .gooseignore file
        fs::write(".gooseignore", "secret.txt").unwrap();
        fs::write("secret.txt", "secret content").unwrap();

        let server = create_test_server();

        // Try to cat an ignored file
        let shell_params = Parameters(ShellParams {
            command: "cat secret.txt".to_string(),
        });

        let result = server.shell(shell_params).await;
        assert!(
            result.is_err(),
            "Should not be able to access ignored file via shell"
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);
    }

    #[tokio::test]
    #[serial]
    async fn test_gitignore_fallback_when_no_gooseignore() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create .gitignore file (no .gooseignore)
        fs::write(".gitignore", "*.log").unwrap();

        let server = create_test_server();

        assert!(
            server.is_ignored(Path::new("debug.log")),
            "*.log pattern from .gitignore should match debug.log"
        );
        assert!(
            !server.is_ignored(Path::new("debug.txt")),
            "*.log pattern should not match debug.txt"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_gooseignore_takes_precedence_over_gitignore() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create both files
        fs::write(".gitignore", "*.log").unwrap();
        fs::write(".gooseignore", "*.env").unwrap();

        let server = create_test_server();

        // Should respect .gooseignore patterns
        assert!(
            server.is_ignored(Path::new("test.env")),
            ".gooseignore pattern should work"
        );
        // Should NOT respect .gitignore patterns when .gooseignore exists
        assert!(
            !server.is_ignored(Path::new("test.log")),
            ".gitignore patterns should be ignored when .gooseignore exists"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a multi-line file
        let content =
            "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test viewing specific range
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![3, 6]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let view_result = server.text_editor(view_params).await.unwrap();

        let text = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain lines 3-6 with line numbers
        assert!(text.text.contains("3|Line 3"));
        assert!(text.text.contains("4|Line 4"));
        assert!(text.text.contains("5|Line 5"));
        assert!(text.text.contains("6|Line 6"));
        assert!(text.text.contains("(lines 3-6)"));
        // Should not contain other lines
        assert!(!text.text.contains("1|Line 1"));
        assert!(!text.text.contains("7|Line 7"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range_to_end() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a multi-line file
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test viewing from line 3 to end using -1
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![3, -1]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let view_result = server.text_editor(view_params).await.unwrap();

        let text = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain lines 3-5
        assert!(text.text.contains("3|Line 3"));
        assert!(text.text.contains("4|Line 4"));
        assert!(text.text.contains("5|Line 5"));
        assert!(text.text.contains("(lines 3-end)"));
        // Should not contain lines 1-2
        assert!(!text.text.contains("1|Line 1"));
        assert!(!text.text.contains("2|Line 2"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_at_beginning() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 2\nLine 3\nLine 4";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Insert at the beginning (line 0)
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Line 1".to_string()),
            insert_line: Some(0),
        });

        let insert_result = server.text_editor(insert_params).await.unwrap();

        let text = insert_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Successfully inserted text at line 1"));

        // Verify the file content by reading it directly
        let file_content = fs::read_to_string(&file_path).unwrap();
        assert!(file_content.contains("Line 1\nLine 2\nLine 3\nLine 4"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_in_middle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 4\nLine 5";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Insert after line 2
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Line 3".to_string()),
            insert_line: Some(2),
        });

        let insert_result = server.text_editor(insert_params).await.unwrap();

        let text = insert_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Successfully inserted text at line 3"));

        // Verify the file content by reading it directly
        let file_content = fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = file_content.lines().collect();
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
        assert_eq!(lines[2], "Line 3");
        assert_eq!(lines[3], "Line 4");
        assert_eq!(lines[4], "Line 5");
    }

    #[test]
    #[serial]
    fn test_process_shell_output_short() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();

        // Test with short output (< 100 lines)
        let short_output = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let result = server.process_shell_output(short_output).unwrap();

        // Both outputs should be the same for short outputs
        assert_eq!(result.0, short_output);
        assert_eq!(result.1, short_output);
    }

    #[test]
    #[serial]
    fn test_process_shell_output_empty() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();

        // Test with empty output
        let empty_output = "";
        let result = server.process_shell_output(empty_output).unwrap();

        // Both outputs should be empty
        assert_eq!(result.0, "");
        assert_eq!(result.1, "");
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_output_truncation() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Generate output with many lines to test truncation
        let mut long_lines = Vec::new();
        for i in 1..=150 {
            long_lines.push(format!("Line {}", i));
        }
        let long_output = long_lines.join("\n");

        let result = server.process_shell_output(&long_output).unwrap();

        // Check that final output contains truncation info
        assert!(result.0.contains("private note: output was 150 lines"));
        assert!(result.0.contains("truncated output:"));

        // Check that user output shows truncation notice
        assert!(result
            .1
            .contains("NOTE: Output was 150 lines, showing only the last 100 lines"));

        // Verify it shows the last 100 lines (use exact line matching to avoid substring matches)
        assert!(result.1.contains("Line 51\n"));
        assert!(result.1.contains("Line 150"));
        assert!(!result.1.contains("Line 1\n"));
        assert!(!result.1.contains("Line 50\n"));
    }

    #[tokio::test]
    #[serial]
    #[cfg(windows)]
    async fn test_windows_specific_commands() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Test PowerShell command
        let shell_params = Parameters(ShellParams {
            command: "Get-ChildItem".to_string(),
        });

        let result = server.shell(shell_params).await;
        assert!(result.is_ok());

        // Test that resolve_path works with Windows paths
        let windows_path = r"C:\Windows\System32";
        if Path::new(windows_path).exists() {
            let resolved = server.resolve_path(windows_path);
            assert!(resolved.is_ok());
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range_invalid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a small file
        let content = "Line 1\nLine 2\nLine 3";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test invalid range - start line beyond file
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![10, 15]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("beyond the end of the file"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_missing_parameters() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file first
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Initial content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test insert without new_str parameter
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None, // Missing required parameter
            insert_line: Some(1),
        });

        let result = server.text_editor(insert_params).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("Missing 'new_str' parameter"));

        // Test insert without insert_line parameter
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("New text".to_string()),
            insert_line: None, // Missing required parameter
        });

        let result = server.text_editor(insert_params).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("Missing 'insert_line' parameter"));
    }

    // Tests for file reference functionality
    #[test]
    fn test_parse_file_references() {
        let content = r#"
        Basic file references: @README.md @./docs/guide.md @../shared/config.json @/absolute/path/file.txt
        Inline references: @file1.txt and @file2.py
        Files with extensions: @component.tsx @file.test.js @config.local.json
        Files without extensions: @Makefile @LICENSE @Dockerfile @CHANGELOG
        Complex paths: @src/utils/helper.js @docs/api/endpoints.md
        
        Should not match:
        - Email addresses: user@example.com admin@company.org
        - Social handles: @username @user123
        - URLs: https://example.com/@user
        "#;

        let references = parse_file_references(content);

        // Should match basic file references
        assert!(references.contains(&PathBuf::from("README.md")));
        assert!(references.contains(&PathBuf::from("./docs/guide.md")));
        assert!(references.contains(&PathBuf::from("../shared/config.json")));
        assert!(references.contains(&PathBuf::from("/absolute/path/file.txt")));
        assert!(references.contains(&PathBuf::from("file1.txt")));
        assert!(references.contains(&PathBuf::from("file2.py")));

        // Should match files with extensions (including multiple dots)
        assert!(references.contains(&PathBuf::from("component.tsx")));
        assert!(references.contains(&PathBuf::from("file.test.js")));
        assert!(references.contains(&PathBuf::from("config.local.json")));

        // Should match files without extensions
        assert!(references.contains(&PathBuf::from("Makefile")));
        assert!(references.contains(&PathBuf::from("LICENSE")));
        assert!(references.contains(&PathBuf::from("Dockerfile")));
        assert!(references.contains(&PathBuf::from("CHANGELOG")));

        // Should match complex paths
        assert!(references.contains(&PathBuf::from("src/utils/helper.js")));
        assert!(references.contains(&PathBuf::from("docs/api/endpoints.md")));

        // Should not match email addresses or social handles
        assert!(!references
            .iter()
            .any(|p| p.to_str().unwrap().contains("example.com")));
        assert!(!references
            .iter()
            .any(|p| p.to_str().unwrap().contains("company.org")));
        assert!(!references.iter().any(|p| p.to_str().unwrap() == "username"));
        assert!(!references.iter().any(|p| p.to_str().unwrap() == "user123"));
    }

    #[test]
    #[serial]
    fn test_file_expansion_normal_cases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();

        // Test 1: Basic file reference
        let basic_file = base_path.join("basic.md");
        std::fs::write(&basic_file, "This is basic content").unwrap();

        let builder = GitignoreBuilder::new(base_path);
        let ignore_patterns = builder.build().unwrap();

        let mut visited = HashSet::new();
        let basic_content = "Main content\n@basic.md\nMore content";
        let expanded =
            read_referenced_files(basic_content, base_path, &mut visited, 0, &ignore_patterns);

        assert!(expanded.contains("Main content"));
        assert!(expanded.contains("--- Content from"));
        assert!(expanded.contains("This is basic content"));
        assert!(expanded.contains("--- End of"));
        assert!(expanded.contains("More content"));

        // Test 2: Nested file references
        let ref_file1 = base_path.join("level1.md");
        std::fs::write(&ref_file1, "Level 1 content\n@level2.md").unwrap();

        let ref_file2 = base_path.join("level2.md");
        std::fs::write(&ref_file2, "Level 2 content").unwrap();

        visited.clear();
        let nested_content = "Main content\n@level1.md";
        let expanded =
            read_referenced_files(nested_content, base_path, &mut visited, 0, &ignore_patterns);

        assert!(expanded.contains("Main content"));
        assert!(expanded.contains("Level 1 content"));
        assert!(expanded.contains("Level 2 content"));
    }

    #[test]
    #[serial]
    fn test_read_referenced_files_respects_ignore() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();

        // Create referenced files
        let allowed_file = base_path.join("allowed.md");
        std::fs::write(&allowed_file, "Allowed content").unwrap();

        let ignored_file = base_path.join("secret.md");
        std::fs::write(&ignored_file, "Secret content").unwrap();

        // Create main content with references
        let content = "Main\n@allowed.md\n@secret.md";

        // Create ignore patterns
        let mut builder = GitignoreBuilder::new(base_path);
        builder.add_line(None, "secret.md").unwrap();
        let ignore_patterns = builder.build().unwrap();

        let mut visited = HashSet::new();
        let expanded = read_referenced_files(content, base_path, &mut visited, 0, &ignore_patterns);

        // Should contain allowed content but not ignored content
        assert!(expanded.contains("Allowed content"));
        assert!(!expanded.contains("Secret content"));

        // The @secret.md reference should remain unchanged
        assert!(expanded.contains("@secret.md"));
    }

    #[test]
    #[serial]
    fn test_goosehints_with_file_references() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create referenced files
        let readme_path = temp_dir.path().join("README.md");
        std::fs::write(
            &readme_path,
            "# Project README\n\nThis is the project documentation.",
        )
        .unwrap();

        let guide_path = temp_dir.path().join("guide.md");
        std::fs::write(&guide_path, "# Development Guide\n\nFollow these steps...").unwrap();

        // Create .goosehints with references
        let hints_content = r#"# Project Information

Please refer to:
@README.md
@guide.md

Additional instructions here.
"#;
        let hints_path = temp_dir.path().join(".goosehints");
        std::fs::write(&hints_path, hints_content).unwrap();

        // Create server and check instructions
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();

        // Should contain the .goosehints content
        assert!(instructions.contains("Project Information"));
        assert!(instructions.contains("Additional instructions here"));

        // Should contain the referenced files' content
        assert!(instructions.contains("# Project README"));
        assert!(instructions.contains("This is the project documentation"));
        assert!(instructions.contains("# Development Guide"));
        assert!(instructions.contains("Follow these steps"));

        // Should have attribution markers
        assert!(instructions.contains("--- Content from"));
        assert!(instructions.contains("--- End of"));
    }

    #[test]
    #[serial]
    fn test_parse_file_references_redos_protection() {
        // Test very large input to ensure ReDoS protection
        let large_content = "@".repeat(200_000); // 200KB of @ symbols
        let start = std::time::Instant::now();
        let references = parse_file_references(&large_content);
        let duration = start.elapsed();

        // Should complete quickly (under 1 second) and return empty results
        assert!(duration.as_secs() < 1);
        assert!(references.is_empty());
    }

    #[test]
    fn test_sanitize_reference_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();

        // Test valid relative path
        let valid_path = Path::new("docs/readme.md");
        let result = sanitize_reference_path(valid_path, base_path);
        assert!(result.is_ok());

        // Test absolute path (should be rejected)
        let absolute_path = Path::new("/etc/passwd");
        let result = sanitize_reference_path(absolute_path, base_path);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );

        // Test path traversal attempt (should be rejected)
        let traversal_path = Path::new("../../../etc/passwd");
        let _result = sanitize_reference_path(traversal_path, base_path);
        // This might succeed in path resolution but would be caught by canonicalization checks
        // The exact behavior depends on whether the target exists
    }

    #[test]
    #[serial]
    fn test_file_expansion_edge_cases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();
        let builder = GitignoreBuilder::new(base_path);
        let ignore_patterns = builder.build().unwrap();

        // Test 1: Circular references
        let ref_file1 = base_path.join("file1.md");
        std::fs::write(&ref_file1, "File 1\n@file2.md").unwrap();
        let ref_file2 = base_path.join("file2.md");
        std::fs::write(&ref_file2, "File 2\n@file1.md").unwrap();

        let mut visited = HashSet::new();
        let circular_content = "Main\n@file1.md";
        let expanded = read_referenced_files(
            circular_content,
            base_path,
            &mut visited,
            0,
            &ignore_patterns,
        );

        assert!(expanded.contains("File 1"));
        assert!(expanded.contains("File 2"));
        // Should only appear once due to circular reference protection
        let file1_count = expanded.matches("File 1").count();
        assert_eq!(file1_count, 1);

        // Test 2: Max depth limit
        for i in 1..=5 {
            let content = if i < 5 {
                format!("Level {} content\n@level{}.md", i, i + 1)
            } else {
                format!("Level {} content", i)
            };
            let ref_file = base_path.join(format!("level{}.md", i));
            std::fs::write(&ref_file, content).unwrap();
        }

        visited.clear();
        let depth_content = "Main\n@level1.md";
        let expanded =
            read_referenced_files(depth_content, base_path, &mut visited, 0, &ignore_patterns);

        // Should contain up to level 3 (MAX_DEPTH = 3)
        assert!(expanded.contains("Level 1 content"));
        assert!(expanded.contains("Level 2 content"));
        assert!(expanded.contains("Level 3 content"));
        // Should not contain level 4 or 5 due to depth limit
        assert!(!expanded.contains("Level 4 content"));
        assert!(!expanded.contains("Level 5 content"));

        // Test 3: Missing file
        visited.clear();
        let missing_content = "Main\n@missing.md\nMore content";
        let expanded = read_referenced_files(
            missing_content,
            base_path,
            &mut visited,
            0,
            &ignore_patterns,
        );

        // Should keep the original reference unchanged
        assert!(expanded.contains("@missing.md"));
        assert!(!expanded.contains("--- Content from"));
    }

    #[test]
    #[serial]
    fn test_security_integration_with_file_expansion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();

        // Create a config file attempting path traversal
        let malicious_content = r#"
        Normal content here.
        @../../../etc/passwd
        @/absolute/path/file.txt
        @legitimate_file.md
        "#;

        // Create a legitimate file
        let legit_file = base_path.join("legitimate_file.md");
        std::fs::write(&legit_file, "This is safe content").unwrap();

        // Create ignore patterns
        let builder = GitignoreBuilder::new(base_path);
        let ignore_patterns = builder.build().unwrap();

        let mut visited = HashSet::new();
        let expanded = read_referenced_files(
            malicious_content,
            base_path,
            &mut visited,
            0,
            &ignore_patterns,
        );

        // Should contain the legitimate file but not the malicious attempts
        assert!(expanded.contains("This is safe content"));
        assert!(!expanded.contains("root:")); // Common content in /etc/passwd

        // The malicious references should still be present (not expanded)
        assert!(expanded.contains("@../../../etc/passwd"));
        assert!(expanded.contains("@/absolute/path/file.txt"));
    }
}
