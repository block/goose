mod agent_response_handler;
mod builder;
mod commands;
mod completion;
mod completion_cache;
mod context_manager;
mod export;
mod input;
mod interactive;
mod message_handler;
mod output;
mod prompt;
mod thinking;
mod utils;

pub use self::export::message_to_markdown;
pub use builder::{build_session, SessionBuilderConfig, SessionSettings};
pub use goose::session::Identifier;

use anyhow::Result;
use completion_cache::CompletionCacheManager;
use goose::agents::Agent;
use goose::message::Message;
use goose::session;
use std::path::PathBuf;

pub enum RunMode {
    Normal,
    Plan,
}

pub struct Session {
    agent: Agent,
    messages: Vec<Message>,
    session_file: Option<PathBuf>,
    // Cache manager for completion data
    completion_cache_manager: CompletionCacheManager,
    debug: bool, // New field for debug mode
    run_mode: RunMode,
    scheduled_job_id: Option<String>, // ID of the scheduled job that triggered this session
    max_turns: Option<u32>,
}

pub use utils::{
    classify_planner_response, extract_session_id, get_reasoner, update_project_tracker,
    PlannerResponseType,
};

impl Session {
    pub fn new(
        agent: Agent,
        session_file: Option<PathBuf>,
        debug: bool,
        scheduled_job_id: Option<String>,
        max_turns: Option<u32>,
    ) -> Self {
        let messages = if let Some(session_file) = &session_file {
            match session::read_messages(session_file) {
                Ok(msgs) => msgs,
                Err(e) => {
                    eprintln!("Warning: Failed to load message history: {}", e);
                    Vec::new()
                }
            }
        } else {
            // Don't try to read messages if we're not saving sessions
            Vec::new()
        };

        Session {
            agent,
            messages,
            session_file,
            completion_cache_manager: CompletionCacheManager::new(),
            debug,
            run_mode: RunMode::Normal,
            scheduled_job_id,
            max_turns,
        }
    }

    /// Process a single message and exit
    pub async fn headless(&mut self, message: String) -> Result<()> {
        self.process_message(message).await
    }

    pub fn session_file(&self) -> Option<PathBuf> {
        self.session_file.clone()
    }

    pub fn message_history(&self) -> Vec<Message> {
        self.messages.clone()
    }

    pub fn get_metadata(&self) -> Result<session::SessionMetadata> {
        if !self.session_file.as_ref().is_some_and(|f| f.exists()) {
            return Err(anyhow::anyhow!("Session file does not exist"));
        }

        session::read_metadata(self.session_file.as_ref().unwrap())
    }

    // Get the session's total token usage
    pub fn get_total_token_usage(&self) -> Result<Option<i32>> {
        let metadata = self.get_metadata()?;
        Ok(metadata.total_tokens)
    }
}
