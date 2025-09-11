use etcetera::{choose_app_strategy, AppStrategy};
use indoc::{formatdoc, indoc};
use reqwest::{Client, Url};
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{
        AnnotateAble, CallToolResult, Content, ErrorCode, ErrorData, Implementation, Role,
        ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc, sync::Mutex};
use tokio::process::Command;

use rmcp::handler::server::wrapper::Parameters;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
mod docx_tool;
mod pdf_tool;
mod platform;
mod xlsx_tool;
use platform::{create_system_automation, SystemAutomation};

/// Parameter struct for web_scrape tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebScrapeParams {
    /// The URL to fetch content from
    pub url: String,
    /// How to interpret and save the content
    #[serde(default = "default_save_as")]
    pub save_as: String,
}

fn default_save_as() -> String {
    "text".to_string()
}

/// Parameter struct for automation_script tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutomationScriptParams {
    /// The scripting language to use
    pub language: String,
    /// The script content
    pub script: String,
    /// Whether to save the script output to a file
    #[serde(default)]
    pub save_output: bool,
}

/// Parameter struct for computer_control tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComputerControlParams {
    /// The automation script content (PowerShell for Windows, AppleScript for macOS)
    pub script: String,
    /// Whether to save the script output to a file
    #[serde(default)]
    pub save_output: bool,
}

/// Parameter struct for cache tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheParams {
    /// The command to perform
    pub command: String,
    /// Path to the cached file for view/delete commands
    pub path: Option<String>,
}

/// Parameter struct for pdf_tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PdfToolParams {
    /// Path to the PDF file
    pub path: String,
    /// Operation to perform on the PDF
    pub operation: String,
}

/// Parameter struct for docx_tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DocxToolParams {
    /// Path to the DOCX file
    pub path: String,
    /// Operation to perform on the DOCX
    pub operation: String,
    /// Content to write (required for update_doc operation)
    pub content: Option<String>,
    /// Additional parameters for update_doc operation
    pub params: Option<Value>,
}

/// Parameter struct for xlsx_tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XlsxToolParams {
    /// Path to the XLSX file
    pub path: String,
    /// Operation to perform on the XLSX file
    pub operation: String,
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

/// ComputerController MCP Server using official RMCP SDK
pub struct ComputerControllerServer {
    tool_router: ToolRouter<Self>,
    cache_dir: PathBuf,
    active_resources: Arc<Mutex<HashMap<String, rmcp::model::Resource>>>,
    http_client: Client,
    instructions: String,
    system_automation: Arc<Box<dyn SystemAutomation + Send + Sync>>,
}

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
                - AppleScript for system and UI control
                - Integration with macOS apps and services
              - Use the screenshot tool if needed to help with tasks

            computer_control
              - System automation using AppleScript
              - Consider the screenshot tool to work out what is on screen and what to do to help with the control task.

            When you need to interact with websites or web applications, consider using the computer_control tool with AppleScript, which can automate Safari or other browsers to:
              - Open specific URLs
              - Fill in forms
              - Click buttons
              - Extract content
              - Handle web-based workflows
            This is often more reliable than web scraping for modern web applications.
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
            http_client: Client::builder().user_agent("Goose/1.0").build().unwrap(),
            instructions: instructions.clone(),
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
    ) -> Result<PathBuf, String> {
        let cache_path = self.get_cache_path(prefix, extension);
        fs::write(&cache_path, content).map_err(|e| format!("Failed to write to cache: {}", e))?;
        Ok(cache_path)
    }

    // Helper function to register a file as a resource
    fn register_as_resource(&self, cache_path: &PathBuf, mime_type: &str) -> Result<(), String> {
        let uri = Url::from_file_path(cache_path)
            .map_err(|_| "Invalid cache path".to_string())?
            .to_string();

        let mut resource =
            rmcp::model::RawResource::new(uri.clone(), cache_path.to_string_lossy().into_owned());
        resource.mime_type = Some(if mime_type == "blob" {
            "blob".to_string()
        } else {
            "text".to_string()
        });
        self.active_resources
            .lock()
            .unwrap()
            .insert(uri, resource.no_annotation());
        Ok(())
    }
}

impl Default for ComputerControllerServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ComputerControllerServer {
    fn clone(&self) -> Self {
        Self {
            tool_router: Self::tool_router(),
            cache_dir: self.cache_dir.clone(),
            active_resources: self.active_resources.clone(),
            http_client: self.http_client.clone(),
            instructions: self.instructions.clone(),
            system_automation: self.system_automation.clone(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ComputerControllerServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-computercontroller".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }
}

#[tool_router(router = tool_router)]
impl ComputerControllerServer {
    /// Fetch and save content from a web page. The content can be saved as:
    /// - text (for HTML pages)
    /// - json (for API responses)
    /// - binary (for images and other files)
    ///
    /// The content is cached locally and can be accessed later using the cache_path
    /// returned in the response.
    #[tool(
        name = "web_scrape",
        description = "Fetch and save content from a web page. The content can be saved as:
- text (for HTML pages)
- json (for API responses)
- binary (for images and other files)

The content is cached locally and can be accessed later using the cache_path
returned in the response."
    )]
    pub async fn web_scrape(
        &self,
        params: Parameters<WebScrapeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        // Fetch the content
        let response = self
            .http_client
            .get(&params.url)
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
        let (content, extension) = match params.save_as.as_str() {
            "text" => {
                let text = response.text().await.map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to get text: {}", e),
                        None,
                    )
                })?;
                (text.into_bytes(), "txt")
            }
            "json" => {
                let text = response.text().await.map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to get text: {}", e),
                        None,
                    )
                })?;
                // Verify it's valid JSON
                serde_json::from_str::<Value>(&text).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Invalid JSON response: {}", e),
                        None,
                    )
                })?;
                (text.into_bytes(), "json")
            }
            "binary" => {
                let bytes = response.bytes().await.map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to get bytes: {}", e),
                        None,
                    )
                })?;
                (bytes.to_vec(), "bin")
            }
            _ => {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!(
                    "Invalid 'save_as' parameter: {}. Valid options are: 'text', 'json', 'binary'",
                    params.save_as
                ),
                    None,
                ));
            }
        };

        // Save to cache
        let cache_path = self
            .save_to_cache(&content, "web", extension)
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e, None))?;

        // Register as a resource
        self.register_as_resource(&cache_path, &params.save_as)
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e, None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Content saved to: {}",
            cache_path.display()
        ))
        .with_audience(vec![Role::Assistant])]))
    }

    /// Create and run small scripts for automation tasks.
    /// Supports Shell and Ruby (on macOS).
    ///
    /// The script is saved to a temporary file and executed.
    /// Consider using shell script (bash) for most simple tasks first.
    /// Ruby is useful for text processing or when you need more sophisticated scripting capabilities.
    #[tool(
        name = "automation_script",
        description = "Create and run small scripts for automation tasks.
Supports Shell and Ruby (on macOS).

The script is saved to a temporary file and executed.
Consider using shell script (bash) for most simple tasks first.
Ruby is useful for text processing or when you need more sophisticated scripting capabilities.
Some examples of shell:
    - create a sorted list of unique lines: sort file.txt | uniq
    - extract 2nd column in csv: awk -F \",\" '{ print $2}'
    - pattern matching: grep pattern file.txt"
    )]
    pub async fn automation_script(
        &self,
        params: Parameters<AutomationScriptParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        // Create a temporary directory for the script
        let script_dir = tempfile::tempdir().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create temporary directory: {}", e),
                None,
            )
        })?;

        let (shell, shell_arg) = self.system_automation.get_shell_command();

        let command = match params.language.as_str() {
            "shell" | "batch" => {
                let script_path = script_dir.path().join(format!(
                    "script.{}",
                    if cfg!(windows) { "bat" } else { "sh" }
                ));
                fs::write(&script_path, &params.script).map_err(|e| {
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
            "ruby" => {
                let script_path = script_dir.path().join("script.rb");
                fs::write(&script_path, &params.script).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write script: {}", e),
                        None,
                    )
                })?;

                format!("ruby {}", script_path.display())
            }
            "powershell" => {
                let script_path = script_dir.path().join("script.ps1");
                fs::write(&script_path, &params.script).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write script: {}", e),
                        None,
                    )
                })?;

                script_path.display().to_string()
            }
            _ => {
                return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, format!("Invalid 'language' parameter: {}. Valid options are: 'shell', 'batch', 'ruby', 'powershell'", params.language), None));
            }
        };

        // Run the script
        let output = match params.language.as_str() {
            "powershell" => {
                // For PowerShell, we need to use -File instead of -Command
                Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-NonInteractive")
                    .arg("-File")
                    .arg(&command)
                    .env("GOOSE_TERMINAL", "1")
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
        if params.save_output && !output_str.is_empty() {
            let cache_path = self
                .save_to_cache(output_str.as_bytes(), "script_output", "txt")
                .await
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e, None))?;
            result.push_str(&format!("\n\nOutput saved to: {}", cache_path.display()));

            // Register as a resource
            self.register_as_resource(&cache_path, "text")
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e, None))?;
        }

        Ok(CallToolResult::success(vec![
            Content::text(result).with_audience(vec![Role::Assistant])
        ]))
    }

    /// Control the computer using system automation.
    /// Features available vary by platform:
    /// - Windows: PowerShell automation for system control
    /// - macOS: AppleScript for application and system control
    /// - Linux: Shell scripting for system control
    ///
    /// Can be combined with screenshot tool for visual task assistance.
    #[tool(
        name = "computer_control",
        description = "Control the computer using AppleScript (macOS only). Automate applications and system features.

Key capabilities:
- Control Applications: Launch, quit, manage apps (Mail, Safari, iTunes, etc)
    - Interact with app-specific feature: (e.g, edit documents, process photos)
    - Perform tasks in third-party apps that support AppleScript
- UI Automation: Simulate user interactions like, clicking buttons, select menus, type text, filling out forms
- System Control: Manage settings (volume, brightness, wifi), shutdown/restart, monitor events
- Web & Email: Open URLs, web automation, send/organize emails, handle attachments
- Media: Manage music libraries, photo collections, playlists
- File Operations: Organize files/folders
- Integration: Calendar, reminders, messages
- Data: Interact with spreadsheets and documents

Can be combined with screenshot tool for visual task assistance."
    )]
    pub async fn computer_control(
        &self,
        params: Parameters<ComputerControlParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        // Use platform-specific automation
        let output = self
            .system_automation
            .execute_system_script(&params.script)
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to execute script: {}", e),
                    None,
                )
            })?;

        let mut result = format!("Script completed successfully.\n\nOutput:\n{}", output);

        // Save output if requested
        if params.save_output && !output.is_empty() {
            let cache_path = self
                .save_to_cache(output.as_bytes(), "automation_output", "txt")
                .await
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e, None))?;
            result.push_str(&format!("\n\nOutput saved to: {}", cache_path.display()));

            // Register as a resource
            self.register_as_resource(&cache_path, "text")
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e, None))?;
        }

        Ok(CallToolResult::success(vec![
            Content::text(result).with_audience(vec![Role::Assistant])
        ]))
    }

    /// Manage cached files and data:
    /// - list: List all cached files
    /// - view: View content of a cached file
    /// - delete: Delete a cached file
    /// - clear: Clear all cached files
    #[tool(
        name = "cache",
        description = "Manage cached files and data: list, view, delete, or clear files."
    )]
    pub async fn cache(
        &self,
        params: Parameters<CacheParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let result = match params.command.as_str() {
            "list" => {
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
                format!("Cached files:\n{}", files.join("\n"))
            }
            "view" => {
                let path = params.path.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'path' parameter for view".to_string(),
                        None,
                    )
                })?;

                let content = fs::read_to_string(&path).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to read file: {}", e),
                        None,
                    )
                })?;

                format!("Content of {}:\n\n{}", path, content)
            }
            "delete" => {
                let path = params.path.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'path' parameter for delete".to_string(),
                        None,
                    )
                })?;

                fs::remove_file(&path).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to delete file: {}", e),
                        None,
                    )
                })?;

                // Remove from active resources if present
                if let Ok(url) = Url::from_file_path(&path) {
                    self.active_resources
                        .lock()
                        .unwrap()
                        .remove(&url.to_string());
                }

                format!("Deleted file: {}", path)
            }
            "clear" => {
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

                "Cache cleared successfully.".to_string()
            }
            _ => {
                return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, format!(
                    "Invalid 'command' parameter: {}. Valid options are: 'list', 'view', 'delete', 'clear'",
                    params.command
                ), None));
            }
        };

        Ok(CallToolResult::success(vec![
            Content::text(result).with_audience(vec![Role::Assistant])
        ]))
    }

    /// Process PDF files to extract text and images.
    /// Supports operations:
    /// - extract_text: Extract all text content from the PDF
    /// - extract_images: Extract and save embedded images to PNG files
    ///
    /// Use this when there is a .pdf file or files that need to be processed.
    #[tool(
        name = "pdf_tool",
        description = "Process PDF files to extract text and images."
    )]
    pub async fn pdf_tool(
        &self,
        params: Parameters<PdfToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let result = pdf_tool::pdf_tool(&params.path, &params.operation, &self.cache_dir)
            .await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.message.to_string(), None))?;

        Ok(CallToolResult::success(result))
    }

    /// Process DOCX files to extract text and create/update documents.
    /// Supports operations:
    /// - extract_text: Extract all text content and structure (headings, TOC) from the DOCX
    /// - update_doc: Create a new DOCX or update existing one with provided content
    ///   Modes:
    ///   - append: Add content to end of document (default)
    ///   - replace: Replace specific text with new content
    ///   - structured: Add content with specific heading level and styling
    ///   - add_image: Add an image to the document (with optional caption)
    ///
    /// Use this when there is a .docx file that needs to be processed or created.
    #[tool(
        name = "docx_tool",
        description = "Process DOCX files to extract text and create/update documents."
    )]
    pub async fn docx_tool(
        &self,
        params: Parameters<DocxToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let result = docx_tool::docx_tool(
            &params.path,
            &params.operation,
            params.content.as_deref(),
            params.params.as_ref(),
        )
        .await
        .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.message.to_string(), None))?;

        Ok(CallToolResult::success(result))
    }

    /// Process Excel (XLSX) files to read and manipulate spreadsheet data.
    /// Supports operations:
    /// - list_worksheets: List all worksheets in the workbook (returns name, index, column_count, row_count)
    /// - get_columns: Get column names from a worksheet (returns values from the first row)
    /// - get_range: Get values and formulas from a cell range (e.g., "A1:C10") (returns a 2D array organized as [row][column])
    /// - find_text: Search for text in a worksheet (returns a list of (row, column) coordinates)
    /// - update_cell: Update a single cell's value (returns confirmation message)
    /// - get_cell: Get value and formula from a specific cell (returns both value and formula if present)
    /// - save: Save changes back to the file (returns confirmation message)
    ///
    /// Use this when working with Excel spreadsheets to analyze or modify data.
    #[tool(
        name = "xlsx_tool",
        description = "Process Excel (XLSX) files to read and manipulate spreadsheet data."
    )]
    pub async fn xlsx_tool(
        &self,
        params: Parameters<XlsxToolParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let result = match params.operation.as_str() {
            "list_worksheets" => {
                let xlsx = xlsx_tool::XlsxTool::new(&params.path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                let worksheets = xlsx
                    .list_worksheets()
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                vec![Content::text(format!("{:#?}", worksheets))]
            }
            "get_columns" => {
                let xlsx = xlsx_tool::XlsxTool::new(&params.path)
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
                vec![Content::text(format!("{:#?}", columns))]
            }
            "get_range" => {
                let range = params.range.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'range' parameter".to_string(),
                        None,
                    )
                })?;

                let xlsx = xlsx_tool::XlsxTool::new(&params.path)
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
                    .get_range(worksheet, &range)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                vec![Content::text(format!("{:#?}", range_data))]
            }
            "find_text" => {
                let search_text = params.search_text.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'search_text' parameter".to_string(),
                        None,
                    )
                })?;

                let xlsx = xlsx_tool::XlsxTool::new(&params.path)
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
                    .find_in_worksheet(worksheet, &search_text, params.case_sensitive)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                vec![Content::text(format!("Found matches at: {:#?}", matches))]
            }
            "update_cell" => {
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
                let value = params.value.ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Missing 'value' parameter".to_string(),
                        None,
                    )
                })?;

                let worksheet_name = params.worksheet.as_deref().unwrap_or("Sheet1");

                let mut xlsx = xlsx_tool::XlsxTool::new(&params.path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                xlsx.update_cell(worksheet_name, row as u32, col as u32, &value)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                xlsx.save(&params.path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                vec![Content::text(format!(
                    "Updated cell ({}, {}) to '{}' in worksheet '{}'",
                    row, col, value, worksheet_name
                ))]
            }
            "save" => {
                let xlsx = xlsx_tool::XlsxTool::new(&params.path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                xlsx.save(&params.path)
                    .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;
                vec![Content::text("File saved successfully.")]
            }
            "get_cell" => {
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

                let xlsx = xlsx_tool::XlsxTool::new(&params.path)
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
                vec![Content::text(format!("{:#?}", cell_value))]
            }
            _ => {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Invalid operation: {}", params.operation),
                    None,
                ));
            }
        };

        Ok(CallToolResult::success(result))
    }
}
