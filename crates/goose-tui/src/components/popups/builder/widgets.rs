use crate::services::config::CustomCommand;
use crate::utils::json::has_input_placeholder;
use crate::utils::styles::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, ListItem};
use ratatui_textarea::TextArea;

pub fn input_block(title: &str, focused: bool, theme: &Theme) -> Block<'static> {
    let border_color = if focused {
        theme.base.border_active
    } else {
        theme.base.border
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title.to_string())
}

pub fn new_text_input(placeholder: &str) -> TextArea<'static> {
    let mut ta = TextArea::default();
    ta.set_cursor_line_style(Style::default());
    ta.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(placeholder.to_string()),
    );
    ta
}

pub fn alias_list_item(cmd: &CustomCommand, theme: &Theme) -> ListItem<'static> {
    let short_tool = cmd.tool.split("__").last().unwrap_or(&cmd.tool);
    let args_preview = preview_args(&cmd.args);

    let mut first_line = vec![
        Span::styled(
            format!("/{}", cmd.name),
            Style::default()
                .fg(theme.status.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" → {short_tool}"),
            Style::default().fg(theme.status.info),
        ),
    ];

    if has_input_placeholder(&cmd.args) {
        first_line.push(Span::styled(
            " <args>".to_string(),
            Style::default().fg(theme.status.warning),
        ));
    }

    ListItem::new(vec![
        Line::from(first_line),
        Line::from(Span::styled(
            format!("  {args_preview}"),
            Style::default().fg(theme.base.border),
        )),
    ])
}

pub fn build_preview_spans<'a>(preview: &str, has_input: bool, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = vec![Span::styled(
        "Preview: ".to_string(),
        Style::default().fg(theme.base.border),
    )];

    if has_input {
        let parts: Vec<&str> = preview.split("{input}").collect();
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                spans.push(Span::styled(
                    (*part).to_string(),
                    Style::default().fg(theme.status.success),
                ));
            }
            if i < parts.len() - 1 {
                spans.push(Span::styled(
                    "{input}".to_string(),
                    Style::default().fg(theme.status.warning),
                ));
            }
        }
        spans.push(Span::styled(
            " (accepts args)".to_string(),
            Style::default().fg(theme.status.warning),
        ));
    } else {
        spans.push(Span::styled(
            preview.to_string(),
            Style::default().fg(theme.status.success),
        ));
    }

    spans
}

fn preview_args(args: &serde_json::Value) -> String {
    args.as_object()
        .map(|obj| {
            let parts: Vec<String> = obj
                .iter()
                .take(2)
                .map(|(k, v)| {
                    let val: String = v.as_str().unwrap_or("").chars().take(20).collect();
                    format!("{k}={val}")
                })
                .collect();
            if parts.is_empty() {
                "(no args)".to_string()
            } else {
                parts.join(", ")
            }
        })
        .unwrap_or_default()
}
