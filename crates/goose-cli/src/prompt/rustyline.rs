use std::collections::HashMap;

use super::{
    renderer::{render, BashDeveloperSystemRenderer, DefaultRenderer, ToolRenderer},
    thinking::get_random_thinking_message,
    Input, InputType, Prompt, Theme,
};

use anyhow::Result;
use cliclack::spinner;
use goose::models::message::Message;

const PROMPT: &str = "\x1b[1m\x1b[38;5;30m( O)> \x1b[0m";

pub struct RustylinePrompt {
    spinner: cliclack::ProgressBar,
    theme: Theme,
    renderers: HashMap<String, Box<dyn ToolRenderer>>,
}

impl RustylinePrompt {
    pub fn new() -> Self {
        let mut renderers: HashMap<String, Box<dyn ToolRenderer>> = HashMap::new();
        let default_renderer = DefaultRenderer;
        renderers.insert(default_renderer.tool_name(), Box::new(default_renderer));
        let bash_dev_system_renderer = BashDeveloperSystemRenderer;
        renderers.insert(
            bash_dev_system_renderer.tool_name(),
            Box::new(bash_dev_system_renderer),
        );

        RustylinePrompt {
            spinner: spinner(),
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

impl Prompt for RustylinePrompt {
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
        let mut editor = rustyline::DefaultEditor::new()?;
        let input = editor.readline(PROMPT);
        let mut message_text = match input {
            Ok(text) => text,
            Err(e) => {
                match e {
                    rustyline::error::ReadlineError::Interrupted => (),
                    _ => eprintln!("Input error: {}", e),
                }
                return Ok(Input {
                    input_type: InputType::Exit,
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
        } else if message_text.eq_ignore_ascii_case("/?")
            || message_text.eq_ignore_ascii_case("/help")
        {
            println!("Commands:");
            println!("/exit - Exit the session");
            println!("/t - Toggle Light/Dark theme");
            println!("/? | /help - Display this help message");
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
