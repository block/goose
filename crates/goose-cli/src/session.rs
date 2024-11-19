use anyhow::Result;
use futures::StreamExt;

use crate::prompt::prompt::{InputType, Prompt};

use goose::agent::Agent;
use goose::developer::DeveloperSystem;
use goose::models::message::{Message, Role};

pub struct Session<'a> {
    agent: Box<Agent>,
    prompt: Box<dyn Prompt + 'a>,
}

impl<'a> Session<'a> {
    pub fn new(agent: Box<Agent>, prompt: Box<impl Prompt + 'a>) -> Self {
        Session { agent, prompt }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.setup_session();

        let mut messages = Vec::new();

        loop {
            let input = self.prompt.get_input().unwrap();
            match input.input_type {
                InputType::Exit => break,
                InputType::Message => {
                    if let Some(content) = &input.content {
                        messages.push(Message::user().with_text(content));
                    }
                }
            }

            self.prompt.show_busy();
            self.agent_process_messages(&mut messages).await;
            self.prompt.hide_busy();
        }
        self.close_session();
        Ok(())
    }

    pub async fn headless_start(
        &mut self,
        initial_message: Box<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.setup_session();

        let mut messages = Vec::new();
        messages.push(Message::user().with_text(initial_message.as_str()));

        self.agent_process_messages(&mut messages).await;

        self.close_session();
        Ok(())
    }

    async fn agent_process_messages(&mut self, messages: &mut Vec<Message>) {
        // Process the stream of messages
        let mut stream = self.agent.reply(&messages);
        loop {
            tokio::select! {
                response = stream.next() => {
                    match response {
                        Some(Ok(message)) => {
                            messages.push(message.clone());
                            self.prompt.render(Box::new(message.clone()));
                        }
                        Some(Err(e)) => {
                            // TODO: Handle error display through prompt
                            eprintln!("Error: {}", e);
                            break;
                        }
                        None => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    drop(stream);
                    // Pop all 'messages' from the assistant and the most recent user message. Resets the interaction to before the interrupted user request.
                    while let Some(message) = messages.pop() {
                        if message.role == Role::User {
                            break;
                        }
                        // else drop any assistant messages.
                    }

                    self.prompt.render(raw_message(" Interrupt: Resetting conversation to before the last sent message...\n"));
                    break;
                }
            }
        }
    }

    fn setup_session(&mut self) {
        self.prompt.render(raw_message("Starting session...\n"));

        let system = Box::new(DeveloperSystem::new());
        self.agent.add_system(system);
        self.prompt
            .render(raw_message("Connected the developer system.\n"));

        self.prompt.goose_ready();
    }

    fn close_session(&mut self) {
        self.prompt.close();
    }
}

fn raw_message(content: &str) -> Box<Message> {
    Box::new(Message::assistant().with_text(content))
}
