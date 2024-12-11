use anyhow::Result;
use core::panic;
use futures::StreamExt;
use serde_json;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::agents::agent::Agent;
use crate::prompt::{InputType, Prompt};
use goose::developer::DeveloperSystem;
use goose::models::message::{Message, MessageContent};
use goose::models::role::Role;
use goose::systems::goose_hints::GooseHintsSystem;

// File management functions
pub fn ensure_session_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Could not determine home directory"))?;
    let config_dir = home_dir.join(".config").join("goose").join("sessions");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

pub fn get_most_recent_session() -> Result<PathBuf> {
    let session_dir = ensure_session_dir()?;
    let mut entries = fs::read_dir(&session_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "jsonl"))
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return Err(anyhow::anyhow!("No session files found"));
    }

    // Sort by modification time, most recent first
    entries.sort_by(|a, b| {
        b.metadata()
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)
            .cmp(
                &a.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH),
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

impl<'a> Session<'a> {
    pub fn new(agent: Box<dyn Agent>, prompt: Box<dyn Prompt + 'a>, session_file: PathBuf) -> Self {
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

        Session {
            agent,
            prompt,
            session_file,
            messages,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.setup_session();
        self.prompt.goose_ready();

        loop {
            let input = self.prompt.get_input().unwrap();
            match input.input_type {
                InputType::Message => {
                    if let Some(content) = &input.content {
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
        self.close_session();
        Ok(())
    }

    pub async fn headless_start(
        &mut self,
        initial_message: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.setup_session();

        self.messages
            .push(Message::user().with_text(initial_message.as_str()));
        persist_messages(&self.session_file, &self.messages)?;

        self.agent_process_messages().await;

        self.close_session();
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
                    goose::process_store::kill_processes();
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
                    Err(goose::errors::AgentError::ExecutionError(
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

    fn setup_session(&mut self) {
        let system = Box::new(DeveloperSystem::new());
        self.agent.add_system(system);
        let goosehints_system = Box::new(GooseHintsSystem::new());
        self.agent.add_system(goosehints_system);
    }

    fn close_session(&mut self) {
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

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::sync::{Arc, Mutex};

    use crate::agents::mock_agent::MockAgent;
    use crate::prompt::{self, Input};
    use crate::test_helpers::run_with_tmp_dir;

    use super::*;
    use goose::models::content::Content;
    use goose::models::tool;
    use goose::{errors::AgentResult, models::tool::ToolCall};
    use tempfile::NamedTempFile;

    // Helper function to create a test session
    fn create_test_session() -> Session<'static> {
        let temp_file = NamedTempFile::new().unwrap();
        let agent = Box::new(MockAgent {});
        let prompt = Box::new(MockPrompt::new());
        Session::new(agent, prompt, temp_file.path().to_path_buf())
    }

    fn create_test_session_with_prompt<'a>(prompt: Box<dyn Prompt + 'a>) -> Session<'a> {
        let temp_file = NamedTempFile::new().unwrap();
        let agent = Box::new(MockAgent {});
        Session::new(agent, prompt, temp_file.path().to_path_buf())
    }

    // Mock prompt implementation for testing
    pub struct MockPrompt {
        messages: Arc<Mutex<Vec<Message>>>, // Thread-safe, owned storage
    }

    impl MockPrompt {
        pub fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn add_message(&self, message: Message) {
            let mut messages = self.messages.lock().unwrap(); // Lock to safely modify
            messages.push(message);
        }

        pub fn get_messages(&self) -> Vec<Message> {
            let messages = self.messages.lock().unwrap(); // Lock to safely read
            messages.clone() // Return a clone to avoid borrowing issues
        }
    }

    impl Prompt for MockPrompt {
        fn get_input(&mut self) -> std::result::Result<prompt::Input, anyhow::Error> {
            Ok(Input {
                input_type: InputType::Message,
                content: Some("Msg:".to_string()),
            })
        }
        fn render(&mut self, message: Box<Message>) {
            self.add_message(message.as_ref().clone());
        }
        fn show_busy(&mut self) {}
        fn hide_busy(&self) {}
        fn goose_ready(&self) {}
        fn close(&self) {}
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_rewind_messages_only_user() {
        let mut session = create_test_session();
        session.messages.push(Message::user().with_text("Hello"));

        session.rewind_messages();
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_rewind_messages_user_then_assistant() {
        let mut session = create_test_session();
        session.messages.push(Message::user().with_text("Hello"));
        session
            .messages
            .push(Message::assistant().with_text("World"));

        session.rewind_messages();
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_rewind_messages_multiple_user_messages() {
        let mut session = create_test_session();
        session.messages.push(Message::user().with_text("First"));
        session
            .messages
            .push(Message::assistant().with_text("Response 1"));
        session.messages.push(Message::user().with_text("Second"));
        session.rewind_messages();
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, Role::User);
        assert_eq!(session.messages[1].role, Role::Assistant);
        assert_eq!(
            session.messages[0].content[0],
            MessageContent::text("First")
        );
        assert_eq!(
            session.messages[1].content[0],
            MessageContent::text("Response 1")
        );
    }

    #[test]
    fn test_rewind_messages_after_interrupted_tool_request() {
        let mut session = create_test_session();
        session.messages.push(Message::user().with_text("First"));
        session
            .messages
            .push(Message::assistant().with_text("Response 1"));
        session.messages.push(Message::user().with_text("Use tool"));

        let mut mixed_msg = Message::assistant();
        mixed_msg.content.push(MessageContent::text("Using tool"));
        mixed_msg.content.push(MessageContent::tool_request(
            "test",
            AgentResult::Ok(ToolCall::new("test", "test".into())),
        ));
        session.messages.push(mixed_msg);

        session.messages.push(Message::user().with_tool_response(
            "test",
            Err(goose::errors::AgentError::ExecutionError(
                "Test".to_string(),
            )),
        ));

        session.rewind_messages();
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, Role::User);
        assert_eq!(session.messages[1].role, Role::Assistant);
        assert_eq!(
            session.messages[0].content[0],
            MessageContent::text("First")
        );
        assert_eq!(
            session.messages[1].content[0],
            MessageContent::text("Response 1")
        );
    }

    #[test]
    fn test_interrupted_messages_only_1_user_msg() {
        let mut session = create_test_session_with_prompt(Box::new(MockPrompt::new()));
        session.messages.push(Message::user().with_text("Hello"));

        session.handle_interrupted_messages();

        assert!(session.messages.is_empty());

        assert_last_prompt_text(
            &session,
            "We interrupted before the model replied and removed the last message.",
        );
    }

    #[test]
    fn test_interrupted_messages_removes_last_user_msg() {
        let mut session = create_test_session_with_prompt(Box::new(MockPrompt::new()));
        session.messages.push(Message::user().with_text("Hello"));
        session.messages.push(Message::assistant().with_text("Hi"));
        session
            .messages
            .push(Message::user().with_text("How are you?"));

        session.handle_interrupted_messages();

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, Role::User);
        assert_eq!(
            session.messages[0].content[0],
            MessageContent::text("Hello")
        );
        assert_eq!(session.messages[1].role, Role::Assistant);
        assert_eq!(session.messages[1].content[0], MessageContent::text("Hi"));

        assert_last_prompt_text(
            &session,
            "We interrupted before the model replied and removed the last message.",
        );
    }

    #[test]
    fn test_interrupted_tool_use_resolves_with_last_tool_use_interrupted() {
        let tool_name1 = "test";
        let tool_call1 = tool::ToolCall::new(tool_name1, "test".into());
        let tool_result1 = AgentResult::Ok(vec![Content::text("Task 1 done")]);

        let tool_name2 = "test2";
        let tool_call2 = tool::ToolCall::new(tool_name2, "test2".into());
        let mut session = create_test_session_with_prompt(Box::new(MockPrompt::new()));
        session
            .messages
            .push(Message::user().with_text("Do something"));
        session.messages.push(
            Message::assistant()
                .with_text("Doing it")
                .with_tool_request("1", Ok(tool_call1.clone())),
        );
        session.messages.push(
            Message::user()
                .with_text("Did Task 1")
                .with_tool_response("1", tool_result1.clone()),
        );
        session
            .messages
            .push(Message::user().with_text("Do something else"));
        session.messages.push(
            Message::assistant()
                .with_text("Doing task 2")
                .with_tool_request("2", Ok(tool_call2.clone())),
        );

        session.handle_interrupted_messages();

        assert_eq!(session.messages.len(), 7);
        assert_eq!(session.messages[0].role, Role::User);
        assert_eq!(
            session.messages[0].content[0],
            MessageContent::text("Do something")
        );
        assert_eq!(session.messages[1].role, Role::Assistant);
        assert_eq!(
            session.messages[1].content[0],
            MessageContent::text("Doing it")
        );
        assert_eq!(
            session.messages[1].content[1],
            MessageContent::tool_request("1", Ok(tool_call1))
        );

        assert_eq!(session.messages[2].role, Role::User);
        assert_eq!(
            session.messages[2].content[0],
            MessageContent::text("Did Task 1")
        );
        assert_eq!(
            session.messages[2].content[1],
            MessageContent::tool_response("1", tool_result1)
        );

        assert_eq!(session.messages[3].role, Role::User);
        assert_eq!(
            session.messages[3].content[0],
            MessageContent::text("Do something else")
        );

        assert_eq!(
            session.messages[4].content[0],
            MessageContent::text("Doing task 2")
        );
        assert_eq!(
            session.messages[4].content[1],
            MessageContent::tool_request("2", Ok(tool_call2))
        );
        // Check the interrupted tool response message
        assert_eq!(session.messages[5].role, Role::User);
        let tool_result = Err(goose::errors::AgentError::ExecutionError(
            "Interrupted by the user to make a correction".to_string(),
        ));
        assert_eq!(
            session.messages[5].content[0],
            MessageContent::tool_response("2", tool_result)
        );

        // Check the follow-up assistant message
        assert_eq!(session.messages[6].role, Role::Assistant);
        assert_eq!(
            session.messages[6].content[0],
            MessageContent::text(format!(
                "We interrupted the existing call to {}. How would you like to proceed?",
                tool_name2
            ))
        );

        assert_last_prompt_text(
            &session,
            format!(
                "We interrupted the existing call to {}. How would you like to proceed?",
                tool_name2
            )
            .as_str(),
        );
    }

    #[test]
    fn test_interrupted_tool_use_interrupts_multiple_tools() {
        let tool_name1 = "test";
        let tool_call1 = tool::ToolCall::new(tool_name1, "test".into());

        let tool_name2 = "test2";
        let tool_call2 = tool::ToolCall::new(tool_name2, "test2".into());
        let mut session = create_test_session_with_prompt(Box::new(MockPrompt::new()));
        session
            .messages
            .push(Message::user().with_text("Do something"));
        session.messages.push(
            Message::assistant()
                .with_text("Doing it")
                .with_tool_request("1", Ok(tool_call1.clone()))
                .with_tool_request("2", Ok(tool_call2.clone())),
        );

        session.handle_interrupted_messages();

        assert_eq!(session.messages.len(), 4);
        assert_eq!(session.messages[0].role, Role::User);
        assert_eq!(
            session.messages[0].content[0],
            MessageContent::text("Do something")
        );
        assert_eq!(session.messages[1].role, Role::Assistant);
        assert_eq!(
            session.messages[1].content[0],
            MessageContent::text("Doing it")
        );
        assert_eq!(
            session.messages[1].content[1],
            MessageContent::tool_request("1", Ok(tool_call1))
        );
        assert_eq!(
            session.messages[1].content[2],
            MessageContent::tool_request("2", Ok(tool_call2))
        );

        // Check the interrupted tool response message
        assert_eq!(session.messages[2].role, Role::User);
        let tool_result = Err(goose::errors::AgentError::ExecutionError(
            "Interrupted by the user to make a correction".to_string(),
        ));
        assert_eq!(
            session.messages[2].content[0],
            MessageContent::tool_response("1", tool_result.clone())
        );
        assert_eq!(
            session.messages[2].content[1],
            MessageContent::tool_response("2", tool_result)
        );

        // Check the follow-up assistant message
        assert_eq!(session.messages[3].role, Role::Assistant);
        assert_eq!(
            session.messages[3].content[0],
            MessageContent::text(format!(
                "We interrupted the existing call to {}. How would you like to proceed?",
                tool_name2
            ))
        );

        assert_last_prompt_text(
            &session,
            format!(
                "We interrupted the existing call to {}. How would you like to proceed?",
                tool_name2
            )
            .as_str(),
        );
    }

    #[test]
    fn test_interrupted_tool_use_interrupts_completed_tool_result_but_no_assistant_msg_yet() {
        let tool_name1 = "test";
        let tool_call1 = tool::ToolCall::new(tool_name1, "test".into());
        let tool_result1 = AgentResult::Ok(vec![Content::text("Task 1 done")]);

        let mut session = create_test_session_with_prompt(Box::new(MockPrompt::new()));
        session
            .messages
            .push(Message::user().with_text("Do something"));
        session.messages.push(
            Message::assistant()
                .with_text("Doing part 1")
                .with_tool_request("1", Ok(tool_call1.clone())),
        );
        session
            .messages
            .push(Message::user().with_tool_response("1", tool_result1.clone()));

        session.handle_interrupted_messages();

        assert_eq!(session.messages.len(), 4);
        assert_eq!(session.messages[0].role, Role::User);
        assert_eq!(
            session.messages[0].content[0],
            MessageContent::text("Do something")
        );
        assert_eq!(session.messages[1].role, Role::Assistant);
        assert_eq!(
            session.messages[1].content[0],
            MessageContent::text("Doing part 1")
        );
        assert_eq!(
            session.messages[1].content[1],
            MessageContent::tool_request("1", Ok(tool_call1))
        );

        assert_eq!(session.messages[2].role, Role::User);
        assert_eq!(
            session.messages[2].content[0],
            MessageContent::tool_response("1", tool_result1.clone())
        );

        // Check the follow-up assistant message
        assert_eq!(session.messages[3].role, Role::Assistant);
        assert_eq!(
            session.messages[3].content[0],
            MessageContent::text(
                "We interrupted the existing calls to tools. How would you like to proceed?",
            )
        );

        assert_last_prompt_text(
            &session,
            "We interrupted the existing calls to tools. How would you like to proceed?",
        );
    }

    #[test]
    fn test_get_most_recent_session() -> Result<()> {
        use std::thread;
        use std::time::Duration;

        // Create a temporary directory for testing
        run_with_tmp_dir(|| {
            let session_dir = ensure_session_dir()?;

            // Create test session files with different timestamps
            let file1_path = session_dir.join("session1.jsonl");
            let file2_path = session_dir.join("session2.jsonl");
            let file3_path = session_dir.join("not_a_session.txt");

            fs::write(&file1_path, "test content")?;
            thread::sleep(Duration::from_millis(1));
            fs::write(&file2_path, "test content")?;
            thread::sleep(Duration::from_millis(1));
            fs::write(&file3_path, "test content")?;

            // Test getting the most recent session
            let most_recent = get_most_recent_session()?;
            assert_eq!(most_recent, file2_path);

            Ok(())
        })
    }

    fn assert_last_prompt_text(session: &Session, expected_text: &str) {
        let prompt = session
            .prompt
            .as_any()
            .downcast_ref::<MockPrompt>()
            .expect("Failed to downcast");
        let messages = prompt.get_messages();
        let msg = messages.last().unwrap();
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content[0], MessageContent::text(expected_text));
    }
}
