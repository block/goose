use super::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{AppState, InputMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem};
use ratatui::Frame;
use tui_textarea::TextArea;

pub struct InputComponent<'a> {
    textarea: TextArea<'a>,
    frame_count: usize,
    last_is_empty: bool,
}

impl<'a> Default for InputComponent<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> InputComponent<'a> {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type a message...");
        textarea.set_cursor_line_style(Style::default()); // Disable underline
        Self {
            textarea,
            frame_count: 0,
            last_is_empty: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.textarea.lines().join("").trim().is_empty()
    }

    pub fn clear(&mut self) {
        self.textarea = TextArea::default();
        self.textarea.set_placeholder_text("Type a message...");
        self.textarea.set_cursor_line_style(Style::default()); // Disable underline
        self.last_is_empty = true;
    }

    pub fn lines_count(&self) -> u16 {
        self.textarea.lines().len() as u16
    }

    pub fn height(&self, max_height: u16) -> u16 {
        // +2 for top/bottom borders of the textarea block
        (self.lines_count() + 2).clamp(3, max_height)
    }

    fn to_rgb(color: Color) -> (u8, u8, u8) {
        match color {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (128, 128, 128),
        }
    }
}

impl<'a> Component for InputComponent<'a> {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if let Event::Tick = event {
            self.frame_count = self.frame_count.wrapping_add(1);
            return Ok(None);
        }

        if let Event::Paste(text) = event {
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            if state.input_mode == InputMode::Normal {
                return Ok(Some(Action::ToggleInputMode));
            }
            self.textarea.insert_str(text);
            let is_empty = self.is_empty();
            if is_empty != self.last_is_empty {
                self.last_is_empty = is_empty;
                return Ok(Some(Action::SetInputEmpty(is_empty)));
            }
            return Ok(None);
        }

        if let Event::Input(key) = event {
            match state.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('i') => return Ok(Some(Action::ToggleInputMode)),
                    KeyCode::Char('e') => return Ok(Some(Action::ToggleInputMode)),
                    _ => {}
                },
                InputMode::Editing => {
                    match key.code {
                        KeyCode::Esc => return Ok(Some(Action::ToggleInputMode)),
                        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            self.textarea.insert_newline();
                        }
                        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.textarea.insert_newline();
                        }
                        KeyCode::Enter => {
                            if state.is_working {
                                return Ok(None);
                            }
                            let text = self.textarea.lines().join("\n");
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                self.textarea = TextArea::default();
                                self.textarea.set_placeholder_text("Type a message...");
                                self.textarea.set_cursor_line_style(Style::default()); // Disable underline

                                if trimmed.starts_with('/') {
                                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                                    let cmd = parts[0];
                                    match cmd {
                                        "/exit" | "/quit" => return Ok(Some(Action::Quit)),
                                        "/help" => return Ok(Some(Action::ToggleHelp)),
                                        "/todos" => return Ok(Some(Action::ToggleTodo)),
                                        "/session" => return Ok(Some(Action::OpenSessionPicker)),
                                        "/alias" => return Ok(Some(Action::StartCommandBuilder)),
                                        "/clear" => return Ok(Some(Action::ClearChat)),
                                        "/theme" => {
                                            if let Some(theme_name) = parts.get(1) {
                                                return Ok(Some(Action::ChangeTheme(
                                                    theme_name.to_string(),
                                                )));
                                            }
                                            return Ok(None);
                                        }
                                        // Custom Commands
                                        _ => {
                                            let cmd_name = cmd.strip_prefix('/').unwrap_or(cmd);
                                            if let Some(custom) = state
                                                .config
                                                .custom_commands
                                                .iter()
                                                .find(|c| c.name == cmd_name)
                                            {
                                                let message_text = format!(
                                                    "Please run the tool '{}' with these arguments: {}",
                                                    custom.tool, custom.args
                                                );
                                                let message =
                                                    goose::conversation::message::Message::user()
                                                        .with_text(message_text);
                                                return Ok(Some(Action::SendMessage(message)));
                                            }
                                        }
                                    }
                                }

                                let message =
                                    goose::conversation::message::Message::user().with_text(&text);
                                self.last_is_empty = true;
                                return Ok(Some(Action::SendMessage(message)));
                            }
                        }
                        _ => {
                            self.textarea.input(*key);
                            let is_empty = self.is_empty();
                            if is_empty != self.last_is_empty {
                                self.last_is_empty = is_empty;
                                return Ok(Some(Action::SetInputEmpty(is_empty)));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;

        let base_color = if state.is_working {
            theme.status.thinking
        } else {
            match state.input_mode {
                InputMode::Editing => theme.base.border_active,
                InputMode::Normal => theme.base.border,
            }
        };

        let (r, g, b) = Self::to_rgb(base_color);

        // Breathing effect
        let (dr, dg, db) = if state.is_working {
            let t = self.frame_count as f32 * 0.1;
            let factor = 0.85 + 0.15 * t.sin();
            (
                (r as f32 * factor) as u8,
                (g as f32 * factor) as u8,
                (b as f32 * factor) as u8,
            )
        } else {
            (r, g, b)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Message")
            .border_style(Style::default().fg(Color::Rgb(dr, dg, db)));

        self.textarea.set_block(block);
        self.textarea.set_style(
            Style::default()
                .fg(theme.base.foreground)
                .bg(theme.base.background),
        );

        self.textarea
            .set_cursor_style(Style::default().bg(theme.base.cursor));

        f.render_widget(&self.textarea, area);

        if state.input_mode == InputMode::Editing {
            if let Some(first_line) = self.textarea.lines().first() {
                if first_line.starts_with('/') {
                    let mut commands = vec![
                        "/exit", "/quit", "/help", "/todos", "/session", "/alias", "/clear",
                        "/theme",
                    ];
                    let custom: Vec<String> = state
                        .config
                        .custom_commands
                        .iter()
                        .map(|c| format!("/{}", c.name))
                        .collect();
                    let custom_refs: Vec<&str> = custom.iter().map(|s| s.as_str()).collect();
                    commands.extend(custom_refs);
                    commands.sort();

                    let filtered: Vec<&str> = commands
                        .iter()
                        .filter(|c| c.starts_with(first_line))
                        .cloned()
                        .collect();

                    if !filtered.is_empty() {
                        let max_height = f.area().height / 2;
                        let content_height = filtered.len() as u16 + 2;
                        let height = content_height.min(max_height).max(3);

                        let width = 30;
                        let popup_area =
                            Rect::new(area.x, area.y.saturating_sub(height), width, height);

                        f.render_widget(Clear, popup_area);

                        let items: Vec<ListItem> = filtered
                            .iter()
                            .map(|c| {
                                ListItem::new(Span::styled(
                                    *c,
                                    Style::default().fg(theme.base.foreground),
                                ))
                            })
                            .collect();

                        let list = List::new(items)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Rounded)
                                    .title("Commands"),
                            )
                            .style(Style::default().bg(theme.base.background));

                        f.render_widget(list, popup_area);
                    }
                }
            }
        }
    }
}
