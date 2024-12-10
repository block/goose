use std::collections::HashMap;

use anyhow::Result;
use cliclack::{input, set_theme, spinner, Theme as CliclackTheme, ThemeState};
use goose::models::message::Message;

use super::{
    renderer::{render, BashDeveloperSystemRenderer, DefaultRenderer, ToolRenderer},
    thinking::get_random_thinking_message,
    Input, InputType, Prompt, Theme,
};

pub struct CliclackPrompt {
    spinner: cliclack::ProgressBar,
    input_mode: InputMode,
    theme: Theme,
    renderers: HashMap<String, Box<dyn ToolRenderer>>,
}

enum InputMode {
    Singleline,
    Multiline,
}

impl CliclackPrompt {
    pub fn new() -> Self {
        set_theme(PromptTheme);

        let mut renderers: HashMap<String, Box<dyn ToolRenderer>> = HashMap::new();
        let default_renderer = DefaultRenderer;
        renderers.insert(default_renderer.tool_name(), Box::new(default_renderer));
        let bash_dev_system_renderer = BashDeveloperSystemRenderer;
        renderers.insert(
            bash_dev_system_renderer.tool_name(),
            Box::new(bash_dev_system_renderer),
        );

        CliclackPrompt {
            spinner: spinner(),
            input_mode: InputMode::Singleline,
            theme: std::env::var("GOOSE_CLI_THEME")
                .ok()
                .map(|val| {
                    if val.eq_ignore_ascii_case("light") {
                        Theme::Light
                    } else {
                        Theme::Dark
                    }
                })
                .unwrap_or(Theme::Dark),
            renderers,
        }
    }
}

impl Prompt for CliclackPrompt {
    fn render(&mut self, message: Box<Message>) {
        render(message, &self.theme, self.renderers.clone());
    }

    fn show_busy(&mut self) {
        self.spinner = spinner();
        self.spinner
            .start(format!("{}...", get_random_thinking_message()));
    }

    fn hide_busy(&self) {
        self.spinner.stop("");
    }

    fn get_input(&mut self) -> Result<Input> {
        let mut input = input("Goose Chat: ( O)>         [Help: /?]").placeholder("");
        match self.input_mode {
            InputMode::Multiline => input = input.multiline(),
            InputMode::Singleline => (),
        }
        let mut message_text: String = match input.interact() {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Error getting input: {}", e);
                println!("If you are trying to exit use /exit");
                return Ok(Input {
                    input_type: InputType::AskAgain,
                    content: None,
                });
            }
        };
        message_text = message_text.trim().to_string();

        if message_text.eq_ignore_ascii_case("/exit") || message_text.eq_ignore_ascii_case("/quit")
        {
            Ok(Input {
                input_type: InputType::Exit,
                content: None,
            })
        } else if message_text.eq_ignore_ascii_case("/m") {
            println!("Switching to Multiline input mode");
            self.input_mode = InputMode::Multiline;
            return Ok(Input {
                input_type: InputType::AskAgain,
                content: None,
            });
        } else if message_text.eq_ignore_ascii_case("/s") {
            println!("Switching to Singleline input mode");
            self.input_mode = InputMode::Singleline;
            return Ok(Input {
                input_type: InputType::AskAgain,
                content: None,
            });
        } else if message_text.eq_ignore_ascii_case("/t") {
            self.theme = match self.theme {
                Theme::Light => {
                    println!("Switching to Dark theme");
                    Theme::Dark
                }
                Theme::Dark => {
                    println!("Switching to Light theme");
                    Theme::Light
                }
            };
            return Ok(Input {
                input_type: InputType::AskAgain,
                content: None,
            });
        } else if message_text.eq_ignore_ascii_case("/?") {
            println!("Commands:");
            println!("/exit - Exit the session");
            println!("/m - Switch to multiline input mode");
            println!("/s - Switch to singleline input mode");
            println!("/t - Toggle Light/Dark theme");
            println!("/? - Display this help message");
            println!("Ctrl+C - Interrupt goose (resets the interaction to before the interrupted user request)");
            return Ok(Input {
                input_type: InputType::AskAgain,
                content: None,
            });
        } else {
            return Ok(Input {
                input_type: InputType::Message,
                content: Some(message_text.to_string()),
            });
        }
    }

    fn close(&self) {
        // No cleanup required
    }

    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        panic!("Not implemented");
    }
}

//////
/// Custom theme for the prompt
//////
struct PromptTheme;

const EDIT_MODE_STR: &str = "[Esc](Preview)";
const PREVIEW_MODE_STR: &str = "[Enter](Submit)";

// We need a wrapper to be able to call the trait default implementation with the same name.
#[allow(dead_code)]
struct Wrapper<'a, T>(&'a T);
impl<'a, T: CliclackTheme> CliclackTheme for Wrapper<'a, T> {}

impl CliclackTheme for PromptTheme {
    /// The original logic for teaching the user how to submit in multiline mode.
    /// https://github.com/fadeevab/cliclack/blob/main/src/input.rs#L250
    /// We are replacing it to be more explicit.
    ///
    fn format_footer_with_message(&self, state: &ThemeState, message: &str) -> String {
        let new_message = match state {
            ThemeState::Active => {
                if EDIT_MODE_STR == message {
                    "Send [Press Esc then Enter]"
                } else if PREVIEW_MODE_STR == message {
                    "Send [Enter]"
                } else {
                    message
                }
            }
            _ => message,
        };

        Wrapper(self).format_footer_with_message(state, new_message)
    }
}
