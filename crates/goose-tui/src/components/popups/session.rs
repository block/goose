use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState,
};
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
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
        }
    }
}

impl Component for SessionPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if !state.showing_session_picker {
            return Ok(None);
        }

        match event {
            Event::Input(key) => {
                let max_idx = state.available_sessions.len().saturating_sub(1);
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                    KeyCode::Char('j') | KeyCode::Down => {
                        let idx = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some((idx + 1).min(max_idx)));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let idx = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some(idx.saturating_sub(1)));
                    }
                    KeyCode::PageDown => {
                        let idx = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some((idx + 10).min(max_idx)));
                    }
                    KeyCode::PageUp => {
                        let idx = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some(idx.saturating_sub(10)));
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.list_state.selected() {
                            if let Some(session) = state.available_sessions.get(idx) {
                                return Ok(Some(Action::ResumeSession(session.id.clone())));
                            }
                        }
                    }
                    _ => {}
                }

                if let Some(idx) = self.list_state.selected() {
                    self.scroll_state = self.scroll_state.position(idx);
                }
            }
            Event::Mouse(mouse) => {
                let max_idx = state.available_sessions.len().saturating_sub(1);
                match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        let idx = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some((idx + 1).min(max_idx)));
                    }
                    MouseEventKind::ScrollUp => {
                        let idx = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some(idx.saturating_sub(1)));
                    }
                    _ => {}
                }

                if let Some(idx) = self.list_state.selected() {
                    self.scroll_state = self.scroll_state.position(idx);
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_session_picker {
            return;
        }

        let area = centered_rect(60, 60, area);
        f.render_widget(Clear, area);

        self.scroll_state = self
            .scroll_state
            .content_length(state.available_sessions.len());

        let items: Vec<ListItem> = state
            .available_sessions
            .iter()
            .map(|s| {
                let id = Span::styled(
                    &s.id,
                    Style::default().fg(Color::Cyan),
                );
                let count = Span::styled(
                    format!(" ({} msgs) ", s.message_count),
                    Style::default().fg(Color::DarkGray),
                );
                let name = Span::styled(&s.name, Style::default().fg(Color::White));
                ListItem::new(Line::from(vec![id, count, name]))
            })
            .collect();

        let block = Block::default()
            .title("Sessions (Enter to Resume)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(state.config.theme.base.background));

        if items.is_empty() {
            let p = Paragraph::new("No saved sessions found.")
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(p, area);
            return;
        }

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(state.config.theme.base.selection)
                .add_modifier(Modifier::BOLD),
        );

        if self.list_state.selected().is_none() && !state.available_sessions.is_empty() {
            self.list_state.select(Some(0));
            self.scroll_state = self.scroll_state.position(0);
        }

        f.render_stateful_widget(list, area, &mut self.list_state);

        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.scroll_state,
        );
    }
}
