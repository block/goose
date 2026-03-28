use iocraft::prelude::*;

use crate::colors::*;

const FRAMES: &[&[&str]] = &[
    &["    ,_", "   (o >", "   //\\", "   \\\\ \\", "    \\\\_/", "     |  |", "     ^ ^"],
    &["     ,_", "    (o >", "    //\\", "    \\\\ \\", "     \\\\_/", "    /  |", "   ^   ^"],
    &["    ,_", "   (o >", "   //\\", "   \\\\ \\", "    \\\\_/", "     |  |", "     ^  ^"],
    &["   ,_", "  (o >", "  //\\", "  \\\\ \\", "   \\\\_/", "    |  \\", "    ^   ^"],
];

const GREETINGS: &[&str] = &[
    "What would you like to work on?",
    "Ready to build something amazing?",
    "What would you like to explore?",
    "What's on your mind?",
    "What shall we create today?",
];

#[derive(Default, Props)]
pub struct SplashProps {
    pub status: String,
    pub anim_frame: usize,
    /// When `true` the input field is shown; otherwise the status line.
    pub show_input: bool,
    pub input: Option<State<String>>,
    pub width: u16,
    pub height: u16,
}

#[component]
pub fn Splash(props: &SplashProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // Pick a greeting once and keep it stable.
    let greeting = hooks.use_state(|| {
        let idx = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize)
            .unwrap_or(0)
            % GREETINGS.len();
        GREETINGS[idx]
    });

    let frame = FRAMES[props.anim_frame % FRAMES.len()];
    let input_width = (props.width.saturating_sub(8)).min(56);
    let rule = "─".repeat(input_width as usize);

    let status_color = if props.status == "ready" { TEAL }
        else if props.status.starts_with("error") { CRANBERRY }
        else { TEXT_DIM };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            width: props.width,
            height: props.height,
        ) {
            // ASCII goose art
            View(flex_direction: FlexDirection::Column, align_items: AlignItems::Center) {
                #(frame.iter().map(|line| element! {
                    Text(content: line.to_string(), color: TEXT_PRIMARY)
                }))
            }

            View(margin_top: 1) {
                Text(content: "goose", weight: Weight::Bold, color: TEXT_PRIMARY)
            }
            Text(content: "your on-machine AI agent", color: TEXT_DIM)

            #(if props.show_input {
                if let Some(mut input_val) = props.input {
                    Some(element! {
                        View(flex_direction: FlexDirection::Column, align_items: AlignItems::Center, margin_top: 2) {
                            View(width: input_width) {
                                Text(content: rule.clone(), color: RULE)
                            }
                            View(flex_direction: FlexDirection::Row) {
                                Text(content: "❯ ", weight: Weight::Bold, color: CRANBERRY)
                                View(width: input_width - 2) {
                                    TextInput(
                                        has_focus: true,
                                        value: input_val.to_string(),
                                        on_change: move |v| input_val.set(v),
                                    )
                                }
                            }
                            View(width: input_width) {
                                Text(content: rule.clone(), color: RULE)
                            }
                            Text(content: greeting.get(), color: TEXT_DIM)
                        }
                    })
                } else {
                    None
                }
            } else {
                Some(element! {
                    View(margin_top: 2, flex_direction: FlexDirection::Row, gap: 1) {
                        Text(content: props.status.clone(), color: status_color)
                    }
                })
            })
        }
    }
}
