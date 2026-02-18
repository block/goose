use crate::subprocess::SubprocessExt;
use base64::Engine;
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::{formatdoc, indoc};
use reqwest::{Client, Url};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        AnnotateAble, CallToolResult, Content, ErrorCode, ErrorData, Implementation,
        ListResourcesResult, PaginatedRequestParams, RawResource, ReadResourceRequestParams,
        ReadResourceResult, Resource, ResourceContents, Role, ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc, sync::Mutex};
use tokio::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

mod docx_tool;
mod pdf_tool;
mod xlsx_tool;

mod platform;
use platform::{create_system_automation, SystemAutomation};

/// Enum for save_as parameter in web_scrape tool
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum SaveAsFormat {
    /// Save as text (for HTML pages)
    #[default]
    Text,
    /// Save as JSON (for API responses)
    Json,
    /// Save as binary (for images and other files)
    Binary,
}

/// Parameters for the web_scrape tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WebScrapeParams {
    /// The URL to fetch content from
    pub url: String,
    /// Format of the response.
    #[serde(default)]
    pub save_as: SaveAsFormat,
}

/// Enum for language parameter in automation_script tool
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLanguage {
    /// Shell/Bash script
    Shell,
    /// Batch script (Windows)
    Batch,
    /// Ruby script
    Ruby,
    /// PowerShell script
    Powershell,
}

/// Enum for command parameter in cache tool
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CacheCommand {
    /// List all cached files
    List,
    /// View content of a cached file
    View,
    /// Delete a cached file
    Delete,
    /// Clear all cached files
    Clear,
}

/// Parameters for the automation_script tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AutomationScriptParams {
    /// The scripting language to use
    #[serde(rename = "language")]
    pub language: ScriptLanguage,
    /// The script content
    pub script: String,
    /// Whether to save the script output to a file
    #[serde(default)]
    pub save_output: bool,
}

/// Parameters for the computer_control tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ComputerControlParams {
    /// The automation script content (PowerShell for Windows, AppleScript for macOS, shell for Linux)
    pub script: String,
    /// Whether to save the script output to a file
    #[serde(default)]
    pub save_output: bool,
}

/// Parameters for the cache tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CacheParams {
    /// The command to perform
    pub command: CacheCommand,
    /// Path to the cached file for view/delete commands
    pub path: Option<String>,
}

/// Parameters for the pdf_tool
/// Enum for operation parameter in pdf_tool
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PdfOperation {
    /// Extract all text content from the PDF
    ExtractText,
    /// Extract and save embedded images to PNG files
    ExtractImages,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PdfToolParams {
    /// Path to the PDF file
    pub path: String,
    /// Operation to perform on the PDF
    pub operation: PdfOperation,
}

/// Enum for operation parameter in docx_tool
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DocxOperation {
    /// Extract all text content and structure from the DOCX
    ExtractText,
    /// Create a new DOCX or update existing one with provided content
    UpdateDoc,
}

/// Enum for update mode in docx_tool params
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum DocxUpdateMode {
    /// Add content to end of document (default)
    #[default]
    Append,
    /// Replace specific text with new content
    Replace,
    /// Add content with specific heading level and styling
    Structured,
    /// Add an image to the document (with optional caption)
    AddImage,
}

/// Enum for text alignment in docx_tool params
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TextAlignment {
    /// Left alignment
    Left,
    /// Center alignment
    Center,
    /// Right alignment
    Right,
    /// Justified alignment
    Justified,
}

/// Styling options for text in docx_tool
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct DocxTextStyle {
    /// Make text bold
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    /// Make text italic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Make text underlined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    /// Font size in points
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    /// Text color in hex format (e.g., 'FF0000' for red)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Text alignment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<TextAlignment>,
}

/// Additional parameters for update_doc operation
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
pub struct DocxUpdateParams {
    /// Update mode (default: append)
    #[serde(default)]
    pub mode: DocxUpdateMode,
    /// Text to replace (required for replace mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_text: Option<String>,
    /// Heading level for structured mode (e.g., 'Heading1', 'Heading2')
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Path to the image file (required for add_image mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    /// Image width in pixels (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Image height in pixels (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Styling options for the text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<DocxTextStyle>,
}

/// Parameters for the docx_tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DocxToolParams {
    /// Path to the DOCX file
    pub path: String,
    /// Operation to perform on the DOCX
    pub operation: DocxOperation,
    /// Content to write (required for update_doc operation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Additional parameters for update_doc operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<DocxUpdateParams>,
}

/// Parameters for the xlsx_tool
/// Enum for operation parameter in xlsx_tool
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum XlsxOperation {
    /// List all worksheets in the workbook
    ListWorksheets,
    /// Get column names from a worksheet
    GetColumns,
    /// Get values and formulas from a cell range
    GetRange,
    /// Search for text in a worksheet
    FindText,
    /// Update a single cell's value
    UpdateCell,
    /// Get value and formula from a specific cell
    GetCell,
    /// Save changes back to the file
    Save,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct XlsxToolParams {
    /// Path to the XLSX file
    pub path: String,
    /// Operation to perform on the XLSX file
    pub operation: XlsxOperation,
    /// Worksheet name (if not provided, uses first worksheet)
    pub worksheet: Option<String>,
    /// Cell range in A1 notation (e.g., 'A1:C10') for get_range operation
    pub range: Option<String>,
    /// Text to search for in find_text operation
    pub search_text: Option<String>,
    /// Whether search should be case-sensitive
    #[serde(default)]
    pub case_sensitive: bool,
    /// Row number for update_cell and get_cell operations
    pub row: Option<u64>,
    /// Column number for update_cell and get_cell operations
    pub col: Option<u64>,
    /// New value for update_cell operation
    pub value: Option<String>,
}

// ============================================================================
// Peekaboo CLI integration tools (macOS only, requires `brew install steipete/tap/peekaboo`)
// ============================================================================

/// Parameters for the peekaboo_see tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PeekabooSeeParams {
    /// Optional: target a specific app by name (e.g., "Safari", "Terminal")
    pub app: Option<String>,
    /// Optional: target a specific window by title
    pub window_title: Option<String>,
    /// Whether to capture the entire screen instead of a window (default: false)
    #[serde(default)]
    pub screen_mode: bool,
}

/// Parameters for the peekaboo_click tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PeekabooClickParams {
    /// Element ID from peekaboo_see output (e.g., "B1", "T2", "L3")
    pub on: Option<String>,
    /// Click at specific coordinates as "x,y" (e.g., "100,200")
    pub coords: Option<String>,
    /// Whether to double-click instead of single click
    #[serde(default)]
    pub double: bool,
    /// Whether to right-click instead of left-click
    #[serde(default)]
    pub right: bool,
}

/// Parameters for the peekaboo_type tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PeekabooTypeParams {
    /// The text to type
    pub text: String,
    /// Whether to clear the field before typing
    #[serde(default)]
    pub clear: bool,
}

/// Enum for peekaboo_app subcommands
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PeekabooAppAction {
    /// Launch an application
    Launch,
    /// Quit an application
    Quit,
    /// Switch to an application
    Switch,
    /// List running applications
    List,
}

/// Parameters for the peekaboo_app tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PeekabooAppParams {
    /// The action to perform
    pub action: PeekabooAppAction,
    /// The app name or bundle identifier (not required for 'list')
    pub app: Option<String>,
    /// Optional: URL to open when launching (e.g., "https://google.com")
    pub open_url: Option<String>,
}

/// Parameters for the peekaboo_hotkey tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PeekabooHotkeyParams {
    /// Comma-separated key combo (e.g., "cmd,shift,t" or "cmd,c")
    pub combo: String,
}

/// ComputerController MCP Server using official RMCP SDK
#[derive(Clone)]
pub struct ComputerControllerServer {
    tool_router: ToolRouter<Self>,
    cache_dir: PathBuf,
    active_resources: Arc<Mutex<HashMap<String, ResourceContents>>>,
    http_client: Client,
    instructions: String,
    system_automation: Arc<Box<dyn SystemAutomation + Send + Sync>>,
}

impl Default for ComputerControllerServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl ComputerControllerServer {
    pub fn new() -> Self {
        // choose_app_strategy().cache_dir()
        // - macOS/Linux: ~/.cache/goose/computer_controller/
        // - Windows:     ~\AppData\Local\Block\goose\cache\computer_controller\
        // keep previous behavior of defaulting to /tmp/
        let cache_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_cache_dir("computer_controller"))
            .unwrap_or_else(|_| create_system_automation().get_temp_path());

        fs::create_dir_all(&cache_dir).unwrap_or_else(|_| {
            println!(
                "Warning: Failed to create cache directory at {:?}",
                cache_dir
            )
        });

        let system_automation: Arc<Box<dyn SystemAutomation + Send + Sync>> =
            Arc::new(create_system_automation());

        let os_specific_instructions = match std::env::consts::OS {
            "windows" => indoc! {r#"
            Here are some extra tools:
            automation_script
              - Create and run PowerShell or Batch scripts
              - PowerShell is recommended for most tasks
              - Scripts can save their output to files
              - Windows-specific features:
                - PowerShell for system automation and UI control
                - Windows Management Instrumentation (WMI)
                - Registry access and system settings
              - Use the screenshot tool if needed to help with tasks

            computer_control
              - System automation using PowerShell
              - Consider the screenshot tool to work out what is on screen and what to do to help with the control task.
            "#},
            "macos" => indoc! {r#"
            Here are some extra tools:
            automation_script
              - Create and run Shell and Ruby scripts
              - Shell (bash) is recommended for most tasks
              - Scripts can save their output to files
              - macOS-specific features:
                - Use shell scripting or AppleScript (via osascript) for app scripting and system settings
                - Integration with macOS apps and services

            Peekaboo UI Automation (requires: brew install steipete/tap/peekaboo):
              Use peekaboo tools for all visual UI interaction on macOS.

              The core workflow is see → click → type:
              1. peekaboo_see: Capture annotated screenshot with element IDs (B1=button, T2=text field, L3=link)
              2. peekaboo_click: Click on an element by its ID from peekaboo_see output
              3. peekaboo_type: Type text into the focused field

              Additional tools:
              - peekaboo_hotkey: Press keyboard shortcuts (e.g., "cmd,c" for copy)
              - peekaboo_app: Launch, quit, switch apps, or list running apps

              For non-visual tasks (app scripting, system settings, file operations),
              use automation_script with shell or AppleScript via osascript.
            "#},
            _ => indoc! {r#"
            Here are some extra tools:
            automation_script
              - Create and run Shell scripts
              - Shell (bash) is recommended for most tasks
              - Scripts can save their output to files
              - Linux-specific features:
                - System automation through shell scripting
                - X11/Wayland window management
                - D-Bus system services integration
                - Desktop environment control
              - Use the screenshot tool if needed to help with tasks

            computer_control
              - System automation using shell commands and system tools
              - Desktop environment automation (GNOME, KDE, etc.)
              - Consider the screenshot tool to work out what is on screen and what to do to help with the control task.

            When you need to interact with websites or web applications, consider using tools like xdotool or wmctrl for:
              - Window management
              - Simulating keyboard/mouse input
              - Automating UI interactions
              - Desktop environment control
            "#},
        };

        let instructions = formatdoc! {r#"
            You are a helpful assistant to a power user who is not a professional developer, but you may use development tools to help assist them.
            The user may not know how to break down tasks, so you will need to ensure that you do, and run things in batches as needed.
            The ComputerControllerExtension helps you with common tasks like web scraping,
            data processing, and automation without requiring programming expertise.

            You can use scripting as needed to work with text files of data, such as csvs, json, or text files etc.
            Using the developer extension is allowed for more sophisticated tasks or instructed to (js or py can be helpful for more complex tasks if tools are available).

            Accessing web sites, even apis, may be common (you can use scripting to do this) without troubling them too much (they won't know what limits are).
            Try to do your best to find ways to complete a task without too many questions or offering options unless it is really unclear, find a way if you can.
            You can also guide them steps if they can help out as you go along.

            There is already a screenshot tool available you can use if needed to see what is on screen.

            {os_instructions}

            web_scrape
              - Fetch content from html websites and APIs
              - Save as text, JSON, or binary files
              - Content is cached locally for later use
              - This is not optimised for complex websites, so don't use this as the first tool.
            cache
              - Manage your cached files
              - List, view, delete files
              - Clear all cached data
            The extension automatically manages:
            - Cache directory: {cache_dir}
            - File organization and cleanup
            "#,
            os_instructions = os_specific_instructions,
            cache_dir = cache_dir.display()
        };

        Self {
            tool_router: Self::tool_router(),
            cache_dir,
            active_resources: Arc::new(Mutex::new(HashMap::new())),
            http_client: Client::builder().user_agent("goose/1.0").build().unwrap(),
            instructions,
            system_automation,
        }
    }

    // Helper function to generate a cache file path
    fn get_cache_path(&self, prefix: &str, extension: &str) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        self.cache_dir
            .join(format!("{}_{}.{}", prefix, timestamp, extension))
    }

    // Helper function to save content to cache
    async fn save_to_cache(
        &self,
        content: &[u8],
        prefix: &str,
        extension: &str,
    ) -> Result<PathBuf, ErrorData> {
        let cache_path = self.get_cache_path(prefix, extension);
        fs::write(&cache_path, content).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to write to cache: {}", e),
                None,
            )
        })?;
        Ok(cache_path)
    }

    // Helper function to register a file as a resource
    fn register_as_resource(&self, cache_path: &PathBuf, mime_type: &str) -> Result<(), ErrorData> {
        let uri = Url::from_file_path(cache_path)
            .map_err(|_| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Invalid cache path".to_string(),
                    None,
                )
            })?
            .to_string();

        let resource = ResourceContents::TextResourceContents {
            uri: uri.clone(),
            text: String::new(), // We'll read it when needed
            mime_type: Some(mime_type.to_string()),
            meta: None,
        };

        self.active_resources.lock().unwrap().insert(uri, resource);
        Ok(())
    }

    /// Fetch and save content from a web page
    #[tool(
        name = "web_scrape",
        description = "
            Fetch and save content from a web page. The content can be saved as:
            - text (for HTML pages)
            - json (for API responses)
            - binary (for images and other files)
            Returns 'Content saved to: <path>'. Use cache to read the content.
        "
    )]
    pub async fn web_scrape(
        &self,
        params: Parameters<WebScrapeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let url = &params.url;
        let save_as = params.save_as;

        // Fetch the content
        let response = self
            .http_client
            .get(url)
            .header("Accept", "text/markdown, */*")
            .send()
            .await
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to fetch URL: {}", e),
                    None,
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("HTTP request failed with status: {}", status),
                None,
            ));
        }

        // Process based on save_as parameter
        let (content, extension, mime_type) = match save_as {
            SaveAsFormat::Text => {
                let text = response.text().await.map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to get text: {}", e),
                        None,
                    )
                })?;
                (text.into_bytes(), "txt", "text/plain")
            }
            SaveAsFormat::Json => {
                let text = response.text().await.map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to get text: {}", e),
                        None,
                    )
                })?;
                // Verify it's valid JSON
                serde_json::from_str::<serde_json::Value>(&text).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Invalid JSON response: {}", e),
                        None,
                    )
                })?;
                (text.into_bytes(), "json", "application/json")
            }
            SaveAsFormat::Binary => {
                let bytes = response.bytes().await.map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to get bytes: {}", e),
                        None,
                    )
                })?;
                (bytes.to_vec(), "bin", "application/octet-stream")
            }
        };

        // Save to cache
        let cache_path = self.save_to_cache(&content, "web", extension).await?;

        // Register as a resource
        self.register_as_resource(&cache_path, mime_type)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Content saved to: {}",
            cache_path.display()
        ))]))
    }

    /// Create and run small scripts for automation tasks
    #[cfg(target_os = "windows")]
    #[tool(
        name = "automation_script",
        description = "
            Create and run small PowerShell or Batch scripts for automation tasks.
            PowerShell is recommended for most tasks.

            The script is saved to a temporary file and executed.
            Some examples:
            - Sort unique lines: Get-Content file.txt | Sort-Object -Unique
            - Extract CSV column: Import-Csv file.csv | Select-Object -ExpandProperty Column2
            - Find text: Select-String -Pattern 'pattern' -Path file.txt
        "
    )]
    pub async fn automation_script(
        &self,
        params: Parameters<AutomationScriptParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.automation_script_impl(params).await
    }

    /// Create and run small scripts for automation tasks
    #[cfg(not(target_os = "windows"))]
    #[tool(
        name = "automation_script",
        description = "
            Create and run small scripts for automation tasks.
            Supports Shell and Ruby (on macOS).

            The script is saved to a temporary file and executed.
            Consider using shell script (bash) for most simple tasks first.
            Ruby is useful for text processing or when you need more sophisticated scripting capabilities.
            Some examples of shell:
                - create a sorted list of unique lines: sort file.txt | uniq
                - extract 2nd column in csv: awk -F ',' '{ print $2}'
                - pattern matching: grep pattern file.txt
        "
    )]
    pub async fn automation_script(
        &self,
        params: Parameters<AutomationScriptParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.automation_script_impl(params).await
    }

    #[allow(clippy::too_many_lines)]
    async fn automation_script_impl(
        &self,
        params: Parameters<AutomationScriptParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let language = params.language;
        let script = &params.script;
        let save_output = params.save_output;

        // Create a temporary directory for the script
        let script_dir = tempfile::tempdir().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create temporary directory: {}", e),
                None,
            )
        })?;

        let (shell, shell_arg) = self.system_automation.get_shell_command();

        let command = match language {
            ScriptLanguage::Shell | ScriptLanguage::Batch => {
                let script_path = script_dir.path().join(format!(
                    "script.{}",
                    if cfg!(windows) { "bat" } else { "sh" }
                ));
                fs::write(&script_path, script).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write script: {}", e),
                        None,
                    )
                })?;

                // Set execute permissions on Unix systems
                #[cfg(unix)]
                {
                    let mut perms = fs::metadata(&script_path)
                        .map_err(|e| {
                            ErrorData::new(
                                ErrorCode::INTERNAL_ERROR,
                                format!("Failed to get file metadata: {}", e),
                                None,
                            )
                        })?
                        .permissions();
                    perms.set_mode(0o755); // rwxr-xr-x
                    fs::set_permissions(&script_path, perms).map_err(|e| {
                        ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            format!("Failed to set execute permissions: {}", e),
                            None,
                        )
                    })?;
                }

                script_path.display().to_string()
            }
            ScriptLanguage::Ruby => {
                let script_path = script_dir.path().join("script.rb");
                fs::write(&script_path, script).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write script: {}", e),
                        None,
                    )
                })?;

                format!("ruby {}", script_path.display())
            }
            ScriptLanguage::Powershell => {
                let script_path = script_dir.path().join("script.ps1");
                fs::write(&script_path, script).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write script: {}", e),
                        None,
                    )
                })?;

                script_path.display().to_string()
            }
        };

        // Run the script
        let output = match language {
            ScriptLanguage::Powershell => {
                // For PowerShell, we need to use -File instead of -Command
                Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-NonInteractive")
                    .arg("-File")
                    .arg(&command)
                    .env("GOOSE_TERMINAL", "1")
                    .env("AGENT", "goose")
                    .set_no_window()
                    .output()
                    .await
                    .map_err(|e| {
                        ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            format!("Failed to run script: {}", e),
                            None,
                        )
                    })?
            }
            _ => Command::new(shell)
                .arg(shell_arg)
                .arg(&command)
                .env("GOOSE_TERMINAL", "1")
                .env("AGENT", "goose")
                .set_no_window()
                .output()
                .await
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to run script: {}", e),
                        None,
                    )
                })?,
        };

        let output_str = String::from_utf8_lossy(&output.stdout).into_owned();
        let error_str = String::from_utf8_lossy(&output.stderr).into_owned();

        let mut result = if output.status.success() {
            format!("Script completed successfully.\n\nOutput:\n{}", output_str)
        } else {
            format!(
                "Script failed with error code {}.\n\nError:\n{}\nOutput:\n{}",
                output.status, error_str, output_str
            )
        };

        // Save output if requested
        if save_output && !output_str.is_empty() {
            let cache_path = self
                .save_to_cache(output_str.as_bytes(), "script_output", "txt")
                .await?;
            result.push_str(&format!("\n\nOutput saved to: {}", cache_path.display()));

            // Register as a resource
            self.register_as_resource(&cache_path, "text")?;
        }

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// On macOS, Peekaboo tools replace computer_control for UI automation.
    /// This is kept only for running AppleScript for non-UI tasks (app scripting, system settings).
    /// Prefer peekaboo_see/peekaboo_click/peekaboo_type for anything visual.
    #[cfg(target_os = "macos")]
    #[tool(
        name = "computer_control",
        description = "Run an AppleScript snippet via osascript for non-UI tasks like app scripting, system settings, or querying application state. Do NOT use this for clicking, typing, or visual UI interaction — use the peekaboo_see/peekaboo_click/peekaboo_type tools instead."
    )]
    pub async fn computer_control(
        &self,
        params: Parameters<ComputerControlParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.computer_control_impl(params).await
    }

    /// Control the computer using system automation (Windows, Linux, other)
    #[cfg(not(target_os = "macos"))]
    #[tool(
        name = "computer_control",
        description = "Control the computer using system automation. On Windows uses PowerShell, on Linux uses shell commands and system tools. Can be combined with screenshot tool for visual task assistance."
    )]
    pub async fn computer_control(
        &self,
        params: Parameters<ComputerControlParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.computer_control_impl(params).await
    }

    async fn computer_control_impl(
        &self,
        params: Parameters<ComputerControlParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let script = &params.script;
        let save_output = params.save_output;

        // Use platform-specific automation
        let output = self
            .system_automation
            .execute_system_script(script)
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to execute script: {}", e),
                    None,
                )
            })?;

        let mut result = format!("Script completed successfully.\n\nOutput:\n{}", output);

        // Save output if requested
        if save_output && !output.is_empty() {
            let cache_path = self
                .save_to_cache(output.as_bytes(), "automation_output", "txt")
                .await?;
            result.push_str(&format!("\n\nOutput saved to: {}", cache_path.display()));

            // Register as a resource
            self.register_as_resource(&cache_path, "text")?;
        }

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Process Excel (XLSX) files to read and manipulate spreadsheet data
    #[tool(
        name = "xlsx_tool",
        description = "
            Process Excel (XLSX) files to read and manipulate spreadsheet data.
            Supports operations:
            - list_worksheets: List all worksheets in the workbook (returns name, index, column_count, row_count)
            - get_columns: Get column names from a worksheet (returns values from the first row)
            - get_range: Get values and formulas from a cell range (e.g., 'A1:C10') (returns a 2D array organized as [row][column])
            - find_text: Search for text in a worksheet (returns a list of (row, column) coordinates)
            - update_cell: Update a single cell's value (returns confirmation message)
            - get_cell: Get value and formula from a specific cell (returns both value and formula if present)
            - save: Save changes back to the file (returns confirmation message)

            Use this when working with Excel spreadsheets to analyze or modify data.
        "
    )]
    pub async fn xlsx_tool(
        &self,
        params: Parameters<XlsxToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = &params.path;
        let operation = params.operation;

        match operation {
            XlsxOperation::ListWorksheets => {
                let xlsx = xlsx_tool::XlsxTool::new(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                let worksheets = xlsx
                    .list_worksheets()
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "{:#?}",
                    worksheets
                ))]))
            }
            XlsxOperation::GetColumns => {
                let xlsx = xlsx_tool::XlsxTool::new(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                let worksheet = if let Some(name) = &params.worksheet {
                    xlsx.get_worksheet_by_name(name).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                } else {
                    xlsx.get_worksheet_by_index(0).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                };
                let columns = xlsx
                    .get_column_names(worksheet)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "{:#?}",
                    columns
                ))]))
            }
            XlsxOperation::GetRange => {
                let range = params.range.as_ref().ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'range' parameter".to_string(),
                        None,
                    )
                })?;

                let xlsx = xlsx_tool::XlsxTool::new(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                let worksheet = if let Some(name) = &params.worksheet {
                    xlsx.get_worksheet_by_name(name).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                } else {
                    xlsx.get_worksheet_by_index(0).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                };
                let range_data = xlsx
                    .get_range(worksheet, range)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "{:#?}",
                    range_data
                ))]))
            }
            XlsxOperation::FindText => {
                let search_text = params.search_text.as_ref().ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'search_text' parameter".to_string(),
                        None,
                    )
                })?;

                let case_sensitive = params.case_sensitive;

                let xlsx = xlsx_tool::XlsxTool::new(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                let worksheet = if let Some(name) = &params.worksheet {
                    xlsx.get_worksheet_by_name(name).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                } else {
                    xlsx.get_worksheet_by_index(0).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                };
                let matches = xlsx
                    .find_in_worksheet(worksheet, search_text, case_sensitive)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Found matches at: {:#?}",
                    matches
                ))]))
            }
            XlsxOperation::UpdateCell => {
                let row = params.row.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'row' parameter".to_string(),
                        None,
                    )
                })?;
                let col = params.col.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'col' parameter".to_string(),
                        None,
                    )
                })?;
                let value = params.value.as_ref().ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'value' parameter".to_string(),
                        None,
                    )
                })?;

                let worksheet_name = params.worksheet.as_deref().unwrap_or("Sheet1");

                let mut xlsx = xlsx_tool::XlsxTool::new(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                xlsx.update_cell(worksheet_name, row as u32, col as u32, value)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                xlsx.save(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Updated cell ({}, {}) to '{}' in worksheet '{}'",
                    row, col, value, worksheet_name
                ))]))
            }
            XlsxOperation::Save => {
                let xlsx = xlsx_tool::XlsxTool::new(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                xlsx.save(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(
                    "File saved successfully.",
                )]))
            }
            XlsxOperation::GetCell => {
                let row = params.row.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'row' parameter".to_string(),
                        None,
                    )
                })?;

                let col = params.col.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'col' parameter".to_string(),
                        None,
                    )
                })?;

                let xlsx = xlsx_tool::XlsxTool::new(path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                let worksheet = if let Some(name) = &params.worksheet {
                    xlsx.get_worksheet_by_name(name).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                } else {
                    xlsx.get_worksheet_by_index(0).map_err(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?
                };
                let cell_value = xlsx
                    .get_cell_value(worksheet, row as u32, col as u32)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "{:#?}",
                    cell_value
                ))]))
            }
        }
    }

    /// Process DOCX files to extract text and create/update documents
    #[tool(
        name = "docx_tool",
        description = "
            Process DOCX files to extract text and create/update documents.
            Supports operations:
            - extract_text: Extract all text content and structure (headings, TOC) from the DOCX
            - update_doc: Create a new DOCX or update existing one with provided content
              Modes:
              - append: Add content to end of document (default)
              - replace: Replace specific text with new content
              - structured: Add content with specific heading level and styling
              - add_image: Add an image to the document (with optional caption)

            Use this when there is a .docx file that needs to be processed or created.
        "
    )]
    pub async fn docx_tool(
        &self,
        params: Parameters<DocxToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = &params.path;
        let operation = params.operation;

        // Convert enum to string for the existing implementation
        let operation_str = match operation {
            DocxOperation::ExtractText => "extract_text",
            DocxOperation::UpdateDoc => "update_doc",
        };

        // Convert typed params back to JSON for the internal docx_tool impl
        let json_params = params
            .params
            .as_ref()
            .map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null));

        let result = crate::computercontroller::docx_tool::docx_tool(
            path,
            operation_str,
            params.content.as_deref(),
            json_params.as_ref(),
        )
        .await
        .map_err(|e| ErrorData::new(e.code, e.message, e.data))?;

        Ok(CallToolResult::success(result))
    }

    /// Process PDF files to extract text and images
    #[tool(
        name = "pdf_tool",
        description = "
            Process PDF files to extract text and images.
            Supports operations:
            - extract_text: Extract all text content from the PDF
            - extract_images: Extract and save embedded images to PNG files

            Use this when there is a .pdf file or files that need to be processed.
        "
    )]
    pub async fn pdf_tool(
        &self,
        params: Parameters<PdfToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let path = &params.path;
        let operation = params.operation;

        // Convert enum to string for the existing implementation
        let operation_str = match operation {
            PdfOperation::ExtractText => "extract_text",
            PdfOperation::ExtractImages => "extract_images",
        };

        let result =
            crate::computercontroller::pdf_tool::pdf_tool(path, operation_str, &self.cache_dir)
                .await
                .map_err(|e| ErrorData::new(e.code, e.message, e.data))?;

        Ok(CallToolResult::success(result))
    }

    /// Manage cached files and data
    #[tool(
        name = "cache",
        description = "
            Manage cached files and data:
            - list: List all cached files
            - view: View content of a cached file
            - delete: Delete a cached file
            - clear: Clear all cached files
        "
    )]
    pub async fn cache(
        &self,
        params: Parameters<CacheParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let command = params.0.command;
        let path = params.0.path.as_deref();

        match command {
            CacheCommand::List => {
                let mut files = Vec::new();
                for entry in fs::read_dir(&self.cache_dir).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to read cache directory: {}", e),
                        None,
                    )
                })? {
                    let entry = entry.map_err(|e| {
                        ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            format!("Failed to read directory entry: {}", e),
                            None,
                        )
                    })?;
                    files.push(format!("{}", entry.path().display()));
                }
                files.sort();
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Cached files:\n{}",
                    files.join("\n")
                ))]))
            }
            CacheCommand::View => {
                let path = path.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'path' parameter for view".to_string(),
                        None,
                    )
                })?;

                let content = fs::read_to_string(path).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to read file: {}", e),
                        None,
                    )
                })?;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Content of {}:\n\n{}",
                    path, content
                ))]))
            }
            CacheCommand::Delete => {
                let path = path.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'path' parameter for delete".to_string(),
                        None,
                    )
                })?;

                fs::remove_file(path).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to delete file: {}", e),
                        None,
                    )
                })?;

                // Remove from active resources if present
                if let Ok(url) = Url::from_file_path(path) {
                    self.active_resources
                        .lock()
                        .unwrap()
                        .remove(&url.to_string());
                }

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Deleted file: {}",
                    path
                ))]))
            }
            CacheCommand::Clear => {
                fs::remove_dir_all(&self.cache_dir).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to clear cache directory: {}", e),
                        None,
                    )
                })?;
                fs::create_dir_all(&self.cache_dir).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to recreate cache directory: {}", e),
                        None,
                    )
                })?;

                // Clear active resources
                self.active_resources.lock().unwrap().clear();

                Ok(CallToolResult::success(vec![Content::text(
                    "Cache cleared successfully.",
                )]))
            }
        }
    }

    // ========================================================================
    // Peekaboo CLI integration tools (macOS only)
    // These tools shell out to the `peekaboo` CLI binary for UI automation.
    // They are always registered but return install instructions if Peekaboo
    // is not found on PATH.
    // ========================================================================

    /// Helper: check if Peekaboo is available and return an install-prompt error if not
    fn require_peekaboo(&self) -> Result<(), ErrorData> {
        if !self.system_automation.has_peekaboo() {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Peekaboo is not installed. Install it with: brew install steipete/tap/peekaboo\n\
                 Peekaboo requires macOS 15+ (Sequoia) with Screen Recording and Accessibility permissions."
                    .to_string(),
                None,
            ));
        }
        Ok(())
    }

    /// Helper: run a peekaboo CLI command synchronously (blocking)
    fn run_peekaboo_cmd(&self, args: &[&str]) -> Result<String, ErrorData> {
        let output = std::process::Command::new("peekaboo")
            .args(args)
            .output()
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to run peekaboo: {}", e),
                    None,
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Peekaboo command failed (exit code {}):\n{}\n{}",
                    output.status, stderr, stdout
                ),
                None,
            ));
        }

        Ok(stdout)
    }

    /// Capture an annotated screenshot showing UI elements with IDs.
    /// This is the first step in the see → click → type workflow.
    /// Returns an annotated screenshot image and a list of UI elements with IDs
    /// (B1=button, T2=text field, L3=link, etc.) that can be used with
    /// peekaboo_click and peekaboo_type.
    #[tool(
        name = "peekaboo_see",
        description = "Capture an annotated screenshot of the macOS UI showing clickable elements with IDs. Returns an annotated image with overlaid element labels (B1=button, T2=text field, L3=link) plus a JSON element list. Use this first, then peekaboo_click/peekaboo_type to interact with elements by ID. Requires Peekaboo CLI (brew install steipete/tap/peekaboo)."
    )]
    pub async fn peekaboo_see(
        &self,
        params: Parameters<PeekabooSeeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.require_peekaboo()?;
        let params = params.0;

        // Build the output path for the annotated screenshot
        let screenshot_path = self.get_cache_path("peekaboo_see", "png");

        let mut args: Vec<&str> = vec!["see", "--annotate", "--json-output"];

        let path_str = screenshot_path.to_string_lossy().to_string();
        args.push("--path");
        args.push(&path_str);

        let app_str;
        if let Some(ref app) = params.app {
            app_str = app.clone();
            args.push("--app");
            args.push(&app_str);
        }

        let window_title_str;
        if let Some(ref title) = params.window_title {
            window_title_str = title.clone();
            args.push("--window-title");
            args.push(&window_title_str);
        }

        if params.screen_mode {
            args.push("--mode");
            args.push("screen");
        }

        // Run peekaboo see
        let json_output = self.run_peekaboo_cmd(&args)?;

        // Build the response: annotated image + element list text
        let mut contents = Vec::new();

        // Try to read the annotated screenshot and return it as base64 image
        // Look for the annotated version first (Peekaboo saves *_annotated.png)
        let annotated_path_str = path_str.replace(".png", "_annotated.png");
        let image_path = if std::path::Path::new(&annotated_path_str).exists() {
            PathBuf::from(&annotated_path_str)
        } else {
            screenshot_path.clone()
        };

        if image_path.exists() {
            if let Ok(image_bytes) = fs::read(&image_path) {
                let data = base64::prelude::BASE64_STANDARD.encode(&image_bytes);
                contents.push(Content::image(data, "image/png").with_priority(0.0));
            }
        }

        // Add the JSON output as text (contains element IDs, labels, bounds)
        // Truncate if very long to avoid context overflow
        let text_output = if json_output.len() > 8000 {
            let truncated: String = json_output.chars().take(8000).collect();
            format!(
                "Annotated screenshot captured. UI elements (truncated):\n{}...\n\n[Output truncated. {} total chars. Use element IDs (B1, T2, etc.) with peekaboo_click.]",
                truncated,
                json_output.len()
            )
        } else {
            format!(
                "Annotated screenshot captured. UI elements:\n{}\n\nUse element IDs (B1, T2, L3, etc.) with peekaboo_click to interact.",
                json_output
            )
        };

        contents.insert(
            0,
            Content::text(&text_output).with_audience(vec![Role::Assistant]),
        );

        Ok(CallToolResult::success(contents))
    }

    /// Click on a UI element identified by peekaboo_see, or at specific coordinates.
    #[tool(
        name = "peekaboo_click",
        description = "Click on a UI element by its ID from peekaboo_see output (e.g., 'B1' for button 1, 'T2' for text field 2), or at specific x,y coordinates. Supports double-click and right-click. Uses the most recent peekaboo_see snapshot automatically."
    )]
    pub async fn peekaboo_click(
        &self,
        params: Parameters<PeekabooClickParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.require_peekaboo()?;
        let params = params.0;

        let mut args: Vec<String> = vec!["click".to_string()];

        if let Some(ref on) = params.on {
            args.push("--on".to_string());
            args.push(on.clone());
        } else if let Some(ref coords) = params.coords {
            args.push("--coords".to_string());
            args.push(coords.clone());
        } else {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Either 'on' (element ID) or 'coords' (x,y) must be provided".to_string(),
                None,
            ));
        }

        if params.double {
            args.push("--double".to_string());
        }

        if params.right {
            args.push("--right".to_string());
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_peekaboo_cmd(&args_refs)?;

        let target = params
            .on
            .as_deref()
            .or(params.coords.as_deref())
            .unwrap_or("unknown");
        let click_type = if params.double {
            "Double-clicked"
        } else if params.right {
            "Right-clicked"
        } else {
            "Clicked"
        };

        Ok(CallToolResult::success(vec![Content::text(format!(
            "{} on '{}'. Output: {}",
            click_type,
            target,
            output.trim()
        ))]))
    }

    /// Type text into the currently focused UI element.
    #[tool(
        name = "peekaboo_type",
        description = "Type text into the currently focused UI element. First use peekaboo_see to identify elements, then peekaboo_click to focus a text field, then peekaboo_type to enter text. Set 'clear' to true to clear the field before typing."
    )]
    pub async fn peekaboo_type(
        &self,
        params: Parameters<PeekabooTypeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.require_peekaboo()?;
        let params = params.0;

        let mut args: Vec<String> = vec!["type".to_string()];
        args.push("--text".to_string());
        args.push(params.text.clone());

        if params.clear {
            args.push("--clear".to_string());
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_peekaboo_cmd(&args_refs)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Typed '{}'. Output: {}",
            params.text,
            output.trim()
        ))]))
    }

    /// Manage applications: launch, quit, switch, or list running apps.
    #[tool(
        name = "peekaboo_app",
        description = "Manage macOS applications: launch (with optional URL), quit, switch to, or list running apps. Requires Peekaboo CLI."
    )]
    pub async fn peekaboo_app(
        &self,
        params: Parameters<PeekabooAppParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.require_peekaboo()?;
        let params = params.0;

        let action_str = match params.action {
            PeekabooAppAction::Launch => "launch",
            PeekabooAppAction::Quit => "quit",
            PeekabooAppAction::Switch => "switch",
            PeekabooAppAction::List => "list",
        };

        let mut args: Vec<String> = vec!["app".to_string(), action_str.to_string()];

        if let Some(ref app) = params.app {
            args.push(app.clone());
        } else if !matches!(params.action, PeekabooAppAction::List) {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "App name is required for launch/quit/switch actions".to_string(),
                None,
            ));
        }

        if let Some(ref url) = params.open_url {
            args.push("--open".to_string());
            args.push(url.clone());
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_peekaboo_cmd(&args_refs)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "App {} completed. Output: {}",
            action_str,
            output.trim()
        ))]))
    }

    /// Press a keyboard shortcut / hotkey combination.
    #[tool(
        name = "peekaboo_hotkey",
        description = "Press a keyboard shortcut combination on macOS. Provide comma-separated keys like 'cmd,c' for copy, 'cmd,shift,t' for reopen tab, 'cmd,space' for Spotlight. Requires Peekaboo CLI."
    )]
    pub async fn peekaboo_hotkey(
        &self,
        params: Parameters<PeekabooHotkeyParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.require_peekaboo()?;
        let params = params.0;

        let args = vec!["hotkey", &params.combo];
        let output = self.run_peekaboo_cmd(&args)?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Pressed hotkey '{}'. Output: {}",
            params.combo,
            output.trim()
        ))]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ComputerControllerServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-computercontroller".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _pagination: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let active_resources = self.active_resources.lock().unwrap();
        let resources: Vec<Resource> = active_resources
            .keys()
            .map(|uri| {
                RawResource::new(
                    uri.clone(),
                    uri.split('/').next_back().unwrap_or("").to_string(),
                )
                .no_annotation()
            })
            .collect();
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        params: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let active_resources = self.active_resources.lock().unwrap();
        let resource = active_resources.get(&params.uri).ok_or_else(|| {
            ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                format!("Resource not found: {}", params.uri),
                None,
            )
        })?;

        // Clone the resource to return
        Ok(ReadResourceResult {
            contents: vec![resource.clone()],
        })
    }
}
