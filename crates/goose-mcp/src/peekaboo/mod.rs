use base64::Engine;
use etcetera::{choose_app_strategy, AppStrategy};
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

/// Parameters for the peekaboo tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PeekabooParams {
    /// The peekaboo subcommand and arguments as a single string.
    /// Examples:
    ///   "see --app Safari --annotate"
    ///   "click --on B1"
    ///   "type --text \"hello\" --return"
    ///   "hotkey --keys cmd,c"
    ///   "app launch Safari --open https://example.com"
    ///   "window list --app Safari --json"
    ///   "press tab --count 3"
    ///   "clipboard --action get"
    pub command: String,
    /// Whether to capture and return a screenshot as part of the result.
    /// Useful after click/type actions to see the updated UI state.
    #[serde(default)]
    pub capture_screenshot: bool,
}

/// Peekaboo MCP Server — macOS GUI automation via the Peekaboo CLI.
///
/// Exposes a single `peekaboo` tool that passes through to the peekaboo CLI,
/// giving the agent access to the full command set. Auto-installs via Homebrew
/// on first use.
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

    fn ensure_peekaboo(&self) -> Result<(), ErrorData> {
        if self.installed.load(Ordering::Relaxed) {
            return Ok(());
        }

        if is_peekaboo_installed() {
            self.installed.store(true, Ordering::Relaxed);
            return Ok(());
        }

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

    /// Run a peekaboo CLI command. Returns (stdout, stderr, success).
    fn run_peekaboo(&self, args: &[&str]) -> Result<(String, String), ErrorData> {
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
                    "peekaboo {} failed (exit {}):\n{}\n{}",
                    args.first().unwrap_or(&""),
                    output.status,
                    stderr.trim(),
                    stdout.trim()
                ),
                None,
            ));
        }

        Ok((stdout, stderr))
    }

    /// Run any peekaboo CLI command. The full Peekaboo CLI is available —
    /// see the extension instructions for the complete command reference.
    #[tool(
        name = "peekaboo",
        description = "Run a Peekaboo CLI command for macOS GUI automation. Pass the subcommand and arguments as a string. The core workflow is: see (capture annotated screenshot) → click (on element IDs) → type (text). Set capture_screenshot=true to see the UI state after actions."
    )]
    pub async fn peekaboo(
        &self,
        params: Parameters<PeekabooParams>,
    ) -> Result<CallToolResult, ErrorData> {
        self.ensure_peekaboo()?;
        let params = params.0;

        // Parse the command string into args, respecting quotes
        let args = shell_words::split(&params.command).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Failed to parse command: {}", e),
                None,
            )
        })?;

        if args.is_empty() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Command cannot be empty".to_string(),
                None,
            ));
        }

        // Check if this is a `see` command — if so, inject --path and --json-output
        let is_see = args[0] == "see";
        let is_image = args[0] == "image";
        let screenshot_path = if is_see || is_image {
            Some(self.get_cache_path(&args[0], "png"))
        } else {
            None
        };

        let mut full_args: Vec<String> = args.clone();

        if let Some(ref path) = screenshot_path {
            if !full_args.iter().any(|a| a == "--path") {
                full_args.push("--path".to_string());
                full_args.push(path.to_string_lossy().to_string());
            }
        }

        // For `see`, always add --json-output for element data
        if is_see && !full_args.iter().any(|a| a == "--json-output") {
            full_args.push("--json-output".to_string());
        }

        // Always request JSON output for structured data when useful
        let wants_json = matches!(
            args[0].as_str(),
            "list" | "window" | "menubar" | "permissions" | "clipboard"
        );
        if wants_json
            && !full_args.iter().any(|a| a == "--json" || a == "-j")
            && !full_args.iter().any(|a| a == "--json-output")
        {
            full_args.push("--json".to_string());
        }

        let arg_refs: Vec<&str> = full_args.iter().map(|s| s.as_str()).collect();
        let (stdout, _stderr) = self.run_peekaboo(&arg_refs)?;

        let mut contents = Vec::new();

        // For see/image commands, try to return the screenshot
        if let Some(ref path) = screenshot_path {
            // For `see --annotate`, Peekaboo saves *_annotated.png
            let annotated = path.to_string_lossy().replace(".png", "_annotated.png");
            let image_path = if is_see && std::path::Path::new(&annotated).exists() {
                PathBuf::from(&annotated)
            } else {
                path.clone()
            };

            if image_path.exists() {
                if let Ok(bytes) = fs::read(&image_path) {
                    let data = base64::prelude::BASE64_STANDARD.encode(&bytes);
                    contents.push(Content::image(data, "image/png").with_priority(0.0));
                }
            }
        }

        // If capture_screenshot requested (e.g., after a click), take a fresh screenshot
        if params.capture_screenshot && screenshot_path.is_none() {
            let cap_path = self.get_cache_path("peekaboo_capture", "png");
            let cap_path_str = cap_path.to_string_lossy().to_string();
            if let Ok((_, _)) =
                self.run_peekaboo(&["image", "--mode", "frontmost", "--path", &cap_path_str])
            {
                if cap_path.exists() {
                    if let Ok(bytes) = fs::read(&cap_path) {
                        let data = base64::prelude::BASE64_STANDARD.encode(&bytes);
                        contents.push(Content::image(data, "image/png").with_priority(0.0));
                    }
                }
            }
        }

        // Truncate large output
        let text = if stdout.len() > 12000 {
            let truncated: String = stdout.chars().take(12000).collect();
            format!(
                "{}\n\n[Output truncated. {} total chars.]",
                truncated,
                stdout.len()
            )
        } else {
            stdout
        };

        contents.insert(0, Content::text(&text).with_audience(vec![Role::Assistant]));

        Ok(CallToolResult::success(contents))
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
            instructions: Some(PEEKABOO_INSTRUCTIONS.to_string()),
            ..Default::default()
        }
    }
}

const PEEKABOO_INSTRUCTIONS: &str = r#"
Peekaboo — macOS UI automation CLI. Auto-installed via Homebrew on first use.

## Core Workflow: see → click → type

1. `see --app Safari --annotate` — capture annotated screenshot with element IDs (B1=button, T2=text, L3=link)
2. `click --on B1` — click element by ID from the see output
3. `type --text "hello" --return` — type text, optionally press return

## Command Reference

### Vision
- `see` — annotated UI maps with element IDs. Use `--app Name`, `--window-title`, `--mode screen|window|frontmost`, `--annotate`, `--analyze "prompt"`, `--path /tmp/out.png`
- `image` — capture screenshots. Use `--mode screen|window|frontmost`, `--app Name`, `--window-title`, `--screen-index N`, `--retina`, `--path /tmp/out.png`

### Interaction
- `click` — click by element ID (`--on B1`), query, or coordinates (`--coords 100,200`). Supports `--double`, `--right`, `--app`
- `type` — type text (`--text "hello"`). Use `--clear` to clear field first, `--return` to press return after, `--delay N` between chars, `--wpm N`
- `press` — press special keys (`press tab`, `press escape`, `press return`). Use `--count N` for repeats, `--delay N` between presses
- `hotkey` — keyboard shortcuts (`--keys cmd,c`, `--keys cmd,shift,t`). Keys are comma-separated
- `paste` — paste text via clipboard (`--text "content"`). More reliable than type for long text
- `scroll` — scroll (`--direction up|down|left|right`, `--amount N`, `--smooth`)
- `drag` — drag and drop (`--from B1 --to T2`, or `--from-coords` / `--to-coords`)
- `move` — move cursor (`move 500,300 --smooth`)
- `swipe` — gesture swipe (`--from-coords 100,500 --to-coords 100,200 --duration 800`)

### Apps & Windows
- `app` — manage apps: `app launch Safari`, `app quit Safari`, `app switch Safari`, `app list`. Use `--open URL` with launch
- `window` — manage windows: `window list --app Safari --json`, `window focus --app Safari`, `window set-bounds --app Safari --x 50 --y 50 --width 1200 --height 800`, `window close/minimize/maximize`
- `list` — list apps, windows, screens: `list apps --json`, `list windows --json`, `list screens --json`

### System
- `clipboard` — read/write clipboard: `clipboard --action get`, `clipboard --action set --text "content"`
- `menu` — click app menus: `menu click --app Safari --item "New Window"`, `menu click --app TextEdit --path "Format > Font > Show Fonts"`
- `menubar` — status bar items: `menubar list --json`, `menubar click --title "WiFi"`
- `dock` — Dock items: `dock launch Safari`, `dock list --json`
- `dialog` — system dialogs: `dialog click --button "OK"`, `dialog list`
- `space` — Spaces: `space list`, `space switch --index 2`
- `open` — enhanced open: `open https://example.com --app Safari`
- `permissions` — check permissions: `permissions status`

### Common Targeting Parameters
- App/window: `--app Name`, `--pid N`, `--window-title`, `--window-id`, `--window-index`
- Element: `--on ID` (element ID from see), `--coords x,y`

### Tips
- Always `see --annotate` first to discover element IDs before clicking
- Use `--json` or `-j` on list/query commands for structured output
- Use `press return` instead of adding \n to type commands
- Use `paste --text` instead of `type --text` for long content
- For hotkeys: keys are comma-separated lowercase (`cmd,c` not `Cmd+C`)
- After actions, use capture_screenshot=true to verify the result
"#;

pub fn is_peekaboo_installed() -> bool {
    std::process::Command::new("which")
        .arg("peekaboo")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn resolve_brew() -> Option<String> {
    if let Ok(output) = std::process::Command::new("which").arg("brew").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    for candidate in &["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    None
}

pub fn auto_install_peekaboo() -> Result<(), String> {
    let brew = resolve_brew().ok_or_else(|| {
        "Homebrew is not installed. Install Homebrew first (https://brew.sh), then run: brew install steipete/tap/peekaboo".to_string()
    })?;

    tracing::info!("Running: {} install {}", brew, BREW_FORMULA);

    let output = std::process::Command::new(&brew)
        .args(["install", BREW_FORMULA])
        .output()
        .map_err(|e| format!("Failed to run brew: {}", e))?;

    if output.status.success() {
        if is_peekaboo_installed() {
            return Ok(());
        }
        // brew succeeded but binary not on PATH — try adding brew bin
        if let Ok(prefix_output) = std::process::Command::new(&brew)
            .args(["--prefix"])
            .output()
        {
            let prefix = String::from_utf8_lossy(&prefix_output.stdout)
                .trim()
                .to_string();
            let bin_path = format!("{}/bin/peekaboo", prefix);
            if std::path::Path::new(&bin_path).exists() {
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
