use goose::config::ExtensionEntry;
use iocraft::prelude::*;

use crate::colors::*;

#[derive(Default, Props)]
pub struct ExtensionDialogProps {
    pub extensions: Vec<ExtensionEntry>,
    pub selected_idx: usize,
    pub width: u16,
}

#[component]
pub fn ExtensionDialog(props: &ExtensionDialogProps) -> impl Into<AnyElement<'static>> {
    if props.extensions.is_empty() {
        return element! {
            View(
                flex_direction: FlexDirection::Column,
                width: props.width,
                border_style: BorderStyle::Round,
                border_color: TEAL,
                padding_left: 1,
                padding_right: 1,
                margin_top: 1,
            ) {
                Text(content: "extensions".to_string(), color: TEAL, weight: Weight::Bold)
                Text(content: "no extensions configured".to_string(), color: TEXT_DIM)
                Text(content: "esc to close".to_string(), color: TEXT_DIM, italic: true)
            }
        };
    }

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: props.width,
            border_style: BorderStyle::Round,
            border_color: TEAL,
            padding_left: 1,
            padding_right: 1,
            margin_top: 1,
        ) {
            Text(content: "extensions".to_string(), color: TEAL, weight: Weight::Bold)
            #(props.extensions.iter().enumerate().map(|(i, entry)| {
                let selected = i == props.selected_idx;
                let name = entry.config.name();
                let status_color = if entry.enabled { TEAL } else { TEXT_DIM };
                let status_glyph = if entry.enabled { "✓" } else { "○" };
                let row_color = if selected { TEXT_PRIMARY } else { TEXT_DIM };
                let cursor = if selected { "❯ " } else { "  " };
                element! {
                    View(key: i.to_string(), flex_direction: FlexDirection::Row) {
                        Text(content: cursor.to_string(), color: TEAL, weight: Weight::Bold)
                        Text(content: format!("{status_glyph} "), color: status_color)
                        Text(content: name, color: row_color)
                    }
                }
            }))
            Text(content: "space/enter to toggle · esc to close".to_string(), color: TEXT_DIM, italic: true)
        }
    }
}
