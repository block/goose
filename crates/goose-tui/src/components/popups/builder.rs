use super::{popup_block, render_hints, render_scrollbar};
use crate::components::Component;
use crate::services::config::CustomCommand;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::json::has_input_placeholder;
use crate::utils::layout::centered_rect;
use crate::utils::styles::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers, MouseEventKind};
use goose_client::ToolInfo;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, ScrollbarState,
};
use ratatui::Frame;
use std::collections::HashMap;
use tui_textarea::TextArea;

#[derive(Clone, Copy, PartialEq, Default)]
enum View {
    #[default]
    ToolSelect,
    AliasManage,
    Editor,
}

pub struct BuilderPopup<'a> {
    view: View,
    list_state: ListState,
    scroll_state: ScrollbarState,
    search: String,
    selected_tool_idx: Option<usize>,
    editing_alias: Option<String>,
    alias_name: TextArea<'a>,
    param_inputs: Vec<(String, TextArea<'a>)>,
    focused_field: usize,
}

impl Default for BuilderPopup<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> BuilderPopup<'a> {
    pub fn new() -> Self {
        Self {
            view: View::default(),
            list_state: ListState::default().with_selected(Some(0)),
            scroll_state: ScrollbarState::default(),
            search: String::new(),
            selected_tool_idx: None,
            editing_alias: None,
            alias_name: new_text_input("Alias name"),
            param_inputs: Vec::new(),
            focused_field: 0,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn tool_indices(&self, state: &AppState) -> Vec<Option<usize>> {
        build_tool_list(&state.available_tools, &self.search, &state.config.theme).1
    }

    fn list_count(&self, state: &AppState) -> usize {
        match self.view {
            View::ToolSelect => self.tool_indices(state).len(),
            View::AliasManage => state.config.custom_commands.len(),
            View::Editor => 0,
        }
    }

    fn navigate(&mut self, delta: i32, state: &AppState) {
        let count = self.list_count(state);
        if count == 0 {
            return;
        }

        let indices = if self.view == View::ToolSelect {
            Some(self.tool_indices(state))
        } else {
            None
        };

        let mut i = self.list_state.selected().unwrap_or(0) as i32;
        loop {
            i = (i + delta).rem_euclid(count as i32);
            if let Some(ref idx) = indices {
                if self.search.is_empty()
                    && i != 0
                    && !idx.get(i as usize).is_some_and(|x| x.is_some())
                {
                    continue;
                }
            }
            break;
        }
        self.list_state.select(Some(i as usize));
        self.scroll_state = self.scroll_state.position(i as usize);
    }

    fn setup_editor(&mut self, tool: &ToolInfo, existing: Option<&CustomCommand>) {
        self.param_inputs = tool
            .parameters
            .iter()
            .map(|param| {
                let mut ta = new_text_input(param);
                if let Some(cmd) = existing {
                    if let Some(val) = cmd.args.get(param).and_then(|v| v.as_str()) {
                        ta.insert_str(val);
                    }
                }
                (param.clone(), ta)
            })
            .collect();

        self.alias_name = new_text_input("Alias name (e.g., gs)");
        if let Some(cmd) = existing {
            self.alias_name.insert_str(&cmd.name);
            self.editing_alias = Some(cmd.name.clone());
        }

        self.focused_field = 0;
        self.view = View::Editor;
    }

    fn build_command(&self, tools: &[ToolInfo]) -> Option<CustomCommand> {
        let name = self.alias_name.lines().join("").trim().replace('/', "");
        if name.is_empty() {
            return None;
        }

        let tool = tools.get(self.selected_tool_idx?)?;
        let args: serde_json::Map<String, serde_json::Value> = self
            .param_inputs
            .iter()
            .map(|(k, ta)| (k.clone(), serde_json::Value::String(ta.lines().join("\n"))))
            .collect();

        Some(CustomCommand {
            name,
            description: format!("Alias for {}", tool.name),
            tool: tool.name.clone(),
            args: serde_json::Value::Object(args),
        })
    }

    fn preview_text(&self, tools: &[ToolInfo]) -> String {
        let name = self.alias_name.lines().join("").trim().replace('/', "");
        let name_display = if name.is_empty() { "..." } else { &name };

        let Some(tool) = self.selected_tool_idx.and_then(|i| tools.get(i)) else {
            return format!("/{name_display}");
        };

        let short = tool.name.split("__").last().unwrap_or(&tool.name);
        let params: String = self
            .param_inputs
            .iter()
            .map(|(k, ta)| {
                format!(
                    "{}={}",
                    k,
                    ta.lines().join("").chars().take(20).collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        if params.is_empty() {
            format!("/{name_display} → {short}")
        } else {
            format!("/{name_display} → {short}({params})")
        }
    }

    fn focus_next(&mut self) {
        let total = self.param_inputs.len() + 1;
        self.focused_field = (self.focused_field + 1) % total;
    }

    fn focus_prev(&mut self) {
        let total = self.param_inputs.len() + 1;
        self.focused_field = (self.focused_field + total - 1) % total;
    }
}

// Event handling
impl<'a> Component for BuilderPopup<'a> {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if !state.showing_command_builder {
            self.reset();
            return Ok(None);
        }

        match event {
            Event::Input(key) => match self.view {
                View::ToolSelect => self.handle_tool_select(key, state),
                View::AliasManage => self.handle_alias_manage(key, state),
                View::Editor => self.handle_editor(key, state),
            },
            Event::Mouse(m) => {
                match m.kind {
                    MouseEventKind::ScrollDown => self.navigate(1, state),
                    MouseEventKind::ScrollUp => self.navigate(-1, state),
                    _ => {}
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_command_builder {
            return;
        }

        let area = centered_rect(70, 70, area);
        f.render_widget(Clear, area);

        match self.view {
            View::ToolSelect => self.render_tool_select(f, area, state),
            View::AliasManage => self.render_alias_manage(f, area, state),
            View::Editor => self.render_editor(f, area, state),
        }
    }
}

// View-specific event handlers
impl BuilderPopup<'_> {
    fn handle_tool_select(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                if self.search.is_empty() {
                    self.reset();
                    return Ok(Some(Action::ClosePopup));
                }
                self.search.clear();
                self.list_state.select(Some(0));
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.push(c);
                self.list_state.select(Some(0));
            }
            KeyCode::Backspace => {
                self.search.pop();
                self.list_state.select(Some(0));
            }
            KeyCode::Down | KeyCode::Tab => self.navigate(1, state),
            KeyCode::Up | KeyCode::BackTab => self.navigate(-1, state),
            KeyCode::Enter => {
                let indices = self.tool_indices(state);
                if let Some(sel) = self.list_state.selected() {
                    if sel == 0 && self.search.is_empty() {
                        self.view = View::AliasManage;
                        self.list_state.select(Some(0));
                        self.scroll_state = ScrollbarState::default();
                    } else if let Some(&Some(tool_idx)) = indices.get(sel) {
                        self.selected_tool_idx = Some(tool_idx);
                        if let Some(tool) = state.available_tools.get(tool_idx) {
                            self.setup_editor(tool, None);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_alias_manage(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.view = View::ToolSelect;
                self.list_state.select(Some(0));
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.navigate(1, state),
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => self.navigate(-1, state),
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(selected) = self.list_state.selected() {
                    if let Some(cmd) = state.config.custom_commands.get(selected) {
                        let name = cmd.name.clone();
                        let new_len = state.config.custom_commands.len().saturating_sub(1);
                        if new_len > 0 && selected >= new_len {
                            self.list_state.select(Some(new_len - 1));
                        }
                        return Ok(Some(Action::DeleteCustomCommand(name)));
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(cmd) = self
                    .list_state
                    .selected()
                    .and_then(|i| state.config.custom_commands.get(i))
                {
                    if let Some((idx, tool)) = state
                        .available_tools
                        .iter()
                        .enumerate()
                        .find(|(_, t)| t.name == cmd.tool)
                    {
                        self.selected_tool_idx = Some(idx);
                        self.setup_editor(tool, Some(cmd));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_editor(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.view = View::ToolSelect;
                self.search.clear();
                self.list_state.select(Some(0));
            }
            KeyCode::Tab | KeyCode::Down => self.focus_next(),
            KeyCode::BackTab | KeyCode::Up => self.focus_prev(),
            KeyCode::Enter => {
                if let Some(cmd) = self.build_command(&state.available_tools) {
                    let msg = if self.editing_alias.is_some() {
                        format!("✓ Updated /{}", cmd.name)
                    } else {
                        format!("✓ Created /{}", cmd.name)
                    };
                    self.reset();
                    return Ok(Some(Action::SubmitCommandBuilder(cmd, msg)));
                }
            }
            _ => {
                if self.focused_field == 0 {
                    self.alias_name.input(*key);
                } else if let Some((_, ta)) = self.param_inputs.get_mut(self.focused_field - 1) {
                    ta.input(*key);
                }
            }
        }
        Ok(None)
    }

    fn has_input_placeholder(&self) -> bool {
        self.param_inputs
            .iter()
            .any(|(_, ta)| ta.lines().join("").contains("{input}"))
    }
}

// View-specific renderers
impl BuilderPopup<'_> {
    fn render_tool_select(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let (items, _) = build_tool_list(&state.available_tools, &self.search, theme);

        let [search_area, list_area, hints_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .areas(area);

        f.render_widget(popup_block(" Create Alias ", theme), area);

        let search_text = if self.search.is_empty() {
            "Type to search tools...".to_string()
        } else {
            format!("Search: {}_", self.search)
        };
        let search_style = if self.search.is_empty() {
            Style::default().fg(theme.base.border)
        } else {
            Style::default().fg(theme.status.warning)
        };
        f.render_widget(
            Paragraph::new(search_text).style(search_style).block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.base.border)),
            ),
            search_area,
        );

        self.scroll_state = self.scroll_state.content_length(items.len());
        f.render_stateful_widget(
            List::new(items)
                .highlight_style(Style::default().bg(theme.base.selection))
                .highlight_symbol("▶ "),
            list_area,
            &mut self.list_state,
        );
        render_scrollbar(f, list_area, &mut self.scroll_state);
        render_hints(
            f,
            hints_area,
            theme,
            &[("↑↓", "nav"), ("Enter", "select"), ("Esc", "close")],
        );
    }

    fn render_alias_manage(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let commands = &state.config.custom_commands;

        let [list_area, hints_area] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
            .margin(1)
            .areas(area);

        f.render_widget(popup_block(" Manage Aliases ", theme), area);

        if commands.is_empty() {
            f.render_widget(
                Paragraph::new("No aliases defined yet.")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.base.border)),
                list_area,
            );
        } else {
            let items: Vec<ListItem> = commands.iter().map(|c| alias_list_item(c, theme)).collect();
            self.scroll_state = self.scroll_state.content_length(items.len());
            f.render_stateful_widget(
                List::new(items)
                    .highlight_style(Style::default().bg(theme.base.selection))
                    .highlight_symbol("▶ "),
                list_area,
                &mut self.list_state,
            );
            render_scrollbar(f, list_area, &mut self.scroll_state);
        }

        render_hints(
            f,
            hints_area,
            theme,
            &[("e/Enter", "edit"), ("d", "delete"), ("Esc", "back")],
        );
    }

    fn render_editor(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let title = if self.editing_alias.is_some() {
            " Edit Alias "
        } else {
            " New Alias "
        };

        let has_input = self.has_input_placeholder();

        // Calculate constraints: alias name + params + hint (if no {input}) + preview + spacer + hints
        let mut constraints: Vec<Constraint> = vec![Constraint::Length(3)];
        constraints.extend(std::iter::repeat_n(
            Constraint::Length(3),
            self.param_inputs.len(),
        ));
        if !has_input && !self.param_inputs.is_empty() {
            constraints.push(Constraint::Length(1)); // hint about {input}
        }
        constraints.extend([
            Constraint::Length(2), // preview
            Constraint::Min(0),    // spacer
            Constraint::Length(1), // hints
        ]);

        let chunks = Layout::vertical(constraints).margin(1).split(area);

        f.render_widget(popup_block(title, theme), area);

        self.alias_name
            .set_block(input_block("Alias name", self.focused_field == 0, theme));
        f.render_widget(&self.alias_name, chunks[0]);

        for (i, (param, ta)) in self.param_inputs.iter_mut().enumerate() {
            ta.set_block(input_block(param, self.focused_field == i + 1, theme));
            f.render_widget(&*ta, chunks[i + 1]);
        }

        let mut next_chunk = self.param_inputs.len() + 1;

        // Show hint about {input} if not already used
        if !has_input && !self.param_inputs.is_empty() {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Tip: ", Style::default().fg(theme.base.border)),
                    Span::styled(
                        "Use {input} in a parameter to accept trailing arguments",
                        Style::default().fg(theme.base.border),
                    ),
                ])),
                chunks[next_chunk],
            );
            next_chunk += 1;
        }

        // Preview with {input} highlighting
        let preview = self.preview_text(&state.available_tools);
        let preview_spans = build_preview_spans(&preview, has_input, theme);
        f.render_widget(
            Paragraph::new(Line::from(preview_spans)),
            chunks[next_chunk],
        );

        render_hints(
            f,
            chunks[next_chunk + 2],
            theme,
            &[("↑↓/Tab", "nav"), ("Enter", "save"), ("Esc", "cancel")],
        );
    }
}

fn input_block(title: &str, focused: bool, theme: &Theme) -> Block<'static> {
    let border_color = if focused {
        theme.base.border_active
    } else {
        theme.base.border
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title.to_string())
}

fn new_text_input(placeholder: &str) -> TextArea<'static> {
    let mut ta = TextArea::default();
    ta.set_cursor_line_style(Style::default());
    ta.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(placeholder.to_string()),
    );
    ta
}

// Tool list building
fn build_tool_list(
    tools: &[ToolInfo],
    search: &str,
    theme: &Theme,
) -> (Vec<ListItem<'static>>, Vec<Option<usize>>) {
    let mut items = Vec::new();
    let mut indices = Vec::new();

    if search.is_empty() {
        items.push(ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Manage aliases...",
                Style::default()
                    .fg(theme.status.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ])));
        indices.push(None);

        for group in group_tools(tools) {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("─ {} ─", group.0),
                Style::default()
                    .fg(theme.base.border)
                    .add_modifier(Modifier::BOLD),
            ))));
            indices.push(None);

            for idx in group.1 {
                items.push(tool_list_item(&tools[idx], theme));
                indices.push(Some(idx));
            }
        }
    } else {
        let query = search.to_lowercase();
        for (idx, tool) in tools.iter().enumerate() {
            if tool.name.to_lowercase().contains(&query)
                || tool.description.to_lowercase().contains(&query)
            {
                items.push(tool_list_item_with_prefix(tool, theme));
                indices.push(Some(idx));
            }
        }
    }

    (items, indices)
}

fn group_tools(tools: &[ToolInfo]) -> Vec<(String, Vec<usize>)> {
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, tool) in tools.iter().enumerate() {
        let prefix = tool
            .name
            .split("__")
            .next()
            .unwrap_or(&tool.name)
            .to_string();
        groups.entry(prefix).or_default().push(i);
    }
    let mut result: Vec<_> = groups.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

fn tool_list_item(tool: &ToolInfo, theme: &Theme) -> ListItem<'static> {
    let short_name = tool.name.split("__").last().unwrap_or(&tool.name);
    ListItem::new(vec![
        Line::from(Span::styled(
            format!("  {short_name}"),
            Style::default().fg(theme.status.info),
        )),
        Line::from(Span::styled(
            format!("    {}", truncate(&tool.description, 50)),
            Style::default().fg(theme.base.border),
        )),
    ])
}

fn tool_list_item_with_prefix(tool: &ToolInfo, theme: &Theme) -> ListItem<'static> {
    let prefix = tool.name.split("__").next().unwrap_or("");
    let short_name = tool.name.split("__").last().unwrap_or(&tool.name);
    ListItem::new(vec![
        Line::from(vec![
            Span::styled(format!("{prefix}/"), Style::default().fg(theme.base.border)),
            Span::styled(
                short_name.to_string(),
                Style::default().fg(theme.status.info),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {}", truncate(&tool.description, 50)),
            Style::default().fg(theme.base.border),
        )),
    ])
}

fn alias_list_item(cmd: &CustomCommand, theme: &Theme) -> ListItem<'static> {
    let short_tool = cmd.tool.split("__").last().unwrap_or(&cmd.tool);
    let args_preview = preview_args(&cmd.args);

    let mut first_line = vec![
        Span::styled(
            format!("/{}", cmd.name),
            Style::default()
                .fg(theme.status.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" → {short_tool}"),
            Style::default().fg(theme.status.info),
        ),
    ];

    if has_input_placeholder(&cmd.args) {
        first_line.push(Span::styled(
            " <args>".to_string(),
            Style::default().fg(theme.status.warning),
        ));
    }

    ListItem::new(vec![
        Line::from(first_line),
        Line::from(Span::styled(
            format!("  {args_preview}"),
            Style::default().fg(theme.base.border),
        )),
    ])
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

fn preview_args(args: &serde_json::Value) -> String {
    args.as_object()
        .map(|obj| {
            let parts: Vec<String> = obj
                .iter()
                .take(2)
                .map(|(k, v)| {
                    let val: String = v.as_str().unwrap_or("").chars().take(20).collect();
                    format!("{k}={val}")
                })
                .collect();
            if parts.is_empty() {
                "(no args)".to_string()
            } else {
                parts.join(", ")
            }
        })
        .unwrap_or_default()
}

fn build_preview_spans<'a>(preview: &str, has_input: bool, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = vec![Span::styled(
        "Preview: ".to_string(),
        Style::default().fg(theme.base.border),
    )];

    if has_input {
        // Split on {input} and highlight it
        let parts: Vec<&str> = preview.split("{input}").collect();
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                spans.push(Span::styled(
                    (*part).to_string(),
                    Style::default().fg(theme.status.success),
                ));
            }
            if i < parts.len() - 1 {
                spans.push(Span::styled(
                    "{input}".to_string(),
                    Style::default().fg(theme.status.warning),
                ));
            }
        }
        spans.push(Span::styled(
            " (accepts args)".to_string(),
            Style::default().fg(theme.status.warning),
        ));
    } else {
        spans.push(Span::styled(
            preview.to_string(),
            Style::default().fg(theme.status.success),
        ));
    }

    spans
}
