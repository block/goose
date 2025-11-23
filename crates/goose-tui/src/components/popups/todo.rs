use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
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

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
                ratatui::layout::Constraint::Percentage(percent_y),
                ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
                ratatui::layout::Constraint::Percentage(percent_x),
                ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn max_scroll(&self) -> u16 {
        if self.content_height <= self.viewport_height {
            0
        } else {
            self.content_height.saturating_sub(1)
        }
    }
}

impl Component for TodoPopup {
    fn handle_event(&mut self, event: &Event, _state: &AppState) -> Result<Option<Action>> {
        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll = self.scroll.saturating_add(1).min(self.max_scroll());
                    self.last_scroll_time = Some(Instant::now());
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll = self.scroll.saturating_sub(1);
                    self.last_scroll_time = Some(Instant::now());
                }
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => {
                    self.scroll = self.scroll.saturating_add(3).min(self.max_scroll());
                    self.last_scroll_time = Some(Instant::now());
                }
                MouseEventKind::ScrollUp => {
                    self.scroll = self.scroll.saturating_sub(3);
                    self.last_scroll_time = Some(Instant::now());
                }
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

        let area = Self::centered_rect(60, 60, area);
        f.render_widget(Clear, area);

        let mut lines = Vec::new();
        for item in &state.todos {
            let (prefix, style) = if item.done {
                ("[x] ", Style::default().fg(Color::DarkGray))
            } else {
                (
                    "[ ] ",
                    Style::default()
                        .fg(Color::White)
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
                Style::default().fg(Color::DarkGray),
            )));
        }

        let block = Block::default()
            .title("Todos (Esc to Close)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(state.config.theme.base.background));

        self.content_height = lines.len() as u16;
        self.viewport_height = area.height.saturating_sub(2);
        
        if self.scroll > self.max_scroll() {
            self.scroll = self.max_scroll();
        }

        let p = Paragraph::new(lines).block(block).scroll((self.scroll, 0));

        f.render_widget(p, area);

        if let Some(last) = self.last_scroll_time {
            if last.elapsed() < std::time::Duration::from_secs(1) {
                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(self.content_height as usize)
                    .viewport_content_length(self.viewport_height as usize)
                    .position(self.scroll as usize);

                f.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None),
                    area,
                    &mut scrollbar_state,
                );
            }
        }
    }
}