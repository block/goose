use super::Component;
use crate::state::{AppState, InputMode};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

pub struct StatusComponent;

impl Default for StatusComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Component for StatusComponent {
    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let mut spans = self.get_mode_spans(state, theme);
        spans.extend(self.get_info_spans(state, theme));

        let hints = self.get_hints(state, theme);
        for (i, hint) in hints.into_iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            }
            spans.push(hint);
        }

        f.render_widget(
            Paragraph::new(Line::from(spans))
                .block(Block::default().style(Style::default().bg(theme.base.selection))),
            area,
        );
    }
}

impl StatusComponent {
    fn get_mode_spans(
        &self,
        state: &AppState,
        theme: &crate::utils::styles::Theme,
    ) -> Vec<Span<'static>> {
        let mode_bg = if state.is_working {
            theme.status.thinking
        } else {
            match state.input_mode {
                InputMode::Normal => theme.base.selection,
                InputMode::Editing => theme.base.border_active,
            }
        };
        let mode_fg = if state.is_working {
            Color::Black
        } else {
            theme.base.background
        };
        let mode_text = if state.is_working {
            " WORKING "
        } else {
            match state.input_mode {
                InputMode::Normal => " NORMAL ",
                InputMode::Editing => " EDITING ",
            }
        };

        vec![
            Span::styled(
                mode_text,
                Style::default()
                    .bg(mode_bg)
                    .fg(mode_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    }

    fn get_info_spans(
        &self,
        state: &AppState,
        theme: &crate::utils::styles::Theme,
    ) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        spans.push(Span::styled(
            state.session_id.clone(),
            Style::default().fg(theme.base.foreground),
        ));
        spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

        let limit = state.model_context_limit;
        let current = state.token_state.total_tokens;
        let fmt_k = |n: i32| {
            if n >= 1000 {
                format!("{}k", n / 1000)
            } else {
                n.to_string()
            }
        };
        let limit_k = if limit >= 1000 {
            format!("{}k", limit / 1000)
        } else {
            limit.to_string()
        };

        spans.push(Span::styled(
            format!("{}/{}", fmt_k(current), limit_k),
            Style::default().fg(theme.base.foreground),
        ));
        spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

        if let Ok(cwd) = std::env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                spans.push(Span::styled(
                    cwd_str.to_string(),
                    Style::default().fg(theme.base.foreground),
                ));
                spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            }
        }
        spans
    }

    fn get_hints(
        &self,
        state: &AppState,
        theme: &crate::utils::styles::Theme,
    ) -> Vec<Span<'static>> {
        let mut hints = Vec::new();

        if state.is_working {
            hints.push(Span::styled(
                "Ctrl+C Interrupt",
                Style::default().fg(theme.status.warning),
            ));
        } else if !state.input_text_is_empty {
            hints.push(Span::styled(
                "Ctrl+C Clear",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            hints.push(Span::styled(
                "Ctrl+C Quit",
                Style::default().fg(Color::DarkGray),
            ));
        }

        if state.input_mode == InputMode::Normal {
            hints.push(Span::styled(
                "i: Edit",
                Style::default().fg(Color::DarkGray),
            ));
            hints.push(Span::styled(
                "Enter: View",
                Style::default().fg(Color::DarkGray),
            ));
            hints.push(Span::styled(
                "j/k: Scroll",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            hints.push(Span::styled(
                "Esc: Normal",
                Style::default().fg(Color::DarkGray),
            ));
            hints.push(Span::styled(
                "Enter: Send",
                Style::default().fg(Color::DarkGray),
            ));
        }
        hints.push(Span::styled(
            "Ctrl+T: Todos",
            Style::default().fg(Color::DarkGray),
        ));
        hints
    }
}
