use super::{popup_block, render_hints};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;
use std::time::Instant;

pub struct TodoPopup {
    scroll: u16,
    last_scroll_time: Option<Instant>,
    content_height: u16,
    viewport_height: u16,
}

impl Default for TodoPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoPopup {
    pub fn new() -> Self {
        Self {
            scroll: 0,
            last_scroll_time: None,
            content_height: 0,
            viewport_height: 0,
        }
    }

    fn max_scroll(&self) -> u16 {
        self.content_height.saturating_sub(self.viewport_height)
    }

    fn scroll_by(&mut self, delta: i16) {
        if delta > 0 {
            self.scroll = self
                .scroll
                .saturating_add(delta as u16)
                .min(self.max_scroll());
        } else {
            self.scroll = self.scroll.saturating_sub((-delta) as u16);
        }
        self.last_scroll_time = Some(Instant::now());
    }
}

impl Component for TodoPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if !state.showing_todo {
            return Ok(None);
        }

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('j') | KeyCode::Down => self.scroll_by(1),
                KeyCode::Char('k') | KeyCode::Up => self.scroll_by(-1),
                KeyCode::PageDown => self.scroll_by(10),
                KeyCode::PageUp => self.scroll_by(-10),
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => self.scroll_by(3),
                MouseEventKind::ScrollUp => self.scroll_by(-3),
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_todo {
            return;
        }

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

        self.content_height = lines.len() as u16;
        self.viewport_height = content_area.height;

        if self.scroll > self.max_scroll() {
            self.scroll = self.max_scroll();
        }

        let p = Paragraph::new(lines).scroll((self.scroll, 0));
        f.render_widget(p, content_area);

        // Show scrollbar briefly after scrolling
        if let Some(last) = self.last_scroll_time {
            if last.elapsed() < std::time::Duration::from_secs(1)
                && self.content_height > self.viewport_height
            {
                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(self.content_height as usize)
                    .viewport_content_length(self.viewport_height as usize)
                    .position(self.scroll as usize);

                f.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("↑"))
                        .end_symbol(Some("↓")),
                    content_area,
                    &mut scrollbar_state,
                );
            }
        }

        render_hints(f, hints_area, theme, &[("↑↓", "scroll"), ("Esc", "close")]);
    }
}
