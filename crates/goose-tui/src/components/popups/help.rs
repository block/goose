use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

pub struct HelpPopup;

impl Component for HelpPopup {
    fn handle_event(&mut self, event: &Event, _state: &AppState) -> Result<Option<Action>> {
        if let Event::Input(key) = event {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                _ => {}
            }
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        if !state.showing_help {
            return;
        }

        let area = centered_rect(60, 60, area);
        f.render_widget(Clear, area);

        let text = vec![
            Line::from(Span::styled(
                "Goose TUI Help",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Keybindings:",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )),
            Line::from("  Enter       Send message / Select item"),
            Line::from("  Ctrl+J      Insert newline"),
            Line::from("  Esc         Switch to Normal mode / Close popups"),
            Line::from("  i           Switch to Editing mode"),
            Line::from("  j / k       Scroll / Navigate"),
            Line::from("  Ctrl+S      Toggle copy mode (disable mouse capture)"),
            Line::from("  Ctrl+T      Toggle Todo List"),
            Line::from("  Ctrl+C      Interrupt / Clear / Quit"),
            Line::from(""),
            Line::from(Span::styled(
                "Slash Commands:",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )),
            Line::from("  /help       Show this help"),
            Line::from("  /copy       Toggle copy mode"),
            Line::from("  /compact    Compact conversation history"),
            Line::from("  /config     Open configuration"),
            Line::from("  /session    Open session picker"),
            Line::from("  /theme      Change theme (e.g. /theme light)"),
            Line::from("  /alias      Create custom command"),
            Line::from(""),
            Line::from("  Custom commands can be defined in config."),
            Line::from(""),
            Line::from(Span::styled(
                "Tips:",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )),
            Line::from("  Press Enter on a message to view details,"),
            Line::from("  then 'c' to copy, or 'f' to fork session."),
            Line::from(""),
            Line::from("  Forking creates a new session with messages"),
            Line::from("  up to the selected point, letting you explore"),
            Line::from("  a different path. Original session is unchanged."),
        ];

        let block = Block::default()
            .title("Help (Esc to Close)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(state.config.theme.base.background));

        f.render_widget(Paragraph::new(text).block(block), area);
    }
}
