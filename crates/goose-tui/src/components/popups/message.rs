use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use goose::conversation::message::MessageContent;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    Wrap,
};
use ratatui::Frame;
use std::time::Instant;

pub struct MessagePopup {
    scroll: u16,
    last_scroll_time: Option<Instant>,
    content_height: u16,
    viewport_height: u16,
}

impl Default for MessagePopup {
    fn default() -> Self {
        Self::new()
    }
}

impl MessagePopup {
    pub fn new() -> Self {
        Self {
            scroll: 0,
            last_scroll_time: None,
            content_height: 0,
            viewport_height: 0,
        }
    }

    fn max_scroll(&self) -> u16 {
        if self.content_height <= self.viewport_height {
            0
        } else {
            self.content_height.saturating_sub(1)
        }
    }
}

impl Component for MessagePopup {
    fn handle_event(&mut self, event: &Event, _state: &AppState) -> Result<Option<Action>> {
        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
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
        let msg_idx = match state.showing_message_info {
            Some(idx) => idx,
            None => return,
        };

        let message = match state.messages.get(msg_idx) {
            Some(m) => m,
            None => return,
        };

        let area = centered_rect(80, 80, area);
        f.render_widget(Clear, area);

        let mut lines = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(t) => {
                    for line in t.text.lines() {
                        lines.push(Line::from(line));
                    }
                }
                MessageContent::ToolRequest(req) => {
                    lines.push(Line::from(Span::styled(
                        "Tool Request:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                    if let Ok(call) = &req.tool_call {
                        lines.push(Line::from(format!("Name: {}", call.name)));
                        if let Some(args) = &call.arguments {
                            let json_str = serde_json::to_string_pretty(args).unwrap_or_default();
                            for line in json_str.lines() {
                                lines.push(Line::from(line.to_string()));
                            }
                        }
                    }
                }
                MessageContent::ToolResponse(resp) => {
                    lines.push(Line::from(Span::styled(
                        "Tool Output:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                    if let Ok(contents) = &resp.tool_result {
                        for content in contents {
                            if let rmcp::model::Content {
                                raw: rmcp::model::RawContent::Text(text_content),
                                ..
                            } = content
                            {
                                let text = &text_content.text;
                                let display_string = if let Ok(v) =
                                    serde_json::from_str::<serde_json::Value>(text)
                                {
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| text.to_string())
                                } else {
                                    text.to_string()
                                };
                                for line in display_string.lines() {
                                    lines.push(Line::from(line.to_string()));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            lines.push(Line::from(""));
        }

        let block = Block::default()
            .title("Message Details (Esc to Close)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(state.config.theme.base.background));

        // Calculate height with wrapping estimation
        let inner_width = area.width.saturating_sub(2).max(1);
        let mut wrapped_height = 0;
        for line in &lines {
            let width = line.width() as u16;
            if width == 0 {
                wrapped_height += 1;
            } else {
                wrapped_height += (width + inner_width - 1) / inner_width;
            }
        }

        self.content_height = wrapped_height;
        self.viewport_height = area.height.saturating_sub(2);

        if self.scroll > self.max_scroll() {
            self.scroll = self.max_scroll();
        }

        let p = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

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