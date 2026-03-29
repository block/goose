use iocraft::prelude::*;

use crate::colors::*;
use crate::types::ElicitationReq;

#[derive(Default, Props)]
pub struct ElicitationDialogProps {
    pub request: Option<ElicitationReq>,
    pub value: Option<State<String>>,
    pub width: u16,
}

#[component]
pub fn ElicitationDialog(props: &ElicitationDialogProps) -> impl Into<AnyElement<'static>> {
    let Some(req) = &props.request else {
        return element!(View);
    };
    let Some(mut value) = props.value else {
        return element!(View);
    };

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
            Text(content: "goose asks:".to_string(), color: GOLD, weight: Weight::Bold)
            // Wrap the message across multiple lines if needed.
            #(req.message.lines().map(|line| element! {
                Text(content: line.to_string(), color: TEXT_PRIMARY)
            }))
            View(flex_direction: FlexDirection::Row, margin_top: 1) {
                Text(content: "❯ ", color: GOLD, weight: Weight::Bold)
                View(flex_grow: 1.0) {
                    TextInput(
                        has_focus: true,
                        value: value.to_string(),
                        on_change: move |v| value.set(v),
                        multiline: false,
                    )
                }
            }
            Text(content: "enter to submit · esc to cancel", color: TEXT_DIM, italic: true)
        }
    }
}
