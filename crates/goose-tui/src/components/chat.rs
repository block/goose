use super::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::state::{AppState, InputMode};
use crate::utils::ascii_art::GOOSE_LOGO;
use crate::utils::markdown::MarkdownRenderer;
use crate::utils::styles::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use goose::conversation::message::MessageContent;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub struct ChatComponent {
    list_state: ListState,
    cached_items: Vec<ListItem<'static>>,
    cached_mapping: Vec<usize>,
    sealed_count: usize,
    last_tool_context: Option<(String, String)>, // Name, Args
    stick_to_bottom: bool,
    frame_count: usize,
    last_input_mode: InputMode,
}

impl ChatComponent {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            cached_items: Vec::new(),
            cached_mapping: Vec::new(),
            sealed_count: 0,
            last_tool_context: None,
            stick_to_bottom: true,
            frame_count: 0,
            last_input_mode: InputMode::Normal,
        }
    }

    fn to_rgb(color: Color) -> (u8, u8, u8) {
        match color {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (128, 128, 128),
        }
    }

    fn render_message(
        msg_idx: usize,
        message: &goose::conversation::message::Message,
        width: usize,
        theme: &Theme,
        last_tool_context: &mut Option<(String, String)>,
    ) -> (Vec<ListItem<'static>>, Vec<usize>) {
        let mut items = Vec::new();
        let mut map = Vec::new();

        match message.role {
            rmcp::model::Role::User => {
                for content in &message.content {
                    match content {
                        MessageContent::Text(t) => {
                            let renderer =
                                MarkdownRenderer::new(&t.text, width.saturating_sub(4), theme);
                            for line in renderer.render_lines() {
                                let mut spans =
                                    vec![Span::styled("│ ", Style::default().fg(Color::DarkGray))];
                                spans.extend(line.spans);
                                items.push(ListItem::new(Line::from(spans)));
                                map.push(msg_idx);
                            }
                        }
                        MessageContent::ToolResponse(resp) => {
                            let (tool_name, tool_args) = last_tool_context
                                .clone()
                                .unwrap_or(("Unknown".to_string(), "".to_string()));
                            let is_success = resp.tool_result.is_ok();
                            let color = if is_success {
                                theme.status.success
                            } else {
                                theme.status.error
                            };

                            let max_args = 50;
                            let display_args = if tool_args.chars().count() > max_args {
                                format!("{}...", tool_args.chars().take(max_args).collect::<String>())
                            } else {
                                tool_args
                            };

                            let header = format!("{} {}", tool_name, display_args);
                            let fixed = 5;
                            let padding = width.saturating_sub(header.len() + fixed + 2); // +2 for borders roughly

                            let header_line = Line::from(vec![
                                Span::styled("╭─ ", Style::default().fg(Color::DarkGray)),
                                Span::styled(
                                    tool_name,
                                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    format!(" {} ", display_args),
                                    Style::default().fg(Color::Gray),
                                ),
                                Span::styled(
                                    format!("{:─<width$}╮", "", width = padding),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]);
                            items.push(ListItem::new(header_line));
                            map.push(msg_idx);

                            if let Ok(contents) = &resp.tool_result {
                                let mut line_count = 0;
                                let max_lines = 10;
                                for content in contents {
                                    if let rmcp::model::Content {
                                        raw: rmcp::model::RawContent::Text(text_content),
                                        ..
                                    } = content
                                    {
                                        for line in text_content.text.lines() {
                                            if line_count >= max_lines {
                                                break;
                                            }
                                            let inner_width = width.saturating_sub(4);
                                            let truncated = if line.len() > inner_width {
                                                &line[..inner_width]
                                            } else {
                                                line
                                            };
                                            let pad = inner_width
                                                .saturating_sub(truncated.chars().count());
                                            let box_line = format!(
                                                "│ {}{: <width$}│",
                                                truncated,
                                                "",
                                                width = pad
                                            );
                                            items.push(ListItem::new(Line::from(Span::styled(
                                                box_line,
                                                Style::default().fg(Color::Gray),
                                            ))));
                                            map.push(msg_idx);
                                            line_count += 1;
                                        }
                                    }
                                }
                                if line_count >= max_lines {
                                    let content = "... (truncated)";
                                    let pad = width.saturating_sub(content.len() + 4);
                                    let box_line =
                                        format!("│ {}{: <width$}│", content, "", width = pad);
                                    items.push(ListItem::new(Line::from(Span::styled(
                                        box_line,
                                        Style::default().fg(Color::DarkGray),
                                    ))));
                                    map.push(msg_idx);
                                }
                            }
                            let footer =
                                format!("╰{:─<width$}╯", "", width = width.saturating_sub(2));
                            items.push(ListItem::new(Line::from(Span::styled(
                                footer,
                                Style::default().fg(Color::DarkGray),
                            ))));
                            map.push(msg_idx);
                        }
                        _ => {}
                    }
                }
            }
            rmcp::model::Role::Assistant => {
                for content in &message.content {
                    match content {
                        MessageContent::Text(t) => {
                            let renderer = MarkdownRenderer::new(&t.text, width, theme);
                            for line in renderer.render_lines() {
                                items.push(ListItem::new(line));
                                map.push(msg_idx);
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
                                *last_tool_context = Some((name.to_string(), args));

                                let line = Line::from(vec![
                                    Span::styled(
                                        "▶ Tool: ",
                                        Style::default().fg(theme.status.warning),
                                    ),
                                    Span::styled(
                                        format!("{} (args hidden)", name),
                                        Style::default().fg(theme.base.foreground),
                                    ),
                                ]);
                                items.push(ListItem::new(line));
                                map.push(msg_idx);
                            }
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
                        }
                        _ => {}
                    }
                }
            }
        }

        items.push(ListItem::new(Line::from("")));
        map.push(msg_idx);

        (items, map)
    }
}

impl Component for ChatComponent {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if let Event::Tick = event {
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        match event {
            Event::Mouse(m) => {
                match m.kind {
                    MouseEventKind::ScrollUp => {
                        self.stick_to_bottom = false;
                        let cur = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some(cur.saturating_sub(3)));
                    }
                    MouseEventKind::ScrollDown => {
                        let cur = self.list_state.selected().unwrap_or(0);
                        self.list_state.select(Some(cur + 3));
                        // Check if at bottom? Hard to know without total length.
                        // We will check in render.
                    }
                    MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                        // On click, select the item at the clicked row?
                        // Ratatui List doesn't auto-select on click easily without calculating rows.
                        // But if we just want to open the *currently selected* item, we can assume
                        // the user selected it (maybe via scroll/hover if we implemented that, but we haven't).
                        
                        // Actually, standard TUI behavior for click is:
                        // 1. Calculate index based on y-coord.
                        // 2. Select it.
                        // 3. Trigger action.
                        
                        // Calculating index is hard here because we don't know the exact layout area offset in handle_event.
                        // However, if we just treat Click as "Open Selected" (if selection follows mouse?), 
                        // or just "Open Selected" (Enter equivalent).
                        
                        // If the user clicks, they expect the thing under the cursor to open.
                        // Implementing full mouse picking requires `area` which we don't have in `handle_event`.
                        // But we can just map Click -> OpenMessageInfo of *selected* item if we assume selection is updated.
                        // But selection isn't updated by mouse hover in this code.
                        
                        // Compromise: Treat Left Click as "Open Details of Selected Item".
                        // The user can scroll to select (which updates selection) then click?
                        // No, scrolling updates selection.
                        
                        if let Some(idx) = self.list_state.selected() {
                             if let Some(&msg_idx) = self.cached_mapping.get(idx) {
                                 return Ok(Some(Action::OpenMessageInfo(msg_idx)));
                             }
                        }
                    }
                    _ => {}
                }
            }
            Event::Input(key) => {
                if state.input_mode == crate::state::state::InputMode::Normal {
                    match key.code {
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.stick_to_bottom = false;
                            let cur = self.list_state.selected().unwrap_or(0);
                            self.list_state.select(Some(cur.saturating_sub(1)));
                        }
                                                 KeyCode::Char('j') | KeyCode::Down => {
                                                     let cur = self.list_state.selected().unwrap_or(0);
                                                     self.list_state.select(Some(cur + 1));
                                                 }
                                                                          KeyCode::Enter => {
                                                                              if let Some(idx) = self.list_state.selected() {
                                                                                  if let Some(&msg_idx) = self.cached_mapping.get(idx) {
                                                                                      return Ok(Some(Action::OpenMessageInfo(msg_idx)));
                                                                                  }
                                                                              }
                                                                          }
                                                                          KeyCode::Esc => return Ok(Some(Action::ToggleInputMode)),
                                                                          _ => {}                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        // Check for mode transition: Normal -> Editing implies stick to bottom
        if self.last_input_mode == InputMode::Normal && state.input_mode == InputMode::Editing {
            self.stick_to_bottom = true;
        }
        self.last_input_mode = state.input_mode;

        let theme = &state.config.theme;
        let width = area.width.saturating_sub(2) as usize;

        // 1. Reconcile Cache
        // If state cleared (new session), clear cache
        if state.messages.len() < self.sealed_count {
            self.cached_items.clear();
            self.cached_mapping.clear();
            self.sealed_count = 0;
            self.last_tool_context = None;
        }

        // Determine how many messages are "sealed"
        let current_len = state.messages.len();
        let new_sealed_count = if state.is_working {
            current_len.saturating_sub(1)
        } else {
            current_len
        };

        // Update Cache
        if new_sealed_count > self.sealed_count {
            for i in self.sealed_count..new_sealed_count {
                let (items, map) = Self::render_message(
                    i,
                    &state.messages[i],
                    width,
                    theme,
                    &mut self.last_tool_context,
                );
                self.cached_items.extend(items);
                self.cached_mapping.extend(map);
            }
            self.sealed_count = new_sealed_count;
        }

        // Prepare Display List (Cache + Dynamic)
        let mut display_items = self.cached_items.clone();
        let mut display_map = self.cached_mapping.clone();

        // Render Dynamic (Last message if working)
        if state.is_working && !state.messages.is_empty() {
            let last_idx = state.messages.len() - 1;
            // Context clone for dynamic render
            let mut ctx = self.last_tool_context.clone();
            let (items, map) =
                Self::render_message(last_idx, &state.messages[last_idx], width, theme, &mut ctx);
            display_items.extend(items);
            display_map.extend(map);
        }

        // Blinking Cursor
        if !state.is_working && !state.messages.is_empty() {
            // Render a prompt line?
        }

        // Auto-scroll logic
        if self.stick_to_bottom {
            if !display_items.is_empty() {
                self.list_state.select(Some(display_items.len() - 1));
            }
        }

        // Render Logo if empty
        if display_items.is_empty() {
            let base_color = if state.is_working {
                theme.status.thinking
            } else {
                match state.input_mode {
                    InputMode::Editing => theme.base.border_active,
                    InputMode::Normal => theme.base.border,
                }
            };
            let (r, g, b) = Self::to_rgb(base_color);
            
            let (dr, dg, db) = if state.is_working {
                let t = self.frame_count as f32 * 0.1;
                let factor = 0.85 + 0.15 * t.sin();
                (
                    (r as f32 * factor) as u8,
                    (g as f32 * factor) as u8,
                    (b as f32 * factor) as u8,
                )
            } else {
                (r, g, b)
            };
            let logo_color = Color::Rgb(dr, dg, db);

            let logo_lines: Vec<Line> = GOOSE_LOGO
                .lines()
                .map(|l| Line::from(Span::styled(l, Style::default().fg(logo_color))))
                .collect();
            let hints = vec![
                "Tips for getting started:",
                "1. Ask questions, edit files, or run commands.",
                "2. Be specific for the best results.",
                "3. Type /help for more information.",
            ];

            // Render to a separate area or as list items?
            // If I render as list items, I can just add them to display_items.
            // But they shouldn't be selectable or part of history.
            // Better to render Paragraph inside the block.
            // But I am rendering List.

            // I'll render the List as empty block, and render Paragraph on top?
            // Yes.

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

            // Render at top-left, with 2 units of padding from left and top
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

            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if state.is_working {
                theme.status.thinking
            } else {
                theme.base.border
            }));

        let list = List::new(display_items)
            .block(block)
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
    }
}
