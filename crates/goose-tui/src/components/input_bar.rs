use iocraft::prelude::*;

use crate::colors::*;

#[derive(Default, Props)]
pub struct InputBarProps {
    pub value: Option<State<String>>,
    pub has_queued: bool,
    pub width: u16,
}

#[component]
pub fn InputBar(props: &InputBarProps) -> impl Into<AnyElement<'static>> {
    let Some(mut value) = props.value else {
        return element!(View);
    };

    // Grow the text box to fit the current line count (at least 1).
    let line_count = value.read().lines().count().max(1) as u16;

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: props.width,
            margin_bottom: 1,
            border_style: BorderStyle::Round,
            border_color: RULE,
            padding_left: 1,
            padding_right: 1,
        ) {
            View(flex_direction: FlexDirection::Row, height: line_count) {
                Text(content: "❯ ", color: CRANBERRY, weight: Weight::Bold)
                View(flex_grow: 1.0, height: line_count) {
                    TextInput(
                        has_focus: true,
                        value: value.to_string(),
                        on_change: move |v| value.set(v),
                        multiline: true,
                    )
                }
            }
            #(props.has_queued.then(|| element! {
                Text(
                    content: "message queued — will send when goose finishes",
                    color: GOLD,
                    italic: true,
                )
            }))
            View(flex_direction: FlexDirection::Row, gap: 2) {
                Text(content: "shift+enter for newline", color: TEXT_DIM, italic: true)
                Text(content: "@path to attach file/image", color: TEXT_DIM, italic: true)
                Text(content: "shift+tab cycle mode", color: TEXT_DIM, italic: true)
                Text(content: "/ext extensions", color: TEXT_DIM, italic: true)
                Text(content: "/model switch model", color: TEXT_DIM, italic: true)
            }
        }
    }
}
