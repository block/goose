use iocraft::prelude::*;

use crate::colors::*;
use crate::components::tool_call_card::{ToolCallCard, ToolCallCompact};
use crate::types::{ToolStatus, Turn};

#[derive(Default, Props)]
pub struct TurnViewProps {
    pub turn: Option<Turn>,
    pub expanded_tool_call: Option<String>,
    /// Whether the agent is currently streaming into this turn.
    pub active: bool,
    pub status: String,
    pub width: u16,
    /// Lines scrolled up from the bottom (unused here; passed to allow future
    /// margin-top offset driving, but indicators are rendered by the parent).
    pub scroll_offset: i32,
}

#[component]
pub fn TurnView(props: &TurnViewProps) -> impl Into<AnyElement<'static>> {
    let Some(turn) = &props.turn else {
        return element!(View);
    };

    // "Featured" tool call: last non-success one, falling back to the last overall.
    let featured_id = turn
        .tool_call_order
        .iter()
        .rev()
        .find(|id| {
            turn.tool_calls
                .get(*id)
                .is_some_and(|tc| tc.status != ToolStatus::Success)
        })
        .or_else(|| turn.tool_call_order.last())
        .cloned();

    element! {
        View(flex_direction: FlexDirection::Column, width: props.width) {
            // Tool calls
            #(turn.tool_call_order.iter().map(|id| {
                let tc = turn.tool_calls.get(id).cloned();
                let expanded = props.expanded_tool_call.as_deref() == Some(id.as_str());
                let is_featured = featured_id.as_deref() == Some(id.as_str());

                let el: AnyElement<'static> = if expanded || is_featured {
                    element! {
                        ToolCallCard(key: id.clone(), info: tc, expanded: expanded)
                    }.into()
                } else {
                    element! {
                        ToolCallCompact(key: id.clone(), info: tc)
                    }.into()
                };
                el
            }))

            // Agent text (markdown already rendered to plain text)
            #((!turn.agent_text.is_empty()).then(|| element! {
                View(flex_direction: FlexDirection::Column, margin_top: 1, padding_left: 5) {
                    #(turn.agent_text.lines().map(|line| element! {
                        Text(content: line.to_string(), color: TEXT_PRIMARY)
                    }))
                }
            }))

            // Streaming indicator
            #((props.active && turn.agent_text.is_empty() && turn.tool_call_order.is_empty()).then(|| element! {
                View(padding_left: 5) {
                    Text(content: props.status.clone(), color: TEXT_DIM, italic: true)
                }
            }))
        }
    }
}
