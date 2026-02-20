use super::Component;
use crate::at_mention;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{AppState, InputMode};
use crate::utils::file_completion::complete_path;
use crate::utils::json::has_input_placeholder;
use crate::utils::styles::breathing_color;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem};
use ratatui::Frame;
use ratatui_textarea::TextArea;

pub const MAX_HISTORY_ENTRIES: usize = 100;
pub const MAX_HISTORY_ENTRY_SIZE: usize = 10_000;

pub struct InputComponent<'a> {
    textarea: TextArea<'a>,
    frame_count: usize,
    last_is_empty: bool,
    file_completions: Vec<(String, bool)>,
    completion_selected: usize,
    history: Vec<String>,
    history_index: Option<usize>,
    saved_input: String,
}

impl<'a> Default for InputComponent<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> InputComponent<'a> {
    pub fn new() -> Self {
        Self {
            textarea: Self::create_textarea(),
            frame_count: 0,
            last_is_empty: true,
            file_completions: Vec::new(),
            completion_selected: 0,
            history: Vec::new(),
            history_index: None,
            saved_input: String::new(),
        }
    }

    fn create_textarea() -> TextArea<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type a message...");
        textarea.set_cursor_line_style(Style::default());
        textarea
    }

    pub fn is_empty(&self) -> bool {
        self.textarea.lines().join("").trim().is_empty()
    }

    pub fn clear(&mut self) {
        self.textarea = Self::create_textarea();
        self.last_is_empty = true;
        self.file_completions.clear();
        self.completion_selected = 0;
        self.reset_history_nav();
    }

    fn reset_history_nav(&mut self) {
        self.history_index = None;
        self.saved_input.clear();
    }

    fn add_to_history(&mut self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_HISTORY_ENTRY_SIZE {
            return;
        }
        if self.history.last().map(|s| s.as_str()) == Some(trimmed) {
            return;
        }
        self.history.push(trimmed.to_string());
        if self.history.len() > MAX_HISTORY_ENTRIES {
            self.history.remove(0);
        }
        self.reset_history_nav();
    }

    fn navigate_history(&mut self, delta: i32) {
        if self.history.is_empty() {
            return;
        }
        let len = self.history.len();
        let new_index = match self.history_index {
            None if delta < 0 => {
                self.saved_input = self.textarea.lines().join("\n");
                Some(len - 1)
            }
            None => return,
            Some(i) => {
                let next = i as i32 + delta;
                if next < 0 {
                    Some(0)
                } else if next >= len as i32 {
                    None
                } else {
                    Some(next as usize)
                }
            }
        };
        self.history_index = new_index;
        let content = match new_index {
            Some(i) => self.history.get(i).cloned().unwrap_or_default(),
            None => std::mem::take(&mut self.saved_input),
        };
        self.set_content(&content);
    }

    fn set_content(&mut self, content: &str) {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.split('\n').map(String::from).collect()
        };
        self.textarea = TextArea::new(lines);
        self.textarea.set_placeholder_text("Type a message...");
        self.textarea.set_cursor_line_style(Style::default());
        self.textarea
            .move_cursor(ratatui_textarea::CursorMove::Bottom);
        self.textarea.move_cursor(ratatui_textarea::CursorMove::End);
    }

    pub fn seed_history(&mut self, messages: &[goose::conversation::message::Message]) {
        self.history.clear();
        let mut is_first_user = true;
        for msg in messages {
            if msg.role == rmcp::model::Role::User {
                let text =
                    crate::hidden_blocks::strip_hidden_blocks(&msg.as_concat_text(), is_first_user);
                is_first_user = false;
                if !text.trim().is_empty() {
                    self.add_to_history(&text);
                }
            }
        }
        self.reset_history_nav();
    }

    pub fn lines_count(&self) -> u16 {
        self.textarea.lines().len() as u16
    }

    pub fn height(&self, max_height: u16) -> u16 {
        (self.lines_count() + 2).clamp(3, max_height)
    }

    fn render_command_autocomplete(&self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let Some(first_line) = self.textarea.lines().first() else {
            return;
        };
        let first_word = first_line.split_whitespace().next().unwrap_or(first_line);
        if !first_word.starts_with('/') {
            return;
        }

        let builtin_commands = vec![
            ("/exit", false),
            ("/quit", false),
            ("/help", false),
            ("/todos", false),
            ("/config", false),
            ("/mcp", false),
            ("/session", false),
            ("/schedule", false),
            ("/alias", false),
            ("/clear", false),
            ("/compact", false),
            ("/theme", true),
            ("/copy", false),
            ("/copymode", false),
            ("/mode", true),
        ];

        let custom: Vec<(String, bool)> = state
            .config
            .custom_commands
            .iter()
            .map(|c| {
                let has_input = has_input_placeholder(&c.args);
                (format!("/{}", c.name), has_input)
            })
            .collect();

        let mut all_commands: Vec<(&str, bool)> = builtin_commands;
        let custom_refs: Vec<(&str, bool)> = custom.iter().map(|(s, b)| (s.as_str(), *b)).collect();
        all_commands.extend(custom_refs);
        all_commands.sort_by(|a, b| a.0.cmp(b.0));

        let filtered: Vec<(&str, bool)> = all_commands
            .into_iter()
            .filter(|(c, _)| c.starts_with(first_word))
            .collect();

        if filtered.is_empty() {
            return;
        }

        let max_height = f.area().height / 2;
        let content_height = filtered.len() as u16 + 2;
        let height = content_height.min(max_height).max(3);
        let width = 30;
        let popup_area = Rect::new(area.x, area.y.saturating_sub(height), width, height);

        f.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|(c, accepts_args)| {
                if *accepts_args {
                    ListItem::new(ratatui::text::Line::from(vec![
                        Span::styled((*c).to_string(), Style::default().fg(theme.base.foreground)),
                        Span::styled(
                            " <args>".to_string(),
                            Style::default().fg(theme.status.warning),
                        ),
                    ]))
                } else {
                    ListItem::new(Span::styled(
                        (*c).to_string(),
                        Style::default().fg(theme.base.foreground),
                    ))
                }
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

    fn render_file_autocomplete(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;

        let Some(partial_path) = self.extract_active_mention() else {
            self.file_completions.clear();
            self.completion_selected = 0;
            return;
        };

        let cwd = std::env::current_dir().unwrap_or_default();
        self.file_completions = complete_path(&partial_path, &cwd);

        if self.file_completions.is_empty() {
            self.completion_selected = 0;
            return;
        }

        if self.completion_selected >= self.file_completions.len() {
            self.completion_selected = 0;
        }

        let max_height = f.area().height / 2;
        let content_height = self.file_completions.len() as u16 + 2;
        let height = content_height.min(max_height).max(3);
        let width = 50.min(area.width.saturating_sub(2));
        let popup_area = Rect::new(area.x, area.y.saturating_sub(height), width, height);

        f.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = self
            .file_completions
            .iter()
            .enumerate()
            .map(|(i, (name, is_dir))| {
                let is_selected = i == self.completion_selected;
                let display = if *is_dir {
                    format!("{name}/")
                } else {
                    name.clone()
                };
                let prefix = if is_selected { "> " } else { "  " };
                let color = if is_selected {
                    theme.status.thinking
                } else if *is_dir {
                    theme.status.info
                } else {
                    theme.base.foreground
                };
                ListItem::new(Span::styled(
                    format!("{prefix}{display}"),
                    Style::default().fg(color),
                ))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Files"),
            )
            .style(Style::default().bg(theme.base.background));

        f.render_widget(list, popup_area);
    }

    fn extract_active_mention(&self) -> Option<String> {
        let (row, col) = self.textarea.cursor();
        let line = self.textarea.lines().get(row)?;

        let byte_pos = line
            .char_indices()
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(line.len());

        let before_cursor = &line[..byte_pos];
        let at_pos = before_cursor.rfind('@')?;

        let before_at = &before_cursor[..at_pos];
        if !before_at.is_empty()
            && !before_at.ends_with(|c: char| c.is_whitespace() || "([{<".contains(c))
        {
            return None;
        }

        let partial = &before_cursor[at_pos + 1..];
        if partial.contains(|c: char| c.is_whitespace() || at_mention::PATH_TERMINATORS.contains(c))
        {
            return None;
        }

        Some(partial.to_string())
    }

    fn handle_slash_command(&self, cmd_line: &str, state: &AppState) -> Option<Action> {
        let trimmed = cmd_line.trim();
        let (cmd, trailing_args) = match trimmed.split_once(' ') {
            Some((c, rest)) => (c, rest.trim()),
            None => (trimmed, ""),
        };

        match cmd {
            "/exit" | "/quit" => Some(Action::Quit),
            "/help" => Some(Action::ToggleHelp),
            "/todos" => Some(Action::ToggleTodo),
            "/config" => Some(Action::OpenConfig),
            "/mcp" => Some(Action::OpenMcp),
            "/session" => Some(Action::OpenSessionPicker),
            "/schedule" | "/schedules" => Some(Action::OpenSchedulePopup),
            "/alias" => Some(Action::StartCommandBuilder),
            "/clear" => Some(Action::ClearChat),
            "/copy" | "/copymode" => Some(Action::ToggleCopyMode),
            "/compact" => {
                let message = goose::conversation::message::Message::user()
                    .with_text(goose::agents::COMPACT_TRIGGERS[0]);
                Some(Action::SendMessage(message))
            }
            "/theme" => {
                if !trailing_args.is_empty() {
                    Some(Action::ChangeTheme(trailing_args.to_string()))
                } else {
                    Some(Action::OpenThemePicker)
                }
            }
            "/mode" => {
                if trailing_args.is_empty() {
                    Some(Action::ShowFlash(
                        "Usage: /mode <auto|approve|chat|smart_approve>".to_string(),
                    ))
                } else {
                    Some(Action::SetGooseMode(trailing_args.to_lowercase()))
                }
            }
            _ => {
                let cmd_name = cmd.strip_prefix('/').unwrap_or(cmd);
                if let Some(custom) = state
                    .config
                    .custom_commands
                    .iter()
                    .find(|c| c.name == cmd_name)
                {
                    let processed_args = replace_input_placeholder(&custom.args, trailing_args);
                    let message_text = format!(
                        "Please run the tool '{}' with these arguments: {}",
                        custom.tool, processed_args
                    );
                    let message =
                        goose::conversation::message::Message::user().with_text(message_text);
                    Some(Action::SendMessage(message))
                } else {
                    None
                }
            }
        }
    }
}

pub fn replace_input_placeholder(args: &serde_json::Value, input: &str) -> serde_json::Value {
    match args {
        serde_json::Value::String(s) => serde_json::Value::String(s.replace("{input}", input)),
        serde_json::Value::Object(obj) => {
            let new_obj: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), replace_input_placeholder(v, input)))
                .collect();
            serde_json::Value::Object(new_obj)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.iter()
                .map(|v| replace_input_placeholder(v, input))
                .collect(),
        ),
        other => other.clone(),
    }
}

impl<'a> Component for InputComponent<'a> {
    #[allow(clippy::too_many_lines)]
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
            self.reset_history_nav();
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
                InputMode::Normal | InputMode::Visual => match key.code {
                    KeyCode::Char('i') | KeyCode::Char('e') => {
                        return Ok(Some(Action::ToggleInputMode))
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Esc => return Ok(Some(Action::ToggleInputMode)),
                    KeyCode::Tab | KeyCode::Enter if !self.file_completions.is_empty() => {
                        if let Some(partial) = self.extract_active_mention() {
                            if let Some((name, is_dir)) =
                                self.file_completions.get(self.completion_selected).cloned()
                            {
                                for _ in 0..partial.chars().count() {
                                    self.textarea.delete_char();
                                }
                                let suffix = if is_dir { "/" } else { " " };
                                self.textarea.insert_str(format!("{name}{suffix}"));
                                self.file_completions.clear();
                                self.completion_selected = 0;
                            }
                        }
                    }
                    KeyCode::Down if !self.file_completions.is_empty() => {
                        self.completion_selected =
                            (self.completion_selected + 1) % self.file_completions.len();
                    }
                    KeyCode::Up if !self.file_completions.is_empty() => {
                        self.completion_selected = self
                            .completion_selected
                            .checked_sub(1)
                            .unwrap_or(self.file_completions.len() - 1);
                    }
                    KeyCode::Up if self.textarea.cursor().0 == 0 => {
                        self.navigate_history(-1);
                    }
                    KeyCode::Down
                        if self.history_index.is_some()
                            && self.textarea.cursor().0
                                == self.textarea.lines().len().saturating_sub(1) =>
                    {
                        self.navigate_history(1);
                    }
                    KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        self.reset_history_nav();
                        self.textarea.insert_newline();
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.reset_history_nav();
                        self.textarea.insert_newline();
                    }
                    KeyCode::Enter => {
                        let text = self.textarea.lines().join("\n");
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            self.add_to_history(&text);

                            if trimmed.starts_with('/') {
                                let cmd = trimmed.split_whitespace().next().unwrap_or("");
                                let safe_commands = [
                                    "/config",
                                    "/mcp",
                                    "/help",
                                    "/todos",
                                    "/theme",
                                    "/exit",
                                    "/quit",
                                    "/alias",
                                    "/copy",
                                    "/copymode",
                                    "/mode",
                                    "/schedule",
                                    "/schedules",
                                ];

                                if safe_commands.contains(&cmd) {
                                    self.clear();
                                    if let Some(action) = self.handle_slash_command(trimmed, state)
                                    {
                                        return Ok(Some(action));
                                    }
                                }
                            }

                            if state.is_working {
                                return Ok(Some(Action::ShowFlash(
                                    "Goose is working... (Ctrl+C to interrupt)".to_string(),
                                )));
                            }

                            self.textarea = Self::create_textarea();

                            if trimmed.starts_with('/') {
                                if let Some(action) = self.handle_slash_command(trimmed, state) {
                                    return Ok(Some(action));
                                }
                            }

                            let cwd = std::env::current_dir().unwrap_or_default();
                            let result = at_mention::process(&text, &cwd);

                            if !result.errors.is_empty() {
                                let error_msg = result
                                    .errors
                                    .iter()
                                    .map(|(path, err)| format!("@{path}: {err}"))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                return Ok(Some(Action::ShowFlash(error_msg)));
                            }

                            let message = goose::conversation::message::Message::user()
                                .with_text(&result.augmented_text);
                            self.last_is_empty = true;

                            if !result.attachments.is_empty() {
                                let summary = result
                                    .attachments
                                    .iter()
                                    .map(|a| {
                                        let name = a
                                            .path
                                            .file_name()
                                            .map(|n| n.to_string_lossy())
                                            .unwrap_or_default();
                                        if a.truncated {
                                            format!("{} ({}+ lines)", name, a.line_count)
                                        } else {
                                            format!("{} ({} lines)", name, a.line_count)
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                return Ok(Some(Action::SendMessageWithFlash {
                                    message,
                                    flash: format!("📎 {summary}"),
                                }));
                            }

                            return Ok(Some(Action::SendMessage(message)));
                        }
                    }
                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete => {
                        self.reset_history_nav();
                        self.textarea.input(*key);
                        let is_empty = self.is_empty();
                        if is_empty != self.last_is_empty {
                            self.last_is_empty = is_empty;
                            return Ok(Some(Action::SetInputEmpty(is_empty)));
                        }
                    }
                    _ => {
                        self.textarea.input(*key);
                    }
                },
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
                InputMode::Normal | InputMode::Visual => theme.base.border,
            }
        };

        let border_color = breathing_color(base_color, self.frame_count, state.is_working);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Message")
            .border_style(Style::default().fg(border_color));

        self.textarea.set_block(block);
        self.textarea.set_style(
            Style::default()
                .fg(theme.base.foreground)
                .bg(theme.base.background),
        );

        self.textarea
            .set_cursor_style(Style::default().bg(theme.base.cursor));

        self.textarea
            .set_search_style(Style::default().fg(theme.status.thinking));
        let _ = self
            .textarea
            .set_search_pattern(r"@[\w./~\-]+(\\ [\w./~\-]+)*");

        f.render_widget(&self.textarea, area);

        if state.input_mode == InputMode::Editing {
            self.render_file_autocomplete(f, area, state);
            self.render_command_autocomplete(f, area, state);
        }
    }
}
