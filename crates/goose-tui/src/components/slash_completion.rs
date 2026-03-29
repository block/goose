use iocraft::prelude::*;

use crate::colors::*;

#[derive(Default, Props)]
pub struct SlashCompletionProps {
    /// Filtered list of (command, description) pairs.
    pub completions: Vec<(String, String)>,
    pub selected_idx: usize,
    pub width: u16,
}

#[component]
pub fn SlashCompletion(props: &SlashCompletionProps) -> impl Into<AnyElement<'static>> {
    if props.completions.is_empty() {
        return element!(View);
    }

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: props.width,
            border_style: BorderStyle::Round,
            border_color: RULE,
            padding_left: 1,
            padding_right: 1,
        ) {
            #(props.completions.iter().enumerate().map(|(i, (cmd, desc))| {
                let selected = i == props.selected_idx;
                let cmd_color  = if selected { TEXT_PRIMARY } else { TEXT_DIM };
                let desc_color = TEXT_DIM;
                let cursor     = if selected { "❯ " } else { "  " };
                element! {
                    View(key: i.to_string(), flex_direction: FlexDirection::Row, gap: 2) {
                        Text(content: cursor.to_string(), color: CRANBERRY, weight: Weight::Bold)
                        Text(content: cmd.clone(), color: cmd_color, weight: Weight::Bold)
                        Text(content: desc.clone(), color: desc_color)
                    }
                }
            }))
        }
    }
}
