use iocraft::prelude::*;

use crate::colors::*;
use crate::types::{ToolCallInfo, ToolStatus};

// ── Compact (one-line) view ───────────────────────────────────────────────────

#[derive(Default, Props)]
pub struct ToolCallCompactProps {
    pub info: Option<ToolCallInfo>,
}

#[component]
pub fn ToolCallCompact(props: &ToolCallCompactProps) -> impl Into<AnyElement<'static>> {
    let Some(info) = &props.info else {
        return element!(View);
    };

    let (color, sym) = status_style(&info.status);

    element! {
        View(flex_direction: FlexDirection::Row, padding_left: 5) {
            Text(content: format!("{sym} "), color: color)
            Text(content: info.title.clone(), color: TEXT_SECONDARY)
        }
    }
}

// ── Full card (expanded) ──────────────────────────────────────────────────────

#[derive(Default, Props)]
pub struct ToolCallCardProps {
    pub info: Option<ToolCallInfo>,
    pub expanded: bool,
}

#[component]
pub fn ToolCallCard(props: &ToolCallCardProps) -> impl Into<AnyElement<'static>> {
    let Some(info) = &props.info else {
        return element!(View);
    };

    let (color, sym) = status_style(&info.status);

    element! {
        View(
            flex_direction: FlexDirection::Column,
            padding_left: 5,
            margin_top: 1,
            border_style: BorderStyle::Single,
            border_color: RULE,
        ) {
            View(flex_direction: FlexDirection::Row, gap: 1) {
                Text(content: sym.to_string(), color: color)
                Text(content: info.title.clone(), color: TEXT_PRIMARY, weight: Weight::Bold)
            }
            #(props.expanded.then(|| {
                element! {
                    View(flex_direction: FlexDirection::Column, margin_top: 1) {
                        #(info.input_preview.as_ref().map(|input| element! {
                            View(flex_direction: FlexDirection::Column) {
                                Text(content: "input:", color: TEXT_DIM)
                                Text(content: input.clone(), color: TEXT_SECONDARY)
                            }
                        }))
                        #(info.output_preview.as_ref().map(|output| element! {
                            View(flex_direction: FlexDirection::Column, margin_top: 1) {
                                Text(content: "output:", color: TEXT_DIM)
                                Text(content: output.clone(), color: TEXT_SECONDARY)
                            }
                        }))
                    }
                }
            }))
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn status_style(status: &ToolStatus) -> (Color, &'static str) {
    match status {
        ToolStatus::Pending => (TEXT_DIM,  "○"),
        ToolStatus::Running => (TEAL,      "◐"),
        ToolStatus::Success => (Color::Rgb { r: 74, g: 222, b: 128 }, "✓"),
        ToolStatus::Error   => (CRANBERRY, "✗"),
    }
}
