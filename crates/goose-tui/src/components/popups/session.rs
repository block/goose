use super::{navigate_list, popup_block, render_hints, render_scrollbar};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph, ScrollbarState};
use ratatui::Frame;

pub struct SessionPopup {
    list_state: ListState,
    scroll_state: ScrollbarState,
}

impl Default for SessionPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionPopup {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default().with_selected(Some(0)),
            scroll_state: ScrollbarState::default(),
        }
    }

    fn item_count(&self, state: &AppState) -> usize {
        state.available_sessions.len() + 1 // +1 for "New session"
    }

    fn navigate(&mut self, delta: i32, state: &AppState) {
        let count = self.item_count(state);
        if let Some(next) = navigate_list(self.list_state.selected(), delta, count) {
            self.list_state.select(Some(next));
            self.scroll_state = self.scroll_state.position(next);
        }
    }
}

impl Component for SessionPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.active_popup != ActivePopup::SessionPicker {
            return Ok(None);
        }

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => self.navigate(1, state),
                KeyCode::Char('k') | KeyCode::Up | KeyCode::BackTab => self.navigate(-1, state),
                KeyCode::PageDown => self.navigate(10, state),
                KeyCode::PageUp => self.navigate(-10, state),
                KeyCode::Enter => {
                    if let Some(idx) = self.list_state.selected() {
                        if idx == 0 {
                            return Ok(Some(Action::CreateNewSession));
                        } else if let Some(session) = state.available_sessions.get(idx - 1) {
                            return Ok(Some(Action::ResumeSession(session.id.clone())));
                        }
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => self.navigate(1, state),
                MouseEventKind::ScrollUp => self.navigate(-1, state),
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let area = centered_rect(60, 60, area);
        f.render_widget(Clear, area);

        let [list_area, hints_area] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
            .margin(1)
            .areas(area);

        f.render_widget(popup_block(" Sessions ", theme), area);

        self.scroll_state = self.scroll_state.content_length(self.item_count(state));

        let mut items = vec![ListItem::new(Span::styled(
            "✚ New session",
            Style::default()
                .fg(theme.status.success)
                .add_modifier(Modifier::BOLD),
        ))];

        items.extend(state.available_sessions.iter().map(|s| {
            let id = Span::styled(&s.id, Style::default().fg(theme.status.info));
            let count = Span::styled(
                format!(" ({} msgs) ", s.message_count),
                Style::default().fg(theme.base.border),
            );
            let name = Span::styled(&s.name, Style::default().fg(theme.base.foreground));
            ListItem::new(Line::from(vec![id, count, name]))
        }));

        if items.len() == 1 {
            // Only "New session" item
            f.render_widget(
                Paragraph::new("No saved sessions found.")
                    .alignment(ratatui::layout::Alignment::Center)
                    .style(Style::default().fg(theme.base.border)),
                list_area,
            );
        } else {
            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(theme.base.selection)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, list_area, &mut self.list_state);
            render_scrollbar(f, list_area, &mut self.scroll_state);
        }

        render_hints(
            f,
            hints_area,
            theme,
            &[("↑↓", "nav"), ("Enter", "select"), ("Esc", "close")],
        );
    }
}
