//! Developer tools for the Goose agent
//!
//! This module contains tools for shell execution, file editing, screen capture,
//! and other developer-focused functionality.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const DEVELOPER_SHELL_TOOL_NAME: &str = "developer__shell";
pub const DEVELOPER_GLOB_TOOL_NAME: &str = "developer__glob";
pub const DEVELOPER_GREP_TOOL_NAME: &str = "developer__grep";
pub const DEVELOPER_TEXT_EDITOR_TOOL_NAME: &str = "developer__text_editor";
pub const DEVELOPER_LIST_WINDOWS_TOOL_NAME: &str = "developer__list_windows";
pub const DEVELOPER_SCREEN_CAPTURE_TOOL_NAME: &str = "developer__screen_capture";
pub const DEVELOPER_IMAGE_PROCESSOR_TOOL_NAME: &str = "developer__image_processor";

/// Create the shell tool definition
pub fn shell_tool() -> Tool {
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

            **Important**: Each shell command runs in its own process. Things like directory changes or
            sourcing files do not persist between tool calls. So you may need to repeat them each time by
            stringing together commands, e.g. `cd example && ls` or `source env/bin/activate && pip install numpy`

            - Restrictions: Avoid find, grep, cat, head, tail, ls - use dedicated tools instead (Grep, Glob, Read, LS)
            - Multiple commands: Use ; or && to chain commands, avoid newlines
            - Pathnames: Use absolute paths and avoid cd unless explicitly requested
        "#},
    };

    Tool::new(
        DEVELOPER_SHELL_TOOL_NAME.to_string(),
        shell_tool_desc.to_string(),
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {"type": "string", "description": "The shell command to execute"}
            }
        }),
        None,
    )
}

/// Create the glob tool definition
pub fn glob_tool() -> Tool {
    Tool::new(
        DEVELOPER_GLOB_TOOL_NAME.to_string(),
        indoc! {r#"
            Search for files using glob patterns.
            
            This tool provides fast file pattern matching using glob syntax.
            Returns matching file paths sorted by modification time.
            Examples:
            - `*.rs` - Find all Rust files in current directory
            - `src/**/*.py` - Find all Python files recursively in src directory
            - `**/test*.js` - Find all JavaScript test files recursively
            
            **Important**: Use this tool instead of shell commands like `find` or `ls -r` for file searching,
            as it properly handles ignored files and is more efficient. This tool respects .gooseignore patterns.
            
            Use this tool when you need to locate files by name patterns rather than content.
        "#}.to_string(),
        json!({
            "type": "object",
            "required": ["pattern"],
            "properties": {
                "pattern": {"type": "string", "description": "The glob pattern to search for"},
                "path": {"type": "string", "description": "The directory to search in (defaults to current directory)"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Search files by pattern".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }),
    )
}

/// Create the grep tool definition
pub fn grep_tool() -> Tool {
    Tool::new(
        DEVELOPER_GREP_TOOL_NAME.to_string(),
        indoc! {r#"
            Execute file content search commands using ripgrep, grep, or find.
            
            Use this tool to run search commands that look for content within files. The tool
            executes your command directly and filters results to respect .gooseignore patterns.
            
            **Recommended tools and usage:**
            
            **ripgrep (rg)** - Fast, recommended for most searches:
            - List files containing pattern: `rg -l "pattern"`
            - Case-insensitive search: `rg -i "pattern"`
            - Search specific file types: `rg "pattern" --glob "*.js"`
            - Show matches with context: `rg "pattern" -C 3`
            - List files by name: `rg --files | rg <filename>`
            - List files that contain a regex: `rg '<regex>' -l`
            - Sort by modification time: `rg -l "pattern" --sort modified`
            
            **grep** - Traditional Unix tool:
            - Recursive search: `grep -r "pattern" .`
            - List files only: `grep -rl "pattern" .`
            - Include specific files: `grep -r "pattern" --include="*.py"`
            
            **find + grep** - When you need complex file filtering:
            - `find . -name "*.py" -exec grep -l "pattern" {} \;`
            - `find . -type f -newer file.txt -exec grep "pattern" {} \;`
            
            **Important**: Use this tool instead of the shell tool for search commands, as it
            properly filters results to respect ignored files.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {"type": "string", "description": "The search command to execute (rg, grep, find, etc.)"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Search file contents".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }),
    )
}

/// Create the text editor tool definition
pub fn text_editor_tool() -> Tool {
    Tool::new(
        DEVELOPER_TEXT_EDITOR_TOOL_NAME.to_string(),
        indoc! {r#"
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
                    "enum": ["view", "write", "str_replace", "insert", "undo_edit"],
                    "description": "Allowed options are: `view`, `write`, `str_replace`, `insert`, `undo_edit`."
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
        None,
    )
}

/// Create the list windows tool definition
pub fn list_windows_tool() -> Tool {
    Tool::new(
        DEVELOPER_LIST_WINDOWS_TOOL_NAME.to_string(),
        indoc! {r#"
            List all available window titles that can be used with screen_capture.
            Returns a list of window titles that can be used with the window_title parameter
            of the screen_capture tool.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": [],
            "properties": {}
        }),
        Some(ToolAnnotations {
            title: Some("List available windows".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

/// Create the screen capture tool definition
pub fn screen_capture_tool() -> Tool {
    Tool::new(
        DEVELOPER_SCREEN_CAPTURE_TOOL_NAME.to_string(),
        indoc! {r#"
            Capture a screenshot of a specified display or window.
            You can capture either:
            1. A full display (monitor) using the display parameter
            2. A specific window by its title using the window_title parameter

            Only one of display or window_title should be specified.
        "#}
        .to_string(),
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
        Some(ToolAnnotations {
            title: Some("Capture a screenshot".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

/// Create the image processor tool definition
pub fn image_processor_tool() -> Tool {
    Tool::new(
        DEVELOPER_IMAGE_PROCESSOR_TOOL_NAME.to_string(),
        indoc! {r#"
            Process an image file from disk. The image will be:
            1. Resized if larger than max width while maintaining aspect ratio
            2. Converted to PNG format
            3. Returned as base64 encoded data

            This allows processing image files for use in the conversation.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the image file to process"
                }
            }
        }),
        Some(ToolAnnotations {
            title: Some("Process Image".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }),
    )
}

/// Developer tools state management
pub struct DeveloperToolsState {
    pub file_history: Arc<Mutex<HashMap<PathBuf, Vec<String>>>>,
}

impl DeveloperToolsState {
    pub fn new() -> Self {
        Self {
            file_history: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for DeveloperToolsState {
    fn default() -> Self {
        Self::new()
    }
}
