use crate::services::config::CustomCommand;
use goose::config::ExtensionEntry;
use goose::session::Session;
use goose_client::{ProviderDetails, ToolInfo};
use goose_server::routes::reply::MessageEvent;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Quit,
    Resize,

    ServerMessage(Arc<MessageEvent>),
    SessionResumed(Box<Session>),
    SessionsListLoaded(Vec<Session>),
    ToolsLoaded(Vec<ToolInfo>),
    ProvidersLoaded(Vec<ProviderDetails>),
    ExtensionsLoaded(Vec<ExtensionEntry>),
    ModelsLoaded {
        provider: String,
        models: Vec<String>,
    },
    ConfigLoaded(serde_json::Value),
    Error(String),

    SendMessage(goose::conversation::message::Message),
    Interrupt,
    ToggleInputMode,

    ToggleTodo,
    ToggleHelp,
    OpenSessionPicker,
    ResumeSession(String),
    CreateNewSession,
    ChangeTheme(String),
    ClearChat,
    ClosePopup,
    OpenMessageInfo(usize),
    SetInputEmpty(bool),
    OpenConfig,
    FetchModels(String),
    UpdateProvider {
        provider: String,
        model: String,
    },
    ToggleExtension {
        name: String,
        enabled: bool,
    },

    DeleteCustomCommand(String),
    StartCommandBuilder,
    SubmitCommandBuilder(CustomCommand),
}
