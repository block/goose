use agent_client_protocol::{self as acp, Agent};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::process::Stdio;
use std::sync::Arc;
use textwrap;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{error, info};

/// Represents a message in the conversation
#[derive(Clone)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: chrono::DateTime<chrono::Local>,
}

/// Application state
struct App {
    input: String,
    messages: Vec<ChatMessage>,
    scroll_offset: u16,
    is_waiting: bool,
    current_response: String,
    session_id: Option<acp::SessionId>,
    status_message: String,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            scroll_offset: 0,
            is_waiting: false,
            current_response: String::new(),
            session_id: None,
            status_message: "Initializing...".to_string(),
        }
    }

    fn add_message(&mut self, role: String, content: String) {
        self.messages.push(ChatMessage {
            role,
            content,
            timestamp: chrono::Local::now(),
        });
    }

    fn append_to_current_response(&mut self, content: &str) {
        self.current_response.push_str(content);
    }

    fn finalize_current_response(&mut self) {
        if !self.current_response.is_empty() {
            self.add_message("Agent".to_string(), self.current_response.clone());
            self.current_response.clear();
        }
        self.is_waiting = false;
    }

    fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
    }
}

/// TUI Client implementation for ACP
struct TuiClient {
    message_tx: mpsc::UnboundedSender<String>,
}

impl TuiClient {
    fn new(message_tx: mpsc::UnboundedSender<String>) -> Self {
        Self { message_tx }
    }
}

impl acp::Client for TuiClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> anyhow::Result<acp::RequestPermissionResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn write_text_file(
        &self,
        _args: acp::WriteTextFileRequest,
    ) -> anyhow::Result<(), acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn read_text_file(
        &self,
        _args: acp::ReadTextFileRequest,
    ) -> anyhow::Result<acp::ReadTextFileResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(
        &self,
        args: acp::SessionNotification,
    ) -> anyhow::Result<(), acp::Error> {
        match args.update {
            acp::SessionUpdate::AgentMessageChunk { content } => {
                let text = match content {
                    acp::ContentBlock::Text(text_content) => text_content.text,
                    acp::ContentBlock::Image(_) => "[Image]".into(),
                    acp::ContentBlock::Audio(_) => "[Audio]".into(),
                    acp::ContentBlock::ResourceLink(resource_link) => {
                        format!("[Resource: {}]", resource_link.uri)
                    }
                    acp::ContentBlock::Resource(_) => "[Resource]".into(),
                };
                self.message_tx.send(text).ok();
            }
            _ => {}
        }
        Ok(())
    }
}

/// Initialize the terminal
fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Draw the UI
fn draw_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(10),   // Messages
            Constraint::Length(3), // Input
        ])
        .split(f.area());

    // Status bar
    let status_style = if app.is_waiting {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().bg(Color::Blue).fg(Color::White)
    };

    let status_text = if app.is_waiting {
        format!(" {} | Agent is thinking...", app.status_message)
    } else {
        format!(" {} | Ready", app.status_message)
    };

    let status = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default());
    f.render_widget(status, chunks[0]);

    // Messages area
    let messages_block = Block::default()
        .title(" Conversation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    // Format messages with word wrapping
    let mut formatted_lines = Vec::new();
    let width = (chunks[1].width - 4) as usize; // Account for borders and padding

    for msg in &app.messages {
        // Add role header
        let role_style = match msg.role.as_str() {
            "User" => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            "Agent" => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            _ => Style::default(),
        };

        formatted_lines.push(Line::from(vec![
            Span::styled(format!("[{}]", msg.role), role_style),
            Span::raw(format!(" {}", msg.timestamp.format("%H:%M:%S"))),
        ]));

        // Wrap and add message content
        let wrapped = textwrap::wrap(&msg.content, width);
        for line in wrapped {
            formatted_lines.push(Line::from(Span::raw(format!("  {}", line))));
        }

        formatted_lines.push(Line::from("")); // Empty line between messages
    }

    // Add current response if in progress
    if app.is_waiting && !app.current_response.is_empty() {
        formatted_lines.push(Line::from(vec![
            Span::styled(
                "[Agent]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" {}", chrono::Local::now().format("%H:%M:%S"))),
        ]));

        let wrapped = textwrap::wrap(&app.current_response, width);
        for line in wrapped {
            formatted_lines.push(Line::from(Span::raw(format!("  {}", line))));
        }

        // Add typing indicator
        formatted_lines.push(Line::from(Span::styled(
            "  ▋",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::SLOW_BLINK),
        )));
    }

    // Calculate scrolling
    let total_lines = formatted_lines.len() as u16;
    let visible_lines = chunks[1].height.saturating_sub(2); // Account for borders
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let scroll_offset = app.scroll_offset.min(max_scroll);

    // Create paragraph with scrolling
    let messages = Paragraph::new(formatted_lines)
        .block(messages_block)
        .scroll((scroll_offset, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(messages, chunks[1]);

    // Scrollbar
    if total_lines > visible_lines {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut scrollbar_state =
            ScrollbarState::new(total_lines as usize).position(scroll_offset as usize);

        let scrollbar_area = Rect {
            x: chunks[1].x + chunks[1].width - 1,
            y: chunks[1].y + 1,
            width: 1,
            height: chunks[1].height - 2,
        };

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // Input area
    let input_block = Block::default()
        .title(" Input (Enter to send, Ctrl-C to exit) ")
        .borders(Borders::ALL)
        .border_style(if app.is_waiting {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        });

    let input_text = if app.is_waiting {
        Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::DarkGray))
            .block(input_block)
    } else {
        Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::White))
            .block(input_block)
    };

    f.render_widget(input_text, chunks[2]);

    // Show cursor in input area when not waiting
    if !app.is_waiting {
        f.set_cursor_position((chunks[2].x + 1 + app.input.len() as u16, chunks[2].y + 1));
    }
}

/// Handle keyboard events
fn handle_key_event(key: KeyEvent, app: &mut App) -> (Option<(acp::SessionId, String)>, bool) {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return (None, true); // Exit
        }
        KeyCode::Enter if !app.is_waiting => {
            if !app.input.trim().is_empty() {
                let input = app.input.clone();
                app.add_message("User".to_string(), input.clone());
                app.input.clear();
                app.is_waiting = true;
                app.reset_scroll();

                // Return the prompt to send
                if let Some(session_id) = &app.session_id {
                    return (Some((session_id.clone(), input)), false);
                }
            }
        }
        KeyCode::Char(c) if !app.is_waiting => {
            app.input.push(c);
        }
        KeyCode::Backspace if !app.is_waiting => {
            app.input.pop();
        }
        KeyCode::Up => {
            app.scroll_up(1);
        }
        KeyCode::Down => {
            app.scroll_down(1);
        }
        KeyCode::PageUp => {
            app.scroll_up(10);
        }
        KeyCode::PageDown => {
            app.scroll_down(10);
        }
        KeyCode::Home => {
            app.scroll_offset = 0;
        }
        KeyCode::End => {
            app.scroll_offset = u16::MAX; // Will be clamped to max in draw_ui
        }
        _ => {}
    }
    (None, false)
}

/// Run the TUI client
pub async fn run_acp_tui() -> Result<()> {
    info!("Starting Goose ACP TUI client");

    // Start the goose acp agent as a subprocess
    let mut child = tokio::process::Command::new(std::env::current_exe()?)
        .arg("acp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();

    // Create message channel for agent responses
    let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    // Create the TUI client
    let client = TuiClient::new(message_tx);

    // Initialize app state
    let app = Arc::new(Mutex::new(App::new()));

    // Initialize terminal
    let terminal = init_terminal()?;

    // Wrap terminal in Arc<Mutex> for sharing
    let terminal = std::sync::Arc::new(tokio::sync::Mutex::new(terminal));
    let terminal_clone = terminal.clone();

    // The ClientSideConnection will spawn futures onto our Tokio runtime
    let local_set = tokio::task::LocalSet::new();
    let result = local_set
        .run_until(async move {
            // Set up the client connection
            let (conn, handle_io) =
                acp::ClientSideConnection::new(client, outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            // Wrap the connection in an Arc for sharing
            let conn = std::sync::Arc::new(conn);

            // Handle I/O in the background
            tokio::task::spawn_local(handle_io);

            // Initialize the agent connection
            conn.initialize(acp::InitializeRequest {
                protocol_version: acp::V1,
                client_capabilities: acp::ClientCapabilities::default(),
            })
            .await?;

            // Create a new session
            let response = conn
                .new_session(acp::NewSessionRequest {
                    mcp_servers: Vec::new(),
                    cwd: std::env::current_dir()?,
                })
                .await?;

            // Update app with session ID
            {
                let mut app = app.lock().await;
                app.session_id = Some(response.session_id.clone());
                app.status_message = "Connected to Goose".to_string();
            }

            // Handle incoming messages
            tokio::task::spawn_local({
                let app = app.clone();
                async move {
                    while let Some(content) = message_rx.recv().await {
                        let mut app = app.lock().await;
                        app.append_to_current_response(&content);
                    }
                }
            });

            // Main UI loop
            let mut should_exit = false;
            while !should_exit {
                // Draw UI
                {
                    let app = app.lock().await;
                    let mut terminal = terminal_clone.lock().await;
                    terminal.draw(|f| draw_ui(f, &app))?;
                }

                // Handle events with timeout
                if event::poll(std::time::Duration::from_millis(100))? {
                    if let Event::Key(key) = event::read()? {
                        let mut app = app.lock().await;
                        let (prompt_to_send, exit) = handle_key_event(key, &mut app);
                        should_exit = exit;

                        // Send prompt if needed
                        if let Some((session_id, input)) = prompt_to_send {
                            let conn = conn.clone();
                            tokio::task::spawn_local(async move {
                                let result = conn
                                    .prompt(acp::PromptRequest {
                                        session_id,
                                        prompt: vec![input.into()],
                                    })
                                    .await;
                                if let Err(e) = result {
                                    error!("Failed to send prompt: {}", e);
                                }
                            });
                        }
                    }
                }

                // Check if response is complete (simple heuristic - no new content for a bit)
                {
                    let mut app = app.lock().await;
                    if app.is_waiting && !app.current_response.is_empty() {
                        // This is a simplified check - in production you'd want proper message completion detection
                        app.finalize_current_response();
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        })
        .await;

    // Restore terminal
    let mut terminal = terminal.lock().await;
    restore_terminal(&mut terminal)?;

    // Kill the child process
    drop(child);

    result
}
