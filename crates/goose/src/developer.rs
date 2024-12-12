mod lang;

use crate::systems::Resource;
use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use base64::Engine;
use indoc::{formatdoc, indoc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use tokio::process::Command;
use url::Url;
use xcap::{Monitor, Window};

use crate::errors::{AgentError, AgentResult};
use crate::models::content::Content;
use crate::models::role::Role;
use crate::models::tool::{Tool, ToolCall};
use crate::systems::System;

pub struct DeveloperSystem {
    tools: Vec<Tool>,
    cwd: Mutex<PathBuf>,
    active_resources: Mutex<HashMap<String, Resource>>, // Use URI string as key instead of PathBuf
    file_history: Mutex<HashMap<PathBuf, Vec<String>>>,
    instructions: String,
}

impl Default for DeveloperSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl DeveloperSystem {
    // Reads a resource from a URI and returns its content.
    // The resource must already exist in active_resources.
    pub async fn read_resource(&self, uri: &str) -> AgentResult<String> {
        let url = Url::parse(uri)
            .map_err(|e| AgentError::InvalidParameters(format!("Invalid URI: {}", e)))?;

        // For all URIs, verify the resource exists in active_resources first
        let active_resources = self.active_resources.lock().unwrap();
        let resource = active_resources.get(uri).ok_or_else(|| {
            // For file URIs, we want to treat unregistered files as an execution error
            if uri.starts_with("file://") {
                AgentError::ExecutionError(format!(
                    "Resource {} must be registered before reading",
                    uri
                ))
            } else {
                AgentError::InvalidParameters(format!("Resource {} could not be found", uri))
            }
        })?;

        // Load the content based on URI scheme and mime type
        let content = match url.scheme() {
            "file" => {
                let path = url.to_file_path().map_err(|_| {
                    AgentError::InvalidParameters("Invalid file path in URI".into())
                })?;

                if !path.exists() {
                    return Err(AgentError::ExecutionError(format!(
                        "File does not exist: {}",
                        path.display()
                    )));
                }

                match resource.mime_type.as_str() {
                    "text" => {
                        // For text mime type, read as string
                        std::fs::read_to_string(&path).map_err(|e| {
                            AgentError::ExecutionError(format!("Failed to read file: {}", e))
                        })?
                    }
                    "blob" => {
                        // For blob mime type, read as bytes and base64 encode
                        let bytes = std::fs::read(&path).map_err(|e| {
                            AgentError::ExecutionError(format!("Failed to read file: {}", e))
                        })?;
                        base64::prelude::BASE64_STANDARD.encode(bytes)
                    }
                    mime_type => {
                        return Err(AgentError::InvalidParameters(format!(
                            "Unsupported mime type: {}",
                            mime_type
                        )))
                    }
                }
            }
            "str" => {
                // For str:// URIs, only text mime type is supported
                if resource.mime_type != "text" {
                    return Err(AgentError::InvalidParameters(format!(
                        "str:// URI only supports text mime type, got {}",
                        resource.mime_type
                    )));
                }

                // Extract content after "str:///" prefix and URL decode it
                let content = url.path().trim_start_matches('/');
                urlencoding::decode(content)
                    .map_err(|e| {
                        AgentError::ExecutionError(format!(
                            "Failed to decode str:// content: {}",
                            e
                        ))
                    })?
                    .into_owned()
            }
            scheme => {
                return Err(AgentError::InvalidParameters(format!(
                    "Unsupported URI scheme: {}",
                    scheme
                )))
            }
        };

        Ok(content)
    }

    pub fn new() -> Self {
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

        let bash_tool = Tool::new(
            "bash",
            indoc! {r#"
                Run a bash command in the shell in the current working directory
                  - You can use multiline commands or && to execute multiple in one pass
                  - Directory changes **are not** persisted from one command to the next
                  - Sourcing files **is not** persisted from one command to the next

                For example, you can use this style to execute python in a virtualenv
                "source .venv/bin/active && python example1.py"

                but need to repeat the source for subsequent commands in that virtualenv
                "source .venv/bin/active && python example2.py"
            "#},
            json!({
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {
                        "type": "string",
                        "default": null,
                        "description": "The bash shell command to run."
                    },
                }
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

        let text_editor_tool = Tool::new(
            "text_editor",
            indoc! {r#"
                Perform text editing operations on files.

                The `command` parameter specifies the operation to perform. Allowed options are:
                - `view`: View the content of a file.
                - `write`: Write a file with the given content (create a new file or overwrite an existing).
                - `str_replace`: Replace a string in a file with a new string.
                - `undo_edit`: Undo the last edit made to a file.
            "#},
            json!({
                "type": "object",
                "required": ["command", "path"],
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file. Can be absolute or relative to the system CWD"
                    },
                    "command": {
                        "enum": ["view", "write", "str_replace", "undo_edit"],
                        "description": "The commands to run."
                    },
                    "new_str": {
                        "type": "string",
                        "default": null,
                        "description": "Required for the `replace` command."
                    },
                    "old_str": {
                        "type": "string",
                        "default": null,
                        "description": "Required for the `replace` command."
                    },
                    "file_text": {
                        "type": "string",
                        "default": null,
                        "description": "Required for `create` command."
                    },
                }
            }),
        );

        let instructions = formatdoc! {r#"
            The developer system is loaded in the directory listed below.
            You can use the shell tool to run any command that would work on the relevant operating system.
            Use the shell tool as needed to locate files or interact with the project. Only files
            that have been read or modified using the edit tools will show up in the active files list.

            bash
              - Prefer ripgrep - `rg` - when you need to locate content, it will respected ignored files for
            efficiency. **Avoid find and ls -r**
                - to locate files by name: `rg --files | rg example.py`
                - to locate consent inside files: `rg 'class Example'`
              - The operating system for these commands is {os}


            text_edit
              - Always use 'view' command first before any edit operations
              - File edits are tracked and can be undone with 'undo'
              - String replacements must match exactly once in the file
              - Line numbers start at 1 for insert operations

            The write mode will do a full overwrite of the existing file, while the str_replace mode will edit it
            using a find and replace. Choose the mode which will make the edit as simple as possible to execute.
            "#,
            os=std::env::consts::OS,
        };
        Self {
            tools: vec![
                bash_tool,
                text_editor_tool,
                screen_capture_tool,
                list_windows_tool,
            ],
            cwd: Mutex::new(std::env::current_dir().unwrap()),
            active_resources: {
                let mut resources = HashMap::new();
                let cwd = std::env::current_dir().unwrap();
                let uri: Option<String> = Some(format!("str:///{}", cwd.display()));
                resources.insert(
                    uri.clone().unwrap(),
                    Resource::new(
                        uri.unwrap(),
                        Some("text".to_string()),
                        Some("cwd".to_string()),
                    )
                    .unwrap()
                    .with_priority(1000), // Set highest priority
                );
                Mutex::new(resources)
            },
            file_history: Mutex::new(HashMap::new()),
            instructions,
        }
    }

    // Helper method to resolve a path relative to cwd
    fn resolve_path(&self, path_str: &str) -> AgentResult<PathBuf> {
        let cwd = self.cwd.lock().unwrap();
        let expanded = shellexpand::tilde(path_str);
        let path = Path::new(expanded.as_ref());
        let resolved_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };

        Ok(resolved_path)
    }

    // Implement bash tool functionality
    async fn bash(&self, params: Value) -> AgentResult<Vec<Content>> {
        let command =
            params
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or(AgentError::InvalidParameters(
                    "The command string is required".into(),
                ))?;

        // Disallow commands that should use other tools
        if command.trim_start().starts_with("cat") {
            return Err(AgentError::InvalidParameters(
                "Do not use `cat` to read files, use the view mode on the text editor tool"
                    .to_string(),
            ));
        }
        // TODO consider enforcing ripgrep over find?

        // Redirect stderr to stdout to interleave outputs
        let cmd_with_redirect = format!("{} 2>&1", command);

        // Execute the command
        let child = Command::new("bash")
            .stdout(Stdio::piped()) // These two pipes required to capture output later.
            .stderr(Stdio::piped())
            .kill_on_drop(true) // Critical so that the command is killed when the agent.reply stream is interrupted.
            .arg("-c")
            .arg(cmd_with_redirect)
            .spawn()
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Store the process ID with the command as the key
        let pid: Option<u32> = child.id();
        if let Some(pid) = pid {
            crate::process_store::store_process(pid);
        }

        // Wait for the command to complete and get output
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Remove the process ID from the store
        if let Some(pid) = pid {
            crate::process_store::remove_process(pid);
        }

        let output_str = format!(
            "Finished with Status Code: {}\nOutput:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
        Ok(vec![
            Content::text(output_str.clone()).with_audience(vec![Role::Assistant]),
            Content::text(output_str)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    }

    // Implement text_editor tool functionality
    async fn text_editor(&self, params: Value) -> AgentResult<Vec<Content>> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'command' parameter".into()))?;

        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'path' parameter".into()))?;

        let path = self.resolve_path(path_str)?;

        match command {
            "view" => self.text_editor_view(&path).await,
            "write" => {
                let file_text = params
                    .get("file_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters("Missing 'file_text' parameter".into())
                    })?;

                self.text_editor_write(&path, file_text).await
            }
            "str_replace" => {
                let old_str = params
                    .get("old_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters("Missing 'old_str' parameter".into())
                    })?;
                let new_str = params
                    .get("new_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters("Missing 'new_str' parameter".into())
                    })?;

                self.text_editor_replace(&path, old_str, new_str).await
            }
            "undo_edit" => self.text_editor_undo(&path).await,
            _ => Err(AgentError::InvalidParameters(format!(
                "Unknown command '{}'",
                command
            ))),
        }
    }

    async fn text_editor_view(&self, path: &PathBuf) -> AgentResult<Vec<Content>> {
        if path.is_file() {
            // Check file size first (2MB limit)
            const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024; // 2MB in bytes
            const MAX_CHAR_COUNT: usize = 1 << 20; // 2^20 characters (1,048,576)
            
            let file_size = std::fs::metadata(path)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to get file metadata: {}", e)))?
                .len();
            
            if file_size > MAX_FILE_SIZE {
                return Err(AgentError::ExecutionError(format!(
                    "File '{}' is too large ({:.2}MB). Maximum size is 2MB to prevent memory issues.",
                    path.display(),
                    file_size as f64 / 1024.0 / 1024.0
                )));
            }
            
            // Create a new resource and add it to active_resources
            let uri = Url::from_file_path(path)
                .map_err(|_| AgentError::ExecutionError("Invalid file path".into()))?
                .to_string();

            // Read the content once
            let content = std::fs::read_to_string(path)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to read file: {}", e)))?;
            
            let char_count = content.chars().count();
            if char_count > MAX_CHAR_COUNT {
                return Err(AgentError::ExecutionError(format!(
                    "File '{}' has too many characters ({}). Maximum character count is {}.",
                    path.display(),
                    char_count,
                    MAX_CHAR_COUNT
                )));
            }

            // Create and store the resource
            let resource =
                Resource::new(uri.clone(), Some("text".to_string()), None).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to create resource: {}", e))
                })?;

            self.active_resources.lock().unwrap().insert(uri, resource);

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
                Content::text(format!(
                    "The file content for {} is now available in the system status.",
                    path.display()
                ))
                .with_audience(vec![Role::Assistant]),
                Content::text(formatted)
                    .with_audience(vec![Role::User])
                    .with_priority(0.0),
            ])
        } else {
            Err(AgentError::ExecutionError(format!(
                "The path '{}' does not exist or is not a file.",
                path.display()
            )))
        }
    }

    async fn text_editor_write(
        &self,
        path: &PathBuf,
        file_text: &str,
    ) -> AgentResult<Vec<Content>> {
        // Get the URI for the file
        let uri = Url::from_file_path(path)
            .map_err(|_| AgentError::ExecutionError("Invalid file path".into()))?
            .to_string();

        // Check if file already exists and is active
        if path.exists() && !self.active_resources.lock().unwrap().contains_key(&uri) {
            return Err(AgentError::InvalidParameters(format!(
                "File '{}' exists but is not active. View it first before overwriting.",
                path.display()
            )));
        }

        // Save history for undo
        self.save_file_history(path)?;

        // Write to the file
        std::fs::write(path, file_text)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Create and store resource

        let resource = Resource::new(uri.clone(), Some("text".to_string()), None)
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
        self.active_resources.lock().unwrap().insert(uri, resource);

        // Try to detect the language from the file extension
        let language = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

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
    ) -> AgentResult<Vec<Content>> {
        // Get the URI for the file
        let uri = Url::from_file_path(path)
            .map_err(|_| AgentError::ExecutionError("Invalid file path".into()))?
            .to_string();

        // Check if file exists and is active
        if !path.exists() {
            return Err(AgentError::InvalidParameters(format!(
                "File '{}' does not exist",
                path.display()
            )));
        }
        if !self.active_resources.lock().unwrap().contains_key(&uri) {
            return Err(AgentError::InvalidParameters(format!(
                "You must view '{}' before editing it",
                path.display()
            )));
        }

        // Read content
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to read file: {}", e)))?;

        // Ensure 'old_str' appears exactly once
        if content.matches(old_str).count() > 1 {
            return Err(AgentError::InvalidParameters(
                "'old_str' must appear exactly once in the file, but it appears multiple times"
                    .into(),
            ));
        }
        if content.matches(old_str).count() == 0 {
            return Err(AgentError::InvalidParameters(
                "'old_str' must appear exactly once in the file, but it does not appear in the file. Make sure the string exactly matches existing file content, including spacing.".into(),
            ));
        }

        // Save history for undo
        self.save_file_history(path)?;

        // Replace and write back
        let new_content = content.replace(old_str, new_str);
        std::fs::write(path, &new_content)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Update resource
        if let Some(resource) = self.active_resources.lock().unwrap().get_mut(&uri) {
            resource.update_timestamp();
        }

        // Try to detect the language from the file extension
        let language = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        Ok(vec![
            Content::text("Successfully replaced text").with_audience(vec![Role::Assistant]),
            Content::text(formatdoc! {r#"
                ### {path}

                *Before*:
                ```{language}
                {old_str}
                ```

                *After*:
                ```{language}
                {new_str}
                ```
                "#,
                path=path.display(),
                language=language,
                old_str=old_str,
                new_str=new_str,
            })
            .with_audience(vec![Role::User])
            .with_priority(0.2),
        ])
    }

    async fn text_editor_undo(&self, path: &PathBuf) -> AgentResult<Vec<Content>> {
        let mut history = self.file_history.lock().unwrap();
        if let Some(contents) = history.get_mut(path) {
            if let Some(previous_content) = contents.pop() {
                // Write previous content back to file
                std::fs::write(path, previous_content).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to write file: {}", e))
                })?;
                Ok(vec![Content::text("Undid the last edit")])
            } else {
                Err(AgentError::InvalidParameters(
                    "No edit history available to undo".into(),
                ))
            }
        } else {
            Err(AgentError::InvalidParameters(
                "No edit history available to undo".into(),
            ))
        }
    }

    fn save_file_history(&self, path: &PathBuf) -> AgentResult<()> {
        let mut history = self.file_history.lock().unwrap();
        let content = if path.exists() {
            std::fs::read_to_string(path)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to read file: {}", e)))?
        } else {
            String::new()
        };
        history.entry(path.clone()).or_default().push(content);
        Ok(())
    }

    // Implement screen capture functionality
    async fn list_windows(&self, _params: Value) -> AgentResult<Vec<Content>> {
        let windows = Window::all()
            .map_err(|_| AgentError::ExecutionError("Failed to list windows".into()))?;

        let window_titles: Vec<String> =
            windows.into_iter().map(|w| w.title().to_string()).collect();

        Ok(vec![Content::text(format!(
            "Available windows:\n{}",
            window_titles.join("\n")
        ))
        .with_audience(vec![Role::Assistant])
        .with_priority(0.0)])
    }

    async fn screen_capture(&self, params: Value) -> AgentResult<Vec<Content>> {
        let mut image = if let Some(window_title) =
            params.get("window_title").and_then(|v| v.as_str())
        {
            // Try to find and capture the specified window
            let windows = Window::all()
                .map_err(|_| AgentError::ExecutionError("Failed to list windows".into()))?;

            let window = windows
                .into_iter()
                .find(|w| w.title() == window_title)
                .ok_or_else(|| {
                    AgentError::ExecutionError(format!(
                        "No window found with title '{}'",
                        window_title
                    ))
                })?;

            window.capture_image().map_err(|e| {
                AgentError::ExecutionError(format!(
                    "Failed to capture window '{}': {}",
                    window_title, e
                ))
            })?
        } else {
            // Default to display capture if no window title is specified
            let display = params.get("display").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let monitors = Monitor::all()
                .map_err(|_| AgentError::ExecutionError("Failed to access monitors".into()))?;
            let monitor = monitors.get(display).ok_or_else(|| {
                AgentError::ExecutionError(format!(
                    "{} was not an available monitor, {} found.",
                    display,
                    monitors.len()
                ))
            })?;

            monitor.capture_image().map_err(|e| {
                AgentError::ExecutionError(format!("Failed to capture display {}: {}", display, e))
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
                AgentError::ExecutionError(format!("Failed to write image buffer {}", e))
            })?;

        // Convert to base64
        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(vec![Content::image(data, "image/png")])
    }
}

#[async_trait]
impl System for DeveloperSystem {
    fn name(&self) -> &str {
        "DeveloperSystem"
    }

    fn description(&self) -> &str {
        "Use the developer system to build software and solve problems by editing files and
running commands on the shell."
    }

    fn instructions(&self) -> &str {
        self.instructions.as_str()
    }

    fn tools(&self) -> &[Tool] {
        &self.tools
    }

    async fn status(&self) -> AnyhowResult<Vec<Resource>> {
        let mut active_resources = self.active_resources.lock().unwrap();

        // Update resources and remove any that are no longer valid
        active_resources.retain(|uri, _| {
            if let Ok(url) = Url::parse(uri) {
                match url.scheme() {
                    "file" => {
                        // For file URIs, check if file exists
                        url.to_file_path()
                            .map(|path| path.exists())
                            .unwrap_or(false)
                    }
                    "str" => true, // str:// URIs are always valid
                    _ => false,    // Other schemes not yet supported
                }
            } else {
                false
            }
        });

        // Convert active resources to a Vec<Resource>
        let resources: Vec<Resource> = active_resources.values().cloned().collect();

        Ok(resources)
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        match tool_call.name.as_str() {
            "bash" => self.bash(tool_call.arguments).await,
            "text_editor" => self.text_editor(tool_call.arguments).await,
            "screen_capture" => self.screen_capture(tool_call.arguments).await,
            "list_windows" => self.list_windows(tool_call.arguments).await,
            _ => Err(AgentError::ToolNotFound(tool_call.name)),
        }
    }

    async fn read_resource(&self, uri: &str) -> AgentResult<String> {
        let content = self.read_resource(uri).await?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::sync::OnceCell;

    // Use OnceCell to initialize the system once for all tests
    static DEV_SYSTEM: OnceCell<DeveloperSystem> = OnceCell::const_new();

    async fn get_system() -> &'static DeveloperSystem {
        DEV_SYSTEM
            .get_or_init(|| async { DeveloperSystem::new() })
            .await
    }

    #[tokio::test]
    async fn test_bash_missing_parameters() {
        let system = get_system().await;

        let tool_call = ToolCall::new("bash", json!({"working_dir": "."}));
        let error = system.call(tool_call).await.unwrap_err();
        assert!(matches!(error, AgentError::InvalidParameters(_)));
    }

    #[tokio::test]
    async fn test_bash_change_directory() {
        let system = get_system().await;

        let tool_call = ToolCall::new("bash", json!({ "working_dir": ".", "command": "pwd" }));
        let result = system.call(tool_call).await.unwrap();
        assert!(result[0]
            .as_text()
            .unwrap()
            .contains(&std::env::current_dir().unwrap().display().to_string()));
    }

    #[tokio::test]
    async fn test_bash_invalid_directory() {
        let system = get_system().await;

        let tool_call = ToolCall::new("bash", json!({ "working_dir": "non_existent_dir" }));
        let error = system.call(tool_call).await.unwrap_err();
        assert!(matches!(error, AgentError::InvalidParameters(_)));
    }

    #[tokio::test]
    async fn test_text_editor_size_limits() {
        let system = get_system().await;
        let temp_dir = tempfile::tempdir().unwrap();

        // Test file size limit
        {
            let large_file_path = temp_dir.path().join("large.txt");
            let large_file_str = large_file_path.to_str().unwrap();
            
            // Create a file larger than 2MB
            let content = "x".repeat(3 * 1024 * 1024); // 3MB
            std::fs::write(&large_file_path, content).unwrap();

            let view_call = ToolCall::new(
                "text_editor",
                json!({
                    "command": "view",
                    "path": large_file_str
                }),
            );
            let error = system.call(view_call).await.unwrap_err();
            assert!(matches!(error, AgentError::ExecutionError(_)));
            assert!(error.to_string().contains("too large"));
            assert!(error.to_string().contains("Maximum size is 2MB"));
        }

        // Test character count limit
        {
            let many_chars_path = temp_dir.path().join("many_chars.txt");
            let many_chars_str = many_chars_path.to_str().unwrap();
            
            // Create a file with more than 2^20 characters but less than 2MB
            let content = "x".repeat((1 << 20) + 1); // 2^20 + 1 characters
            std::fs::write(&many_chars_path, content).unwrap();

            let view_call = ToolCall::new(
                "text_editor",
                json!({
                    "command": "view",
                    "path": many_chars_str
                }),
            );
            let error = system.call(view_call).await.unwrap_err();
            assert!(matches!(error, AgentError::ExecutionError(_)));
            assert!(error.to_string().contains("too many characters"));
            assert!(error.to_string().contains("Maximum character count is"));
        }

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_text_editor_write_and_view_file() {
        let system = get_system().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();

        // Create a new file
        let create_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello, world!"
            }),
        );
        let create_result = system.call(create_call).await.unwrap();
        assert!(create_result[0]
            .as_text()
            .unwrap()
            .contains("Successfully wrote to"));

        // View the file
        let view_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
        );
        let view_result = system.call(view_call).await.unwrap();
        assert!(view_result[0]
            .as_text()
            .unwrap()
            .contains("The file content for"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_text_editor_str_replace() {
        let system = get_system().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();

        // Create a new file
        let create_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "Hello, world!"
            }),
        );
        system.call(create_call).await.unwrap();

        // View the file to make it active
        let view_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
        );
        system.call(view_call).await.unwrap();

        // Replace string
        let replace_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "str_replace",
                "path": file_path_str,
                "old_str": "world",
                "new_str": "Rust"
            }),
        );
        let replace_result = system.call(replace_call).await.unwrap();
        assert!(replace_result[0]
            .as_text()
            .unwrap()
            .contains("Successfully replaced text"));

        // View the file again
        let view_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
        );
        let view_result = system.call(view_call).await.unwrap();
        assert!(view_result[0]
            .as_text()
            .unwrap()
            .contains("The file content for"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_read_resource() {
        let system = get_system().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_content = "Hello, world!";
        std::fs::write(&file_path, test_content).unwrap();

        let uri = Url::from_file_path(&file_path).unwrap().to_string();

        // Test text mime type with file:// URI
        {
            let mut active_resources = system.active_resources.lock().unwrap();
            let resource = Resource::new(uri.clone(), Some("text".to_string()), None).unwrap();
            active_resources.insert(uri.clone(), resource);
        }
        let content = system.read_resource(&uri).await.unwrap();
        assert_eq!(content, test_content);

        // Test blob mime type with file:// URI
        let blob_path = temp_dir.path().join("test.bin");
        let blob_content = b"Binary content";
        std::fs::write(&blob_path, blob_content).unwrap();
        let blob_uri = Url::from_file_path(&blob_path).unwrap().to_string();
        {
            let mut active_resources = system.active_resources.lock().unwrap();
            let resource = Resource::new(blob_uri.clone(), Some("blob".to_string()), None).unwrap();
            active_resources.insert(blob_uri.clone(), resource);
        }
        let encoded_content = system.read_resource(&blob_uri).await.unwrap();
        assert_eq!(
            base64::prelude::BASE64_STANDARD
                .decode(encoded_content)
                .unwrap(),
            blob_content
        );

        // Test str:// URI with text mime type
        let str_uri = format!("str:///{}", test_content);
        {
            let mut active_resources = system.active_resources.lock().unwrap();
            let resource = Resource::new(str_uri.clone(), Some("text".to_string()), None).unwrap();
            active_resources.insert(str_uri.clone(), resource);
        }
        let str_content = system.read_resource(&str_uri).await.unwrap();
        assert_eq!(str_content, test_content);

        // Test str:// URI with blob mime type (should fail)
        let str_blob_uri = format!("str:///{}", test_content);
        {
            let mut active_resources = system.active_resources.lock().unwrap();
            let resource =
                Resource::new(str_blob_uri.clone(), Some("blob".to_string()), None).unwrap();
            active_resources.insert(str_blob_uri.clone(), resource);
        }
        let error = system.read_resource(&str_blob_uri).await.unwrap_err();
        assert!(matches!(error, AgentError::InvalidParameters(_)));
        assert!(error.to_string().contains("only supports text mime type"));

        // Test invalid URI
        let error = system.read_resource("invalid://uri").await.unwrap_err();
        assert!(matches!(error, AgentError::InvalidParameters(_)));

        // Test file:// URI without registration
        let non_registered = Url::from_file_path(temp_dir.path().join("not_registered.txt"))
            .unwrap()
            .to_string();
        let error = system.read_resource(&non_registered).await.unwrap_err();
        assert!(matches!(error, AgentError::ExecutionError(_)));
        assert!(error
            .to_string()
            .contains("must be registered before reading"));

        // Test file:// URI with non-existent file but registered
        let non_existent = Url::from_file_path(temp_dir.path().join("non_existent.txt"))
            .unwrap()
            .to_string();
        {
            let mut active_resources = system.active_resources.lock().unwrap();
            let resource =
                Resource::new(non_existent.clone(), Some("text".to_string()), None).unwrap();
            active_resources.insert(non_existent.clone(), resource);
        }
        let result = system.read_resource(&non_existent).await;
        let error = result.unwrap_err();
        assert!(matches!(error, AgentError::ExecutionError(_)));
        assert!(error.to_string().contains("does not exist"));

        // Test invalid mime type
        let invalid_mime = Url::from_file_path(&file_path).unwrap().to_string();
        {
            let mut active_resources = system.active_resources.lock().unwrap();
            // Create with text mime type but modify it to be invalid
            let mut resource =
                Resource::new(invalid_mime.clone(), Some("text".to_string()), None).unwrap();
            resource.mime_type = "invalid".to_string();
            active_resources.insert(invalid_mime.clone(), resource);
        }
        let error = system.read_resource(&invalid_mime).await.unwrap_err();
        assert!(matches!(error, AgentError::InvalidParameters(_)));
        assert!(error.to_string().contains("Unsupported mime type"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_text_editor_undo_edit() {
        let system = get_system().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();

        // Create a new file
        let create_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "write",
                "path": file_path_str,
                "file_text": "First line"
            }),
        );
        system.call(create_call).await.unwrap();

        // View the file to make it active
        let view_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "view",
                "path": file_path_str
            }),
        );
        system.call(view_call).await.unwrap();

        // replace an entry
        let insert_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "str_replace",
                "path": file_path_str,
                "old_str": "First line",
                "new_str": "Second line"
            }),
        );
        system.call(insert_call).await.unwrap();

        // Undo the edit
        let undo_call = ToolCall::new(
            "text_editor",
            json!({
                "command": "undo_edit",
                "path": file_path_str
            }),
        );
        let undo_result = system.call(undo_call).await.unwrap();
        assert!(undo_result[0]
            .as_text()
            .unwrap()
            .contains("Undid the last edit"));

        // View the file again
        let view_result = system
            .call(ToolCall::new(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
            ))
            .await
            .unwrap();
        assert!(view_result[0]
            .as_text()
            .unwrap()
            .contains("The file content for"));

        temp_dir.close().unwrap();
    }
}
