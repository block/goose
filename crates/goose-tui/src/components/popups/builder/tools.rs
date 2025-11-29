use crate::utils::styles::Theme;
use goose_client::ToolInfo;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;
use std::collections::HashMap;

pub fn build_tool_list(
    tools: &[ToolInfo],
    search: &str,
    theme: &Theme,
) -> (Vec<ListItem<'static>>, Vec<Option<usize>>) {
    let mut items = Vec::new();
    let mut indices = Vec::new();

    if search.is_empty() {
        items.push(ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Manage aliases...",
                Style::default()
                    .fg(theme.status.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ])));
        indices.push(None);

        for (group_name, group_indices) in group_tools(tools) {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("─ {group_name} ─"),
                Style::default()
                    .fg(theme.base.border)
                    .add_modifier(Modifier::BOLD),
            ))));
            indices.push(None);

            for idx in group_indices {
                items.push(tool_list_item(&tools[idx], theme));
                indices.push(Some(idx));
            }
        }
    } else {
        let query = search.to_lowercase();
        for (idx, tool) in tools.iter().enumerate() {
            if tool.name.to_lowercase().contains(&query)
                || tool.description.to_lowercase().contains(&query)
            {
                items.push(tool_list_item_with_prefix(tool, theme));
                indices.push(Some(idx));
            }
        }
    }

    (items, indices)
}

fn group_tools(tools: &[ToolInfo]) -> Vec<(String, Vec<usize>)> {
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, tool) in tools.iter().enumerate() {
        let prefix = tool
            .name
            .split("__")
            .next()
            .unwrap_or(&tool.name)
            .to_string();
        groups.entry(prefix).or_default().push(i);
    }
    let mut result: Vec<_> = groups.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

fn tool_list_item(tool: &ToolInfo, theme: &Theme) -> ListItem<'static> {
    let short_name = tool.name.split("__").last().unwrap_or(&tool.name);
    ListItem::new(vec![
        Line::from(Span::styled(
            format!("  {short_name}"),
            Style::default().fg(theme.status.info),
        )),
        Line::from(Span::styled(
            format!("    {}", truncate(&tool.description, 50)),
            Style::default().fg(theme.base.border),
        )),
    ])
}

fn tool_list_item_with_prefix(tool: &ToolInfo, theme: &Theme) -> ListItem<'static> {
    let prefix = tool.name.split("__").next().unwrap_or("");
    let short_name = tool.name.split("__").last().unwrap_or(&tool.name);
    ListItem::new(vec![
        Line::from(vec![
            Span::styled(format!("{prefix}/"), Style::default().fg(theme.base.border)),
            Span::styled(
                short_name.to_string(),
                Style::default().fg(theme.status.info),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {}", truncate(&tool.description, 50)),
            Style::default().fg(theme.base.border),
        )),
    ])
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
