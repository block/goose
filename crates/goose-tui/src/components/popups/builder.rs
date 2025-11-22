use crate::components::Component;
use crate::services::config::CustomCommand;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use std::collections::HashMap;
use tui_textarea::TextArea;

enum Step {
    Menu,                // Shows "Remove existing alias" and Tools list combined
    DeleteSelect,        // Secondary page to select alias to remove
    ConfigureArg(usize), // Index of argument in tool.parameters
    NameCommand,
}

pub struct BuilderPopup<'a> {
    step: Step,
    list_state: ListState,
    input: TextArea<'a>,
    selected_tool_idx: Option<usize>,
    arg_values: HashMap<String, String>,
}

impl<'a> Default for BuilderPopup<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> BuilderPopup<'a> {
    pub fn new() -> Self {
        let mut input = TextArea::default();
        input.set_block(Block::default().borders(Borders::ALL));

        Self {
            step: Step::Menu,
            list_state: ListState::default(),
            input,
            selected_tool_idx: None,
            arg_values: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.step = Step::Menu;
        self.list_state.select(Some(0));
        self.selected_tool_idx = None;
        self.arg_values.clear();
        self.input = TextArea::default();
        self.input.set_block(Block::default().borders(Borders::ALL));
    }
}

impl<'a> Component for BuilderPopup<'a> {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if !state.showing_command_builder {
            self.reset();
            return Ok(None);
        }

        if let Event::Input(key) = event {
            match self.step {
                Step::Menu => self.handle_menu_event(key, state),
                Step::DeleteSelect => self.handle_delete_select_event(key, state),
                Step::ConfigureArg(arg_idx) => self.handle_configure_arg_event(key, state, arg_idx),
                Step::NameCommand => self.handle_name_command_event(key, state),
            }
        } else {
            Ok(None)
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_command_builder {
            return;
        }

        let area = centered_rect(70, 60, area);
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(state.config.theme.base.background));

        match self.step {
            Step::Menu => self.render_menu(f, area, state, block),
            Step::DeleteSelect => self.render_delete_select(f, area, state, block),
            Step::ConfigureArg(arg_idx) => {
                self.render_configure_arg(f, area, state, block, arg_idx)
            }
            Step::NameCommand => self.render_name_command(f, area, state, block),
        }
    }
}

impl<'a> BuilderPopup<'a> {
    fn handle_menu_event(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.reset();
                Ok(Some(Action::ClosePopup))
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let idx = self.list_state.selected().unwrap_or(0);
                let max = state.available_tools.len();
                let total_items = max + 1;
                self.list_state
                    .select(Some((idx + 1).min(total_items.saturating_sub(1))));
                Ok(None)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let idx = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some(idx.saturating_sub(1)));
                Ok(None)
            }
            KeyCode::Enter => {
                let idx = self.list_state.selected().unwrap_or(0);
                if idx == 0 {
                    self.step = Step::DeleteSelect;
                    self.list_state.select(Some(0));
                    self.selected_tool_idx = None;
                    self.input = TextArea::default();
                    self.input.set_block(Block::default().borders(Borders::ALL));
                } else {
                    let tool_idx = idx - 1;
                    if state.available_tools.get(tool_idx).is_some() {
                        self.selected_tool_idx = Some(tool_idx);
                        let tool = &state.available_tools[tool_idx];
                        if tool.parameters.is_empty() {
                            self.step = Step::NameCommand;
                        } else {
                            self.step = Step::ConfigureArg(0);
                        }
                        self.input = TextArea::default();
                        self.input.set_block(Block::default().borders(Borders::ALL));
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn handle_delete_select_event(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.step = Step::Menu;
                self.list_state.select(Some(0));
                self.selected_tool_idx = None;
                self.input = TextArea::default();
                self.input.set_block(Block::default().borders(Borders::ALL));
                Ok(None)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let idx = self.list_state.selected().unwrap_or(0);
                let max = state.config.custom_commands.len().saturating_sub(1);
                self.list_state.select(Some((idx + 1).min(max)));
                Ok(None)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let idx = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some(idx.saturating_sub(1)));
                Ok(None)
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(cmd) = state.config.custom_commands.get(idx) {
                        self.reset();
                        return Ok(Some(Action::DeleteCustomCommand(cmd.name.clone())));
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn handle_configure_arg_event(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
        arg_idx: usize,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.step = Step::Menu;
                Ok(None)
            }
            KeyCode::Enter => {
                let tool = &state.available_tools[self.selected_tool_idx.unwrap()];
                let arg_name = &tool.parameters[arg_idx];
                let value = self.input.lines().join("\n");

                self.arg_values.insert(arg_name.clone(), value);

                if arg_idx + 1 < tool.parameters.len() {
                    self.step = Step::ConfigureArg(arg_idx + 1);
                    self.input = TextArea::default();
                    self.input.set_block(Block::default().borders(Borders::ALL));
                } else {
                    self.step = Step::NameCommand;
                    self.input = TextArea::default();
                    self.input.set_block(Block::default().borders(Borders::ALL));
                }
                Ok(None)
            }
            _ => {
                self.input.input(*key);
                Ok(None)
            }
        }
    }

    fn handle_name_command_event(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.step = Step::Menu;
                Ok(None)
            }
            KeyCode::Enter => {
                let name = self.input.lines().join("\n").trim().to_string();
                if !name.is_empty() {
                    let tool = &state.available_tools[self.selected_tool_idx.unwrap()];

                    let mut args_map = serde_json::Map::new();
                    for (k, v) in &self.arg_values {
                        args_map.insert(k.clone(), serde_json::Value::String(v.clone()));
                    }

                    let cmd = CustomCommand {
                        name: name.replace("/", ""),
                        description: format!("Alias for {}", tool.name),
                        tool: tool.name.clone(),
                        args: serde_json::Value::Object(args_map),
                    };

                    self.reset();
                    return Ok(Some(Action::SubmitCommandBuilder(cmd)));
                }
                Ok(None)
            }
            _ => {
                self.input.input(*key);
                Ok(None)
            }
        }
    }

    fn render_menu(&mut self, f: &mut Frame, area: Rect, state: &AppState, block: Block<'static>) {
        let mut items = vec![ListItem::new(Span::styled(
            "Remove existing alias...",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))];

        for tool in &state.available_tools {
            items.push(ListItem::new(vec![
                Line::from(Span::styled(
                    &tool.name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    &tool.description,
                    Style::default().fg(Color::Gray),
                )),
            ]));
        }

        let list = List::new(items)
            .block(block.title("Command Builder: Select Tool (Enter to Select)"))
            .highlight_style(
                Style::default()
                    .bg(state.config.theme.base.selection)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_delete_select(
        &mut self,
        f: &mut Frame,
        area: Rect,
        state: &AppState,
        block: Block<'static>,
    ) {
        let items: Vec<ListItem> = state
            .config
            .custom_commands
            .iter()
            .map(|c| ListItem::new(format!("/{} -> {}", c.name, c.tool)))
            .collect();

        if items.is_empty() {
            let p = Paragraph::new("No aliases defined.")
                .block(block.title("Remove Alias"))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(p, area);
        } else {
            let list = List::new(items)
                .block(block.title("Select Alias to Remove (Enter to Delete)"))
                .highlight_style(
                    Style::default()
                        .bg(state.config.theme.base.selection)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(list, area, &mut self.list_state);
        }
    }

    fn render_configure_arg(
        &mut self,
        f: &mut Frame,
        area: Rect,
        state: &AppState,
        block: Block<'static>,
        arg_idx: usize,
    ) {
        let tool = &state.available_tools[self.selected_tool_idx.unwrap()];
        let arg_name = &tool.parameters[arg_idx];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .margin(1)
            .split(area);

        f.render_widget(
            block.title(format!(
                "Configure '{}' ({}/{})",
                tool.name,
                arg_idx + 1,
                tool.parameters.len()
            )),
            area,
        );

        let mut lines = vec![Line::from("Enter value for argument:")];
        lines.push(Line::from(Span::styled(
            arg_name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        f.render_widget(Paragraph::new(lines), chunks[0]);
        f.render_widget(&self.input, chunks[1]);
    }

    fn render_name_command(
        &mut self,
        f: &mut Frame,
        area: Rect,
        state: &AppState,
        block: Block<'static>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .margin(1)
            .split(area);

        f.render_widget(block.title("Final Step: Name Command"), area);

        let mut summary = vec![Line::from("Command Summary:")];
        let tool = &state.available_tools[self.selected_tool_idx.unwrap()];
        summary.push(Line::from(format!("Tool: {}", tool.name)));
        for (k, v) in &self.arg_values {
            summary.push(Line::from(format!("  {k}: {v}")));
        }

        f.render_widget(Paragraph::new(summary), chunks[0]);

        self.input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Alias Name (e.g. mycmd)"),
        );
        f.render_widget(&self.input, chunks[1]);
    }
}
