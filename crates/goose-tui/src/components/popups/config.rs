use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use goose::providers::base::ModelInfo;
use goose_client::ProviderDetails;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Tabs,
};
use ratatui::Frame;

pub struct ConfigPopup {
    tab_index: usize,
    list_state: ListState,
    scroll_state: ScrollbarState,
    selected_provider_idx: Option<usize>,
    search_query: String,
}

impl Default for ConfigPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigPopup {
    pub fn new() -> Self {
        Self {
            tab_index: 0,
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
            selected_provider_idx: None,
            search_query: String::new(),
        }
    }

    fn next_tab(&mut self) {
        if self.selected_provider_idx.is_some() {
            return;
        }
        self.tab_index = (self.tab_index + 1) % 2;
        self.list_state.select(Some(0));
        self.scroll_state = ScrollbarState::default();
    }

    fn previous_tab(&mut self) {
        if self.selected_provider_idx.is_some() {
            return;
        }
        if self.tab_index > 0 {
            self.tab_index -= 1;
        } else {
            self.tab_index = 1;
        }
        self.list_state.select(Some(0));
        self.scroll_state = ScrollbarState::default();
    }

    fn reset(&mut self) {
        self.tab_index = 0;
        self.list_state.select(Some(0));
        self.scroll_state = ScrollbarState::default();
        self.selected_provider_idx = None;
        self.search_query.clear();
    }

    fn get_display_providers<'a>(&self, state: &'a AppState) -> Vec<&'a ProviderDetails> {
        let mut providers: Vec<&ProviderDetails> = state.providers.iter().collect();
        if let Some(active_provider_name) = &state.active_provider {
            if let Some(pos) = providers
                .iter()
                .position(|p| &p.name == active_provider_name)
            {
                let active_provider = providers.remove(pos);
                providers.insert(0, active_provider);
            }
        }
        providers
    }

    fn get_filtered_models<'a>(
        &self,
        state: &'a AppState,
        provider_idx: usize,
    ) -> Vec<&'a ModelInfo> {
        let providers = self.get_display_providers(state);
        if let Some(provider) = providers.get(provider_idx) {
            if self.search_query.is_empty() {
                return provider.metadata.known_models.iter().collect();
            }
            let query = self.search_query.to_lowercase();
            return provider
                .metadata
                .known_models
                .iter()
                .filter(|m| m.name.to_lowercase().contains(&query))
                .collect();
        }
        Vec::new()
    }

    fn get_item_count(&self, state: &AppState) -> usize {
        if self.tab_index == 0 {
            if let Some(idx) = self.selected_provider_idx {
                return self.get_filtered_models(state, idx).len();
            }
            state.providers.len()
        } else {
            state.extensions.len()
        }
    }

    fn render_models(&mut self, f: &mut Frame, area: Rect, state: &AppState, provider_idx: usize) {
        let models = self.get_filtered_models(state, provider_idx);

        let items: Vec<ListItem> = models
            .iter()
            .map(|m| {
                let name = &m.name;
                let line = Line::from(Span::styled(name, Style::default().fg(Color::White)));
                ListItem::new(line)
            })
            .collect();

        if items.is_empty() {
            f.render_widget(
                Paragraph::new("No matching models.")
                    .alignment(ratatui::layout::Alignment::Center),
                area,
            );
            return;
        }

        let list = List::new(items).highlight_style(
            Style::default()
                .bg(state.config.theme.base.selection)
                .add_modifier(Modifier::BOLD),
        );

        if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }

        self.scroll_state = self.scroll_state.content_length(models.len());

        f.render_stateful_widget(list, area, &mut self.list_state);

        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_providers(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let providers = self.get_display_providers(state);

        let items: Vec<ListItem> = providers
            .iter()
            .map(|p| {
                let name = &p.metadata.display_name;
                let desc = &p.metadata.description;
                let style = if p.is_configured {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let is_active = state.active_provider.as_deref() == Some(&p.name);

                let (symbol, color) = if is_active {
                    ("✔", Color::Green)
                } else if p.is_configured {
                    ("●", Color::Blue)
                } else {
                    (" ", Color::Reset)
                };

                let line1 = Line::from(vec![
                    Span::styled(format!("{} ", symbol), Style::default().fg(color)),
                    Span::styled(name, style.add_modifier(Modifier::BOLD)),
                ]);
                let line2 = Line::from(Span::styled(
                    format!("  {}", desc),
                    Style::default().fg(Color::Gray),
                ));

                ListItem::new(vec![line1, line2, Line::from("")])
            })
            .collect();

        if items.is_empty() {
            f.render_widget(
                Paragraph::new("Loading providers...")
                    .alignment(ratatui::layout::Alignment::Center),
                area,
            );
            return;
        }

        let list = List::new(items)
            .highlight_style(Style::default().bg(state.config.theme.base.selection));

        if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }

        self.scroll_state = self.scroll_state.content_length(providers.len());

        f.render_stateful_widget(list, area, &mut self.list_state);

        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_extensions(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let items: Vec<ListItem> = state
            .extensions
            .iter()
            .map(|e| {
                let name_str = e.config.name();
                let desc_str = match &e.config {
                    goose::agents::ExtensionConfig::Platform { description, .. } => description,
                    goose::agents::ExtensionConfig::Stdio { description, .. } => description,
                    goose::agents::ExtensionConfig::Sse { description, .. } => description,
                    goose::agents::ExtensionConfig::Builtin { description, .. } => description,
                    goose::agents::ExtensionConfig::StreamableHttp { description, .. } => {
                        description
                    }
                    goose::agents::ExtensionConfig::Frontend { description, .. } => description,
                    goose::agents::ExtensionConfig::InlinePython { description, .. } => description,
                };

                let check = if e.enabled { "[x]" } else { "[ ]" };
                let style = if e.enabled {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };

                let line1 = Line::from(vec![
                    Span::styled(
                        format!("{} ", check),
                        if e.enabled {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(name_str, style.add_modifier(Modifier::BOLD)),
                ]);
                let line2 = Line::from(Span::styled(
                    format!("  {}", desc_str),
                    Style::default().fg(Color::DarkGray),
                ));

                ListItem::new(vec![line1, line2, Line::from("")])
            })
            .collect();

        if items.is_empty() {
            f.render_widget(
                Paragraph::new("Loading extensions...")
                    .alignment(ratatui::layout::Alignment::Center),
                area,
            );
            return;
        }

        let list = List::new(items)
            .highlight_style(Style::default().bg(state.config.theme.base.selection));

        if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }

        self.scroll_state = self.scroll_state.content_length(state.extensions.len());

        f.render_stateful_widget(list, area, &mut self.list_state);

        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut self.scroll_state,
        );
    }
}

impl Component for ConfigPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if !state.showing_config {
            self.reset();
            return Ok(None);
        }

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc => {
                    if self.selected_provider_idx.is_some() {
                        // Go back to provider list
                        self.list_state.select(self.selected_provider_idx);
                        self.selected_provider_idx = None;
                        self.search_query.clear();
                        return Ok(None);
                    }
                    self.reset();
                    return Ok(Some(Action::ClosePopup));
                }
                KeyCode::Char('q')
                    if self.search_query.is_empty() && self.selected_provider_idx.is_none() =>
                {
                    self.reset();
                    return Ok(Some(Action::ClosePopup));
                }
                KeyCode::Tab => {
                    self.next_tab();
                }
                KeyCode::BackTab => {
                    self.previous_tab();
                }
                KeyCode::Right => {
                    if self.selected_provider_idx.is_none() {
                        self.next_tab();
                    }
                }
                KeyCode::Left => {
                    if self.selected_provider_idx.is_some() && self.search_query.is_empty() {
                        // Go back
                        self.list_state.select(self.selected_provider_idx);
                        self.selected_provider_idx = None;
                        self.search_query.clear();
                    } else if self.selected_provider_idx.is_none() {
                        self.previous_tab();
                    }
                }
                KeyCode::Char(c) => {
                    if self.selected_provider_idx.is_some() {
                        self.search_query.push(c);
                        self.list_state.select(Some(0));
                        self.scroll_state = self.scroll_state.position(0);
                    } else {
                        match c {
                            'l' => self.next_tab(),
                            'h' => self.previous_tab(),
                            'j' => {
                                let count = self.get_item_count(state);
                                if count > 0 {
                                    let i = match self.list_state.selected() {
                                        Some(i) => {
                                            if i >= count - 1 {
                                                0
                                            } else {
                                                i + 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    self.list_state.select(Some(i));
                                    self.scroll_state = self.scroll_state.position(i);
                                }
                            }
                            'k' => {
                                let count = self.get_item_count(state);
                                if count > 0 {
                                    let i = match self.list_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                count - 1
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    self.list_state.select(Some(i));
                                    self.scroll_state = self.scroll_state.position(i);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                KeyCode::Backspace => {
                    if self.selected_provider_idx.is_some() {
                        self.search_query.pop();
                        self.list_state.select(Some(0));
                        self.scroll_state = self.scroll_state.position(0);
                    }
                }
                KeyCode::Down => {
                    let count = self.get_item_count(state);
                    if count > 0 {
                        let i = match self.list_state.selected() {
                            Some(i) => {
                                if i >= count - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        self.list_state.select(Some(i));
                        self.scroll_state = self.scroll_state.position(i);
                    }
                }
                KeyCode::Up => {
                    let count = self.get_item_count(state);
                    if count > 0 {
                        let i = match self.list_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    count - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        self.list_state.select(Some(i));
                        self.scroll_state = self.scroll_state.position(i);
                    }
                }
                KeyCode::Enter => {
                    if let Some(idx) = self.list_state.selected() {
                        if self.tab_index == 0 {
                            if let Some(p_idx) = self.selected_provider_idx {
                                // Model selection (filtered)
                                let models = self.get_filtered_models(state, p_idx);
                                let providers = self.get_display_providers(state);
                                if let Some(&provider) = providers.get(p_idx) {
                                    if let Some(model) = models.get(idx) {
                                        self.reset();
                                        return Ok(Some(Action::UpdateProvider {
                                            provider: provider.name.clone(),
                                            model: model.name.clone(),
                                        }));
                                    }
                                }
                            } else {
                                // Provider selection - Drill down
                                let providers = self.get_display_providers(state);
                                if let Some(&provider) = providers.get(idx) {
                                    self.selected_provider_idx = Some(idx);
                                    self.list_state.select(Some(0));
                                    self.scroll_state = ScrollbarState::default();
                                    self.search_query.clear();
                                    return Ok(Some(Action::FetchModels(provider.name.clone())));
                                }
                            }
                        } else {
                            // Extension toggle
                            if let Some(ext) = state.extensions.get(idx) {
                                return Ok(Some(Action::ToggleExtension {
                                    name: ext.config.name().to_string(),
                                    enabled: !ext.enabled,
                                }));
                            }
                        }
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown => {
                    let count = self.get_item_count(state);
                    if count > 0 {
                        let i = self.list_state.selected().unwrap_or(0);
                        let next = if i >= count - 1 { 0 } else { i + 1 };
                        self.list_state.select(Some(next));
                        self.scroll_state = self.scroll_state.position(next);
                    }
                }
                MouseEventKind::ScrollUp => {
                    let count = self.get_item_count(state);
                    if count > 0 {
                        let i = self.list_state.selected().unwrap_or(0);
                        let next = if i == 0 { count - 1 } else { i - 1 };
                        self.list_state.select(Some(next));
                        self.scroll_state = self.scroll_state.position(next);
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_config {
            return;
        }

        let area = centered_rect(70, 70, area);
        f.render_widget(Clear, area);

        let title = if let Some(idx) = self.selected_provider_idx {
            let providers = self.get_display_providers(state);
            if let Some(&provider) = providers.get(idx) {
                format!("Select Model for {}", provider.metadata.display_name)
            } else {
                "Select Model".to_string()
            }
        } else {
            "Configuration".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .style(Style::default().bg(state.config.theme.base.background));

        f.render_widget(block.clone(), area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        if self.selected_provider_idx.is_none() {
            let titles: Vec<Line> = vec!["Providers", "Extensions"]
                .iter()
                .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::Green))))
                .collect();

            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::BOTTOM))
                .select(self.tab_index)
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_widget(tabs, chunks[0]);
        } else {
            // Show search box
            let search_text = if self.search_query.is_empty() {
                "Type to search... (Esc to back)".to_string()
            } else {
                format!("Search: {}_", self.search_query)
            };

            let p = Paragraph::new(search_text)
                .block(Block::default().borders(Borders::BOTTOM))
                .style(if self.search_query.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Yellow)
                });

            f.render_widget(p, chunks[0]);
        }

        if self.tab_index == 0 {
            if let Some(idx) = self.selected_provider_idx {
                self.render_models(f, chunks[1], state, idx);
            } else {
                self.render_providers(f, chunks[1], state);
            }
        } else {
            self.render_extensions(f, chunks[1], state);
        }
    }
}