use goose::providers::base::ModelInfo;
use iocraft::prelude::*;

use crate::colors::*;

#[derive(Default, Props)]
pub struct ModelDialogProps {
    pub models: Vec<ModelInfo>,
    pub current_model: String,
    pub selected_idx: usize,
    pub width: u16,
}

#[component]
pub fn ModelDialog(props: &ModelDialogProps) -> impl Into<AnyElement<'static>> {
    if props.models.is_empty() {
        return element! {
            View(
                flex_direction: FlexDirection::Column,
                width: props.width,
                border_style: BorderStyle::Round,
                border_color: GOLD,
                padding_left: 1,
                padding_right: 1,
                margin_top: 1,
            ) {
                Text(content: "switch model".to_string(), color: GOLD, weight: Weight::Bold)
                Text(content: "no models available".to_string(), color: TEXT_DIM)
                Text(content: "esc to close".to_string(), color: TEXT_DIM, italic: true)
            }
        };
    }

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: props.width,
            border_style: BorderStyle::Round,
            border_color: GOLD,
            padding_left: 1,
            padding_right: 1,
            margin_top: 1,
        ) {
            Text(content: "switch model".to_string(), color: GOLD, weight: Weight::Bold)
            #(props.models.iter().enumerate().map(|(i, info)| {
                let selected  = i == props.selected_idx;
                let is_active = info.name == props.current_model;
                let cursor    = if selected { "❯ " } else { "  " };
                let marker    = if is_active { "✓ " } else { "  " };
                let name_color = if selected { TEXT_PRIMARY } else { TEXT_DIM };
                let marker_color = if is_active { TEAL } else { TEXT_DIM };
                element! {
                    View(key: i.to_string(), flex_direction: FlexDirection::Row) {
                        Text(content: cursor.to_string(), color: GOLD, weight: Weight::Bold)
                        Text(content: marker.to_string(), color: marker_color)
                        Text(content: info.name.clone(), color: name_color)
                    }
                }
            }))
            Text(content: "enter to switch · esc to close".to_string(), color: TEXT_DIM, italic: true)
        }
    }
}
