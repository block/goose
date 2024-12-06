use std::{
    collections::HashMap,
    io::{self, Write},
};

use anyhow::Result;
use bat::WrappingMode;
use cliclack::spinner;
use console::style;
use goose::models::message::{Message, MessageContent, ToolRequest, ToolResponse};
use goose::models::role::Role;
use goose::models::{content::Content, tool::ToolCall};
use serde_json::Value;

use super::{thinking::get_random_thinking_message, Input, InputType, Prompt, Theme};

const PROMPT: &str = "\x1b[1m\x1b[38;5;30m( O)> \x1b[0m";
const MAX_STRING_LENGTH: usize = 40;
const INDENT: &str = "    ";

pub struct RustylinePrompt {
    spinner: cliclack::ProgressBar,
    theme: Theme,
    renderers: HashMap<String, Box<dyn ToolRenderer>>,
}

impl RustylinePrompt {
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
                default_print_request_header(call);

                // Format and print the parameters
                print_params(&call.arguments, 0);
                print_newline();
            }
            Err(e) => print(&e.to_string(), theme),
        }
    }

    fn response(&self, tool_response: &ToolResponse, theme: &str) {
        default_response_renderer(tool_response, theme);
    }
}

fn default_response_renderer(tool_response: &ToolResponse, theme: &str) {
    match &tool_response.tool_result {
        Ok(contents) => {
            for content in contents {
                if content
                    .audience()
                    .is_some_and(|audience| !audience.contains(&Role::User))
                {
                    continue;
                }

                if content.priority().is_some_and(|priority| priority == 0.0) {
                    continue;
                }

                if let Content::Text(text) = content {
                    print_markdown(&text.text, theme);
                }
            }
        }
        Err(e) => print(&e.to_string(), theme),
    }
}

fn default_print_request_header(call: &ToolCall) {
    // Print the tool name with an emoji
    let parts: Vec<_> = call.name.split("__").collect();

    let tool_header = format!(
        "─── {} | {} ──────────────────────────",
        style(parts.get(1).unwrap_or(&"unknown")),
        style(parts.first().unwrap_or(&"unknown")).magenta().dim(),
    );
    print_newline();
    println!("{}", tool_header);
}
struct BashDeveloperSystemRenderer;

impl ToolRenderer for BashDeveloperSystemRenderer {
    fn tool_name(&self) -> String {
        "DeveloperSystem__bash".to_string()
    }

    fn request(&self, tool_request: &ToolRequest, theme: &str) {
        match &tool_request.tool_call {
            Ok(call) => {
                default_print_request_header(call);

                match call.arguments.get("command") {
                    Some(Value::String(s)) => {
                        println!("{}: {}", style("command").dim(), style(s).green());
                    }
                    _ => print_params(&call.arguments, 0),
                }
                print_newline();
            }
            Err(e) => print(&e.to_string(), theme),
        }
    }

    fn response(&self, tool_response: &ToolResponse, theme: &str) {
        default_response_renderer(tool_response, theme);
    }
}

fn print_markdown(content: &str, theme: &str) {
    bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()))
        .theme(theme)
        .language("Markdown")
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

/// Format and print parameters recursively with proper indentation and colors
fn print_params(value: &Value, depth: usize) {
    let indent = INDENT.repeat(depth);

    match value {
        Value::Object(map) => {
            for (key, val) in map {
                match val {
                    Value::Object(_) => {
                        println!("{}{}:", indent, style(key).dim());
                        print_params(val, depth + 1);
                    }
                    Value::Array(arr) => {
                        println!("{}{}:", indent, style(key).dim());
                        for item in arr.iter() {
                            println!("{}{}- ", indent, INDENT);
                            print_params(item, depth + 2);
                        }
                    }
                    Value::String(s) => {
                        if s.len() > MAX_STRING_LENGTH {
                            println!("{}{}: {}", indent, style(key).dim(), style("...").dim());
                        } else {
                            println!("{}{}: {}", indent, style(key).dim(), style(s).green());
                        }
                    }
                    Value::Number(n) => {
                        println!("{}{}: {}", indent, style(key).dim(), style(n).blue());
                    }
                    Value::Bool(b) => {
                        println!("{}{}: {}", indent, style(key).dim(), style(b).blue());
                    }
                    Value::Null => {
                        println!("{}{}: {}", indent, style(key).dim(), style("null").dim());
                    }
                }
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                println!("{}{}.", indent, i + 1);
                print_params(item, depth + 1);
            }
        }
        Value::String(s) => {
            if s.len() > MAX_STRING_LENGTH {
                println!(
                    "{}{}",
                    indent,
                    style(format!("[REDACTED: {} chars]", s.len())).yellow()
                );
            } else {
                println!("{}{}", indent, style(s).green());
            }
        }
        Value::Number(n) => {
            println!("{}{}", indent, style(n).yellow());
        }
        Value::Bool(b) => {
            println!("{}{}", indent, style(b).yellow());
        }
        Value::Null => {
            println!("{}{}", indent, style("null").dim());
        }
    }
}

fn print_newline() {
    println!();
}

impl Prompt for RustylinePrompt {
    fn render(&mut self, message: Box<Message>) {
        let theme = match self.theme {
            Theme::Light => "GitHub",
            Theme::Dark => "zenburn",
        };

        let mut last_tool_name: &str = "default";
        for message_content in &message.content {
            match message_content {
                MessageContent::Text(text) => print_markdown(&text.text, theme),
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
        let mut editor = rustyline::DefaultEditor::new()?;
        let input = editor.readline(PROMPT);
        let mut message_text = match input {
            Ok(text) => text,
            Err(e) => {
                match e {
                    rustyline::error::ReadlineError::Interrupted => (),
                    rustyline::error::ReadlineError::Eof => (),
                    _ => eprintln!("Input error: {}", e),
                }
                return Ok(Input {
                    input_type: InputType::Exit,
                    content: None,
                });
            }
        };
        message_text = message_text.trim().to_string();

        if message_text.is_empty() {
            Ok(Input {
                input_type: InputType::AskAgain,
                content: None,
            })
        } else if message_text.eq_ignore_ascii_case("/exit")
            || message_text.eq_ignore_ascii_case("/quit")
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
