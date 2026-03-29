use iocraft::prelude::*;

use crate::colors::*;
use crate::types::PermissionReq;

#[derive(Default, Props)]
pub struct PermissionDialogProps {
    pub request: Option<PermissionReq>,
    pub selected_idx: usize,
}

#[component]
pub fn PermissionDialog(props: &PermissionDialogProps) -> impl Into<AnyElement<'static>> {
    let Some(req) = &props.request else {
        return element!(View);
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            padding_left: 5,
            margin_top: 1,
            border_style: BorderStyle::Round,
            border_color: GOLD,
            width: 60,
        ) {
            Text(content: "🔒 Permission required", color: GOLD, weight: Weight::Bold)
            View(margin_top: 1) {
                Text(content: req.tool_title.clone(), color: TEXT_PRIMARY)
            }
            View(flex_direction: FlexDirection::Column, margin_top: 1) {
                #(req.options.iter().enumerate().map(|(i, opt)| {
                    let active = i == props.selected_idx;
                    let arrow = if active { " ▸ " } else { "   " };
                    let arrow_color = if active { GOLD } else { RULE };
                    let label = format!("[{}] {}", opt.key, opt.label);
                    let label_color = if active { TEXT_PRIMARY } else { TEXT_DIM };
                    let bold = if active { Weight::Bold } else { Weight::Normal };
                    element! {
                        View(key: opt.id.clone(), flex_direction: FlexDirection::Row) {
                            Text(content: arrow.to_string(), color: arrow_color)
                            Text(content: label, color: label_color, weight: bold)
                        }
                    }
                }))
            }
            View(margin_top: 1) {
                Text(content: "↑↓ select · enter confirm · esc cancel", color: TEXT_DIM)
            }
        }
    }
}
