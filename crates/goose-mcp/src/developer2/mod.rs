mod lang;

use anyhow::Result;
use indoc::formatdoc;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    future::Future,
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

pub struct Developer2Router {
    tools: Vec<Tool>,
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    instructions: String,
}

impl Default for Developer2Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Developer2Router {
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
                - `create`: Create a new file with the given content (it will fail if the file already exists)
                - `str_replace`: Replace a string in a file with a new string.
                - `undo_edit`: Undo the last edit made to a file.

                To use the create command, you must specify `file_text` which will become the content of the new file.

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
                        "enum": ["view", "create", "str_replace", "undo_edit"],
                        "description": "Allowed options are: `view`, `create`, `str_replace`, undo_edit`."
                    },
                    "old_str": {"type": "string"},
                    "new_str": {"type": "string"},
                    "file_text": {"type": "string"}
                }
            }),
        );

        let instructions = formatdoc! {r#"
            The developer system gives you the capabilities to edit code files and run shell commands,
            and can be used to solve a wide range of problems.

            You can use the shell tool to run any command that would work on the relevant operating system.
            Use the shell tool as needed to locate files or interact with the project.

            operating system: {os}
            current directory: {cwd}

            "#,
            os=std::env::consts::OS,
            cwd=std::env::current_dir().expect("should have a current working dir").to_string_lossy(),
        };

        Self {
            tools: vec![bash_tool, text_editor_tool],
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
            "create" => {
                let file_text = params
                    .get("file_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ToolError::InvalidParameters("Missing 'file_text' parameter".into())
                    })?;

                self.text_editor_create(&path, file_text).await
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
            // Check file size first (2MB limit)
            const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024; // 2MB in bytes
            const MAX_CHAR_COUNT: usize = 1 << 20; // 2^20 characters (1,048,576)

            let file_size = std::fs::metadata(path)
                .map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to get file metadata: {}", e))
                })?
                .len();

            if file_size > MAX_FILE_SIZE {
                return Err(ToolError::ExecutionError(format!(
                    "File '{}' is too large ({:.2}MB). Maximum size is 2MB to prevent memory issues.",
                    path.display(),
                    file_size as f64 / 1024.0 / 1024.0
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

    async fn text_editor_create(
        &self,
        path: &PathBuf,
        file_text: &str,
    ) -> Result<Vec<Content>, ToolError> {
        // Check if file already exists
        if path.exists() {
            return Err(ToolError::InvalidParameters(format!(
                "File '{}' already exists - you will need to edit it with the `str_replace` command",
                path.display()
            )));
        }

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
                "File '{}' does not exist, you can write a new file with the `create` command",
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
}

impl Router for Developer2Router {
    fn name(&self) -> String {
        "developer".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .build()
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

impl Clone for Developer2Router {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            file_history: Arc::clone(&self.file_history),
            instructions: self.instructions.clone(),
        }
    }
}
