use super::tools::build_tool_list;
use super::widgets::{alias_list_item, build_preview_spans, input_block};
use super::BuilderPopup;
use crate::components::popups::{popup_block, render_hints, render_scrollbar};
use crate::state::AppState;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Borders, List, Paragraph};
use ratatui::Frame;

impl BuilderPopup<'_> {
    pub(super) fn render_tool_select(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let (items, _) = build_tool_list(&state.available_tools, &self.search, theme);

        let [search_area, list_area, hints_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .areas(area);

        f.render_widget(popup_block(" Create Alias ", theme), area);

        let search_text = if self.search.is_empty() {
            "Type to search tools...".to_string()
        } else {
            format!("Search: {}_", self.search)
        };
        let search_style = if self.search.is_empty() {
            Style::default().fg(theme.base.border)
        } else {
            Style::default().fg(theme.status.warning)
        };
        f.render_widget(
            Paragraph::new(search_text).style(search_style).block(
                ratatui::widgets::Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.base.border)),
            ),
            search_area,
        );

        self.scroll_state = self.scroll_state.content_length(items.len());
        f.render_stateful_widget(
            List::new(items)
                .highlight_style(Style::default().bg(theme.base.selection))
                .highlight_symbol("▶ "),
            list_area,
            &mut self.list_state,
        );
        render_scrollbar(f, list_area, &mut self.scroll_state);

        render_hints(
            f,
            hints_area,
            theme,
            &[("↑↓", "nav"), ("Enter", "select"), ("Esc", "close")],
        );
    }

    pub(super) fn render_alias_manage(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let commands = &state.config.custom_commands;

        let [list_area, hints_area] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
            .margin(1)
            .areas(area);

        f.render_widget(popup_block(" Manage Aliases ", theme), area);

        if commands.is_empty() {
            f.render_widget(
                Paragraph::new("No aliases defined yet.")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.base.border)),
                list_area,
            );
        } else {
            let items: Vec<_> = commands.iter().map(|c| alias_list_item(c, theme)).collect();
            self.scroll_state = self.scroll_state.content_length(items.len());
            f.render_stateful_widget(
                List::new(items)
                    .highlight_style(Style::default().bg(theme.base.selection))
                    .highlight_symbol("▶ "),
                list_area,
                &mut self.list_state,
            );
            render_scrollbar(f, list_area, &mut self.scroll_state);
        }

        render_hints(
            f,
            hints_area,
            theme,
            &[("e/Enter", "edit"), ("d", "delete"), ("Esc", "back")],
        );
    }

    pub(super) fn render_editor(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let title = if self.editing_alias.is_some() {
            " Edit Alias "
        } else {
            " New Alias "
        };

        let has_input = self.has_input_placeholder();

        let mut constraints: Vec<Constraint> = vec![Constraint::Length(3)];
        constraints.extend(std::iter::repeat_n(
            Constraint::Length(3),
            self.param_inputs.len(),
        ));
        if !has_input && !self.param_inputs.is_empty() {
            constraints.push(Constraint::Length(1));
        }
        constraints.extend([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);

        let chunks = Layout::vertical(constraints).margin(1).split(area);

        f.render_widget(popup_block(title, theme), area);

        self.alias_name
            .set_block(input_block("Alias name", self.focused_field == 0, theme));
        f.render_widget(&self.alias_name, chunks[0]);

        for (i, (param, ta)) in self.param_inputs.iter_mut().enumerate() {
            ta.set_block(input_block(param, self.focused_field == i + 1, theme));
            f.render_widget(&*ta, chunks[i + 1]);
        }

        let mut next_chunk = self.param_inputs.len() + 1;

        if !has_input && !self.param_inputs.is_empty() {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Tip: ", Style::default().fg(theme.base.border)),
                    Span::styled(
                        "Use {input} in a parameter to accept trailing arguments",
                        Style::default().fg(theme.base.border),
                    ),
                ])),
                chunks[next_chunk],
            );
            next_chunk += 1;
        }

        let preview = self.preview_text(&state.available_tools);
        let preview_spans = build_preview_spans(&preview, has_input, theme);
        f.render_widget(
            Paragraph::new(Line::from(preview_spans)),
            chunks[next_chunk],
        );

        render_hints(
            f,
            chunks[next_chunk + 2],
            theme,
            &[("↑↓/Tab", "nav"), ("Enter", "save"), ("Esc", "cancel")],
        );
    }
}
