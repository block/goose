use super::{popup_block, render_hints, PopupScrollState};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

pub struct HelpPopup {
    scroll_state: PopupScrollState,
}

impl Default for HelpPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpPopup {
    pub fn new() -> Self {
        Self {
            scroll_state: PopupScrollState::new(),
        }
    }
}

impl Component for HelpPopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.active_popup != ActivePopup::Help {
            self.scroll_state.reset();
            return Ok(None);
        }

        match event {
            Event::Input(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(Some(Action::ClosePopup)),
                KeyCode::Char('j') | KeyCode::Down => self.scroll_state.scroll_by(1),
                KeyCode::Char('k') | KeyCode::Up => self.scroll_state.scroll_by(-1),
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => self.scroll_state.scroll_by(3),
                MouseEventKind::ScrollUp => self.scroll_state.scroll_by(-3),
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let area = centered_rect(60, 70, area);
        f.render_widget(Clear, area);

        let [content_area, hints_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .areas(area);

        f.render_widget(popup_block(" Help ", theme), area);

        let lines = vec![
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
            help_line(
                "/mode",
                "Set mode (auto, approve, chat, smart_approve)",
                theme,
            ),
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

        self.scroll_state.content_height = lines.len() as u16;
        self.scroll_state.viewport_height = content_area.height;
        self.scroll_state.clamp();

        let p = Paragraph::new(lines).scroll((self.scroll_state.scroll, 0));
        f.render_widget(p, content_area);

        self.scroll_state
            .render_transient_scrollbar(f, content_area);

        render_hints(f, hints_area, theme, &[("↑↓", "scroll"), ("Esc", "close")]);
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
