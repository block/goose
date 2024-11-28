use std::{
    collections::HashMap,
    io::{self, Write},
};

use anyhow::Result;
use bat::WrappingMode;
use cliclack::{input, set_theme, spinner, Theme as CliclackTheme, ThemeState};
use goose::models::message::{Message, MessageContent, ToolRequest, ToolResponse};

use super::{thinking::get_random_thinking_message, Input, InputType, Prompt, Theme};

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

        set_theme(PromptTheme);

        let mut renderers: HashMap<String, Box<dyn ToolRenderer>> = HashMap::new();
        let default_renderer = DefaultRenderer;
        renderers.insert(default_renderer.tool_name(), Box::new(default_renderer));

        CliclackPrompt {
            spinner: spinner(),
            input_mode: InputMode::Multiline,
            theme: Theme::Dark,
            renderers,
        }
    }
}

/// Implement the ToolRenderer trait for each tool that you want to render in the prompt.
trait ToolRenderer {
    fn tool_name(&self) -> String;
    fn request(&self, tool_request: &ToolRequest, theme: &str);
    fn response(&self, tool_response: &ToolResponse, theme: &str);
}

struct DefaultRenderer;

impl ToolRenderer for DefaultRenderer {
    fn tool_name(&self) -> String {
        "default".to_string()
    }

    fn request(&self, tool_request: &ToolRequest, theme: &str) {
        match &tool_request.tool_call {
            Ok(call) => {
                print_tool_request(
                    &serde_json::to_string_pretty(&call.arguments).unwrap(),
                    theme,
                    &call.name,
                );
            }
            Err(e) => print(&e.to_string(), theme),
        }
    }

    fn response(&self, tool_response: &ToolResponse, theme: &str) {
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

        let mut last_tool_name: &str = "default";
        for message_content in &message.content {
            match message_content {
                MessageContent::Text(text) => print(&text.text, theme),
                MessageContent::ToolRequest(tool_request) => match &tool_request.tool_call {
                    Ok(call) => {
                        last_tool_name = &call.name;
                        self.renderers
                            .get(&call.name)
                            .or_else(|| self.renderers.get("default"))
                            .unwrap()
                            .request(tool_request, theme);
                    }
                    Err(_) => self
                        .renderers
                        .get("default")
                        .unwrap()
                        .request(tool_request, theme),
                },
                MessageContent::ToolResponse(tool_response) => self
                    .renderers
                    .get(last_tool_name)
                    .or_else(|| self.renderers.get("default"))
                    .unwrap()
                    .response(tool_response, theme),
                MessageContent::Image(image) => {
                    println!("Image: [data: {}, type: {}]", image.data, image.mime_type);
                }
            }
        }

        print_newline();
        io::stdout().flush().expect("Failed to flush stdout");
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
