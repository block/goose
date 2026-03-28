use iocraft::prelude::*;

use crate::colors::*;

const SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];

#[derive(Default, Props)]
pub struct HeaderProps {
    pub status: String,
    pub loading: bool,
    pub spin_idx: usize,
    /// (current_turn, total_turns) — shown when > 1 turn exists.
    pub turn_info: Option<(usize, usize)>,
    pub width: u16,
}

#[component]
pub fn Header(props: &HeaderProps) -> impl Into<AnyElement<'static>> {
    let status_color = if props.status == "ready" {
        TEAL
    } else if props.status.starts_with("error") || props.status.starts_with("failed") {
        CRANBERRY
    } else {
        TEXT_DIM
    };

    let spinner = SPINNER[props.spin_idx % SPINNER.len()];
    let rule = "─".repeat(props.width as usize);

    element! {
        View(flex_direction: FlexDirection::Column, width: props.width) {
            View(flex_direction: FlexDirection::Row, width: props.width) {
                // Left side: "goose · <status> [spinner]"
                View(flex_direction: FlexDirection::Row, gap: 0) {
                    Text(content: "goose", weight: Weight::Bold, color: TEXT_PRIMARY)
                    Text(content: " · ", color: RULE)
                    Text(content: props.status.clone(), color: status_color)
                    #(props.loading.then(|| element! {
                        Text(content: format!(" {spinner}"), color: CRANBERRY)
                    }))
                }
                // Right side: turn counter + exit hint
                View(flex_grow: 1.0, justify_content: JustifyContent::End, flex_direction: FlexDirection::Row, gap: 2) {
                    #(props.turn_info.filter(|(_, t)| *t > 1).map(|(c, t)| element! {
                        Text(content: format!("{c}/{t}"), color: TEXT_DIM)
                    }))
                    Text(
                        content: if props.loading { "^C stop" } else { "^C exit" }.to_string(),
                        color: if props.loading { CRANBERRY } else { TEXT_DIM },
                    )
                }
            }
            Text(content: rule, color: RULE)
        }
    }
}
