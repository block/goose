use anyhow::Result;
use goose::message::Message;

pub mod renderer;
pub mod rustyline;
pub mod thinking;

pub trait Prompt {
    fn render(&mut self, message: Box<Message>);
    fn get_input(&mut self) -> Result<Input>;
    fn show_busy(&mut self);
    fn hide_busy(&self);
    fn close(&self);
    /// Load the user's message history into the prompt for command history navigation. First message is the oldest message.
    /// When history is supported by the prompt.
    fn load_user_message_history(&mut self, _messages: Vec<Message>) {}
    fn goose_ready(&self) {
        println!("\n");
        println!("Goose is running! Enter your instructions, or try asking what goose can do.");
        println!("\n");
    }
}

pub struct Input {
    pub input_type: InputType,
    pub content: Option<String>, // Optional content as sometimes the user may be issuing a command eg. (Exit)
}

pub enum InputType {
    AskAgain, // Ask the user for input again. Control flow command.
    Message,  // User sent a message
    Exit,     // User wants to exit the session
}

pub enum Theme {
    Light,
    Dark,
    Ansi, // Use terminal's ANSI/base16 colors directly.
}
