use bat::WrappingMode;
use console::{style, Color};
use goose::config::Config;
use goose::message::{Message, MessageContent, ToolRequest, ToolResponse};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use mcp_core::prompt::PromptArgument;
use mcp_core::tool::ToolCall;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

// Re-export theme for use in main
#[derive(Clone, Copy)]
pub enum Theme {
    Light,
    Dark,
    Ansi,
}

impl Theme {
    fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "GitHub",
            Theme::Dark => "zenburn",
            Theme::Ansi => "base16",
        }
    }

    fn from_config_str(val: &str) -> Self {
        if val.eq_ignore_ascii_case("light") {
            Theme::Light
        } else if val.eq_ignore_ascii_case("ansi") {
            Theme::Ansi
        } else {
            Theme::Dark
        }
    }

    fn as_config_string(&self) -> String {
        match self {
            Theme::Light => "light".to_string(),
            Theme::Dark => "dark".to_string(),
            Theme::Ansi => "ansi".to_string(),
        }
    }
}

thread_local! {
    static CURRENT_THEME: RefCell<Theme> = RefCell::new(
        std::env::var("GOOSE_CLI_THEME").ok()
            .map(|val| Theme::from_config_str(&val))
            .unwrap_or_else(||
                Config::global().get_param::<String>("GOOSE_CLI_THEME").ok()
                    .map(|val| Theme::from_config_str(&val))
                    .unwrap_or(Theme::Dark)
            )
    );
}

pub fn set_theme(theme: Theme) {
    let config = Config::global();
    config
        .set_param("GOOSE_CLI_THEME", Value::String(theme.as_config_string()))
        .expect("Failed to set theme");
    CURRENT_THEME.with(|t| *t.borrow_mut() = theme);
}

pub fn get_theme() -> Theme {
    CURRENT_THEME.with(|t| *t.borrow())
}

// Simple wrapper around spinner to manage its state
#[derive(Default)]
pub struct ThinkingIndicator {
    spinner: Option<cliclack::ProgressBar>,
}

impl ThinkingIndicator {
    pub fn show(&mut self) {
        let spinner = cliclack::spinner();
        spinner.start(format!(
            "{}...",
            super::thinking::get_random_thinking_message()
        ));
        self.spinner = Some(spinner);
    }

    pub fn hide(&mut self) {
        if let Some(spinner) = self.spinner.take() {
            spinner.stop("");
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
    pub extension: Option<String>,
}

// Global thinking indicator
thread_local! {
    static THINKING: RefCell<ThinkingIndicator> = RefCell::new(ThinkingIndicator::default());
}

pub fn show_thinking() {
    THINKING.with(|t| t.borrow_mut().show());
}

pub fn hide_thinking() {
    THINKING.with(|t| t.borrow_mut().hide());
}

#[allow(dead_code)]
pub fn set_thinking_message(s: &String) {
    THINKING.with(|t| {
        if let Some(spinner) = t.borrow_mut().spinner.as_mut() {
            spinner.set_message(s);
        }
    });
}

pub fn render_message(message: &Message, debug: bool) {
    let theme = get_theme();

    for content in &message.content {
        match content {
            MessageContent::Text(text) => print_markdown(&text.text, theme),
            MessageContent::ToolRequest(req) => render_tool_request(req, theme, debug),
            MessageContent::ToolResponse(resp) => render_tool_response(resp, theme, debug),
            MessageContent::Image(image) => {
                println!("Image: [data: {}, type: {}]", image.data, image.mime_type);
            }
            MessageContent::Thinking(thinking) => {
                if std::env::var("GOOSE_CLI_SHOW_THINKING").is_ok() {
                    println!("\n{}", style("Thinking:").dim().italic());
                    print_markdown(&thinking.thinking, theme);
                }
            }
            MessageContent::RedactedThinking(_) => {
                // For redacted thinking, print thinking was redacted
                println!("\n{}", style("Thinking:").dim().italic());
                print_markdown("Thinking was redacted", theme);
            }
            _ => {
                println!("WARNING: Message content type could not be rendered");
            }
        }
    }
    println!();
}

pub fn render_text(text: &str, color: Option<Color>, dim: bool) {
    render_text_no_newlines(format!("\n{}\n\n", text).as_str(), color, dim);
}

pub fn render_text_no_newlines(text: &str, color: Option<Color>, dim: bool) {
    let mut styled_text = style(text);
    if dim {
        styled_text = styled_text.dim();
    }
    if let Some(color) = color {
        styled_text = styled_text.fg(color);
    } else {
        styled_text = styled_text.green();
    }
    print!("{}", styled_text);
}

pub fn render_enter_plan_mode() {
    println!(
        "\n{} {}\n",
        style("Entering plan mode.").green().bold(),
        style("You can provide instructions to create a plan and then act on it. To exit early, type /endplan")
            .green()
            .dim()
    );
}

pub fn render_act_on_plan() {
    println!(
        "\n{}\n",
        style("Exiting plan mode and acting on the above plan")
            .green()
            .bold(),
    );
}

pub fn render_exit_plan_mode() {
    println!("\n{}\n", style("Exiting plan mode.").green().bold());
}

pub fn goose_mode_message(text: &str) {
    println!("\n{}", style(text).yellow(),);
}

fn render_tool_request(req: &ToolRequest, theme: Theme, debug: bool) {
    match &req.tool_call {
        Ok(call) => match call.name.as_str() {
            "developer__text_editor" => render_text_editor_request(call, debug),
            "developer__shell" => render_shell_request(call, debug),
            _ => render_default_request(call, debug),
        },
        Err(e) => print_markdown(&e.to_string(), theme),
    }
}

fn render_tool_response(resp: &ToolResponse, theme: Theme, debug: bool) {
    let config = Config::global();

    match &resp.tool_result {
        Ok(contents) => {
            for content in contents {
                if let Some(audience) = content.audience() {
                    if !audience.contains(&mcp_core::role::Role::User) {
                        continue;
                    }
                }

                let min_priority = config
                    .get_param::<f32>("GOOSE_CLI_MIN_PRIORITY")
                    .ok()
                    .unwrap_or(0.5);

                if content
                    .priority()
                    .is_some_and(|priority| priority < min_priority)
                    || (content.priority().is_none() && !debug)
                {
                    continue;
                }

                if debug {
                    println!("{:#?}", content);
                } else if let mcp_core::content::Content::Text(text) = content {
                    print_markdown(&text.text, theme);
                }
            }
        }
        Err(e) => print_markdown(&e.to_string(), theme),
    }
}

pub fn render_error(message: &str) {
    println!("\n  {} {}\n", style("error:").red().bold(), message);
}

pub fn render_prompts(prompts: &HashMap<String, Vec<String>>) {
    println!();
    for (extension, prompts) in prompts {
        println!(" {}", style(extension).green());
        for prompt in prompts {
            println!("  - {}", style(prompt).cyan());
        }
    }
    println!();
}

pub fn render_prompt_info(info: &PromptInfo) {
    println!();

    if let Some(ext) = &info.extension {
        println!(" {}: {}", style("Extension").green(), ext);
    }

    println!(" Prompt: {}", style(&info.name).cyan().bold());

    if let Some(desc) = &info.description {
        println!("\n {}", desc);
    }

    if let Some(args) = &info.arguments {
        println!("\n Arguments:");
        for arg in args {
            let required = arg.required.unwrap_or(false);
            let req_str = if required {
                style("(required)").red()
            } else {
                style("(optional)").dim()
            };

            println!(
                "  {} {} {}",
                style(&arg.name).yellow(),
                req_str,
                arg.description.as_deref().unwrap_or("")
            );
        }
    }
    println!();
}

pub fn render_extension_success(name: &str) {
    println!();
    println!(
        "  {} extension `{}`",
        style("added").green(),
        style(name).cyan(),
    );
    println!();
}

pub fn render_extension_error(name: &str, error: &str) {
    println!();
    println!(
        "  {} to add extension {}",
        style("failed").red(),
        style(name).red()
    );
    println!();
    println!("{}", style(error).dim());
    println!();
}

pub fn render_builtin_success(names: &str) {
    println!();
    println!(
        "  {} builtin{}: {}",
        style("added").green(),
        if names.contains(',') { "s" } else { "" },
        style(names).cyan()
    );
    println!();
}

pub fn render_builtin_error(names: &str, error: &str) {
    println!();
    println!(
        "  {} to add builtin{}: {}",
        style("failed").red(),
        if names.contains(',') { "s" } else { "" },
        style(names).red()
    );
    println!();
    println!("{}", style(error).dim());
    println!();
}

fn render_text_editor_request(call: &ToolCall, debug: bool) {
    print_tool_header(call);

    // Print path first with special formatting
    if let Some(Value::String(path)) = call.arguments.get("path") {
        println!(
            "{}: {}",
            style("path").dim(),
            style(shorten_path(path, debug)).green()
        );
    }

    // Print other arguments normally, excluding path
    if let Some(args) = call.arguments.as_object() {
        let mut other_args = serde_json::Map::new();
        for (k, v) in args {
            if k != "path" {
                other_args.insert(k.clone(), v.clone());
            }
        }
        print_params(&Value::Object(other_args), 0, debug);
    }
    println!();
}

fn render_shell_request(call: &ToolCall, debug: bool) {
    print_tool_header(call);

    match call.arguments.get("command") {
        Some(Value::String(s)) => {
            println!("{}: {}", style("command").dim(), style(s).green());
        }
        _ => print_params(&call.arguments, 0, debug),
    }
}

fn render_default_request(call: &ToolCall, debug: bool) {
    print_tool_header(call);
    print_params(&call.arguments, 0, debug);
    println!();
}

// Helper functions

fn print_tool_header(call: &ToolCall) {
    let parts: Vec<_> = call.name.rsplit("__").collect();
    let tool_header = format!(
        "─── {} | {} ──────────────────────────",
        style(parts.first().unwrap_or(&"unknown")),
        style(
            parts
                .split_first()
                .map(|(_, s)| s.iter().rev().copied().collect::<Vec<_>>().join("__"))
                .unwrap_or_else(|| "unknown".to_string())
        )
        .magenta()
        .dim(),
    );
    println!();
    println!("{}", tool_header);
}

// Respect NO_COLOR, as https://crates.io/crates/console already does
pub fn env_no_color() -> bool {
    // if NO_COLOR is defined at all disable colors
    std::env::var_os("NO_COLOR").is_none()
}

fn print_markdown(content: &str, theme: Theme) {
    bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()))
        .theme(theme.as_str())
        .colored_output(env_no_color())
        .language("Markdown")
        .wrapping_mode(WrappingMode::NoWrapping(true))
        .print()
        .unwrap();
}

const INDENT: &str = "    ";

fn get_tool_params_max_length() -> usize {
    Config::global()
        .get_param::<usize>("GOOSE_CLI_TOOL_PARAMS_TRUNCATION_MAX_LENGTH")
        .ok()
        .unwrap_or(40)
}

fn print_params(value: &Value, depth: usize, debug: bool) {
    let indent = INDENT.repeat(depth);

    match value {
        Value::Object(map) => {
            for (key, val) in map {
                match val {
                    Value::Object(_) => {
                        println!("{}{}:", indent, style(key).dim());
                        print_params(val, depth + 1, debug);
                    }
                    Value::Array(arr) => {
                        println!("{}{}:", indent, style(key).dim());
                        for item in arr.iter() {
                            println!("{}{}- ", indent, INDENT);
                            print_params(item, depth + 2, debug);
                        }
                    }
                    Value::String(s) => {
                        if !debug && s.len() > get_tool_params_max_length() {
                            println!("{}{}: {}", indent, style(key).dim(), style("...").dim());
                        } else {
                            println!("{}{}: {}", indent, style(key).dim(), style(s).green());
                        }
                    }
                    Value::Number(n) => {
                        println!("{}{}: {}", indent, style(key).dim(), style(n).blue());
                    }
                    Value::Bool(b) => {
                        println!("{}{}: {}", indent, style(key).dim(), style(b).blue());
                    }
                    Value::Null => {
                        println!("{}{}: {}", indent, style(key).dim(), style("null").dim());
                    }
                }
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                println!("{}{}.", indent, i + 1);
                print_params(item, depth + 1, debug);
            }
        }
        Value::String(s) => {
            if !debug && s.len() > get_tool_params_max_length() {
                println!(
                    "{}{}",
                    indent,
                    style(format!("[REDACTED: {} chars]", s.len())).yellow()
                );
            } else {
                println!("{}{}", indent, style(s).green());
            }
        }
        Value::Number(n) => {
            println!("{}{}", indent, style(n).yellow());
        }
        Value::Bool(b) => {
            println!("{}{}", indent, style(b).yellow());
        }
        Value::Null => {
            println!("{}{}", indent, style("null").dim());
        }
    }
}

fn shorten_path(path: &str, debug: bool) -> String {
    // In debug mode, return the full path
    if debug {
        return path.to_string();
    }

    let path = Path::new(path);

    // First try to convert to ~ if it's in home directory
    let home = etcetera::home_dir().ok();
    let path_str = if let Some(home) = home {
        if let Ok(stripped) = path.strip_prefix(home) {
            format!("~/{}", stripped.display())
        } else {
            path.display().to_string()
        }
    } else {
        path.display().to_string()
    };

    // If path is already short enough, return as is
    if path_str.len() <= 60 {
        return path_str;
    }

    let parts: Vec<_> = path_str.split('/').collect();

    // If we have 3 or fewer parts, return as is
    if parts.len() <= 3 {
        return path_str;
    }

    // Keep the first component (empty string before root / or ~) and last two components intact
    let mut shortened = vec![parts[0].to_string()];

    // Shorten middle components to their first letter
    for component in &parts[1..parts.len() - 2] {
        if !component.is_empty() {
            shortened.push(component.chars().next().unwrap_or('?').to_string());
        }
    }

    // Add the last two components
    shortened.push(parts[parts.len() - 2].to_string());
    shortened.push(parts[parts.len() - 1].to_string());

    shortened.join("/")
}

// Session display functions
pub fn display_session_info(
    resume: bool,
    provider: &str,
    model: &str,
    session_file: &Option<PathBuf>,
    provider_instance: Option<&Arc<dyn goose::providers::base::Provider>>,
) {
    let start_session_msg = if resume {
        "resuming session |"
    } else if session_file.is_none() {
        "running without session |"
    } else {
        "starting session |"
    };

    // Check if we have lead/worker mode
    if let Some(provider_inst) = provider_instance {
        if let Some(lead_worker) = provider_inst.as_lead_worker() {
            let (lead_model, worker_model) = lead_worker.get_model_info();
            println!(
                "{} {} {} {} {} {} {}",
                style(start_session_msg).dim(),
                style("provider:").dim(),
                style(provider).cyan().dim(),
                style("lead model:").dim(),
                style(&lead_model).cyan().dim(),
                style("worker model:").dim(),
                style(&worker_model).cyan().dim(),
            );
        } else {
            println!(
                "{} {} {} {} {}",
                style(start_session_msg).dim(),
                style("provider:").dim(),
                style(provider).cyan().dim(),
                style("model:").dim(),
                style(model).cyan().dim(),
            );
        }
    } else {
        // Fallback to original behavior if no provider instance
        println!(
            "{} {} {} {} {}",
            style(start_session_msg).dim(),
            style("provider:").dim(),
            style(provider).cyan().dim(),
            style("model:").dim(),
            style(model).cyan().dim(),
        );
    }

    if let Some(session_file) = session_file {
        println!(
            "    {} {}",
            style("logging to").dim(),
            style(session_file.display()).dim().cyan(),
        );
    }

    println!(
        "    {} {}",
        style("working directory:").dim(),
        style(std::env::current_dir().unwrap().display())
            .cyan()
            .dim()
    );
}

pub fn display_greeting() {
    println!("\nGoose is running! Enter your instructions, or try asking what goose can do.\n");
}

/// Display context window usage with both current and session totals
pub fn display_context_usage(total_tokens: usize, context_limit: usize) {
    use console::style;

    if context_limit == 0 {
        println!("Context: Error - context limit is zero");
        return;
    }

    // Calculate percentage used with bounds checking
    let percentage =
        (((total_tokens as f64 / context_limit as f64) * 100.0).round() as usize).min(100);

    // Create dot visualization with safety bounds
    let dot_count = 10;
    let filled_dots =
        (((percentage as f64 / 100.0) * dot_count as f64).round() as usize).min(dot_count);
    let empty_dots = dot_count - filled_dots;

    let filled = "●".repeat(filled_dots);
    let empty = "○".repeat(empty_dots);

    // Combine dots and apply color
    let dots = format!("{}{}", filled, empty);
    let colored_dots = if percentage < 50 {
        style(dots).green()
    } else if percentage < 85 {
        style(dots).yellow()
    } else {
        style(dots).red()
    };

    // Print the status line
    println!(
        "Context: {} {}% ({}/{} tokens)",
        colored_dots, percentage, total_tokens, context_limit
    );
}

pub struct McpSpinners {
    bars: HashMap<String, ProgressBar>,
    log_spinner: Option<ProgressBar>,

    multi_bar: MultiProgress,
}

impl McpSpinners {
    pub fn new() -> Self {
        McpSpinners {
            bars: HashMap::new(),
            log_spinner: None,
            multi_bar: MultiProgress::new(),
        }
    }

    pub fn log(&mut self, message: &str) {
        let spinner = self.log_spinner.get_or_insert_with(|| {
            let bar = self.multi_bar.add(
                ProgressBar::new_spinner()
                    .with_style(
                        ProgressStyle::with_template("{spinner:.green} {msg}")
                            .unwrap()
                            .tick_chars("⠋⠙⠚⠛⠓⠒⠊⠉"),
                    )
                    .with_message(message.to_string()),
            );
            bar.enable_steady_tick(Duration::from_millis(100));
            bar
        });

        spinner.set_message(message.to_string());
    }

    pub fn update(&mut self, token: &str, value: f64, total: Option<f64>, message: Option<&str>) {
        let bar = self.bars.entry(token.to_string()).or_insert_with(|| {
            if let Some(total) = total {
                self.multi_bar.add(
                    ProgressBar::new((total * 100.0) as u64).with_style(
                        ProgressStyle::with_template("[{elapsed}] {bar:40} {pos:>3}/{len:3} {msg}")
                            .unwrap(),
                    ),
                )
            } else {
                self.multi_bar.add(ProgressBar::new_spinner())
            }
        });
        bar.set_position((value * 100.0) as u64);
        if let Some(msg) = message {
            bar.set_message(msg.to_string());
        }
    }

    pub fn hide(&mut self) -> Result<(), Error> {
        self.bars.iter_mut().for_each(|(_, bar)| {
            bar.disable_steady_tick();
        });
        if let Some(spinner) = self.log_spinner.as_mut() {
            spinner.disable_steady_tick();
        }
        self.multi_bar.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_short_paths_unchanged() {
        assert_eq!(shorten_path("/usr/bin", false), "/usr/bin");
        assert_eq!(shorten_path("/a/b/c", false), "/a/b/c");
        assert_eq!(shorten_path("file.txt", false), "file.txt");
    }

    #[test]
    fn test_debug_mode_returns_full_path() {
        assert_eq!(
            shorten_path("/very/long/path/that/would/normally/be/shortened", true),
            "/very/long/path/that/would/normally/be/shortened"
        );
    }

    #[test]
    fn test_home_directory_conversion() {
        // Save the current home dir
        let original_home = env::var("HOME").ok();

        // Set a test home directory
        env::set_var("HOME", "/Users/testuser");

        assert_eq!(
            shorten_path("/Users/testuser/documents/file.txt", false),
            "~/documents/file.txt"
        );

        // A path that starts similarly to home but isn't in home
        assert_eq!(
            shorten_path("/Users/testuser2/documents/file.txt", false),
            "/Users/testuser2/documents/file.txt"
        );

        // Restore the original home dir
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_long_path_shortening() {
        assert_eq!(
            shorten_path(
                "/vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv/long/path/with/many/components/file.txt",
                false
            ),
            "/v/l/p/w/m/components/file.txt"
        );
    }
}
