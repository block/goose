use super::Component;
use crate::at_mention;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{AppState, InputMode};
use crate::utils::json::has_input_placeholder;
use crate::utils::styles::breathing_color;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem};
use ratatui::Frame;
use std::path::Path;
use tui_textarea::TextArea;

pub struct InputComponent<'a> {
    textarea: TextArea<'a>,
    frame_count: usize,
    last_is_empty: bool,
    file_completions: Vec<(String, bool)>,
    completion_selected: usize,
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
        textarea.set_cursor_line_style(Style::default());
        Self {
            textarea,
            frame_count: 0,
            last_is_empty: true,
            file_completions: Vec::new(),
            completion_selected: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.textarea.lines().join("").trim().is_empty()
    }

    pub fn clear(&mut self) {
        self.textarea = TextArea::default();
        self.textarea.set_placeholder_text("Type a message...");
        self.textarea.set_cursor_line_style(Style::default());
        self.last_is_empty = true;
        self.file_completions.clear();
        self.completion_selected = 0;
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
            ("/session", false),
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
            "/session" => Some(Action::OpenSessionPicker),
            "/alias" => Some(Action::StartCommandBuilder),
            "/clear" => Some(Action::ClearChat),
            "/copy" | "/copymode" => Some(Action::ToggleCopyMode),
            "/compact" => {
                let message = goose::conversation::message::Message::user()
                    .with_text(goose::agents::MANUAL_COMPACT_TRIGGERS[0]);
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

fn complete_path(partial: &str, cwd: &Path) -> Vec<(String, bool)> {
    const MAX_COMPLETIONS: usize = 15;

    let (dir_path, prefix) = if partial.contains('/') {
        let last_slash = partial.rfind('/').unwrap();
        let dir_part = &partial[..=last_slash];
        let file_part = &partial[last_slash + 1..];

        let resolved_dir = if dir_part.starts_with("~/") {
            dirs::home_dir()
                .map(|h| h.join(&dir_part[2..]))
                .unwrap_or_else(|| cwd.join(dir_part))
        } else if Path::new(dir_part).is_absolute() {
            std::path::PathBuf::from(dir_part)
        } else {
            cwd.join(dir_part)
        };

        (resolved_dir, file_part.to_lowercase())
    } else {
        (cwd.to_path_buf(), partial.to_lowercase())
    };

    let Ok(entries) = std::fs::read_dir(&dir_path) else {
        return vec![];
    };

    let mut completions: Vec<(String, bool)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            !name.starts_with('.') && name.starts_with(&prefix)
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (name, is_dir)
        })
        .collect();

    completions.sort_by(|a, b| match (a.1, b.1) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
    });

    completions.truncate(MAX_COMPLETIONS);
    completions
}

fn replace_input_placeholder(args: &serde_json::Value, input: &str) -> serde_json::Value {
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
                    KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        self.textarea.insert_newline();
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.textarea.insert_newline();
                    }
                    KeyCode::Enter => {
                        let text = self.textarea.lines().join("\n");
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            if trimmed.starts_with('/') {
                                let cmd = trimmed.split_whitespace().next().unwrap_or("");
                                let safe_commands = [
                                    "/config",
                                    "/help",
                                    "/todos",
                                    "/theme",
                                    "/exit",
                                    "/quit",
                                    "/alias",
                                    "/copy",
                                    "/copymode",
                                    "/mode",
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

                            self.textarea = TextArea::default();
                            self.textarea.set_placeholder_text("Type a message...");
                            self.textarea.set_cursor_line_style(Style::default());

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
                    _ => {
                        self.textarea.input(*key);
                        let is_empty = self.is_empty();
                        if is_empty != self.last_is_empty {
                            self.last_is_empty = is_empty;
                            return Ok(Some(Action::SetInputEmpty(is_empty)));
                        }
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
                InputMode::Normal => theme.base.border,
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
