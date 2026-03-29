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
    pub working_dir: String,
    pub token_total: i64,
    /// Current goose mode (e.g. "auto", "approve", "smart_approve", "chat").
    pub goose_mode: String,
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

    // Format token count as "1.2k" / "45.6k" / "1.2M" for brevity.
    let token_str = format_tokens(props.token_total);

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
                // Right side: cwd · tokens · turn counter · mode · exit hint
                View(flex_grow: 1.0, justify_content: JustifyContent::End, flex_direction: FlexDirection::Row, gap: 2) {
                    #((!props.working_dir.is_empty()).then(|| element! {
                        Text(content: props.working_dir.clone(), color: TEXT_DIM)
                    }))
                    #((props.token_total > 0).then(|| element! {
                        Text(content: format!("{token_str} tok"), color: TEXT_DIM)
                    }))
                    #(props.turn_info.filter(|(_, t)| *t > 1).map(|(c, t)| element! {
                        Text(content: format!("{c}/{t}"), color: TEXT_DIM)
                    }))
                    #((!props.goose_mode.is_empty()).then(|| element! {
                        Text(content: props.goose_mode.clone(), color: TEAL)
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

fn format_tokens(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
