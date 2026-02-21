use super::Component;
use crate::hidden_blocks::strip_hidden_blocks;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{AppState, InputMode, PendingToolConfirmation};
use crate::utils::ascii_art::render_logo_with_gradient;
use crate::utils::sanitize::sanitize_line;
use crate::utils::styles::{breathing_color, Theme};
use crate::utils::termimad_renderer::MarkdownRenderer;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use goose::conversation::message::{MessageContent, ToolConfirmationRequest};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

pub struct ChatComponent {
    list_state: ListState,
    cached_items: Vec<ListItem<'static>>,
    cached_mapping: Vec<usize>,
    cached_line_text: Vec<String>,
    display_mapping: Vec<usize>,
    display_line_text: Vec<String>,
    sealed_count: usize,
    last_tool_context: HashMap<String, (String, String)>,
    stick_to_bottom: bool,
    frame_count: usize,
    last_input_mode: InputMode,
    last_item_count: usize,
    last_width: u16,
    last_height: u16,
    last_theme_name: String,
    visual_anchor: Option<usize>,
    pending_g: bool,
}

impl Default for ChatComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatComponent {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            cached_items: Vec::new(),
            cached_mapping: Vec::new(),
            cached_line_text: Vec::new(),
            display_mapping: Vec::new(),
            display_line_text: Vec::new(),
            sealed_count: 0,
            last_tool_context: HashMap::new(),
            stick_to_bottom: true,
            frame_count: 0,
            last_input_mode: InputMode::Normal,
            last_item_count: 0,
            last_width: 0,
            last_height: 0,
            last_theme_name: String::new(),
            visual_anchor: None,
            pending_g: false,
        }
    }

    fn current_display_index(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    fn selection_range(&self) -> Option<std::ops::RangeInclusive<usize>> {
        let anchor = self.visual_anchor?;
        let cursor = self.current_display_index();
        Some(anchor.min(cursor)..=anchor.max(cursor))
    }

    fn selected_message_indices(&self) -> Vec<usize> {
        self.selection_range()
            .map(|range| {
                range
                    .filter_map(|i| self.display_mapping.get(i).copied())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn build_selection_text(&self) -> String {
        self.selection_range()
            .map(|range| {
                range
                    .filter_map(|i| self.display_line_text.get(i))
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    fn swap_anchor_cursor(&mut self) {
        if let Some(old_anchor) = self.visual_anchor {
            let current = self.current_display_index();
            if old_anchor != current {
                self.visual_anchor = Some(current);
                self.list_state.select(Some(old_anchor));
            }
        }
    }

    fn exit_visual_mode(&mut self) {
        self.visual_anchor = None;
        self.pending_g = false;
    }

    fn handle_visual_mode(&mut self, key: KeyCode, _state: &AppState) -> Result<Option<Action>> {
        if key != KeyCode::Char('g') {
            self.pending_g = false;
        }

        match key {
            KeyCode::Char('y') | KeyCode::Char('c') => {
                let text = self.build_selection_text();
                let count = self.selected_message_indices().len();
                self.exit_visual_mode();
                Ok(Some(Action::YankVisualSelection { text, count }))
            }
            KeyCode::Char('o') => {
                self.swap_anchor_cursor();
                Ok(None)
            }
            KeyCode::Char('g') => {
                if self.pending_g {
                    self.pending_g = false;
                    self.stick_to_bottom = false;
                    self.list_state.select(Some(0));
                } else {
                    self.pending_g = true;
                }
                Ok(None)
            }
            KeyCode::Char('G') => {
                self.stick_to_bottom = true;
                if self.last_item_count > 0 {
                    self.list_state
                        .select(Some(self.last_item_count.saturating_sub(1)));
                }
                Ok(None)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let cur = self.list_state.selected().unwrap_or(0);
                let next = cur + 1;
                self.list_state.select(Some(next));
                if next >= self.last_item_count.saturating_sub(1) {
                    self.stick_to_bottom = true;
                }
                Ok(None)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.stick_to_bottom = false;
                let cur = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some(cur.saturating_sub(1)));
                Ok(None)
            }
            KeyCode::Esc | KeyCode::Char('V') => {
                self.exit_visual_mode();
                Ok(Some(Action::ExitVisualMode))
            }
            _ => Ok(None),
        }
    }

    fn line_to_plain_text(line: &Line) -> String {
        line.spans.iter().map(|s| &*s.content).collect()
    }

    fn render_message(
        msg_idx: usize,
        message: &goose::conversation::message::Message,
        width: usize,
        theme: &Theme,
        last_tool_context: &mut HashMap<String, (String, String)>,
        pending_confirmation: Option<&PendingToolConfirmation>,
    ) -> (Vec<ListItem<'static>>, Vec<usize>, Vec<String>) {
        match message.role {
            rmcp::model::Role::User => {
                Self::render_user_message(msg_idx, message, width, theme, last_tool_context)
            }
            rmcp::model::Role::Assistant => Self::render_assistant_message(
                msg_idx,
                message,
                width,
                theme,
                last_tool_context,
                pending_confirmation,
            ),
        }
    }

    fn render_user_message(
        msg_idx: usize,
        message: &goose::conversation::message::Message,
        width: usize,
        theme: &Theme,
        last_tool_context: &mut HashMap<String, (String, String)>,
    ) -> (Vec<ListItem<'static>>, Vec<usize>, Vec<String>) {
        let mut items = Vec::new();
        let mut map = Vec::new();
        let mut texts = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(t) => Self::render_user_text(
                    t, msg_idx, width, theme, &mut items, &mut map, &mut texts,
                ),
                MessageContent::ToolResponse(resp) => Self::render_tool_response(
                    resp,
                    msg_idx,
                    width,
                    theme,
                    last_tool_context,
                    &mut items,
                    &mut map,
                    &mut texts,
                ),
                _ => {}
            }
        }
        items.push(ListItem::new(Line::from("")));
        map.push(msg_idx);
        texts.push(String::new());
        (items, map, texts)
    }

    fn render_user_text(
        t: &rmcp::model::TextContent,
        msg_idx: usize,
        width: usize,
        theme: &Theme,
        items: &mut Vec<ListItem<'static>>,
        map: &mut Vec<usize>,
        texts: &mut Vec<String>,
    ) {
        let display_text = strip_hidden_blocks(&t.text, msg_idx == 0);

        let user_text_style = Style::default().fg(theme.base.user_message_foreground);
        let renderer = MarkdownRenderer::new(theme, Some(user_text_style));
        let mut rendered_lines = renderer.render_lines(&display_text, width.saturating_sub(4));

        if !display_text.ends_with("\n\n")
            && rendered_lines
                .last()
                .is_some_and(|line| line.spans.is_empty() || line.width() == 0)
        {
            rendered_lines.pop();
        }

        for line in rendered_lines {
            let plain = Self::line_to_plain_text(&line);
            let mut spans = vec![Span::styled("│ ", Style::default().fg(Color::DarkGray))];
            spans.extend(line.spans);
            items.push(ListItem::new(Line::from(spans)));
            map.push(msg_idx);
            texts.push(plain);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tool_response(
        resp: &goose::conversation::message::ToolResponse,
        msg_idx: usize,
        width: usize,
        theme: &Theme,
        last_tool_context: &mut HashMap<String, (String, String)>,
        items: &mut Vec<ListItem<'static>>,
        map: &mut Vec<usize>,
        texts: &mut Vec<String>,
    ) {
        let (tool_name, tool_args) = last_tool_context
            .get(&resp.id)
            .cloned()
            .unwrap_or(("Unknown".to_string(), "".to_string()));
        let color = if resp.tool_result.is_ok() {
            theme.status.success
        } else {
            theme.status.error
        };

        Self::render_tool_response_header(
            &tool_name, &tool_args, width, color, items, map, texts, msg_idx,
        );

        if let Ok(call_tool_result) = &resp.tool_result {
            Self::render_tool_response_body(
                &call_tool_result.content,
                width,
                items,
                map,
                texts,
                msg_idx,
            );
        }

        let footer = format!("╰{:─<width$}╯", "", width = width.saturating_sub(2));
        items.push(ListItem::new(Line::from(Span::styled(
            footer,
            Style::default().fg(Color::DarkGray),
        ))));
        map.push(msg_idx);
        texts.push(String::new());
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tool_response_header(
        tool_name: &str,
        tool_args: &str,
        width: usize,
        color: Color,
        items: &mut Vec<ListItem<'static>>,
        map: &mut Vec<usize>,
        texts: &mut Vec<String>,
        msg_idx: usize,
    ) {
        let max_args = 50;
        let display_args = if tool_args.chars().count() > max_args {
            format!(
                "{}...",
                tool_args.chars().take(max_args).collect::<String>()
            )
        } else {
            tool_args.to_string()
        };

        let header = format!("{tool_name} {display_args}");
        let header_width = UnicodeWidthStr::width(header.as_str());
        let fixed = 5;
        let padding = width.saturating_sub(header_width + fixed);

        let header_line = Line::from(vec![
            Span::styled("╭─ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                tool_name.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {display_args} "),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!("{:─<width$}╮", "", width = padding),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        let plain = format!("{tool_name} {display_args}");
        items.push(ListItem::new(header_line));
        map.push(msg_idx);
        texts.push(plain);
    }

    fn render_tool_response_body(
        contents: &[rmcp::model::Content],
        width: usize,
        items: &mut Vec<ListItem<'static>>,
        map: &mut Vec<usize>,
        texts: &mut Vec<String>,
        msg_idx: usize,
    ) {
        let max_lines = 10;
        let content_width = width.saturating_sub(3);
        let mut line_count = 0;

        let user_content: Vec<_> = contents
            .iter()
            .filter(|c| {
                c.audience()
                    .is_none_or(|aud| aud.contains(&rmcp::model::Role::User))
            })
            .collect();
        let display_content: Vec<_> = if user_content.is_empty() {
            contents.iter().collect()
        } else {
            user_content
        };

        for content in display_content {
            if let rmcp::model::Content {
                raw: rmcp::model::RawContent::Text(text_content),
                ..
            } = content
            {
                for line in text_content.text.lines() {
                    if line_count >= max_lines {
                        break;
                    }
                    let box_line = Self::format_box_line(line, content_width);
                    items.push(ListItem::new(Line::from(Span::styled(
                        box_line,
                        Style::default().fg(Color::Gray),
                    ))));
                    map.push(msg_idx);
                    texts.push(line.to_string());
                    line_count += 1;
                }
            }
        }

        if line_count >= max_lines {
            let truncated = Self::format_box_line("... (truncated)", content_width);
            items.push(ListItem::new(Line::from(Span::styled(
                truncated,
                Style::default().fg(Color::DarkGray),
            ))));
            map.push(msg_idx);
            texts.push("... (truncated)".to_string());
        }
    }

    fn format_box_line(line: &str, content_width: usize) -> String {
        let (sanitized, line_width) = sanitize_line(line);
        let (truncated_line, truncated_width) = if line_width > content_width {
            let mut w = 0;
            let mut s = String::new();
            for c in sanitized.chars() {
                let cw = UnicodeWidthStr::width(c.to_string().as_str());
                if w + cw > content_width {
                    break;
                }
                w += cw;
                s.push(c);
            }
            (s, w)
        } else {
            (sanitized, line_width)
        };
        let pad = content_width.saturating_sub(truncated_width);
        format!("│ {}{: <width$}│", truncated_line, "", width = pad)
    }

    fn render_assistant_message(
        msg_idx: usize,
        message: &goose::conversation::message::Message,
        width: usize,
        theme: &Theme,
        last_tool_context: &mut HashMap<String, (String, String)>,
        pending_confirmation: Option<&PendingToolConfirmation>,
    ) -> (Vec<ListItem<'static>>, Vec<usize>, Vec<String>) {
        let mut items = Vec::new();
        let mut map = Vec::new();
        let mut texts = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(t) => {
                    let renderer = MarkdownRenderer::new(theme, None);
                    let rendered_lines = renderer.render_lines(&t.text, width);
                    for line in rendered_lines {
                        let plain = Self::line_to_plain_text(&line);
                        items.push(ListItem::new(line));
                        map.push(msg_idx);
                        texts.push(plain);
                    }
                }
                MessageContent::ToolRequest(req) => {
                    if let Ok(call) = &req.tool_call {
                        let name = call.name.clone();
                        let args = if let Some(a) = &call.arguments {
                            serde_json::to_string(a).unwrap_or_default()
                        } else {
                            "".to_string()
                        };
                        last_tool_context.insert(req.id.clone(), (name.to_string(), args));

                        let line = Line::from(vec![
                            Span::styled("▶ Tool: ", Style::default().fg(theme.status.warning)),
                            Span::styled(
                                format!("{name} (args hidden)"),
                                Style::default().fg(theme.base.foreground),
                            ),
                        ]);
                        let plain = format!("Tool: {name}");
                        items.push(ListItem::new(line));
                        map.push(msg_idx);
                        texts.push(plain);
                    }
                }
                MessageContent::ToolConfirmationRequest(req) => {
                    let is_pending = pending_confirmation
                        .map(|p| p.id == req.id)
                        .unwrap_or(false);
                    let (conf_items, conf_map, conf_texts) =
                        Self::render_tool_confirmation(msg_idx, req, width, theme, is_pending);
                    items.extend(conf_items);
                    map.extend(conf_map);
                    texts.extend(conf_texts);
                }
                MessageContent::Thinking(t) => {
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled("🤔 ", Style::default()),
                        Span::styled(
                            t.thinking.clone(),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ])));
                    map.push(msg_idx);
                    texts.push(t.thinking.clone());
                }
                _ => {}
            }
        }
        items.push(ListItem::new(Line::from("")));
        map.push(msg_idx);
        texts.push(String::new());
        (items, map, texts)
    }

    fn render_tool_confirmation(
        msg_idx: usize,
        req: &ToolConfirmationRequest,
        width: usize,
        theme: &Theme,
        is_pending: bool,
    ) -> (Vec<ListItem<'static>>, Vec<usize>, Vec<String>) {
        let mut items = Vec::new();
        let mut map = Vec::new();
        let mut texts = Vec::new();

        let border_color = if is_pending {
            theme.status.warning
        } else {
            Color::DarkGray
        };

        if let Some(warning) = &req.prompt {
            let warning_line = Line::from(vec![
                Span::styled(
                    "⚠ Security: ",
                    Style::default()
                        .fg(theme.status.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(warning.clone(), Style::default().fg(theme.status.warning)),
            ]);
            items.push(ListItem::new(warning_line));
            map.push(msg_idx);
            texts.push(format!("Security: {warning}"));
        }

        let status = if is_pending {
            "AWAITING APPROVAL"
        } else {
            "CONFIRMED"
        };
        let header_text = format!("┌─ {} ─ {} ", req.tool_name, status);
        let header_width = UnicodeWidthStr::width(header_text.as_str());
        let padding = width.saturating_sub(header_width + 1);

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                header_text.clone(),
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:─<w$}┐", "", w = padding),
                Style::default().fg(border_color),
            ),
        ])));
        map.push(msg_idx);
        texts.push(format!("{} - {}", req.tool_name, status));

        let args_str = serde_json::to_string(&req.arguments).unwrap_or_default();
        let preview = if args_str.len() > 60 {
            format!("{}...", &args_str[..57])
        } else {
            args_str.clone()
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled("│ ", Style::default().fg(border_color)),
            Span::styled(preview.clone(), Style::default().fg(Color::Gray)),
        ])));
        map.push(msg_idx);
        texts.push(preview);

        if is_pending {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("│ ", Style::default().fg(border_color)),
                Span::styled(
                    "Press Y to allow, N to deny",
                    Style::default().fg(theme.status.info),
                ),
            ])));
            map.push(msg_idx);
            texts.push("Press Y to allow, N to deny".to_string());
        }

        items.push(ListItem::new(Line::from(Span::styled(
            format!("└{:─<w$}┘", "", w = width.saturating_sub(2)),
            Style::default().fg(border_color),
        ))));
        map.push(msg_idx);
        texts.push(String::new());

        (items, map, texts)
    }

    fn check_confirmation_keys(&self, key: KeyCode, state: &AppState) -> Option<Action> {
        let pending = state.pending_confirmation.as_ref()?;
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(Action::ConfirmToolCall {
                id: pending.id.clone(),
                approved: true,
            }),
            KeyCode::Char('n') | KeyCode::Char('N') => Some(Action::ConfirmToolCall {
                id: pending.id.clone(),
                approved: false,
            }),
            _ => None,
        }
    }

    fn render_empty_state(&self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let start_color = if state.is_working {
            theme.base.border
        } else {
            match state.input_mode {
                InputMode::Editing => theme.base.border_active,
                InputMode::Normal | InputMode::Visual => theme.base.border,
            }
        };
        let end_color = theme.status.thinking;

        let start_breathing = breathing_color(start_color, self.frame_count, state.is_working);
        let end_breathing = breathing_color(end_color, self.frame_count, state.is_working);

        let logo_lines = render_logo_with_gradient(start_breathing, end_breathing);

        let hints = [
            "Tips for getting started:",
            "1. Ask questions, edit files, or run commands.",
            "2. Be specific for the best results.",
            "3. Type /help for more information.",
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if state.is_working {
                theme.status.thinking
            } else {
                theme.base.border
            }));

        f.render_widget(block.clone(), area);

        let inner_area = block.inner(area);

        let logo_start_y = inner_area.y + 1;
        let logo_start_x = inner_area.x + 2;

        let logo_area = Rect::new(
            logo_start_x,
            logo_start_y,
            inner_area.width.saturating_sub(2),
            logo_lines.len() as u16,
        );
        f.render_widget(Paragraph::new(logo_lines), logo_area);

        let hints_area = Rect::new(
            logo_start_x,
            logo_area.bottom() + 2,
            inner_area.width.saturating_sub(2),
            hints.len() as u16,
        );
        let hint_lines: Vec<Line> = hints
            .iter()
            .map(|h| Line::from(Span::styled(*h, Style::default().fg(Color::DarkGray))))
            .collect();
        f.render_widget(Paragraph::new(hint_lines), hints_area);
    }

    fn render_chat_list(
        &mut self,
        f: &mut Frame,
        area: Rect,
        state: &AppState,
        display_items: Vec<ListItem<'static>>,
    ) {
        let theme = &state.config.theme;
        let block = if state.copy_mode {
            Block::default()
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if state.is_working {
                    theme.status.thinking
                } else {
                    theme.base.border
                }))
        };

        let items_len = display_items.len();

        let list = List::new(display_items)
            .block(block.clone())
            .style(
                Style::default()
                    .bg(theme.base.background)
                    .fg(theme.base.foreground),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.base.selection)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.list_state);

        if state.input_mode == InputMode::Visual {
            if let Some(range) = self.selection_range() {
                let inner_area = block.inner(area);
                let scroll_offset = self.list_state.offset();
                let selected_idx = self.list_state.selected().unwrap_or(0);

                for display_idx in range {
                    if display_idx == selected_idx {
                        continue;
                    }
                    if display_idx < scroll_offset {
                        continue;
                    }

                    let visible_row = display_idx - scroll_offset;
                    if visible_row >= inner_area.height as usize {
                        continue;
                    }

                    let y = inner_area.y + visible_row as u16;
                    let buf = f.buffer_mut();
                    for x in inner_area.x..inner_area.x + inner_area.width {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            let mut style = cell.style();
                            style.bg = Some(theme.base.selection);
                            cell.set_style(style);
                        }
                    }
                }
            }
        }

        if !state.copy_mode && !self.stick_to_bottom && items_len > 0 {
            use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
            let mut scroll_state = ScrollbarState::default()
                .content_length(items_len)
                .position(self.list_state.selected().unwrap_or(0));

            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                area,
                &mut scroll_state,
            );
        }
    }
}

impl Component for ChatComponent {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if let Event::Tick = event {
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        match event {
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    self.stick_to_bottom = false;
                    let cur = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(cur.saturating_sub(3)));
                }
                MouseEventKind::ScrollDown => {
                    let cur = self.list_state.selected().unwrap_or(0);
                    let next = cur + 3;
                    self.list_state.select(Some(next));
                    if next >= self.last_item_count.saturating_sub(1) {
                        self.stick_to_bottom = true;
                    }
                }
                _ => {}
            },
            Event::Input(key) => {
                if state.input_mode == InputMode::Visual {
                    return self.handle_visual_mode(key.code, state);
                }

                if state.input_mode == InputMode::Normal {
                    if let Some(action) = self.check_confirmation_keys(key.code, state) {
                        return Ok(Some(action));
                    }

                    if key.code == KeyCode::Char('V') && !state.messages.is_empty() {
                        self.visual_anchor = Some(self.current_display_index());
                        return Ok(Some(Action::EnterVisualMode));
                    }

                    let page_size = self.last_height.saturating_sub(4) as usize;
                    match key.code {
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.stick_to_bottom = false;
                            let cur = self.list_state.selected().unwrap_or(0);
                            self.list_state.select(Some(cur.saturating_sub(1)));
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            let cur = self.list_state.selected().unwrap_or(0);
                            let next = cur + 1;
                            self.list_state.select(Some(next));
                            if next >= self.last_item_count.saturating_sub(1) {
                                self.stick_to_bottom = true;
                            }
                        }
                        KeyCode::PageUp => {
                            self.stick_to_bottom = false;
                            let cur = self.list_state.selected().unwrap_or(0);
                            self.list_state.select(Some(cur.saturating_sub(page_size)));
                        }
                        KeyCode::PageDown => {
                            let cur = self.list_state.selected().unwrap_or(0);
                            let max_idx = self.last_item_count.saturating_sub(1);
                            let next = (cur + page_size).min(max_idx);
                            self.list_state.select(Some(next));
                            if next >= max_idx {
                                self.stick_to_bottom = true;
                            }
                        }
                        KeyCode::Home => {
                            self.stick_to_bottom = false;
                            self.list_state.select(Some(0));
                        }
                        KeyCode::End => {
                            self.stick_to_bottom = true;
                            if self.last_item_count > 0 {
                                self.list_state
                                    .select(Some(self.last_item_count.saturating_sub(1)));
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = self.list_state.selected() {
                                if let Some(&msg_idx) = self.display_mapping.get(idx) {
                                    return Ok(Some(Action::OpenMessageInfo(msg_idx)));
                                }
                            }
                        }
                        KeyCode::Char('c') => {
                            if let Some(idx) = self.list_state.selected() {
                                if let Some(&msg_idx) = self.display_mapping.get(idx) {
                                    if let Some(message) = state.messages.get(msg_idx) {
                                        let text =
                                            crate::utils::message_format::message_to_plain_text(
                                                message,
                                            );
                                        return Ok(Some(Action::CopyToClipboard(text)));
                                    }
                                }
                            }
                        }
                        KeyCode::Esc => return Ok(Some(Action::ToggleInputMode)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        self.last_height = area.height;

        if self.last_input_mode == InputMode::Normal && state.input_mode == InputMode::Editing {
            self.stick_to_bottom = true;
        }
        self.last_input_mode = state.input_mode;

        if area.width != self.last_width {
            self.cached_items.clear();
            self.cached_mapping.clear();
            self.cached_line_text.clear();
            self.sealed_count = 0;
            self.last_tool_context.clear();
            self.last_width = area.width;
        }

        // If theme changed, clear cache to re-render with new colors
        if state.config.theme.name != self.last_theme_name {
            self.cached_items.clear();
            self.cached_mapping.clear();
            self.cached_line_text.clear();
            self.sealed_count = 0;
            self.last_tool_context.clear();
            self.last_theme_name = state.config.theme.name.clone();
        }

        // If state cleared (new session), clear cache and reset scroll
        if state.messages.len() < self.sealed_count {
            self.cached_items.clear();
            self.cached_mapping.clear();
            self.cached_line_text.clear();
            self.sealed_count = 0;
            self.last_tool_context.clear();
            self.list_state = ListState::default();
            self.stick_to_bottom = true;
        }

        // Determine how many messages are "sealed"
        let current_len = state.messages.len();
        let new_sealed_count = if state.is_working {
            current_len.saturating_sub(1)
        } else {
            current_len
        };

        if new_sealed_count > self.sealed_count {
            let theme = &state.config.theme;
            let width = area.width.saturating_sub(2) as usize;
            for i in self.sealed_count..new_sealed_count {
                if !state.messages[i].is_user_visible() {
                    continue;
                }
                let (items, map, texts) = Self::render_message(
                    i,
                    &state.messages[i],
                    width,
                    theme,
                    &mut self.last_tool_context,
                    state.pending_confirmation.as_ref(),
                );
                self.cached_items.extend(items);
                self.cached_mapping.extend(map);
                self.cached_line_text.extend(texts);
            }
            self.sealed_count = new_sealed_count;
        }

        let mut display_items = self.cached_items.clone();
        self.display_mapping = self.cached_mapping.clone();
        self.display_line_text = self.cached_line_text.clone();

        if state.is_working && !state.messages.is_empty() {
            let last_idx = state.messages.len() - 1;
            if state.messages[last_idx].is_user_visible() {
                let theme = &state.config.theme;
                let width = area.width.saturating_sub(2) as usize;
                let mut ctx = self.last_tool_context.clone();
                let (items, map, texts) = Self::render_message(
                    last_idx,
                    &state.messages[last_idx],
                    width,
                    theme,
                    &mut ctx,
                    state.pending_confirmation.as_ref(),
                );
                display_items.extend(items);
                self.display_mapping.extend(map);
                self.display_line_text.extend(texts);
            }
        }

        if self.stick_to_bottom && !display_items.is_empty() {
            self.list_state.select(Some(display_items.len() - 1));
        }

        self.last_item_count = display_items.len();

        // Render Logo if empty
        if display_items.is_empty() {
            self.render_empty_state(f, area, state);
        } else {
            self.render_chat_list(f, area, state, display_items);
        }
    }
}
