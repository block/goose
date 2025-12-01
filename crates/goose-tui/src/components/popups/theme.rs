use super::{navigate_list, popup_block, render_hints};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
use crate::utils::layout::centered_rect;
use crate::utils::styles::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState};
use ratatui::Frame;

pub struct ThemePopup {
    list_state: ListState,
    original_theme: Option<String>,
}

impl Default for ThemePopup {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemePopup {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default().with_selected(Some(0)),
            original_theme: None,
        }
    }

    fn navigate(&mut self, delta: i32) -> Option<Action> {
        let names = Theme::all_names();
        let count = names.len();
        if let Some(next) = navigate_list(self.list_state.selected(), delta, count) {
            self.list_state.select(Some(next));
            if let Some(name) = names.get(next) {
                return Some(Action::PreviewTheme(name.clone()));
            }
        }
        None
    }
}

impl Component for ThemePopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.active_popup != ActivePopup::ThemePicker {
            return Ok(None);
        }

        if self.original_theme.is_none() {
            self.original_theme = Some(state.config.theme.name.clone());
        }

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if let Some(original) = self.original_theme.take() {
                        return Ok(Some(Action::PreviewTheme(original)));
                    }
                    return Ok(Some(Action::ClosePopup));
                }
                KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => {
                    return Ok(self.navigate(1));
                }
                KeyCode::Char('k') | KeyCode::Up | KeyCode::BackTab => {
                    return Ok(self.navigate(-1));
                }
                KeyCode::Enter => {
                    let names = Theme::all_names();
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(name) = names.get(idx) {
                            self.original_theme = None;
                            return Ok(Some(Action::ChangeTheme(name.clone())));
                        }
                    }
                }
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => return Ok(self.navigate(1)),
                MouseEventKind::ScrollUp => return Ok(self.navigate(-1)),
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let names = Theme::all_names();

        let area = centered_rect(30, 50, area);
        f.render_widget(Clear, area);

        let [list_area, hints_area] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
            .margin(1)
            .areas(area);

        f.render_widget(popup_block(" Select Theme ", theme), area);

        let items: Vec<ListItem> = names
            .iter()
            .map(|name| {
                let is_current = name.eq_ignore_ascii_case(&theme.name);
                let style = if is_current {
                    Style::default()
                        .fg(theme.status.success)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.base.foreground)
                };
                let suffix = if is_current { " ✓" } else { "" };
                ListItem::new(Line::from(Span::styled(format!("{name}{suffix}"), style)))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.base.selection)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, list_area, &mut self.list_state);

        render_hints(
            f,
            hints_area,
            theme,
            &[("↑↓", "nav"), ("Enter", "apply"), ("Esc", "close")],
        );
    }
}
