use crate::services::config::TuiConfig;
use goose::conversation::message::{Message, TokenState};
use goose::session::Session;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub text: String,
    pub done: bool,
}

pub struct AppState {
    pub session_id: String,
    pub messages: Vec<Message>,
    pub token_state: TokenState,
    pub is_working: bool,
    pub input_mode: InputMode,
    pub todos: Vec<TodoItem>,
    pub flash_message: Option<(String, Instant)>,
    pub config: TuiConfig,
    pub available_tools: Vec<ToolInfo>,
    pub available_sessions: Vec<Session>,
    pub showing_help: bool,
    pub showing_todo: bool,
    pub showing_session_picker: bool,
    pub showing_command_builder: bool,
    pub showing_message_info: Option<usize>,
    pub has_worked: bool,
    pub input_text_is_empty: bool,
    pub model_context_limit: usize,
}

impl AppState {
    pub fn new(session_id: String, config: TuiConfig) -> Self {
        Self {
            session_id,
            messages: Vec::new(),
            token_state: TokenState::default(),
            is_working: false,
            input_mode: InputMode::Editing,
            todos: Vec::new(),
            flash_message: None,
            config,
            available_tools: Vec::new(),
            available_sessions: Vec::new(),
            showing_help: false,
            showing_todo: false,
            showing_session_picker: false,
            showing_command_builder: false,
            showing_message_info: None,
            has_worked: false,
            input_text_is_empty: true,
            model_context_limit: 128_000,
        }
    }
}
