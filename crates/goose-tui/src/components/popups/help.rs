use super::popup_block;
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

pub struct HelpPopup;

impl Component for HelpPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if !state.showing_help {
            return Ok(None);
        }

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

        let theme = &state.config.theme;
        let area = centered_rect(60, 60, area);
        f.render_widget(Clear, area);

        let text = vec![
            Line::from(Span::styled(
                "Goose TUI Help",
                Style::default()
                    .fg(theme.status.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Keybindings:",
                Style::default()
                    .fg(theme.base.foreground)
                    .add_modifier(Modifier::UNDERLINED),
            )),
            help_line("Enter", "Send message / Select item", theme),
            help_line("Ctrl+J", "Insert newline", theme),
            help_line("Esc", "Switch to Normal mode / Close popups", theme),
            help_line("i", "Switch to Editing mode", theme),
            help_line("j / k", "Scroll / Navigate", theme),
            help_line("Ctrl+S", "Toggle copy mode (disable mouse capture)", theme),
            help_line("Ctrl+T", "Toggle Todo List", theme),
            help_line("Ctrl+C", "Interrupt / Clear / Quit", theme),
            Line::from(""),
            Line::from(Span::styled(
                "Slash Commands:",
                Style::default()
                    .fg(theme.base.foreground)
                    .add_modifier(Modifier::UNDERLINED),
            )),
            help_line("/help", "Show this help", theme),
            help_line("/copy", "Toggle copy mode", theme),
            help_line("/compact", "Compact conversation history", theme),
            help_line("/config", "Open configuration", theme),
            help_line("/session", "Open session picker", theme),
            help_line("/theme", "Change theme (e.g. /theme light)", theme),
            help_line("/alias", "Create custom command", theme),
            Line::from(""),
            Line::from(Span::styled(
                "  Custom commands can be defined via /alias.",
                Style::default().fg(theme.base.border),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Tips:",
                Style::default()
                    .fg(theme.base.foreground)
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(Span::styled(
                "  Press Enter on a message to view details,",
                Style::default().fg(theme.base.foreground),
            )),
            Line::from(Span::styled(
                "  then 'c' to copy, or 'f' to fork session.",
                Style::default().fg(theme.base.foreground),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Forking creates a new session with messages",
                Style::default().fg(theme.base.foreground),
            )),
            Line::from(Span::styled(
                "  up to the selected point, letting you explore",
                Style::default().fg(theme.base.foreground),
            )),
            Line::from(Span::styled(
                "  a different path. Original session is unchanged.",
                Style::default().fg(theme.base.foreground),
            )),
        ];

        let block = popup_block(" Help (Esc to close) ", theme);
        f.render_widget(Paragraph::new(text).block(block), area);
    }
}

fn help_line<'a>(key: &'a str, desc: &'a str, theme: &crate::utils::styles::Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {key:<12}"),
            Style::default().fg(theme.status.info),
        ),
        Span::styled(desc, Style::default().fg(theme.base.foreground)),
    ])
}
