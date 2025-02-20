mod builder;
mod input;
mod output;
mod prompt;
mod storage;
mod thinking;

pub use builder::build_session;

use anyhow::Result;
use etcetera::choose_app_strategy;
use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::Agent;
use goose::message::{Message, MessageContent};
use mcp_core::handler::ToolError;
use rand::{distributions::Alphanumeric, Rng};
use std::path::PathBuf;
use tokio;

use crate::log_usage::log_usage;

pub struct Session {
    agent: Box<dyn Agent>,
    messages: Vec<Message>,
    session_file: PathBuf,
}

impl Session {
    pub fn new(agent: Box<dyn Agent>, session_file: PathBuf) -> Self {
        let messages = match storage::read_messages(&session_file) {
            Ok(msgs) => msgs,
            Err(e) => {
                eprintln!("Warning: Failed to load message history: {}", e);
                Vec::new()
            }
        };

        Session {
            agent,
            messages,
            session_file,
        }
    }

    /// Add a stdio extension to the session
    ///
    /// # Arguments
    /// * `extension_command` - Full command string including environment variables
    ///   Format: "ENV1=val1 ENV2=val2 command args..."
    pub async fn add_extension(&mut self, extension_command: String) -> Result<()> {
        let mut parts: Vec<&str> = extension_command.split_whitespace().collect();
        let mut envs = std::collections::HashMap::new();

        // Parse environment variables (format: KEY=value)
        while let Some(part) = parts.first() {
            if !part.contains('=') {
                break;
            }
            let env_part = parts.remove(0);
            let (key, value) = env_part.split_once('=').unwrap();
            envs.insert(key.to_string(), value.to_string());
        }

        if parts.is_empty() {
            return Err(anyhow::anyhow!("No command provided in extension string"));
        }

        let cmd = parts.remove(0).to_string();
        // Generate a random name for the ephemeral extension
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let config = ExtensionConfig::Stdio {
            name,
            cmd,
            args: parts.iter().map(|s| s.to_string()).collect(),
            envs: Envs::new(envs),
        };

        self.agent
            .add_extension(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start extension: {}", e))
    }

    /// Add a builtin extension to the session
    ///
    /// # Arguments
    /// * `builtin_name` - Name of the builtin extension(s), comma separated
    pub async fn add_builtin(&mut self, builtin_name: String) -> Result<()> {
        for name in builtin_name.split(',') {
            let config = ExtensionConfig::Builtin {
                name: name.trim().to_string(),
            };
            self.agent
                .add_extension(config)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start builtin extension: {}", e))?;
        }
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut editor = rustyline::Editor::<(), rustyline::history::DefaultHistory>::new()?;

        // Load history from messages
        for msg in self
            .messages
            .iter()
            .filter(|m| m.role == mcp_core::role::Role::User)
        {
            for content in msg.content.iter() {
                if let Some(text) = content.as_text() {
                    if let Err(e) = editor.add_history_entry(text) {
                        eprintln!("Warning: Failed to add history entry: {}", e);
                    }
                }
            }
        }

        output::display_greeting();
        loop {
            match input::get_input(&mut editor)? {
                input::InputResult::Message(content) => {
                    self.messages.push(Message::user().with_text(&content));
                    storage::persist_messages(&self.session_file, &self.messages)?;

                    output::show_thinking();
                    self.process_agent_response().await?;
                    output::hide_thinking();
                }
                input::InputResult::Exit => break,
                input::InputResult::AddExtension(cmd) => {
                    match self.add_extension(cmd.clone()).await {
                        Ok(_) => output::render_extension_success(&cmd),
                        Err(e) => output::render_extension_error(&cmd, &e.to_string()),
                    }
                }
                input::InputResult::AddBuiltin(names) => {
                    match self.add_builtin(names.clone()).await {
                        Ok(_) => output::render_builtin_success(&names),
                        Err(e) => output::render_builtin_error(&names, &e.to_string()),
                    }
                }
                input::InputResult::ToggleTheme => {
                    let current = output::get_theme();
                    let new_theme = match current {
                        output::Theme::Light => {
                            println!("Switching to Dark theme");
                            output::Theme::Dark
                        }
                        output::Theme::Dark => {
                            println!("Switching to Ansi theme");
                            output::Theme::Ansi
                        }
                        output::Theme::Ansi => {
                            println!("Switching to Light theme");
                            output::Theme::Light
                        }
                    };
                    output::set_theme(new_theme);
                    continue;
                }
                input::InputResult::Retry => continue,
            }
        }

        // Log usage and cleanup
        if let Ok(home_dir) = choose_app_strategy(crate::APP_STRATEGY.clone()) {
            let usage = self.agent.usage().await;
            log_usage(
                home_dir,
                self.session_file.to_string_lossy().to_string(),
                usage,
            );
            println!(
                "\nClosing session. Recorded to {}",
                self.session_file.display()
            );
        }
        Ok(())
    }

    pub async fn headless_start(&mut self, initial_message: String) -> Result<()> {
        self.messages
            .push(Message::user().with_text(&initial_message));
        storage::persist_messages(&self.session_file, &self.messages)?;
        self.process_agent_response().await?;
        Ok(())
    }

    async fn process_agent_response(&mut self) -> Result<()> {
        let mut stream = self.agent.reply(&self.messages).await?;

        use futures::StreamExt;
        loop {
            tokio::select! {
                result = stream.next() => {
                    match result {
                        Some(Ok(message)) => {
                            self.messages.push(message.clone());
                            storage::persist_messages(&self.session_file, &self.messages)?;
                            output::hide_thinking();
                            output::render_message(&message);
                            output::show_thinking();
                        }
                        Some(Err(e)) => {
                            eprintln!("Error: {}", e);
                            drop(stream);
                            self.handle_interrupted_messages(false);
                            output::render_error(
                                "The error above was an exception we were not able to handle.\n\
                                These errors are often related to connection or authentication\n\
                                We've removed the conversation up to the most recent user message\n\
                                - depending on the error you may be able to continue",
                            );
                            break;
                        }
                        None => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    drop(stream);
                    self.handle_interrupted_messages(true);
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_interrupted_messages(&mut self, interrupt: bool) {
        // First, get any tool requests from the last message if it exists
        let tool_requests = self
            .messages
            .last()
            .filter(|msg| msg.role == mcp_core::role::Role::Assistant)
            .map_or(Vec::new(), |msg| {
                msg.content
                    .iter()
                    .filter_map(|content| {
                        if let MessageContent::ToolRequest(req) = content {
                            Some((req.id.clone(), req.tool_call.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            });

        if !tool_requests.is_empty() {
            // Interrupted during a tool request
            // Create tool responses for all interrupted tool requests
            let mut response_message = Message::user();
            let last_tool_name = tool_requests
                .last()
                .and_then(|(_, tool_call)| tool_call.as_ref().ok().map(|tool| tool.name.clone()))
                .unwrap_or_else(|| "tool".to_string());

            let notification = if interrupt {
                "Interrupted by the user to make a correction".to_string()
            } else {
                "An uncaught error happened during tool use".to_string()
            };
            for (req_id, _) in &tool_requests {
                response_message.content.push(MessageContent::tool_response(
                    req_id.clone(),
                    Err(ToolError::ExecutionError(notification.clone())),
                ));
            }
            self.messages.push(response_message);

            let prompt = format!(
                "The existing call to {} was interrupted. How would you like to proceed?",
                last_tool_name
            );
            self.messages.push(Message::assistant().with_text(&prompt));
            output::render_message(&Message::assistant().with_text(&prompt));
        } else {
            // An interruption occurred outside of a tool request-response.
            if let Some(last_msg) = self.messages.last() {
                if last_msg.role == mcp_core::role::Role::User {
                    match last_msg.content.first() {
                        Some(MessageContent::ToolResponse(_)) => {
                            // Interruption occurred after a tool had completed but not assistant reply
                            let prompt = "The tool calling loop was interrupted. How would you like to proceed?";
                            self.messages.push(Message::assistant().with_text(prompt));
                            output::render_message(&Message::assistant().with_text(prompt));
                        }
                        Some(_) => {
                            // A real users message
                            self.messages.pop();
                            let prompt = "Interrupted before the model replied and removed the last message.";
                            output::render_message(&Message::assistant().with_text(prompt));
                        }
                        None => panic!("No content in last message"),
                    }
                }
            }
        }
    }

    pub fn session_file(&self) -> PathBuf {
        self.session_file.clone()
    }
}
