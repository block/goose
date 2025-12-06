//! CLI mode for goose-tui
//!
//! Provides an interactive command-line interface as a fallback when TUI
//! is not suitable. Uses the same embedded server and goose-client as TUI mode.

use crate::utils::spinner::SPINNER_FRAMES;
use anyhow::Result;
use goose::conversation::message::{Message, MessageContent, ToolConfirmationRequest};
use goose_client::Client;
use goose_server::routes::reply::MessageEvent;
use rustyline::error::ReadlineError;
use rustyline::{Cmd, DefaultEditor, EventHandler, KeyCode, KeyEvent, Modifiers};
use std::io::{stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_stream::StreamExt;

struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Spinner {
    fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    fn start(&mut self) {
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        self.handle = Some(tokio::spawn(async move {
            let mut frame_idx = 0;
            while running.load(Ordering::SeqCst) {
                print!(
                    "\r{}{} thinking...{}",
                    colors::DIM,
                    SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()],
                    colors::RESET
                );
                let _ = stdout().flush();
                frame_idx += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
            }
        }));
    }

    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        print!("\r                    \r");
        let _ = stdout().flush();
    }
}

mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";

    pub const CYAN: &str = "\x1b[36m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const RED: &str = "\x1b[31m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const GRAY: &str = "\x1b[90m";

    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BLACK: &str = "\x1b[30m";
}

fn print_session_header(provider: &str, model: Option<&str>, session_id: &str) {
    use colors::*;

    println!();
    println!("{BOLD}goose{RESET} {DIM}CLI mode{RESET}");
    println!("{DIM}provider:{RESET} {GREEN}{provider}{RESET}");
    if let Some(m) = model {
        println!("{DIM}model:{RESET}    {GREEN}{m}{RESET}");
    }
    println!("{DIM}session:{RESET}  {GRAY}{session_id}{RESET}");
    println!();
    println!("{DIM}Type your message and press Enter. Use /help for commands.{RESET}");
    println!();
}

fn print_context_usage(total_tokens: i32, context_limit: usize) {
    use colors::*;

    if context_limit == 0 {
        return;
    }

    let percentage = ((total_tokens as f64 / context_limit as f64) * 100.0).min(100.0) as usize;
    let bar_width = 20;
    let filled = (percentage * bar_width / 100).min(bar_width);
    let empty = bar_width - filled;

    let bar_color = if percentage < 50 {
        GREEN
    } else if percentage < 80 {
        YELLOW
    } else {
        RED
    };

    let bar = format!(
        "{}{}{}{}",
        bar_color,
        "█".repeat(filled),
        GRAY,
        "░".repeat(empty)
    );

    println!(
        "{DIM}context:{RESET} [{bar}{RESET}] {DIM}{percentage}% ({total_tokens}/{context_limit}){RESET}"
    );
}

fn get_terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(cols, _)| cols as usize)
        .unwrap_or(80)
}

fn print_tool_call(name: &str, args: Option<&serde_json::Map<String, serde_json::Value>>) {
    use colors::*;

    let term_width = get_terminal_width();
    // Reserve space for "│  key: " prefix (roughly 20 chars for box + key + spacing)
    let max_value_width = term_width.saturating_sub(20).max(30);

    println!();
    println!("{YELLOW}┌─ {BOLD}tool:{RESET} {MAGENTA}{name}{RESET}");

    if let Some(obj) = args {
        for (key, value) in obj {
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };

            // Replace newlines with spaces and truncate to fit terminal width
            let single_line = value_str.replace('\n', " ").replace('\r', "");
            let truncated = if single_line.len() > max_value_width {
                format!("{}...", &single_line[..max_value_width.saturating_sub(3)])
            } else {
                single_line
            };

            println!("{YELLOW}│{RESET}  {DIM}{key}:{RESET} {GRAY}{truncated}{RESET}");
        }
    }
    println!("{YELLOW}└─{RESET}");
}

fn print_tool_result(success: bool, tool_name: &str) {
    use colors::*;

    if success {
        println!("{GREEN}✓{RESET} {DIM}{tool_name} completed{RESET}");
    } else {
        println!("{RED}✗{RESET} {DIM}{tool_name} failed{RESET}");
    }
}

fn print_tool_confirmation(req: &ToolConfirmationRequest) -> bool {
    use colors::*;

    println!();
    println!("{BG_YELLOW}{BLACK}{BOLD} TOOL APPROVAL REQUIRED {RESET}");
    println!();

    if let Some(warning) = &req.prompt {
        println!("{RED}{BOLD}⚠ Security:{RESET} {YELLOW}{warning}{RESET}");
        println!();
    }

    println!("{BOLD}Tool:{RESET} {MAGENTA}{}{RESET}", req.tool_name);

    if !req.arguments.is_empty() {
        println!("{BOLD}Arguments:{RESET}");
        for (key, value) in &req.arguments {
            let value_str = match value {
                serde_json::Value::String(s) => {
                    if s.len() > 50 {
                        format!("{}...", &s[..47])
                    } else {
                        s.clone()
                    }
                }
                _ => {
                    let s = value.to_string();
                    if s.len() > 50 {
                        format!("{}...", &s[..47])
                    } else {
                        s
                    }
                }
            };
            println!("  {DIM}{key}:{RESET} {GRAY}{value_str}{RESET}");
        }
    }

    println!();
    print!("{BOLD}Allow this tool call? {RESET}[{GREEN}Y{RESET}/{RED}n{RESET}]: ");
    stdout().flush().ok();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
    } else {
        false
    }
}

fn print_help() {
    use colors::*;

    println!();
    println!("{BOLD}{CYAN}Commands:{RESET}");
    println!("  {GREEN}/help{RESET}, {GREEN}/?{RESET}       Show this help");
    println!("  {GREEN}/exit{RESET}, {GREEN}/quit{RESET}   Exit the CLI");
    println!("  {GREEN}/clear{RESET}         Clear the screen");
    println!();
    println!("{BOLD}{CYAN}Input:{RESET}");
    println!("  {DIM}Enter{RESET}          Send message");
    println!("  {DIM}Ctrl+J{RESET}         Insert newline (multi-line input)");
    println!("  {DIM}↑/↓{RESET}            Navigate command history");
    println!("  {DIM}Ctrl+C{RESET}         Cancel current input");
    println!("  {DIM}Ctrl+D{RESET}         Exit");
    println!();
}

pub async fn run_cli(
    client: Client,
    session_id: String,
    provider: String,
    model: Option<String>,
    context_limit: usize,
) -> Result<()> {
    use colors::*;

    let mut rl = DefaultEditor::new()?;
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('J'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::Newline),
    );
    let mut messages: Vec<Message> = Vec::new();
    let mut last_token_count: i32 = 0;

    print_session_header(&provider, model.as_deref(), &session_id);

    let mut pending_tools: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    loop {
        if last_token_count > 0 {
            print_context_usage(last_token_count, context_limit);
        }

        // Read input with readline support
        let prompt = format!("{CYAN}{BOLD}( O)>{RESET} ");
        let input = match rl.readline(&prompt) {
            Ok(line) => {
                if !line.trim().is_empty() {
                    let _ = rl.add_history_entry(&line);
                }
                line
            }
            Err(ReadlineError::Eof) => break,
            Err(ReadlineError::Interrupted) => {
                println!("{DIM}^C{RESET}");
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Handle commands
        match trimmed {
            "/exit" | "/quit" | "/q" => break,
            "/help" | "/?" => {
                print_help();
                continue;
            }
            "/clear" => {
                print!("\x1b[2J\x1b[H");
                stdout().flush()?;
                continue;
            }
            cmd if cmd.starts_with('/') => {
                println!(
                    "{YELLOW}Unknown command: {cmd}. Type /help for available commands.{RESET}"
                );
                continue;
            }
            _ => {}
        }

        // Send message and stream response
        let user_message = Message::user().with_text(trimmed);
        messages.push(user_message.clone());

        // Start spinner while waiting for response
        let mut spinner = Spinner::new();
        spinner.start();

        // Stream the response
        match stream_response(
            &client,
            &session_id,
            messages.clone(),
            &mut pending_tools,
            &mut last_token_count,
            &mut spinner,
        )
        .await
        {
            Ok(response_messages) => {
                messages.extend(response_messages);
            }
            Err(e) => {
                spinner.stop();
                println!("{RED}Error: {e}{RESET}");
            }
        }

        println!();
    }

    println!();
    println!("{DIM}Goodbye.{RESET}");
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn stream_response(
    client: &Client,
    session_id: &str,
    messages: Vec<Message>,
    pending_tools: &mut std::collections::HashMap<String, String>,
    last_token_count: &mut i32,
    spinner: &mut Spinner,
) -> Result<Vec<Message>> {
    use colors::*;

    let mut stream = client.reply(messages, session_id.to_string()).await?;
    let mut in_text_stream = false;
    let mut collected_messages: Vec<Message> = Vec::new();
    let mut pending_confirmation: Option<ToolConfirmationRequest> = None;
    let mut seen_tool_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut spinner_stopped = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => match event {
                MessageEvent::Message {
                    message,
                    token_state,
                } => {
                    *last_token_count = token_state.total_tokens;

                    for content in &message.content {
                        match content {
                            MessageContent::Text(t) => {
                                // Stop spinner when we get actual text content
                                if !spinner_stopped {
                                    spinner.stop();
                                    spinner_stopped = true;
                                }
                                // Each message contains only the new text delta - print directly
                                if !in_text_stream {
                                    println!();
                                    in_text_stream = true;
                                }
                                print!("{}", t.text);
                                stdout().flush()?;
                            }
                            MessageContent::ToolRequest(req) => {
                                // Stop spinner when we get a tool request
                                if !spinner_stopped {
                                    spinner.stop();
                                    spinner_stopped = true;
                                }
                                if in_text_stream {
                                    println!();
                                    in_text_stream = false;
                                }
                                if let Ok(call) = &req.tool_call {
                                    // Only print if we haven't seen this tool request before
                                    if seen_tool_ids.insert(req.id.clone()) {
                                        pending_tools.insert(req.id.clone(), call.name.to_string());
                                        print_tool_call(&call.name, call.arguments.as_ref());
                                    }
                                }
                            }
                            MessageContent::ToolResponse(resp) => {
                                // Stop spinner on tool response too
                                if !spinner_stopped {
                                    spinner.stop();
                                    spinner_stopped = true;
                                }
                                // Only print if we haven't seen this response before
                                if seen_tool_ids.insert(format!("resp_{}", resp.id)) {
                                    let tool_name = pending_tools
                                        .remove(&resp.id)
                                        .unwrap_or_else(|| "unknown".to_string());
                                    print_tool_result(resp.tool_result.is_ok(), &tool_name);
                                }
                            }
                            MessageContent::ToolConfirmationRequest(req) => {
                                // Stop spinner for confirmation requests
                                if !spinner_stopped {
                                    spinner.stop();
                                    spinner_stopped = true;
                                }
                                if in_text_stream {
                                    println!();
                                    in_text_stream = false;
                                }
                                pending_confirmation = Some(req.clone());
                            }
                            MessageContent::Thinking(t) => {
                                // Don't stop spinner for thinking - it's brief
                                if !t.thinking.is_empty() {
                                    println!(
                                        "{DIM}{ITALIC}💭 {}{RESET}",
                                        &t.thinking[..t.thinking.len().min(100)]
                                    );
                                }
                            }
                            _ => {}
                        }
                    }

                    collected_messages.push(message.clone());
                }
                MessageEvent::Finish { token_state, .. } => {
                    if !spinner_stopped {
                        spinner.stop();
                    }
                    *last_token_count = token_state.total_tokens;
                    if in_text_stream {
                        println!();
                    }
                    break;
                }
                MessageEvent::Error { error } => {
                    if !spinner_stopped {
                        spinner.stop();
                    }
                    if in_text_stream {
                        println!();
                    }
                    println!("{RED}Error: {error}{RESET}");
                    break;
                }
                _ => {}
            },
            Err(e) => {
                if !spinner_stopped {
                    spinner.stop();
                }
                if in_text_stream {
                    println!();
                }
                println!("{RED}Stream error: {e}{RESET}");
                break;
            }
        }

        // Handle pending confirmation outside the stream
        if let Some(req) = pending_confirmation.take() {
            let approved = print_tool_confirmation(&req);
            let action = if approved { "allow_once" } else { "deny" };

            if let Err(e) = client
                .confirm_tool_permission(session_id, &req.id, action)
                .await
            {
                println!("{RED}Failed to send confirmation: {e}{RESET}");
            } else if approved {
                println!("{GREEN}✓ Tool approved{RESET}");
            } else {
                println!("{YELLOW}✗ Tool denied{RESET}");
            }
        }
    }

    Ok(collected_messages)
}
