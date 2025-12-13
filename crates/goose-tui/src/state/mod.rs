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
#[allow(dead_code)]
pub struct PendingToolConfirmation {
    pub id: String,
    pub tool_name: String,
    pub arguments: serde_json::Map<String, serde_json::Value>,
    pub security_warning: Option<String>,
    pub message_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActivePopup {
    #[default]
    None,
    Help,
    Todo,
    SessionPicker,
    CommandBuilder,
    Config(usize),
    ThemePicker,
    MessageInfo(usize),
}

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Default)]
pub enum CwdAnalysisState {
    #[default]
    NotStarted,
    Pending,
    Complete(String),
    Failed,
}

impl CwdAnalysisState {
    pub fn take_result(&mut self) -> Option<String> {
        match std::mem::take(self) {
            CwdAnalysisState::Complete(s) => Some(s),
            other => {
                *self = other;
                None
            }
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, CwdAnalysisState::Pending)
    }
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
    pub active_popup: ActivePopup,
    pub has_worked: bool,
    pub input_text_is_empty: bool,
    pub model_context_limit: usize,
    pub active_provider: Option<String>,
    pub active_model: Option<String>,
    pub needs_refresh: bool,
    pub copy_mode: bool,
    pub pending_confirmation: Option<PendingToolConfirmation>,
    pub cwd_analysis: CwdAnalysisState,
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
            active_popup: ActivePopup::None,
            has_worked: false,
            input_text_is_empty: true,
            model_context_limit: crate::utils::DEFAULT_CONTEXT_LIMIT,
            active_provider,
            active_model,
            needs_refresh: false,
            copy_mode: false,
            pending_confirmation: None,
            cwd_analysis: CwdAnalysisState::default(),
        }
    }
}
