use super::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct InfoComponent {
    frame_count: usize,
    pun_index: usize,
}

impl Default for InfoComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl InfoComponent {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            pun_index: 0,
        }
    }

    fn get_pun(&self) -> &'static str {
        const PUNS: &[&str] = &[
            "Honking at the mainframe...",
            "Chasing bugs (and breadcrumbs)...",
            "Migrating data south...",
            "Deploying the golden egg...",
            "Flapping wings at warp speed...",
            "Waddling through the code...",
            "Goose is loose in the system...",
            "Compiling feathers...",
            "Synthesizing honks...",
            "Calculating flight path...",
            "Optimizing the gaggle...",
            "Hacking the breadbox...",
            "In a wild goose chase for answers...",
            "Syncing with the flock...",
            "Preening the pixels...",
            "Navigating the digital pond...",
            "Gathering intelligence (and seeds)...",
            "Formatting the V-formation...",
            "Decoding the Matrix (it's all corn)...",
            "System Status: HONK.",
        ];
        PUNS[self.pun_index % PUNS.len()]
    }
}

impl Component for InfoComponent {
    fn handle_event(&mut self, event: &Event, _state: &AppState) -> Result<Option<Action>> {
        if let Event::Tick = event {
            self.frame_count = self.frame_count.wrapping_add(1);
            if self.frame_count % 300 == 0 {
                self.pun_index = self.pun_index.wrapping_add(1);
            }
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let mut spans = Vec::new();
        let theme = &state.config.theme;

        // Check for flash message
        if let Some((msg, _)) = &state.flash_message {
            spans.push(Span::styled(
                msg,
                Style::default()
                    .fg(theme.status.warning)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if state.is_working {
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner = spinner_frames[(self.frame_count / 4) % spinner_frames.len()];

            spans.push(Span::styled(
                format!("{spinner} "),
                Style::default()
                    .fg(theme.status.thinking)
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ));

            if !state.todos.is_empty() {
                let active_task = state
                    .todos
                    .iter()
                    .find(|item| !item.done)
                    .map(|item| item.text.as_str())
                    .unwrap_or("Done!");
                let total = state.todos.len();
                let completed = state.todos.iter().filter(|item| item.done).count();

                spans.push(Span::styled(
                    format!("{active_task} ({completed}/{total}) "),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else {
                spans.push(Span::styled(
                    self.get_pun(),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        } else {
            spans.push(Span::styled(
                "⠿ ",
                Style::default()
                    .fg(theme.status.thinking)
                    .add_modifier(Modifier::BOLD),
            ));

            if !state.todos.is_empty() {
                let total = state.todos.len();
                let completed = state.todos.iter().filter(|item| item.done).count();
                let active_task = state
                    .todos
                    .iter()
                    .find(|item| !item.done)
                    .map(|item| item.text.as_str())
                    .unwrap_or("All tasks completed!");

                spans.push(Span::styled(
                    format!("{active_task} ({completed}/{total}) "),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else if !state.has_worked {
                spans.push(Span::styled(
                    "goose 1.14.0",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else {
                spans.push(Span::styled(
                    "Waiting for user input...",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        }

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}
