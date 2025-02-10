use anyhow::Result;
use core::panic;
use etcetera::{choose_app_strategy, AppStrategy};
use futures::StreamExt;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::log_usage::log_usage;
use crate::prompt::{InputType, Prompt};
use goose::agents::Agent;
use goose::message::{Message, MessageContent};
use mcp_core::handler::ToolError;
use mcp_core::role::Role;

// File management functions
pub fn ensure_session_dir() -> Result<PathBuf> {
    // choose_app_strategy().data_dir()
    // - macOS/Linux: ~/.local/share/goose/sessions/
    // - Windows:     ~\AppData\Roaming\Block\goose\data\sessions
    let config_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .in_data_dir("sessions");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

/// LEGACY NOTE: remove this once old paths are no longer needed.
pub fn legacy_session_dir() -> Option<PathBuf> {
    // legacy path was in the config dir ~/.config/goose/sessions/
    // ignore errors if we can't re-create the legacy session dir
    choose_app_strategy(crate::APP_STRATEGY.clone())
        .map(|strategy| strategy.in_config_dir("sessions"))
        .ok()
}

pub fn get_most_recent_session() -> Result<PathBuf> {
    let session_dir = ensure_session_dir()?;
    let mut entries = fs::read_dir(&session_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
        .collect::<Vec<_>>();

    // LEGACY NOTE: remove this once old paths are no longer needed.
    if entries.is_empty() {
        if let Some(old_dir) = legacy_session_dir() {
            // okay to return the error via ?, since that means we have no sessions in the
            // new location, and this old location doesn't exist, so a new session will be created
            let old_entries = fs::read_dir(&old_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
                .collect::<Vec<_>>();
            entries.extend(old_entries);
        }
    }

    if entries.is_empty() {
        return Err(anyhow::anyhow!("No session files found"));
    }

    // Sort by modification time, most recent first
    entries.sort_by(|a, b| {
        b.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(
                &a.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            )
    });

    Ok(entries[0].path())
}

pub fn readable_session_file(session_file: &PathBuf) -> Result<File> {
    match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(session_file)
    {
        Ok(file) => Ok(file),
        Err(e) => Err(anyhow::anyhow!("Failed to open session file: {}", e)),
    }
}

pub fn persist_messages(session_file: &PathBuf, messages: &[Message]) -> Result<()> {
    let file = fs::File::create(session_file)?; // Create or truncate the file
    persist_messages_internal(file, messages)
}

fn persist_messages_internal(session_file: File, messages: &[Message]) -> Result<()> {
    let mut writer = std::io::BufWriter::new(session_file);

    for message in messages {
        serde_json::to_writer(&mut writer, &message)?;
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn deserialize_messages(file: File) -> Result<Vec<Message>> {
    let reader = io::BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        messages.push(serde_json::from_str::<Message>(&line?)?);
    }

    Ok(messages)
}

// Session management
pub struct Session<'a> {
    agent: Box<dyn Agent>,
    prompt: Box<dyn Prompt + 'a>,
    session_file: PathBuf,
    messages: Vec<Message>,
}

#[allow(dead_code)]
impl<'a> Session<'a> {
    pub fn new(
        agent: Box<dyn Agent>,
        mut prompt: Box<dyn Prompt + 'a>,
        session_file: PathBuf,
    ) -> Self {
        let messages = match readable_session_file(&session_file) {
            Ok(file) => deserialize_messages(file).unwrap_or_else(|e| {
                eprintln!(
                    "Failed to read messages from session file. Starting fresh.\n{}",
                    e
                );
                Vec::<Message>::new()
            }),
            Err(e) => {
                eprintln!("Failed to load session file. Starting fresh.\n{}", e);
                Vec::<Message>::new()
            }
        };

        prompt.load_user_message_history(messages.clone());

        Session {
            agent,
            prompt,
            session_file,
            messages,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.prompt.goose_ready();

        loop {
            let input = self.prompt.get_input().unwrap();
            match input.input_type {
                InputType::Message => {
                    if let Some(content) = &input.content {
                        if content.is_empty() {
                            continue;
                        }
                        self.messages.push(Message::user().with_text(content));
                        persist_messages(&self.session_file, &self.messages)?;
                    }
                }
                InputType::Exit => break,
                InputType::AskAgain => continue,
            }

            self.prompt.show_busy();
            self.agent_process_messages().await;
            self.prompt.hide_busy();
        }
        self.close_session().await;
        Ok(())
    }

    pub async fn headless_start(
        &mut self,
        initial_message: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.messages
            .push(Message::user().with_text(initial_message.as_str()));
        persist_messages(&self.session_file, &self.messages)?;

        self.agent_process_messages().await;

        self.close_session().await;
        Ok(())
    }

    async fn agent_process_messages(&mut self) {
        let mut stream = match self.agent.reply(&self.messages).await {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Error starting reply stream: {}", e);
                return;
            }
        };
        loop {
            tokio::select! {
                response = stream.next() => {
                    match response {
                        Some(Ok(message)) => {
                            self.messages.push(message.clone());
                            persist_messages(&self.session_file, &self.messages).unwrap_or_else(|e| eprintln!("Failed to persist messages: {}", e));
                            self.prompt.hide_busy();
                            self.prompt.render(Box::new(message.clone()));
                            self.prompt.show_busy();
                        }
                        Some(Err(e)) => {
                            eprintln!("Error: {}", e);
                            drop(stream);
                            self.rewind_messages();
                            self.prompt.render(raw_message(r#"
The error above was an exception we were not able to handle.\n\n
These errors are often related to connection or authentication\n
We've removed the conversation up to the most recent user message
 - depending on the error you may be able to continue"#));
                            break;
                        }
                        None => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    // Kill any running processes when the client disconnects
                    // TODO is this used? I suspect post MCP this is on the server instead
                    // goose::process_store::kill_processes();
                    drop(stream);
                    self.handle_interrupted_messages();
                    break;
                }
            }
        }
    }

    /// Rewind the messages to before the last user message (they have cancelled it).
    fn rewind_messages(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        // Remove messages until we find the last user 'Text' message (not a tool response).
        while let Some(message) = self.messages.last() {
            if message.role == Role::User
                && message
                    .content
                    .iter()
                    .any(|c| matches!(c, MessageContent::Text(_)))
            {
                break;
            }
            self.messages.pop();
        }

        // Remove the last user text message we found.
        if !self.messages.is_empty() {
            self.messages.pop();
        }
    }

    fn handle_interrupted_messages(&mut self) {
        // First, get any tool requests from the last message if it exists
        let tool_requests = self
            .messages
            .last()
            .filter(|msg| msg.role == Role::Assistant)
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

            for (req_id, _) in &tool_requests {
                response_message.content.push(MessageContent::tool_response(
                    req_id.clone(),
                    Err(ToolError::ExecutionError(
                        "Interrupted by the user to make a correction".to_string(),
                    )),
                ));
            }
            self.messages.push(response_message);

            let prompt_response = &format!(
                "We interrupted the existing call to {}. How would you like to proceed?",
                last_tool_name
            );
            self.messages
                .push(Message::assistant().with_text(prompt_response));
            self.prompt.render(raw_message(prompt_response));
        } else {
            // An interruption occurred outside of a tool request-response.
            if let Some(last_msg) = self.messages.last() {
                if last_msg.role == Role::User {
                    match last_msg.content.first() {
                        Some(MessageContent::ToolResponse(_)) => {
                            // Interruption occurred after a tool had completed but not assistant reply
                            let prompt_response = "We interrupted the existing calls to tools. How would you like to proceed?";
                            self.messages
                                .push(Message::assistant().with_text(prompt_response));
                            self.prompt.render(raw_message(prompt_response));
                        }
                        Some(_) => {
                            // A real users message
                            self.messages.pop();
                            let prompt_response = "We interrupted before the model replied and removed the last message.";
                            self.prompt.render(raw_message(prompt_response));
                        }
                        None => panic!("No content in last message"),
                    }
                }
            }
        }
    }

    async fn close_session(&mut self) {
        let usage = self.agent.usage().await;
        log_usage(self.session_file.to_string_lossy().to_string(), usage);

        self.prompt.render(raw_message(
            format!(
                "Closing session. Recorded to {}\n",
                self.session_file.display()
            )
            .as_str(),
        ));
        self.prompt.close();
    }

    pub fn session_file(&self) -> PathBuf {
        self.session_file.clone()
    }
}

fn raw_message(content: &str) -> Box<Message> {
    Box::new(Message::assistant().with_text(content))
}
