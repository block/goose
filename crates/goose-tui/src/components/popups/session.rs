use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub struct SessionPopup {
    list_state: ListState,
}

impl SessionPopup {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
    }
}

impl Component for SessionPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if !state.showing_session_picker {
            // Reset selection when not showing, so next open starts fresh
            // But wait, handle_event is called even if not showing? 
            // App logic calls only if state.showing_session_picker is true.
            // So we can't reset here "when closed". 
            // We rely on render to set initial selection if None.
            // Or we need a Reset action?
            // Let's just ensure render sets it if None.
            // render logic handles it: if self.list_state.selected().is_none() && !state.available_sessions.is_empty() { select(0) }
            // The issue is if it WAS selected before, it remembers.
            // We should reset on ClosePopup or OpenSessionPicker action in reducer?
            // Reducer can't touch Component state (list_state).
            // So the Component must detect "Just Opened".
            // But it's stateless mostly.
            // Let's reset selection if we are at the end of the list and list shrank?
            // Or just reset to 0 every time it's opened?
            // We can check if state.showing_session_picker became true? No history here.
            
            // Workaround: in render, if state.available_sessions changed?
            // Let's just use the existing render logic but maybe force 0 if `state.available_sessions` is populated and nothing selected.
            // Actually, if the user says "picker doesn't show any sessions", it implies `available_sessions` is empty.
            // This might be an async timing issue.
            return Ok(None);
        }

        if let Event::Input(key) = event {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('j') | KeyCode::Down => {
                    let idx = self.list_state.selected().unwrap_or(0);
                    let max = state.available_sessions.len().saturating_sub(1);
                    self.list_state.select(Some((idx + 1).min(max)));
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let idx = self.list_state.selected().unwrap_or(0);
                    self.list_state.select(Some(idx.saturating_sub(1)));
                }
                KeyCode::Enter => {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(session) = state.available_sessions.get(idx) {
                            return Ok(Some(Action::ResumeSession(session.id.clone())));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_session_picker {
            return;
        }

        let area = centered_rect(60, 60, area);
        f.render_widget(Clear, area);

        let items: Vec<ListItem> = state
            .available_sessions
            .iter()
            .map(|s| {
                let id = Span::styled(
                    format!("{:<10}", &s.id[..8]),
                    Style::default().fg(Color::Cyan),
                );
                let name = Span::styled(&s.name, Style::default().fg(Color::White));
                ListItem::new(Line::from(vec![id, Span::raw(" "), name]))
            })
            .collect();

        let block = Block::default()
            .title("Sessions (Enter to Resume)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(state.config.theme.base.background));

        if items.is_empty() {
            let p = Paragraph::new("No saved sessions found.")
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(p, area);
            return;
        }

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(state.config.theme.base.selection)
                .add_modifier(Modifier::BOLD),
        );

        if self.list_state.selected().is_none() && !state.available_sessions.is_empty() {
            self.list_state.select(Some(0));
        }

        f.render_stateful_widget(list, area, &mut self.list_state);
    }
}
