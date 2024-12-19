use std::rc::Rc;

use ratatui::{
    layout::{self, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, HighlightSpacing, List, ListState, Paragraph},
    Frame,
};

pub fn chunks_for_list_and_view_split(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(layout::Direction::Horizontal)
        .constraints([layout::Constraint::Length(33), layout::Constraint::Min(1)])
        .split(area)
}

pub fn render_left_list(
    f: &mut Frame,
    header: String,
    items: Vec<String>,
    list_state: &mut ListState,
    is_focussed_view: bool,
    area: Rect,
) {
    // Main - left list area
    let list_chunks = Layout::default()
        .direction(layout::Direction::Vertical)
        .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1)])
        .split(area);

    let list_header = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("   "),
            Span::styled(header, Style::default().add_modifier(Modifier::ITALIC)),
        ]),
        Line::from(vec!["".into()]),
    ])
    .block(Block::default().borders(Borders::RIGHT));
    f.render_widget(list_header, list_chunks[0]);

    let mut list = List::new(items.clone())
        .highlight_symbol(" > ")
        .highlight_spacing(HighlightSpacing::Always)
        .block(Block::default().borders(Borders::RIGHT));
    if is_focussed_view {
        list = list.highlight_style(Style::default().add_modifier(Modifier::BOLD));
    }
    f.render_stateful_widget(list, list_chunks[1], list_state);
}
