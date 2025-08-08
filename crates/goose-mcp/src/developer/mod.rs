mod editor_models;
mod lang;
mod shell;

use anyhow::Result;
use base64::Engine;
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::formatdoc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    future::Future,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
};
use tokio_stream::{wrappers::SplitStream, StreamExt as _};
use url::Url;

use include_dir::{include_dir, Dir};
use mcp_core::{
    handler::{require_str_parameter, PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
};

use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

use rmcp::model::{
    Content, JsonRpcMessage, JsonRpcNotification, JsonRpcVersion2_0, Notification, Prompt,
    PromptArgument, Resource, Role, Tool, ToolAnnotations,
};
use rmcp::object;

use self::editor_models::{create_editor_model, EditorModel};
use self::shell::{expand_path, get_shell_config, is_absolute_path, normalize_line_endings};
use indoc::indoc;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

#[cfg(feature = "screen_capture")]
use xcap::{Monitor, Window};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub template: String,
    pub arguments: Vec<PromptArgumentTemplate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptArgumentTemplate {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

// Embeds the prompts directory to the build
static PROMPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/developer/prompts");
const LINE_READ_LIMIT: usize = 2000;

/// Loads prompt files from the embedded PROMPTS_DIR and returns a HashMap of prompts.
/// Ensures that each prompt name is unique.
pub fn load_prompt_files() -> HashMap<String, Prompt> {
    let mut prompts = HashMap::new();

    for entry in PROMPTS_DIR.files() {
        let prompt_str = String::from_utf8_lossy(entry.contents()).into_owned();

        let template: PromptTemplate = match serde_json::from_str(&prompt_str) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to parse prompt template in {}: {}",
                    entry.path().display(),
                    e
                );
                continue; // Skip invalid prompt file
            }
        };

        let arguments = template
            .arguments
            .into_iter()
            .map(|arg| PromptArgument {
                name: arg.name,
                description: arg.description,
                required: arg.required,
            })
            .collect::<Vec<PromptArgument>>();

        let prompt = Prompt::new(&template.id, Some(&template.template), Some(arguments));

        if prompts.contains_key(&prompt.name) {
            eprintln!("Duplicate prompt name '{}' found. Skipping.", prompt.name);
            continue; // Skip duplicate prompt name
        }

        prompts.insert(prompt.name.clone(), prompt);
    }

    prompts
}

pub struct DeveloperRouter {
    tools: Vec<Tool>,
    prompts: Arc<HashMap<String, Prompt>>,
    instructions: String,
    file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
    ignore_patterns: Arc<Gitignore>,
    editor_model: Option<EditorModel>,
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

        // An editor model is optionally provided, if configured, for fast edit apply
        // it will fall back to norma string replacement if not configured
        //
        // when there is an editor model, the prompts are slightly changed as it takes
        // a load off the main LLM making the tool calls and you get faster more correct applies
        let editor_model = create_editor_model();

        // Get OS-specific shell tool description
        let shell_tool_desc = match std::env::consts::OS {
            "windows" => indoc! {r#"
                Execute a command in the shell.

                This will return the output and error concatenated into a single string, as
                you would see from running on the command line. There will also be an indication
                of if the command succeeded or failed.

                Avoid commands that produce a large amount of output, and consider piping those outputs to files.

                **Important**: For searching files and code:

                Preferred: Use ripgrep (`rg`) when available - it respects .gitignore and is fast:
                  - To locate a file by name: `rg --files | rg example.py`
                  - To locate content inside files: `rg 'class Example'`

                Alternative Windows commands (if ripgrep is not installed):
                  - To locate a file by name: `dir /s /b example.py`
                  - To locate content inside files: `findstr /s /i "class Example" *.py`

                Note: Alternative commands may show ignored/hidden files that should be excluded.
            "#},
            _ => indoc! {r#"
                Execute a command in the shell.

                This will return the output and error concatenated into a single string, as
                you would see from running on the command line. There will also be an indication
                of if the command succeeded or failed.

                Avoid commands that produce a large amount of output, and consider piping those outputs to files.
                If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
                this tool does not run indefinitely.

                **Important**: Use ripgrep - `rg` - exclusively when you need to locate a file or a code reference,
                other solutions may produce too large output because of hidden files! For example *do not* use `find` or `ls -r`
                  - List files by name: `rg --files | rg <filename>`
                  - List files that contain a regex: `rg '<regex>' -l`

                **Important**: Each shell command runs in its own process. Things like directory changes or
                sourcing files do not persist between tool calls. So you may need to repeat them each time by
                stringing together commands, e.g. `cd example && ls` or `source env/bin/activate && pip install numpy`
                  - Multiple commands: Use ; or && to chain commands, avoid newlines
                  - Pathnames: Use absolute paths and avoid cd unless explicitly requested
            "#},
        };

        let bash_tool = Tool::new(
            "shell".to_string(),
            shell_tool_desc.to_string(),
            object!({
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {"type": "string"}
                }
            }),
        );

        // Create text editor tool with different descriptions based on editor API configuration
        let (text_editor_desc, str_replace_command) = if let Some(ref editor) = editor_model {
            (
                formatdoc! {r#"
                Perform text editing operations on files.

                The `command` parameter specifies the operation to perform. Allowed options are:
                - `view`: View the content of a file.
                - `write`: Create or overwrite a file with the given content
                - `edit_file`: Edit the file with the new content.
                - `insert`: Insert text at a specific line location in the file.
                - `undo_edit`: Undo the last edit made to a file.

                To use the write command, you must specify `file_text` which will become the new content of the file. Be careful with
                existing files! This is a full overwrite, so you must include everything - not just sections you are modifying.
                
                To use the insert command, you must specify both `insert_line` (the line number after which to insert, 0 for beginning) 
                and `new_str` (the text to insert).

                To use the edit_file command, you must specify both `old_str` and `new_str` 
                {}
                
            "#, editor.get_str_replace_description()},
                "edit_file",
            )
        } else {
            (indoc! {r#"
                Perform text editing operations on files.

                The `command` parameter specifies the operation to perform. Allowed options are:
                - `view`: View the content of a file.
                - `write`: Create or overwrite a file with the given content
                - `str_replace`: Replace a string in a file with a new string.
                - `insert`: Insert text at a specific line location in the file.
                - `undo_edit`: Undo the last edit made to a file.

                To use the write command, you must specify `file_text` which will become the new content of the file. Be careful with
                existing files! This is a full overwrite, so you must include everything - not just sections you are modifying.

                To use the str_replace command, you must specify both `old_str` and `new_str` - the `old_str` needs to exactly match one
                unique section of the original file, including any whitespace. Make sure to include enough context that the match is not
                ambiguous. The entire original string will be replaced with `new_str`.

                To use the insert command, you must specify both `insert_line` (the line number after which to insert, 0 for beginning) 
                and `new_str` (the text to insert).
            "#}.to_string(), "str_replace")
        };

        let text_editor_tool = Tool::new(
            "text_editor".to_string(),
            text_editor_desc.to_string(),
            object!({
                "type": "object",
                "required": ["command", "path"],
                "properties": {
                    "path": {
                        "description": "Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`.",
                        "type": "string"
                    },
                    "command": {
                        "type": "string",
                        "enum": ["view", "write", str_replace_command, "insert", "undo_edit"],
                        "description": format!("Allowed options are: `view`, `write`, `{}`, `insert`, `undo_edit`.", str_replace_command)
                    },
                    "view_range": {
                        "type": "array",
                        "items": {"type": "integer"},
                        "minItems": 2,
                        "maxItems": 2,
                        "description": "Optional array of two integers specifying the start and end line numbers to view. Line numbers are 1-indexed, and -1 for the end line means read to the end of the file. This parameter only applies when viewing files, not directories."
                    },
                    "insert_line": {
                        "type": "integer",
                        "description": "The line number after which to insert the text (0 for beginning of file). This parameter is required when using the insert command."
                    },
                    "old_str": {"type": "string"},
                    "new_str": {"type": "string"},
                    "file_text": {"type": "string"}
                }
            }),
        );

        #[cfg(feature = "screen_capture")]
        let list_windows_tool = Tool::new(
            "list_windows",
            indoc! {r#"
                List all available window titles that can be used with screen_capture.
                Returns a list of window titles that can be used with the window_title parameter
                of the screen_capture tool.
            "#},
            object!({
                "type": "object",
                "required": [],
                "properties": {}
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("List available windows".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        #[cfg(feature = "screen_capture")]
        let screen_capture_tool = Tool::new(
            "screen_capture",
            indoc! {r#"
                Capture a screenshot of a specified display or window.
                You can capture either:
                1. A full display (monitor) using the display parameter
                2. A specific window by its title using the window_title parameter

                Only one of display or window_title should be specified.
            "#},
            object!({
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
            })
        ).annotate(ToolAnnotations {
            title: Some("Capture a full screen".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        #[cfg(feature = "screen_capture")]
        let image_processor_tool = Tool::new(
            "image_processor",
            indoc! {r#"
                Process an image file from disk. The image will be:
                1. Resized if larger than max width while maintaining aspect ratio
                2. Converted to PNG format
                3. Returned as base64 encoded data

                This allows processing image files for use in the conversation.
            "#},
            object!({
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the image file to process"
                    }
                }
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Process Image".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        });

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
            .unwrap_or_else(|| vec![".goosehints".to_string()]);

        let mut global_hints_contents = Vec::with_capacity(hints_filenames.len());
        let mut local_hints_contents = Vec::with_capacity(hints_filenames.len());

        for hints_filename in &hints_filenames {
            // Global hints
            // choose_app_strategy().config_dir()
            // - macOS/Linux: ~/.config/goose/
            // - Windows:     ~\AppData\Roaming\Block\goose\config\
            // keep previous behavior of expanding ~/.config in case this fails
            let global_hints_path = choose_app_strategy(crate::APP_STRATEGY.clone())
                .map(|strategy| strategy.in_config_dir(hints_filename))
                .unwrap_or_else(|_| {
                    let path_str = format!("~/.config/goose/{}", hints_filename);
                    PathBuf::from(shellexpand::tilde(&path_str).to_string())
                });

            if let Some(parent) = global_hints_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            if global_hints_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&global_hints_path) {
                    global_hints_contents.push(content);
                }
            }

            let local_hints_path = cwd.join(hints_filename);
            if local_hints_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&local_hints_path) {
                    local_hints_contents.push(content);
                }
            }
        }

        let mut hints = String::new();
        if !global_hints_contents.is_empty() {
            hints.push_str("\n### Global Hints\nThe developer extension includes some global hints that apply to all projects & directories.\n");
            hints.push_str(&global_hints_contents.join("\n"));
        }

        if !local_hints_contents.is_empty() {
            if !hints.is_empty() {
                hints.push_str("\n\n");
            }
            hints.push_str("### Project Hints\nThe developer extension includes some hints for working on the project in this directory.\n");
            hints.push_str(&local_hints_contents.join("\n"));
        }

        // Return base instructions directly when no hints are found
        let instructions = if hints.is_empty() {
            base_instructions
        } else {
            format!("{base_instructions}\n{hints}")
        };

        let mut builder = GitignoreBuilder::new(cwd.clone());
        let mut has_ignore_file = false;
        // Initialize ignore patterns
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        let global_ignore_path = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir(".gooseignore"))
            .unwrap_or_else(|_| {
                PathBuf::from(shellexpand::tilde("~/.config/goose/.gooseignore").to_string())
            });

        // Create the directory if it doesn't exist
        let _ = std::fs::create_dir_all(global_ignore_path.parent().unwrap());

        // Read global ignores if they exist
        if global_ignore_path.is_file() {
            let _ = builder.add(global_ignore_path);
            has_ignore_file = true;
        }

        // Check for local ignores in current directory
        let local_ignore_path = cwd.join(".gooseignore");

        // Read local ignores if they exist
        if local_ignore_path.is_file() {
            let _ = builder.add(local_ignore_path);
            has_ignore_file = true;
        } else {
            // If no .gooseignore exists, check for .gitignore as fallback
            let gitignore_path = cwd.join(".gitignore");
            if gitignore_path.is_file() {
                tracing::debug!(
                    "No .gooseignore found, using .gitignore as fallback for ignore patterns"
                );
                let _ = builder.add(gitignore_path);
                has_ignore_file = true;
            }
        }

        // Only use default patterns if no .gooseignore files were found
        // AND no .gitignore was used as fallback
        if !has_ignore_file {
            // Add some sensible defaults
            let _ = builder.add_line(None, "**/.env");
            let _ = builder.add_line(None, "**/.env.*");
            let _ = builder.add_line(None, "**/secrets.*");
        }

        let ignore_patterns = builder.build().expect("Failed to build ignore patterns");

        Self {
            tools: {
                let tools = vec![bash_tool, text_editor_tool];
                #[cfg(feature = "screen_capture")]
                {
                    tools.push(list_windows_tool);
                    tools.push(screen_capture_tool);
                    tools.push(image_processor_tool);
                }
                tools
            },
            prompts: Arc::new(load_prompt_files()),
            instructions,
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: Arc::new(ignore_patterns),
            editor_model,
        }
    }

    // Helper method to check if a path should be ignored
    fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_patterns.matched(path, false).is_ignore()
    }

    // shell output can be large, this will help manage that
    fn process_shell_output(&self, output_str: &str) -> Result<(String, String), ToolError> {
        let lines: Vec<&str> = output_str.lines().collect();
        let line_count = lines.len();

        let start = lines.len().saturating_sub(100);
        let last_100_lines_str = lines[start..].join("\n");

        let final_output = if line_count > 100 {
            let tmp_file = tempfile::NamedTempFile::new().map_err(|e| {
                ToolError::ExecutionError(format!("Failed to create temporary file: {}", e))
            })?;

            std::fs::write(tmp_file.path(), output_str).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to write to temporary file: {}", e))
            })?;

            let (_, path) = tmp_file.keep().map_err(|e| {
                ToolError::ExecutionError(format!("Failed to persist temporary file: {}", e))
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
            format!("... \n{}", last_100_lines_str)
        } else {
            output_str.to_string()
        };

        Ok((final_output, user_output))
    }

    // Helper method to resolve a path relative to cwd with platform-specific handling
    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, ToolError> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = expand_path(path_str);
        let path = Path::new(&expanded);

        let suggestion = cwd.join(path);

        match is_absolute_path(&expanded) {
            true => Ok(path.to_path_buf()),
            false => Err(ToolError::InvalidParameters(format!(
                "The path {} is not an absolute path, did you possibly mean {}?",
                path_str,
                suggestion.to_string_lossy(),
            ))),
        }
    }

    // Shell command execution with platform-specific handling
    async fn bash(
        &self,
        params: Value,
        notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Result<Vec<Content>, ToolError> {
        let command =
            params
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The command string is required".to_string(),
                ))?;

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
                return Err(ToolError::ExecutionError(format!(
                    "The command attempts to access '{}' which is restricted by .gooseignore",
                    arg
                )));
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
            .args(&shell_config.args)
            .arg(command)
            .spawn()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());

        let output_task = tokio::spawn(async move {
            let mut combined_output = String::new();

            // We have the individual two streams above, now merge them into one unified stream of
            // an enum. ref https://blog.yoshuawuyts.com/futures-concurrency-3
            let stdout = SplitStream::new(stdout.split(b'\n')).map(|v| ("stdout", v));
            let stderr = SplitStream::new(stderr.split(b'\n')).map(|v| ("stderr", v));
            let mut merged = stdout.merge(stderr);

            while let Some((key, line)) = merged.next().await {
                let mut line = line?;
                // Re-add this as clients expect it
                line.push(b'\n');
                // Here we always convert to UTF-8 so agents don't have to deal with corrupted output
                let line = String::from_utf8_lossy(&line);

                combined_output.push_str(&line);

                notifier
                    .try_send(JsonRpcMessage::Notification(JsonRpcNotification {
                        jsonrpc: JsonRpcVersion2_0,
                        notification: Notification {
                            method: "notifications/message".to_string(),
                            params: object!({
                                "level": "info",
                                "data": {
                                    "type": "shell",
                                    "stream": key,
                                    "output": line,
                                }
                            }),
                            extensions: Default::default(),
                        },
                    }))
                    .ok();
            }
            Ok::<_, std::io::Error>(combined_output)
        });

        // Wait for the command to complete and get output
        child
            .wait()
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let output_str = match output_task.await {
            Ok(result) => result.map_err(|e| ToolError::ExecutionError(e.to_string()))?,
            Err(e) => return Err(ToolError::ExecutionError(e.to_string())),
        };

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

        let (final_output, user_output) = self.process_shell_output(&output_str)?;

        Ok(vec![
            Content::text(final_output).with_audience(vec![Role::Assistant]),
            Content::text(user_output)
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

        // Check if file is ignored before proceeding with any text editor operation
        if self.is_ignored(&path) {
            return Err(ToolError::ExecutionError(format!(
                "Access to '{}' is restricted by .gooseignore",
                path.display()
            )));
        }

        match command {
            "view" => {
                let view_range = params
                    .get("view_range")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| {
                        if arr.len() == 2 {
                            let start = arr[0].as_i64().unwrap_or(1) as usize;
                            let end = arr[1].as_i64().unwrap_or(-1);
                            Some((start, end))
                        } else {
                            None
                        }
                    });
                self.text_editor_view(&path, view_range).await
            }
            "write" => {
                let file_text = require_str_parameter(&params, "file_text")?;

                self.text_editor_write(&path, file_text).await
            }
            "str_replace" | "edit_file" => {
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
            "insert" => {
                let insert_line = params
                    .get("insert_line")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| {
                        ToolError::InvalidParameters("Missing 'insert_line' parameter".into())
                    })? as usize;
                let new_str = params
                    .get("new_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ToolError::InvalidParameters("Missing 'new_str' parameter".into())
                    })?;

                self.text_editor_insert(&path, insert_line, new_str).await
            }
            "undo_edit" => self.text_editor_undo(&path).await,
            _ => Err(ToolError::InvalidParameters(format!(
                "Unknown command '{}'",
                command
            ))),
        }
    }

    // Helper method to validate and calculate view range indices
    fn calculate_view_range(
        &self,
        view_range: Option<(usize, i64)>,
        total_lines: usize,
    ) -> Result<(usize, usize), ToolError> {
        if let Some((start_line, end_line)) = view_range {
            // Convert 1-indexed line numbers to 0-indexed
            let start_idx = if start_line > 0 { start_line - 1 } else { 0 };
            let end_idx = if end_line == -1 {
                total_lines
            } else {
                std::cmp::min(end_line as usize, total_lines)
            };

            if start_idx >= total_lines {
                return Err(ToolError::InvalidParameters(format!(
                    "Start line {} is beyond the end of the file (total lines: {})",
                    start_line, total_lines
                )));
            }

            if start_idx >= end_idx {
                return Err(ToolError::InvalidParameters(format!(
                    "Start line {} must be less than end line {}",
                    start_line, end_line
                )));
            }

            Ok((start_idx, end_idx))
        } else {
            Ok((0, total_lines))
        }
    }

    // Helper method to format file content with line numbers
    fn format_file_content(
        &self,
        path: &Path,
        lines: &[&str],
        start_idx: usize,
        end_idx: usize,
        view_range: Option<(usize, i64)>,
    ) -> String {
        let display_content = if lines.is_empty() {
            String::new()
        } else {
            let selected_lines: Vec<String> = lines[start_idx..end_idx]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{}: {}", start_idx + i + 1, line))
                .collect();

            selected_lines.join("\n")
        };

        let language = lang::get_language_identifier(path);
        if view_range.is_some() {
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
                content=display_content,
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
                content=display_content,
            }
        }
    }

    async fn text_editor_view(
        &self,
        path: &PathBuf,
        view_range: Option<(usize, i64)>,
    ) -> Result<Vec<Content>, ToolError> {
        if !path.is_file() {
            return Err(ToolError::ExecutionError(format!(
                "The path '{}' does not exist or is not a file.",
                path.display()
            )));
        }

        const MAX_FILE_SIZE: u64 = 400 * 1024; // 400KB

        let f = File::open(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to open file: {}", e)))?;

        let file_size = f
            .metadata()
            .map_err(|e| ToolError::ExecutionError(format!("Failed to get file metadata: {}", e)))?
            .len();

        if file_size > MAX_FILE_SIZE {
            return Err(ToolError::ExecutionError(format!(
                "File '{}' is too large ({:.2}KB). Maximum size is 400KB to prevent memory issues.",
                path.display(),
                file_size as f64 / 1024.0
            )));
        }

        // Ensure we never read over that limit even if the file is being concurrently mutated
        let mut f = f.take(MAX_FILE_SIZE);

        let uri = Url::from_file_path(path)
            .map_err(|_| ToolError::ExecutionError("Invalid file path".into()))?
            .to_string();

        let mut content = String::new();
        f.read_to_string(&mut content)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // We will gently encourage the LLM to specify a range for large line count files
        // it can of course specify exact range to read any size file
        if view_range.is_none() && total_lines > LINE_READ_LIMIT {
            return recommend_read_range(path, total_lines);
        }

        let (start_idx, end_idx) = self.calculate_view_range(view_range, total_lines)?;
        let formatted = self.format_file_content(path, &lines, start_idx, end_idx, view_range);

        // The LLM gets just a quick update as we expect the file to view in the status
        // but we send a low priority message for the human
        Ok(vec![
            Content::embedded_text(uri, content).with_audience(vec![Role::Assistant]),
            Content::text(formatted)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    }

    async fn text_editor_write(
        &self,
        path: &PathBuf,
        file_text: &str,
    ) -> Result<Vec<Content>, ToolError> {
        // Normalize line endings based on platform
        let mut normalized_text = normalize_line_endings(file_text); // Make mutable

        // Ensure the text ends with a newline
        if !normalized_text.ends_with('\n') {
            normalized_text.push('\n');
        }

        // Write to the file
        std::fs::write(path, &normalized_text) // Write the potentially modified text
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Try to detect the language from the file extension
        let language = lang::get_language_identifier(path);

        // The assistant output does not show the file again because the content is already in the tool request
        // but we do show it to the user here, using the final written content
        Ok(vec![
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
                content=&normalized_text // Use the final normalized_text for user feedback
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

        // Check if Editor API is configured and use it as the primary path
        if let Some(ref editor) = self.editor_model {
            // Editor API path - save history then call API directly
            self.save_file_history(path)?;

            match editor.edit_code(&content, old_str, new_str).await {
                Ok(updated_content) => {
                    // Write the updated content directly
                    let normalized_content = normalize_line_endings(&updated_content);
                    std::fs::write(path, &normalized_content).map_err(|e| {
                        ToolError::ExecutionError(format!("Failed to write file: {}", e))
                    })?;

                    // Simple success message for Editor API
                    return Ok(vec![
                        Content::text(format!("Successfully edited {}", path.display()))
                            .with_audience(vec![Role::Assistant]),
                        Content::text(format!("File {} has been edited", path.display()))
                            .with_audience(vec![Role::User])
                            .with_priority(0.2),
                    ]);
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

        // Traditional string replacement path (original logic)
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

        // Save history for undo (original behavior - after validation)
        self.save_file_history(path)?;

        let new_content = content.replace(old_str, new_str);
        let normalized_content = normalize_line_endings(&new_content);
        std::fs::write(path, &normalized_content)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Try to detect the language from the file extension
        let language = lang::get_language_identifier(path);

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
        let end_line = replacement_line + SNIPPET_LINES + new_content.matches('\n').count();

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

    async fn text_editor_insert(
        &self,
        path: &PathBuf,
        insert_line: usize,
        new_str: &str,
    ) -> Result<Vec<Content>, ToolError> {
        // Check if file exists
        if !path.exists() {
            return Err(ToolError::InvalidParameters(format!(
                "File '{}' does not exist, you can write a new file with the `write` command",
                path.display()
            )));
        }

        // Read content
        let content = std::fs::read_to_string(path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read file: {}", e)))?;

        // Save history for undo
        self.save_file_history(path)?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Validate insert_line parameter
        if insert_line > total_lines {
            return Err(ToolError::InvalidParameters(format!(
                "Insert line {} is beyond the end of the file (total lines: {}). Use 0 to insert at the beginning or {} to insert at the end.",
                insert_line, total_lines, total_lines
            )));
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

        // If inserting at the end (after all existing lines)
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

        std::fs::write(path, &final_content)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Try to detect the language from the file extension
        let language = lang::get_language_identifier(path);

        // Show a snippet of the inserted content with context
        const SNIPPET_LINES: usize = 4;
        let insertion_line = insert_line + 1; // Convert to 1-indexed for display

        // Calculate start and end lines for the snippet
        let start_line = insertion_line.saturating_sub(SNIPPET_LINES);
        let end_line = std::cmp::min(insertion_line + SNIPPET_LINES, new_lines.len());

        // Get the relevant lines for our snippet with line numbers
        let snippet_lines: Vec<String> = new_lines[start_line.saturating_sub(1)..end_line]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{}: {}", start_line + i, line))
            .collect();

        let snippet = snippet_lines.join("\n");

        let output = formatdoc! {r#"
            ```{language}
            {snippet}
            ```
            "#,
            language=language,
            snippet=snippet
        };

        let success_message = formatdoc! {r#"
            Text has been inserted at line {} in {}. The section now reads:
            {}
            Review the changes above for errors. Undo and edit the file again if necessary!
            "#,
            insertion_line,
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

    #[cfg(feature = "screen_capture")]
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

    // Helper function to handle Mac screenshot filenames that contain U+202F (narrow no-break space)
    #[cfg(feature = "screen_capture")]
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
        path.to_path_buf()
    }

    #[cfg(feature = "screen_capture")]
    async fn image_processor(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'path' parameter".into()))?;

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
            return Err(ToolError::ExecutionError(format!(
                "Access to '{}' is restricted by .gooseignore",
                path.display()
            )));
        }

        // Check if file exists
        if !path.exists() {
            return Err(ToolError::ExecutionError(format!(
                "File '{}' does not exist",
                path.display()
            )));
        }

        // Check file size (10MB limit for image files)
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB in bytes
        let file_size = std::fs::metadata(&path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to get file metadata: {}", e)))?
            .len();

        if file_size > MAX_FILE_SIZE {
            return Err(ToolError::ExecutionError(format!(
                "File '{}' is too large ({:.2}MB). Maximum size is 10MB.",
                path.display(),
                file_size as f64 / (1024.0 * 1024.0)
            )));
        }

        // Open and decode the image
        let image = xcap::image::open(&path)
            .map_err(|e| ToolError::ExecutionError(format!("Failed to open image file: {}", e)))?;

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
                ToolError::ExecutionError(format!("Failed to write image buffer: {}", e))
            })?;

        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(vec![
            Content::text(format!(
                "Successfully processed image from {}",
                path.display()
            ))
            .with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png").with_priority(0.0),
        ])
    }

    #[cfg(feature = "screen_capture")]
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

fn recommend_read_range(path: &Path, total_lines: usize) -> Result<Vec<Content>, ToolError> {
    Err(ToolError::ExecutionError(format!(
        "File '{}' is {} lines long, recommended to read in with view_range (or searching) to get bite size content. If you do wish to read all the file, please pass in view_range with [1, {}] to read it all at once",
        path.display(),
        total_lines,
        total_lines
    )))
}

impl Router for DeveloperRouter {
    fn name(&self) -> String {
        "developer".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "shell" => this.bash(arguments, notifier).await,
                "text_editor" => this.text_editor(arguments).await,
                #[cfg(feature = "screen_capture")]
                "list_windows" => this.list_windows(arguments).await,
                #[cfg(feature = "screen_capture")]
                "screen_capture" => this.screen_capture(arguments).await,
                #[cfg(feature = "screen_capture")]
                "image_processor" => this.image_processor(arguments).await,
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

    fn list_prompts(&self) -> Vec<Prompt> {
        self.prompts.values().cloned().collect()
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.trim().to_owned();

        // Validate prompt name is not empty
        if prompt_name.is_empty() {
            return Box::pin(async move {
                Err(PromptError::InvalidParameters(
                    "Prompt name cannot be empty".to_string(),
                ))
            });
        }

        let prompts = Arc::clone(&self.prompts);

        Box::pin(async move {
            match prompts.get(&prompt_name) {
                Some(prompt) => Ok(prompt.description.clone().unwrap_or_default()),
                None => Err(PromptError::NotFound(format!(
                    "Prompt '{prompt_name}' not found"
                ))),
            }
        })
    }
}

impl Clone for DeveloperRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            prompts: Arc::clone(&self.prompts),
            instructions: self.instructions.clone(),
            file_history: Arc::clone(&self.file_history),
            ignore_patterns: Arc::clone(&self.ignore_patterns),
            editor_model: create_editor_model(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::panic;
    use serde_json::json;
    use serial_test::serial;
    use std::fs::{self, read_to_string};
    use tempfile::TempDir;
    use tokio::sync::OnceCell;

    #[test]
    #[serial]
    fn test_global_goosehints() {
        // if ~/.config/goose/.goosehints exists, it should be included in the instructions
        // copy the existing global hints file to a .bak file
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

        let router = DeveloperRouter::new();
        let instructions = router.instructions();

        assert!(instructions.contains("### Global Hints"));
        assert!(instructions.contains("my global goose hints."));

        // restore backup if globalhints previously existed
        if globalhints_existed {
            fs::copy(&global_hints_bak_path, &global_hints_path).unwrap();
            fs::remove_file(&global_hints_bak_path).unwrap();
        }
    }

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

    fn dummy_sender() -> mpsc::Sender<JsonRpcMessage> {
        mpsc::channel(1).0
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_missing_parameters() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let router = get_router().await;
        let result = router.call_tool("shell", json!({}), dummy_sender()).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));

        temp_dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn test_goosehints_multiple_filenames() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::env::set_var("CONTEXT_FILE_NAMES", r#"["CLAUDE.md", ".goosehints"]"#);

        fs::write("CLAUDE.md", "Custom hints file content from CLAUDE.md").unwrap();
        fs::write(".goosehints", "Custom hints file content from .goosehints").unwrap();
        let router = DeveloperRouter::new();
        let instructions = router.instructions();

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
        let router = DeveloperRouter::new();
        let instructions = router.instructions();

        assert!(instructions.contains("Custom hints file content"));
        assert!(!instructions.contains(".goosehints")); // Make sure it's not loading the default
        std::env::remove_var("CONTEXT_FILE_NAMES");
    }

    #[tokio::test]
    #[serial]
    #[cfg(windows)]
    async fn test_windows_specific_commands() {
        let router = get_router().await;

        // Test PowerShell command
        let result = router
            .call_tool(
                "shell",
                json!({
                    "command": "Get-ChildItem"
                }),
            )
            .await;
        assert!(result.is_ok());

        // Test Windows path handling
        let result = router.resolve_path("C:\\Windows\\System32");
        assert!(result.is_ok());

        // Test UNC path handling
        let result = router.resolve_path("\\\\server\\share");
        assert!(result.is_ok());
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
                    dummy_sender(),
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

            // This is above MAX_FILE_SIZE
            let content = "x".repeat(500_000);
            std::fs::write(&many_chars_path, content).unwrap();

            let result = router
                .call_tool(
                    "text_editor",
                    json!({
                        "command": "view",
                        "path": many_chars_str
                    }),
                    dummy_sender(),
                )
                .await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert!(matches!(err, ToolError::ExecutionError(_)));
            assert!(err.to_string().contains("is too large"));
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
                dummy_sender(),
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
                dummy_sender(),
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
        assert!(text.text.contains("Hello, world!"));

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
                dummy_sender(),
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
                dummy_sender(),
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

        assert!(text
            .text
            .contains("has been edited, and the section now reads"));

        // View the file to verify the change
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
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

        // Check that the file has been modified and contains some form of "Rust"
        // The Editor API might transform the content differently than simple string replacement
        assert!(
            text.text.contains("Rust") || text.text.contains("Hello, Rust!"),
            "Expected content to contain 'Rust', but got: {}",
            text.text
        );

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
                dummy_sender(),
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
                dummy_sender(),
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
                dummy_sender(),
            )
            .await
            .unwrap();

        let text = undo_result.first().unwrap().as_text().unwrap();
        assert!(text.text.contains("Undid the last edit"));

        // View the file to verify the undo
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
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
        assert!(text.text.contains("First line"));

        temp_dir.close().unwrap();
    }

    // Test GooseIgnore pattern matching
    #[tokio::test]
    #[serial]
    async fn test_goose_ignore_basic_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a DeveloperRouter with custom ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path());
        builder.add_line(None, "secret.txt").unwrap();
        builder.add_line(None, "*.env").unwrap();
        let ignore_patterns = builder.build().unwrap();

        let router = DeveloperRouter {
            tools: vec![],
            prompts: Arc::new(HashMap::new()),
            instructions: String::new(),
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: Arc::new(ignore_patterns),
            editor_model: None,
        };

        // Test basic file matching
        assert!(
            router.is_ignored(Path::new("secret.txt")),
            "secret.txt should be ignored"
        );
        assert!(
            router.is_ignored(Path::new("./secret.txt")),
            "./secret.txt should be ignored"
        );
        assert!(
            !router.is_ignored(Path::new("not_secret.txt")),
            "not_secret.txt should not be ignored"
        );

        // Test pattern matching
        assert!(
            router.is_ignored(Path::new("test.env")),
            "*.env pattern should match test.env"
        );
        assert!(
            router.is_ignored(Path::new("./test.env")),
            "*.env pattern should match ./test.env"
        );
        assert!(
            !router.is_ignored(Path::new("test.txt")),
            "*.env pattern should not match test.txt"
        );

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_respects_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a DeveloperRouter with custom ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path());
        builder.add_line(None, "secret.txt").unwrap();
        let ignore_patterns = builder.build().unwrap();

        let router = DeveloperRouter {
            tools: DeveloperRouter::new().tools, // Reuse default tools
            prompts: Arc::new(HashMap::new()),
            instructions: String::new(),
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: Arc::new(ignore_patterns),
            editor_model: None,
        };

        // Try to write to an ignored file
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": temp_dir.path().join("secret.txt").to_str().unwrap(),
                    "file_text": "test content"
                }),
                dummy_sender(),
            )
            .await;

        assert!(
            result.is_err(),
            "Should not be able to write to ignored file"
        );
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionError(_)));

        // Try to write to a non-ignored file
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": temp_dir.path().join("allowed.txt").to_str().unwrap(),
                    "file_text": "test content"
                }),
                dummy_sender(),
            )
            .await;

        assert!(
            result.is_ok(),
            "Should be able to write to non-ignored file"
        );

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_bash_respects_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a DeveloperRouter with custom ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path());
        builder.add_line(None, "secret.txt").unwrap();
        let ignore_patterns = builder.build().unwrap();

        let router = DeveloperRouter {
            tools: DeveloperRouter::new().tools, // Reuse default tools
            prompts: Arc::new(HashMap::new()),
            instructions: String::new(),
            file_history: Arc::new(Mutex::new(HashMap::new())),
            ignore_patterns: Arc::new(ignore_patterns),
            editor_model: None,
        };

        // Create an ignored file
        let secret_file_path = temp_dir.path().join("secret.txt");
        std::fs::write(&secret_file_path, "secret content").unwrap();

        // Try to cat the ignored file
        let result = router
            .call_tool(
                "shell",
                json!({
                    "command": format!("cat {}", secret_file_path.to_str().unwrap())
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err(), "Should not be able to cat ignored file");
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionError(_)));

        // Try to cat a non-ignored file
        let allowed_file_path = temp_dir.path().join("allowed.txt");
        std::fs::write(&allowed_file_path, "allowed content").unwrap();

        let result = router
            .call_tool(
                "shell",
                json!({
                    "command": format!("cat {}", allowed_file_path.to_str().unwrap())
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_ok(), "Should be able to cat non-ignored file");

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_gitignore_fallback_when_no_gooseignore() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a .gitignore file but no .gooseignore
        std::fs::write(temp_dir.path().join(".gitignore"), "*.log\n*.tmp\n.env").unwrap();

        let router = DeveloperRouter::new();

        // Test that gitignore patterns are respected
        assert!(
            router.is_ignored(Path::new("test.log")),
            "*.log pattern from .gitignore should be ignored"
        );
        assert!(
            router.is_ignored(Path::new("build.tmp")),
            "*.tmp pattern from .gitignore should be ignored"
        );
        assert!(
            router.is_ignored(Path::new(".env")),
            ".env pattern from .gitignore should be ignored"
        );
        assert!(
            !router.is_ignored(Path::new("test.txt")),
            "test.txt should not be ignored"
        );

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_gooseignore_takes_precedence_over_gitignore() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create both .gooseignore and .gitignore files with different patterns
        std::fs::write(temp_dir.path().join(".gooseignore"), "*.secret").unwrap();
        std::fs::write(temp_dir.path().join(".gitignore"), "*.log\ntarget/").unwrap();

        let router = DeveloperRouter::new();

        // .gooseignore patterns should be used
        assert!(
            router.is_ignored(Path::new("test.secret")),
            "*.secret pattern from .gooseignore should be ignored"
        );

        // .gitignore patterns should NOT be used when .gooseignore exists
        assert!(
            !router.is_ignored(Path::new("test.log")),
            "*.log pattern from .gitignore should NOT be ignored when .gooseignore exists"
        );
        assert!(
            !router.is_ignored(Path::new("build.tmp")),
            "*.tmp pattern from .gitignore should NOT be ignored when .gooseignore exists"
        );

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_default_patterns_when_no_ignore_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Don't create any ignore files
        let router = DeveloperRouter::new();

        // Default patterns should be used
        assert!(
            router.is_ignored(Path::new(".env")),
            ".env should be ignored by default patterns"
        );
        assert!(
            router.is_ignored(Path::new(".env.local")),
            ".env.local should be ignored by default patterns"
        );
        assert!(
            router.is_ignored(Path::new("secrets.txt")),
            "secrets.txt should be ignored by default patterns"
        );
        assert!(
            !router.is_ignored(Path::new("normal.txt")),
            "normal.txt should not be ignored"
        );

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_descriptions() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Test without editor API configured (should be the case in tests due to cfg!(test))
        let router = DeveloperRouter::new();
        let tools = router.list_tools();
        let text_editor_tool = tools.iter().find(|t| t.name == "text_editor").unwrap();

        // Should use traditional description with str_replace command
        assert!(text_editor_tool
            .description
            .as_ref()
            .map_or(false, |desc| desc
                .contains("Replace a string in a file with a new string")));
        assert!(text_editor_tool
            .description
            .as_ref()
            .map_or(false, |desc| desc
                .contains("the `old_str` needs to exactly match one")));
        assert!(text_editor_tool
            .description
            .as_ref()
            .map_or(false, |desc| desc.contains("str_replace")));

        // Should not contain editor API description or edit_file command
        assert!(!text_editor_tool
            .description
            .as_ref()
            .map_or(false, |desc| desc
                .contains("Edit the file with the new content")));
        assert!(!text_editor_tool
            .description
            .as_ref()
            .map_or(false, |desc| desc.contains("edit_file")));
        assert!(!text_editor_tool
            .description
            .as_ref()
            .map_or(false, |desc| desc
                .contains("work out how to place old_str with it intelligently")));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_respects_gitignore_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a .gitignore file but no .gooseignore
        std::fs::write(temp_dir.path().join(".gitignore"), "*.log").unwrap();

        let router = DeveloperRouter::new();

        // Try to write to a file ignored by .gitignore
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": temp_dir.path().join("test.log").to_str().unwrap(),
                    "file_text": "test content"
                }),
                dummy_sender(),
            )
            .await;

        assert!(
            result.is_err(),
            "Should not be able to write to file ignored by .gitignore fallback"
        );
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionError(_)));

        // Try to write to a non-ignored file
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": temp_dir.path().join("allowed.txt").to_str().unwrap(),
                    "file_text": "test content"
                }),
                dummy_sender(),
            )
            .await;

        assert!(
            result.is_ok(),
            "Should be able to write to non-ignored file"
        );

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_bash_respects_gitignore_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a .gitignore file but no .gooseignore
        std::fs::write(temp_dir.path().join(".gitignore"), "*.log").unwrap();

        let router = DeveloperRouter::new();

        // Create a file that would be ignored by .gitignore
        let log_file_path = temp_dir.path().join("test.log");
        std::fs::write(&log_file_path, "log content").unwrap();

        // Try to cat the ignored file
        let result = router
            .call_tool(
                "shell",
                json!({
                    "command": format!("cat {}", log_file_path.to_str().unwrap())
                }),
                dummy_sender(),
            )
            .await;

        assert!(
            result.is_err(),
            "Should not be able to cat file ignored by .gitignore fallback"
        );
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionError(_)));

        // Try to cat a non-ignored file
        let allowed_file_path = temp_dir.path().join("allowed.txt");
        std::fs::write(&allowed_file_path, "allowed content").unwrap();

        let result = router
            .call_tool(
                "shell",
                json!({
                    "command": format!("cat {}", allowed_file_path.to_str().unwrap())
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_ok(), "Should be able to cat non-ignored file");

        temp_dir.close().unwrap();
    }

    // Tests for view_range functionality
    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a multi-line file
        let content =
            "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Test viewing specific range
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str,
                    "view_range": [3, 6]
                }),
                dummy_sender(),
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

        // Should contain lines 3-6 with line numbers
        assert!(text.text.contains("3: Line 3"));
        assert!(text.text.contains("4: Line 4"));
        assert!(text.text.contains("5: Line 5"));
        assert!(text.text.contains("6: Line 6"));
        assert!(text.text.contains("(lines 3-6)"));
        // Should not contain other lines
        assert!(!text.text.contains("1: Line 1"));
        assert!(!text.text.contains("7: Line 7"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range_to_end() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a multi-line file
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Test viewing from line 3 to end using -1
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str,
                    "view_range": [3, -1]
                }),
                dummy_sender(),
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

        // Should contain lines 3 to end
        assert!(text.text.contains("3: Line 3"));
        assert!(text.text.contains("4: Line 4"));
        assert!(text.text.contains("5: Line 5"));
        assert!(text.text.contains("(lines 3-end)"));
        // Should not contain earlier lines
        assert!(!text.text.contains("1: Line 1"));
        assert!(!text.text.contains("2: Line 2"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range_invalid() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a small file
        let content = "Line 1\nLine 2\nLine 3";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Test invalid range - start beyond end of file
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str,
                    "view_range": [10, 15]
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));
        assert!(err.to_string().contains("beyond the end of the file"));

        // Test invalid range - start >= end
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str,
                    "view_range": [3, 2]
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));
        assert!(err.to_string().contains("must be less than end line"));

        temp_dir.close().unwrap();
    }

    // Tests for insert functionality
    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_at_beginning() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with some content
        let content = "Line 2\nLine 3\nLine 4";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Insert at the beginning (line 0)
        let insert_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "insert_line": 0,
                    "new_str": "Line 1"
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let text = insert_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Text has been inserted at line 1"));

        // Verify the file content
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let view_text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(view_text.text.contains("1: Line 1"));
        assert!(view_text.text.contains("2: Line 2"));
        assert!(view_text.text.contains("3: Line 3"));
        assert!(view_text.text.contains("4: Line 4"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_in_middle() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 4\nLine 5";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Insert after line 2
        let insert_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "insert_line": 2,
                    "new_str": "Line 3"
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let text = insert_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Text has been inserted at line 3"));

        // Verify the file content
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let view_text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(view_text.text.contains("1: Line 1"));
        assert!(view_text.text.contains("2: Line 2"));
        assert!(view_text.text.contains("3: Line 3"));
        assert!(view_text.text.contains("4: Line 4"));
        assert!(view_text.text.contains("5: Line 5"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_at_end() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 3";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Insert at the end (after line 3)
        let insert_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "insert_line": 3,
                    "new_str": "Line 4"
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let text = insert_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Text has been inserted at line 4"));

        // Verify the file content
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let view_text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(view_text.text.contains("1: Line 1"));
        assert!(view_text.text.contains("2: Line 2"));
        assert!(view_text.text.contains("3: Line 3"));
        assert!(view_text.text.contains("4: Line 4"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_invalid_line() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 3";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Try to insert beyond the end of the file
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "insert_line": 10,
                    "new_str": "Line 11"
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));
        assert!(err.to_string().contains("beyond the end of the file"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_missing_parameters() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": "Test content"
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Try insert without insert_line parameter
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "new_str": "New line"
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));
        assert!(err.to_string().contains("Missing 'insert_line' parameter"));

        // Try insert without new_str parameter
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "insert_line": 1
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));
        assert!(err.to_string().contains("Missing 'new_str' parameter"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_with_undo() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with some content
        let content = "Line 1\nLine 2";
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Insert a line
        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "insert_line": 1,
                    "new_str": "Inserted Line"
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Undo the insert
        let undo_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "undo_edit",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let text = undo_result.first().unwrap().as_text().unwrap();
        assert!(text.text.contains("Undid the last edit"));

        // Verify the file is back to original content
        let view_result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        let view_text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(view_text.text.contains("1: Line 1"));
        assert!(view_text.text.contains("2: Line 2"));
        assert!(!view_text.text.contains("Inserted Line"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_nonexistent_file() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Try to insert into a nonexistent file
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "insert",
                    "path": file_path_str,
                    "insert_line": 0,
                    "new_str": "New line"
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));
        assert!(err.to_string().contains("does not exist"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_large_file_without_range() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with more than LINE_READ_LIMIT lines
        let mut content = String::new();
        for i in 1..=LINE_READ_LIMIT + 1 {
            content.push_str(&format!("Line {}\n", i));
        }

        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Test viewing without view_range - should trigger the error
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::ExecutionError(_)));
        assert!(err.to_string().contains("2001 lines long"));
        assert!(err
            .to_string()
            .contains("recommended to read in with view_range"));
        assert!(err
            .to_string()
            .contains("please pass in view_range with [1, 2001]"));

        // Test viewing with view_range - should work
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str,
                    "view_range": [1, 100]
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_ok());
        let view_result = result.unwrap();
        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain lines 1-100
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("100: Line 100"));
        assert!(!text.text.contains("101: Line 101"));

        // Test viewing with explicit full range - should work
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str,
                    "view_range": [1, 2001]
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_ok());

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_file_with_exactly_2000_lines() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("file_2000.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with exactly 2000 lines (should not trigger the check)
        let mut content = String::new();
        for i in 1..=2000 {
            content.push_str(&format!("Line {}\n", i));
        }

        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Test viewing without view_range - should work since it's exactly 2000 lines
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_ok());
        let view_result = result.unwrap();
        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain all lines
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("2000: Line 2000"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_small_file_without_range() {
        let router = get_router().await;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("small_file.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a file with less than 2000 lines
        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!("Line {}\n", i));
        }

        router
            .call_tool(
                "text_editor",
                json!({
                    "command": "write",
                    "path": file_path_str,
                    "file_text": content
                }),
                dummy_sender(),
            )
            .await
            .unwrap();

        // Test viewing without view_range - should work fine
        let result = router
            .call_tool(
                "text_editor",
                json!({
                    "command": "view",
                    "path": file_path_str
                }),
                dummy_sender(),
            )
            .await;

        assert!(result.is_ok());
        let view_result = result.unwrap();
        let text = view_result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain all lines
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("100: Line 100"));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_bash_output_truncation() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let router = get_router().await;

        // Create a command that generates > 100 lines of output
        let command = if cfg!(windows) {
            "for /L %i in (1,1,150) do @echo Line %i"
        } else {
            "for i in {1..150}; do echo \"Line $i\"; done"
        };

        let result = router
            .call_tool("shell", json!({ "command": command }), dummy_sender())
            .await
            .unwrap();

        // Should have two Content items
        assert_eq!(result.len(), 2);

        // Find the Assistant and User content
        let assistant_content = result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        let user_content = result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Assistant should get the full message with temp file info
        assert!(assistant_content.text.contains("private note: output was"));

        // User should only get the truncated output with prefix
        assert!(user_content.text.starts_with("..."));
        assert!(!user_content.text.contains("private note: output was"));

        // User output should contain lines 51-150 (last 100 lines)
        assert!(user_content.text.contains("Line 51"));
        assert!(user_content.text.contains("Line 150"));
        assert!(!user_content.text.contains("Line 50"));

        let start_tag = "remainder of lines in";
        let end_tag = "do not show tmp file to user";

        if let (Some(start), Some(end)) = (
            assistant_content.text.find(start_tag),
            assistant_content.text.find(end_tag),
        ) {
            let start_idx = start + start_tag.len();
            if start_idx < end {
                let path = assistant_content.text[start_idx..end].trim();
                println!("Extracted path: {}", path);

                let file_contents =
                    read_to_string(path).expect("Failed to read extracted temp file");

                let lines: Vec<&str> = file_contents.lines().collect();

                // Ensure we have exactly 150 lines
                assert_eq!(lines.len(), 150, "Expected 150 lines in temp file");

                // Ensure the first and last lines are correct
                assert_eq!(lines.first(), Some(&"Line 1"), "First line mismatch");
                assert_eq!(lines.last(), Some(&"Line 150"), "Last line mismatch");
            } else {
                panic!("No path found in bash output truncation output");
            }
        } else {
            panic!("Failed to find start or end tag in bash output truncation output");
        }

        temp_dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn test_process_shell_output_short() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let router = DeveloperRouter::new();

        // Test with short output (< 100 lines)
        let short_output = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let result = router.process_shell_output(short_output).unwrap();

        // Both outputs should be the same for short outputs
        assert_eq!(result.0, short_output);
        assert_eq!(result.1, short_output);
    }

    #[test]
    #[serial]
    fn test_process_shell_output_empty() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let router = DeveloperRouter::new();

        // Test with empty output
        let empty_output = "";
        let result = router.process_shell_output(empty_output).unwrap();

        // Both outputs should be empty
        assert_eq!(result.0, "");
        assert_eq!(result.1, "");
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_output_without_trailing_newline() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let router = get_router().await;

        // Test command that outputs content without a trailing newline
        let command = if cfg!(windows) {
            "echo|set /p=\"Content without newline\""
        } else {
            "printf 'Content without newline'"
        };

        let result = router
            .call_tool("shell", json!({ "command": command }), dummy_sender())
            .await
            .unwrap();

        // Find the assistant content (which contains the full output)
        let assistant_content = result
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // The output should contain the content even without a trailing newline
        assert!(
            assistant_content.text.contains("Content without newline"),
            "Output should contain content even without trailing newline, but got: {}",
            assistant_content.text
        );

        temp_dir.close().unwrap();
    }
}
