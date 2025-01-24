use std::collections::HashMap;

use super::{
    renderer::{
        render, BashDeveloperExtensionRenderer, DefaultRenderer, TextEditorRenderer, ToolRenderer,
    },
    thinking::get_random_thinking_message,
    Input, InputType, Prompt, Theme,
};

use anyhow::Result;
use cliclack::spinner;
use goose::message::Message;
use mcp_core::Role;
use rustyline::{DefaultEditor, EventHandler, KeyCode, KeyEvent, Modifiers};

const PROMPT: &str = "\x1b[1m\x1b[38;5;30m( O)> \x1b[0m";

pub struct RustylinePrompt {
    spinner: cliclack::ProgressBar,
    theme: Theme,
    renderers: HashMap<String, Box<dyn ToolRenderer>>,
    editor: DefaultEditor,
}

impl RustylinePrompt {
    pub fn new() -> Self {
        let mut renderers: HashMap<String, Box<dyn ToolRenderer>> = HashMap::new();
        let default_renderer = DefaultRenderer;
        renderers.insert(default_renderer.tool_name(), Box::new(default_renderer));

        let bash_dev_extension_renderer = BashDeveloperExtensionRenderer;
        renderers.insert(
            bash_dev_extension_renderer.tool_name(),
            Box::new(bash_dev_extension_renderer),
        );

        let text_editor_renderer = TextEditorRenderer;
        renderers.insert(
            text_editor_renderer.tool_name(),
            Box::new(text_editor_renderer),
        );

        let mut editor = DefaultEditor::new().expect("Failed to create editor");
        editor.bind_sequence(
            KeyEvent(KeyCode::Char('j'), Modifiers::CTRL),
            EventHandler::Simple(rustyline::Cmd::Newline),
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
            editor,
        }
    }
}

impl Prompt for RustylinePrompt {
    fn render(&mut self, message: Box<Message>) {
        render(&message, &self.theme, self.renderers.clone());
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
        let input = self.editor.readline(PROMPT);
        let mut message_text = match input {
            Ok(text) => {
                // Add valid input to history
                if let Err(e) = self.editor.add_history_entry(text.as_str()) {
                    eprintln!("Failed to add to history: {}", e);
                }
                text
            }
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

        if message_text.eq_ignore_ascii_case("/exit")
            || message_text.eq_ignore_ascii_case("/quit")
            || message_text.eq_ignore_ascii_case("exit")
            || message_text.eq_ignore_ascii_case("quit")
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
            println!("Ctrl+j - Adds a newline");
            println!("Use Up/Down arrow keys to navigate through command history");
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

    fn load_user_message_history(&mut self, messages: Vec<Message>) {
        for message in messages.into_iter().filter(|m| m.role == Role::User) {
            for content in message.content {
                if let Some(text) = content.as_text() {
                    if let Err(e) = self.editor.add_history_entry(text) {
                        eprintln!("Failed to add to history: {}", e);
                    }
                }
            }
        }
    }

    fn close(&self) {
        // No cleanup required
    }
}
