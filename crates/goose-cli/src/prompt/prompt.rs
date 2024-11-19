use anyhow::Result;
use goose::models::message::Message;

pub trait Prompt {
    fn render(&mut self, message: Box<Message>);
    fn get_input(&mut self) -> Result<Input>;
    fn show_busy(&self);
    fn hide_busy(&self);
    fn close(&self);
    fn goose_ready(&self) {
        self.draw_goose();
    }

    fn draw_goose(&self) {
        println!(
            r#"
        __
       ( 0)>
       ||
       ||
     __||_
  <=/     \=>
    \_____/
     |  |
     ^  ^
    "#
        );
    }
}

pub struct Input {
    pub input_type: InputType,
    pub content: Option<String>, // Optional content as sometimes the user may be issuing a command eg. (Exit)
}

pub enum InputType {
    Message, // User sent a message
    Exit,    // User wants to exit the session
}

pub enum Theme {
    Light,
    Dark,
}
