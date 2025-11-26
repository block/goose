use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use crate::utils::styles::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;

pub struct ThemePopup {
    list_state: ListState,
}

impl Default for ThemePopup {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemePopup {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }
}

impl Component for ThemePopup {
    fn handle_event(&mut self, event: &Event, _state: &AppState) -> Result<Option<Action>> {
        let names = Theme::all_names();
        let len = names.len();

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('j') | KeyCode::Down => {
                    let cur = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((cur + 1) % len));
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let cur = self.list_state.selected().unwrap_or(0);
                    self.list_state
                        .select(Some(cur.checked_sub(1).unwrap_or(len - 1)));
                }
                KeyCode::Enter => {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(&name) = names.get(idx) {
                            return Ok(Some(Action::ChangeTheme(name.to_string())));
                        }
                    }
                }
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => {
                    let cur = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some((cur + 1) % len));
                }
                MouseEventKind::ScrollUp => {
                    let cur = self.list_state.selected().unwrap_or(0);
                    self.list_state
                        .select(Some(cur.checked_sub(1).unwrap_or(len - 1)));
                }
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_theme_picker {
            return;
        }

        let theme = &state.config.theme;
        let names = Theme::all_names();

        let area = centered_rect(30, 50, area);
        f.render_widget(Clear, area);

        let items: Vec<ListItem> = names
            .iter()
            .map(|&name| {
                let is_current = name.eq_ignore_ascii_case(&theme.name);
                let style = if is_current {
                    Style::default()
                        .fg(theme.status.success)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.base.foreground)
                };
                let suffix = if is_current { " ✓" } else { "" };
                ListItem::new(Line::from(Span::styled(
                    format!("{name}{suffix}"),
                    style,
                )))
            })
            .collect();

        let block = Block::default()
            .title("Select Theme (Enter to apply)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(theme.base.background));

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(theme.base.selection)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }
}
