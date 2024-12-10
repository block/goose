use anyhow::Result;
use goose::models::message::Message;

pub mod cliclack;
pub mod renderer;
pub mod rustyline;
pub mod thinking;

pub trait Prompt {
    fn render(&mut self, message: Box<Message>);
    fn get_input(&mut self) -> Result<Input>;
    fn show_busy(&mut self);
    fn hide_busy(&self);
    fn close(&self);
    fn goose_ready(&self) {
        println!("\n");
        println!("Goose is running! Enter your instructions, or try asking what goose can do.");
        println!("\n");
    }
    // Used for testing. Allows us to downcast to any type.
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any;
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
}
