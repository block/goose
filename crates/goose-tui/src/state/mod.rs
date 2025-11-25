pub mod action;
pub mod reducer;

use crate::services::config::TuiConfig;
use goose::config::ExtensionEntry;
use goose::conversation::message::{Message, TokenState};
use goose::session::Session;
use goose_client::{ProviderDetails, ToolInfo};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
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
    pub providers: Vec<ProviderDetails>,
    pub extensions: Vec<ExtensionEntry>,
    pub showing_help: bool,
    pub showing_todo: bool,
    pub showing_session_picker: bool,
    pub showing_command_builder: bool,
    pub showing_message_info: Option<usize>,
    pub showing_config: bool,
    pub has_worked: bool,
    pub input_text_is_empty: bool,
    pub model_context_limit: usize,
    pub active_provider: Option<String>,
    pub active_model: Option<String>,
    pub needs_refresh: bool,
}

impl AppState {
    pub fn new(
        session_id: String,
        config: TuiConfig,
        active_provider: Option<String>,
        active_model: Option<String>,
    ) -> Self {
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
            providers: Vec::new(),
            extensions: Vec::new(),
            showing_help: false,
            showing_todo: false,
            showing_session_picker: false,
            showing_command_builder: false,
            showing_message_info: None,
            showing_config: false,
            has_worked: false,
            input_text_is_empty: true,
            model_context_limit: 128_000,
            active_provider,
            active_model,
            needs_refresh: false,
        }
    }
}
