use base64::Engine;
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::indoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData, Implementation, Role, ServerCapabilities,
        ServerInfo,
    },
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

const BREW_FORMULA: &str = "steipete/tap/peekaboo";

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

/// Peekaboo MCP Server — macOS GUI automation via the Peekaboo CLI.
///
/// Provides a see → click → type workflow for visual UI interaction.
/// Auto-installs Peekaboo via Homebrew on first use if not already present.
#[derive(Clone)]
pub struct PeekabooServer {
    tool_router: ToolRouter<Self>,
    cache_dir: PathBuf,
    installed: std::sync::Arc<AtomicBool>,
}

impl Default for PeekabooServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl PeekabooServer {
    pub fn new() -> Self {
        let cache_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_cache_dir("peekaboo"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/peekaboo"));

        fs::create_dir_all(&cache_dir).unwrap_or_else(|_| {
            tracing::warn!(
                "Failed to create peekaboo cache directory at {:?}",
                cache_dir
            )
        });

        let installed = is_peekaboo_installed();

        Self {
            tool_router: Self::tool_router(),
            cache_dir,
            installed: std::sync::Arc::new(AtomicBool::new(installed)),
        }
    }

    fn get_cache_path(&self, prefix: &str, extension: &str) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        self.cache_dir
            .join(format!("{}_{}.{}", prefix, timestamp, extension))
    }

    /// Ensure Peekaboo is installed, auto-installing via brew if needed.
    fn ensure_peekaboo(&self) -> Result<(), ErrorData> {
        if self.installed.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Double-check in case another thread installed it
        if is_peekaboo_installed() {
            self.installed.store(true, Ordering::Relaxed);
            return Ok(());
        }

        // Try auto-install via brew
        tracing::info!("Peekaboo not found, attempting auto-install via brew");
        match auto_install_peekaboo() {
            Ok(()) => {
                self.installed.store(true, Ordering::Relaxed);
                tracing::info!("Peekaboo installed successfully");
                Ok(())
            }
            Err(msg) => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Peekaboo is not installed and auto-install failed: {}\n\
                     Install manually with: brew install {}\n\
                     Peekaboo requires macOS 15+ (Sequoia) with Screen Recording and Accessibility permissions.",
                    msg, BREW_FORMULA
                ),
                None,
            )),
        }
    }

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
    #[tool(
        name = "peekaboo_see",
        description = "Capture an annotated screenshot of the macOS UI showing clickable elements with IDs. Returns an annotated image with overlaid element labels (B1=button, T2=text field, L3=link) plus a JSON element list. Use this first, then peekaboo_click/peekaboo_type to interact with elements by ID."
    )]
    pub async fn peekaboo_see(
        &self,
        params: Parameters<PeekabooSeeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.ensure_peekaboo()?;
        let params = params.0;

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

        let json_output = self.run_peekaboo_cmd(&args)?;

        let mut contents = Vec::new();

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
        description = "Click on a UI element by its ID from peekaboo_see output (e.g., 'B1' for button 1, 'T2' for text field 2), or at specific x,y coordinates. Supports double-click and right-click."
    )]
    pub async fn peekaboo_click(
        &self,
        params: Parameters<PeekabooClickParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.ensure_peekaboo()?;
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
        self.ensure_peekaboo()?;
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
        description = "Manage macOS applications: launch (with optional URL), quit, switch to, or list running apps."
    )]
    pub async fn peekaboo_app(
        &self,
        params: Parameters<PeekabooAppParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.ensure_peekaboo()?;
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
        description = "Press a keyboard shortcut combination on macOS. Provide comma-separated keys like 'cmd,c' for copy, 'cmd,shift,t' for reopen tab, 'cmd,space' for Spotlight."
    )]
    pub async fn peekaboo_hotkey(
        &self,
        params: Parameters<PeekabooHotkeyParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.ensure_peekaboo()?;
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
impl ServerHandler for PeekabooServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-peekaboo".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                indoc! {r#"
                    Peekaboo — macOS UI automation via annotated screenshots and element targeting.

                    The core workflow is see → click → type:
                    1. peekaboo_see: Capture annotated screenshot with element IDs (B1=button, T2=text field, L3=link)
                    2. peekaboo_click: Click on an element by its ID from peekaboo_see output
                    3. peekaboo_type: Type text into the focused field

                    Additional tools:
                    - peekaboo_hotkey: Press keyboard shortcuts (e.g., "cmd,c" for copy)
                    - peekaboo_app: Launch, quit, switch apps, or list running apps

                    Use these tools for all visual UI interaction on macOS.
                "#}
                .to_string(),
            ),
            ..Default::default()
        }
    }
}

fn is_peekaboo_installed() -> bool {
    std::process::Command::new("which")
        .arg("peekaboo")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Resolve the brew executable, checking common locations.
fn resolve_brew() -> Option<String> {
    // Check PATH first
    if let Ok(output) = std::process::Command::new("which").arg("brew").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    // Check common Homebrew locations
    for candidate in &["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    None
}

/// Auto-install Peekaboo via Homebrew. Returns Ok(()) on success.
fn auto_install_peekaboo() -> Result<(), String> {
    let brew = resolve_brew().ok_or_else(|| {
        "Homebrew is not installed. Install Homebrew first (https://brew.sh), then run: brew install steipete/tap/peekaboo".to_string()
    })?;

    tracing::info!("Running: {} install {}", brew, BREW_FORMULA);

    let output = std::process::Command::new(&brew)
        .args(["install", BREW_FORMULA])
        .output()
        .map_err(|e| format!("Failed to run brew: {}", e))?;

    if output.status.success() {
        // Verify the binary is now available
        if is_peekaboo_installed() {
            return Ok(());
        }
        // brew succeeded but binary not on PATH — might need PATH update
        // Check brew prefix for the binary
        if let Ok(prefix_output) = std::process::Command::new(&brew)
            .args(["--prefix"])
            .output()
        {
            let prefix = String::from_utf8_lossy(&prefix_output.stdout)
                .trim()
                .to_string();
            let bin_path = format!("{}/bin/peekaboo", prefix);
            if std::path::Path::new(&bin_path).exists() {
                // Add brew bin to PATH for this process
                if let Ok(current_path) = std::env::var("PATH") {
                    std::env::set_var("PATH", format!("{}/bin:{}", prefix, current_path));
                    if is_peekaboo_installed() {
                        return Ok(());
                    }
                }
            }
        }
        Err("brew install succeeded but peekaboo binary not found on PATH".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "brew install failed (exit {}):\n{}{}",
            output.status,
            stderr.trim(),
            if stdout.trim().is_empty() {
                String::new()
            } else {
                format!("\n{}", stdout.trim())
            }
        ))
    }
}
