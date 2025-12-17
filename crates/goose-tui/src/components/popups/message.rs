use super::{popup_block, render_hints, PopupScrollState};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
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
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;

pub struct MessagePopup {
    scroll_state: PopupScrollState,
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
            scroll_state: PopupScrollState::new(),
            cached_message_idx: None,
            cached_plain_text: String::new(),
        }
    }

    fn render_content(
        &self,
        message: &goose::conversation::message::Message,
        theme: &Theme,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for content in &message.content {
            match content {
                MessageContent::Text(t) => Self::render_text_content(&mut lines, t, theme),
                MessageContent::ToolRequest(req) => {
                    Self::render_tool_request(&mut lines, req, theme)
                }
                MessageContent::ToolResponse(resp) => {
                    Self::render_tool_response(&mut lines, resp, theme)
                }
                MessageContent::ToolConfirmationRequest(req) => {
                    Self::render_tool_confirmation(&mut lines, req, theme)
                }
                _ => {}
            }
            lines.push(Line::from(""));
        }
        lines
    }

    fn render_text_content(
        lines: &mut Vec<Line<'static>>,
        t: &rmcp::model::TextContent,
        theme: &Theme,
    ) {
        for line in t.text.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(theme.base.foreground),
            )));
        }
    }

    fn render_tool_request(
        lines: &mut Vec<Line<'static>>,
        req: &goose::conversation::message::ToolRequest,
        theme: &Theme,
    ) {
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
                Self::render_json_lines(lines, args, theme);
            }
        }
    }

    fn render_tool_response(
        lines: &mut Vec<Line<'static>>,
        resp: &goose::conversation::message::ToolResponse,
        theme: &Theme,
    ) {
        lines.push(Line::from(Span::styled(
            "Tool Output:",
            Style::default()
                .fg(theme.status.warning)
                .add_modifier(Modifier::BOLD),
        )));
        let Ok(call_tool_result) = &resp.tool_result else {
            return;
        };
        for content in &call_tool_result.content {
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
                let display = serde_json::from_str::<serde_json::Value>(text)
                    .ok()
                    .and_then(|v| serde_json::to_string_pretty(&v).ok())
                    .unwrap_or_else(|| text.to_string());
                for line in display.lines() {
                    let (sanitized, _) = sanitize_line(line);
                    lines.push(Line::from(Span::styled(
                        sanitized,
                        Style::default().fg(theme.base.foreground),
                    )));
                }
            }
        }
    }

    fn render_tool_confirmation(
        lines: &mut Vec<Line<'static>>,
        req: &goose::conversation::message::ToolConfirmationRequest,
        theme: &Theme,
    ) {
        lines.push(Line::from(Span::styled(
            "Tool Confirmation Request:",
            Style::default()
                .fg(theme.status.warning)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("Tool: {}", req.tool_name),
            Style::default().fg(theme.status.info),
        )));
        if let Some(warning) = &req.prompt {
            lines.push(Line::from(Span::styled(
                format!("⚠ {warning}"),
                Style::default().fg(theme.status.error),
            )));
        }
        lines.push(Line::from(Span::styled(
            "Arguments:",
            Style::default().fg(theme.base.foreground),
        )));
        Self::render_json_lines(lines, &req.arguments, theme);
    }

    fn render_json_lines<T: serde::Serialize>(
        lines: &mut Vec<Line<'static>>,
        value: &T,
        theme: &Theme,
    ) {
        let json_str = serde_json::to_string_pretty(value).unwrap_or_default();
        for line in json_str.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(theme.base.foreground),
            )));
        }
    }
}

impl Component for MessagePopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        let ActivePopup::MessageInfo(_) = state.active_popup else {
            return Ok(None);
        };

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some(pending) = &state.pending_confirmation {
                        return Ok(Some(Action::ConfirmToolCall {
                            id: pending.id.clone(),
                            approved: true,
                        }));
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    if let Some(pending) = &state.pending_confirmation {
                        return Ok(Some(Action::ConfirmToolCall {
                            id: pending.id.clone(),
                            approved: false,
                        }));
                    }
                }
                KeyCode::Char('c') => {
                    return Ok(Some(Action::CopyToClipboard(
                        self.cached_plain_text.clone(),
                    )));
                }
                KeyCode::Char('f') => {
                    if let Some(idx) = self.cached_message_idx {
                        return Ok(Some(Action::ForkFromMessage(idx)));
                    }
                }
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
        let ActivePopup::MessageInfo(msg_idx) = state.active_popup else {
            return;
        };

        let Some(message) = state.messages.get(msg_idx) else {
            return;
        };

        let theme = &state.config.theme;

        if self.cached_message_idx != Some(msg_idx) {
            self.cached_message_idx = Some(msg_idx);
            self.cached_plain_text = message_to_plain_text(message);
            self.scroll_state.reset();
        }

        let area = centered_rect(80, 80, area);
        f.render_widget(Clear, area);

        let block = popup_block(" Message Details ", theme);
        let inner_area = block.inner(area);

        let [content_area, hints_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner_area);

        f.render_widget(block, area);

        let lines = self.render_content(message, theme);

        let p = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });

        self.scroll_state.content_height = p.line_count(content_area.width) as u16;
        self.scroll_state.viewport_height = content_area.height;
        self.scroll_state.clamp();

        let p = p.scroll((self.scroll_state.scroll, 0));

        f.render_widget(p, content_area);

        self.scroll_state
            .render_transient_scrollbar(f, content_area);

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
