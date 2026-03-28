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
            View(flex_direction: FlexDirection::Row) {
                Text(content: "❯ ", color: CRANBERRY, weight: Weight::Bold)
                View(flex_grow: 1.0) {
                    TextInput(
                        has_focus: true,
                        value: value.to_string(),
                        on_change: move |v| value.set(v),
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
        }
    }
}
