use iocraft::prelude::*;

use crate::colors::*;
use crate::types::{tool_kind_icon, ToolCallInfo, ToolStatus};

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

    let kind = tool_kind_icon(&info.title);

    element! {
        View(flex_direction: FlexDirection::Row, padding_left: 5) {
            Text(content: format!("{sym} "), color: color)
            Text(content: format!("{kind} "), color: TEXT_DIM)
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

    let kind = tool_kind_icon(&info.title);

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
                Text(content: kind.to_string(), color: TEXT_DIM)
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
                                #(render_output(output))
                            }
                        }))
                    }
                }
            }))
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Render tool output, colorizing unified diffs if detected.
///
/// A diff is detected heuristically: the text has at least one `@@` hunk
/// header and at least two `+`/`-` lines.
fn render_output(text: &str) -> Vec<AnyElement<'static>> {
    let has_hunk   = text.lines().any(|l| l.starts_with("@@"));
    let diff_lines = text.lines().filter(|l| l.starts_with('+') || l.starts_with('-')).count();
    let is_diff    = has_hunk && diff_lines >= 2;

    if !is_diff {
        return vec![element! { Text(content: text.to_string(), color: TEXT_SECONDARY) }.into()];
    }

    text.lines().map(|line| {
        let color = if line.starts_with("+++") || line.starts_with("---") {
            TEXT_SECONDARY
        } else if line.starts_with('+') {
            Color::Rgb { r: 74, g: 222, b: 128 }   // green
        } else if line.starts_with('-') {
            Color::Rgb { r: 248, g: 113, b: 113 }   // red
        } else if line.starts_with("@@") {
            Color::Rgb { r: 103, g: 232, b: 249 }   // cyan
        } else {
            TEXT_DIM
        };
        element! { Text(content: line.to_string(), color: color) }.into()
    }).collect()
}

fn status_style(status: &ToolStatus) -> (Color, &'static str) {
    match status {
        ToolStatus::Pending => (TEXT_DIM,  "○"),
        ToolStatus::Running => (TEAL,      "◐"),
        ToolStatus::Success => (Color::Rgb { r: 74, g: 222, b: 128 }, "✓"),
        ToolStatus::Error   => (CRANBERRY, "✗"),
    }
}
