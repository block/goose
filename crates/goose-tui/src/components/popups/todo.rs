use super::{popup_block, render_hints, PopupScrollState};
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
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

#[derive(Default)]
pub struct TodoPopup {
    scroll_state: PopupScrollState,
}

impl TodoPopup {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for TodoPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.active_popup != ActivePopup::Todo {
            return Ok(None);
        }

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('j') | KeyCode::Down => self.scroll_state.scroll_by(1),
                KeyCode::Char('k') | KeyCode::Up => self.scroll_state.scroll_by(-1),
                KeyCode::PageDown => self.scroll_state.scroll_by(10),
                KeyCode::PageUp => self.scroll_state.scroll_by(-10),
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => self.scroll_state.scroll_by(3),
                MouseEventKind::ScrollUp => self.scroll_state.scroll_by(-3),
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

        let [content_area, hints_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .areas(area);

        f.render_widget(popup_block(" Todos ", theme), area);

        let mut lines = Vec::new();
        for item in &state.todos {
            let (prefix, style) = if item.done {
                (
                    "[x] ",
                    Style::default()
                        .fg(theme.base.border)
                        .add_modifier(Modifier::DIM),
                )
            } else {
                (
                    "[ ] ",
                    Style::default()
                        .fg(theme.base.foreground)
                        .add_modifier(Modifier::BOLD),
                )
            };
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&item.text, style),
            ]));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No tasks yet.",
                Style::default().fg(theme.base.border),
            )));
        }

        self.scroll_state.content_height = lines.len() as u16;
        self.scroll_state.viewport_height = content_area.height;
        self.scroll_state.clamp();

        let p = Paragraph::new(lines).scroll((self.scroll_state.scroll, 0));
        f.render_widget(p, content_area);

        self.scroll_state
            .render_transient_scrollbar(f, content_area);

        render_hints(f, hints_area, theme, &[("↑↓", "scroll"), ("Esc", "close")]);
    }
}
