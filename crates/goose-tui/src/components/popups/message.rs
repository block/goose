use super::{popup_block, render_hints};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use crate::utils::message_format::message_to_plain_text;
use crate::utils::sanitize::sanitize_line;
use crate::utils::styles::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use goose::conversation::message::MessageContent;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;
use std::time::Instant;

pub struct MessagePopup {
    scroll: u16,
    last_scroll_time: Option<Instant>,
    content_height: u16,
    viewport_height: u16,
    cached_message_idx: Option<usize>,
    cached_plain_text: String,
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
            cached_message_idx: None,
            cached_plain_text: String::new(),
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

    fn render_content(
        &self,
        message: &goose::conversation::message::Message,
        theme: &Theme,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(t) => {
                    for line in t.text.lines() {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(theme.base.foreground),
                        )));
                    }
                }
                MessageContent::ToolRequest(req) => {
                    lines.push(Line::from(Span::styled(
                        "Tool Request:",
                        Style::default()
                            .fg(theme.status.warning)
                            .add_modifier(Modifier::BOLD),
                    )));
                    if let Ok(call) = &req.tool_call {
                        lines.push(Line::from(Span::styled(
                            format!("Name: {}", call.name),
                            Style::default().fg(theme.status.info),
                        )));
                        if let Some(args) = &call.arguments {
                            let json_str = serde_json::to_string_pretty(args).unwrap_or_default();
                            for line in json_str.lines() {
                                lines.push(Line::from(Span::styled(
                                    line.to_string(),
                                    Style::default().fg(theme.base.foreground),
                                )));
                            }
                        }
                    }
                }
                MessageContent::ToolResponse(resp) => {
                    lines.push(Line::from(Span::styled(
                        "Tool Output:",
                        Style::default()
                            .fg(theme.status.warning)
                            .add_modifier(Modifier::BOLD),
                    )));
                    if let Ok(contents) = &resp.tool_result {
                        for content in contents {
                            if let Some(audience) = content.audience() {
                                if !audience.contains(&rmcp::model::Role::User) {
                                    continue;
                                }
                            }
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
                                    let (sanitized, _) = sanitize_line(line);
                                    lines.push(Line::from(Span::styled(
                                        sanitized,
                                        Style::default().fg(theme.base.foreground),
                                    )));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            lines.push(Line::from(""));
        }
        lines
    }

    fn copy_to_clipboard(&self) -> Result<(), String> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;
        clipboard
            .set_text(&self.cached_plain_text)
            .map_err(|e| format!("Clipboard error: {e}"))
    }
}

impl Component for MessagePopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.showing_message_info.is_none() {
            return Ok(None);
        }

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('c') => {
                    return match self.copy_to_clipboard() {
                        Ok(()) => Ok(Some(Action::ShowFlash("Copied to clipboard".to_string()))),
                        Err(e) => Ok(Some(Action::ShowFlash(e))),
                    };
                }
                KeyCode::Char('f') => {
                    if let Some(idx) = self.cached_message_idx {
                        return Ok(Some(Action::ForkFromMessage(idx)));
                    }
                }
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
        let msg_idx = match state.showing_message_info {
            Some(idx) => idx,
            None => return,
        };

        let message = match state.messages.get(msg_idx) {
            Some(m) => m,
            None => return,
        };

        let theme = &state.config.theme;

        if self.cached_message_idx != Some(msg_idx) {
            self.cached_message_idx = Some(msg_idx);
            self.cached_plain_text = message_to_plain_text(message);
            self.scroll = 0;
        }

        let area = centered_rect(80, 80, area);
        f.render_widget(Clear, area);

        let [content_area, hints_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .areas(area);

        f.render_widget(popup_block(" Message Details ", theme), area);

        let lines = self.render_content(message, theme);

        // Calculate wrapped height
        let inner_width = content_area.width.max(1);
        let mut wrapped_height = 0;
        for line in &lines {
            let width = line.width() as u16;
            if width == 0 {
                wrapped_height += 1;
            } else {
                wrapped_height += width.div_ceil(inner_width);
            }
        }

        self.content_height = wrapped_height;
        self.viewport_height = content_area.height;

        if self.scroll > self.max_scroll() {
            self.scroll = self.max_scroll();
        }

        let p = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

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

        render_hints(
            f,
            hints_area,
            theme,
            &[
                ("↑↓", "scroll"),
                ("c", "copy"),
                ("f", "fork"),
                ("Esc", "close"),
            ],
        );
    }
}
