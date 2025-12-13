use super::{navigate_list, popup_block, render_hints, render_scrollbar};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
use crate::utils::layout::centered_rect;
use crate::utils::styles::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use goose::providers::base::ModelInfo;
use goose_client::ProviderDetails;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph, ScrollbarState, Tabs};
use ratatui::Frame;

pub struct ConfigPopup {
    tab_index: usize,
    list_state: ListState,
    scroll_state: ScrollbarState,
    selected_provider_idx: Option<usize>,
    search_query: String,
    initialized: bool,
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
            list_state: ListState::default().with_selected(Some(0)),
            scroll_state: ScrollbarState::default(),
            selected_provider_idx: None,
            search_query: String::new(),
            initialized: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
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
        self.tab_index = if self.tab_index > 0 {
            self.tab_index - 1
        } else {
            1
        };
        self.list_state.select(Some(0));
        self.scroll_state = ScrollbarState::default();
    }

    fn navigate(&mut self, delta: i32, count: usize) {
        if let Some(next) = navigate_list(self.list_state.selected(), delta, count) {
            self.list_state.select(Some(next));
            self.scroll_state = self.scroll_state.position(next);
        }
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

    fn handle_key_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        let count = self.get_item_count(state);

        match key.code {
            KeyCode::Esc => {
                if self.selected_provider_idx.is_some() {
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
            KeyCode::Tab | KeyCode::Right if self.selected_provider_idx.is_none() => {
                self.next_tab();
            }
            KeyCode::BackTab | KeyCode::Left => {
                if self.selected_provider_idx.is_some() && self.search_query.is_empty() {
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
                        'j' => self.navigate(1, count),
                        'k' => self.navigate(-1, count),
                        _ => {}
                    }
                }
            }
            KeyCode::Backspace if self.selected_provider_idx.is_some() => {
                self.search_query.pop();
                self.list_state.select(Some(0));
                self.scroll_state = self.scroll_state.position(0);
            }
            KeyCode::Down => self.navigate(1, count),
            KeyCode::Up => self.navigate(-1, count),
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if self.tab_index == 0 {
                        if let Some(p_idx) = self.selected_provider_idx {
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
                            let providers = self.get_display_providers(state);
                            if let Some(&provider) = providers.get(idx) {
                                self.selected_provider_idx = Some(idx);
                                self.list_state.select(Some(0));
                                self.scroll_state = ScrollbarState::default();
                                self.search_query.clear();
                                return Ok(Some(Action::FetchModels(provider.name.clone())));
                            }
                        }
                    } else if let Some(ext) = state.extensions.get(idx) {
                        return Ok(Some(Action::ToggleExtension {
                            name: ext.config.name().to_string(),
                            enabled: !ext.enabled,
                        }));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render_models(&mut self, f: &mut Frame, area: Rect, state: &AppState, provider_idx: usize) {
        let theme = &state.config.theme;
        let models = self.get_filtered_models(state, provider_idx);

        if models.is_empty() {
            f.render_widget(
                Paragraph::new("No matching models.")
                    .alignment(ratatui::layout::Alignment::Center)
                    .style(Style::default().fg(theme.base.border)),
                area,
            );
            return;
        }

        let items: Vec<ListItem> = models
            .iter()
            .map(|m| {
                ListItem::new(Line::from(Span::styled(
                    &m.name,
                    Style::default().fg(theme.base.foreground),
                )))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(theme.base.selection)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        self.scroll_state = self.scroll_state.content_length(models.len());
        f.render_stateful_widget(list, area, &mut self.list_state);
        render_scrollbar(f, area, &mut self.scroll_state);
    }

    fn render_providers(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let providers = self.get_display_providers(state);

        if providers.is_empty() {
            f.render_widget(
                Paragraph::new("Loading providers...")
                    .alignment(ratatui::layout::Alignment::Center)
                    .style(Style::default().fg(theme.base.border)),
                area,
            );
            return;
        }

        let items: Vec<ListItem> = providers
            .iter()
            .map(|p| {
                let is_active = state.active_provider.as_deref() == Some(&p.name);
                let style = if p.is_configured {
                    Style::default().fg(theme.base.foreground)
                } else {
                    Style::default().fg(theme.base.border)
                };

                let (symbol, symbol_style) = if is_active {
                    ("✔ ", Style::default().fg(theme.status.success))
                } else if p.is_configured {
                    ("● ", Style::default().fg(theme.status.info))
                } else {
                    ("  ", Style::default())
                };

                let line1 = Line::from(vec![
                    Span::styled(symbol, symbol_style),
                    Span::styled(&p.metadata.display_name, style.add_modifier(Modifier::BOLD)),
                ]);
                let line2 = Line::from(Span::styled(
                    format!("  {}", &p.metadata.description),
                    Style::default().fg(theme.base.border),
                ));

                ListItem::new(vec![line1, line2, Line::from("")])
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(theme.base.selection))
            .highlight_symbol("▶ ");

        self.scroll_state = self.scroll_state.content_length(providers.len());
        f.render_stateful_widget(list, area, &mut self.list_state);
        render_scrollbar(f, area, &mut self.scroll_state);
    }

    fn render_extensions(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;

        if state.extensions.is_empty() {
            f.render_widget(
                Paragraph::new("Loading extensions...")
                    .alignment(ratatui::layout::Alignment::Center)
                    .style(Style::default().fg(theme.base.border)),
                area,
            );
            return;
        }

        let items: Vec<ListItem> = state
            .extensions
            .iter()
            .map(|e| {
                let name_str = e.config.name();
                let desc_str = match &e.config {
                    goose::agents::ExtensionConfig::Platform { description, .. }
                    | goose::agents::ExtensionConfig::Stdio { description, .. }
                    | goose::agents::ExtensionConfig::Sse { description, .. }
                    | goose::agents::ExtensionConfig::Builtin { description, .. }
                    | goose::agents::ExtensionConfig::StreamableHttp { description, .. }
                    | goose::agents::ExtensionConfig::Frontend { description, .. }
                    | goose::agents::ExtensionConfig::InlinePython { description, .. } => {
                        description
                    }
                };

                let (check, check_style) = if e.enabled {
                    ("[x] ", Style::default().fg(theme.status.success))
                } else {
                    ("[ ] ", Style::default().fg(theme.base.border))
                };

                let text_style = if e.enabled {
                    Style::default().fg(theme.base.foreground)
                } else {
                    Style::default().fg(theme.base.border)
                };

                let line1 = Line::from(vec![
                    Span::styled(check, check_style),
                    Span::styled(name_str, text_style.add_modifier(Modifier::BOLD)),
                ]);
                let line2 = Line::from(Span::styled(
                    format!("  {desc_str}"),
                    Style::default().fg(theme.base.border),
                ));

                ListItem::new(vec![line1, line2, Line::from("")])
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(theme.base.selection))
            .highlight_symbol("▶ ");

        self.scroll_state = self.scroll_state.content_length(state.extensions.len());
        f.render_stateful_widget(list, area, &mut self.list_state);
        render_scrollbar(f, area, &mut self.scroll_state);
    }
}

impl Component for ConfigPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        let initial_tab = match state.active_popup {
            ActivePopup::Config(tab) => tab,
            _ => {
                self.reset();
                return Ok(None);
            }
        };
        if !self.initialized {
            self.tab_index = initial_tab;
            self.list_state.select(Some(0));
            self.initialized = true;
        }

        match event {
            Event::Input(key) => self.handle_key_input(*key, state),
            Event::Mouse(mouse) => {
                let count = self.get_item_count(state);
                match mouse.kind {
                    MouseEventKind::ScrollDown => self.navigate(1, count),
                    MouseEventKind::ScrollUp => self.navigate(-1, count),
                    _ => {}
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let area = centered_rect(70, 70, area);
        f.render_widget(Clear, area);

        let title = if let Some(idx) = self.selected_provider_idx {
            let providers = self.get_display_providers(state);
            if let Some(&provider) = providers.get(idx) {
                format!(" Select Model: {} ", provider.metadata.display_name)
            } else {
                " Select Model ".to_string()
            }
        } else {
            " Configuration ".to_string()
        };

        f.render_widget(popup_block(&title, theme), area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        if self.selected_provider_idx.is_none() {
            render_tabs(f, chunks[0], self.tab_index, theme);
        } else {
            render_search_box(f, chunks[0], &self.search_query, theme);
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

        let hints = if self.selected_provider_idx.is_some() {
            vec![("↑↓", "nav"), ("Enter", "select"), ("Esc", "back")]
        } else if self.tab_index == 0 {
            vec![
                ("Tab", "switch"),
                ("↑↓", "nav"),
                ("Enter", "select"),
                ("Esc", "close"),
            ]
        } else {
            vec![
                ("Tab", "switch"),
                ("↑↓", "nav"),
                ("Enter", "toggle"),
                ("Esc", "close"),
            ]
        };
        render_hints(f, chunks[2], theme, &hints);
    }
}

fn render_tabs(f: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    let titles: Vec<Line> = ["Providers", "Extensions"]
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(theme.base.foreground))))
        .collect();

    let tabs = Tabs::new(titles).select(selected).highlight_style(
        Style::default()
            .fg(theme.status.warning)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(tabs, area);
}

fn render_search_box(f: &mut Frame, area: Rect, query: &str, theme: &Theme) {
    let text = if query.is_empty() {
        "Type to search...".to_string()
    } else {
        format!("Search: {query}_")
    };

    let style = if query.is_empty() {
        Style::default().fg(theme.base.border)
    } else {
        Style::default().fg(theme.status.warning)
    };

    f.render_widget(Paragraph::new(text).style(style), area);
}
