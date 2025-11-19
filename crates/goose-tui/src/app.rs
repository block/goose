use crate::client::{Client, ToolInfo};
use crate::config::{CustomCommand, TuiConfig};
use crate::event::{Event, EventHandler};
use crate::tui;
use crate::ui;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use goose::config::Config;
use goose::conversation::message::{Message, MessageContent};
use goose_server::routes::reply::MessageEvent;
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub enum BuilderState {
    SelectTool,
    ConfigureArgs {
        tool_idx: usize,
        field_values: HashMap<String, String>, // field_name -> value
        current_field_idx: usize,
    },
    NameCommand {
        tool_idx: usize,
        field_values: HashMap<String, String>,
    },
}

pub struct App<'a> {
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub input: TextArea<'a>,
    pub messages: Vec<Message>,
    pub config: TuiConfig,
    pub scroll_state: ListState,
    pub port: u16,
    pub client: Client,
    pub session_id: String,
    pub tx: Option<mpsc::UnboundedSender<Event>>,
    pub auto_scroll: bool,
    pub waiting_for_response: bool,
    pub todos: Vec<(String, bool)>, // (Text, Completed)
    pub animation_frame: usize,
    pub has_user_input_pending: bool,
    pub reply_task: Option<tokio::task::JoinHandle<()>>,
    pub focused_message_index: Option<usize>,
    pub popup_scroll: usize,
    pub visual_line_to_message_index: Vec<usize>,
    pub selectable_indices: Vec<usize>, // Indices of visual lines that are selectable
    pub showing_todo_popup: bool,
    pub todo_scroll: usize,
    pub slash_popup_scroll: usize,
    pub showing_help_popup: bool,
    pub showing_about_popup: bool,

    // Command Builder State
    pub showing_command_builder: bool,
    pub builder_state: BuilderState,
    pub available_tools: Vec<ToolInfo>,
    pub builder_list_state: ListState,
    pub builder_input: TextArea<'a>, // Reusing a textarea for builder inputs
}

impl<'a> App<'a> {
    pub async fn new(port: u16, secret_key: String) -> Result<Self> {
        let config = TuiConfig::load()?;
        let mut input = TextArea::default();
        input.set_cursor_line_style(ratatui::style::Style::default());
        input.set_placeholder_text("Type a message...");

        let mut builder_input = TextArea::default();
        builder_input.set_cursor_line_style(ratatui::style::Style::default());

        // Initialize Client
        let client = Client::new(port, secret_key);

        // Start agent via API to ensure provider/model are loaded from config
        let cwd = std::env::current_dir()?;
        let session = client
            .start_agent(cwd.to_string_lossy().to_string())
            .await?;
        let session_id = session.id;

        tracing::info!("Started agent session: {}", session_id);

        // Configure Provider
        let global_config = Config::global();
        let provider = global_config
            .get_goose_provider()
            .unwrap_or_else(|_| "openai".to_string());
        let model = global_config.get_goose_model().ok();

        if let Err(e) = client.update_provider(&session_id, provider, model).await {
            tracing::error!("Failed to update provider: {}", e);
            // We don't error out here, but the agent might fail to reply
        }

        // Load Enabled Extensions
        let extensions = goose::config::get_enabled_extensions();
        for ext in extensions {
            if let Err(e) = client.add_extension(&session_id, ext.clone()).await {
                tracing::error!("Failed to add extension {}: {}", ext.name(), e);
            }
        }

        // Fetch initial tools (best effort, might fail if server not ready, but it should be)
        let available_tools = client.get_tools(&session_id).await.unwrap_or_default();

        Ok(Self {
            should_quit: false,
            input_mode: InputMode::Editing,
            input,
            messages: vec![],
            config,
            scroll_state: ListState::default(),
            port,
            client,
            session_id,
            tx: None,
            auto_scroll: true,
            waiting_for_response: false,
            todos: vec![],
            animation_frame: 0,
            has_user_input_pending: true,
            reply_task: None,
            focused_message_index: None,
            popup_scroll: 0,
            visual_line_to_message_index: vec![],
            selectable_indices: vec![],
            showing_todo_popup: false,
            todo_scroll: 0,
            slash_popup_scroll: 0,
            showing_help_popup: false,
            showing_about_popup: false,

            showing_command_builder: false,
            builder_state: BuilderState::SelectTool,
            available_tools,
            builder_list_state: ListState::default(),
            builder_input,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = tui::init()?;
        let mut events = EventHandler::new(Duration::from_millis(250));

        // Store the sender so we can use it in handle_input
        self.tx = Some(events.sender());

        loop {
            tui.draw(|f| {
                ui::draw(f, self);
            })?;

            match events.next().await {
                Some(Event::Input(key)) => self.handle_input(key),
                Some(Event::Mouse(mouse)) => self.handle_mouse(mouse),
                Some(Event::Tick) => {
                    self.animation_frame = self.animation_frame.wrapping_add(1);
                }
                Some(Event::Resize(..)) => {} // Autohandled by Ratatui usually
                Some(Event::Server(msg)) => {
                    match msg {
                        MessageEvent::Message { message, .. } => {
                            // Parse Todos from ToolRequest (todo__todo_write arguments)
                            for content in &message.content {
                                if let MessageContent::ToolRequest(req) = content {
                                    if let Ok(tool_call) = &req.tool_call {
                                        if tool_call.name == "todo__todo_write" {
                                            if let Some(args) = &tool_call.arguments {
                                                if let Some(content_val) = args.get("content") {
                                                    if let Some(content_str) = content_val.as_str()
                                                    {
                                                        let mut new_todos = Vec::new();
                                                        let mut has_todos = false;
                                                        for line in content_str.lines() {
                                                            let trimmed = line.trim();
                                                            if let Some(task) =
                                                                trimmed.strip_prefix("- [ ] ")
                                                            {
                                                                new_todos.push((
                                                                    task.to_string(),
                                                                    false,
                                                                ));
                                                                has_todos = true;
                                                            } else if let Some(task) =
                                                                trimmed.strip_prefix("- [x] ")
                                                            {
                                                                new_todos
                                                                    .push((task.to_string(), true));
                                                                has_todos = true;
                                                            }
                                                        }

                                                        if has_todos {
                                                            self.todos = new_todos;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(last_msg) = self.messages.last_mut() {
                                if last_msg.id == message.id {
                                    // Merge content intelligently
                                    for content in message.content {
                                        if let MessageContent::Text(new_text) = &content {
                                            if let Some(MessageContent::Text(last_text)) =
                                                last_msg.content.last_mut()
                                            {
                                                last_text.text.push_str(&new_text.text);
                                                continue;
                                            }
                                        }
                                        last_msg.content.push(content);
                                    }
                                } else {
                                    self.messages.push(message);
                                }
                            } else {
                                self.messages.push(message);
                            }

                            if self.auto_scroll {
                                self.scroll_to_bottom();
                            }
                        }
                        MessageEvent::UpdateConversation { conversation } => {
                            self.messages = conversation.messages().clone();
                            if self.auto_scroll {
                                self.scroll_to_bottom();
                            }
                        }
                        MessageEvent::Error { error } => {
                            tracing::error!("Server error: {}", error);
                            self.waiting_for_response = false;
                            self.has_user_input_pending = true;
                            self.reply_task = None;
                        }
                        MessageEvent::Finish { .. } => {
                            // Generation finished
                            self.waiting_for_response = false;
                            self.has_user_input_pending = true;
                            self.reply_task = None;
                        }
                        _ => {}
                    }
                }
                Some(Event::Error(e)) => {
                    // For now just log errors
                    tracing::error!("Error: {}", e);
                }
                None => break,
            }

            if self.should_quit {
                break;
            }
        }

        tui::restore()?;
        Ok(())
    }

    fn scroll_to_bottom(&mut self) {
        // We don't know the exact number of *lines* here because that depends on rendering.
        // However, we can just select a very large index and Ratatui clamps it?
        // Actually, ListState index corresponds to the item index in the List.
        // But our ListItems are not 1-to-1 with Messages because we split Markdown into lines.
        // This makes `scroll_state` hard to use correctly from `App` without knowing the render logic.

        // Alternative: The `ui::draw_chat` function calculates the list items.
        // We might need to move the list generation logic to `App` or a helper,
        // OR just set a flag "scroll_to_end" and let the UI handle it?
        // For now, let's assume we want to scroll to the last *Message*.
        // BUT `draw_chat` flattens messages into multiple lines.

        // FIX: We need to track the visual line count.
        // Since we can't easily know it here, we will use a hack:
        // Select index usize::MAX. Ratatui's List handles out-of-bounds by showing the end?
        // No, it doesn't auto-clamp to last item if index > len.

        // Better approach for MVP:
        // Just unselect to let it stick to bottom? No, List doesn't auto-scroll.

        // We will simply set `auto_scroll` flag and handle the actual state update in `ui::draw`.
        // But `ui::draw` takes `&mut Frame` and `&mut App` but we want to separate logic.

        // Let's assume we unselect (None) means "auto scroll".
        self.scroll_state.select(None);
    }

    fn scroll_up(&mut self) {
        if self.showing_todo_popup {
            self.todo_scroll = self.todo_scroll.saturating_sub(1);
        } else if self.focused_message_index.is_some() {
            self.popup_scroll = self.popup_scroll.saturating_sub(1);
        } else {
            self.auto_scroll = false;
            let current = self.scroll_state.selected().unwrap_or(0);

            if self.input_mode == InputMode::Normal && !self.selectable_indices.is_empty() {
                // Jump to previous selectable item
                if let Some(&prev) = self.selectable_indices.iter().rev().find(|&&i| i < current) {
                    self.scroll_state.select(Some(prev));
                } else if let Some(&first) = self.selectable_indices.first() {
                    // If we are before the first selectable (or at it), maybe stay or go to first?
                    // If current > first, but no prev < current found? Impossible if sorted.
                    // If current <= first, we stay.
                    if current > first {
                        self.scroll_state.select(Some(first));
                    }
                }
            } else {
                self.scroll_state.select(Some(current.saturating_sub(1)));
            }
        }
    }

    fn scroll_down(&mut self) {
        if self.showing_todo_popup {
            self.todo_scroll += 1;
        } else if self.focused_message_index.is_some() {
            self.popup_scroll += 1;
        } else {
            let current = self.scroll_state.selected().unwrap_or(0);

            if self.input_mode == InputMode::Normal && !self.selectable_indices.is_empty() {
                // Jump to next selectable item
                if let Some(&next) = self.selectable_indices.iter().find(|&&i| i > current) {
                    self.scroll_state.select(Some(next));
                }
            } else {
                self.scroll_state.select(Some(current + 1));
            }
        }
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        match mouse.kind {
            crossterm::event::MouseEventKind::ScrollDown => self.scroll_down(),
            crossterm::event::MouseEventKind::ScrollUp => self.scroll_up(),
            _ => {}
        }
    }

    fn handle_input(&mut self, key: KeyEvent) {
        // Global: Toggle Todo Popup (Ctrl+T)
        if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.showing_todo_popup = !self.showing_todo_popup;
            self.todo_scroll = 0;
            return;
        }

        // Handle Todo Popup inputs
        if self.showing_todo_popup {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.showing_todo_popup = false;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll_up();
                }
                _ => {}
            }
            return;
        }

        if self.showing_command_builder {
            self.handle_builder_input(key);
            return;
        }

        if self.showing_help_popup {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.showing_help_popup = false;
            }
            return;
        }

        if self.showing_about_popup {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                self.showing_about_popup = false;
            }
            return;
        }

        // Handle Message Focus Popup inputs
        if self.focused_message_index.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.focused_message_index = None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll_up();
                }
                _ => {}
            }
            return;
        }

        // Global shortcuts (Ctrl+C)
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !self.input.is_empty() {
                // 1. Clear input if present
                self.input = TextArea::default();
                self.input
                    .set_cursor_line_style(ratatui::style::Style::default());
                self.input.set_placeholder_text("Type a message...");
            } else if self.waiting_for_response {
                // 2. Interrupt Goose if working
                if let Some(task) = self.reply_task.take() {
                    task.abort();
                }
                self.waiting_for_response = false;
                self.has_user_input_pending = true;
                tracing::info!("User interrupted response");
            } else {
                // 3. Exit if idle
                self.should_quit = true;
            }
            return;
        }

        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') | KeyCode::Char('i') => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Enter => {
                    // Try to expand message
                    let selected = self.scroll_state.selected().unwrap_or(0);
                    if let Some(msg_idx) = self.visual_line_to_message_index.get(selected) {
                        self.focused_message_index = Some(*msg_idx);
                        self.popup_scroll = 0;
                    } else {
                        // Fallback if no message selected (shouldn't happen often)
                        self.input_mode = InputMode::Editing;
                    }
                }
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll_up();
                }
                _ => {}
            },
            InputMode::Editing => {
                match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+j = Newline
                        self.input.insert_newline();
                    }
                    KeyCode::Enter => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            // Shift+Enter = Newline
                            self.input.insert_newline();
                        } else {
                            use crate::commands::{self, CommandResult};

                            // ... (inside handle_input, Enter case)

                            // Enter = Send message
                            let text = self.input.lines().join("\n");
                            if !text.trim().is_empty() {
                                let mut message_text = Some(text.clone());

                                // Check for slash command
                                if text.starts_with('/') {
                                    match commands::dispatch(self, &text) {
                                        CommandResult::Quit => {
                                            self.should_quit = true;
                                            return;
                                        }
                                        CommandResult::Continue => {
                                            // Command handled internally, clear input
                                            self.input = TextArea::default();
                                            self.input.set_cursor_line_style(
                                                ratatui::style::Style::default(),
                                            );
                                            self.input.set_placeholder_text("Type a message...");
                                            return;
                                        }
                                        CommandResult::Reply(msg) => {
                                            // Command generated a reply message (e.g. custom command alias)
                                            message_text = Some(msg);
                                        }
                                        CommandResult::ExecuteTool(msg) => {
                                            // Inject Assistant Tool Request
                                            self.messages.push(msg);
                                            self.auto_scroll = true;
                                            self.waiting_for_response = true;
                                            self.has_user_input_pending = false;

                                            if let Some(tx) = &self.tx {
                                                let client = self.client.clone();
                                                let messages = self.messages.clone();
                                                let session_id = self.session_id.clone();
                                                let tx = tx.clone();

                                                let task = tokio::spawn(async move {
                                                    if let Err(e) =
                                                        client.reply(messages, session_id, tx).await
                                                    {
                                                        tracing::error!(
                                                            "Failed to send message: {}",
                                                            e
                                                        );
                                                    }
                                                });
                                                self.reply_task = Some(task);
                                            }

                                            self.input = TextArea::default();
                                            self.input.set_cursor_line_style(
                                                ratatui::style::Style::default(),
                                            );
                                            self.input.set_placeholder_text("Type a message...");
                                            return;
                                        }
                                        CommandResult::OpenBuilder => {
                                            self.showing_command_builder = true;
                                            self.builder_state = BuilderState::SelectTool;
                                            self.builder_list_state.select(Some(0));
                                            self.input = TextArea::default();
                                            self.input.set_cursor_line_style(
                                                ratatui::style::Style::default(),
                                            );
                                            self.input.set_placeholder_text("Type a message...");
                                            return;
                                        }
                                    }
                                }

                                if let Some(final_text) = message_text {
                                    let message = Message::user().with_text(&final_text);
                                    self.messages.push(message.clone());
                                    self.auto_scroll = true;
                                    self.waiting_for_response = true;
                                    self.has_user_input_pending = false;

                                    if let Some(tx) = &self.tx {
                                        let client = self.client.clone();
                                        let messages = self.messages.clone();
                                        let session_id = self.session_id.clone();
                                        let tx = tx.clone();

                                        let task = tokio::spawn(async move {
                                            if let Err(e) =
                                                client.reply(messages, session_id, tx).await
                                            {
                                                tracing::error!("Failed to send message: {}", e);
                                            }
                                        });
                                        self.reply_task = Some(task);
                                    }

                                    self.input = TextArea::default();
                                    self.input
                                        .set_cursor_line_style(ratatui::style::Style::default());
                                    self.input.set_placeholder_text("Type a message...");
                                }
                            }
                        }
                    }
                    _ => {
                        self.input.input(key);
                    }
                }
            }
        }
    }

    fn handle_builder_input(&mut self, key: KeyEvent) {
        match &self.builder_state {
            BuilderState::SelectTool => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.showing_command_builder = false;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let current = self.builder_list_state.selected().unwrap_or(0);
                        let next = (current + 1).min(self.available_tools.len().saturating_sub(1));
                        self.builder_list_state.select(Some(next));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let current = self.builder_list_state.selected().unwrap_or(0);
                        let prev = current.saturating_sub(1);
                        self.builder_list_state.select(Some(prev));
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.builder_list_state.selected() {
                            // Initialize ConfigureArgs
                            self.builder_state = BuilderState::ConfigureArgs {
                                tool_idx: idx,
                                field_values: HashMap::new(),
                                current_field_idx: 0,
                            };
                            self.builder_input = TextArea::default();
                            // Placeholder for first arg
                            if let Some(tool) = self.available_tools.get(idx) {
                                if let Some(first_arg) = tool.parameters.first() {
                                    self.builder_input.set_placeholder_text(format!(
                                        "Value for '{}'...",
                                        first_arg
                                    ));
                                } else {
                                    // No args needed? Skip to Name
                                    self.builder_state = BuilderState::NameCommand {
                                        tool_idx: idx,
                                        field_values: HashMap::new(),
                                    };
                                    self.builder_input
                                        .set_placeholder_text("Slash command name (e.g. myls)...");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            BuilderState::ConfigureArgs {
                tool_idx,
                field_values,
                current_field_idx,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        self.showing_command_builder = false;
                    }
                    KeyCode::Enter => {
                        let tool = &self.available_tools[*tool_idx];
                        let arg_name = &tool.parameters[*current_field_idx];
                        let value = self.builder_input.lines().join("\n");

                        let mut new_values = field_values.clone();
                        new_values.insert(arg_name.clone(), value);

                        let next_idx = current_field_idx + 1;
                        if next_idx < tool.parameters.len() {
                            // Next arg
                            self.builder_state = BuilderState::ConfigureArgs {
                                tool_idx: *tool_idx,
                                field_values: new_values,
                                current_field_idx: next_idx,
                            };
                            self.builder_input = TextArea::default();
                            self.builder_input.set_placeholder_text(format!(
                                "Value for '{}'...",
                                tool.parameters[next_idx]
                            ));
                        } else {
                            // Done with args, go to Name
                            self.builder_state = BuilderState::NameCommand {
                                tool_idx: *tool_idx,
                                field_values: new_values,
                            };
                            self.builder_input = TextArea::default();
                            self.builder_input
                                .set_placeholder_text("Slash command name (e.g. myls)...");
                        }
                    }
                    _ => {
                        self.builder_input.input(key);
                    }
                }
            }
            BuilderState::NameCommand {
                tool_idx,
                field_values,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        self.showing_command_builder = false;
                    }
                    KeyCode::Enter => {
                        let name = self.builder_input.lines().join("\n").trim().to_string();
                        if !name.is_empty() {
                            // Save Command
                            let tool = &self.available_tools[*tool_idx];

                            // Convert field_values (Map) to JSON Value
                            let mut args_map = serde_json::Map::new();
                            for (k, v) in field_values {
                                args_map.insert(k.clone(), serde_json::Value::String(v.clone()));
                            }

                            let command = CustomCommand {
                                name: name.replace("/", ""), // Strip leading slash if user added it
                                description: format!("Alias for {}", tool.name),
                                tool: tool.name.clone(),
                                args: serde_json::Value::Object(args_map),
                            };

                            self.config.custom_commands.push(command);
                            if let Err(e) = self.config.save() {
                                tracing::error!("Failed to save config: {}", e);
                            }

                            self.showing_command_builder = false;
                        }
                    }
                    _ => {
                        self.builder_input.input(key);
                    }
                }
            }
        }
    }
}
