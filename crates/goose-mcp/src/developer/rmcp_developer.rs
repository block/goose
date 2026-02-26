use anyhow::anyhow;
use base64::Engine;
use etcetera::AppStrategy;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use include_dir::{include_dir, Dir};
use indoc::{formatdoc, indoc};
use once_cell::sync::Lazy;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, CancelledNotificationParam, Content, ErrorCode, ErrorData,
        GetPromptRequestParams, GetPromptResult, Implementation, ListPromptsResult, LoggingLevel,
        LoggingMessageNotificationParam, Meta, PaginatedRequestParams, Prompt, PromptArgument,
        PromptMessage, PromptMessageRole, Role, ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    service::{NotificationContext, RequestContext},
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};

const WORKING_DIR_HEADER: &str = "agent-working-dir";
const SESSION_ID_HEADER: &str = "agent-session-id";

pub const WORKING_DIR_PLACEHOLDER: &str = "{{WORKING_DIR}}";

fn extract_working_dir_from_meta(meta: &Meta) -> Option<PathBuf> {
    meta.0
        .get(WORKING_DIR_HEADER)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .filter(|s| !s.contains('\0'))
        .map(PathBuf::from)
}

fn extract_session_id_from_meta(meta: &Meta) -> Option<String> {
    meta.0
        .get(SESSION_ID_HEADER)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .filter(|s| !s.contains('\0'))
        .map(String::from)
}

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::join_paths,
    ffi::OsString,
    future::Future,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};
use xcap::{Monitor, Window};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::RwLock,
};
use tokio_stream::{wrappers::SplitStream, StreamExt as _};
use tokio_util::sync::CancellationToken;

use crate::developer::{paths::get_shell_path_dirs, shell::ShellConfig};

use super::analyze::{types::AnalyzeParams, CodeAnalyzer};
use super::editor_models::{create_editor_model, EditorModel};
use super::shell::{configure_shell_command, expand_path, is_absolute_path, kill_process_group};
use super::text_editor::{text_editor_replace, text_editor_view, text_editor_write};

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

/// Parameters for the read_file tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    /// Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`.
    pub path: String,

    /// Optional array of two integers specifying the start and end line numbers to view.
    /// Line numbers are 1-indexed, and -1 for the end line means read to the end of the file.
    /// This parameter only applies when viewing files, not directories.
    pub view_range: Option<Vec<i64>>,
}

/// Parameters for the edit_file tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EditFileParams {
    /// Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`.
    pub path: String,

    /// The old string to replace. Must match exactly once in the file.
    pub old_str: Option<String>,

    /// The new string to replace old_str with.
    pub new_str: Option<String>,

    /// Unified diff to apply. Supports editing multiple files simultaneously.
    /// Example: "--- a/file\n+++ b/file\n@@ -1,3 +1,3 @@\n context\n-old\n+new\n context"
    /// Preferred for multi-file edits.
    pub diff: Option<String>,
}

/// Parameters for the write_file tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WriteFileParams {
    /// Absolute path to file, e.g. `/repo/file.py`.
    pub path: String,

    /// The content to write to the file. This is a full overwrite.
    pub file_text: String,
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

/// Template structure for prompt definitions
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub template: String,
    pub arguments: Vec<PromptArgumentTemplate>,
}

/// Template structure for prompt arguments
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptArgumentTemplate {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

// Embeds the prompts directory to the build
static PROMPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/developer/prompts");

static MACOS_SCREENSHOT_FILENAME_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(
        r"^Screenshot \d{4}-\d{2}-\d{2} at \d{1,2}\.\d{2}\.\d{2} (AM|PM|am|pm)(?: \(\d+\))?\.png$",
    )
    .expect("macOS screenshot filename regex should be valid")
});

const DEFAULT_GOOSEIGNORE_CONTENT: &str = concat!(
    "# This file is created automatically if no .gooseignore exists.\n",
    "# Customize or uncomment the patterns below instead of deleting the file.\n",
    "# Removing it will simply cause goose to recreate it on the next start.\n",
    "#\n",
    "# Suggested patterns you can uncomment if desired:\n",
    "# **/.ssh/**        # block SSH keys and configs\n",
    "# **/*.key         # block loose private keys\n",
    "# **/*.pem         # block certificates/private keys\n",
    "# **/.git/**        # block git metadata entirely\n",
    "# **/target/**     # block Rust build artifacts\n",
    "# **/node_modules/** # block JS/TS dependencies\n",
    "# **/*.db          # block local database files\n",
    "# **/*.sqlite      # block SQLite databases\n",
    "#\n",
    "\n",
    "**/.env\n",
    "**/.env.*\n",
    "**/secrets.*\n",
);

/// Loads prompt files from the embedded PROMPTS_DIR and returns a HashMap of prompts.
/// Ensures that each prompt name is unique.
fn load_prompt_files() -> HashMap<String, Prompt> {
    let mut prompts = HashMap::new();

    for entry in PROMPTS_DIR.files() {
        // Only process JSON files
        if entry.path().extension().is_none_or(|ext| ext != "json") {
            continue;
        }

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
                title: None,
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

/// Developer MCP Server using official RMCP SDK
#[derive(Clone)]
pub struct DeveloperServer {
    tool_router: ToolRouter<Self>,
    ignore_patterns: Gitignore,
    editor_model: Option<EditorModel>,
    prompts: HashMap<String, Prompt>,
    code_analyzer: CodeAnalyzer,
    #[cfg(test)]
    pub running_processes: Arc<RwLock<HashMap<String, CancellationToken>>>,
    #[cfg(not(test))]
    running_processes: Arc<RwLock<HashMap<String, CancellationToken>>>,
    bash_env_file: Option<PathBuf>,
    extend_path_with_shell: bool,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DeveloperServer {
    #[allow(clippy::too_many_lines)]
    fn get_info(&self) -> ServerInfo {
        let os = std::env::consts::OS;
        let in_container = Self::is_definitely_container();

        let base_instructions = match os {
            "windows" => formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

                You can use the shell tool to run Windows commands (PowerShell or CMD).
                When using paths, you can use either backslashes or forward slashes.

                Use the shell tool as needed to locate files or interact with the project.

                Leverage `analyze` through `return_last_only=true` subagents for deep codebase understanding with lean context
                - delegate analysis, retain summaries

                Your windows/screen tools can be used for visual debugging. You should not use these tools unless
                prompted to, but you can mention they are available if they are relevant.

                operating system: {os}
                current directory: {cwd}
                {container_info}
                "#,
                os=os,
                cwd=WORKING_DIR_PLACEHOLDER,
                container_info=if in_container { "container: true" } else { "" },
            },
            _ => {
                let shell_info = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

                formatdoc! {r#"
                The developer extension gives you the capabilities to edit code files and run shell commands,
                and can be used to solve a wide range of problems.

            You can use the shell tool to run any command that would work on the relevant operating system.
            Use the shell tool as needed to locate files or interact with the project.

            Leverage `analyze` through `return_last_only=true` subagents for deep codebase understanding with lean context
            - delegate analysis, retain summaries

            Your windows/screen tools can be used for visual debugging. You should not use these tools unless
            prompted to, but you can mention they are available if they are relevant.

            Always prefer ripgrep (rg -C 3) to grep.

            operating system: {os}
            current directory: {cwd}
            shell: {shell}
            {container_info}
                "#,
                os=os,
                cwd=WORKING_DIR_PLACEHOLDER,
                shell=shell_info,
                container_info=if in_container { "container: true" } else { "" },
                }
            }
        };

        let editor_description = {
            let editor_extra = if let Some(ref editor) = self.editor_model {
                format!("\n{}", editor.get_str_replace_description())
            } else {
                String::new()
            };
            formatdoc! {r#"

                Additional File Tool Instructions:

                Use `read_file` to view file contents or list directories. Use `view_range` for large files.

                Use `edit_file` to make surgical edits. Provide `old_str` and `new_str` for single replacements -
                `old_str` must match exactly one unique section of the file including whitespace.
                For multi-file edits, use the `diff` parameter with a unified diff format.

                Use `write_file` to create new files or fully overwrite existing ones. Include the complete file content.
                {editor_extra}
            "#}
        };

        // Create comprehensive shell tool instructions
        let common_shell_instructions = indoc! {r#"
            Additional Shell Tool Instructions:
            Execute a command in the shell.

            This will return the output and error concatenated into a single string, as
            you would see from running on the command line. There will also be an indication
            of if the command succeeded or failed.

            Avoid commands that produce a large amount of output, and consider piping those outputs to files.

            **Important**: Each shell command runs in its own process. Things like directory changes or
            sourcing files do not persist between tool calls. So you may need to repeat them each time by
            stringing together commands.

            If fetching web content, consider adding Accept: text/markdown header
        "#};

        let windows_specific = indoc! {r#"
            **Important**: For searching files and code:

            Preferred: Use ripgrep (`rg`) when available - it respects .gitignore and is fast:
              - To locate a file by name: `rg --files | rg example.py`
              - To locate content inside files: `rg 'class Example'`

            Alternative Windows commands (if ripgrep is not installed):
              - To locate a file by name: `dir /s /b example.py`
              - To locate content inside files: `findstr /s /i "class Example" *.py`

            Note: Alternative commands may show ignored/hidden files that should be excluded.

              - Multiple commands: Use && to chain commands, avoid newlines
              - Example: `cd example && dir` or `activate.bat && pip install numpy`

             **Important**: Use forward slashes in paths (e.g., `C:/Users/name`) to avoid
                 escape character issues with backslashes, i.e. \n in a path could be
                 mistaken for a newline.
        "#};

        let unix_specific = indoc! {r#"
            If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
            this tool does not run indefinitely.

            **Important**: Use ripgrep - `rg` - exclusively when you need to locate a file or a code reference,
            other solutions may produce too large output because of hidden files! For example *do not* use `find` or `ls -r`
              - List files by name: `rg --files | rg <filename>`
              - List files that contain a regex: `rg '<regex>' -l`

              - Multiple commands: Use && to chain commands, avoid newlines
              - Example: `cd example && ls` or `source env/bin/activate && pip install numpy`
        "#};

        let shell_tool_desc = match os {
            "windows" => format!("{}{}", common_shell_instructions, windows_specific),
            _ => format!("{}{}", common_shell_instructions, unix_specific),
        };

        let instructions = format!("{base_instructions}{editor_description}\n{shell_tool_desc}");

        ServerInfo {
            server_info: Implementation {
                name: "goose-developer".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            instructions: Some(instructions),
            ..Default::default()
        }
    }

    // TODO: use the rmcp prompt macros instead when SDK is updated
    // Current rmcp version 0.6.0 doesn't support prompt macros yet.
    // When upgrading to a newer version that supports it, replace this manual
    // implementation with the macro-based approach for better maintainability.
    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, ErrorData>> + Send + '_ {
        let prompts: Vec<Prompt> = self.prompts.values().cloned().collect();
        std::future::ready(Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
            meta: None,
        }))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, ErrorData>> + Send + '_ {
        let prompt_name = request.name;
        let arguments = request.arguments.unwrap_or_default();

        match self.prompts.get(&prompt_name) {
            Some(prompt) => {
                // Get the template from the prompt description
                let template = prompt.description.clone().unwrap_or_default();

                // Validate template length
                if template.len() > 10000 {
                    return std::future::ready(Err(ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        "Prompt template exceeds maximum allowed length".to_string(),
                        None,
                    )));
                }

                // Validate arguments for security (same checks as router)
                for (key, value) in &arguments {
                    // Check for empty or overly long keys/values
                    if key.is_empty() || key.len() > 1000 {
                        return std::future::ready(Err(ErrorData::new(
                            ErrorCode::INVALID_PARAMS,
                            "Argument keys must be between 1-1000 characters".to_string(),
                            None,
                        )));
                    }

                    let value_str = value.as_str().unwrap_or_default();
                    if value_str.len() > 1000 {
                        return std::future::ready(Err(ErrorData::new(
                            ErrorCode::INVALID_PARAMS,
                            "Argument values must not exceed 1000 characters".to_string(),
                            None,
                        )));
                    }

                    // Check for potentially dangerous patterns
                    let dangerous_patterns = ["../", "//", "\\\\", "<script>", "{{", "}}"];
                    for pattern in dangerous_patterns {
                        if key.contains(pattern) || value_str.contains(pattern) {
                            return std::future::ready(Err(ErrorData::new(
                                ErrorCode::INVALID_PARAMS,
                                format!(
                                    "Arguments contain potentially unsafe pattern: {}",
                                    pattern
                                ),
                                None,
                            )));
                        }
                    }
                }

                // Validate required arguments
                if let Some(args) = &prompt.arguments {
                    for arg in args {
                        if arg.required.unwrap_or(false)
                            && (!arguments.contains_key(&arg.name)
                                || arguments
                                    .get(&arg.name)
                                    .and_then(|v| v.as_str())
                                    .is_none_or(str::is_empty))
                        {
                            return std::future::ready(Err(ErrorData::new(
                                ErrorCode::INVALID_PARAMS,
                                format!("Missing required argument: '{}'", arg.name),
                                None,
                            )));
                        }
                    }
                }

                // Create a mutable copy of the template to fill in arguments
                let mut template_filled = template.clone();

                // Replace each argument placeholder with its value from the arguments object
                for (key, value) in &arguments {
                    let placeholder = format!("{{{}}}", key);
                    template_filled =
                        template_filled.replace(&placeholder, value.as_str().unwrap_or_default());
                }

                // Create prompt messages with the filled template
                let messages = vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    template_filled.clone(),
                )];

                let result = GetPromptResult {
                    description: Some(template_filled),
                    messages,
                };
                std::future::ready(Ok(result))
            }
            None => std::future::ready(Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Prompt '{}' not found", prompt_name),
                None,
            ))),
        }
    }

    /// Called when the client cancels a specific request.
    /// This method cancels the running process associated with the given request_id.
    #[allow(clippy::manual_async_fn)]
    fn on_cancelled(
        &self,
        notification: CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        async move {
            let request_id = notification.request_id.to_string();
            let processes = self.running_processes.read().await;

            if let Some(token) = processes.get(&request_id) {
                token.cancel();
                tracing::debug!("Found process for request {}, cancelling token", request_id);
            } else {
                tracing::warn!("No process found for request ID: {}", request_id);
            }
        }
    }
}

impl Default for DeveloperServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl DeveloperServer {
    pub fn new() -> Self {
        // Build ignore patterns (simplified version for this tool)
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let ignore_patterns = Self::build_ignore_patterns(&cwd);

        // Initialize editor model for AI-powered code editing
        let editor_model = create_editor_model();

        Self {
            tool_router: Self::tool_router(),
            ignore_patterns,
            editor_model,
            prompts: load_prompt_files(),
            code_analyzer: CodeAnalyzer::new(),
            running_processes: Arc::new(RwLock::new(HashMap::new())),
            extend_path_with_shell: false,
            bash_env_file: None,
        }
    }

    pub fn extend_path_with_shell(mut self, value: bool) -> Self {
        self.extend_path_with_shell = value;
        self
    }

    pub fn bash_env_file(mut self, value: Option<PathBuf>) -> Self {
        self.bash_env_file = value;
        self
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
            windows.into_iter().filter_map(|w| w.title().ok()).collect();

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

        let image = if let Some(window_title) = &params.window_title {
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
                .find(|w| w.title().is_ok_and(|t| &t == window_title))
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

        let dynamic_image = xcap::image::DynamicImage::ImageRgba8(image);
        let (bytes, mime_type) = Self::prepare_image_for_llm(dynamic_image)?;

        // Convert to base64
        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        // Return two Content objects like the old implementation:
        // one text for Assistant, one image with priority 0.0
        Ok(CallToolResult::success(vec![
            Content::text("Screenshot captured").with_audience(vec![Role::Assistant]),
            Content::image(data, &mime_type).with_priority(0.0),
        ]))
    }

    /// Read the contents of a file or list a directory.
    #[tool(
        name = "read_file",
        description = "Read the contents of a file or list a directory. Supports optional line range for viewing specific sections."
    )]
    pub async fn read_file(
        &self,
        params: Parameters<ReadFileParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = self.resolve_path(&params.path)?;

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

        let view_range = params.view_range.as_ref().and_then(|vr| {
            if vr.len() == 2 {
                Some((vr[0] as usize, vr[1]))
            } else {
                None
            }
        });
        let content = text_editor_view(&path, view_range).await?;
        Ok(CallToolResult::success(content))
    }

    /// Edit a file by replacing exact text or applying a unified diff.
    #[tool(
        name = "edit_file",
        description = "Edit a file by replacing exact text (old_str/new_str) or applying a unified diff. For old_str/new_str, the old text must match exactly once in the file."
    )]
    pub async fn edit_file(
        &self,
        params: Parameters<EditFileParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = self.resolve_path(&params.path)?;

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

        if let Some(ref diff) = params.diff {
            let content =
                text_editor_replace(&path, "", "", Some(diff), &self.editor_model).await?;
            Ok(CallToolResult::success(content))
        } else {
            let old_str = params.old_str.ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "Either 'diff' or both 'old_str' and 'new_str' must be provided".to_string(),
                    None,
                )
            })?;
            let new_str = params.new_str.ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "Missing 'new_str' parameter".to_string(),
                    None,
                )
            })?;
            let content =
                text_editor_replace(&path, &old_str, &new_str, None, &self.editor_model).await?;
            Ok(CallToolResult::success(content))
        }
    }

    /// Write content to a file, creating it if needed or overwriting if it exists.
    #[tool(
        name = "write_file",
        description = "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories."
    )]
    pub async fn write_file(
        &self,
        params: Parameters<WriteFileParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = self.resolve_path(&params.path)?;

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

        let content = text_editor_write(&path, &params.file_text).await?;
        Ok(CallToolResult::success(content))
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
        description = "Execute a command in the shell.This will return the output and error concatenated into a single string, as you would see from running on the command line. There will also be an indication of if the command succeeded or failed. Avoid commands that produce a large amount of output, and consider piping those outputs to files. If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that this tool does not run indefinitely."
    )]
    pub async fn shell(
        &self,
        params: Parameters<ShellParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let command = &params.command;
        let peer = context.peer;
        let request_id = context.id;

        let working_dir = extract_working_dir_from_meta(&context.meta);
        let session_id = extract_session_id_from_meta(&context.meta);

        // Validate the shell command
        self.validate_shell_command(command)?;

        let cancellation_token = CancellationToken::new();
        // Track the process using the request ID
        {
            let mut processes = self.running_processes.write().await;
            let request_id_str = request_id.to_string();
            processes.insert(request_id_str.clone(), cancellation_token.clone());
        }

        // Execute the command and capture output
        let output_result = self
            .execute_shell_command(
                command,
                &peer,
                cancellation_token.clone(),
                working_dir,
                session_id,
            )
            .await;

        // Clean up the process from tracking
        {
            let mut processes = self.running_processes.write().await;
            let request_id_str = request_id.to_string();
            let was_present = processes.remove(&request_id_str).is_some();
            if !was_present {
                tracing::warn!(
                    "Process for request_id {} was not in tracking map when trying to remove",
                    request_id
                );
            }
        }

        let output_str = output_result?;

        // Validate output size
        self.validate_shell_output_size(command, &output_str)?;

        // Process and format the output
        let (final_output, user_output) = self.process_shell_output(&output_str)?;

        Ok(CallToolResult::success(vec![
            Content::text(final_output).with_audience(vec![Role::Assistant]),
            Content::text(user_output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    /// Validate a shell command before execution.
    ///
    /// Checks for empty commands and ensures the command doesn't attempt to access
    /// files that are restricted by ignore patterns.
    fn validate_shell_command(&self, command: &str) -> Result<(), ErrorData> {
        // Check for empty commands
        if command.trim().is_empty() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Shell command cannot be empty".to_string(),
                None,
            ));
        }

        let cmd_parts: Vec<&str> = command.split_whitespace().collect();

        // Check if command arguments reference ignored files
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

        Ok(())
    }

    /// Execute a shell command and return the combined output.
    ///
    /// Streams output in real-time to the client using logging notifications.
    async fn execute_shell_command(
        &self,
        command: &str,
        peer: &rmcp::service::Peer<RoleServer>,
        cancellation_token: CancellationToken,
        working_dir: Option<PathBuf>,
        session_id: Option<String>,
    ) -> Result<String, ErrorData> {
        let mut shell_config = ShellConfig::default();
        let shell_name = std::path::Path::new(&shell_config.executable)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("bash");

        if let Some(ref env_file) = self.bash_env_file {
            if shell_name == "bash" {
                shell_config.envs.push((
                    OsString::from("BASH_ENV"),
                    env_file.clone().into_os_string(),
                ))
            }
        }

        if let Some(sid) = session_id {
            shell_config
                .envs
                .push((OsString::from("AGENT_SESSION_ID"), OsString::from(sid)));
        }

        let mut command = configure_shell_command(&shell_config, command, working_dir.as_deref());

        if self.extend_path_with_shell {
            if let Err(e) = get_shell_path_dirs()
                .await
                .and_then(|dirs| join_paths(dirs).map_err(|e| anyhow!(e)))
                .map(|path| command.env("PATH", path))
            {
                tracing::error!("Failed to extend PATH with shell directories: {}", e)
            }
        }

        let mut child = command
            .spawn()
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        let pid = child.id();
        if let Some(pid) = pid {
            tracing::debug!("Shell process spawned with PID: {}", pid);
        } else {
            tracing::warn!("Shell process spawned but PID not available");
        }

        // Stream the output and wait for completion with cancellation support
        let output_task = self.stream_shell_output(
            child.stdout.take().unwrap(),
            child.stderr.take().unwrap(),
            peer.clone(),
        );

        tokio::select! {
            output_result = output_task => {
                // Wait for the process to complete
                let _exit_status = child.wait().await.map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                output_result
            }
            _ = cancellation_token.cancelled() => {
                tracing::info!("Cancellation token triggered! Attempting to kill process and all child processes");

                // Kill the process and its children using platform-specific approach
                match kill_process_group(&mut child, pid).await {
                    Ok(_) => {
                        tracing::debug!("Successfully killed shell process and child processes");
                    }
                    Err(e) => {
                        tracing::error!("Failed to kill shell process and child processes: {}", e);
                    }
                }

                Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Shell command was cancelled by user".to_string(),
                    None,
                ))
            }
        }
    }

    /// Stream shell output in real-time and return the combined output.
    ///
    /// Merges stdout and stderr streams and sends each line as a logging notification.
    async fn stream_shell_output(
        &self,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
        peer: rmcp::service::Peer<RoleServer>,
    ) -> Result<String, ErrorData> {
        let stdout = BufReader::new(stdout);
        let stderr = BufReader::new(stderr);

        let output_task = tokio::spawn(async move {
            let mut combined_output = String::new();

            // Merge stdout and stderr streams
            // ref https://blog.yoshuawuyts.com/futures-concurrency-3
            let stdout = SplitStream::new(stdout.split(b'\n')).map(|v| ("stdout", v));
            let stderr = SplitStream::new(stderr.split(b'\n')).map(|v| ("stderr", v));
            let mut merged = stdout.merge(stderr);

            while let Some((stream_type, line)) = merged.next().await {
                let mut line = line?;
                // Re-add newline as clients expect it
                line.push(b'\n');
                // Convert to UTF-8 to avoid corrupted output
                let line_str = String::from_utf8_lossy(&line);

                combined_output.push_str(&line_str);

                // Stream each line back to the client in real-time
                let trimmed_line = line_str.trim();
                if !trimmed_line.is_empty() {
                    // Send the output line as a structured logging message
                    if let Err(e) = peer
                        .notify_logging_message(LoggingMessageNotificationParam {
                            level: LoggingLevel::Info,
                            data: serde_json::json!({
                                "type": "shell_output",
                                "stream": stream_type,
                                "output": trimmed_line
                            }),
                            logger: Some("shell_tool".to_string()),
                        })
                        .await
                    {
                        // Don't break execution if streaming fails, just log it
                        eprintln!("Failed to stream output line: {}", e);
                    }
                }
            }
            Ok::<_, std::io::Error>(combined_output)
        });

        match output_task.await {
            Ok(result) => {
                result.map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))
            }
            Err(e) => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                e.to_string(),
                None,
            )),
        }
    }

    /// Validate that shell output doesn't exceed size limits.
    fn validate_shell_output_size(&self, command: &str, output: &str) -> Result<(), ErrorData> {
        const MAX_CHAR_COUNT: usize = 400_000; // 400KB
        let char_count = output.chars().count();

        if char_count > MAX_CHAR_COUNT {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Shell output from command '{}' has too many characters ({}). Maximum character count is {}.",
                    command,
                    char_count,
                    MAX_CHAR_COUNT
                ),
                None,
            ));
        }

        Ok(())
    }

    /// Analyze code structure and relationships.
    ///
    /// Automatically selects the appropriate analysis:
    /// - Files: Semantic analysis with call graphs
    /// - Directories: Structure overview with metrics
    /// - With focus parameter: Track symbol across files
    ///
    /// Examples:
    /// analyze(path="file.py") -> semantic analysis
    /// analyze(path="src/") -> structure overview down to max_depth subdirs
    /// analyze(path="src/", focus="main") -> track main() across files in src/ down to max_depth subdirs
    #[tool(
        name = "analyze",
        description = "Analyze code structure in 3 modes: 1) Directory overview - file tree with LOC/function/class counts to max_depth. 2) File details - functions, classes, imports. 3) Symbol focus - call graphs across directory to max_depth (requires directory path, case-sensitive). Typical flow: directory → files → symbols. Functions called >3x show •N."
    )]
    pub async fn analyze(
        &self,
        params: Parameters<AnalyzeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = self.resolve_path(&params.path)?;
        self.code_analyzer
            .analyze(params, path, &self.ignore_patterns)
    }

    /// Process an image file from disk.
    ///
    /// The image will be:
    /// 1. Resized to max 1024px on either dimension while maintaining aspect ratio
    /// 2. Converted to JPEG format (85% quality)
    /// 3. Returned as base64 encoded data
    ///
    /// This allows processing image files for use in the conversation with optimized file sizes.
    #[tool(
        name = "image_processor",
        description = "Process an image file from disk. Resizes to max 1024px, converts to JPEG (85% quality), and returns as base64 data for optimized LLM consumption."
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

        let (bytes, mime_type) = Self::prepare_image_for_llm(image)?;

        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        Ok(CallToolResult::success(vec![
            Content::text(format!(
                "Successfully processed image from {}",
                path.display()
            ))
            .with_audience(vec![Role::Assistant]),
            Content::image(data, &mime_type).with_priority(0.0),
        ]))
    }

    fn prepare_image_for_llm(
        mut image: xcap::image::DynamicImage,
    ) -> Result<(Vec<u8>, String), ErrorData> {
        let max_dimension = 1024;
        let (width, height) = (image.width(), image.height());

        if width > max_dimension || height > max_dimension {
            let (new_width, new_height) = if width > height {
                let scale = max_dimension as f32 / width as f32;
                (max_dimension, (height as f32 * scale) as u32)
            } else {
                let scale = max_dimension as f32 / height as f32;
                ((width as f32 * scale) as u32, max_dimension)
            };

            image = xcap::image::DynamicImage::ImageRgba8(xcap::image::imageops::resize(
                &image,
                new_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            ));
        }

        let rgb_image = image.to_rgb8();
        let (img_width, img_height) = rgb_image.dimensions();

        let mut bytes: Vec<u8> = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);

        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 85)
            .encode(
                rgb_image.as_raw(),
                img_width,
                img_height,
                image::ColorType::Rgb8,
            )
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to encode image as JPEG: {}", e),
                    None,
                )
            })?;

        Ok((bytes, "image/jpeg".to_string()))
    }

    // Helper method to resolve and validate file paths
    fn resolve_path(&self, path_str: &str) -> Result<PathBuf, ErrorData> {
        let cwd = std::env::current_dir().expect("should have a current working dir");
        let expanded = expand_path(path_str);
        let path = Path::new(&expanded);

        // If the path is absolute, return it as-is
        if is_absolute_path(&expanded) {
            Ok(path.to_path_buf())
        } else {
            // For relative paths, resolve them relative to the current working directory
            Ok(cwd.join(path))
        }
    }

    fn build_ignore_patterns(cwd: &PathBuf) -> Gitignore {
        let mut builder = GitignoreBuilder::new(cwd);
        let local_ignore_path = cwd.join(".gooseignore");

        let global_ignore_path = etcetera::choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.config_dir().join(".gooseignore"))
            .ok();

        let has_local_ignore = local_ignore_path.is_file();
        let has_global_ignore = global_ignore_path
            .as_ref()
            .map(|p| p.is_file())
            .unwrap_or(false);

        // If no ignore file exists, apply default patterns in memory without writing to disk
        if !has_local_ignore && !has_global_ignore {
            for pattern in DEFAULT_GOOSEIGNORE_CONTENT.lines() {
                let trimmed = pattern.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let _ = builder.add_line(None, trimmed);
            }
        }

        if has_global_ignore {
            let _ = builder.add(global_ignore_path.as_ref().unwrap());
        }

        if has_local_ignore {
            let _ = builder.add(&local_ignore_path);
        }

        builder.build().expect("Failed to build ignore patterns")
    }

    // Helper method to check if a path should be ignored
    fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_patterns.matched(path, false).is_ignore()
    }

    // Only returns true when 100% certain (checks /proc/1/cgroup for container markers)
    fn is_definitely_container() -> bool {
        let Ok(content) = std::fs::read_to_string("/proc/1/cgroup") else {
            // If the file doesn't exist, we're definitely not in a Linux container
            return false;
        };

        // Check for definitive container markers in cgroup paths
        for line in content.lines() {
            if line.contains("/docker/")
                || line.contains("/docker-")
                || line.contains("/kubepods/")
                || line.contains("/libpod-")
                || line.contains("/lxc/")
                || line.contains("/containerd/")
            {
                return true;
            }
        }

        // Check for cgroups v2 unified hierarchy in containers
        // In Docker with cgroups v2, we typically see just "0::/"
        // This is a strong signal when it's the only line
        if content.trim() == "0::/" {
            return true;
        }

        false
    }

    // Helper function to handle Mac screenshot filenames that contain U+202F (narrow no-break space)
    fn normalize_mac_screenshot_path(&self, path: &Path) -> PathBuf {
        // Only process if the path has a filename
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            // Check if this matches Mac screenshot pattern:
            // "Screenshot YYYY-MM-DD at H.MM.SS AM/PM.png"
            if let Some(captures) = MACOS_SCREENSHOT_FILENAME_RE.captures(filename) {
                // Get the AM/PM part
                let meridian = captures.get(1).unwrap().as_str();

                // Find the last space before AM/PM and replace it with U+202F
                let space_pos = filename
                    .rfind(meridian)
                    .and_then(|pos| filename.get(..pos).map(|s| s.trim_end().len()))
                    .unwrap_or(0);

                if space_pos > 0 {
                    let parent = path.parent().unwrap_or(Path::new(""));
                    if let (Some(before), Some(after)) =
                        (filename.get(..space_pos), filename.get(space_pos + 1..))
                    {
                        let new_filename = format!("{}{}{}", before, '\u{202F}', after);
                        let new_path = parent.join(new_filename);

                        return new_path;
                    }
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
    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::{CancelledNotificationParam, NumberOrString};
    use rmcp::service::{serve_directly, NotificationContext};
    use rmcp::ServerHandler;
    use serial_test::serial;
    use std::{
        fs,
        time::{Duration, Instant},
    };
    use tempfile::TempDir;
    use tokio::time::timeout;

    fn create_test_server() -> DeveloperServer {
        DeveloperServer::new()
    }

    /// Creates a test transport using in-memory streams instead of stdio
    /// This avoids the hanging issues caused by multiple tests competing for stdio
    fn create_test_transport() -> impl rmcp::transport::IntoTransport<
        RoleServer,
        std::io::Error,
        rmcp::transport::async_rw::TransportAdapterAsyncCombinedRW,
    > {
        let (_client, server) = tokio::io::duplex(1024);
        server
    }

    /// Helper function to run shell tests with proper runtime management
    /// This ensures clean shutdown and prevents hanging tests
    fn run_shell_test<F, Fut, T>(test_fn: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        // Create a separate runtime for this test to ensure clean shutdown
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(test_fn());

        // Force shutdown the runtime to kill ALL spawned tasks
        // This terminates the fire-and-forget tasks that rmcp doesn't track
        rt.shutdown_timeout(std::time::Duration::from_millis(100));

        // Return the test result
        result
    }

    /// Helper function to clean up test services and prevent hanging tests
    /// This should be called at the end of tests that create running services
    fn cleanup_test_service(
        running_service: rmcp::service::RunningService<RoleServer, DeveloperServer>,
        peer: rmcp::service::Peer<RoleServer>,
    ) {
        let cancellation_token = running_service.cancellation_token();
        cancellation_token.cancel();
        drop(peer);
        drop(running_service);
    }

    #[test]
    #[serial]
    fn test_shell_missing_parameters() {
        run_shell_test(|| async {
            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Test directly on the server instead of using peer.call_tool
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.code, ErrorCode::INVALID_PARAMS);

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);
        });
    }

    #[test]
    #[serial]
    #[cfg(windows)]
    fn test_windows_specific_commands() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Test PowerShell command
            let shell_params = Parameters(ShellParams {
                command: "Get-ChildItem".to_string(),
            });

            let result = server
                .shell(
                    shell_params,
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_err());

            // Test that resolve_path works with Windows paths
            let windows_path = r"C:\Windows\System32";
            if Path::new(windows_path).exists() {
                let resolved = server.resolve_path(windows_path);
                assert!(resolved.is_ok());
            }

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_size_limits() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
        let server = create_test_server();

        // Test file size limit
        {
            let large_file_path = temp_dir.path().join("large.txt");

            // Create a file larger than 2MB
            let content = "x".repeat(3 * 1024 * 1024); // 3MB
            fs::write(&large_file_path, content).unwrap();

            let view_params = Parameters(ReadFileParams {
                path: large_file_path.to_str().unwrap().to_string(),
                view_range: None,
            });

            let result = server.read_file(view_params).await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
            assert!(err.to_string().contains("too large"));
        }

        // Test character count limit
        {
            let many_chars_path = temp_dir.path().join("many_chars.txt");

            // This is above MAX_FILE_SIZE
            let content = "x".repeat(500_000);
            fs::write(&many_chars_path, content).unwrap();

            let view_params = Parameters(ReadFileParams {
                path: many_chars_path.to_str().unwrap().to_string(),
                view_range: None,
            });

            let result = server.read_file(view_params).await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
            assert!(err.to_string().contains("is too large"));
        }
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
        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: "Hello, world!".to_string(),
        });

        server.write_file(write_params).await.unwrap();

        // View the file
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: None,
        });

        let view_result = server.read_file(view_params).await.unwrap();

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

        // The assistant-audience content must be extractable via as_text()
        let assistant_content = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .expect("view should return content with Assistant audience");
        assert!(
            assistant_content.as_text().is_some(),
            "assistant content must be RawContent::Text, not Resource"
        );
        assert!(assistant_content
            .as_text()
            .unwrap()
            .text
            .contains("Hello, world!"));
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
        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: "Hello, world!".to_string(),
        });

        server.write_file(write_params).await.unwrap();

        // Replace string
        let replace_params = Parameters(EditFileParams {
            path: file_path_str.to_string(),
            old_str: Some("world".to_string()),
            new_str: Some("Rust".to_string()),
            diff: None,
        });

        let replace_result = server.edit_file(replace_params).await.unwrap();

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

        assert!(assistant_content
            .text
            .contains("Successfully replaced text in"));

        // Verify the file contents changed
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Hello, Rust!"));
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
        let write_params = Parameters(WriteFileParams {
            path: secret_path.to_str().unwrap().to_string(),
            file_text: "test content".to_string(),
        });

        let result = server.write_file(write_params).await;
        assert!(
            result.is_err(),
            "Should not be able to write to ignored file"
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

        // Try to write to a non-ignored file
        let allowed_path = temp_dir.path().join("allowed.txt");
        let write_params = Parameters(WriteFileParams {
            path: allowed_path.to_str().unwrap().to_string(),
            file_text: "test content".to_string(),
        });

        let result = server.write_file(write_params).await;
        assert!(
            result.is_ok(),
            "Should be able to write to non-ignored file"
        );
    }

    #[test]
    #[serial]
    fn test_shell_respects_ignore_patterns() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Create an ignored file
            let secret_file_path = temp_dir.path().join("secrets.txt");
            fs::write(&secret_file_path, "secret content").unwrap();

            // try to cat the ignored file
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: format!("cat {}", secret_file_path.to_str().unwrap()),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_err(), "Should not be able to cat ignored file");
            assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

            // Try to cat a non-ignored file
            let allowed_file_path = temp_dir.path().join("allowed.txt");
            fs::write(&allowed_file_path, "allowed content").unwrap();

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: format!("cat {}", allowed_file_path.to_str().unwrap()),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "Should be able to cat non-ignored file");

            // Clean up
            let cancellation_token = running_service.cancellation_token();
            cancellation_token.cancel();
            drop(peer);
            drop(running_service);
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_descriptions() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        let server_info = server.get_info();
        let instructions = server_info.instructions.unwrap_or_default();

        assert!(instructions.contains("read_file"));
        assert!(instructions.contains("edit_file"));
        assert!(instructions.contains("write_file"));
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
        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: content.to_string(),
        });

        server.write_file(write_params).await.unwrap();

        // Test viewing specific range
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: Some(vec![3, 6]),
        });

        let view_result = server.read_file(view_params).await.unwrap();

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
        assert!(text.text.contains("3: Line 3"));
        assert!(text.text.contains("4: Line 4"));
        assert!(text.text.contains("5: Line 5"));
        assert!(text.text.contains("6: Line 6"));
        assert!(text.text.contains("(lines 3-6)"));
        // Should not contain other lines
        assert!(!text.text.contains("1: Line 1"));
        assert!(!text.text.contains("7: Line 7"));
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
        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: content.to_string(),
        });

        server.write_file(write_params).await.unwrap();

        // Test viewing from line 3 to end using -1
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: Some(vec![3, -1]),
        });

        let view_result = server.read_file(view_params).await.unwrap();

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
        assert!(text.text.contains("3: Line 3"));
        assert!(text.text.contains("4: Line 4"));
        assert!(text.text.contains("5: Line 5"));
        assert!(text.text.contains("(lines 3-end)"));
        // Should not contain lines 1-2
        assert!(!text.text.contains("1: Line 1"));
        assert!(!text.text.contains("2: Line 2"));
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
        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: content.to_string(),
        });

        server.write_file(write_params).await.unwrap();

        // Test invalid range - start line beyond file
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: Some(vec![10, 15]),
        });

        let result = server.read_file(view_params).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("beyond the end of the file"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_large_file_without_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with more than 2000 lines (LINE_READ_LIMIT)
        let mut content = String::new();
        for i in 1..=2001 {
            content.push_str(&format!("Line {}\n", i));
        }

        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: content,
        });

        server.write_file(write_params).await.unwrap();

        // Test viewing without view_range - should trigger the error
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: None,
        });

        let result = server.read_file(view_params).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
        assert!(err.message.contains("2001 lines long"));
        assert!(err
            .message
            .contains("recommended to read in with view_range"));
        assert!(err
            .message
            .contains("please pass in view_range with [1, 2001]"));

        // Test viewing with view_range - should work
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: Some(vec![1, 100]),
        });

        let result = server.read_file(view_params).await;
        assert!(result.is_ok());

        let view_result = result.unwrap();
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

        // Should contain lines 1-100
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("100: Line 100"));
        assert!(!text.text.contains("101: Line 101"));

        // Test viewing with explicit full range - should work
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: Some(vec![1, 2001]),
        });

        let result = server.read_file(view_params).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_file_with_exactly_2000_lines() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("file_2000.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with exactly 2000 lines (should not trigger the check)
        let mut content = String::new();
        for i in 1..=2000 {
            content.push_str(&format!("Line {}\n", i));
        }

        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: content,
        });

        server.write_file(write_params).await.unwrap();

        // Test viewing without view_range - should work since it's exactly 2000 lines
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: None,
        });

        let result = server.read_file(view_params).await;

        assert!(result.is_ok());
        let view_result = result.unwrap();
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

        // Should contain all lines
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("2000: Line 2000"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_small_file_without_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("small_file.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with less than 2000 lines
        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!("Line {}\n", i));
        }

        let write_params = Parameters(WriteFileParams {
            path: file_path_str.to_string(),
            file_text: content,
        });

        server.write_file(write_params).await.unwrap();

        // Test viewing without view_range - should work fine
        let view_params = Parameters(ReadFileParams {
            path: file_path_str.to_string(),
            view_range: None,
        });

        let result = server.read_file(view_params).await;

        assert!(result.is_ok());
        let view_result = result.unwrap();
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

        // Should contain all lines
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("100: Line 100"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Set the current directory before creating the server
        std::env::set_current_dir(temp_path).unwrap();

        // Create some test files and directories
        fs::create_dir(temp_path.join("subdir1")).unwrap();
        fs::create_dir(temp_path.join("subdir2")).unwrap();
        fs::create_dir(temp_path.join("another_dir")).unwrap();

        fs::write(temp_path.join("file1.txt"), "content1").unwrap();
        fs::write(temp_path.join("file2.rs"), "content2").unwrap();
        fs::write(temp_path.join("README.md"), "content3").unwrap();

        let server = create_test_server();

        // Test viewing a directory
        let result = server
            .read_file(Parameters(ReadFileParams {
                path: temp_path.to_str().unwrap().to_string(),
                view_range: None,
            }))
            .await;

        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert_eq!(content.len(), 1);

        // Check the content is a text message with directory listing
        let text_content = content[0].as_text().expect("Expected text content");
        let output = &text_content.text;

        // Check that it identifies as a directory
        assert!(output.contains("is a directory"));
        assert!(output.contains("Contents:"));

        // Check directories are listed with trailing slash
        assert!(output.contains("Directories:"));
        assert!(output.contains("another_dir/"));
        assert!(output.contains("subdir1/"));
        assert!(output.contains("subdir2/"));

        // Check files are listed
        assert!(output.contains("Files:"));
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.rs"));
        assert!(output.contains("README.md"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_directory_with_many_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Set the current directory before creating the server
        std::env::set_current_dir(temp_path).unwrap();

        // Create more than 50 files to test the limit
        for i in 0..60 {
            fs::write(
                temp_path.join(format!("file{:03}.txt", i)),
                format!("content{}", i),
            )
            .unwrap();
        }

        // Create some directories too
        for i in 0..10 {
            fs::create_dir(temp_path.join(format!("dir{:02}", i))).unwrap();
        }

        let server = create_test_server();

        let result = server
            .read_file(Parameters(ReadFileParams {
                path: temp_path.to_str().unwrap().to_string(),
                view_range: None,
            }))
            .await;

        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert_eq!(content.len(), 1);

        let text_content = content[0].as_text().expect("Expected text content");
        let output = &text_content.text;

        // Check that it shows the limit message
        assert!(output.contains("... and"));
        assert!(output.contains("more items"));
        assert!(output.contains("(showing first 50 items)"));

        // Count the actual number of items shown (should be 50)
        let dir_count = output.matches("/\n").count(); // directories end with /
        let file_count = output.matches(".txt\n").count(); // only counting .txt files for simplicity
        assert!(dir_count + file_count <= 50);
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Set the current directory before creating the server
        std::env::set_current_dir(temp_path).unwrap();

        let server = create_test_server();

        let gooseignore_path = temp_path.join(".gooseignore");
        if gooseignore_path.exists() {
            fs::remove_file(&gooseignore_path).unwrap();
        }

        let result = server
            .read_file(Parameters(ReadFileParams {
                path: temp_path.to_str().unwrap().to_string(),
                view_range: None,
            }))
            .await;

        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert_eq!(content.len(), 1);

        let text_content = content[0].as_text().expect("Expected text content");
        let output = &text_content.text;

        // Check that it shows empty directory message
        assert!(output.contains("is a directory"));
        assert!(output.contains("(empty directory)"));
    }

    #[test]
    #[serial]
    fn test_shell_output_truncation() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Create a command that generates > 100 lines of output
            let command = if cfg!(windows) {
                "for /L %i in (1,1,150) do @echo Line %i"
            } else {
                "for i in {1..150}; do echo \"Line $i\"; done"
            };

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: command.to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            // Should have two Content items
            assert_eq!(result.clone().unwrap().content.len(), 2);

            let content = result.clone().unwrap().content;

            // Find the Assistant and User content
            let assistant_content = content
                .iter()
                .find(|c| {
                    c.audience()
                        .is_some_and(|roles| roles.contains(&Role::Assistant))
                })
                .unwrap()
                .as_text()
                .unwrap();

            let user_content = content
                .iter()
                .find(|c| {
                    c.audience()
                        .is_some_and(|roles| roles.contains(&Role::User))
                })
                .unwrap()
                .as_text()
                .unwrap();

            // Assistant should get the full message with temp file info
            assert!(assistant_content
                .text
                .contains("private note: output was 150 lines"));

            // User should only get the truncated output with prefix
            assert!(user_content
                .text
                .starts_with("NOTE: Output was 150 lines, showing only the last 100 lines"));
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
                    let Some(path) = assistant_content.text.get(start_idx..end).map(|s| s.trim())
                    else {
                        panic!("Failed to extract path from assistant content");
                    };
                    println!("Extracted path: {}", path);

                    let file_contents =
                        std::fs::read_to_string(path).expect("Failed to read extracted temp file");

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

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);

            temp_dir.close().unwrap();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_process_shell_output_short() {
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

    #[tokio::test]
    #[serial]
    async fn test_process_shell_output_empty() {
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

    #[test]
    #[serial]
    fn test_shell_output_without_trailing_newline() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Test command that outputs content without a trailing newline
            let command = if cfg!(windows) {
                "echo|set /p=\"Content without newline\""
            } else {
                "printf 'Content without newline'"
            };

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: command.to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok());

            // Test the output processing logic that would be used by shell method
            let output_without_newline = "Content without newline";
            let result = server.process_shell_output(output_without_newline).unwrap();

            // The output should contain the content even without a trailing newline
            assert!(
                result.0.contains("Content without newline"),
                "Output should contain content even without trailing newline, but got: {}",
                result.0
            );
            assert!(
                result.1.contains("Content without newline"),
                "User output should contain content even without trailing newline, but got: {}",
                result.1
            );

            // Both should be the same for short output
            assert_eq!(result.0, output_without_newline);
            assert_eq!(result.1, output_without_newline);

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_output_handling_logic() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Test output truncation logic with content without trailing newlines
        let content_without_newline = "Content without newline";
        let result = server
            .process_shell_output(content_without_newline)
            .unwrap();

        assert_eq!(result.0, content_without_newline);
        assert_eq!(result.1, content_without_newline);
        assert!(
            result.0.contains("Content without newline"),
            "Output processing should preserve content without trailing newlines"
        );

        // Test with content that has trailing newlines
        let content_with_newline = "Content with newline\n";
        let result = server.process_shell_output(content_with_newline).unwrap();
        assert_eq!(result.0, content_with_newline);
        assert_eq!(result.1, content_with_newline);

        // Test empty output handling
        let empty_output = "";
        let result = server.process_shell_output(empty_output).unwrap();
        assert_eq!(result.0, "");
        assert_eq!(result.1, "");
    }

    #[tokio::test]
    #[serial]
    async fn test_default_patterns_when_no_ignore_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Don't create any ignore files
        let server = create_test_server();

        // Verify that .gooseignore is NOT created on disk (patterns applied in memory only)
        let gooseignore_path = temp_dir.path().join(".gooseignore");
        assert!(
            !gooseignore_path.exists(),
            ".gooseignore should NOT be created on disk"
        );

        // Default patterns should still be applied in memory
        assert!(
            server.is_ignored(Path::new(".env")),
            ".env should be ignored by default patterns"
        );
        assert!(
            server.is_ignored(Path::new(".env.local")),
            ".env.local should be ignored by default patterns"
        );
        assert!(
            server.is_ignored(Path::new("secrets.txt")),
            "secrets.txt should be ignored by default patterns"
        );
        assert!(
            !server.is_ignored(Path::new("normal.txt")),
            "normal.txt should not be ignored"
        );
    }

    #[test]
    #[serial]
    fn test_resolve_path_absolute() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();
        let absolute_path = temp_dir.path().join("test.txt");
        let absolute_path_str = absolute_path.to_str().unwrap();

        let resolved = server.resolve_path(absolute_path_str).unwrap();
        assert_eq!(resolved, absolute_path);
    }

    #[tokio::test]
    #[serial]
    async fn test_resolve_path_relative() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();
        let relative_path = "subdir/test.txt";

        let resolved = server.resolve_path(relative_path).unwrap();
        let expected = std::env::current_dir().unwrap().join("subdir/test.txt");
        assert_eq!(resolved, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_with_absolute_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();
        let absolute_path = temp_dir.path().join("absolute_test.txt");
        let absolute_path_str = absolute_path.to_str().unwrap();

        let write_params = Parameters(WriteFileParams {
            path: absolute_path_str.to_string(),
            file_text: "Absolute path test".to_string(),
        });

        let result = server.write_file(write_params).await;
        assert!(result.is_ok());

        let content = fs::read_to_string(&absolute_path).unwrap();
        assert_eq!(content.trim(), "Absolute path test");
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_with_relative_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();
        let relative_path = "relative_test.txt";

        let write_params = Parameters(WriteFileParams {
            path: relative_path.to_string(),
            file_text: "Relative path test".to_string(),
        });

        let result = server.write_file(write_params).await;
        assert!(result.is_ok());

        let absolute_path = temp_dir.path().join(relative_path);
        let content = fs::read_to_string(&absolute_path).unwrap();
        assert_eq!(content.trim(), "Relative path test");
    }

    #[test]
    #[serial]
    #[cfg(unix)] // Unix-specific test using sleep command
    fn test_shell_command_cancellation() {
        run_shell_test(|| async {
            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            let request_id = NumberOrString::Number(123);

            let context = RequestContext {
                ct: Default::default(),
                id: request_id.clone(),
                meta: Default::default(),
                extensions: Default::default(),
                peer: peer.clone(),
            };

            // Start a long-running shell command in the background
            let server_clone = server.clone();
            let shell_task = tokio::spawn(async move {
                server_clone
                    .shell(
                        Parameters(ShellParams {
                            command: "sleep 30".to_string(),
                        }),
                        context,
                    )
                    .await
            });

            // Give the command a moment to start
            tokio::time::sleep(Duration::from_millis(200)).await;

            // Verify the process is tracked
            {
                let processes = server.running_processes.read().await;
                assert!(processes.contains_key("123"), "Process should be tracked");
            }

            let start_time = Instant::now();

            // Cancel the command
            let cancel_params = CancelledNotificationParam {
                request_id,
                reason: Some("test cancellation".to_string()),
            };

            let notification_context = NotificationContext {
                peer: peer.clone(),
                meta: Default::default(),
                extensions: Default::default(),
            };

            server
                .on_cancelled(cancel_params, notification_context)
                .await;

            // Wait for the shell task to complete
            let result = timeout(Duration::from_secs(5), shell_task).await;
            let elapsed = start_time.elapsed();

            // Verify the task completed due to cancellation (not timeout)
            assert!(result.is_ok(), "Shell task should complete within timeout");
            let task_result = result.unwrap();
            assert!(task_result.is_ok(), "Shell task should not panic");

            // Verify the command was cancelled quickly (much less than 30 seconds)
            assert!(
                elapsed < Duration::from_secs(5),
                "Command should be cancelled quickly, took {:?}",
                elapsed
            );

            // Verify the process is no longer tracked
            {
                let processes = server.running_processes.read().await;
                assert!(
                    !processes.contains_key("123"),
                    "Process should be removed from tracking"
                );
            }

            cleanup_test_service(running_service, peer);
        });
    }

    #[test]
    #[serial]
    #[cfg(unix)] // Unix-specific test using shell commands
    fn test_child_process_cancellation() {
        run_shell_test(|| async {
            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            let request_id = NumberOrString::Number(456);

            let context = RequestContext {
                ct: Default::default(),
                id: request_id.clone(),
                meta: Default::default(),
                extensions: Default::default(),
                peer: peer.clone(),
            };

            // Start a command that spawns child processes
            let server_clone = server.clone();
            let shell_task = tokio::spawn(async move {
                server_clone
                    .shell(
                        Parameters(ShellParams {
                            command: "bash -c 'sleep 60 & wait'".to_string(),
                        }),
                        context,
                    )
                    .await
            });

            // Give the command time to start and spawn child processes
            tokio::time::sleep(Duration::from_millis(300)).await;

            let start_time = Instant::now();

            // Cancel the command
            let cancel_params = CancelledNotificationParam {
                request_id,
                reason: Some("test cancellation".to_string()),
            };

            let notification_context = NotificationContext {
                peer: peer.clone(),
                meta: Default::default(),
                extensions: Default::default(),
            };

            server
                .on_cancelled(cancel_params, notification_context)
                .await;

            // Wait for completion
            let result = timeout(Duration::from_secs(5), shell_task).await;
            let elapsed = start_time.elapsed();

            assert!(result.is_ok(), "Shell task should complete within timeout");
            assert!(
                elapsed < Duration::from_secs(5),
                "Command with child processes should be cancelled quickly, took {:?}",
                elapsed
            );

            cleanup_test_service(running_service, peer);
        });
    }

    #[test]
    #[serial]
    fn test_cancel_nonexistent_process() {
        run_shell_test(|| async {
            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Try to cancel a process that doesn't exist
            let cancel_params = CancelledNotificationParam {
                request_id: NumberOrString::Number(999),
                reason: Some("test cancellation".to_string()),
            };

            let notification_context = NotificationContext {
                peer: peer.clone(),
                meta: Default::default(),
                extensions: Default::default(),
            };

            // This should not panic or cause issues
            server
                .on_cancelled(cancel_params, notification_context)
                .await;

            // Verify no processes are tracked
            let processes = server.running_processes.read().await;
            assert!(processes.is_empty(), "No processes should be tracked");

            cleanup_test_service(running_service, peer);
        });
    }

    #[test]
    #[serial]
    #[cfg(unix)]
    fn test_successful_shell_command_completion() {
        run_shell_test(|| async {
            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            let context = RequestContext {
                ct: Default::default(),
                id: NumberOrString::Number(789),
                meta: Default::default(),
                extensions: Default::default(),
                peer: peer.clone(),
            };

            // Run a quick command that should complete successfully
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "echo 'Hello, World!'".to_string(),
                    }),
                    context,
                )
                .await;

            assert!(
                result.is_ok(),
                "Simple shell command should succeed: {:?}",
                result
            );

            // Verify no processes are left tracked after completion
            let processes = server.running_processes.read().await;
            assert!(
                !processes.contains_key("789"),
                "Process should be cleaned up after completion"
            );

            cleanup_test_service(running_service, peer);
        });
    }
}
