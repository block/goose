use super::Component;
use crate::state::{AppState, InputMode};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

fn get_git_branch() -> Option<String> {
    let repo = git2::Repository::discover(".").ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}

fn spans_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

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

    pub fn height(&self, width: u16, state: &AppState) -> u16 {
        let theme = &state.config.theme;
        let (main_spans, hint_spans) = self.build_spans(state, theme);
        let total_width = spans_width(&main_spans) + spans_width(&hint_spans);
        if total_width > width as usize {
            2
        } else {
            1
        }
    }

    fn build_spans(
        &self,
        state: &AppState,
        theme: &crate::utils::styles::Theme,
    ) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
        let mut main_spans = self.get_mode_spans(state, theme);

        if state.copy_mode {
            main_spans.push(Span::styled(
                " COPY ",
                Style::default()
                    .bg(theme.status.success)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
            main_spans.push(Span::raw(" "));
        }

        main_spans.extend(self.get_info_spans(state, theme));

        let mut hint_spans = Vec::new();
        let hints = self.get_hints(state, theme);
        for (i, hint) in hints.into_iter().enumerate() {
            if i > 0 {
                hint_spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            }
            hint_spans.push(hint);
        }

        (main_spans, hint_spans)
    }
}

impl Component for StatusComponent {
    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let (main_spans, hint_spans) = self.build_spans(state, theme);
        let total_width = spans_width(&main_spans) + spans_width(&hint_spans);

        let text = if total_width > area.width as usize && area.height >= 2 {
            Text::from(vec![Line::from(main_spans), Line::from(hint_spans)])
        } else {
            let mut all_spans = main_spans;
            all_spans.extend(hint_spans);
            Text::from(Line::from(all_spans))
        };

        f.render_widget(
            Paragraph::new(text)
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
                if let Some(branch) = get_git_branch() {
                    spans.push(Span::styled(" ⎇ ", Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(
                        branch,
                        Style::default().fg(theme.base.foreground),
                    ));
                }
                spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            }
        }

        if let Some(model) = &state.active_model {
            spans.push(Span::styled(
                model.clone(),
                Style::default().fg(theme.base.foreground),
            ));
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }
        spans
    }

    fn get_hints(
        &self,
        state: &AppState,
        theme: &crate::utils::styles::Theme,
    ) -> Vec<Span<'static>> {
        let mut hints = Vec::new();

        if state.pending_confirmation.is_some() {
            hints.push(Span::styled(
                "Y: Allow",
                Style::default()
                    .fg(theme.status.success)
                    .add_modifier(Modifier::BOLD),
            ));
            hints.push(Span::styled(
                "N: Deny",
                Style::default()
                    .fg(theme.status.error)
                    .add_modifier(Modifier::BOLD),
            ));
            return hints;
        }

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
        }
        hints.push(Span::styled(
            "Ctrl+S: Copy",
            Style::default().fg(Color::DarkGray),
        ));
        hints.push(Span::styled(
            "Ctrl+T: Todos",
            Style::default().fg(Color::DarkGray),
        ));
        hints
    }
}
