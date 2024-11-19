use std::io::{self, Write};

use anyhow::Result;
use bat::WrappingMode;
use cliclack::{input, spinner};
use goose::models::message::{Message, MessageContent};

use super::prompt::{Input, InputType, Prompt, Theme};

pub struct CliclackPrompt {
    spinner: cliclack::ProgressBar,
    input_mode: InputMode,
    theme: Theme,
}

enum InputMode {
    Singleline,
    Multiline,
}

impl CliclackPrompt {
    pub fn new() -> Self {
        // // Load highlighting assets
        // let assets = HighlightingAssets::from_binary();

        // // Fetch and list all available themes
        // let themes = assets.themes();
        // for theme_name in themes {
        //     println!("{}", theme_name);
        // }

        // // List all available syntaxes (languages)
        // let syntax_set = assets.get_syntaxes().unwrap();
        // for syntax in syntax_set {
        //     println!("{}", syntax.name);
        // }

        CliclackPrompt {
            spinner: spinner(),
            input_mode: InputMode::Multiline,
            theme: Theme::Dark,
        }
    }
}

fn print_tool_request(content: &str, theme: &str, tool_name: &str) {
    bat::PrettyPrinter::new()
        .input(
            bat::Input::from_bytes(content.as_bytes()).name(format!("Tool Request: {}", tool_name)),
        )
        .theme(theme)
        .language("JSON")
        .grid(true)
        .header(true)
        .wrapping_mode(WrappingMode::Character)
        .print()
        .unwrap();
}

fn print_tool_response(content: &str, theme: &str, language: &str) {
    bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()).name("Tool Response:"))
        .theme(theme)
        .language(language)
        .grid(true)
        .header(true)
        .wrapping_mode(WrappingMode::Character)
        .print()
        .unwrap();
}

fn print(content: &str, theme: &str) {
    bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()))
        .theme(theme)
        .language("Markdown")
        .wrapping_mode(WrappingMode::Character)
        .print()
        .unwrap();
}

fn print_newline() {
    println!();
}

impl Prompt for CliclackPrompt {
    fn render(&mut self, message: Box<Message>) {
        let theme = match self.theme {
            Theme::Light => "GitHub",
            Theme::Dark => "zenburn",
        };

        // Currently tool usages seem to be coming back as User messages, is that right?
        // if message.role == goose::providers::types::message::Role::Assistant

        for message_content in &message.content {
            match message_content {
                MessageContent::Text(text) => print(&text.text, theme),
                MessageContent::ToolRequest(tool_request) => match &tool_request.tool_call {
                    Ok(call) => {
                        print_tool_request(
                            &serde_json::to_string_pretty(&call.arguments).unwrap(),
                            theme,
                            &call.name,
                        );
                    }
                    Err(e) => print(&e.to_string(), theme),
                },
                MessageContent::ToolResponse(tool_response) => {
                    match &tool_response.tool_result {
                        Ok(output) => {
                            let output_value = serde_json::to_string_pretty(output).unwrap();

                            // For pure text responses, strip the quotes and replace escaped newlines. Eg. bash responses
                            let unquoted = output_value.trim_matches('"');
                            let formatted = unquoted.replace("\\n", "\n");

                            let language = if formatted.starts_with("{") {
                                "JSON"
                            } else {
                                "Markdown"
                            };
                            print_tool_response(&formatted, theme, language);
                        }
                        Err(e) => print(&e.to_string(), theme),
                    }
                }
                MessageContent::Image(image) => {
                    println!("Image: [data: {}, type: {}]", image.data, image.mime_type);
                }
            }
        }

        print_newline();
        io::stdout().flush().expect("Failed to flush stdout");
    }

    fn show_busy(&self) {
        spinner().start("awaiting reply");
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
        let mut message_text: String = input.interact()?;
        message_text = message_text.trim().to_string();

        if message_text.eq_ignore_ascii_case("/exit") || message_text.eq_ignore_ascii_case("/quit")
        {
            return Ok(Input {
                input_type: InputType::Exit,
                content: None,
            });
        } else if message_text.eq_ignore_ascii_case("/m") {
            self.input_mode = InputMode::Multiline;
            return self.get_input();
        } else if message_text.eq_ignore_ascii_case("/s") {
            self.input_mode = InputMode::Singleline;
            return self.get_input();
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
            return self.get_input();
        } else if message_text.eq_ignore_ascii_case("/?") {
            println!("Commands:");
            println!("/exit - Exit the session");
            println!("/m - Switch to multiline input mode");
            println!("/s - Switch to singleline input mode");
            println!("/t - Toggle Light/Dark theme");
            println!("/? - Display this help message");
            println!("Ctrl+C - Interrupt goose (resets the interaction to before the interrupted user request)");
            return self.get_input();
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
}
