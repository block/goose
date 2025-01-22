use std::collections::HashMap;
use std::io::{self, Write};

use bat::WrappingMode;
use console::style;
use goose::message::{Message, MessageContent, ToolRequest, ToolResponse};
use mcp_core::role::Role;
use mcp_core::{content::Content, tool::ToolCall};
use serde_json::Value;

use super::Theme;

const MAX_STRING_LENGTH: usize = 40;
const INDENT: &str = "    ";

/// Implement the ToolRenderer trait for each tool that you want to render in the prompt.
pub trait ToolRenderer: ToolRendererClone {
    fn tool_name(&self) -> String;
    fn request(&self, tool_request: &ToolRequest, theme: &str);
    fn response(&self, tool_response: &ToolResponse, theme: &str);
}

// Helper trait for cloning boxed ToolRenderer objects
pub trait ToolRendererClone {
    fn clone_box(&self) -> Box<dyn ToolRenderer>;
}

// Implement the helper trait for any type that implements ToolRenderer and Clone
impl<T> ToolRendererClone for T
where
    T: 'static + ToolRenderer + Clone,
{
    fn clone_box(&self) -> Box<dyn ToolRenderer> {
        Box::new(self.clone())
    }
}

// Make Box<dyn ToolRenderer> clonable
impl Clone for Box<dyn ToolRenderer> {
    fn clone(&self) -> Box<dyn ToolRenderer> {
        self.clone_box()
    }
}

#[derive(Clone)]
pub struct DefaultRenderer;

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
            Err(e) => print_markdown(&e.to_string(), theme),
        }
    }

    fn response(&self, tool_response: &ToolResponse, theme: &str) {
        default_response_renderer(tool_response, theme);
    }
}

#[derive(Clone)]
pub struct BashDeveloperExtensionRenderer;

impl ToolRenderer for BashDeveloperExtensionRenderer {
    fn tool_name(&self) -> String {
        "DeveloperExtension__bash".to_string()
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
            Err(e) => print_markdown(&e.to_string(), theme),
        }
    }

    fn response(&self, tool_response: &ToolResponse, theme: &str) {
        default_response_renderer(tool_response, theme);
    }
}

pub fn render(message: &Message, theme: &Theme, renderers: HashMap<String, Box<dyn ToolRenderer>>) {
    let theme = match theme {
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
                    renderers
                        .get(&call.name)
                        .or_else(|| renderers.get("default"))
                        .unwrap()
                        .request(tool_request, theme);
                }
                Err(_) => renderers
                    .get("default")
                    .unwrap()
                    .request(tool_request, theme),
            },
            MessageContent::ToolResponse(tool_response) => renderers
                .get(last_tool_name)
                .or_else(|| renderers.get("default"))
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

pub fn default_response_renderer(tool_response: &ToolResponse, theme: &str) {
    match &tool_response.tool_result {
        Ok(contents) => {
            for content in contents {
                if content
                    .audience()
                    .is_some_and(|audience| !audience.contains(&Role::User))
                {
                    continue;
                }

                let min_priority = std::env::var("GOOSE_CLI_MIN_PRIORITY")
                    .ok()
                    .and_then(|val| val.parse::<f32>().ok())
                    .unwrap_or(0.0);

                if content
                    .priority()
                    .is_some_and(|priority| priority <= min_priority)
                {
                    continue;
                }

                if let Content::Text(text) = content {
                    print_markdown(&text.text, theme);
                }
            }
        }
        Err(e) => print_markdown(&e.to_string(), theme),
    }
}

pub fn default_print_request_header(call: &ToolCall) {
    // Print the tool name with an emoji

    // use rsplit to handle any prefixed tools with more underscores
    // unicode gets converted to underscores during sanitization
    let parts: Vec<_> = call.name.rsplit("__").collect();

    let tool_header = format!(
        "─── {} | {} ──────────────────────────",
        style(parts.first().unwrap_or(&"unknown")),
        style(
            parts
                .split_first()
                // client name is the rest of the split, reversed
                // reverse the iterator and re-join on __
                .map(|(_, s)| s.iter().rev().copied().collect::<Vec<_>>().join("__"))
                .unwrap_or_else(|| "unknown".to_string())
        )
        .magenta()
        .dim(),
    );
    print_newline();
    println!("{}", tool_header);
}

pub fn print_markdown(content: &str, theme: &str) {
    bat::PrettyPrinter::new()
        .input(bat::Input::from_bytes(content.as_bytes()))
        .theme(theme)
        .language("Markdown")
        .wrapping_mode(WrappingMode::Character)
        .print()
        .unwrap();
}

/// Format and print parameters recursively with proper indentation and colors
pub fn print_params(value: &Value, depth: usize) {
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

pub fn print_newline() {
    println!();
}
