mod lang;

use anyhow::Result;
use base64::Engine;
use indoc::formatdoc;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    future::Future,
    io::Cursor,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::process::Command;
use url::Url;

use mcp_core::{
    handler::{ResourceError, ToolError},
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

use mcp_core::content::Content;
use mcp_core::role::Role;

use indoc::indoc;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use xcap::{Monitor, Window};

pub struct DeveloperRouter {
    tools: Vec<Tool>,
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    instructions: String,
}

impl Default for DeveloperRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl DeveloperRouter {
    pub fn new() -> Self {
        // TODO consider rust native search tools, we could use
        // https://docs.rs/ignore/latest/ignore/

        let bash_tool = Tool::new(
            "shell".to_string(),
            indoc! {r#"
                Execute a command in the shell.

                This will return the output and error concatenated into a single string, as
                you would see from running on the command line. There will also be an indication
                of if the command succeeded or failed.

                Avoid commands that produce a large amount of ouput, and consider piping those outputs to files.
                If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
                this tool does not run indefinitely.

                **Important**: Use ripgrep - `rg` - when you need to locate a file or a code reference, other solutions
                may show ignored or hidden files. For example *do not* use `find` or `ls -r`
                  - To locate a file by name: `rg --files | rg example.py`
                  - To locate consent inside files: `rg 'class Example'`
            "#}.to_string(),
            json!({
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {"type": "string"}
                }
            }),
        );

        let text_editor_tool = Tool::new(
            "text_editor".to_string(),
            indoc! {r#"
                Perform text editing operations on files.

                The `command` parameter specifies the operation to perform. Allowed options are:
                - `view`: View the content of a file.
                - `write`: Create or overwrite a file with the given content
                - `str_replace`: Replace a string in a file with a new string.
                - `undo_edit`: Undo the last edit made to a file.

                To use the write command, you must specify `file_text` which will become the new content of the file. Be careful with
                existing files! This is a full overwrite, so you must include everything - not just sections you are modifying.

                To use the str_replace command, you must specify both `old_str` and `new_str` - the `old_str` needs to exactly match one
                unique section of the original file, including any whitespace. Make sure to include enough context that the match is not
                ambiguous. The entire original string will be replaced with `new_str`.
            "#}.to_string(),
            json!({
                "type": "object",
                "required": ["command", "path"],
                "properties": {
                    "path": {
                        "description": "Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`.",
                        "type": "string"
                    },
                    "command": {
                        "type": "string",
                        "enum": ["view", "write", "str_replace", "undo_edit"],
                        "description": "Allowed options are: `view`, `write`, `str_replace`, undo_edit`."
                    },
                    "old_str": {"type": "string"},
                    "new_str": {"type": "string"},
                    "file_text": {"type": "string"}
                }
            }),
        );

        let list_windows_tool = Tool::new(
            "list_windows",
            indoc! {r#"
                List all available window titles that can be used with screen_capture.
                Returns a list of window titles that can be used with the window_title parameter
                of the screen_capture tool.
            "#},
            json!({
                "type": "object",
                "required": [],
                "properties": {}
            }),
        );

        let screen_capture_tool = Tool::new(
            "screen_capture",
            indoc! {r#"
                Capture a screenshot of a specified display or window.
                You can capture either:
                1. A full display (monitor) using the display parameter
                2. A specific window by its title using the window_title parameter

                Only one of display or window_title should be specified.
            "#},
            json!({
                "type": "object",
                "required": [],
                "properties": {
                    "display": {
                        "type": "integer",
                        "default": 0,
                        "description": "The display number to capture (0 is main display)"
                    },
                    "window_title": {
                        "type": "string",
                        "default": null,
                        "description": "Optional: the exact title of the window to capture. use the list_windows tool to find the available windows."
                    }
                }
            }),
        );

        // Get base instructions and working directory
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let base_instructions = formatdoc! {r#"
            The developer extension gives you the capabilities to edit code files and run shell commands,
            and can be used to solve a wide range of problems.

            You can use the shell tool to run any command that would work on the relevant operating system.
            Use the shell tool as needed to locate files or interact with the project.

            Your windows/screen tools can be used for visual debugging. You should not use these tools unless
            prompted to, but you can mention they are available if they are relevant.

            operating system: {os}
            current directory: {cwd}

            "#,
            os=std::env::consts::OS,
            cwd=cwd.to_string_lossy(),
        };

        // Check for and read .goosehints file if it exists
        let hints_path = cwd.join(".goosehints");
        let instructions = if hints_path.is_file() {
            if let Ok(hints) = std::fs::read_to_string(&hints_path) {
                format!("{base_instructions}\n### Project Hints\nThe developer extension includes some hints for working on the project in this directory.\n{hints}")
            } else {
                base_instructions
            }
        } else {
            base_instructions
        };

        Self {
            tools: vec![
                bash_tool,
                text_editor_tool,
                list_windows_tool,
                screen_capture_tool,
            ],
            file_history: Arc::new(Mutex::new(HashMap::new())),
            instructions,
        }
    }

    // Helper method to resolve a path relative to cwd
    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, ToolError> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = shellexpand::tilde(path_str);
        let path = Path::new(expanded.as_ref());

        let suggestion = cwd.join(path);

        match path.is_absolute() {
            true => Ok(path.to_path_buf()),
            false => Err(ToolError::InvalidParameters(format!(
                "The path {} is not an absolute path, did you possibly mean {}?",
                path_str,
                suggestion.to_string_lossy(),
            ))),
        }
    }

    // Implement bash tool functionality
    async fn bash(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let command =
            params
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The command string is required".to_string(),
                ))?;

        // TODO consider command suggestions and safety rails

        // TODO be more careful about backgrounding, revisit interleave
        // Redirect stderr to stdout to interleave outputs
        let cmd_with_redirect = format!("{} 2>&1", command);

        // Execute the command
        let child = Command::new("bash")
            .stdout(Stdio::piped()) // These two pipes required to capture output later.
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true) // Critical so that the command is killed when the agent.reply stream is interrupted.
            .arg("-c")
            .arg(cmd_with_redirect)
            .spawn()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Wait for the command to complete and get output
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Check the character count of the output
        const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB
        let char_count = output_str.chars().count();
        if char_count > MAX_CHAR_COUNT {
            return Err(ToolError::ExecutionError(format!(
                    "Shell output from command '{}' has too many characters ({}). Maximum character count is {}.",
                    command,
                    char_count,
                    MAX_CHAR_COUNT
                )));
        }

        Ok(vec![
            Content::text(output_str.clone()).with_audience(vec![Role::Assistant]),
            Content::text(output_str)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    }

    async fn text_editor(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidParameters("Missing 'command' parameter".to_string())
            })?;

        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'path' parameter".into()))?;

        let path = self.resolve_path(path_str)?;

        match command {
            "view" => self.text_editor_view(&path).await,
            "write" => {
                let file_text = params
                    .get("file_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ToolError::InvalidParameters("Missing 'file_text' parameter".into())
                    })?;

                self.text_editor_write(&path, file_text).await
            }
            "str_replace" => {
                let old_str = params
                    .get("old_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ToolError::InvalidParameters("Missing 'old_str' parameter".into())
                    })?;
                let new_str = params
                    .get("new_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ToolError::InvalidParameters("Missing 'new_str' parameter".into())
                    })?;

                self.text_editor_replace(&path, old_str, new_str).await
            }
            "undo_edit" => self.text_editor_undo(&path).await,
            _ => Err(ToolError::InvalidParameters(format!(
                "Unknown command '{}'",
                command
            ))),
        }
    }

    async fn text_editor_view(&self, path: &PathBuf) -> Result<Vec<Content>, ToolError> {
        if path.is_file() {
            // Check file size first (400KB limit)
            const MAX_FILE_SIZE: u64 = 400 * 1024; // 400KB in bytes
            const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB

            let file_size = std::fs::metadata(path)
                .map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to get file metadata: {}", e))
                })?
                .len();

            if file_size > MAX_FILE_SIZE {
                return Err(ToolError::ExecutionError(format!(
                    "File '{}' is too large ({:.2}KB). Maximum size is 400KB to prevent memory issues.",
                    path.display(),
                    file_size as f64 / 1024.0
                )));
            }

            let uri = Url::from_file_path(path)
                .map_err(|_| ToolError::ExecutionError("Invalid file path".into()))?
                .to_string();

            let content = std::fs::read_to_string(path)
                .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

            let char_count = content.chars().count();
            if char_count > MAX_CHAR_COUNT {
                return Err(ToolError::ExecutionError(format!(
                    "File '{}' has too many characters ({}). Maximum character count is {}.",
                    path.display(),
                    char_count,
                    MAX_CHAR_COUNT
                )));
            }

            let language = lang::get_language_identifier(path);
            let formatted = formatdoc! {"
                ### {path}
                ```{language}
                {content}
                ```
                ",
                path=path.display(),
                language=language,
                content=content,
            };

            // The LLM gets just a quick update as we expect the file to view in the status
            // but we send a low priority message for the human
            Ok(vec![
                Content::embedded_text(uri, content).with_audience(vec![Role::Assistant]),
                Content::text(formatted)
                    .with_audience(vec![Role::User])
                    .with_priority(0.0),
            ])
        } else {
            Err(ToolError::ExecutionError(format!(
                "The path '{}' does not exist or is not a file.",
                path.display()
            )))
        }
    }

    async fn text_editor_write(
        &self,
        path: &PathBuf,
        file_text: &str,
    ) -> Result<Vec<Content>, ToolError> {
        // Write to the file
        std::fs::write(path, file_text)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Try to detect the language from the file extension
        let language = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        // The assistant output does not show the file again because the content is already in the tool request
        // but we do show it to the user here
        Ok(vec![
            Content::text(format!("Successfully wrote to {}", path.display()))
                .with_audience(vec![Role::Assistant]),
            Content::text(formatdoc! {r#"
                ### {path}
                ```{language}
                {content}
                ```
                "#,
                path=path.display(),
                language=language,
                content=file_text,
            })
            .with_audience(vec![Role::User])
            .with_priority(0.2),
        ])
    }

    async fn text_editor_replace(
        &self,
        path: &PathBuf,
        old_str: &str,
        new_str: &str,
    ) -> Result<Vec<Content>, ToolError> {
        // Check if file exists and is active
        if !path.exists() {
            return Err(ToolError::InvalidParameters(format!(
                "File '{}' does not exist, you can write a new file with the `write` command",
                path.display()
            )));
        }

        // Read content
        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

        // Ensure 'old_str' appears exactly once
        if content.matches(old_str).count() > 1 {
            return Err(ToolError::InvalidParameters(
                "'old_str' must appear exactly once in the file, but it appears multiple times"
                    .into(),
            ));
        }
        if content.matches(old_str).count() == 0 {
            return Err(ToolError::InvalidParameters(
                "'old_str' must appear exactly once in the file, but it does not appear in the file. Make sure the string exactly matches existing file content, including whitespace!".into(),
            ));
        }

        // Save history for undo
        self.save_file_history(path)?;

        // Replace and write back
        let new_content = content.replace(old_str, new_str);
        std::fs::write(path, &new_content)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Try to detect the language from the file extension
        let language = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        // Show a snippet of the changed content with context
        const SNIPPET_LINES: usize = 4;

        // Count newlines before the replacement to find the line number
        let replacement_line = content
            .split(old_str)
            .next()
            .expect("should split on already matched content")
            .matches('\n')
            .count();

        // Calculate start and end lines for the snippet
        let start_line = replacement_line.saturating_sub(SNIPPET_LINES);
        let end_line = replacement_line + SNIPPET_LINES + new_str.matches('\n').count();

        // Get the relevant lines for our snippet
        let lines: Vec<&str> = new_content.lines().collect();
        let snippet = lines
            .iter()
            .skip(start_line)
            .take(end_line - start_line + 1)
            .cloned()
            .collect::<Vec<&str>>()
            .join("\n");

        let output = formatdoc! {r#"
            ```{language}
            {snippet}
            ```
            "#,
            language=language,
            snippet=snippet
        };

        let success_message = formatdoc! {r#"
            The file {} has been edited, and the section now reads:
            {}
            Review the changes above for errors. Undo and edit the file again if necessary!
            "#,
            path.display(),
            output
        };

        Ok(vec![
            Content::text(success_message).with_audience(vec![Role::Assistant]),
            Content::text(output)
                .with_audience(vec![Role::User])
                .with_priority(0.2),
        ])
    }

    async fn text_editor_undo(&self, path: &PathBuf) -> Result<Vec<Content>, ToolError> {
        let mut history = self.file_history.lock().unwrap();
        if let Some(contents) = history.get_mut(path) {
            if let Some(previous_content) = contents.pop() {
                // Write previous content back to file
                std::fs::write(path, previous_content).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to write file: {}", e))
                })?;
                Ok(vec![Content::text("Undid the last edit")])
            } else {
                Err(ToolError::InvalidParameters(
                    "No edit history available to undo".into(),
                ))
            }
        } else {
            Err(ToolError::InvalidParameters(
                "No edit history available to undo".into(),
            ))
        }
    }

    fn save_file_history(&self, path: &PathBuf) -> Result<(), ToolError> {
        let mut history = self.file_history.lock().unwrap();
        let content = if path.exists() {
            std::fs::read_to_string(path)
                .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?
        } else {
            String::new()
        };
        history.entry(path.clone()).or_default().push(content);
        Ok(())
    }

    async fn list_windows(&self, _params: Value) -> Result<Vec<Content>, ToolError> {
        let windows = Window::all()
            .map_err(|_| ToolError::ExecutionError("Failed to list windows".into()))?;

        let window_titles: Vec<String> =
            windows.into_iter().map(|w| w.title().to_string()).collect();

        Ok(vec![
            Content::text(format!("Available windows:\n{}", window_titles.join("\n")))
                .with_audience(vec![Role::Assistant]),
            Content::text(format!("Available windows:\n{}", window_titles.join("\n")))
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    }

    async fn screen_capture(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let mut image = if let Some(window_title) =
            params.get("window_title").and_then(|v| v.as_str())
        {
            // Try to find and capture the specified window
            let windows = Window::all()
                .map_err(|_| ToolError::ExecutionError("Failed to list windows".into()))?;

            let window = windows
                .into_iter()
                .find(|w| w.title() == window_title)
                .ok_or_else(|| {
                    ToolError::ExecutionError(format!(
                        "No window found with title '{}'",
                        window_title
                    ))
                })?;

            window.capture_image().map_err(|e| {
                ToolError::ExecutionError(format!(
                    "Failed to capture window '{}': {}",
                    window_title, e
                ))
            })?
        } else {
            // Default to display capture if no window title is specified
            let display = params.get("display").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let monitors = Monitor::all()
                .map_err(|_| ToolError::ExecutionError("Failed to access monitors".into()))?;
            let monitor = monitors.get(display).ok_or_else(|| {
                ToolError::ExecutionError(format!(
                    "{} was not an available monitor, {} found.",
                    display,
                    monitors.len()
                ))
            })?;

            monitor.capture_image().map_err(|e| {
                ToolError::ExecutionError(format!("Failed to capture display {}: {}", display, e))
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
            )
        };

        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| {
                ToolError::ExecutionError(format!("Failed to write image buffer {}", e))
            })?;

        // Convert to base64
        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(vec![
            Content::text("Screenshot captured").with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png").with_priority(0.0),
        ])
    }
}

impl Router for DeveloperRouter {
    fn name(&self) -> String {
        "developer".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new().with_tools(false).build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "shell" => this.bash(arguments).await,
                "text_editor" => this.text_editor(arguments).await,
                "list_windows" => this.list_windows(arguments).await,
                "screen_capture" => this.screen_capture(arguments).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    // TODO see if we can make it easy to skip implementing these
    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async move { Ok("".to_string()) })
    }
}

impl Clone for DeveloperRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            file_history: Arc::clone(&self.file_history),
            instructions: self.instructions.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;
    use tokio::sync::OnceCell;

    #[test]
    #[serial]
    fn test_goosehints_when_present() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(".goosehints", "Test hint content").unwrap();
        let router = DeveloperRouter::new();
        let instructions = router.instructions();

        assert!(instructions.contains("Test hint content"));
    }

    #[test]
    #[serial]
    fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let router = DeveloperRouter::new();
        let instructions = router.instructions();

        assert!(!instructions.contains("Project Hints"));
    }

    static DEV_ROUTER: OnceCell<DeveloperRouter> = OnceCell::const_new();

    async fn get_router() -> &'static DeveloperRouter {
        DEV_ROUTER
            .get_or_init(|| async { DeveloperRouter::new() })
            .await
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_missing_parameters() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let router = get_router().await;
        let result = router.call_tool("shell", json!({})).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_size_limits() {
        // Create temp directory first so it stays in scope for the whole test
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Get router after setting current directory
        let router = get_router().await;

        // Test file size limit
        {
            let large_file_path = temp_dir.path().join("large.txt");
            let large_file_str = large_file_path.to_str().unwrap();

            // Create a file larger than 2MB
            let content = "x".repeat(3 * 1024 * 1024); // 3MB
            std::fs::write(&large_file_path, content).unwrap();

            let result = router
                .call_tool(
                    "text_editor",
                    json!({
                        "command": "view",
                        "path": large_file_str
                    }),
                )
                .await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert!(matches!(err, ToolError::ExecutionError(_)));
            assert!(err.to_string().contains("too large"));
        }

        // Test character count limit
        {
            let many_chars_path = temp_dir.path().join("many_chars.txt");
            let many_chars_str = many_chars_path.to_str().unwrap();

            // Create a file with more than 400K characters but less than 400KB
            let content = "x".repeat(405_000);
            std::fs::write(&many_chars_path, content).unwrap();

            let result = router
                .call_tool(
                    "text_editor",
                    json!({
                        "command": "view",
                        "path": many_chars_str
                    }),
                )
                .await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert!(matches!(err, ToolError::ExecutionError(_)));
            assert!(err.to_string().contains("too many characters"));
        }

        // Let temp_dir drop naturally at end of scope
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_write_and_view_file() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a new file
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": "Hello, world!"
                }),
            )
            .await
            .unwrap();

        // View the file
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
            )
            .await
            .unwrap();

        assert!(!view_result.is_empty());
        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("Hello, world!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_str_replace() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a new file
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": "Hello, world!"
                }),
            )
            .await
            .unwrap();

        // Replace string
        let replace_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "str_replace",
                    "path": file_path_str,
                    "old_str": "world",
                    "new_str": "Rust"
                }),
            )
            .await
            .unwrap();

        let text = replace_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.contains("has been edited, and the section now reads"));

        // View the file to verify the change
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
            )
            .await
            .unwrap();

        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("Hello, Rust!"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_undo_edit() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a new file
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": "First line"
                }),
            )
            .await
            .unwrap();

        // Replace string
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "str_replace",
                    "path": file_path_str,
                    "old_str": "First line",
                    "new_str": "Second line"
                }),
            )
            .await
            .unwrap();

        // Undo the edit
        let undo_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "undo_edit",
                    "path": file_path_str
                }),
            )
            .await
            .unwrap();

        let text = undo_result.first().unwrap().as_text().unwrap();
        assert!(text.contains("Undid the last edit"));

        // View the file to verify the undo
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
            )
            .await
            .unwrap();

        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.contains("First line"));

        temp_dir.close().unwrap();
    }
}
